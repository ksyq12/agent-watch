//! Shared types for MacAgentWatch
//!
//! Contains common types used across multiple modules.

use serde::{Deserialize, Serialize};

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

/// Trait for monitoring subsystem lifecycle management.
///
/// Implemented by `FileSystemWatcher`, `NetworkMonitor`, and `ProcessTracker`
/// to provide a uniform interface for the `MonitoringOrchestrator`.
pub trait MonitoringSubsystem: Send {
    /// Start the subsystem
    fn start(&mut self) -> anyhow::Result<()>;
    /// Stop the subsystem, joining any internal threads
    fn stop(&mut self);
    /// Signal the subsystem to stop without blocking (for two-phase shutdown)
    fn signal_stop(&self);
    /// Check if the subsystem is currently running
    fn is_running(&self) -> bool;
}
