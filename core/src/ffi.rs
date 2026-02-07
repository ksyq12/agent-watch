//! FFI module for UniFFI Swift bindings
//!
//! Provides FFI-safe types, conversions, and exported functions for the Swift app layer.

use crate::config::Config;
use crate::error::CoreError;
use crate::event::{Event, EventType, FileAction, ProcessAction, RiskLevel, SessionAction};
use crate::risk::RiskScorer;
use crate::storage::SessionLogger;
use std::io::BufRead;
use std::sync::Mutex;

// ─── FFI Enum Types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiRiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiFileAction {
    Read,
    Write,
    Delete,
    Create,
    Chmod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiProcessAction {
    Start,
    Exit,
    Fork,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiSessionAction {
    Start,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum FfiEventType {
    Command {
        command: String,
        args: Vec<String>,
        exit_code: Option<i32>,
    },
    FileAccess {
        path: String,
        action: FfiFileAction,
    },
    Network {
        host: String,
        port: u16,
        protocol: String,
    },
    Process {
        pid: u32,
        ppid: Option<u32>,
        action: FfiProcessAction,
    },
    Session {
        action: FfiSessionAction,
    },
}

// ─── FFI Record Types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiEvent {
    pub id: String,
    pub timestamp_ms: i64,
    pub timestamp_str: String,
    pub event_type: FfiEventType,
    pub process: String,
    pub pid: u32,
    pub risk_level: FfiRiskLevel,
    pub alert: bool,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiGeneralConfig {
    pub verbose: bool,
    pub default_format: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiLoggingConfig {
    pub enabled: bool,
    pub log_dir: Option<String>,
    pub retention_days: u32,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiMonitoringConfig {
    pub fs_enabled: bool,
    pub net_enabled: bool,
    pub track_children: bool,
    pub tracking_poll_ms: u64,
    pub fs_debounce_ms: u64,
    pub net_poll_ms: u64,
    pub watch_paths: Vec<String>,
    pub sensitive_patterns: Vec<String>,
    pub network_whitelist: Vec<String>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiAlertConfig {
    pub min_level: String,
    pub custom_high_risk: Vec<String>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiConfig {
    pub general: FfiGeneralConfig,
    pub logging: FfiLoggingConfig,
    pub monitoring: FfiMonitoringConfig,
    pub alerts: FfiAlertConfig,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiActivitySummary {
    pub total_events: u32,
    pub critical_count: u32,
    pub high_count: u32,
    pub medium_count: u32,
    pub low_count: u32,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiSessionInfo {
    pub session_id: String,
    pub file_path: String,
    pub start_time_str: String,
}

// ─── FFI Error Type ───────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FfiError {
    #[error("Config error: {message}")]
    Config { message: String },
    #[error("Storage error: {message}")]
    Storage { message: String },
    #[error("IO error: {message}")]
    Io { message: String },
    #[error("{message}")]
    Other { message: String },
}

// ─── From Conversions ─────────────────────────────────────────────────────────

impl From<RiskLevel> for FfiRiskLevel {
    fn from(level: RiskLevel) -> Self {
        match level {
            RiskLevel::Low => FfiRiskLevel::Low,
            RiskLevel::Medium => FfiRiskLevel::Medium,
            RiskLevel::High => FfiRiskLevel::High,
            RiskLevel::Critical => FfiRiskLevel::Critical,
        }
    }
}

impl From<FileAction> for FfiFileAction {
    fn from(action: FileAction) -> Self {
        match action {
            FileAction::Read => FfiFileAction::Read,
            FileAction::Write => FfiFileAction::Write,
            FileAction::Delete => FfiFileAction::Delete,
            FileAction::Create => FfiFileAction::Create,
            FileAction::Chmod => FfiFileAction::Chmod,
        }
    }
}

impl From<ProcessAction> for FfiProcessAction {
    fn from(action: ProcessAction) -> Self {
        match action {
            ProcessAction::Start => FfiProcessAction::Start,
            ProcessAction::Exit => FfiProcessAction::Exit,
            ProcessAction::Fork => FfiProcessAction::Fork,
        }
    }
}

impl From<SessionAction> for FfiSessionAction {
    fn from(action: SessionAction) -> Self {
        match action {
            SessionAction::Start => FfiSessionAction::Start,
            SessionAction::End => FfiSessionAction::End,
        }
    }
}

impl From<EventType> for FfiEventType {
    fn from(event_type: EventType) -> Self {
        match event_type {
            EventType::Command {
                command,
                args,
                exit_code,
            } => FfiEventType::Command {
                command,
                args,
                exit_code,
            },
            EventType::FileAccess { path, action } => FfiEventType::FileAccess {
                path: path.to_string_lossy().to_string(),
                action: action.into(),
            },
            EventType::Network {
                host,
                port,
                protocol,
            } => FfiEventType::Network {
                host,
                port,
                protocol,
            },
            EventType::Process { pid, ppid, action } => FfiEventType::Process {
                pid,
                ppid,
                action: action.into(),
            },
            EventType::Session { action } => FfiEventType::Session {
                action: action.into(),
            },
        }
    }
}

impl From<Event> for FfiEvent {
    fn from(event: Event) -> Self {
        FfiEvent {
            id: event.id.to_string(),
            timestamp_ms: event.timestamp.timestamp_millis(),
            timestamp_str: event.timestamp.to_rfc3339(),
            event_type: event.event_type.into(),
            process: event.process,
            pid: event.pid,
            risk_level: event.risk_level.into(),
            alert: event.alert,
        }
    }
}

impl From<Config> for FfiConfig {
    fn from(config: Config) -> Self {
        FfiConfig {
            general: FfiGeneralConfig {
                verbose: config.general.verbose,
                default_format: config.general.default_format,
            },
            logging: FfiLoggingConfig {
                enabled: config.logging.enabled,
                log_dir: config
                    .logging
                    .log_dir
                    .map(|p| p.to_string_lossy().to_string()),
                retention_days: config.logging.retention_days,
            },
            monitoring: FfiMonitoringConfig {
                fs_enabled: config.monitoring.fs_enabled,
                net_enabled: config.monitoring.net_enabled,
                track_children: config.monitoring.track_children,
                tracking_poll_ms: config.monitoring.tracking_poll_ms,
                fs_debounce_ms: config.monitoring.fs_debounce_ms,
                net_poll_ms: config.monitoring.net_poll_ms,
                watch_paths: config
                    .monitoring
                    .watch_paths
                    .into_iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect(),
                sensitive_patterns: config.monitoring.sensitive_patterns,
                network_whitelist: config.monitoring.network_whitelist,
            },
            alerts: FfiAlertConfig {
                min_level: config.alerts.min_level,
                custom_high_risk: config.alerts.custom_high_risk,
            },
        }
    }
}

impl From<CoreError> for FfiError {
    fn from(err: CoreError) -> Self {
        match err {
            CoreError::Config(e) => FfiError::Config {
                message: e.to_string(),
            },
            CoreError::Storage(e) => FfiError::Storage {
                message: e.to_string(),
            },
            CoreError::Io(e) => FfiError::Io {
                message: e.to_string(),
            },
            other => FfiError::Other {
                message: other.to_string(),
            },
        }
    }
}

// ─── Exported Functions ───────────────────────────────────────────────────────

#[uniffi::export]
pub fn load_config() -> Result<FfiConfig, FfiError> {
    let config = Config::load().map_err(FfiError::from)?;
    Ok(config.into())
}

#[uniffi::export]
pub fn analyze_command(command: String, args: Vec<String>) -> FfiEvent {
    let scorer = RiskScorer::new();
    let (risk_level, _reason) = scorer.score(&command, &args);
    let event = Event::command(
        command,
        args,
        "agent".to_string(),
        std::process::id(),
        risk_level,
    );
    event.into()
}

#[uniffi::export]
pub fn get_version() -> String {
    crate::VERSION.to_string()
}

#[uniffi::export]
pub fn read_session_log(path: String) -> Result<Vec<FfiEvent>, FfiError> {
    let file = std::fs::File::open(&path).map_err(|e| FfiError::Io {
        message: format!("Failed to open {}: {}", path, e),
    })?;
    let reader = std::io::BufReader::new(file);
    let mut events = Vec::new();

    for line in reader.lines() {
        let line = line.map_err(|e| FfiError::Io {
            message: format!("Failed to read line: {}", e),
        })?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Try parsing as an Event; skip session header/footer lines
        if let Ok(event) = serde_json::from_str::<Event>(trimmed) {
            events.push(event.into());
        }
    }

    Ok(events)
}

#[uniffi::export]
pub fn list_session_logs() -> Result<Vec<FfiSessionInfo>, FfiError> {
    let log_dir = Config::default_log_dir().map_err(FfiError::from)?;

    if !log_dir.exists() {
        return Ok(Vec::new());
    }

    let pattern = log_dir.join("session-*.jsonl");
    let pattern_str = pattern.to_string_lossy().to_string();

    let mut sessions = Vec::new();

    for entry in glob::glob(&pattern_str).map_err(|e| FfiError::Other {
        message: format!("Invalid glob pattern: {}", e),
    })? {
        let path = entry.map_err(|e| FfiError::Io {
            message: format!("Glob error: {}", e),
        })?;

        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Extract session ID from filename: "session-{id}.jsonl"
        let session_id = filename
            .strip_prefix("session-")
            .and_then(|s| s.strip_suffix(".jsonl"))
            .unwrap_or(&filename)
            .to_string();

        // Get file modification time as a fallback start time
        let start_time_str = if let Ok(metadata) = std::fs::metadata(&path) {
            if let Ok(modified) = metadata.modified() {
                let dt: chrono::DateTime<chrono::Utc> = modified.into();
                dt.to_rfc3339()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        sessions.push(FfiSessionInfo {
            session_id,
            file_path: path.to_string_lossy().to_string(),
            start_time_str,
        });
    }

    // Sort by file path (which includes timestamp) in reverse for newest first
    sessions.sort_by(|a, b| b.file_path.cmp(&a.file_path));

    Ok(sessions)
}

#[uniffi::export]
pub fn get_activity_summary(events: Vec<FfiEvent>) -> FfiActivitySummary {
    let mut summary = FfiActivitySummary {
        total_events: events.len() as u32,
        critical_count: 0,
        high_count: 0,
        medium_count: 0,
        low_count: 0,
    };

    for event in &events {
        match event.risk_level {
            FfiRiskLevel::Critical => summary.critical_count += 1,
            FfiRiskLevel::High => summary.high_count += 1,
            FfiRiskLevel::Medium => summary.medium_count += 1,
            FfiRiskLevel::Low => summary.low_count += 1,
        }
    }

    summary
}

// ─── FfiMonitoringEngine Object ───────────────────────────────────────────────

/// Session lifecycle state to prevent race conditions from concurrent start/stop calls
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionState {
    Idle,
    Starting,
    Active,
    Stopping,
}

struct MonitoringSession {
    logger: SessionLogger,
    #[allow(dead_code)]
    process_name: String,
}

#[derive(uniffi::Object)]
pub struct FfiMonitoringEngine {
    state: Mutex<(SessionState, Option<MonitoringSession>)>,
}

#[uniffi::export]
impl FfiMonitoringEngine {
    #[uniffi::constructor]
    pub fn new() -> Self {
        FfiMonitoringEngine {
            state: Mutex::new((SessionState::Idle, None)),
        }
    }

    pub fn start_session(&self, process_name: String) -> Result<String, FfiError> {
        // Acquire lock and check state atomically
        let mut guard = self.state.lock().map_err(|e| FfiError::Other {
            message: format!("FfiMonitoringEngine lock poisoned in start_session: {}", e),
        })?;

        let (ref mut state, ref mut session) = *guard;

        // Only allow starting from Idle state
        if *state != SessionState::Idle {
            return Err(FfiError::Other {
                message: format!("Cannot start session: engine is in {:?} state", state),
            });
        }

        *state = SessionState::Starting;

        let config = Config::load().map_err(|e| {
            *state = SessionState::Idle;
            FfiError::from(e)
        })?;
        let log_dir = config.logging.effective_log_dir().map_err(|e| {
            *state = SessionState::Idle;
            FfiError::from(e)
        })?;

        let mut logger = SessionLogger::new(&log_dir, None).map_err(|e| {
            *state = SessionState::Idle;
            FfiError::Storage {
                message: format!("Failed to create session logger: {}", e),
            }
        })?;

        logger
            .write_session_header(&process_name, std::process::id())
            .map_err(|e| {
                *state = SessionState::Idle;
                FfiError::Storage {
                    message: format!("Failed to write session header: {}", e),
                }
            })?;

        let session_id = logger.session_id().to_string();

        *session = Some(MonitoringSession {
            logger,
            process_name,
        });
        *state = SessionState::Active;

        Ok(session_id)
    }

    pub fn stop_session(&self) -> Result<(), FfiError> {
        let mut guard = self.state.lock().map_err(|e| FfiError::Other {
            message: format!("FfiMonitoringEngine lock poisoned in stop_session: {}", e),
        })?;

        let (ref mut state, ref mut session) = *guard;

        // Only allow stopping from Active state
        if *state != SessionState::Active {
            return Err(FfiError::Other {
                message: format!("Cannot stop session: engine is in {:?} state", state),
            });
        }

        *state = SessionState::Stopping;

        if let Some(mut s) = session.take() {
            s.logger.write_session_footer(Some(0)).map_err(|e| {
                *state = SessionState::Active;
                FfiError::Storage {
                    message: format!("Failed to write session footer: {}", e),
                }
            })?;
        }

        *state = SessionState::Idle;

        Ok(())
    }

    pub fn is_active(&self) -> bool {
        self.state
            .lock()
            .map(|guard| guard.0 == SessionState::Active)
            .unwrap_or(false)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Event, EventType, FileAction, ProcessAction, RiskLevel, SessionAction};
    use std::path::PathBuf;

    #[test]
    fn test_risk_level_conversion() {
        assert_eq!(FfiRiskLevel::from(RiskLevel::Low), FfiRiskLevel::Low);
        assert_eq!(FfiRiskLevel::from(RiskLevel::Medium), FfiRiskLevel::Medium);
        assert_eq!(FfiRiskLevel::from(RiskLevel::High), FfiRiskLevel::High);
        assert_eq!(
            FfiRiskLevel::from(RiskLevel::Critical),
            FfiRiskLevel::Critical
        );
    }

    #[test]
    fn test_file_action_conversion() {
        assert_eq!(FfiFileAction::from(FileAction::Read), FfiFileAction::Read);
        assert_eq!(FfiFileAction::from(FileAction::Write), FfiFileAction::Write);
        assert_eq!(
            FfiFileAction::from(FileAction::Delete),
            FfiFileAction::Delete
        );
        assert_eq!(
            FfiFileAction::from(FileAction::Create),
            FfiFileAction::Create
        );
        assert_eq!(FfiFileAction::from(FileAction::Chmod), FfiFileAction::Chmod);
    }

    #[test]
    fn test_process_action_conversion() {
        assert_eq!(
            FfiProcessAction::from(ProcessAction::Start),
            FfiProcessAction::Start
        );
        assert_eq!(
            FfiProcessAction::from(ProcessAction::Exit),
            FfiProcessAction::Exit
        );
        assert_eq!(
            FfiProcessAction::from(ProcessAction::Fork),
            FfiProcessAction::Fork
        );
    }

    #[test]
    fn test_session_action_conversion() {
        assert_eq!(
            FfiSessionAction::from(SessionAction::Start),
            FfiSessionAction::Start
        );
        assert_eq!(
            FfiSessionAction::from(SessionAction::End),
            FfiSessionAction::End
        );
    }

    #[test]
    fn test_event_type_command_conversion() {
        let et = EventType::Command {
            command: "ls".to_string(),
            args: vec!["-la".to_string()],
            exit_code: Some(0),
        };
        let ffi_et: FfiEventType = et.into();
        match ffi_et {
            FfiEventType::Command {
                command,
                args,
                exit_code,
            } => {
                assert_eq!(command, "ls");
                assert_eq!(args, vec!["-la"]);
                assert_eq!(exit_code, Some(0));
            }
            _ => panic!("Expected Command variant"),
        }
    }

    #[test]
    fn test_event_type_file_access_conversion() {
        let et = EventType::FileAccess {
            path: PathBuf::from("/tmp/test.txt"),
            action: FileAction::Read,
        };
        let ffi_et: FfiEventType = et.into();
        match ffi_et {
            FfiEventType::FileAccess { path, action } => {
                assert_eq!(path, "/tmp/test.txt");
                assert_eq!(action, FfiFileAction::Read);
            }
            _ => panic!("Expected FileAccess variant"),
        }
    }

    #[test]
    fn test_event_type_network_conversion() {
        let et = EventType::Network {
            host: "example.com".to_string(),
            port: 443,
            protocol: "tcp".to_string(),
        };
        let ffi_et: FfiEventType = et.into();
        match ffi_et {
            FfiEventType::Network {
                host,
                port,
                protocol,
            } => {
                assert_eq!(host, "example.com");
                assert_eq!(port, 443);
                assert_eq!(protocol, "tcp");
            }
            _ => panic!("Expected Network variant"),
        }
    }

    #[test]
    fn test_event_type_process_conversion() {
        let et = EventType::Process {
            pid: 1234,
            ppid: Some(1),
            action: ProcessAction::Start,
        };
        let ffi_et: FfiEventType = et.into();
        match ffi_et {
            FfiEventType::Process { pid, ppid, action } => {
                assert_eq!(pid, 1234);
                assert_eq!(ppid, Some(1));
                assert_eq!(action, FfiProcessAction::Start);
            }
            _ => panic!("Expected Process variant"),
        }
    }

    #[test]
    fn test_event_type_session_conversion() {
        let et = EventType::Session {
            action: SessionAction::Start,
        };
        let ffi_et: FfiEventType = et.into();
        match ffi_et {
            FfiEventType::Session { action } => {
                assert_eq!(action, FfiSessionAction::Start);
            }
            _ => panic!("Expected Session variant"),
        }
    }

    #[test]
    fn test_event_conversion() {
        let event = Event::command(
            "ls".to_string(),
            vec!["-la".to_string()],
            "bash".to_string(),
            1234,
            RiskLevel::Low,
        );
        let ffi_event: FfiEvent = event.into();

        assert!(!ffi_event.id.is_empty());
        assert!(ffi_event.timestamp_ms > 0);
        assert!(!ffi_event.timestamp_str.is_empty());
        assert_eq!(ffi_event.process, "bash");
        assert_eq!(ffi_event.pid, 1234);
        assert_eq!(ffi_event.risk_level, FfiRiskLevel::Low);
        assert!(!ffi_event.alert);
    }

    #[test]
    fn test_config_conversion() {
        let config = Config::default();
        let ffi_config: FfiConfig = config.into();

        assert!(!ffi_config.general.verbose);
        assert_eq!(ffi_config.general.default_format, "pretty");
        assert!(ffi_config.logging.enabled);
        assert_eq!(ffi_config.logging.retention_days, 30);
        assert!(!ffi_config.monitoring.fs_enabled);
        assert!(!ffi_config.monitoring.net_enabled);
        assert!(ffi_config.monitoring.track_children);
    }

    #[test]
    fn test_core_error_to_ffi_error_config() {
        let err = CoreError::Config(crate::error::ConfigError::NoHomeDir);
        let ffi_err: FfiError = err.into();
        match ffi_err {
            FfiError::Config { message } => {
                assert!(message.contains("home directory"));
            }
            _ => panic!("Expected Config variant"),
        }
    }

    #[test]
    fn test_core_error_to_ffi_error_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = CoreError::Io(io_err);
        let ffi_err: FfiError = err.into();
        match ffi_err {
            FfiError::Io { message } => {
                assert!(message.contains("file not found"));
            }
            _ => panic!("Expected Io variant"),
        }
    }

    #[test]
    fn test_core_error_to_ffi_error_other() {
        let err = CoreError::Wrapper("something went wrong".to_string());
        let ffi_err: FfiError = err.into();
        match ffi_err {
            FfiError::Other { message } => {
                assert!(message.contains("something went wrong"));
            }
            _ => panic!("Expected Other variant"),
        }
    }

    #[test]
    fn test_analyze_command_low_risk() {
        let event = analyze_command("ls".to_string(), vec!["-la".to_string()]);
        assert_eq!(event.risk_level, FfiRiskLevel::Low);
    }

    #[test]
    fn test_analyze_command_medium_risk() {
        let event = analyze_command("curl".to_string(), vec!["https://example.com".to_string()]);
        assert_eq!(event.risk_level, FfiRiskLevel::Medium);
    }

    #[test]
    fn test_analyze_command_high_risk() {
        let event = analyze_command(
            "rm".to_string(),
            vec!["-rf".to_string(), "directory".to_string()],
        );
        assert_eq!(event.risk_level, FfiRiskLevel::High);
    }

    #[test]
    fn test_analyze_command_critical_risk() {
        let event = analyze_command("rm".to_string(), vec!["-rf".to_string(), "/".to_string()]);
        assert_eq!(event.risk_level, FfiRiskLevel::Critical);
    }

    #[test]
    fn test_get_version() {
        let version = get_version();
        assert!(!version.is_empty());
        assert!(version.contains('.'));
    }

    #[test]
    fn test_load_config() {
        let result = load_config();
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.general.default_format, "pretty");
    }

    #[test]
    fn test_get_activity_summary() {
        let events = vec![
            analyze_command("ls".to_string(), vec![]),
            analyze_command("curl".to_string(), vec!["http://x.com".to_string()]),
            analyze_command("sudo".to_string(), vec!["rm".to_string()]),
            analyze_command("rm".to_string(), vec!["-rf".to_string(), "/".to_string()]),
            analyze_command("echo".to_string(), vec!["hello".to_string()]),
        ];
        let summary = get_activity_summary(events);
        assert_eq!(summary.total_events, 5);
        assert!(summary.low_count >= 1);
        assert!(summary.medium_count >= 1);
        assert!(summary.high_count >= 1);
        assert!(summary.critical_count >= 1);
    }

    #[test]
    fn test_get_activity_summary_empty() {
        let summary = get_activity_summary(vec![]);
        assert_eq!(summary.total_events, 0);
        assert_eq!(summary.critical_count, 0);
        assert_eq!(summary.high_count, 0);
        assert_eq!(summary.medium_count, 0);
        assert_eq!(summary.low_count, 0);
    }

    #[test]
    fn test_read_session_log_nonexistent() {
        let result = read_session_log("/nonexistent/path.jsonl".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_read_session_log_with_events() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test-session.jsonl");

        // Write some test events
        let events = vec![
            Event::command(
                "ls".to_string(),
                vec![],
                "bash".to_string(),
                1,
                RiskLevel::Low,
            ),
            Event::command(
                "curl".to_string(),
                vec!["http://example.com".to_string()],
                "bash".to_string(),
                2,
                RiskLevel::Medium,
            ),
        ];

        let mut content = String::new();
        for event in &events {
            content.push_str(&serde_json::to_string(event).unwrap());
            content.push('\n');
        }
        std::fs::write(&log_path, &content).unwrap();

        let result = read_session_log(log_path.to_string_lossy().to_string());
        assert!(result.is_ok());
        let ffi_events = result.unwrap();
        assert_eq!(ffi_events.len(), 2);
        assert_eq!(ffi_events[0].risk_level, FfiRiskLevel::Low);
        assert_eq!(ffi_events[1].risk_level, FfiRiskLevel::Medium);
    }

    #[test]
    fn test_list_session_logs_empty() {
        // This should succeed even if the log directory doesn't exist
        let result = list_session_logs();
        assert!(result.is_ok());
    }

    #[test]
    fn test_monitoring_engine_lifecycle() {
        let engine = FfiMonitoringEngine::new();
        assert!(!engine.is_active());
    }

    #[test]
    fn test_monitoring_engine_start_stop() {
        let engine = FfiMonitoringEngine::new();
        assert!(!engine.is_active());

        let result = engine.start_session("test-process".to_string());
        assert!(result.is_ok());
        assert!(engine.is_active());

        let stop_result = engine.stop_session();
        assert!(stop_result.is_ok());
        assert!(!engine.is_active());
    }

    #[test]
    fn test_monitoring_engine_session_id() {
        let engine = FfiMonitoringEngine::new();
        let session_id = engine.start_session("test".to_string()).unwrap();
        assert!(!session_id.is_empty());
        engine.stop_session().unwrap();
    }

    #[test]
    fn test_ffi_error_display() {
        let err = FfiError::Config {
            message: "bad config".to_string(),
        };
        assert_eq!(err.to_string(), "Config error: bad config");

        let err = FfiError::Storage {
            message: "disk full".to_string(),
        };
        assert_eq!(err.to_string(), "Storage error: disk full");

        let err = FfiError::Io {
            message: "not found".to_string(),
        };
        assert_eq!(err.to_string(), "IO error: not found");

        let err = FfiError::Other {
            message: "unknown".to_string(),
        };
        assert_eq!(err.to_string(), "unknown");
    }
}
