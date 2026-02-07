//! Storage module for MacAgentWatch
//!
//! Handles session-based log file storage with JSON Lines format.
//! Each monitoring session creates a new log file.

use crate::error::{CoreError, StorageError};
use crate::event::Event;
use chrono::{DateTime, Utc};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

/// Trait for event storage implementations
pub trait EventStorage: Send {
    /// Write an event to storage
    fn write_event(&mut self, event: &Event) -> Result<(), CoreError>;
    /// Flush buffered data to disk
    fn flush(&mut self) -> Result<(), CoreError>;
    /// Get the storage file path
    fn path(&self) -> &PathBuf;
}

/// Session-based log file writer
///
/// Creates a new log file for each monitoring session.
/// Format: `session-{timestamp}-{uuid}.jsonl`
pub struct SessionLogger {
    session_id: String,
    session_start: DateTime<Utc>,
    file_path: PathBuf,
    writer: BufWriter<File>,
    event_count: usize,
}

impl SessionLogger {
    /// Create a new session logger
    ///
    /// # Arguments
    /// * `log_dir` - Directory to store log files
    /// * `session_id` - Optional custom session ID (auto-generated if None)
    pub fn new(log_dir: &PathBuf, session_id: Option<String>) -> Result<Self, CoreError> {
        // Ensure log directory exists
        if !log_dir.exists() {
            std::fs::create_dir_all(log_dir).map_err(|e| StorageError::CreateDir {
                path: log_dir.clone(),
                source: e,
            })?;
        }

        let session_start = Utc::now();
        let session_id = session_id.unwrap_or_else(|| {
            format!(
                "{}-{}",
                session_start.format("%Y%m%d-%H%M%S"),
                &uuid::Uuid::new_v4().to_string()[..8]
            )
        });

        let filename = format!("session-{}.jsonl", session_id);
        let file_path = log_dir.join(&filename);

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .map_err(|e| StorageError::OpenFile {
                path: file_path.clone(),
                source: e,
            })?;

        let writer = BufWriter::new(file);

        Ok(Self {
            session_id,
            session_start,
            file_path,
            writer,
            event_count: 0,
        })
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get session start time
    pub fn session_start(&self) -> DateTime<Utc> {
        self.session_start
    }

    /// Get number of events written
    pub fn event_count(&self) -> usize {
        self.event_count
    }

    /// Write session metadata as first line
    pub fn write_session_header(&mut self, process: &str, pid: u32) -> Result<(), CoreError> {
        let header = serde_json::json!({
            "session_id": self.session_id,
            "session_start": self.session_start.to_rfc3339(),
            "process": process,
            "pid": pid,
            "type": "session_start"
        });
        writeln!(self.writer, "{}", header).map_err(StorageError::Write)?;
        self.flush()?;
        Ok(())
    }

    /// Write session end marker
    pub fn write_session_footer(&mut self, exit_code: Option<i32>) -> Result<(), CoreError> {
        let footer = serde_json::json!({
            "session_id": self.session_id,
            "session_end": Utc::now().to_rfc3339(),
            "event_count": self.event_count,
            "exit_code": exit_code,
            "type": "session_end"
        });
        writeln!(self.writer, "{}", footer).map_err(StorageError::Write)?;
        self.flush()?;
        Ok(())
    }
}

impl EventStorage for SessionLogger {
    fn write_event(&mut self, event: &Event) -> Result<(), CoreError> {
        let json = serde_json::to_string(event).map_err(StorageError::Serialize)?;
        writeln!(self.writer, "{}", json).map_err(StorageError::Write)?;
        self.event_count += 1;
        Ok(())
    }

    fn flush(&mut self) -> Result<(), CoreError> {
        self.writer.flush().map_err(StorageError::Flush)?;
        Ok(())
    }

    fn path(&self) -> &PathBuf {
        &self.file_path
    }
}

impl Drop for SessionLogger {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

/// Clean up old log files based on retention policy
pub fn cleanup_old_logs(log_dir: &PathBuf, retention_days: u32) -> Result<usize, CoreError> {
    if retention_days == 0 {
        return Ok(0);
    }

    let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
    let mut removed = 0;

    if !log_dir.exists() {
        return Ok(0);
    }

    for entry in std::fs::read_dir(log_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("jsonl")
            && let Ok(metadata) = entry.metadata()
            && let Ok(modified) = metadata.modified()
        {
            let modified: DateTime<Utc> = modified.into();
            if modified < cutoff && std::fs::remove_file(&path).is_ok() {
                removed += 1;
            }
        }
    }

    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventType, RiskLevel};
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
    fn test_session_logger_creation() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().to_path_buf();

        let logger = SessionLogger::new(&log_dir, None).unwrap();

        assert!(logger.path().exists());
        assert!(logger.path().to_string_lossy().contains("session-"));
        assert!(logger.path().to_string_lossy().ends_with(".jsonl"));
        assert_eq!(logger.event_count(), 0);
    }

    #[test]
    fn test_session_logger_custom_id() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().to_path_buf();

        let logger = SessionLogger::new(&log_dir, Some("test-session-123".to_string())).unwrap();

        assert_eq!(logger.session_id(), "test-session-123");
        assert!(logger.path().to_string_lossy().contains("test-session-123"));
    }

    #[test]
    fn test_session_logger_write_event() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().to_path_buf();

        let mut logger = SessionLogger::new(&log_dir, Some("test".to_string())).unwrap();
        let event = create_test_event();

        logger.write_event(&event).unwrap();
        logger.flush().unwrap();

        assert_eq!(logger.event_count(), 1);

        let content = std::fs::read_to_string(logger.path()).unwrap();
        assert!(content.contains("\"type\":\"command\""));
        assert!(content.contains("\"command\":\"ls\""));
    }

    #[test]
    fn test_session_logger_multiple_events() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().to_path_buf();

        let mut logger = SessionLogger::new(&log_dir, Some("multi".to_string())).unwrap();

        for i in 0..5 {
            let event = Event::command(
                format!("cmd{}", i),
                vec![],
                "bash".to_string(),
                1234,
                RiskLevel::Low,
            );
            logger.write_event(&event).unwrap();
        }
        logger.flush().unwrap();

        assert_eq!(logger.event_count(), 5);

        let content = std::fs::read_to_string(logger.path()).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 5);
    }

    #[test]
    fn test_session_header_footer() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().to_path_buf();

        let mut logger = SessionLogger::new(&log_dir, Some("hf-test".to_string())).unwrap();

        logger.write_session_header("test-process", 9999).unwrap();
        logger.write_event(&create_test_event()).unwrap();
        logger.write_session_footer(Some(0)).unwrap();

        let content = std::fs::read_to_string(logger.path()).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("\"type\":\"session_start\""));
        assert!(lines[0].contains("\"process\":\"test-process\""));
        assert!(lines[2].contains("\"type\":\"session_end\""));
        assert!(lines[2].contains("\"exit_code\":0"));
    }

    #[test]
    fn test_creates_log_directory() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().join("nested").join("logs");

        assert!(!log_dir.exists());

        let logger = SessionLogger::new(&log_dir, None).unwrap();

        assert!(log_dir.exists());
        assert!(logger.path().exists());
    }

    #[test]
    fn test_cleanup_old_logs() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().to_path_buf();

        // Create some test log files
        for i in 0..3 {
            let filename = format!("session-test-{}.jsonl", i);
            std::fs::write(log_dir.join(&filename), "test content").unwrap();
        }

        // With 0 retention, nothing should be deleted
        let removed = cleanup_old_logs(&log_dir, 0).unwrap();
        assert_eq!(removed, 0);

        // Files are new, so they shouldn't be deleted with retention
        let removed = cleanup_old_logs(&log_dir, 30).unwrap();
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_event_storage_trait() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().to_path_buf();

        let mut logger = SessionLogger::new(&log_dir, None).unwrap();

        // Use trait methods
        let event = create_test_event();
        logger.write_event(&event).unwrap();
        logger.flush().unwrap();

        assert!(logger.path().exists());
    }

    #[test]
    fn test_jsonl_format_validity() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().to_path_buf();

        let mut logger = SessionLogger::new(&log_dir, Some("json-test".to_string())).unwrap();

        let events = vec![
            Event::command(
                "ls".to_string(),
                vec![],
                "bash".to_string(),
                1,
                RiskLevel::Low,
            ),
            Event::command(
                "rm".to_string(),
                vec!["-rf".to_string()],
                "bash".to_string(),
                2,
                RiskLevel::High,
            ),
            Event::new(
                EventType::FileAccess {
                    path: PathBuf::from("/tmp/test.txt"),
                    action: crate::event::FileAction::Read,
                },
                "cat".to_string(),
                3,
                RiskLevel::Low,
            ),
        ];

        for event in &events {
            logger.write_event(event).unwrap();
        }
        logger.flush().unwrap();

        // Verify each line is valid JSON
        let content = std::fs::read_to_string(logger.path()).unwrap();
        for line in content.lines() {
            let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
            assert!(parsed.is_object());
        }
    }
}
