//! SQLite-based event storage for MacAgentWatch
//!
//! Provides a SQLite backend implementing the `EventStorage` trait,
//! offering structured queries over events alongside the existing JSONL logger.

use crate::error::{CoreError, StorageError};
use crate::event::{Event, RiskLevel};
use crate::storage::EventStorage;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::PathBuf;

/// SQLite-backed event storage
pub struct SqliteStorage {
    conn: Connection,
    db_path: PathBuf,
    event_count: usize,
}

/// Filters for querying events
#[derive(Debug, Default)]
pub struct EventQuery {
    pub session_id: Option<String>,
    pub risk_level: Option<RiskLevel>,
    pub event_type: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

impl SqliteStorage {
    /// Create a new SQLite storage, opening or creating the database at `db_path`.
    pub fn new(db_path: &PathBuf) -> Result<Self, CoreError> {
        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| StorageError::CreateDir {
                    path: parent.to_path_buf(),
                    source: e,
                })?;
            }
        }
        let conn = Connection::open(db_path).map_err(StorageError::Sqlite)?;
        let mut storage = Self {
            conn,
            db_path: db_path.clone(),
            event_count: 0,
        };
        storage.init_schema()?;
        Ok(storage)
    }

    /// Create an in-memory SQLite storage (useful for testing).
    #[cfg(test)]
    pub fn in_memory() -> Result<Self, CoreError> {
        let conn = Connection::open_in_memory().map_err(StorageError::Sqlite)?;
        let mut storage = Self {
            conn,
            db_path: PathBuf::from(":memory:"),
            event_count: 0,
        };
        storage.init_schema()?;
        Ok(storage)
    }

    /// Initialize the database schema (tables and indexes).
    fn init_schema(&mut self) -> Result<(), CoreError> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS events (
                    id TEXT PRIMARY KEY,
                    session_id TEXT,
                    timestamp TEXT NOT NULL,
                    event_type TEXT NOT NULL,
                    event_data TEXT NOT NULL,
                    process TEXT NOT NULL,
                    pid INTEGER NOT NULL,
                    risk_level TEXT NOT NULL,
                    alert INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_events_session ON events(session_id);
                CREATE INDEX IF NOT EXISTS idx_events_risk ON events(risk_level);
                CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);

                CREATE TABLE IF NOT EXISTS sessions (
                    session_id TEXT PRIMARY KEY,
                    process_name TEXT,
                    pid INTEGER,
                    start_time TEXT,
                    end_time TEXT
                );",
            )
            .map_err(StorageError::Sqlite)?;
        Ok(())
    }

    /// Write a session start record.
    pub fn write_session_header(
        &self,
        session_id: &str,
        process: &str,
        pid: u32,
        start_time: &DateTime<Utc>,
    ) -> Result<(), CoreError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO sessions (session_id, process_name, pid, start_time) VALUES (?1, ?2, ?3, ?4)",
                params![session_id, process, pid, start_time.to_rfc3339()],
            )
            .map_err(StorageError::Sqlite)?;
        Ok(())
    }

    /// Write a session end record.
    pub fn write_session_footer(
        &self,
        session_id: &str,
        end_time: &DateTime<Utc>,
    ) -> Result<(), CoreError> {
        self.conn
            .execute(
                "UPDATE sessions SET end_time = ?1 WHERE session_id = ?2",
                params![end_time.to_rfc3339(), session_id],
            )
            .map_err(StorageError::Sqlite)?;
        Ok(())
    }

    /// Query events with optional filters.
    pub fn query_events(&self, query: &EventQuery) -> Result<Vec<Event>, CoreError> {
        let mut sql = String::from("SELECT event_data FROM events WHERE 1=1");
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref session_id) = query.session_id {
            sql.push_str(" AND session_id = ?");
            param_values.push(Box::new(session_id.clone()));
        }
        if let Some(ref risk_level) = query.risk_level {
            sql.push_str(" AND risk_level = ?");
            param_values.push(Box::new(risk_level.to_string()));
        }
        if let Some(ref event_type) = query.event_type {
            sql.push_str(" AND event_type = ?");
            param_values.push(Box::new(event_type.clone()));
        }
        if let Some(ref start_time) = query.start_time {
            sql.push_str(" AND timestamp >= ?");
            param_values.push(Box::new(start_time.to_rfc3339()));
        }
        if let Some(ref end_time) = query.end_time {
            sql.push_str(" AND timestamp <= ?");
            param_values.push(Box::new(end_time.to_rfc3339()));
        }

        sql.push_str(" ORDER BY timestamp ASC");

        if let Some(limit) = query.limit {
            sql.push_str(&format!(" LIMIT {limit}"));
        }

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = self.conn.prepare(&sql).map_err(StorageError::Sqlite)?;
        let rows = stmt
            .query_map(params_refs.as_slice(), |row| {
                let json_str: String = row.get(0)?;
                Ok(json_str)
            })
            .map_err(StorageError::Sqlite)?;

        let mut events = Vec::new();
        for row in rows {
            let json_str = row.map_err(StorageError::Sqlite)?;
            let event: Event = serde_json::from_str(&json_str).map_err(StorageError::Serialize)?;
            events.push(event);
        }
        Ok(events)
    }

    /// Get the number of events written in this session.
    pub fn event_count(&self) -> usize {
        self.event_count
    }

    /// Extract the event type tag from an Event for indexing.
    fn event_type_tag(event: &Event) -> &'static str {
        match &event.event_type {
            crate::event::EventType::Command { .. } => "command",
            crate::event::EventType::FileAccess { .. } => "file_access",
            crate::event::EventType::Network { .. } => "network",
            crate::event::EventType::Process { .. } => "process",
            crate::event::EventType::Session { .. } => "session",
        }
    }
}

impl EventStorage for SqliteStorage {
    fn write_event(&mut self, event: &Event) -> Result<(), CoreError> {
        let event_data = serde_json::to_string(event).map_err(StorageError::Serialize)?;
        let event_type_tag = Self::event_type_tag(event);

        self.conn
            .execute(
                "INSERT INTO events (id, session_id, timestamp, event_type, event_data, process, pid, risk_level, alert)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    event.id.to_string(),
                    Option::<String>::None,
                    event.timestamp.to_rfc3339(),
                    event_type_tag,
                    event_data,
                    event.process,
                    event.pid,
                    event.risk_level.to_string(),
                    event.alert as i32,
                ],
            )
            .map_err(StorageError::Sqlite)?;
        self.event_count += 1;
        Ok(())
    }

    fn flush(&mut self) -> Result<(), CoreError> {
        // SQLite auto-commits; no buffering to flush.
        Ok(())
    }

    fn path(&self) -> &PathBuf {
        &self.db_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventType, FileAction, RiskLevel};
    use tempfile::TempDir;

    fn create_test_event() -> Event {
        Event::command(
            "ls".to_string(),
            vec!["-la".to_string()],
            "bash".to_string(),
            1234,
            RiskLevel::Low,
        )
    }

    #[test]
    fn test_sqlite_storage_creation_file() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let storage = SqliteStorage::new(&db_path).unwrap();
        assert!(db_path.exists());
        assert_eq!(storage.event_count(), 0);
        assert_eq!(storage.path(), &db_path);
    }

    #[test]
    fn test_sqlite_storage_in_memory() {
        let storage = SqliteStorage::in_memory().unwrap();
        assert_eq!(storage.event_count(), 0);
    }

    #[test]
    fn test_sqlite_write_event() {
        let mut storage = SqliteStorage::in_memory().unwrap();
        let event = create_test_event();

        storage.write_event(&event).unwrap();
        assert_eq!(storage.event_count(), 1);

        // Verify event can be read back
        let events = storage.query_events(&EventQuery::default()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].process, "bash");
        assert_eq!(events[0].pid, 1234);
    }

    #[test]
    fn test_sqlite_write_multiple_events() {
        let mut storage = SqliteStorage::in_memory().unwrap();

        for i in 0..5 {
            let event = Event::command(
                format!("cmd{i}"),
                vec![],
                "bash".to_string(),
                1234,
                RiskLevel::Low,
            );
            storage.write_event(&event).unwrap();
        }

        assert_eq!(storage.event_count(), 5);

        let events = storage.query_events(&EventQuery::default()).unwrap();
        assert_eq!(events.len(), 5);
    }

    #[test]
    fn test_sqlite_query_by_risk_level() {
        let mut storage = SqliteStorage::in_memory().unwrap();

        storage
            .write_event(&Event::command(
                "ls".into(),
                vec![],
                "bash".into(),
                1,
                RiskLevel::Low,
            ))
            .unwrap();
        storage
            .write_event(&Event::command(
                "rm".into(),
                vec!["-rf".into()],
                "bash".into(),
                2,
                RiskLevel::High,
            ))
            .unwrap();
        storage
            .write_event(&Event::command(
                "cat".into(),
                vec![],
                "bash".into(),
                3,
                RiskLevel::Low,
            ))
            .unwrap();

        let query = EventQuery {
            risk_level: Some(RiskLevel::High),
            ..Default::default()
        };
        let high_events = storage.query_events(&query).unwrap();
        assert_eq!(high_events.len(), 1);
        assert_eq!(high_events[0].risk_level, RiskLevel::High);

        let query = EventQuery {
            risk_level: Some(RiskLevel::Low),
            ..Default::default()
        };
        let low_events = storage.query_events(&query).unwrap();
        assert_eq!(low_events.len(), 2);
    }

    #[test]
    fn test_sqlite_query_by_event_type() {
        let mut storage = SqliteStorage::in_memory().unwrap();

        storage
            .write_event(&Event::command(
                "ls".into(),
                vec![],
                "bash".into(),
                1,
                RiskLevel::Low,
            ))
            .unwrap();
        storage
            .write_event(&Event::new(
                EventType::FileAccess {
                    path: PathBuf::from("/tmp/test.txt"),
                    action: FileAction::Read,
                },
                "cat".into(),
                2,
                RiskLevel::Low,
            ))
            .unwrap();

        let query = EventQuery {
            event_type: Some("command".into()),
            ..Default::default()
        };
        let cmd_events = storage.query_events(&query).unwrap();
        assert_eq!(cmd_events.len(), 1);

        let query = EventQuery {
            event_type: Some("file_access".into()),
            ..Default::default()
        };
        let fs_events = storage.query_events(&query).unwrap();
        assert_eq!(fs_events.len(), 1);
    }

    #[test]
    fn test_sqlite_query_with_limit() {
        let mut storage = SqliteStorage::in_memory().unwrap();

        for i in 0..10 {
            storage
                .write_event(&Event::command(
                    format!("cmd{i}"),
                    vec![],
                    "bash".into(),
                    1234,
                    RiskLevel::Low,
                ))
                .unwrap();
        }

        let query = EventQuery {
            limit: Some(3),
            ..Default::default()
        };
        let events = storage.query_events(&query).unwrap();
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn test_sqlite_session_lifecycle() {
        let storage = SqliteStorage::in_memory().unwrap();
        let now = Utc::now();

        storage
            .write_session_header("sess-001", "claude-code", 5678, &now)
            .unwrap();

        // Verify session was inserted
        let process_name: String = storage
            .conn
            .query_row(
                "SELECT process_name FROM sessions WHERE session_id = ?1",
                params!["sess-001"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(process_name, "claude-code");

        // Write footer
        let end_time = Utc::now();
        storage.write_session_footer("sess-001", &end_time).unwrap();

        let end_str: String = storage
            .conn
            .query_row(
                "SELECT end_time FROM sessions WHERE session_id = ?1",
                params!["sess-001"],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!end_str.is_empty());
    }

    #[test]
    fn test_sqlite_flush_is_noop() {
        let mut storage = SqliteStorage::in_memory().unwrap();
        // flush should succeed without error
        storage.flush().unwrap();
    }

    #[test]
    fn test_sqlite_creates_parent_dir() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("nested").join("dir").join("test.db");

        assert!(!db_path.parent().unwrap().exists());

        let storage = SqliteStorage::new(&db_path).unwrap();
        assert!(db_path.exists());
        assert_eq!(storage.event_count(), 0);
    }

    #[test]
    fn test_sqlite_event_roundtrip_fidelity() {
        let mut storage = SqliteStorage::in_memory().unwrap();

        let original = Event::command(
            "rm".into(),
            vec!["-rf".into(), "/tmp/test".into()],
            "zsh".into(),
            9999,
            RiskLevel::Critical,
        );
        let original_id = original.id;
        let original_timestamp = original.timestamp;

        storage.write_event(&original).unwrap();

        let events = storage.query_events(&EventQuery::default()).unwrap();
        assert_eq!(events.len(), 1);

        let recovered = &events[0];
        assert_eq!(recovered.id, original_id);
        assert_eq!(recovered.timestamp, original_timestamp);
        assert_eq!(recovered.process, "zsh");
        assert_eq!(recovered.pid, 9999);
        assert_eq!(recovered.risk_level, RiskLevel::Critical);
        assert!(recovered.alert);
        match &recovered.event_type {
            EventType::Command { command, args, .. } => {
                assert_eq!(command, "rm");
                assert_eq!(args, &vec!["-rf".to_string(), "/tmp/test".to_string()]);
            }
            _ => panic!("Expected Command event type"),
        }
    }

    #[test]
    fn test_event_storage_trait_compliance() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("trait_test.db");

        let mut storage = SqliteStorage::new(&db_path).unwrap();

        // Use only trait methods
        let event = create_test_event();
        EventStorage::write_event(&mut storage, &event).unwrap();
        EventStorage::flush(&mut storage).unwrap();
        let _ = EventStorage::path(&storage);

        assert_eq!(storage.event_count(), 1);
    }
}
