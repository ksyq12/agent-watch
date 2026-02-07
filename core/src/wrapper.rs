//! Process wrapper module with PTY support
//!
//! Wraps and monitors child processes, capturing their I/O and tracking commands.

use crate::detector::NetworkWhitelist;
use crate::event::{Event, RiskLevel};
use crate::fswatch::{FileSystemWatcher, FsWatchConfig};
use crate::logger::{Logger, LoggerConfig};
use crate::netmon::{NetMonConfig, NetworkMonitor};
use crate::process_tracker::{ProcessTracker, TrackerConfig, TrackerEvent};
use crate::risk::RiskScorer;
use crate::sanitize::sanitize_args;
use crate::storage::{EventStorage, SessionLogger};
use crate::types::MonitoringSubsystem;
use crate::error::CoreError;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::path::PathBuf;
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
    /// Enable file system monitoring
    pub enable_fswatch: bool,
    /// Paths to watch for file system changes
    pub watch_paths: Vec<PathBuf>,
    /// Enable network monitoring
    pub enable_netmon: bool,
    /// Network whitelist for allowed hosts
    pub network_whitelist: Option<NetworkWhitelist>,
    /// Session log directory (for JSON Lines logging)
    pub session_log_dir: Option<PathBuf>,
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
            enable_fswatch: false,
            watch_paths: Vec::new(),
            enable_netmon: false,
            network_whitelist: None,
            session_log_dir: None,
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

    /// Enable file system monitoring
    pub fn enable_fswatch(mut self, enabled: bool) -> Self {
        self.enable_fswatch = enabled;
        self
    }

    /// Set paths to watch for file system changes
    pub fn watch_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.watch_paths = paths;
        self
    }

    /// Enable network monitoring
    pub fn enable_netmon(mut self, enabled: bool) -> Self {
        self.enable_netmon = enabled;
        self
    }

    /// Set network whitelist
    pub fn network_whitelist(mut self, whitelist: NetworkWhitelist) -> Self {
        self.network_whitelist = Some(whitelist);
        self
    }

    /// Set session log directory
    pub fn session_log_dir(mut self, dir: PathBuf) -> Self {
        self.session_log_dir = Some(dir);
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
    /// File system event
    FileAccess {
        path: PathBuf,
        action: crate::event::FileAction,
        risk_level: RiskLevel,
    },
    /// Network connection event
    NetworkConnection {
        host: String,
        port: u16,
        protocol: String,
        risk_level: RiskLevel,
    },
}

/// Manages the lifecycle of all monitoring subsystems
struct MonitoringOrchestrator {
    tracker: Option<(ProcessTracker, thread::JoinHandle<()>)>,
    fs_watcher: Option<(FileSystemWatcher, thread::JoinHandle<()>)>,
    net_monitor: Option<(NetworkMonitor, thread::JoinHandle<()>)>,
}

impl MonitoringOrchestrator {
    /// Start all configured monitoring subsystems
    fn start(
        config: &WrapperConfig,
        pid: u32,
        risk_scorer: &RiskScorer,
        logger: &Logger,
        event_tx: &Option<Sender<WrapperEvent>>,
    ) -> Self {
        let fs_watcher = Self::start_fswatch(config, event_tx);
        let net_monitor = Self::start_netmon(config, pid, event_tx);
        let tracker = Self::start_tracker(config, pid, risk_scorer, logger, event_tx);

        Self {
            tracker,
            fs_watcher,
            net_monitor,
        }
    }

    /// Stop all monitoring subsystems gracefully using two-phase shutdown.
    /// Phase 1 signals all subsystems to stop (non-blocking), preventing new
    /// events from being generated. Phase 2 joins all threads.
    /// This avoids the race condition where events are lost because one
    /// subsystem is still running while another is being torn down.
    fn stop(self) {
        // Phase 1: Signal all subsystems to stop (non-blocking) via trait
        if let Some((ref tracker, _)) = self.tracker {
            MonitoringSubsystem::signal_stop(tracker);
        }
        if let Some((ref watcher, _)) = self.fs_watcher {
            MonitoringSubsystem::signal_stop(watcher);
        }
        if let Some((ref monitor, _)) = self.net_monitor {
            MonitoringSubsystem::signal_stop(monitor);
        }

        // Phase 2: Stop subsystems and join forwarding threads
        if let Some((mut tracker, handle)) = self.tracker {
            MonitoringSubsystem::stop(&mut tracker);
            let _ = handle.join();
        }
        if let Some((mut watcher, handle)) = self.fs_watcher {
            MonitoringSubsystem::stop(&mut watcher);
            let _ = handle.join();
        }
        if let Some((mut monitor, handle)) = self.net_monitor {
            MonitoringSubsystem::stop(&mut monitor);
            let _ = handle.join();
        }
    }

    fn start_fswatch(
        config: &WrapperConfig,
        event_tx: &Option<Sender<WrapperEvent>>,
    ) -> Option<(FileSystemWatcher, thread::JoinHandle<()>)> {
        if !config.enable_fswatch || config.watch_paths.is_empty() {
            return None;
        }

        let fs_config = FsWatchConfig::new(config.watch_paths.clone());
        let mut watcher = FileSystemWatcher::new(fs_config);
        let fs_rx = watcher.subscribe();
        let event_tx = event_tx.clone();

        if let Err(e) = watcher.start() {
            eprintln!("[agent-watch] Warning: Failed to start file system watcher: {e}");
            return None;
        }

        let handle = thread::spawn(move || {
            while let Ok(event) = fs_rx.recv() {
                if let crate::event::EventType::FileAccess { ref path, action } = event.event_type {
                    if let Some(ref tx) = event_tx {
                        let _ = tx.send(WrapperEvent::FileAccess {
                            path: path.clone(),
                            action,
                            risk_level: event.risk_level,
                        });
                    }
                }
            }
        });

        Some((watcher, handle))
    }

    fn start_netmon(
        config: &WrapperConfig,
        pid: u32,
        event_tx: &Option<Sender<WrapperEvent>>,
    ) -> Option<(NetworkMonitor, thread::JoinHandle<()>)> {
        if !config.enable_netmon || pid == 0 {
            return None;
        }

        let net_config = NetMonConfig::new(pid);
        let mut monitor = if let Some(ref whitelist) = config.network_whitelist {
            NetworkMonitor::new(net_config).with_whitelist(whitelist.clone())
        } else {
            NetworkMonitor::new(net_config)
        };
        let net_rx = monitor.subscribe();
        let event_tx = event_tx.clone();

        if let Err(e) = monitor.start() {
            eprintln!("[agent-watch] Warning: Failed to start network monitor: {e}");
            return None;
        }

        let handle = thread::spawn(move || {
            while let Ok(event) = net_rx.recv() {
                if let crate::event::EventType::Network {
                    ref host,
                    port,
                    ref protocol,
                } = event.event_type
                {
                    if let Some(ref tx) = event_tx {
                        let _ = tx.send(WrapperEvent::NetworkConnection {
                            host: host.clone(),
                            port,
                            protocol: protocol.clone(),
                            risk_level: event.risk_level,
                        });
                    }
                }
            }
        });

        Some((monitor, handle))
    }

    fn start_tracker(
        config: &WrapperConfig,
        pid: u32,
        risk_scorer: &RiskScorer,
        logger: &Logger,
        event_tx: &Option<Sender<WrapperEvent>>,
    ) -> Option<(ProcessTracker, thread::JoinHandle<()>)> {
        if !config.track_children || pid == 0 {
            return None;
        }

        let tracker_config =
            TrackerConfig::new(pid).poll_interval(Duration::from_millis(config.tracking_poll_ms));
        let mut tracker = ProcessTracker::new(tracker_config).with_risk_scorer(risk_scorer.clone());
        let tracker_rx = tracker.subscribe();
        let event_tx = event_tx.clone();
        let logger = logger.clone();

        tracker.start();

        let handle = thread::spawn(move || {
            while let Ok(tracker_event) = tracker_rx.recv() {
                match tracker_event {
                    TrackerEvent::ChildStarted {
                        pid,
                        ppid,
                        name,
                        path,
                        risk_level,
                    } => {
                        let event = Event::process_start(name.clone(), pid, Some(ppid), risk_level);
                        let _ = logger.log_stdout(&event);

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

        Some((tracker, handle))
    }
}

/// Process wrapper that monitors child process activity
pub struct ProcessWrapper {
    config: WrapperConfig,
    risk_scorer: RiskScorer,
    logger: Logger,
    event_tx: Option<Sender<WrapperEvent>>,
    /// Session logger for persistent event storage.
    /// Uses Mutex (not Arc) since it is only accessed from the main thread;
    /// Mutex provides the interior mutability needed for &self methods.
    session_logger: Option<Mutex<SessionLogger>>,
}

impl ProcessWrapper {
    /// Create a new process wrapper
    pub fn new(config: WrapperConfig) -> Self {
        let logger = Logger::new(config.logger_config.clone());
        let session_logger = config.session_log_dir.as_ref().and_then(|dir| {
            // Pass None for session_id to auto-generate timestamp-based ID
            match SessionLogger::new(dir, None) {
                Ok(l) => Some(Mutex::new(l)),
                Err(e) => {
                    eprintln!("[agent-watch] Warning: Failed to create session logger: {e}");
                    None
                }
            }
        });
        Self {
            config,
            risk_scorer: RiskScorer::new(),
            logger,
            event_tx: None,
            session_logger,
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
    pub fn run(&self) -> std::result::Result<i32, CoreError> {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows: self.config.pty_size.1,
                cols: self.config.pty_size.0,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| CoreError::Wrapper(format!("Failed to open PTY: {}", e)))?;

        let mut cmd = CommandBuilder::new(&self.config.command);
        cmd.args(&self.config.args);

        if let Some(ref cwd) = self.config.cwd {
            cmd.cwd(cwd);
        }

        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }

        // Spawn the child process
        let mut child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| CoreError::Wrapper(format!("Failed to spawn command: {}", e)))?;

        // Get child PID (platform-specific)
        let pid = child.process_id().unwrap_or(0);

        // Emit start event
        self.emit_event(WrapperEvent::Started { pid });
        self.log_session_start(pid);

        // Start all monitoring via orchestrator
        let orchestrator = MonitoringOrchestrator::start(
            &self.config,
            pid,
            &self.risk_scorer,
            &self.logger,
            &self.event_tx,
        );

        // Set up I/O handling
        let master = pair.master;

        // Create reader for master output
        let mut reader = master
            .try_clone_reader()
            .map_err(|e| CoreError::Wrapper(format!("Failed to clone reader: {}", e)))?;

        // Forward stdin to the PTY
        let writer = Arc::new(Mutex::new(
            master.take_writer().map_err(|e| CoreError::Wrapper(format!("Failed to take writer: {}", e)))?,
        ));
        let writer_clone = Arc::clone(&writer);

        // Spawn stdin forwarding thread
        let stdin_handle = thread::spawn(move || {
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

        let output_handle = thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            let mut line_buffer = String::new();
            // Cursor tracks how far into line_buffer we've already consumed,
            // avoiding O(n) drain/shift on every newline.
            let mut cursor: usize = 0;

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

                        // Parse for commands using cursor-based approach
                        line_buffer.push_str(&chunk);
                        while let Some(rel_pos) = line_buffer[cursor..].find('\n') {
                            let newline_pos = cursor + rel_pos;
                            let line = &line_buffer[cursor..newline_pos];

                            // Simple command detection from shell prompts
                            if let Some(cmd) = Self::detect_command(line) {
                                if let Some(ref tx) = event_tx {
                                    // Sanitize args before sending event
                                    let sanitized = crate::sanitize::sanitize_args(&cmd.1);
                                    let _ = tx.send(WrapperEvent::Command {
                                        command: cmd.0.clone(),
                                        args: sanitized,
                                    });
                                }
                            }

                            cursor = newline_pos + 1;
                        }

                        // Compact buffer when consumed portion exceeds 8KB
                        if cursor > 8192 {
                            line_buffer.drain(..cursor);
                            cursor = 0;
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
        let status = child.wait().map_err(|e| CoreError::Wrapper(format!("Failed to wait for child: {}", e)))?;
        let exit_code = status.exit_code();

        // Stop all monitoring
        orchestrator.stop();

        // Shutdown sequence for I/O threads:
        // 1. drop(writer) closes the PTY writer, causing the output reader to get EOF
        // 2. output_handle exits once it reads EOF from the PTY master
        // 3. stdin_handle exits because writer_clone.lock() succeeds but write_all
        //    fails (broken pipe), or stdin itself reaches EOF
        drop(writer);
        let _ = output_handle.join();
        let _ = stdin_handle.join();

        // Emit exit event
        self.emit_event(WrapperEvent::Exited {
            exit_code: Some(exit_code as i32),
        });
        self.log_session_end(pid);

        Ok(exit_code as i32)
    }

    /// Run a simple command without PTY (for testing or non-interactive use)
    pub fn run_simple(&self) -> std::result::Result<i32, CoreError> {
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
        let (risk_level, reason) = self
            .risk_scorer
            .score(&self.config.command, &self.config.args);

        // Log the command with sanitized args
        let sanitized_args = sanitize_args(&self.config.args);
        let event = Event::command(
            self.config.command.clone(),
            sanitized_args,
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

        let status = cmd.status().map_err(|e| CoreError::Wrapper(format!("Failed to execute command: {}", e)))?;

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
        if let Some(ref logger) = self.session_logger {
            if let Ok(mut l) = logger.lock() {
                if let Err(e) = l.write_event(&event) {
                    eprintln!("[agent-watch] Warning: Failed to log session start: {e}");
                }
            }
        }
        self.emit_event(WrapperEvent::Event(event));
    }

    fn log_session_end(&self, pid: u32) {
        let event = Event::session_end(self.config.command.clone(), pid);
        let _ = self.logger.log_stdout(&event);
        if let Some(ref logger) = self.session_logger {
            if let Ok(mut l) = logger.lock() {
                if let Err(e) = l.write_event(&event) {
                    eprintln!("[agent-watch] Warning: Failed to log session end: {e}");
                }
                if let Err(e) = l.flush() {
                    eprintln!("[agent-watch] Warning: Failed to flush session log: {e}");
                }
            }
        }
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
            if pos == 0
                || line
                    .chars()
                    .nth(pos - 1)
                    .map(|c| c.is_whitespace())
                    .unwrap_or(true)
            {
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
        config
            .env
            .push(("TEST_VAR".to_string(), "hello".to_string()));

        let wrapper = ProcessWrapper::new(config);
        let result = wrapper.run_simple();

        assert!(result.is_ok());
    }

    #[test]
    fn test_risk_scoring_integration() {
        let config =
            WrapperConfig::new("rm").args(vec!["-rf".to_string(), "/tmp/test".to_string()]);

        let wrapper = ProcessWrapper::new(config);

        // The wrapper should score this as high risk
        let (level, _) = wrapper
            .risk_scorer
            .score("rm", &["-rf".to_string(), "/tmp/test".to_string()]);
        assert_eq!(level, RiskLevel::High);
    }

    // --- MonitoringOrchestrator integration tests ---

    #[test]
    fn test_orchestrator_no_subsystems() {
        // When all subsystems are disabled, start/stop should work without error
        let config = WrapperConfig::new("echo")
            .track_children(false)
            .enable_fswatch(false)
            .enable_netmon(false);

        let risk_scorer = RiskScorer::new();
        let logger = Logger::new(config.logger_config.clone());
        let event_tx: Option<Sender<WrapperEvent>> = None;

        let orchestrator =
            MonitoringOrchestrator::start(&config, 0, &risk_scorer, &logger, &event_tx);

        // All subsystems should be None
        assert!(orchestrator.tracker.is_none());
        assert!(orchestrator.fs_watcher.is_none());
        assert!(orchestrator.net_monitor.is_none());

        orchestrator.stop();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_orchestrator_with_tracker_only() {
        let config = WrapperConfig::new("echo")
            .track_children(true)
            .tracking_poll_ms(50)
            .enable_fswatch(false)
            .enable_netmon(false);

        let risk_scorer = RiskScorer::new();
        let logger = Logger::new(config.logger_config.clone());
        let (tx, _rx) = mpsc::channel();
        let event_tx = Some(tx);

        let pid = std::process::id();
        let orchestrator =
            MonitoringOrchestrator::start(&config, pid, &risk_scorer, &logger, &event_tx);

        assert!(orchestrator.tracker.is_some());
        assert!(orchestrator.fs_watcher.is_none());
        assert!(orchestrator.net_monitor.is_none());

        // Let it run briefly
        std::thread::sleep(Duration::from_millis(100));

        // Decompose and stop subsystems, dropping them before joining forwarding threads
        let MonitoringOrchestrator {
            tracker,
            fs_watcher,
            net_monitor,
        } = orchestrator;

        if let Some((mut t, handle)) = tracker {
            t.stop();
            drop(t);
            let _ = handle.join();
        }
        assert!(fs_watcher.is_none());
        assert!(net_monitor.is_none());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_orchestrator_with_fswatch_only() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config = WrapperConfig::new("echo")
            .track_children(false)
            .enable_fswatch(true)
            .watch_paths(vec![temp_dir.path().to_path_buf()])
            .enable_netmon(false);

        let risk_scorer = RiskScorer::new();
        let logger = Logger::new(config.logger_config.clone());
        let (tx, _rx) = mpsc::channel();
        let event_tx = Some(tx);

        let orchestrator =
            MonitoringOrchestrator::start(&config, 1, &risk_scorer, &logger, &event_tx);

        assert!(orchestrator.tracker.is_none());
        assert!(orchestrator.fs_watcher.is_some());
        assert!(orchestrator.net_monitor.is_none());

        std::thread::sleep(Duration::from_millis(100));

        // Decompose, drop subsystem, then join forwarding thread
        let MonitoringOrchestrator {
            tracker,
            fs_watcher,
            net_monitor,
        } = orchestrator;

        assert!(tracker.is_none());
        assert!(net_monitor.is_none());
        if let Some((mut w, handle)) = fs_watcher {
            w.stop();
            drop(w);
            let _ = handle.join();
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_orchestrator_with_netmon_only() {
        let config = WrapperConfig::new("echo")
            .track_children(false)
            .enable_fswatch(false)
            .enable_netmon(true);

        let risk_scorer = RiskScorer::new();
        let logger = Logger::new(config.logger_config.clone());
        let (tx, _rx) = mpsc::channel();
        let event_tx = Some(tx);

        let pid = std::process::id();
        let orchestrator =
            MonitoringOrchestrator::start(&config, pid, &risk_scorer, &logger, &event_tx);

        assert!(orchestrator.tracker.is_none());
        assert!(orchestrator.fs_watcher.is_none());
        assert!(orchestrator.net_monitor.is_some());

        std::thread::sleep(Duration::from_millis(100));

        // Decompose, drop subsystem, then join forwarding thread
        let MonitoringOrchestrator {
            tracker,
            fs_watcher,
            net_monitor,
        } = orchestrator;

        assert!(tracker.is_none());
        assert!(fs_watcher.is_none());
        if let Some((mut m, handle)) = net_monitor {
            m.stop();
            drop(m);
            let _ = handle.join();
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_orchestrator_all_subsystems() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config = WrapperConfig::new("echo")
            .track_children(true)
            .tracking_poll_ms(50)
            .enable_fswatch(true)
            .watch_paths(vec![temp_dir.path().to_path_buf()])
            .enable_netmon(true);

        let risk_scorer = RiskScorer::new();
        let logger = Logger::new(config.logger_config.clone());
        let (tx, _rx) = mpsc::channel();
        let event_tx = Some(tx);

        let pid = std::process::id();
        let orchestrator =
            MonitoringOrchestrator::start(&config, pid, &risk_scorer, &logger, &event_tx);

        assert!(orchestrator.tracker.is_some());
        assert!(orchestrator.fs_watcher.is_some());
        assert!(orchestrator.net_monitor.is_some());

        std::thread::sleep(Duration::from_millis(100));

        // Phase 1: Signal all to stop
        if let Some((ref t, _)) = orchestrator.tracker {
            t.signal_stop();
        }
        if let Some((ref w, _)) = orchestrator.fs_watcher {
            w.signal_stop();
        }
        if let Some((ref m, _)) = orchestrator.net_monitor {
            m.signal_stop();
        }

        // Phase 2: Stop and drop each subsystem, then join forwarding thread
        let MonitoringOrchestrator {
            tracker,
            fs_watcher,
            net_monitor,
        } = orchestrator;

        if let Some((mut t, handle)) = tracker {
            t.stop();
            drop(t);
            let _ = handle.join();
        }
        if let Some((mut w, handle)) = fs_watcher {
            w.stop();
            drop(w);
            let _ = handle.join();
        }
        if let Some((mut m, handle)) = net_monitor {
            m.stop();
            drop(m);
            let _ = handle.join();
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_orchestrator_two_phase_shutdown() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config = WrapperConfig::new("echo")
            .track_children(true)
            .tracking_poll_ms(50)
            .enable_fswatch(true)
            .watch_paths(vec![temp_dir.path().to_path_buf()])
            .enable_netmon(true);

        let risk_scorer = RiskScorer::new();
        let logger = Logger::new(config.logger_config.clone());
        let (tx, _rx) = mpsc::channel();
        let event_tx = Some(tx);

        let pid = std::process::id();
        let orchestrator =
            MonitoringOrchestrator::start(&config, pid, &risk_scorer, &logger, &event_tx);

        std::thread::sleep(Duration::from_millis(100));

        // Phase 1: Signal all subsystems to stop (non-blocking)
        if let Some((ref t, _)) = orchestrator.tracker {
            t.signal_stop();
        }
        if let Some((ref w, _)) = orchestrator.fs_watcher {
            w.signal_stop();
        }
        if let Some((ref m, _)) = orchestrator.net_monitor {
            m.signal_stop();
        }

        // Phase 2: Stop, drop, and join each subsystem
        let MonitoringOrchestrator {
            tracker,
            fs_watcher,
            net_monitor,
        } = orchestrator;

        if let Some((mut t, handle)) = tracker {
            t.stop();
            drop(t);
            let _ = handle.join();
        }
        if let Some((mut w, handle)) = fs_watcher {
            w.stop();
            drop(w);
            let _ = handle.join();
        }
        if let Some((mut m, handle)) = net_monitor {
            m.stop();
            drop(m);
            let _ = handle.join();
        }
        // If we get here without hanging, the two-phase shutdown worked
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_orchestrator_fswatch_event_delivery() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config = WrapperConfig::new("echo")
            .track_children(false)
            .enable_fswatch(true)
            .watch_paths(vec![temp_dir.path().to_path_buf()])
            .enable_netmon(false);

        let risk_scorer = RiskScorer::new();
        let logger = Logger::new(config.logger_config.clone());
        let (tx, rx) = mpsc::channel();
        let event_tx = Some(tx);

        let orchestrator =
            MonitoringOrchestrator::start(&config, 1, &risk_scorer, &logger, &event_tx);

        // Give FSEvents time to start
        std::thread::sleep(Duration::from_millis(500));

        // Create a file in the watched directory
        let test_file = temp_dir.path().join("orchestrator_test.txt");
        fs::write(&test_file, "test content").unwrap();

        // Wait for event delivery
        std::thread::sleep(Duration::from_millis(1000));

        // Stop, drop, then join to avoid deadlock
        let MonitoringOrchestrator {
            tracker,
            fs_watcher,
            net_monitor,
        } = orchestrator;

        assert!(tracker.is_none());
        assert!(net_monitor.is_none());
        if let Some((mut w, handle)) = fs_watcher {
            w.stop();
            drop(w);
            let _ = handle.join();
        }

        // Check that we received a FileAccess event through the wrapper channel
        let mut found_file_event = false;
        while let Ok(event) = rx.try_recv() {
            if let WrapperEvent::FileAccess { ref path, .. } = event {
                if path.to_string_lossy().contains("orchestrator_test.txt") {
                    found_file_event = true;
                    break;
                }
            }
        }
        assert!(
            found_file_event,
            "Should have received a FileAccess event through the orchestrator"
        );
    }

    #[test]
    fn test_wrapper_lifecycle_run_simple() {
        // Test the full wrapper lifecycle: create -> subscribe -> run -> check events
        let config = WrapperConfig::new("sh")
            .args(vec!["-c".to_string(), "echo hello && exit 0".to_string()])
            .track_children(false)
            .enable_fswatch(false)
            .enable_netmon(false);

        let mut wrapper = ProcessWrapper::new(config);
        let rx = wrapper.subscribe();

        let exit_code = wrapper.run_simple().unwrap();
        assert_eq!(exit_code, 0);

        // Collect all events
        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }

        // Should have received at least the command Event
        assert!(
            events.iter().any(|e| matches!(e, WrapperEvent::Event(_))),
            "Should have received at least one Event"
        );
    }

    #[test]
    fn test_orchestrator_fswatch_disabled_with_empty_paths() {
        // Even if fswatch is enabled, empty watch_paths should result in no watcher
        let config = WrapperConfig::new("echo")
            .track_children(false)
            .enable_fswatch(true)
            .watch_paths(vec![])
            .enable_netmon(false);

        let risk_scorer = RiskScorer::new();
        let logger = Logger::new(config.logger_config.clone());
        let event_tx: Option<Sender<WrapperEvent>> = None;

        let orchestrator =
            MonitoringOrchestrator::start(&config, 1, &risk_scorer, &logger, &event_tx);

        assert!(orchestrator.fs_watcher.is_none());
        orchestrator.stop();
    }

    #[test]
    fn test_orchestrator_netmon_disabled_with_zero_pid() {
        // netmon should not start if pid is 0
        let config = WrapperConfig::new("echo")
            .track_children(false)
            .enable_fswatch(false)
            .enable_netmon(true);

        let risk_scorer = RiskScorer::new();
        let logger = Logger::new(config.logger_config.clone());
        let event_tx: Option<Sender<WrapperEvent>> = None;

        let orchestrator =
            MonitoringOrchestrator::start(&config, 0, &risk_scorer, &logger, &event_tx);

        assert!(orchestrator.net_monitor.is_none());
        orchestrator.stop();
    }

    #[test]
    fn test_orchestrator_tracker_disabled_with_zero_pid() {
        // tracker should not start if pid is 0
        let config = WrapperConfig::new("echo")
            .track_children(true)
            .enable_fswatch(false)
            .enable_netmon(false);

        let risk_scorer = RiskScorer::new();
        let logger = Logger::new(config.logger_config.clone());
        let event_tx: Option<Sender<WrapperEvent>> = None;

        let orchestrator =
            MonitoringOrchestrator::start(&config, 0, &risk_scorer, &logger, &event_tx);

        assert!(orchestrator.tracker.is_none());
        orchestrator.stop();
    }
}
