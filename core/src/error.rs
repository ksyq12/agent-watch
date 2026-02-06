//! Unified error types for MacAgentWatch Core
//!
//! Provides structured error types instead of anyhow for better
//! error handling, pattern matching, and FFI compatibility.

use std::path::PathBuf;
use thiserror::Error;

/// Core library error type
#[derive(Error, Debug)]
pub enum CoreError {
    /// Configuration file errors
    #[error("Config error: {0}")]
    Config(#[from] ConfigError),

    /// Storage/logging errors
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    /// Process wrapper errors
    #[error("Wrapper error: {0}")]
    Wrapper(String),

    /// Process tracker errors
    #[error("Process tracker error: {0}")]
    ProcessTracker(String),

    /// File system watcher errors
    #[error("FSWatch error: {0}")]
    FsWatch(String),

    /// Network monitor errors
    #[error("Network monitor error: {0}")]
    NetMon(String),

    /// Generic I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Configuration-specific errors
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Failed to read config file
    #[error("Failed to read config file {path}: {source}")]
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Failed to parse TOML
    #[error("Failed to parse TOML config: {0}")]
    ParseToml(#[from] toml::de::Error),

    /// Failed to serialize config
    #[error("Failed to serialize config: {0}")]
    SerializeToml(#[from] toml::ser::Error),

    /// Failed to write config file
    #[error("Failed to write config file {path}: {source}")]
    WriteFile {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Home directory not found
    #[error("Could not determine home directory")]
    NoHomeDir,

    /// Failed to create directory
    #[error("Failed to create directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// Storage-specific errors
#[derive(Error, Debug)]
pub enum StorageError {
    /// Failed to create log directory
    #[error("Failed to create log directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Failed to create/open log file
    #[error("Failed to open log file {path}: {source}")]
    OpenFile {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Failed to serialize event
    #[error("Failed to serialize event: {0}")]
    Serialize(#[from] serde_json::Error),

    /// Failed to write to log file
    #[error("Failed to write to log file: {0}")]
    Write(std::io::Error),

    /// Failed to flush buffer
    #[error("Failed to flush log buffer: {0}")]
    Flush(std::io::Error),
}

/// Convenience type alias
pub type Result<T> = std::result::Result<T, CoreError>;
