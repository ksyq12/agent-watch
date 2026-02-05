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

pub mod event;
pub mod logger;
pub mod process_tracker;
pub mod risk;
pub mod wrapper;

// Re-export commonly used types
pub use event::{Event, EventType, FileAction, ProcessAction, RiskLevel, SessionAction};
pub use logger::{LogFormat, LogDestination, Logger, LoggerConfig};
pub use process_tracker::{ProcessTracker, TrackedProcess, TrackerConfig, TrackerEvent};
pub use risk::{RiskPattern, RiskRule, RiskScorer};
pub use wrapper::{ProcessWrapper, WrapperConfig, WrapperEvent};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");
