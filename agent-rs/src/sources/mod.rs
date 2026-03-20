mod file_source;
mod line_codec;

use crate::proto::ingest;

pub use file_source::spawn_file_source;
pub use line_codec::decode_line;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCheckpoint {
    pub source_id: String,
    pub path: String,
    pub file_key: Option<String>,
    pub offset: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceEvent {
    pub checkpoint: SourceCheckpoint,
    pub approx_bytes: usize,
    pub event: ingest::LogEvent,
}
