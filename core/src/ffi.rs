//! FFI module for UniFFI Swift bindings
//!
//! Provides FFI-safe types, conversions, and exported functions for the Swift app layer.

use crate::agent_detector::AgentDetector;
use crate::config::{Config, NotificationConfig};
use crate::error::CoreError;
use crate::event::{Event, EventType, FileAction, ProcessAction, RiskLevel, SessionAction};
use crate::fswatch::{FileSystemWatcher, FsWatchConfig};
use crate::netmon::{NetMonConfig, NetworkMonitor};
use crate::process_tracker::{ProcessTracker, TrackerConfig, TrackerEvent};
use crate::risk::RiskScorer;
use crate::storage::{EventStorage, SessionLogger};
use std::io::BufRead;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

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
pub struct FfiNotificationConfig {
    pub enabled: bool,
    pub min_risk_level: String,
    pub sound_enabled: bool,
    pub badge_enabled: bool,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiConfig {
    pub general: FfiGeneralConfig,
    pub logging: FfiLoggingConfig,
    pub monitoring: FfiMonitoringConfig,
    pub alerts: FfiAlertConfig,
    pub notification: FfiNotificationConfig,
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

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiChartDataPoint {
    pub timestamp_ms: i64,
    pub total: u32,
    pub critical: u32,
    pub high: u32,
    pub medium: u32,
    pub low: u32,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiDetectedAgent {
    pub pid: u32,
    pub name: String,
    pub path: String,
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

impl From<NotificationConfig> for FfiNotificationConfig {
    fn from(config: NotificationConfig) -> Self {
        FfiNotificationConfig {
            enabled: config.enabled,
            min_risk_level: config.min_risk_level,
            sound_enabled: config.sound_enabled,
            badge_enabled: config.badge_enabled,
        }
    }
}

impl From<FfiNotificationConfig> for NotificationConfig {
    fn from(ffi: FfiNotificationConfig) -> Self {
        NotificationConfig {
            enabled: ffi.enabled,
            min_risk_level: ffi.min_risk_level,
            sound_enabled: ffi.sound_enabled,
            badge_enabled: ffi.badge_enabled,
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
            notification: config.notifications.into(),
        }
    }
}

impl From<FfiConfig> for Config {
    fn from(ffi: FfiConfig) -> Self {
        use crate::config::*;
        Config {
            general: GeneralConfig {
                verbose: ffi.general.verbose,
                default_format: ffi.general.default_format,
            },
            logging: LoggingConfig {
                enabled: ffi.logging.enabled,
                log_dir: ffi.logging.log_dir.map(std::path::PathBuf::from),
                retention_days: ffi.logging.retention_days,
                storage_backend: StorageBackend::default(),
            },
            monitoring: MonitoringConfig {
                fs_enabled: ffi.monitoring.fs_enabled,
                net_enabled: ffi.monitoring.net_enabled,
                track_children: ffi.monitoring.track_children,
                tracking_poll_ms: ffi.monitoring.tracking_poll_ms,
                fs_debounce_ms: ffi.monitoring.fs_debounce_ms,
                net_poll_ms: ffi.monitoring.net_poll_ms,
                watch_paths: ffi
                    .monitoring
                    .watch_paths
                    .into_iter()
                    .map(std::path::PathBuf::from)
                    .collect(),
                sensitive_patterns: ffi.monitoring.sensitive_patterns,
                network_whitelist: ffi.monitoring.network_whitelist,
            },
            alerts: AlertConfig {
                min_level: ffi.alerts.min_level,
                custom_high_risk: ffi.alerts.custom_high_risk,
            },
            notifications: ffi.notification.into(),
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
pub fn save_config(config: FfiConfig) -> Result<(), FfiError> {
    let config: Config = config.into();
    let path = Config::default_path().map_err(FfiError::from)?;
    config.save(&path).map_err(FfiError::from)?;
    Ok(())
}

#[uniffi::export]
pub fn get_notification_config() -> Result<FfiNotificationConfig, FfiError> {
    let config = Config::load().map_err(FfiError::from)?;
    Ok(config.notifications.into())
}

#[uniffi::export]
pub fn save_notification_config(notification: FfiNotificationConfig) -> Result<(), FfiError> {
    let path = Config::default_path().map_err(FfiError::from)?;
    let mut config = Config::load().map_err(FfiError::from)?;
    config.notifications = notification.into();
    config.save(&path).map_err(FfiError::from)?;
    Ok(())
}

#[uniffi::export]
pub fn analyze_command(command: String, args: Vec<String>) -> Result<FfiEvent, FfiError> {
    let scorer = RiskScorer::new();
    let (risk_level, _reason) = scorer.score(&command, &args);
    let event = Event::command(
        command,
        args,
        "agent".to_string(),
        std::process::id(),
        risk_level,
    );
    Ok(event.into())
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
        match serde_json::from_str::<Event>(trimmed) {
            Ok(event) => events.push(event.into()),
            Err(e) => {
                // Check if it's a known session metadata line (not a warning)
                let value: Result<serde_json::Value, _> = serde_json::from_str(trimmed);
                let is_session_meta = value
                    .ok()
                    .and_then(|v| v.get("type").and_then(|t| t.as_str()).map(String::from))
                    .is_some_and(|t| t == "session_start" || t == "session_end");
                if !is_session_meta {
                    eprintln!("[agent-watch] Warning: skipping invalid JSONL line: {e}");
                }
            }
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
pub fn get_activity_summary(events: Vec<FfiEvent>) -> Result<FfiActivitySummary, FfiError> {
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

    Ok(summary)
}

// ─── Helper: parse events from JSONL file ─────────────────────────────────────

/// Parse events from a JSONL session file, returning (Event, line_index) pairs.
/// Skips session metadata lines (session_start/session_end) and empty lines.
fn parse_events_from_file(path: &str) -> Result<Vec<Event>, FfiError> {
    let file = std::fs::File::open(path).map_err(|e| FfiError::Io {
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
        match serde_json::from_str::<Event>(trimmed) {
            Ok(event) => events.push(event),
            Err(_) => {
                // Skip session metadata lines and invalid lines
            }
        }
    }

    Ok(events)
}

// ─── New Exported Functions (v0.4.0) ──────────────────────────────────────────

#[uniffi::export]
pub fn read_session_log_paginated(
    path: String,
    offset: u32,
    limit: u32,
) -> Result<Vec<FfiEvent>, FfiError> {
    let events = parse_events_from_file(&path)?;
    let offset = offset as usize;
    let limit = limit as usize;

    let paginated: Vec<FfiEvent> = events
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(FfiEvent::from)
        .collect();

    Ok(paginated)
}

#[uniffi::export]
pub fn get_session_event_count(path: String) -> Result<u32, FfiError> {
    let events = parse_events_from_file(&path)?;
    Ok(events.len() as u32)
}

#[uniffi::export]
pub fn get_chart_data(
    path: String,
    bucket_minutes: u32,
) -> Result<Vec<FfiChartDataPoint>, FfiError> {
    let bucket_minutes = if bucket_minutes == 0 {
        60
    } else {
        bucket_minutes
    };
    let bucket_ms: i64 = bucket_minutes as i64 * 60 * 1000;

    let events = parse_events_from_file(&path)?;
    if events.is_empty() {
        return Ok(Vec::new());
    }

    let mut buckets: std::collections::BTreeMap<i64, FfiChartDataPoint> =
        std::collections::BTreeMap::new();

    for event in &events {
        let ts = event.timestamp.timestamp_millis();
        let bucket_key = (ts / bucket_ms) * bucket_ms;

        let point = buckets.entry(bucket_key).or_insert(FfiChartDataPoint {
            timestamp_ms: bucket_key,
            total: 0,
            critical: 0,
            high: 0,
            medium: 0,
            low: 0,
        });

        point.total += 1;
        match event.risk_level {
            RiskLevel::Critical => point.critical += 1,
            RiskLevel::High => point.high += 1,
            RiskLevel::Medium => point.medium += 1,
            RiskLevel::Low => point.low += 1,
        }
    }

    Ok(buckets.into_values().collect())
}

#[uniffi::export]
pub fn search_events(
    path: String,
    query: String,
    event_type_filter: Option<String>,
    risk_level_filter: Option<FfiRiskLevel>,
    start_time_ms: Option<i64>,
    end_time_ms: Option<i64>,
) -> Result<Vec<FfiEvent>, FfiError> {
    let events = parse_events_from_file(&path)?;
    let query_lower = query.to_lowercase();

    let filtered: Vec<FfiEvent> = events
        .into_iter()
        .filter(|event| {
            // Time range filter
            let ts = event.timestamp.timestamp_millis();
            if let Some(start) = start_time_ms {
                if ts < start {
                    return false;
                }
            }
            if let Some(end) = end_time_ms {
                if ts > end {
                    return false;
                }
            }

            // Risk level filter
            if let Some(ref rl) = risk_level_filter {
                let event_rl: FfiRiskLevel = event.risk_level.into();
                if event_rl != *rl {
                    return false;
                }
            }

            // Event type filter
            if let Some(ref et_filter) = event_type_filter {
                let matches_type = match et_filter.as_str() {
                    "command" => matches!(event.event_type, EventType::Command { .. }),
                    "file_access" => matches!(event.event_type, EventType::FileAccess { .. }),
                    "network" => matches!(event.event_type, EventType::Network { .. }),
                    "process" => matches!(event.event_type, EventType::Process { .. }),
                    _ => true,
                };
                if !matches_type {
                    return false;
                }
            }

            // Full-text search across command, file path, host fields
            if query_lower.is_empty() {
                return true;
            }
            match &event.event_type {
                EventType::Command { command, args, .. } => {
                    command.to_lowercase().contains(&query_lower)
                        || args.iter().any(|a| a.to_lowercase().contains(&query_lower))
                }
                EventType::FileAccess { path, .. } => {
                    path.to_string_lossy().to_lowercase().contains(&query_lower)
                }
                EventType::Network { host, .. } => host.to_lowercase().contains(&query_lower),
                EventType::Process { .. } => false,
                EventType::Session { .. } => false,
            }
        })
        .map(FfiEvent::from)
        .collect();

    Ok(filtered)
}

#[uniffi::export]
pub fn get_latest_events(path: String, since_index: u32) -> Result<Vec<FfiEvent>, FfiError> {
    let events = parse_events_from_file(&path)?;
    let since = since_index as usize;

    let latest: Vec<FfiEvent> = events.into_iter().skip(since).map(FfiEvent::from).collect();

    Ok(latest)
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
    logger: Arc<Mutex<SessionLogger>>,
    trackers: Vec<ProcessTracker>,
    fs_watcher: Option<FileSystemWatcher>,
    net_monitors: Vec<NetworkMonitor>,
    writer_thread: Option<JoinHandle<()>>,
    detected_agents: Vec<FfiDetectedAgent>,
    /// Sender side of the unified event channel. Held here so we can drop it
    /// on stop, which causes the writer thread's recv() to return Err and exit.
    unified_tx: Option<mpsc::Sender<Event>>,
    /// Handles for forwarding threads (TrackerEvent → Event bridges)
    forwarding_threads: Vec<JoinHandle<()>>,
}

#[derive(uniffi::Object)]
pub struct FfiMonitoringEngine {
    state: Mutex<(SessionState, Option<MonitoringSession>)>,
}

impl Default for FfiMonitoringEngine {
    fn default() -> Self {
        Self::new()
    }
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

        // 1. Load config
        let config = Config::load().map_err(|e| {
            *state = SessionState::Idle;
            FfiError::from(e)
        })?;
        let log_dir = config.logging.effective_log_dir().map_err(|e| {
            *state = SessionState::Idle;
            FfiError::from(e)
        })?;

        // 2. Create SessionLogger
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
        let logger = Arc::new(Mutex::new(logger));

        // 3. Run AgentDetector
        let detector = AgentDetector::new();
        let raw_agents = detector.scan_for_agents();
        let detected_agents: Vec<FfiDetectedAgent> = raw_agents
            .iter()
            .map(|a| FfiDetectedAgent {
                pid: a.pid,
                name: a.name.clone(),
                path: a.path.clone(),
            })
            .collect();

        if detected_agents.is_empty() {
            *state = SessionState::Idle;
            return Err(FfiError::Other {
                message: "No AI agents detected. Start an AI agent (Claude, Cursor, Copilot, etc.) before monitoring.".to_string(),
            });
        }

        // 4. Create unified event channel
        let (unified_tx, unified_rx) = mpsc::channel::<Event>();

        // 5. For each detected agent: create ProcessTracker and NetworkMonitor
        let mut trackers = Vec::new();
        let mut net_monitors = Vec::new();
        let mut forwarding_threads = Vec::new();

        for agent in &raw_agents {
            // ProcessTracker
            if config.monitoring.track_children {
                let mut tracker = ProcessTracker::new(TrackerConfig::new(agent.pid).poll_interval(
                    std::time::Duration::from_millis(config.monitoring.tracking_poll_ms),
                ));
                let tracker_rx = tracker.subscribe();
                tracker.start();

                // Forwarding thread: TrackerEvent → Event
                let fwd_tx = unified_tx.clone();
                let agent_name = agent.name.clone();
                let fwd_handle = thread::spawn(move || {
                    while let Ok(tracker_event) = tracker_rx.recv() {
                        let event = match tracker_event {
                            TrackerEvent::ChildStarted {
                                pid,
                                ppid,
                                name,
                                risk_level,
                                ..
                            } => Event::new(
                                EventType::Process {
                                    pid,
                                    ppid: Some(ppid),
                                    action: ProcessAction::Start,
                                },
                                name,
                                pid,
                                risk_level,
                            ),
                            TrackerEvent::ChildExited { pid } => Event::new(
                                EventType::Process {
                                    pid,
                                    ppid: None,
                                    action: ProcessAction::Exit,
                                },
                                agent_name.clone(),
                                pid,
                                RiskLevel::Low,
                            ),
                        };
                        if fwd_tx.send(event).is_err() {
                            break;
                        }
                    }
                });
                forwarding_threads.push(fwd_handle);
                trackers.push(tracker);
            }

            // NetworkMonitor
            if config.monitoring.net_enabled {
                let mut monitor = NetworkMonitor::new(NetMonConfig::new(agent.pid).poll_interval(
                    std::time::Duration::from_millis(config.monitoring.net_poll_ms),
                ));
                let net_rx = monitor.subscribe();
                if monitor.start().is_ok() {
                    let fwd_tx = unified_tx.clone();
                    let fwd_handle = thread::spawn(move || {
                        while let Ok(event) = net_rx.recv() {
                            if fwd_tx.send(event).is_err() {
                                break;
                            }
                        }
                    });
                    forwarding_threads.push(fwd_handle);
                    net_monitors.push(monitor);
                }
            }
        }

        // 6. FileSystemWatcher
        let mut fs_watcher = None;
        if config.monitoring.fs_enabled {
            let watch_paths = if config.monitoring.watch_paths.is_empty() {
                if let Some(home) = dirs::home_dir() {
                    vec![home]
                } else {
                    Vec::new()
                }
            } else {
                config.monitoring.watch_paths.clone()
            };

            if !watch_paths.is_empty() {
                let mut watcher = FileSystemWatcher::new(FsWatchConfig::new(watch_paths));
                let fs_rx = watcher.subscribe();
                if watcher.start().is_ok() {
                    let fwd_tx = unified_tx.clone();
                    let fwd_handle = thread::spawn(move || {
                        while let Ok(event) = fs_rx.recv() {
                            if fwd_tx.send(event).is_err() {
                                break;
                            }
                        }
                    });
                    forwarding_threads.push(fwd_handle);
                    fs_watcher = Some(watcher);
                }
            }
        }

        // 7. Spawn event writer thread
        let logger_clone = Arc::clone(&logger);
        let writer_handle = thread::spawn(move || {
            while let Ok(event) = unified_rx.recv() {
                if let Ok(mut l) = logger_clone.lock() {
                    let _ = l.write_event(&event);
                }
            }
            // Channel closed, flush
            if let Ok(mut l) = logger_clone.lock() {
                let _ = l.flush();
            }
        });

        *session = Some(MonitoringSession {
            logger,
            trackers,
            fs_watcher,
            net_monitors,
            writer_thread: Some(writer_handle),
            detected_agents,
            unified_tx: Some(unified_tx),
            forwarding_threads,
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
            // 1. Signal all subsystems to stop
            for tracker in &mut s.trackers {
                tracker.signal_stop();
            }
            if let Some(ref watcher) = s.fs_watcher {
                watcher.signal_stop();
            }
            for monitor in &s.net_monitors {
                monitor.signal_stop();
            }

            // 2. Stop and drop all subsystems. Dropping them closes the
            //    TrackerEvent / Event senders so forwarding threads unblock.
            for tracker in s.trackers.drain(..) {
                drop(tracker);
            }
            if let Some(watcher) = s.fs_watcher.take() {
                drop(watcher);
            }
            for monitor in s.net_monitors.drain(..) {
                drop(monitor);
            }

            // 3. Wait for forwarding threads to finish (they exit once
            //    the subsystem senders are dropped above)
            for handle in s.forwarding_threads.drain(..) {
                let _ = handle.join();
            }

            // 4. Drop the unified sender so the writer thread exits
            drop(s.unified_tx.take());

            // 5. Join writer thread
            if let Some(handle) = s.writer_thread.take() {
                let _ = handle.join();
            }

            // 6. Write session footer (best effort — session is already destroyed)
            if let Ok(mut logger) = s.logger.lock() {
                if let Err(e) = logger.write_session_footer(Some(0)) {
                    eprintln!(
                        "[agent-watch] Warning: Failed to write session footer: {}",
                        e
                    );
                }
            }
        }

        *state = SessionState::Idle;

        Ok(())
    }

    pub fn is_active(&self) -> Result<bool, FfiError> {
        // Use poison recovery for read-only access — the state is still readable
        // even if a previous holder panicked
        let guard = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Ok(guard.0 == SessionState::Active)
    }

    pub fn get_monitored_agents(&self) -> Result<Vec<FfiDetectedAgent>, FfiError> {
        let guard = self.state.lock().map_err(|e| FfiError::Other {
            message: format!(
                "FfiMonitoringEngine lock poisoned in get_monitored_agents: {}",
                e
            ),
        })?;

        let (ref current_state, ref session) = *guard;

        if *current_state != SessionState::Active {
            return Err(FfiError::Other {
                message: format!(
                    "Cannot get monitored agents: engine is in {:?} state",
                    current_state
                ),
            });
        }

        match session {
            Some(s) => Ok(s.detected_agents.clone()),
            None => Ok(Vec::new()),
        }
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
        assert!(ffi_config.notification.enabled);
        assert_eq!(ffi_config.notification.min_risk_level, "high");
        assert!(ffi_config.notification.sound_enabled);
        assert!(ffi_config.notification.badge_enabled);
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
        let event = analyze_command("ls".to_string(), vec!["-la".to_string()]).unwrap();
        assert_eq!(event.risk_level, FfiRiskLevel::Low);
    }

    #[test]
    fn test_analyze_command_medium_risk() {
        let event =
            analyze_command("curl".to_string(), vec!["https://example.com".to_string()]).unwrap();
        assert_eq!(event.risk_level, FfiRiskLevel::Medium);
    }

    #[test]
    fn test_analyze_command_high_risk() {
        let event = analyze_command(
            "rm".to_string(),
            vec!["-rf".to_string(), "directory".to_string()],
        )
        .unwrap();
        assert_eq!(event.risk_level, FfiRiskLevel::High);
    }

    #[test]
    fn test_analyze_command_critical_risk() {
        let event =
            analyze_command("rm".to_string(), vec!["-rf".to_string(), "/".to_string()]).unwrap();
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
            analyze_command("ls".to_string(), vec![]).unwrap(),
            analyze_command("curl".to_string(), vec!["http://x.com".to_string()]).unwrap(),
            analyze_command("sudo".to_string(), vec!["rm".to_string()]).unwrap(),
            analyze_command("rm".to_string(), vec!["-rf".to_string(), "/".to_string()]).unwrap(),
            analyze_command("echo".to_string(), vec!["hello".to_string()]).unwrap(),
        ];
        let summary = get_activity_summary(events).unwrap();
        assert_eq!(summary.total_events, 5);
        assert!(summary.low_count >= 1);
        assert!(summary.medium_count >= 1);
        assert!(summary.high_count >= 1);
        assert!(summary.critical_count >= 1);
    }

    #[test]
    fn test_get_activity_summary_empty() {
        let summary = get_activity_summary(vec![]).unwrap();
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
        assert!(!engine.is_active().unwrap());
    }

    #[test]
    fn test_monitoring_engine_start_stop() {
        let engine = FfiMonitoringEngine::new();
        assert!(!engine.is_active().unwrap());

        let result = engine.start_session("test-process".to_string());
        assert!(result.is_ok());
        assert!(engine.is_active().unwrap());

        let stop_result = engine.stop_session();
        assert!(stop_result.is_ok());
        assert!(!engine.is_active().unwrap());
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

    // ─── Helper: create a temp JSONL file with test events ────────────────────

    fn create_test_session_file(events: &[Event]) -> (tempfile::TempDir, String) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test-session.jsonl");

        let mut content = String::new();
        for event in events {
            content.push_str(&serde_json::to_string(event).unwrap());
            content.push('\n');
        }
        std::fs::write(&log_path, &content).unwrap();

        let path_str = log_path.to_string_lossy().to_string();
        (temp_dir, path_str)
    }

    fn sample_events() -> Vec<Event> {
        vec![
            Event::command(
                "ls".to_string(),
                vec!["-la".to_string()],
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
            Event::command(
                "rm".to_string(),
                vec!["-rf".to_string(), "dir".to_string()],
                "bash".to_string(),
                3,
                RiskLevel::High,
            ),
            Event::command(
                "rm".to_string(),
                vec!["-rf".to_string(), "/".to_string()],
                "bash".to_string(),
                4,
                RiskLevel::Critical,
            ),
            Event::command(
                "echo".to_string(),
                vec!["hello".to_string()],
                "bash".to_string(),
                5,
                RiskLevel::Low,
            ),
        ]
    }

    // ─── Tests for new v0.4.0 FFI functions ───────────────────────────────────

    #[test]
    fn test_read_session_log_paginated_basic() {
        let events = sample_events();
        let (_dir, path) = create_test_session_file(&events);

        let result = read_session_log_paginated(path, 0, 10).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_read_session_log_paginated_offset_limit() {
        let events = sample_events();
        let (_dir, path) = create_test_session_file(&events);

        let result = read_session_log_paginated(path.clone(), 1, 2).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].risk_level, FfiRiskLevel::Medium);
        assert_eq!(result[1].risk_level, FfiRiskLevel::High);
    }

    #[test]
    fn test_read_session_log_paginated_offset_beyond() {
        let events = sample_events();
        let (_dir, path) = create_test_session_file(&events);

        let result = read_session_log_paginated(path, 100, 10).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_read_session_log_paginated_nonexistent() {
        let result = read_session_log_paginated("/nonexistent/path.jsonl".to_string(), 0, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_session_event_count() {
        let events = sample_events();
        let (_dir, path) = create_test_session_file(&events);

        let count = get_session_event_count(path).unwrap();
        assert_eq!(count, 5);
    }

    #[test]
    fn test_get_session_event_count_empty() {
        let (_dir, path) = create_test_session_file(&[]);
        let count = get_session_event_count(path).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_get_session_event_count_with_metadata() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test-session.jsonl");

        let event = Event::command(
            "ls".to_string(),
            vec![],
            "bash".to_string(),
            1,
            RiskLevel::Low,
        );

        let mut content = String::new();
        content.push_str(r#"{"type":"session_start","session_id":"abc"}"#);
        content.push('\n');
        content.push_str(&serde_json::to_string(&event).unwrap());
        content.push('\n');
        content.push_str(r#"{"type":"session_end","exit_code":0}"#);
        content.push('\n');
        std::fs::write(&log_path, &content).unwrap();

        let count = get_session_event_count(log_path.to_string_lossy().to_string()).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_get_session_event_count_nonexistent() {
        let result = get_session_event_count("/nonexistent/path.jsonl".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_get_chart_data_basic() {
        let events = sample_events();
        let (_dir, path) = create_test_session_file(&events);

        let chart = get_chart_data(path, 60).unwrap();
        assert!(!chart.is_empty());

        // All events created at roughly the same time should be in one bucket
        let total: u32 = chart.iter().map(|p| p.total).sum();
        assert_eq!(total, 5);
    }

    #[test]
    fn test_get_chart_data_risk_breakdown() {
        let events = sample_events();
        let (_dir, path) = create_test_session_file(&events);

        let chart = get_chart_data(path, 60).unwrap();
        let total_low: u32 = chart.iter().map(|p| p.low).sum();
        let total_medium: u32 = chart.iter().map(|p| p.medium).sum();
        let total_high: u32 = chart.iter().map(|p| p.high).sum();
        let total_critical: u32 = chart.iter().map(|p| p.critical).sum();

        assert_eq!(total_low, 2);
        assert_eq!(total_medium, 1);
        assert_eq!(total_high, 1);
        assert_eq!(total_critical, 1);
    }

    #[test]
    fn test_get_chart_data_empty() {
        let (_dir, path) = create_test_session_file(&[]);
        let chart = get_chart_data(path, 60).unwrap();
        assert!(chart.is_empty());
    }

    #[test]
    fn test_get_chart_data_zero_bucket_defaults_to_60() {
        let events = sample_events();
        let (_dir, path) = create_test_session_file(&events);

        let chart = get_chart_data(path, 0).unwrap();
        assert!(!chart.is_empty());
    }

    #[test]
    fn test_search_events_by_query() {
        let events = sample_events();
        let (_dir, path) = create_test_session_file(&events);

        let results = search_events(path, "curl".to_string(), None, None, None, None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].risk_level, FfiRiskLevel::Medium);
    }

    #[test]
    fn test_search_events_case_insensitive() {
        let events = sample_events();
        let (_dir, path) = create_test_session_file(&events);

        let results = search_events(path, "CURL".to_string(), None, None, None, None).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_events_empty_query_returns_all() {
        let events = sample_events();
        let (_dir, path) = create_test_session_file(&events);

        let results = search_events(path, "".to_string(), None, None, None, None).unwrap();
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn test_search_events_by_event_type() {
        let events = sample_events();
        let (_dir, path) = create_test_session_file(&events);

        let results = search_events(
            path,
            "".to_string(),
            Some("command".to_string()),
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn test_search_events_by_risk_level() {
        let events = sample_events();
        let (_dir, path) = create_test_session_file(&events);

        let results = search_events(
            path,
            "".to_string(),
            None,
            Some(FfiRiskLevel::Critical),
            None,
            None,
        )
        .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_events_by_time_range() {
        let events = sample_events();
        let (_dir, path) = create_test_session_file(&events);

        // Use a very wide time range that includes all events
        let results = search_events(
            path.clone(),
            "".to_string(),
            None,
            None,
            Some(0),
            Some(i64::MAX),
        )
        .unwrap();
        assert_eq!(results.len(), 5);

        // Use a time range in the far past — no events should match
        let results = search_events(path, "".to_string(), None, None, Some(0), Some(1)).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_events_no_match() {
        let events = sample_events();
        let (_dir, path) = create_test_session_file(&events);

        let results = search_events(
            path,
            "nonexistent_command".to_string(),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_events_in_args() {
        let events = sample_events();
        let (_dir, path) = create_test_session_file(&events);

        let results =
            search_events(path, "example.com".to_string(), None, None, None, None).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_get_latest_events_from_start() {
        let events = sample_events();
        let (_dir, path) = create_test_session_file(&events);

        let results = get_latest_events(path, 0).unwrap();
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn test_get_latest_events_from_middle() {
        let events = sample_events();
        let (_dir, path) = create_test_session_file(&events);

        let results = get_latest_events(path, 3).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].risk_level, FfiRiskLevel::Critical);
        assert_eq!(results[1].risk_level, FfiRiskLevel::Low);
    }

    #[test]
    fn test_get_latest_events_beyond_end() {
        let events = sample_events();
        let (_dir, path) = create_test_session_file(&events);

        let results = get_latest_events(path, 100).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_get_latest_events_nonexistent() {
        let result = get_latest_events("/nonexistent/path.jsonl".to_string(), 0);
        assert!(result.is_err());
    }

    // ─── Tests for v0.5.0 FFI: notification config and save_config ────────────

    #[test]
    fn test_ffi_notification_config_defaults() {
        let config = Config::default();
        let ffi_config: FfiConfig = config.into();

        assert!(ffi_config.notification.enabled);
        assert_eq!(ffi_config.notification.min_risk_level, "high");
        assert!(ffi_config.notification.sound_enabled);
        assert!(ffi_config.notification.badge_enabled);
    }

    #[test]
    fn test_notification_config_round_trip() {
        use crate::config::NotificationConfig;

        let original = NotificationConfig {
            enabled: false,
            min_risk_level: "critical".to_string(),
            sound_enabled: false,
            badge_enabled: true,
        };

        let ffi: FfiNotificationConfig = original.clone().into();
        let back: NotificationConfig = ffi.into();

        assert_eq!(back.enabled, original.enabled);
        assert_eq!(back.min_risk_level, original.min_risk_level);
        assert_eq!(back.sound_enabled, original.sound_enabled);
        assert_eq!(back.badge_enabled, original.badge_enabled);
    }

    #[test]
    fn test_config_round_trip_save_load() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let mut config = Config::default();
        config.general.verbose = true;
        config.monitoring.fs_enabled = true;
        config.notifications.enabled = false;
        config.notifications.min_risk_level = "critical".to_string();
        config.notifications.sound_enabled = false;

        config.save(&config_path).unwrap();
        let loaded = Config::load_from_path(&config_path).unwrap();

        assert!(loaded.general.verbose);
        assert!(loaded.monitoring.fs_enabled);
        assert!(!loaded.notifications.enabled);
        assert_eq!(loaded.notifications.min_risk_level, "critical");
        assert!(!loaded.notifications.sound_enabled);
        assert!(loaded.notifications.badge_enabled);
    }

    #[test]
    fn test_ffi_config_round_trip() {
        let mut config = Config::default();
        config.general.verbose = true;
        config.notifications.enabled = false;
        config.notifications.min_risk_level = "medium".to_string();

        let ffi_config: FfiConfig = config.into();
        let back: Config = ffi_config.into();

        assert!(back.general.verbose);
        assert!(!back.notifications.enabled);
        assert_eq!(back.notifications.min_risk_level, "medium");
    }

    #[test]
    fn test_save_config_ffi() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let mut config = Config::default();
        config.notifications.min_risk_level = "low".to_string();

        // Save via Config::save directly (save_config uses default_path which we can't override)
        config.save(&config_path).unwrap();

        let loaded = Config::load_from_path(&config_path).unwrap();
        assert_eq!(loaded.notifications.min_risk_level, "low");
    }

    #[test]
    fn test_get_notification_config() {
        // get_notification_config loads from default path; should return defaults
        let result = get_notification_config();
        assert!(result.is_ok());
        let notif = result.unwrap();
        assert!(notif.enabled);
        assert_eq!(notif.min_risk_level, "high");
        assert!(notif.sound_enabled);
        assert!(notif.badge_enabled);
    }

    #[test]
    fn test_save_notification_config_round_trip() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Start with default config, save it
        let config = Config::default();
        config.save(&config_path).unwrap();

        // Modify notification config and save via Config directly
        let new_notif = FfiNotificationConfig {
            enabled: false,
            min_risk_level: "critical".to_string(),
            sound_enabled: false,
            badge_enabled: false,
        };

        let mut loaded = Config::load_from_path(&config_path).unwrap();
        loaded.notifications = new_notif.into();
        loaded.save(&config_path).unwrap();

        // Verify the notification section persisted correctly
        let reloaded = Config::load_from_path(&config_path).unwrap();
        assert!(!reloaded.notifications.enabled);
        assert_eq!(reloaded.notifications.min_risk_level, "critical");
        assert!(!reloaded.notifications.sound_enabled);
        assert!(!reloaded.notifications.badge_enabled);
    }

    // ─── Tests for v0.5.2: enhanced monitoring engine ─────────────────────────

    #[test]
    fn test_start_session_with_monitoring() {
        let engine = FfiMonitoringEngine::new();
        assert!(!engine.is_active().unwrap());

        let result = engine.start_session("test-agent".to_string());
        assert!(result.is_ok());
        let session_id = result.unwrap();
        assert!(!session_id.is_empty());
        assert!(engine.is_active().unwrap());

        // Clean up
        engine.stop_session().unwrap();
        assert!(!engine.is_active().unwrap());
    }

    #[test]
    fn test_stop_session_cleanup() {
        let engine = FfiMonitoringEngine::new();

        // Start a session
        let session_id = engine.start_session("test-cleanup".to_string()).unwrap();
        assert!(!session_id.is_empty());
        assert!(engine.is_active().unwrap());

        // Stop should clean up all subsystems
        let stop_result = engine.stop_session();
        assert!(stop_result.is_ok());
        assert!(!engine.is_active().unwrap());

        // Double stop should fail gracefully
        let second_stop = engine.stop_session();
        assert!(second_stop.is_err());
    }

    #[test]
    fn test_get_monitored_agents() {
        let engine = FfiMonitoringEngine::new();

        // Before start, should error
        let result = engine.get_monitored_agents();
        assert!(result.is_err());

        // Start session
        engine.start_session("test-agents".to_string()).unwrap();

        // Should return a list (may be empty if no agents are running)
        let agents = engine.get_monitored_agents().unwrap();
        // Just verify it returns a Vec
        let _ = agents.len();

        // Clean up
        engine.stop_session().unwrap();

        // After stop, should error again
        let result = engine.get_monitored_agents();
        assert!(result.is_err());
    }

    #[test]
    fn test_ffi_detected_agent_fields() {
        let agent = FfiDetectedAgent {
            pid: 999,
            name: "test-agent".to_string(),
            path: "/usr/bin/test-agent".to_string(),
        };
        assert_eq!(agent.pid, 999);
        assert_eq!(agent.name, "test-agent");
        assert_eq!(agent.path, "/usr/bin/test-agent");

        // Test clone
        let cloned = agent.clone();
        assert_eq!(cloned.pid, agent.pid);
        assert_eq!(cloned.name, agent.name);
    }
}
