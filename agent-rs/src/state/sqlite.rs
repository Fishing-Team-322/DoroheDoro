use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::AppResult;

use super::{AgentIdentity, FileOffsetRecord, FileOffsetUpdate, RuntimeStateRecord};

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
                        last_successful_send_at_unix_ms, last_known_edge_url, updated_at_unix_ms
                 FROM agent_runtime_state WHERE singleton_id = 1",
                [],
                |row| {
                    Ok(RuntimeStateRecord {
                        applied_policy_revision: row.get(0)?,
                        policy_body_json: row.get(1)?,
                        last_successful_send_at_unix_ms: row.get(2)?,
                        last_known_edge_url: row.get(3)?,
                        updated_at_unix_ms: row.get(4)?,
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
                last_successful_send_at_unix_ms, last_known_edge_url, updated_at_unix_ms
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(singleton_id) DO UPDATE SET
                applied_policy_revision = excluded.applied_policy_revision,
                policy_body_json = excluded.policy_body_json,
                last_successful_send_at_unix_ms = excluded.last_successful_send_at_unix_ms,
                last_known_edge_url = excluded.last_known_edge_url,
                updated_at_unix_ms = excluded.updated_at_unix_ms",
            params![
                state.applied_policy_revision,
                state.policy_body_json,
                state.last_successful_send_at_unix_ms,
                state.last_known_edge_url,
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
                "SELECT path, file_key, offset, updated_at_unix_ms
                 FROM file_offsets WHERE path = ?1",
                params![path],
                |row| {
                    Ok(FileOffsetRecord {
                        path: row.get(0)?,
                        file_key: row.get(1)?,
                        offset: row.get::<_, u64>(2)?,
                        updated_at_unix_ms: row.get(3)?,
                    })
                },
            )
            .optional()?;
        Ok(record)
    }

    pub fn list_file_offsets(&self) -> AppResult<Vec<FileOffsetRecord>> {
        let conn = self.open()?;
        let mut statement = conn.prepare(
            "SELECT path, file_key, offset, updated_at_unix_ms
             FROM file_offsets ORDER BY path ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(FileOffsetRecord {
                path: row.get(0)?,
                file_key: row.get(1)?,
                offset: row.get::<_, u64>(2)?,
                updated_at_unix_ms: row.get(3)?,
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
                "INSERT INTO file_offsets (path, file_key, offset, updated_at_unix_ms)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(path) DO UPDATE SET
                    file_key = excluded.file_key,
                    offset = excluded.offset,
                    updated_at_unix_ms = excluded.updated_at_unix_ms",
                params![update.path, update.file_key, update.offset, now],
            )?;
        }
        transaction.commit()?;
        Ok(())
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
                updated_at_unix_ms INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS file_offsets (
                path TEXT PRIMARY KEY,
                file_key TEXT NULL,
                offset INTEGER NOT NULL,
                updated_at_unix_ms INTEGER NOT NULL
             );",
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

fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::{RuntimeStateRecord, SqliteStateStore};
    use crate::state::FileOffsetUpdate;

    #[test]
    fn persists_identity_and_offsets_across_reopen() {
        let dir = TempDir::new().unwrap();
        let store = SqliteStateStore::new(dir.path()).unwrap();
        store
            .save_identity("agent-1", "demo-host", "0.1.0")
            .unwrap();
        store
            .commit_file_offsets(&[FileOffsetUpdate {
                path: "/tmp/demo.log".to_string(),
                file_key: Some("1:2".to_string()),
                offset: 128,
            }])
            .unwrap();

        let reopened = SqliteStateStore::new(dir.path()).unwrap();
        let identity = reopened.load_identity().unwrap().unwrap();
        let offset = reopened
            .load_file_offset(std::path::Path::new("/tmp/demo.log"))
            .unwrap()
            .unwrap();

        assert_eq!(identity.agent_id, "agent-1");
        assert_eq!(offset.offset, 128);
        assert_eq!(offset.file_key.as_deref(), Some("1:2"));
    }

    #[test]
    fn saves_runtime_state() {
        let dir = TempDir::new().unwrap();
        let store = SqliteStateStore::new(dir.path()).unwrap();
        let state = RuntimeStateRecord {
            applied_policy_revision: Some("rev-2".to_string()),
            policy_body_json: Some("{\"sources\":[]}".to_string()),
            last_successful_send_at_unix_ms: Some(42),
            last_known_edge_url: Some("https://edge.local".to_string()),
            updated_at_unix_ms: 77,
        };

        store.save_runtime_state(&state).unwrap();
        let loaded = store.load_runtime_state().unwrap();

        assert_eq!(loaded.applied_policy_revision.as_deref(), Some("rev-2"));
        assert_eq!(
            loaded.last_known_edge_url.as_deref(),
            Some("https://edge.local")
        );
    }
}
