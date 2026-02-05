//! Logging module for MacAgentWatch
//!
//! Provides event logging with multiple output formats and destinations.

use crate::event::{Event, EventType, RiskLevel};
use colored::Colorize;
use std::io::{self, Write};
use std::path::PathBuf;

/// Log output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    /// Human-readable format for terminal
    #[default]
    Pretty,
    /// JSON Lines format for machine processing
    JsonLines,
    /// Compact single-line format
    Compact,
}

/// Log destination
#[derive(Debug, Clone, Default)]
pub enum LogDestination {
    /// Standard output
    #[default]
    Stdout,
    /// Standard error
    Stderr,
    /// File path
    File(PathBuf),
}

/// Logger configuration
#[derive(Debug, Clone)]
pub struct LoggerConfig {
    /// Output format
    pub format: LogFormat,
    /// Minimum risk level to log
    pub min_level: RiskLevel,
    /// Whether to show timestamps
    pub show_timestamps: bool,
    /// Whether to use colors (for Pretty format)
    pub use_colors: bool,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            format: LogFormat::Pretty,
            min_level: RiskLevel::Low,
            show_timestamps: true,
            use_colors: true,
        }
    }
}

/// Event logger
#[derive(Clone)]
pub struct Logger {
    config: LoggerConfig,
}

impl Default for Logger {
    fn default() -> Self {
        Self::new(LoggerConfig::default())
    }
}

impl Logger {
    /// Create a new logger with the given configuration
    pub fn new(config: LoggerConfig) -> Self {
        Self { config }
    }

    /// Format an event according to the logger configuration
    pub fn format(&self, event: &Event) -> String {
        match self.config.format {
            LogFormat::Pretty => self.format_pretty(event),
            LogFormat::JsonLines => self.format_json(event),
            LogFormat::Compact => self.format_compact(event),
        }
    }

    /// Log an event to the given writer
    pub fn log<W: Write>(&self, event: &Event, writer: &mut W) -> io::Result<()> {
        if event.risk_level < self.config.min_level {
            return Ok(());
        }

        let formatted = self.format(event);
        writeln!(writer, "{}", formatted)
    }

    /// Log an event to stdout
    pub fn log_stdout(&self, event: &Event) -> io::Result<()> {
        if event.risk_level < self.config.min_level {
            return Ok(());
        }

        let formatted = self.format(event);
        println!("{}", formatted);
        Ok(())
    }

    fn format_pretty(&self, event: &Event) -> String {
        let mut parts = Vec::new();

        // Timestamp
        if self.config.show_timestamps {
            let time = event.timestamp.format("%H:%M:%S").to_string();
            parts.push(if self.config.use_colors {
                time.dimmed().to_string()
            } else {
                time
            });
        }

        // Risk level emoji
        parts.push(event.risk_level.emoji().to_string());

        // Event details
        let details = match &event.event_type {
            EventType::Command {
                command,
                args,
                exit_code,
            } => {
                let cmd = if args.is_empty() {
                    command.clone()
                } else {
                    format!("{} {}", command, args.join(" "))
                };
                let exit = exit_code
                    .map(|c| format!(" (exit: {})", c))
                    .unwrap_or_default();

                if self.config.use_colors {
                    match event.risk_level {
                        RiskLevel::Critical => format!("{}{}", cmd.red().bold(), exit),
                        RiskLevel::High => format!("{}{}", cmd.yellow().bold(), exit),
                        RiskLevel::Medium => format!("{}{}", cmd.yellow(), exit),
                        RiskLevel::Low => format!("{}{}", cmd, exit),
                    }
                } else {
                    format!("{}{}", cmd, exit)
                }
            }
            EventType::FileAccess { path, action } => {
                let msg = format!("[{}] {}", action, path.display());
                if self.config.use_colors && event.risk_level >= RiskLevel::High {
                    msg.red().to_string()
                } else {
                    msg
                }
            }
            EventType::Network {
                host,
                port,
                protocol,
            } => {
                let msg = format!("[net] {}:{} ({})", host, port, protocol);
                if self.config.use_colors {
                    msg.blue().to_string()
                } else {
                    msg
                }
            }
            EventType::Process { pid, ppid, action } => {
                let ppid_str = ppid.map(|p| format!(" ppid:{}", p)).unwrap_or_default();
                format!("[proc] {:?} pid:{}{}", action, pid, ppid_str)
            }
            EventType::Session { action } => {
                let msg = format!("[session] {:?}", action);
                if self.config.use_colors {
                    msg.cyan().to_string()
                } else {
                    msg
                }
            }
        };

        parts.push(details);

        // Alert indicator
        if event.alert {
            let alert = "⚠️  ALERT";
            parts.push(if self.config.use_colors {
                alert.red().bold().to_string()
            } else {
                alert.to_string()
            });
        }

        parts.join("  ")
    }

    fn format_json(&self, event: &Event) -> String {
        serde_json::to_string(event).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
    }

    fn format_compact(&self, event: &Event) -> String {
        let time = event.timestamp.format("%H:%M:%S");
        let level = match event.risk_level {
            RiskLevel::Critical => "CRIT",
            RiskLevel::High => "HIGH",
            RiskLevel::Medium => "MED ",
            RiskLevel::Low => "LOW ",
        };

        let details = match &event.event_type {
            EventType::Command { command, args, .. } => {
                if args.is_empty() {
                    command.clone()
                } else {
                    format!("{} {}", command, args.join(" "))
                }
            }
            EventType::FileAccess { path, action } => {
                format!("{}:{}", action, path.display())
            }
            EventType::Network { host, port, .. } => {
                format!("net:{}:{}", host, port)
            }
            EventType::Process { pid, action, .. } => {
                format!("proc:{:?}:{}", action, pid)
            }
            EventType::Session { action } => {
                format!("session:{:?}", action)
            }
        };

        format!("{} [{}] {}", time, level, details)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::FileAction;
    use std::path::PathBuf;

    #[test]
    fn test_default_logger() {
        let logger = Logger::default();
        assert_eq!(logger.config.format, LogFormat::Pretty);
        assert_eq!(logger.config.min_level, RiskLevel::Low);
        assert!(logger.config.show_timestamps);
    }

    #[test]
    fn test_json_format() {
        let config = LoggerConfig {
            format: LogFormat::JsonLines,
            ..Default::default()
        };
        let logger = Logger::new(config);

        let event = Event::command(
            "ls".to_string(),
            vec!["-la".to_string()],
            "bash".to_string(),
            1234,
            RiskLevel::Low,
        );

        let output = logger.format(&event);
        assert!(output.contains("\"type\":\"command\""));
        assert!(output.contains("\"command\":\"ls\""));
        assert!(output.contains("\"risk_level\":\"low\""));
    }

    #[test]
    fn test_compact_format() {
        let config = LoggerConfig {
            format: LogFormat::Compact,
            ..Default::default()
        };
        let logger = Logger::new(config);

        let event = Event::command(
            "ls".to_string(),
            vec!["-la".to_string()],
            "bash".to_string(),
            1234,
            RiskLevel::Low,
        );

        let output = logger.format(&event);
        assert!(output.contains("[LOW ]"));
        assert!(output.contains("ls -la"));
    }

    #[test]
    fn test_pretty_format_with_colors() {
        let config = LoggerConfig {
            format: LogFormat::Pretty,
            use_colors: true,
            ..Default::default()
        };
        let logger = Logger::new(config);

        let event = Event::command(
            "rm".to_string(),
            vec!["-rf".to_string(), "/".to_string()],
            "bash".to_string(),
            1234,
            RiskLevel::Critical,
        );

        let output = logger.format(&event);
        assert!(output.contains("🔴"));
        assert!(output.contains("rm -rf /"));
        assert!(output.contains("ALERT"));
    }

    #[test]
    fn test_pretty_format_without_colors() {
        let config = LoggerConfig {
            format: LogFormat::Pretty,
            use_colors: false,
            ..Default::default()
        };
        let logger = Logger::new(config);

        let event = Event::command(
            "ls".to_string(),
            vec![],
            "bash".to_string(),
            1234,
            RiskLevel::Low,
        );

        let output = logger.format(&event);
        assert!(output.contains("🟢"));
        assert!(output.contains("ls"));
    }

    #[test]
    fn test_min_level_filtering() {
        let config = LoggerConfig {
            format: LogFormat::Compact,
            min_level: RiskLevel::High,
            ..Default::default()
        };
        let logger = Logger::new(config);

        let low_event = Event::command(
            "ls".to_string(),
            vec![],
            "bash".to_string(),
            1234,
            RiskLevel::Low,
        );

        let high_event = Event::command(
            "sudo".to_string(),
            vec!["rm".to_string()],
            "bash".to_string(),
            1234,
            RiskLevel::High,
        );

        let mut output = Vec::new();

        // Low event should be filtered
        logger.log(&low_event, &mut output).unwrap();
        assert!(output.is_empty());

        // High event should be logged
        logger.log(&high_event, &mut output).unwrap();
        assert!(!output.is_empty());
    }

    #[test]
    fn test_file_access_format() {
        let config = LoggerConfig {
            format: LogFormat::Pretty,
            use_colors: false,
            ..Default::default()
        };
        let logger = Logger::new(config);

        let event = Event::new(
            EventType::FileAccess {
                path: PathBuf::from("/home/user/.env"),
                action: FileAction::Read,
            },
            "claude-code".to_string(),
            5678,
            RiskLevel::Critical,
        );

        let output = logger.format(&event);
        assert!(output.contains("[read]"));
        assert!(output.contains(".env"));
    }

    #[test]
    fn test_network_format() {
        let config = LoggerConfig {
            format: LogFormat::Pretty,
            use_colors: false,
            ..Default::default()
        };
        let logger = Logger::new(config);

        let event = Event::new(
            EventType::Network {
                host: "api.anthropic.com".to_string(),
                port: 443,
                protocol: "tcp".to_string(),
            },
            "curl".to_string(),
            9999,
            RiskLevel::Medium,
        );

        let output = logger.format(&event);
        assert!(output.contains("[net]"));
        assert!(output.contains("api.anthropic.com:443"));
    }

    #[test]
    fn test_session_format() {
        let config = LoggerConfig {
            format: LogFormat::Compact,
            ..Default::default()
        };
        let logger = Logger::new(config);

        let event = Event::session_start("claude-code".to_string(), 1111);
        let output = logger.format(&event);
        assert!(output.contains("session:"));
    }

    #[test]
    fn test_log_to_writer() {
        let logger = Logger::default();
        let event = Event::command(
            "echo".to_string(),
            vec!["hello".to_string()],
            "bash".to_string(),
            1234,
            RiskLevel::Low,
        );

        let mut output = Vec::new();
        logger.log(&event, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("echo hello"));
    }

    #[test]
    fn test_without_timestamps() {
        let config = LoggerConfig {
            format: LogFormat::Pretty,
            show_timestamps: false,
            use_colors: false,
            ..Default::default()
        };
        let logger = Logger::new(config);

        let event = Event::command(
            "ls".to_string(),
            vec![],
            "bash".to_string(),
            1234,
            RiskLevel::Low,
        );

        let output = logger.format(&event);
        // Should not contain time pattern like "HH:MM:SS"
        assert!(!output.contains(':') || output.matches(':').count() <= 1); // Only in path or command
    }
}
