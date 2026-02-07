//! Event types for MacAgentWatch
//!
//! Defines all event types that can be captured during agent monitoring.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Risk level for categorizing event severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    /// Safe operations (ls, cat, echo, cd)
    Low,
    /// Network operations, package installs (curl, wget, pip, npm)
    Medium,
    /// Destructive or privilege operations (rm -rf, sudo, ssh)
    High,
    /// Extremely dangerous (rm -rf /, chmod 777, curl | bash)
    Critical,
}

impl RiskLevel {
    /// Returns the emoji representation of the risk level
    pub fn emoji(&self) -> &'static str {
        match self {
            RiskLevel::Low => "🟢",
            RiskLevel::Medium => "🟡",
            RiskLevel::High => "🟠",
            RiskLevel::Critical => "🔴",
        }
    }

    /// Returns a text alternative for the risk level, suitable for
    /// terminals that may not render emojis properly.
    pub fn text_label(&self) -> &'static str {
        match self {
            RiskLevel::Low => "[LOW]",
            RiskLevel::Medium => "[MED]",
            RiskLevel::High => "[HIGH]",
            RiskLevel::Critical => "[CRIT]",
        }
    }

    /// Returns the color name for terminal output
    pub fn color(&self) -> &'static str {
        match self {
            RiskLevel::Low => "green",
            RiskLevel::Medium => "yellow",
            RiskLevel::High => "bright yellow",
            RiskLevel::Critical => "red",
        }
    }
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Low => write!(f, "low"),
            RiskLevel::Medium => write!(f, "medium"),
            RiskLevel::High => write!(f, "high"),
            RiskLevel::Critical => write!(f, "critical"),
        }
    }
}

/// Type of event captured
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventType {
    /// Command execution
    Command {
        /// The command that was executed
        command: String,
        /// Command arguments
        args: Vec<String>,
        /// Exit code if completed
        exit_code: Option<i32>,
    },
    /// File system access
    FileAccess {
        /// Path to the file
        path: PathBuf,
        /// Type of access
        action: FileAction,
    },
    /// Network connection
    Network {
        /// Remote host
        host: String,
        /// Remote port
        port: u16,
        /// Protocol (tcp, udp)
        protocol: String,
    },
    /// Process lifecycle
    Process {
        /// Process ID
        pid: u32,
        /// Parent process ID
        ppid: Option<u32>,
        /// Process action
        action: ProcessAction,
    },
    /// Session lifecycle
    Session {
        /// Session action
        action: SessionAction,
    },
}

/// File system action types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileAction {
    Read,
    Write,
    Delete,
    Create,
    Chmod,
}

impl std::fmt::Display for FileAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileAction::Read => write!(f, "read"),
            FileAction::Write => write!(f, "write"),
            FileAction::Delete => write!(f, "delete"),
            FileAction::Create => write!(f, "create"),
            FileAction::Chmod => write!(f, "chmod"),
        }
    }
}

/// Process action types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessAction {
    Start,
    Exit,
    Fork,
}

/// Session action types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionAction {
    Start,
    End,
}

/// A monitoring event captured by MacAgentWatch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique event ID
    pub id: uuid::Uuid,
    /// Timestamp of the event
    pub timestamp: DateTime<Utc>,
    /// Event type and details
    #[serde(flatten)]
    pub event_type: EventType,
    /// Process that generated the event
    pub process: String,
    /// Process ID
    pub pid: u32,
    /// Risk level
    pub risk_level: RiskLevel,
    /// Whether this event triggered an alert
    pub alert: bool,
}

impl Event {
    /// Create a new event with current timestamp
    pub fn new(event_type: EventType, process: String, pid: u32, risk_level: RiskLevel) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type,
            process,
            pid,
            risk_level,
            alert: matches!(risk_level, RiskLevel::Critical | RiskLevel::High),
        }
    }

    /// Create a command event
    pub fn command(
        command: String,
        args: Vec<String>,
        process: String,
        pid: u32,
        risk_level: RiskLevel,
    ) -> Self {
        Self::new(
            EventType::Command {
                command,
                args,
                exit_code: None,
            },
            process,
            pid,
            risk_level,
        )
    }

    /// Create a session start event
    pub fn session_start(process: String, pid: u32) -> Self {
        Self::new(
            EventType::Session {
                action: SessionAction::Start,
            },
            process,
            pid,
            RiskLevel::Low,
        )
    }

    /// Create a session end event
    pub fn session_end(process: String, pid: u32) -> Self {
        Self::new(
            EventType::Session {
                action: SessionAction::End,
            },
            process,
            pid,
            RiskLevel::Low,
        )
    }

    /// Create a process start event
    pub fn process_start(
        process: String,
        pid: u32,
        ppid: Option<u32>,
        risk_level: RiskLevel,
    ) -> Self {
        Self::new(
            EventType::Process {
                pid,
                ppid,
                action: ProcessAction::Start,
            },
            process,
            pid,
            risk_level,
        )
    }

    /// Create a process exit event
    pub fn process_exit(process: String, pid: u32, ppid: Option<u32>) -> Self {
        Self::new(
            EventType::Process {
                pid,
                ppid,
                action: ProcessAction::Exit,
            },
            process,
            pid,
            RiskLevel::Low,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
        assert!(RiskLevel::High < RiskLevel::Critical);
    }

    #[test]
    fn test_risk_level_emoji() {
        assert_eq!(RiskLevel::Low.emoji(), "🟢");
        assert_eq!(RiskLevel::Medium.emoji(), "🟡");
        assert_eq!(RiskLevel::High.emoji(), "🟠");
        assert_eq!(RiskLevel::Critical.emoji(), "🔴");
    }

    #[test]
    fn test_risk_level_text_label() {
        assert_eq!(RiskLevel::Low.text_label(), "[LOW]");
        assert_eq!(RiskLevel::Medium.text_label(), "[MED]");
        assert_eq!(RiskLevel::High.text_label(), "[HIGH]");
        assert_eq!(RiskLevel::Critical.text_label(), "[CRIT]");
    }

    #[test]
    fn test_risk_level_display() {
        assert_eq!(RiskLevel::Low.to_string(), "low");
        assert_eq!(RiskLevel::Critical.to_string(), "critical");
    }

    #[test]
    fn test_event_creation() {
        let event = Event::command(
            "ls".to_string(),
            vec!["-la".to_string()],
            "bash".to_string(),
            1234,
            RiskLevel::Low,
        );

        assert_eq!(event.process, "bash");
        assert_eq!(event.pid, 1234);
        assert_eq!(event.risk_level, RiskLevel::Low);
        assert!(!event.alert);
    }

    #[test]
    fn test_high_risk_triggers_alert() {
        let event = Event::command(
            "rm".to_string(),
            vec!["-rf".to_string(), "/".to_string()],
            "bash".to_string(),
            1234,
            RiskLevel::Critical,
        );

        assert!(event.alert);
    }

    #[test]
    fn test_event_serialization() {
        let event = Event::command(
            "echo".to_string(),
            vec!["hello".to_string()],
            "bash".to_string(),
            1234,
            RiskLevel::Low,
        );

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"command\""));
        assert!(json.contains("\"risk_level\":\"low\""));
    }

    #[test]
    fn test_file_action_display() {
        assert_eq!(FileAction::Read.to_string(), "read");
        assert_eq!(FileAction::Write.to_string(), "write");
        assert_eq!(FileAction::Delete.to_string(), "delete");
    }

    #[test]
    fn test_session_events() {
        let start = Event::session_start("claude-code".to_string(), 5678);
        let end = Event::session_end("claude-code".to_string(), 5678);

        assert!(matches!(
            start.event_type,
            EventType::Session {
                action: SessionAction::Start
            }
        ));
        assert!(matches!(
            end.event_type,
            EventType::Session {
                action: SessionAction::End
            }
        ));
    }
}
