use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{AppError, AppResult};

use super::{
    AgentIdentity, FileOffsetRecord, FileOffsetUpdate, RuntimeStateRecord, SpoolBatchRecord,
    SpoolStats,
};

#[derive(Debug, Clone)]
pub struct SqliteStateStore {
    db_path: PathBuf,
}

impl SqliteStateStore {
    pub fn new(state_dir: &Path) -> AppResult<Self> {
        std::fs::create_dir_all(state_dir)?;
        let store = Self {
            db_path: state_dir.join("state.db"),
        };
        store.init()?;
        Ok(store)
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn load_identity(&self) -> AppResult<Option<AgentIdentity>> {
        let conn = self.open()?;
        let record = conn
            .query_row(
                "SELECT agent_id, hostname, version, created_at_unix_ms, updated_at_unix_ms
                 FROM agent_identity WHERE singleton_id = 1",
                [],
                |row| {
                    Ok(AgentIdentity {
                        agent_id: row.get(0)?,
                        hostname: row.get(1)?,
                        version: row.get(2)?,
                        created_at_unix_ms: row.get(3)?,
                        updated_at_unix_ms: row.get(4)?,
                    })
                },
            )
            .optional()?;

        Ok(record)
    }

    pub fn save_identity(&self, agent_id: &str, hostname: &str, version: &str) -> AppResult<()> {
        let conn = self.open()?;
        let now = now_ms();
        conn.execute(
            "INSERT INTO agent_identity (
                singleton_id, agent_id, hostname, version, created_at_unix_ms, updated_at_unix_ms
             ) VALUES (1, ?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(singleton_id) DO UPDATE SET
                agent_id = excluded.agent_id,
                hostname = excluded.hostname,
                version = excluded.version,
                updated_at_unix_ms = excluded.updated_at_unix_ms",
            params![agent_id, hostname, version, now],
        )?;
        Ok(())
    }

    pub fn load_runtime_state(&self) -> AppResult<RuntimeStateRecord> {
        let conn = self.open()?;
        let record = conn
            .query_row(
                "SELECT applied_policy_revision, policy_body_json,
                        last_successful_send_at_unix_ms, last_known_edge_url,
                        identity_status, identity_status_reason,
                        degraded_mode, blocked_delivery, blocked_reason,
                        spool_enabled, consecutive_send_failures, updated_at_unix_ms
                 FROM agent_runtime_state WHERE singleton_id = 1",
                [],
                |row| {
                    Ok(RuntimeStateRecord {
                        applied_policy_revision: row.get(0)?,
                        policy_body_json: row.get(1)?,
                        last_successful_send_at_unix_ms: row.get(2)?,
                        last_known_edge_url: row.get(3)?,
                        identity_status: row.get(4)?,
                        identity_status_reason: row.get(5)?,
                        degraded_mode: row.get::<_, i64>(6)? != 0,
                        blocked_delivery: row.get::<_, i64>(7)? != 0,
                        blocked_reason: row.get(8)?,
                        spool_enabled: row.get::<_, i64>(9)? != 0,
                        consecutive_send_failures: row.get::<_, u32>(10)?,
                        updated_at_unix_ms: row.get(11)?,
                    })
                },
            )
            .optional()?;

        Ok(record.unwrap_or_default())
    }

    pub fn save_runtime_state(&self, state: &RuntimeStateRecord) -> AppResult<()> {
        let conn = self.open()?;
        let updated_at_unix_ms = if state.updated_at_unix_ms == 0 {
            now_ms()
        } else {
            state.updated_at_unix_ms
        };
        conn.execute(
            "INSERT INTO agent_runtime_state (
                singleton_id, applied_policy_revision, policy_body_json,
                last_successful_send_at_unix_ms, last_known_edge_url,
                identity_status, identity_status_reason,
                degraded_mode, blocked_delivery, blocked_reason,
                spool_enabled, consecutive_send_failures, updated_at_unix_ms
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(singleton_id) DO UPDATE SET
                applied_policy_revision = excluded.applied_policy_revision,
                policy_body_json = excluded.policy_body_json,
                last_successful_send_at_unix_ms = excluded.last_successful_send_at_unix_ms,
                last_known_edge_url = excluded.last_known_edge_url,
                identity_status = excluded.identity_status,
                identity_status_reason = excluded.identity_status_reason,
                degraded_mode = excluded.degraded_mode,
                blocked_delivery = excluded.blocked_delivery,
                blocked_reason = excluded.blocked_reason,
                spool_enabled = excluded.spool_enabled,
                consecutive_send_failures = excluded.consecutive_send_failures,
                updated_at_unix_ms = excluded.updated_at_unix_ms",
            params![
                state.applied_policy_revision,
                state.policy_body_json,
                state.last_successful_send_at_unix_ms,
                state.last_known_edge_url,
                state.identity_status,
                state.identity_status_reason,
                if state.degraded_mode { 1_i64 } else { 0_i64 },
                if state.blocked_delivery { 1_i64 } else { 0_i64 },
                state.blocked_reason,
                if state.spool_enabled { 1_i64 } else { 0_i64 },
                state.consecutive_send_failures,
                updated_at_unix_ms
            ],
        )?;
        Ok(())
    }

    pub fn load_file_offset(&self, path: &Path) -> AppResult<Option<FileOffsetRecord>> {
        let conn = self.open()?;
        let path = path.to_string_lossy().into_owned();
        let record = conn
            .query_row(
                "SELECT path, file_key, read_offset, acked_offset, updated_at_unix_ms
                 FROM file_offsets WHERE path = ?1",
                params![path],
                |row| {
                    Ok(FileOffsetRecord {
                        path: row.get(0)?,
                        file_key: row.get(1)?,
                        durable_read_offset: row.get::<_, u64>(2)?,
                        acked_offset: row.get::<_, u64>(3)?,
                        updated_at_unix_ms: row.get(4)?,
                    })
                },
            )
            .optional()?;
        Ok(record)
    }

    pub fn list_file_offsets(&self) -> AppResult<Vec<FileOffsetRecord>> {
        let conn = self.open()?;
        let mut statement = conn.prepare(
            "SELECT path, file_key, read_offset, acked_offset, updated_at_unix_ms
             FROM file_offsets ORDER BY path ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(FileOffsetRecord {
                path: row.get(0)?,
                file_key: row.get(1)?,
                durable_read_offset: row.get::<_, u64>(2)?,
                acked_offset: row.get::<_, u64>(3)?,
                updated_at_unix_ms: row.get(4)?,
            })
        })?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }

    pub fn commit_file_offsets(&self, updates: &[FileOffsetUpdate]) -> AppResult<()> {
        if updates.is_empty() {
            return Ok(());
        }

        let mut conn = self.open()?;
        let transaction = conn.transaction()?;
        let now = now_ms();
        for update in updates {
            transaction.execute(
                "INSERT INTO file_offsets (path, file_key, read_offset, acked_offset, updated_at_unix_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(path) DO UPDATE SET
                    file_key = excluded.file_key,
                    read_offset = excluded.read_offset,
                    acked_offset = excluded.acked_offset,
                    updated_at_unix_ms = excluded.updated_at_unix_ms",
                params![
                    update.path,
                    update.file_key,
                    update.durable_read_offset,
                    update.acked_offset,
                    now
                ],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn list_spool_batches(&self) -> AppResult<Vec<SpoolBatchRecord>> {
        let conn = self.open()?;
        let mut statement = conn.prepare(
            "SELECT batch_id, payload_path, codec, created_at_unix_ms,
                    attempt_count, next_retry_at_unix_ms, approx_bytes, source_offsets_json
             FROM spool_batches
             ORDER BY created_at_unix_ms ASC",
        )?;
        let rows = statement.query_map([], |row| {
            let source_offsets_json: String = row.get(7)?;
            let source_offsets = serde_json::from_str(&source_offsets_json).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    7,
                    rusqlite::types::Type::Text,
                    Box::new(err),
                )
            })?;
            Ok(SpoolBatchRecord {
                batch_id: row.get(0)?,
                payload_path: PathBuf::from(row.get::<_, String>(1)?),
                codec: row.get(2)?,
                created_at_unix_ms: row.get(3)?,
                attempt_count: row.get::<_, u32>(4)?,
                next_retry_at_unix_ms: row.get(5)?,
                approx_bytes: row.get::<_, i64>(6)? as usize,
                source_offsets,
            })
        })?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }

    pub fn insert_spool_batch(&self, record: &SpoolBatchRecord) -> AppResult<()> {
        let conn = self.open()?;
        let source_offsets_json = serde_json::to_string(&record.source_offsets)?;
        conn.execute(
            "INSERT INTO spool_batches (
                batch_id, payload_path, codec, created_at_unix_ms, attempt_count,
                next_retry_at_unix_ms, approx_bytes, source_offsets_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                record.batch_id,
                record.payload_path.to_string_lossy(),
                record.codec,
                record.created_at_unix_ms,
                record.attempt_count,
                record.next_retry_at_unix_ms,
                record.approx_bytes as i64,
                source_offsets_json
            ],
        )?;
        Ok(())
    }

    pub fn load_due_spool_batch(&self, now_unix_ms: i64) -> AppResult<Option<SpoolBatchRecord>> {
        let conn = self.open()?;
        let record = conn
            .query_row(
                "SELECT batch_id, payload_path, codec, created_at_unix_ms,
                        attempt_count, next_retry_at_unix_ms, approx_bytes, source_offsets_json
                 FROM spool_batches
                 WHERE next_retry_at_unix_ms <= ?1
                 ORDER BY next_retry_at_unix_ms ASC, created_at_unix_ms ASC
                 LIMIT 1",
                params![now_unix_ms],
                |row| {
                    let source_offsets_json: String = row.get(7)?;
                    let source_offsets =
                        serde_json::from_str(&source_offsets_json).map_err(|err| {
                            rusqlite::Error::FromSqlConversionFailure(
                                7,
                                rusqlite::types::Type::Text,
                                Box::new(err),
                            )
                        })?;
                    Ok(SpoolBatchRecord {
                        batch_id: row.get(0)?,
                        payload_path: PathBuf::from(row.get::<_, String>(1)?),
                        codec: row.get(2)?,
                        created_at_unix_ms: row.get(3)?,
                        attempt_count: row.get::<_, u32>(4)?,
                        next_retry_at_unix_ms: row.get(5)?,
                        approx_bytes: row.get::<_, i64>(6)? as usize,
                        source_offsets,
                    })
                },
            )
            .optional()?;
        Ok(record)
    }

    pub fn mark_spool_retry(
        &self,
        batch_id: &str,
        attempt_count: u32,
        next_retry_at_unix_ms: i64,
    ) -> AppResult<()> {
        let conn = self.open()?;
        conn.execute(
            "UPDATE spool_batches
             SET attempt_count = ?2, next_retry_at_unix_ms = ?3
             WHERE batch_id = ?1",
            params![batch_id, attempt_count, next_retry_at_unix_ms],
        )?;
        Ok(())
    }

    pub fn delete_spool_batch(&self, batch_id: &str) -> AppResult<()> {
        let conn = self.open()?;
        conn.execute(
            "DELETE FROM spool_batches WHERE batch_id = ?1",
            params![batch_id],
        )?;
        Ok(())
    }

    pub fn spool_stats(&self) -> AppResult<SpoolStats> {
        let conn = self.open()?;
        let stats = conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(approx_bytes), 0) FROM spool_batches",
            [],
            |row| {
                Ok(SpoolStats {
                    batch_count: row.get::<_, i64>(0)? as usize,
                    total_bytes: row.get::<_, i64>(1)? as u64,
                })
            },
        )?;
        Ok(stats)
    }

    fn init(&self) -> AppResult<()> {
        let conn = self.open()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS agent_identity (
                singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
                agent_id TEXT NOT NULL,
                hostname TEXT NOT NULL,
                version TEXT NOT NULL,
                created_at_unix_ms INTEGER NOT NULL,
                updated_at_unix_ms INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS agent_runtime_state (
                singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
                applied_policy_revision TEXT NULL,
                policy_body_json TEXT NULL,
                last_successful_send_at_unix_ms INTEGER NULL,
                last_known_edge_url TEXT NULL,
                identity_status TEXT NULL,
                identity_status_reason TEXT NULL,
                degraded_mode INTEGER NOT NULL DEFAULT 0,
                blocked_delivery INTEGER NOT NULL DEFAULT 0,
                blocked_reason TEXT NULL,
                spool_enabled INTEGER NOT NULL DEFAULT 1,
                consecutive_send_failures INTEGER NOT NULL DEFAULT 0,
                updated_at_unix_ms INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS file_offsets (
                path TEXT PRIMARY KEY,
                file_key TEXT NULL,
                read_offset INTEGER NOT NULL DEFAULT 0,
                acked_offset INTEGER NOT NULL DEFAULT 0,
                updated_at_unix_ms INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS spool_batches (
                batch_id TEXT PRIMARY KEY,
                payload_path TEXT NOT NULL,
                codec TEXT NOT NULL,
                created_at_unix_ms INTEGER NOT NULL,
                attempt_count INTEGER NOT NULL,
                next_retry_at_unix_ms INTEGER NOT NULL,
                approx_bytes INTEGER NOT NULL,
                source_offsets_json TEXT NOT NULL
             );",
        )?;
        self.migrate_runtime_state(&conn)?;
        self.migrate_file_offsets(&conn)?;
        Ok(())
    }

    fn migrate_runtime_state(&self, conn: &Connection) -> AppResult<()> {
        for (column, definition) in [
            ("identity_status", "TEXT NULL"),
            ("identity_status_reason", "TEXT NULL"),
            ("degraded_mode", "INTEGER NOT NULL DEFAULT 0"),
            ("blocked_delivery", "INTEGER NOT NULL DEFAULT 0"),
            ("blocked_reason", "TEXT NULL"),
            ("spool_enabled", "INTEGER NOT NULL DEFAULT 1"),
            ("consecutive_send_failures", "INTEGER NOT NULL DEFAULT 0"),
        ] {
            self.ensure_column(conn, "agent_runtime_state", column, definition)?;
        }
        Ok(())
    }

    fn migrate_file_offsets(&self, conn: &Connection) -> AppResult<()> {
        let columns = table_columns(conn, "file_offsets")?;
        let has_legacy_offset = columns.iter().any(|column| column == "offset");
        let has_read_offset = columns.iter().any(|column| column == "read_offset");
        let has_acked_offset = columns.iter().any(|column| column == "acked_offset");

        if has_legacy_offset || !has_read_offset || !has_acked_offset {
            let read_expr = if has_read_offset {
                "read_offset"
            } else if has_legacy_offset {
                "offset"
            } else if has_acked_offset {
                "acked_offset"
            } else {
                "0"
            };
            let acked_expr = if has_acked_offset {
                "acked_offset"
            } else if has_legacy_offset {
                "offset"
            } else if has_read_offset {
                "read_offset"
            } else {
                "0"
            };
            conn.execute_batch(&format!(
                "ALTER TABLE file_offsets RENAME TO file_offsets_legacy;
                 CREATE TABLE file_offsets (
                    path TEXT PRIMARY KEY,
                    file_key TEXT NULL,
                    read_offset INTEGER NOT NULL DEFAULT 0,
                    acked_offset INTEGER NOT NULL DEFAULT 0,
                    updated_at_unix_ms INTEGER NOT NULL
                 );
                 INSERT INTO file_offsets (path, file_key, read_offset, acked_offset, updated_at_unix_ms)
                 SELECT
                    path,
                    file_key,
                    {read_expr},
                    {acked_expr},
                    updated_at_unix_ms
                 FROM file_offsets_legacy;
                 DROP TABLE file_offsets_legacy;"
            ))?;
        }
        Ok(())
    }

    fn ensure_column(
        &self,
        conn: &Connection,
        table: &str,
        column: &str,
        definition: &str,
    ) -> AppResult<()> {
        let columns = table_columns(conn, table)?;
        if columns.iter().any(|existing| existing == column) {
            return Ok(());
        }
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
            [],
        )?;
        Ok(())
    }

    fn open(&self) -> AppResult<Connection> {
        let conn = Connection::open(&self.db_path)?;
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )?;
        Ok(conn)
    }
}

fn table_columns(conn: &Connection, table: &str) -> AppResult<Vec<String>> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    let mut columns = Vec::new();
    for row in rows {
        columns.push(row?);
    }
    if columns.is_empty() {
        return Err(AppError::protocol(format!(
            "table `{table}` is missing from sqlite schema"
        )));
    }
    Ok(columns)
}

fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use tempfile::TempDir;

    use super::{RuntimeStateRecord, SqliteStateStore};
    use crate::state::{FileOffsetUpdate, SourceOffsetMarker, SpoolBatchRecord};

    #[test]
    fn migrates_legacy_offset_schema() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("state.db");
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE file_offsets (
                path TEXT PRIMARY KEY,
                file_key TEXT NULL,
                offset INTEGER NOT NULL,
                updated_at_unix_ms INTEGER NOT NULL
             );
             INSERT INTO file_offsets(path, file_key, offset, updated_at_unix_ms)
             VALUES ('/tmp/demo.log', '1:2', 128, 10);",
        )
        .unwrap();
        drop(conn);

        let store = SqliteStateStore::new(dir.path()).unwrap();
        let record = store
            .load_file_offset(std::path::Path::new("/tmp/demo.log"))
            .unwrap()
            .unwrap();

        assert_eq!(record.durable_read_offset, 128);
        assert_eq!(record.acked_offset, 128);
    }

    #[test]
    fn persists_identity_offsets_and_runtime_state() {
        let dir = TempDir::new().unwrap();
        let store = SqliteStateStore::new(dir.path()).unwrap();
        store
            .save_identity("agent-1", "demo-host", "0.1.0")
            .unwrap();
        store
            .commit_file_offsets(&[FileOffsetUpdate {
                path: "/tmp/demo.log".to_string(),
                file_key: Some("1:2".to_string()),
                durable_read_offset: 128,
                acked_offset: 64,
            }])
            .unwrap();
        store
            .save_runtime_state(&RuntimeStateRecord {
                applied_policy_revision: Some("rev-2".to_string()),
                policy_body_json: Some("{\"sources\":[]}".to_string()),
                last_successful_send_at_unix_ms: Some(42),
                last_known_edge_url: Some("https://edge.local".to_string()),
                identity_status: Some("reused".to_string()),
                identity_status_reason: Some("persisted identity accepted".to_string()),
                degraded_mode: true,
                blocked_delivery: true,
                blocked_reason: Some("permanent transport failure".to_string()),
                spool_enabled: true,
                consecutive_send_failures: 3,
                updated_at_unix_ms: 77,
            })
            .unwrap();

        let reopened = SqliteStateStore::new(dir.path()).unwrap();
        let identity = reopened.load_identity().unwrap().unwrap();
        let offset = reopened
            .load_file_offset(std::path::Path::new("/tmp/demo.log"))
            .unwrap()
            .unwrap();
        let runtime = reopened.load_runtime_state().unwrap();

        assert_eq!(identity.agent_id, "agent-1");
        assert_eq!(offset.durable_read_offset, 128);
        assert_eq!(offset.acked_offset, 64);
        assert!(runtime.degraded_mode);
        assert!(runtime.blocked_delivery);
        assert_eq!(runtime.identity_status.as_deref(), Some("reused"));
        assert_eq!(
            runtime.identity_status_reason.as_deref(),
            Some("persisted identity accepted")
        );
        assert_eq!(
            runtime.blocked_reason.as_deref(),
            Some("permanent transport failure")
        );
        assert_eq!(runtime.consecutive_send_failures, 3);
    }

    #[test]
    fn stores_and_reads_spool_metadata() {
        let dir = TempDir::new().unwrap();
        let store = SqliteStateStore::new(dir.path()).unwrap();
        let record = SpoolBatchRecord {
            batch_id: "batch-1".to_string(),
            payload_path: dir.path().join("spool").join("batch-1.bin"),
            codec: "identity".to_string(),
            created_at_unix_ms: 10,
            attempt_count: 0,
            next_retry_at_unix_ms: 20,
            approx_bytes: 512,
            source_offsets: vec![SourceOffsetMarker {
                source_id: "file:/tmp/demo.log".to_string(),
                path: "/tmp/demo.log".to_string(),
                file_key: Some("1:2".to_string()),
                offset: 50,
            }],
        };

        store.insert_spool_batch(&record).unwrap();
        let loaded = store.load_due_spool_batch(20).unwrap().unwrap();
        let stats = store.spool_stats().unwrap();

        assert_eq!(loaded.batch_id, "batch-1");
        assert_eq!(loaded.source_offsets[0].offset, 50);
        assert_eq!(stats.batch_count, 1);
        assert_eq!(stats.total_bytes, 512);
    }
}
