use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Seek, SeekFrom},
    path::Path,
    thread,
    time::Duration,
};

use chrono::Utc;
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_util::sync::CancellationToken;

use crate::{
    batching::approximate_event_size,
    config::{QueueConfig, SourceConfig, StartAt},
    metadata::EventEnrichmentContext,
    proto::ingest,
    runtime::RuntimeStatusHandle,
    sources::{decode_line, SourceCheckpoint, SourceEvent},
    state::{FileOffsetRecord, SqliteStateStore},
};

const FILE_POLL_INTERVAL: Duration = Duration::from_millis(500);

pub fn spawn_file_source(
    config: SourceConfig,
    queue_config: QueueConfig,
    enrichment: EventEnrichmentContext,
    store: SqliteStateStore,
    status: RuntimeStatusHandle,
    tx: mpsc::Sender<SourceEvent>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        run_file_source(
            config,
            queue_config,
            enrichment,
            store,
            status,
            tx,
            shutdown,
        );
    })
}

fn run_file_source(
    config: SourceConfig,
    queue_config: QueueConfig,
    enrichment: EventEnrichmentContext,
    store: SqliteStateStore,
    status: RuntimeStatusHandle,
    tx: mpsc::Sender<SourceEvent>,
    shutdown: CancellationToken,
) {
    let source_path = config.path.to_string_lossy().into_owned();
    let source_id = config.source_id().to_string();

    while !shutdown.is_cancelled() {
        let metadata = match fs::metadata(&config.path) {
            Ok(metadata) => metadata,
            Err(error) => {
                status.record_source_missing(&source_path, error.to_string());
                thread::sleep(FILE_POLL_INTERVAL);
                continue;
            }
        };

        let file_key = file_key_from_metadata(&metadata);
        let stored_offset = store.load_file_offset(&config.path).ok().flatten();
        let start_offset = detect_start_offset(
            stored_offset.as_ref(),
            file_key.as_deref(),
            metadata.len(),
            &config.start_at,
        );

        let mut reader = match open_reader(&config.path, start_offset) {
            Ok(reader) => reader,
            Err(error) => {
                status.record_source_error(&source_path, error.to_string());
                thread::sleep(FILE_POLL_INTERVAL);
                continue;
            }
        };

        let mut active_file_key = file_key;
        let mut pending_reopen = false;
        status.record_source_ready(
            &source_path,
            &source_id,
            active_file_key.clone(),
            start_offset,
        );

        loop {
            if shutdown.is_cancelled() {
                return;
            }

            match inspect_path_state(&config.path, &mut reader, active_file_key.as_deref()) {
                PathState::CopyTruncate(new_file_key) => {
                    match open_reader(&config.path, 0) {
                        Ok(new_reader) => {
                            reader = new_reader;
                            active_file_key = new_file_key;
                            status.record_source_replaced(
                                &source_path,
                                &source_id,
                                active_file_key.clone(),
                                0,
                            );
                        }
                        Err(error) => {
                            status.record_source_error(&source_path, error.to_string());
                            break;
                        }
                    }
                    continue;
                }
                PathState::Rotated(new_file_key) => {
                    pending_reopen = true;
                    status.record_source_rotation_detected(
                        &source_path,
                        &source_id,
                        new_file_key.clone(),
                    );
                }
                PathState::Missing(error) => {
                    status.record_source_missing(&source_path, error);
                }
                PathState::Unchanged => {}
            }

            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    if pending_reopen {
                        match reopen_rotated(&config.path) {
                            Ok(Some((new_reader, new_file_key))) => {
                                reader = new_reader;
                                active_file_key = new_file_key;
                                pending_reopen = false;
                                status.record_source_replaced(
                                    &source_path,
                                    &source_id,
                                    active_file_key.clone(),
                                    0,
                                );
                                continue;
                            }
                            Ok(None) => {}
                            Err(error) => {
                                status.record_source_error(&source_path, error.to_string());
                            }
                        }
                    }
                    thread::sleep(FILE_POLL_INTERVAL);
                }
                Ok(_) => {
                    let offset = reader.stream_position().unwrap_or_default();
                    if let Some(message) = decode_line(&line) {
                        let labels =
                            enrichment.labels_for_source(&source_path, &config.source, &source_id);

                        let event = ingest::LogEvent {
                            timestamp_unix_ms: Utc::now().timestamp_millis(),
                            message: message.clone(),
                            source: config.source.clone(),
                            source_type: "file".to_string(),
                            service: config.service.clone(),
                            severity: config.severity_hint.clone(),
                            labels,
                            raw: message,
                        };

                        let approx_bytes = approximate_event_size(&event);
                        let source_event = SourceEvent {
                            checkpoint: SourceCheckpoint {
                                source_id: source_id.clone(),
                                path: source_path.clone(),
                                file_key: active_file_key.clone(),
                                offset,
                            },
                            approx_bytes,
                            event,
                        };

                        if send_with_backpressure(
                            &status,
                            &queue_config,
                            &tx,
                            source_event,
                            shutdown.clone(),
                        )
                        .is_err()
                        {
                            return;
                        }
                        status.record_source_read(
                            &source_path,
                            &source_id,
                            active_file_key.clone(),
                            offset,
                        );
                    }
                }
                Err(error) => {
                    status.record_source_error(&source_path, error.to_string());
                    break;
                }
            }
        }

        thread::sleep(FILE_POLL_INTERVAL);
    }
}

fn send_with_backpressure(
    status: &RuntimeStatusHandle,
    queue_config: &QueueConfig,
    tx: &mpsc::Sender<SourceEvent>,
    mut source_event: SourceEvent,
    shutdown: CancellationToken,
) -> Result<(), ()> {
    loop {
        if shutdown.is_cancelled() {
            return Err(());
        }

        if status.current_event_queue_bytes() >= queue_config.event_bytes_soft_limit {
            status.record_event_queue_full();
            thread::sleep(status.reader_backoff_duration());
            continue;
        }

        let approx_bytes = source_event.approx_bytes;
        match tx.try_send(source_event) {
            Ok(()) => {
                status.record_event_queue_push(approx_bytes);
                return Ok(());
            }
            Err(mpsc::error::TrySendError::Full(event)) => {
                source_event = event;
                status.record_event_queue_full();
                thread::sleep(status.reader_backoff_duration());
            }
            Err(mpsc::error::TrySendError::Closed(_)) => return Err(()),
        }
    }
}

fn open_reader(path: &Path, start_offset: u64) -> std::io::Result<BufReader<File>> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(start_offset))?;
    Ok(BufReader::new(file))
}

fn inspect_path_state(
    path: &Path,
    reader: &mut BufReader<File>,
    current_file_key: Option<&str>,
) -> PathState {
    let current_position = match reader.stream_position() {
        Ok(position) => position,
        Err(error) => return PathState::Missing(error.to_string()),
    };

    match fs::metadata(path) {
        Ok(metadata) => {
            let new_file_key = file_key_from_metadata(&metadata);
            if metadata.len() < current_position && new_file_key.as_deref() == current_file_key {
                return PathState::CopyTruncate(new_file_key);
            }
            if new_file_key.as_deref() != current_file_key {
                return PathState::Rotated(new_file_key);
            }
            PathState::Unchanged
        }
        Err(error) => PathState::Missing(error.to_string()),
    }
}

fn reopen_rotated(path: &Path) -> std::io::Result<Option<(BufReader<File>, Option<String>)>> {
    match fs::metadata(path) {
        Ok(metadata) => {
            let new_file_key = file_key_from_metadata(&metadata);
            let reader = open_reader(path, 0)?;
            Ok(Some((reader, new_file_key)))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn detect_start_offset(
    stored_offset: Option<&FileOffsetRecord>,
    current_file_key: Option<&str>,
    file_len: u64,
    start_at: &StartAt,
) -> u64 {
    match stored_offset {
        Some(record) if record.file_key.as_deref() == current_file_key => {
            let durable_read_offset = record.durable_read_offset.max(record.acked_offset);
            if durable_read_offset <= file_len {
                durable_read_offset
            } else {
                0
            }
        }
        _ => match start_at {
            StartAt::Beginning => 0,
            StartAt::End => file_len,
        },
    }
}

pub fn file_key_from_metadata(metadata: &fs::Metadata) -> Option<String> {
    file_key_from_parts(platform_file_key(metadata))
}

#[cfg(unix)]
fn platform_file_key(metadata: &fs::Metadata) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;

    Some((metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
fn platform_file_key(_metadata: &fs::Metadata) -> Option<(u64, u64)> {
    None
}

fn file_key_from_parts(parts: Option<(u64, u64)>) -> Option<String> {
    parts.map(|(dev, ino)| format!("{dev}:{ino}"))
}

enum PathState {
    Unchanged,
    CopyTruncate(Option<String>),
    Rotated(Option<String>),
    Missing(String),
}

#[cfg(test)]
mod tests {
    use crate::{config::StartAt, state::FileOffsetRecord};

    use super::detect_start_offset;

    #[test]
    fn resumes_from_durable_read_offset_when_spool_is_present() {
        let record = FileOffsetRecord {
            path: "/tmp/demo.log".to_string(),
            file_key: Some("1:2".to_string()),
            durable_read_offset: 50,
            acked_offset: 25,
            updated_at_unix_ms: 0,
        };

        assert_eq!(
            detect_start_offset(Some(&record), Some("1:2"), 100, &StartAt::End),
            50
        );
    }

    #[test]
    fn respects_start_at_beginning_without_state() {
        assert_eq!(
            detect_start_offset(None, Some("1:2"), 100, &StartAt::Beginning),
            0
        );
    }

    #[test]
    fn resets_to_zero_after_truncate_on_same_inode() {
        let record = FileOffsetRecord {
            path: "/tmp/demo.log".to_string(),
            file_key: Some("1:2".to_string()),
            durable_read_offset: 200,
            acked_offset: 100,
            updated_at_unix_ms: 0,
        };

        assert_eq!(
            detect_start_offset(Some(&record), Some("1:2"), 20, &StartAt::End),
            0
        );
    }
}
