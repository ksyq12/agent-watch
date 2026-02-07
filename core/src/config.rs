//! Configuration module for MacAgentWatch
//!
//! Handles loading, parsing, and validation of configuration files.
//! Default configuration path: `~/.macagentwatch/config.toml`

use crate::error::{ConfigError, CoreError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Main configuration structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// General settings
    pub general: GeneralConfig,
    /// Logging settings
    pub logging: LoggingConfig,
    /// File monitoring settings
    pub monitoring: MonitoringConfig,
    /// Alert settings
    pub alerts: AlertConfig,
}

impl Config {
    /// Load configuration from default path (~/.macagentwatch/config.toml)
    pub fn load() -> Result<Self, CoreError> {
        let path = Self::default_path()?;
        if path.exists() {
            Self::load_from_path(&path)
        } else {
            Ok(Self::default())
        }
    }

    /// Load configuration from a specific path
    pub fn load_from_path(path: &std::path::Path) -> Result<Self, CoreError> {
        let content = std::fs::read_to_string(path).map_err(|e| ConfigError::ReadFile {
            path: path.to_path_buf(),
            source: e,
        })?;
        Self::from_toml(&content)
    }

    /// Parse configuration from TOML string
    pub fn from_toml(content: &str) -> Result<Self, CoreError> {
        Ok(toml::from_str(content).map_err(ConfigError::ParseToml)?)
    }

    /// Get the base configuration directory path (~/.macagentwatch)
    fn config_base_dir() -> Result<PathBuf, CoreError> {
        dirs::home_dir()
            .ok_or(ConfigError::NoHomeDir)
            .map(|home| home.join(".macagentwatch"))
            .map_err(CoreError::Config)
    }

    /// Get default configuration file path
    pub fn default_path() -> Result<PathBuf, CoreError> {
        Self::config_base_dir().map(|dir| dir.join("config.toml"))
    }

    /// Get default log directory path
    pub fn default_log_dir() -> Result<PathBuf, CoreError> {
        Self::config_base_dir().map(|dir| dir.join("logs"))
    }

    /// Ensure configuration directory exists
    pub fn ensure_config_dir() -> Result<PathBuf, CoreError> {
        let config_dir = Self::config_base_dir()?;
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir).map_err(|e| ConfigError::CreateDir {
                path: config_dir.clone(),
                source: e,
            })?;
        }
        Ok(config_dir)
    }

    /// Save configuration to file
    pub fn save(&self, path: &std::path::Path) -> Result<(), CoreError> {
        let content = toml::to_string_pretty(self).map_err(ConfigError::SerializeToml)?;
        std::fs::write(path, content).map_err(|e| ConfigError::WriteFile {
            path: path.to_path_buf(),
            source: e,
        })?;
        Ok(())
    }
}

/// General configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// Enable verbose output
    pub verbose: bool,
    /// Default output format (pretty, json, compact)
    pub default_format: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            verbose: false,
            default_format: "pretty".to_string(),
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// Enable session log files
    pub enabled: bool,
    /// Log directory path (default: ~/.macagentwatch/logs)
    pub log_dir: Option<PathBuf>,
    /// Log retention in days (0 = no limit)
    pub retention_days: u32,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_dir: None,
            retention_days: 30,
        }
    }
}

impl LoggingConfig {
    /// Get effective log directory (custom or default)
    pub fn effective_log_dir(&self) -> Result<PathBuf, CoreError> {
        match &self.log_dir {
            Some(path) => Ok(path.clone()),
            None => Config::default_log_dir(),
        }
    }
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MonitoringConfig {
    /// Enable file system monitoring
    pub fs_enabled: bool,
    /// Enable network monitoring
    pub net_enabled: bool,
    /// Child process tracking enabled
    pub track_children: bool,
    /// Child process tracking poll interval in milliseconds
    pub tracking_poll_ms: u64,
    /// FSEvents debounce time in milliseconds
    pub fs_debounce_ms: u64,
    /// Network polling interval in milliseconds
    pub net_poll_ms: u64,
    /// Paths to watch for file system events
    pub watch_paths: Vec<PathBuf>,
    /// Sensitive file patterns (glob patterns)
    pub sensitive_patterns: Vec<String>,
    /// Network whitelist (allowed hosts)
    pub network_whitelist: Vec<String>,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            fs_enabled: false,
            net_enabled: false,
            track_children: true,
            tracking_poll_ms: 100,
            fs_debounce_ms: 100,
            net_poll_ms: 500,
            watch_paths: Vec::new(),
            sensitive_patterns: vec![
                ".env".to_string(),
                ".env.*".to_string(),
                "*.pem".to_string(),
                "*.key".to_string(),
                "*credential*".to_string(),
                "*secret*".to_string(),
            ],
            network_whitelist: vec![
                "api.anthropic.com".to_string(),
                "github.com".to_string(),
                "api.github.com".to_string(),
            ],
        }
    }
}

impl MonitoringConfig {
    /// Get FSEvents debounce duration
    pub fn fs_debounce_duration(&self) -> Duration {
        Duration::from_millis(self.fs_debounce_ms)
    }

    /// Get network poll duration
    pub fn net_poll_duration(&self) -> Duration {
        Duration::from_millis(self.net_poll_ms)
    }

    /// Get child process tracking poll duration
    pub fn tracking_poll_duration(&self) -> Duration {
        Duration::from_millis(self.tracking_poll_ms)
    }
}

/// Alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AlertConfig {
    /// Minimum risk level to trigger alerts (low, medium, high, critical)
    pub min_level: String,
    /// Custom high-risk commands
    pub custom_high_risk: Vec<String>,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            min_level: "high".to_string(),
            custom_high_risk: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(!config.general.verbose);
        assert_eq!(config.general.default_format, "pretty");
        assert!(config.logging.enabled);
        assert_eq!(config.logging.retention_days, 30);
        assert!(!config.monitoring.fs_enabled);
        assert!(!config.monitoring.net_enabled);
        assert!(config.monitoring.track_children);
    }

    #[test]
    fn test_config_parse_toml() {
        let toml_content = r#"
[general]
verbose = true
default_format = "json"

[logging]
enabled = true
retention_days = 7

[monitoring]
fs_enabled = true
net_enabled = true
track_children = true
tracking_poll_ms = 50
fs_debounce_ms = 200
sensitive_patterns = [".env", "*.key", "my_secret.txt"]
network_whitelist = ["example.com"]

[alerts]
min_level = "medium"
custom_high_risk = ["docker rm", "kubectl delete"]
"#;

        let config = Config::from_toml(toml_content).unwrap();
        assert!(config.general.verbose);
        assert_eq!(config.general.default_format, "json");
        assert!(config.logging.enabled);
        assert_eq!(config.logging.retention_days, 7);
        assert!(config.monitoring.fs_enabled);
        assert!(config.monitoring.net_enabled);
        assert_eq!(config.monitoring.tracking_poll_ms, 50);
        assert_eq!(config.monitoring.fs_debounce_ms, 200);
        assert_eq!(config.monitoring.sensitive_patterns.len(), 3);
        assert_eq!(config.monitoring.network_whitelist, vec!["example.com"]);
        assert_eq!(config.alerts.min_level, "medium");
        assert_eq!(config.alerts.custom_high_risk.len(), 2);
    }

    #[test]
    fn test_config_partial_toml() {
        let toml_content = r#"
[general]
verbose = true
"#;

        let config = Config::from_toml(toml_content).unwrap();
        assert!(config.general.verbose);
        // Other fields should have defaults
        assert!(config.logging.enabled);
        assert!(!config.monitoring.fs_enabled);
    }

    #[test]
    fn test_config_empty_toml() {
        let config = Config::from_toml("").unwrap();
        assert!(!config.general.verbose);
        assert_eq!(config.general.default_format, "pretty");
    }

    #[test]
    fn test_config_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let mut config = Config::default();
        config.general.verbose = true;
        config.monitoring.fs_enabled = true;
        config.monitoring.sensitive_patterns = vec![".env".to_string(), "*.key".to_string()];

        config.save(&config_path).unwrap();
        assert!(config_path.exists());

        let loaded = Config::load_from_path(&config_path).unwrap();
        assert!(loaded.general.verbose);
        assert!(loaded.monitoring.fs_enabled);
        assert_eq!(loaded.monitoring.sensitive_patterns.len(), 2);
    }

    #[test]
    fn test_config_load_nonexistent() {
        let result = Config::load();
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(!config.general.verbose);
    }

    #[test]
    fn test_monitoring_config_durations() {
        let config = MonitoringConfig {
            fs_debounce_ms: 100,
            net_poll_ms: 500,
            tracking_poll_ms: 50,
            ..Default::default()
        };

        assert_eq!(config.fs_debounce_duration(), Duration::from_millis(100));
        assert_eq!(config.net_poll_duration(), Duration::from_millis(500));
        assert_eq!(config.tracking_poll_duration(), Duration::from_millis(50));
    }

    #[test]
    fn test_default_sensitive_patterns() {
        let config = MonitoringConfig::default();
        assert!(config.sensitive_patterns.contains(&".env".to_string()));
        assert!(config.sensitive_patterns.contains(&"*.pem".to_string()));
        assert!(config.sensitive_patterns.contains(&"*.key".to_string()));
    }

    #[test]
    fn test_default_network_whitelist() {
        let config = MonitoringConfig::default();
        assert!(config
            .network_whitelist
            .contains(&"api.anthropic.com".to_string()));
        assert!(config.network_whitelist.contains(&"github.com".to_string()));
    }

    #[test]
    fn test_logging_effective_log_dir_custom() {
        let config = LoggingConfig {
            log_dir: Some(PathBuf::from("/custom/logs")),
            ..Default::default()
        };
        assert_eq!(
            config.effective_log_dir().unwrap(),
            PathBuf::from("/custom/logs")
        );
    }

    #[test]
    fn test_logging_effective_log_dir_default() {
        let config = LoggingConfig::default();
        let log_dir = config.effective_log_dir().unwrap();
        assert!(log_dir.to_string_lossy().contains(".macagentwatch"));
        assert!(log_dir.to_string_lossy().contains("logs"));
    }

    #[test]
    fn test_invalid_toml() {
        let result = Config::from_toml("invalid { toml content");
        assert!(result.is_err());
    }
}
