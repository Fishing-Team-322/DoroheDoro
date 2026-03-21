use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use prost::Message;

use crate::{
    error::{AppError, AppResult},
    proto::ingest,
};

pub const SPOOL_CODEC_IDENTITY: &str = "identity";
pub const SPOOL_CODEC_GZIP: &str = "gzip";

pub fn encode_spool_payload(
    batch: &ingest::LogBatch,
    compress_threshold_bytes: usize,
) -> AppResult<(String, Vec<u8>)> {
    let mut payload = Vec::new();
    batch.encode(&mut payload)?;

    if payload.len() < compress_threshold_bytes {
        return Ok((SPOOL_CODEC_IDENTITY.to_string(), payload));
    }

    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(&payload)?;
    let compressed = encoder.finish()?;
    Ok((SPOOL_CODEC_GZIP.to_string(), compressed))
}

pub fn decode_spool_payload(codec: &str, payload: &[u8]) -> AppResult<ingest::LogBatch> {
    let decoded = match codec {
        SPOOL_CODEC_IDENTITY => payload.to_vec(),
        SPOOL_CODEC_GZIP => {
            let mut decoder = GzDecoder::new(payload);
            let mut output = Vec::new();
            decoder.read_to_end(&mut output)?;
            output
        }
        other => {
            return Err(AppError::protocol(format!(
                "unsupported spool codec `{other}`"
            )))
        }
    };

    Ok(ingest::LogBatch::decode(decoded.as_slice())?)
}

pub fn write_spool_payload(
    spool_dir: &Path,
    batch_id: &str,
    codec: &str,
    payload: &[u8],
) -> AppResult<PathBuf> {
    fs::create_dir_all(spool_dir)?;
    let extension = match codec {
        SPOOL_CODEC_IDENTITY => "bin",
        SPOOL_CODEC_GZIP => "gz",
        other => {
            return Err(AppError::protocol(format!(
                "unsupported spool codec `{other}`"
            )))
        }
    };
    let path = spool_dir.join(format!("{batch_id}.{extension}"));
    fs::write(&path, payload)?;
    Ok(path)
}

pub fn remove_spool_payload(path: &Path) -> AppResult<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use crate::proto::ingest;

    use super::{decode_spool_payload, encode_spool_payload, SPOOL_CODEC_GZIP};

    #[test]
    fn round_trips_compressed_spool_payload() {
        let batch = ingest::LogBatch {
            agent_id: "agent-1".to_string(),
            host: "demo-host".to_string(),
            sent_at_unix_ms: 10,
            events: vec![ingest::LogEvent {
                timestamp_unix_ms: 10,
                message: "hello".repeat(1024),
                source: "demo".to_string(),
                source_type: "file".to_string(),
                service: "svc".to_string(),
                severity: "info".to_string(),
                labels: Default::default(),
                raw: "hello".repeat(1024),
            }],
        };

        let (codec, payload) = encode_spool_payload(&batch, 100).unwrap();
        let decoded = decode_spool_payload(&codec, &payload).unwrap();

        assert_eq!(codec, SPOOL_CODEC_GZIP);
        assert_eq!(decoded.events.len(), 1);
        assert_eq!(decoded.agent_id, "agent-1");
    }
}
