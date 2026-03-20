mod batcher;

use std::{path::PathBuf, time::Duration};

use crate::{error::TransportErrorKind, proto::ingest, state::SourceOffsetMarker};

pub use batcher::spawn_batcher;

#[derive(Debug, Clone, PartialEq)]
pub struct PendingBatch {
    pub batch_id: String,
    pub batch: ingest::LogBatch,
    pub approx_bytes: usize,
    pub source_offsets: Vec<SourceOffsetMarker>,
    pub created_at_unix_ms: i64,
    pub attempt_count: u32,
    pub from_spool: bool,
    pub spool_payload_path: Option<PathBuf>,
    pub spool_codec: Option<String>,
}

impl PendingBatch {
    pub fn event_count(&self) -> usize {
        self.batch.events.len()
    }

    pub fn next_retry_delay(
        &self,
        error_kind: TransportErrorKind,
        degraded_mode: bool,
    ) -> Duration {
        let base_secs = match error_kind {
            TransportErrorKind::TransientNetwork => 1,
            TransportErrorKind::Unauthorized | TransportErrorKind::ServerRejected => 30,
            TransportErrorKind::SerializationError | TransportErrorKind::Unknown => 15,
        };
        let multiplier = 2_u64.saturating_pow(self.attempt_count.min(6));
        let max_secs = if degraded_mode { 60 } else { 30 };
        Duration::from_secs((base_secs * multiplier).min(max_secs))
    }
}

pub fn approximate_event_size(event: &ingest::LogEvent) -> usize {
    let mut total = 64 + event.message.len() + event.raw.len();
    total +=
        event.source.len() + event.source_type.len() + event.service.len() + event.severity.len();
    total += event
        .labels
        .iter()
        .map(|(key, value)| key.len() + value.len() + 8)
        .sum::<usize>();
    total
}
