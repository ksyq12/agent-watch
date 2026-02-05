//! Process wrapper module with PTY support
//!
//! Wraps and monitors child processes, capturing their I/O and tracking commands.

use crate::event::{Event, RiskLevel};
use crate::logger::{Logger, LoggerConfig};
use crate::process_tracker::{ProcessTracker, TrackerConfig, TrackerEvent};
use crate::risk::RiskScorer;
use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Configuration for the process wrapper
#[derive(Debug, Clone)]
pub struct WrapperConfig {
    /// Command to execute
    pub command: String,
    /// Command arguments
    pub args: Vec<String>,
    /// Working directory
    pub cwd: Option<String>,
    /// Environment variables to set
    pub env: Vec<(String, String)>,
    /// PTY size (columns, rows)
    pub pty_size: (u16, u16),
    /// Logger configuration
    pub logger_config: LoggerConfig,
    /// Enable child process tracking
    pub track_children: bool,
    /// Polling interval for child process tracking (milliseconds)
    pub tracking_poll_ms: u64,
}

impl Default for WrapperConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
            pty_size: (80, 24),
            logger_config: LoggerConfig::default(),
            track_children: true,
            tracking_poll_ms: 100,
        }
    }
}

impl WrapperConfig {
    /// Create a new wrapper config for a command
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            ..Default::default()
        }
    }

    /// Add arguments
    pub fn args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Set working directory
    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Set PTY size
    pub fn pty_size(mut self, cols: u16, rows: u16) -> Self {
        self.pty_size = (cols, rows);
        self
    }

    /// Set logger config
    pub fn logger_config(mut self, config: LoggerConfig) -> Self {
        self.logger_config = config;
        self
    }

    /// Enable or disable child process tracking
    pub fn track_children(mut self, enabled: bool) -> Self {
        self.track_children = enabled;
        self
    }

    /// Set the polling interval for child process tracking (in milliseconds)
    pub fn tracking_poll_ms(mut self, ms: u64) -> Self {
        self.tracking_poll_ms = ms;
        self
    }
}

/// Event emitted by the wrapper
#[derive(Debug, Clone)]
pub enum WrapperEvent {
    /// Process started
    Started { pid: u32 },
    /// Output from stdout
    Stdout(String),
    /// Output from stderr
    Stderr(String),
    /// Command detected in output
    Command { command: String, args: Vec<String> },
    /// Process exited
    Exited { exit_code: Option<i32> },
    /// Monitoring event
    Event(Event),
    /// Child process started
    ChildStarted {
        pid: u32,
        ppid: u32,
        name: String,
        path: Option<String>,
        risk_level: RiskLevel,
    },
    /// Child process exited
    ChildExited { pid: u32 },
}

/// Process wrapper that monitors child process activity
pub struct ProcessWrapper {
    config: WrapperConfig,
    risk_scorer: RiskScorer,
    logger: Logger,
    event_tx: Option<Sender<WrapperEvent>>,
}

impl ProcessWrapper {
    /// Create a new process wrapper
    pub fn new(config: WrapperConfig) -> Self {
        let logger = Logger::new(config.logger_config.clone());
        Self {
            config,
            risk_scorer: RiskScorer::new(),
            logger,
            event_tx: None,
        }
    }

    /// Create with a custom risk scorer
    pub fn with_risk_scorer(mut self, scorer: RiskScorer) -> Self {
        self.risk_scorer = scorer;
        self
    }

    /// Subscribe to wrapper events
    pub fn subscribe(&mut self) -> Receiver<WrapperEvent> {
        let (tx, rx) = mpsc::channel();
        self.event_tx = Some(tx);
        rx
    }

    /// Run the wrapped process with PTY
    pub fn run(&self) -> Result<i32> {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows: self.config.pty_size.1,
                cols: self.config.pty_size.0,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to open PTY")?;

        let mut cmd = CommandBuilder::new(&self.config.command);
        cmd.args(&self.config.args);

        if let Some(ref cwd) = self.config.cwd {
            cmd.cwd(cwd);
        }

        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }

        // Spawn the child process
        let mut child = pair.slave.spawn_command(cmd).context("Failed to spawn command")?;

        // Get child PID (platform-specific)
        let pid = child.process_id().unwrap_or(0);

        // Emit start event
        self.emit_event(WrapperEvent::Started { pid });
        self.log_session_start(pid);

        // Start child process tracker if enabled
        let tracker_handle = if self.config.track_children && pid != 0 {
            let tracker_config = TrackerConfig::new(pid)
                .poll_interval(Duration::from_millis(self.config.tracking_poll_ms));
            let mut tracker = ProcessTracker::new(tracker_config)
                .with_risk_scorer(self.risk_scorer.clone());
            let tracker_rx = tracker.subscribe();
            let event_tx = self.event_tx.clone();
            let logger = self.logger.clone();
            let _process_name = self.config.command.clone();

            tracker.start();

            // Spawn thread to forward tracker events
            let forward_handle = thread::spawn(move || {
                while let Ok(tracker_event) = tracker_rx.recv() {
                    match tracker_event {
                        TrackerEvent::ChildStarted { pid, ppid, name, path, risk_level } => {
                            // Log the child process
                            let event = Event::process_start(
                                name.clone(),
                                pid,
                                Some(ppid),
                                risk_level,
                            );
                            let _ = logger.log_stdout(&event);

                            // Forward to wrapper event subscribers
                            if let Some(ref tx) = event_tx {
                                let _ = tx.send(WrapperEvent::ChildStarted {
                                    pid,
                                    ppid,
                                    name,
                                    path,
                                    risk_level,
                                });
                            }
                        }
                        TrackerEvent::ChildExited { pid } => {
                            if let Some(ref tx) = event_tx {
                                let _ = tx.send(WrapperEvent::ChildExited { pid });
                            }
                        }
                    }
                }
            });

            Some((tracker, forward_handle))
        } else {
            None
        };

        // Set up I/O handling
        let master = pair.master;

        // Create reader for master output
        let mut reader = master.try_clone_reader().context("Failed to clone reader")?;

        // Forward stdin to the PTY
        let writer = Arc::new(Mutex::new(
            master.take_writer().context("Failed to take writer")?,
        ));
        let writer_clone = Arc::clone(&writer);

        // Spawn stdin forwarding thread
        let _stdin_handle = thread::spawn(move || {
            let stdin = std::io::stdin();
            let mut stdin_lock = stdin.lock();
            let mut buffer = [0u8; 1024];

            loop {
                match stdin_lock.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if let Ok(mut writer) = writer_clone.lock() {
                            let _ = writer.write_all(&buffer[..n]);
                            let _ = writer.flush();
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Read and process output
        let event_tx = self.event_tx.clone();
        let _process_name = self.config.command.clone();
        let _risk_scorer = self.risk_scorer.clone();

        let output_handle = thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            let mut line_buffer = String::new();

            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let chunk = String::from_utf8_lossy(&buffer[..n]);

                        // Output to stdout
                        print!("{}", chunk);
                        let _ = std::io::stdout().flush();

                        // Emit stdout event
                        if let Some(ref tx) = event_tx {
                            let _ = tx.send(WrapperEvent::Stdout(chunk.to_string()));
                        }

                        // Parse for commands (simple detection)
                        line_buffer.push_str(&chunk);
                        while let Some(newline_pos) = line_buffer.find('\n') {
                            let line = line_buffer[..newline_pos].to_string();
                            line_buffer = line_buffer[newline_pos + 1..].to_string();

                            // Simple command detection from shell prompts
                            if let Some(cmd) = Self::detect_command(&line) {
                                if let Some(ref tx) = event_tx {
                                    let _ = tx.send(WrapperEvent::Command {
                                        command: cmd.0.clone(),
                                        args: cmd.1.clone(),
                                    });
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Read error: {}", e);
                        break;
                    }
                }
            }
        });

        // Wait for the child process to exit
        let status = child.wait().context("Failed to wait for child")?;
        let exit_code = status.exit_code();

        // Stop the process tracker
        if let Some((mut tracker, forward_handle)) = tracker_handle {
            tracker.stop();
            // Give the forward thread a chance to finish
            let _ = forward_handle.join();
        }

        // Signal threads to stop and wait for them
        drop(writer);
        let _ = output_handle.join();
        // stdin_handle will exit when stdin closes or process exits

        // Emit exit event
        self.emit_event(WrapperEvent::Exited { exit_code: Some(exit_code as i32) });
        self.log_session_end(pid);

        Ok(exit_code as i32)
    }

    /// Run a simple command without PTY (for testing or non-interactive use)
    pub fn run_simple(&self) -> Result<i32> {
        use std::process::{Command, Stdio};

        let mut cmd = Command::new(&self.config.command);
        cmd.args(&self.config.args);

        if let Some(ref cwd) = self.config.cwd {
            cmd.current_dir(cwd);
        }

        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }

        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        // Score the command
        let (risk_level, reason) = self.risk_scorer.score(&self.config.command, &self.config.args);

        // Log the command
        let event = Event::command(
            self.config.command.clone(),
            self.config.args.clone(),
            self.config.command.clone(),
            std::process::id(),
            risk_level,
        );
        let _ = self.logger.log_stdout(&event);
        self.emit_event(WrapperEvent::Event(event));

        if let Some(reason) = reason {
            if risk_level >= RiskLevel::High {
                eprintln!("⚠️  Warning: {}", reason);
            }
        }

        let status = cmd.status().context("Failed to execute command")?;

        Ok(status.code().unwrap_or(-1))
    }

    fn emit_event(&self, event: WrapperEvent) {
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(event);
        }
    }

    fn log_session_start(&self, pid: u32) {
        let event = Event::session_start(self.config.command.clone(), pid);
        let _ = self.logger.log_stdout(&event);
        self.emit_event(WrapperEvent::Event(event));
    }

    fn log_session_end(&self, pid: u32) {
        let event = Event::session_end(self.config.command.clone(), pid);
        let _ = self.logger.log_stdout(&event);
        self.emit_event(WrapperEvent::Event(event));
    }

    /// Simple command detection from output line
    /// Looks for common shell prompt patterns
    fn detect_command(line: &str) -> Option<(String, Vec<String>)> {
        let line = line.trim();

        // Skip empty lines and common non-command patterns
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with("//")
            || line.starts_with("/*")
        {
            return None;
        }

        // Look for shell prompt patterns and extract command
        // Common patterns: "$ cmd", "% cmd", "> cmd", "user@host:~$ cmd"
        let command_part = if let Some(pos) = line.rfind("$ ") {
            &line[pos + 2..]
        } else if let Some(pos) = line.rfind("% ") {
            &line[pos + 2..]
        } else if let Some(pos) = line.rfind("> ") {
            // Be careful with > as it's also used for redirection
            if pos == 0 || line.chars().nth(pos - 1).map(|c| c.is_whitespace()).unwrap_or(true) {
                &line[pos + 2..]
            } else {
                return None;
            }
        } else {
            return None;
        };

        // Parse the command
        let parts: Vec<&str> = command_part.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let cmd = parts[0].to_string();
        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

        Some((cmd, args))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrapper_config_builder() {
        let config = WrapperConfig::new("ls")
            .args(vec!["-la".to_string()])
            .cwd("/home")
            .pty_size(120, 40);

        assert_eq!(config.command, "ls");
        assert_eq!(config.args, vec!["-la"]);
        assert_eq!(config.cwd, Some("/home".to_string()));
        assert_eq!(config.pty_size, (120, 40));
    }

    #[test]
    fn test_wrapper_config_default() {
        let config = WrapperConfig::default();
        assert!(config.command.is_empty());
        assert!(config.args.is_empty());
        assert_eq!(config.pty_size, (80, 24));
    }

    #[test]
    fn test_detect_command_dollar_prompt() {
        let result = ProcessWrapper::detect_command("$ ls -la");
        assert_eq!(result, Some(("ls".to_string(), vec!["-la".to_string()])));
    }

    #[test]
    fn test_detect_command_user_prompt() {
        let result = ProcessWrapper::detect_command("user@host:~/project$ git status");
        assert_eq!(
            result,
            Some(("git".to_string(), vec!["status".to_string()]))
        );
    }

    #[test]
    fn test_detect_command_percent_prompt() {
        let result = ProcessWrapper::detect_command("% echo hello");
        assert_eq!(
            result,
            Some(("echo".to_string(), vec!["hello".to_string()]))
        );
    }

    #[test]
    fn test_detect_command_no_prompt() {
        let result = ProcessWrapper::detect_command("just some output");
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_command_empty() {
        assert!(ProcessWrapper::detect_command("").is_none());
        assert!(ProcessWrapper::detect_command("   ").is_none());
    }

    #[test]
    fn test_detect_command_comment() {
        assert!(ProcessWrapper::detect_command("# this is a comment").is_none());
        assert!(ProcessWrapper::detect_command("// another comment").is_none());
    }

    #[test]
    fn test_run_simple_command() {
        let config = WrapperConfig::new("echo").args(vec!["hello".to_string()]);

        let wrapper = ProcessWrapper::new(config);
        let result = wrapper.run_simple();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_run_simple_with_exit_code() {
        let config = WrapperConfig::new("sh").args(vec!["-c".to_string(), "exit 42".to_string()]);

        let wrapper = ProcessWrapper::new(config);
        let result = wrapper.run_simple();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_wrapper_event_subscription() {
        let config = WrapperConfig::new("echo").args(vec!["test".to_string()]);

        let mut wrapper = ProcessWrapper::new(config);
        let rx = wrapper.subscribe();

        // Run and check events
        let _ = wrapper.run_simple();

        // Should receive at least one event (the command event)
        let mut received_events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            received_events.push(event);
        }

        assert!(!received_events.is_empty());
    }

    #[test]
    fn test_wrapper_with_env() {
        let config = WrapperConfig::new("sh")
            .args(vec!["-c".to_string(), "echo $TEST_VAR".to_string()])
            .cwd("/tmp");

        let mut config = config;
        config.env.push(("TEST_VAR".to_string(), "hello".to_string()));

        let wrapper = ProcessWrapper::new(config);
        let result = wrapper.run_simple();

        assert!(result.is_ok());
    }

    #[test]
    fn test_risk_scoring_integration() {
        let config = WrapperConfig::new("rm").args(vec!["-rf".to_string(), "/tmp/test".to_string()]);

        let wrapper = ProcessWrapper::new(config);

        // The wrapper should score this as high risk
        let (level, _) = wrapper.risk_scorer.score("rm", &["-rf".to_string(), "/tmp/test".to_string()]);
        assert_eq!(level, RiskLevel::High);
    }
}
