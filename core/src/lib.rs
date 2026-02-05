//! MacAgentWatch Core Library
//!
//! Core monitoring functionality for AI agent activity tracking.
//!
//! # Features
//!
//! - **Process Wrapping**: Run commands through a PTY wrapper that captures all I/O
//! - **Risk Scoring**: Analyze commands for potential security risks
//! - **Event Logging**: Structured logging in multiple formats (Pretty, JSON, Compact)
//!
//! # Example
//!
//! ```no_run
//! use macagentwatch_core::{WrapperConfig, ProcessWrapper};
//!
//! let config = WrapperConfig::new("claude-code")
//!     .args(vec!["--help".to_string()])
//!     .pty_size(120, 40);
//!
//! let wrapper = ProcessWrapper::new(config);
//! let exit_code = wrapper.run_simple().expect("Failed to run");
//! ```

pub mod config;
pub mod detector;
pub mod event;
pub mod fswatch;
pub mod logger;
pub mod netmon;
pub mod process_tracker;
pub mod risk;
pub mod sanitize;
pub mod storage;
pub mod wrapper;

// Re-export commonly used types
pub use config::{AlertConfig, Config, GeneralConfig, LoggingConfig, MonitoringConfig};
pub use detector::{
    default_network_whitelist, default_sensitive_patterns, Detector, NetworkConnection,
    NetworkWhitelist, SensitiveFileDetector,
};
pub use event::{Event, EventType, FileAction, ProcessAction, RiskLevel, SessionAction};
pub use fswatch::{FileMonitor, FileSystemWatcher, FsEvent, FsWatchConfig};
pub use logger::{LogDestination, LogFormat, Logger, LoggerConfig};
pub use netmon::{NetMonConfig, NetworkMonitor, NetworkTracker, TrackedConnection};
pub use process_tracker::{ProcessTracker, TrackedProcess, TrackerConfig, TrackerEvent};
pub use risk::{RiskPattern, RiskRule, RiskScorer};
pub use sanitize::{sanitize_args, sanitize_command_string};
pub use storage::{cleanup_old_logs, EventStorage, SessionLogger};
pub use wrapper::{ProcessWrapper, WrapperConfig, WrapperEvent};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");
