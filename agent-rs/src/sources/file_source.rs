use std::{
    collections::BTreeMap,
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
    config::SourceConfig,
    proto::ingest,
    runtime::RuntimeStatusHandle,
    sources::{decode_line, SourceCheckpoint, SourceEvent},
    state::{FileOffsetRecord, SqliteStateStore},
};

const FILE_POLL_INTERVAL: Duration = Duration::from_millis(500);

pub fn spawn_file_source(
    config: SourceConfig,
    store: SqliteStateStore,
    status: RuntimeStatusHandle,
    tx: mpsc::Sender<SourceEvent>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        run_file_source(config, store, status, tx, shutdown);
    })
}

fn run_file_source(
    config: SourceConfig,
    store: SqliteStateStore,
    status: RuntimeStatusHandle,
    tx: mpsc::Sender<SourceEvent>,
    shutdown: CancellationToken,
) {
    let source_path = config.path.to_string_lossy().into_owned();

    while !shutdown.is_cancelled() {
        let metadata = match fs::metadata(&config.path) {
            Ok(metadata) => metadata,
            Err(error) => {
                status.record_source_error(&source_path, error.to_string());
                thread::sleep(FILE_POLL_INTERVAL);
                continue;
            }
        };

        let file_key = file_key_from_metadata(&metadata);
        let stored_offset = store.load_file_offset(&config.path).ok().flatten();
        let start_offset =
            detect_start_offset(stored_offset.as_ref(), file_key.as_deref(), metadata.len());

        let mut reader = match open_reader(&config.path, start_offset) {
            Ok(reader) => reader,
            Err(error) => {
                status.record_source_error(&source_path, error.to_string());
                thread::sleep(FILE_POLL_INTERVAL);
                continue;
            }
        };

        let mut active_file_key = file_key;
        status.record_source_ready(&source_path, active_file_key.clone(), start_offset);

        loop {
            if shutdown.is_cancelled() {
                return;
            }

            match maybe_reopen_if_replaced(&config.path, &mut reader, active_file_key.as_deref()) {
                Ok(Some((new_reader, new_file_key, new_offset))) => {
                    reader = new_reader;
                    active_file_key = new_file_key.clone();
                    status.record_source_replaced(&source_path, new_file_key, new_offset);
                    continue;
                }
                Ok(None) => {}
                Err(error) => {
                    status.record_source_error(&source_path, error.to_string());
                    break;
                }
            }

            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    thread::sleep(FILE_POLL_INTERVAL);
                }
                Ok(_) => {
                    let offset = reader.stream_position().unwrap_or_default();
                    if let Some(message) = decode_line(&line) {
                        let mut labels = BTreeMap::new();
                        labels.insert("path".to_string(), source_path.clone());
                        labels.insert("source".to_string(), config.source.clone());

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

                        let source_event = SourceEvent {
                            checkpoint: SourceCheckpoint {
                                path: source_path.clone(),
                                file_key: active_file_key.clone(),
                                offset,
                            },
                            event,
                        };

                        if tx.blocking_send(source_event).is_err() {
                            return;
                        }
                        status.record_source_read(&source_path, offset);
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

fn open_reader(path: &Path, start_offset: u64) -> std::io::Result<BufReader<File>> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(start_offset))?;
    Ok(BufReader::new(file))
}

fn maybe_reopen_if_replaced(
    path: &Path,
    reader: &mut BufReader<File>,
    current_file_key: Option<&str>,
) -> std::io::Result<Option<(BufReader<File>, Option<String>, u64)>> {
    let current_position = reader.stream_position()?;
    let metadata = fs::metadata(path)?;
    let new_file_key = file_key_from_metadata(&metadata);
    let replaced = metadata.len() < current_position || new_file_key.as_deref() != current_file_key;

    if replaced {
        let start_offset = metadata.len();
        let reopened = open_reader(path, start_offset)?;
        return Ok(Some((reopened, new_file_key, start_offset)));
    }

    Ok(None)
}

pub fn detect_start_offset(
    stored_offset: Option<&FileOffsetRecord>,
    current_file_key: Option<&str>,
    file_len: u64,
) -> u64 {
    match stored_offset {
        Some(record)
            if record.file_key.as_deref() == current_file_key && record.offset <= file_len =>
        {
            record.offset
        }
        _ => file_len,
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

#[cfg(test)]
mod tests {
    use crate::state::FileOffsetRecord;

    use super::detect_start_offset;

    #[test]
    fn uses_stored_offset_when_file_matches() {
        let record = FileOffsetRecord {
            path: "/tmp/demo.log".to_string(),
            file_key: Some("1:2".to_string()),
            offset: 25,
            updated_at_unix_ms: 0,
        };

        assert_eq!(detect_start_offset(Some(&record), Some("1:2"), 100), 25);
    }

    #[test]
    fn resets_to_eof_when_file_changes() {
        let record = FileOffsetRecord {
            path: "/tmp/demo.log".to_string(),
            file_key: Some("1:2".to_string()),
            offset: 25,
            updated_at_unix_ms: 0,
        };

        assert_eq!(detect_start_offset(Some(&record), Some("9:9"), 100), 100);
    }
}
