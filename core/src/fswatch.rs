//! File system monitoring module for MacAgentWatch
//!
//! Uses macOS FSEvents API to monitor file system changes.
//! Detects file access patterns and integrates with sensitive file detection.

use crate::detector::{Detector, SensitiveFileDetector};
use crate::event::{Event, EventType, FileAction, RiskLevel};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// File system watcher configuration
#[derive(Debug, Clone)]
pub struct FsWatchConfig {
    /// Paths to watch
    pub watch_paths: Vec<PathBuf>,
    /// Latency for FSEvents (how long to coalesce events)
    pub latency: Duration,
}

impl Default for FsWatchConfig {
    fn default() -> Self {
        Self {
            watch_paths: Vec::new(),
            latency: Duration::from_millis(100),
        }
    }
}

impl FsWatchConfig {
    /// Create a new config with watch paths
    pub fn new(watch_paths: Vec<PathBuf>) -> Self {
        Self {
            watch_paths,
            ..Default::default()
        }
    }

    /// Set latency
    pub fn latency(mut self, latency: Duration) -> Self {
        self.latency = latency;
        self
    }

    /// Add a watch path
    pub fn add_path(mut self, path: PathBuf) -> Self {
        self.watch_paths.push(path);
        self
    }
}

/// File system event from FSEvents
#[derive(Debug, Clone)]
pub struct FsEvent {
    /// Path that changed
    pub path: PathBuf,
    /// Type of change
    pub action: FileAction,
    /// FSEvents flags (raw)
    pub flags: u32,
}

impl FsEvent {
    /// Create a new file system event
    pub fn new(path: PathBuf, action: FileAction, flags: u32) -> Self {
        Self {
            path,
            action,
            flags,
        }
    }
}

/// File system watcher using macOS FSEvents
pub struct FileSystemWatcher {
    config: FsWatchConfig,
    detector: SensitiveFileDetector,
    event_tx: Option<Sender<Event>>,
    stop_flag: Arc<AtomicBool>,
    watch_thread: Option<JoinHandle<()>>,
}

impl FileSystemWatcher {
    /// Create a new file system watcher
    pub fn new(config: FsWatchConfig) -> Self {
        Self {
            config,
            detector: SensitiveFileDetector::default(),
            event_tx: None,
            stop_flag: Arc::new(AtomicBool::new(false)),
            watch_thread: None,
        }
    }

    /// Create with a custom detector
    pub fn with_detector(mut self, detector: SensitiveFileDetector) -> Self {
        self.detector = detector;
        self
    }

    /// Subscribe to file system events
    pub fn subscribe(&mut self) -> Receiver<Event> {
        let (tx, rx) = channel();
        self.event_tx = Some(tx);
        rx
    }

    /// Check if watcher is running
    pub fn is_running(&self) -> bool {
        self.watch_thread.is_some() && !self.stop_flag.load(Ordering::Relaxed)
    }

    /// Start watching file system
    #[cfg(target_os = "macos")]
    pub fn start(&mut self) -> Result<()> {
        if self.config.watch_paths.is_empty() {
            return Ok(());
        }

        self.stop_flag.store(false, Ordering::Relaxed);

        let paths: Vec<String> = self
            .config
            .watch_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        let latency_secs = self.config.latency.as_secs_f64();
        let event_tx = self.event_tx.clone();
        let detector = self.detector.clone();
        let stop_flag = self.stop_flag.clone();

        // Spawn a thread that owns the FsEvent
        let handle = thread::spawn(move || {
            Self::watch_thread(paths, latency_secs, event_tx, detector, stop_flag);
        });

        self.watch_thread = Some(handle);
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn start(&mut self) -> Result<()> {
        // No-op on non-macOS platforms
        Ok(())
    }

    /// The main watch thread that creates and manages FsEvent
    #[cfg(target_os = "macos")]
    fn watch_thread(
        paths: Vec<String>,
        _latency_secs: f64,
        event_tx: Option<Sender<Event>>,
        detector: SensitiveFileDetector,
        stop_flag: Arc<AtomicBool>,
    ) {
        // Channel for FSEvents
        let (fs_tx, fs_rx) = channel::<fsevent::Event>();

        // Create FSEvent watcher in this thread
        let mut fs_event = fsevent::FsEvent::new(paths);

        // Start observation (this blocks internally so we use observe_async)
        if fs_event.observe_async(fs_tx).is_err() {
            return;
        }

        // Use catch_unwind to ensure FSEvents cleanup even on panic (C6 fix)
        let loop_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            loop {
                if stop_flag.load(Ordering::Relaxed) {
                    break;
                }

                match fs_rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(fse) => {
                        let path = PathBuf::from(&fse.path);
                        let action = Self::flags_to_action(fse.flag);

                        let risk_level = if detector.is_sensitive(&path) {
                            RiskLevel::Critical
                        } else {
                            RiskLevel::Low
                        };

                        let event = Event::new(
                            EventType::FileAccess {
                                path: path.clone(),
                                action,
                            },
                            "fswatch".to_string(),
                            std::process::id(),
                            risk_level,
                        );

                        if let Some(ref tx) = event_tx {
                            let _ = tx.send(event);
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        }));

        // Always shutdown FSEvents, even after panic
        fs_event.shutdown_observe();

        // Re-raise panic if one occurred
        if let Err(panic_err) = loop_result {
            std::panic::resume_unwind(panic_err);
        }
    }

    /// Stop watching
    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);

        if let Some(handle) = self.watch_thread.take() {
            let _ = handle.join();
        }
    }

    /// Signal the watcher to stop without waiting for the thread to finish.
    /// Used by MonitoringOrchestrator for two-phase shutdown.
    pub fn signal_stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    /// Convert FSEvents flags to FileAction
    #[cfg(target_os = "macos")]
    fn flags_to_action(flags: fsevent::StreamFlags) -> FileAction {
        use fsevent::StreamFlags;

        if flags.contains(StreamFlags::ITEM_REMOVED) {
            FileAction::Delete
        } else if flags.contains(StreamFlags::ITEM_CREATED) {
            FileAction::Create
        } else if flags.contains(StreamFlags::ITEM_MODIFIED) {
            FileAction::Write
        } else if flags.contains(StreamFlags::ITEM_RENAMED) {
            FileAction::Write
        } else if flags.contains(StreamFlags::ITEM_XATTR_MOD) {
            FileAction::Chmod
        } else {
            FileAction::Read
        }
    }

    /// Convert raw flags to FileAction (for testing and non-macOS)
    pub fn raw_flags_to_action(flags: u32) -> FileAction {
        // Common FSEvents flag values
        const ITEM_CREATED: u32 = 0x00000100;
        const ITEM_REMOVED: u32 = 0x00000200;
        const ITEM_RENAMED: u32 = 0x00000800;
        const ITEM_MODIFIED: u32 = 0x00001000;
        const ITEM_XATTR_MOD: u32 = 0x00008000;

        if flags & ITEM_REMOVED != 0 {
            FileAction::Delete
        } else if flags & ITEM_CREATED != 0 {
            FileAction::Create
        } else if flags & ITEM_MODIFIED != 0 {
            FileAction::Write
        } else if flags & ITEM_RENAMED != 0 {
            FileAction::Write
        } else if flags & ITEM_XATTR_MOD != 0 {
            FileAction::Chmod
        } else {
            FileAction::Read
        }
    }

    /// Create an event from a file system change (for manual/testing use)
    pub fn create_event(&self, path: PathBuf, action: FileAction) -> Event {
        let risk_level = if self.detector.is_sensitive(&path) {
            RiskLevel::Critical
        } else {
            RiskLevel::Low
        };

        Event::new(
            EventType::FileAccess { path, action },
            "fswatch".to_string(),
            std::process::id(),
            risk_level,
        )
    }
}

impl Drop for FileSystemWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Trait for file monitors (without Send constraint for flexibility)
pub trait FileMonitor {
    /// Start monitoring
    fn start(&mut self) -> Result<()>;
    /// Stop monitoring
    fn stop(&mut self);
    /// Check if running
    fn is_running(&self) -> bool;
}

impl FileMonitor for FileSystemWatcher {
    fn start(&mut self) -> Result<()> {
        FileSystemWatcher::start(self)
    }

    fn stop(&mut self) {
        FileSystemWatcher::stop(self)
    }

    fn is_running(&self) -> bool {
        FileSystemWatcher::is_running(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fswatch_config_default() {
        let config = FsWatchConfig::default();
        assert!(config.watch_paths.is_empty());
        assert_eq!(config.latency, Duration::from_millis(100));
    }

    #[test]
    fn test_fswatch_config_builder() {
        let config = FsWatchConfig::new(vec![PathBuf::from("/tmp")])
            .latency(Duration::from_millis(200))
            .add_path(PathBuf::from("/home"));

        assert_eq!(config.watch_paths.len(), 2);
        assert_eq!(config.latency, Duration::from_millis(200));
    }

    #[test]
    fn test_fs_event_creation() {
        let event = FsEvent::new(PathBuf::from("/tmp/test.txt"), FileAction::Write, 0x1000);

        assert_eq!(event.path, PathBuf::from("/tmp/test.txt"));
        assert_eq!(event.action, FileAction::Write);
    }

    #[test]
    fn test_watcher_creation() {
        let config = FsWatchConfig::new(vec![PathBuf::from("/tmp")]);
        let watcher = FileSystemWatcher::new(config);

        assert!(!watcher.is_running());
    }

    #[test]
    fn test_watcher_subscribe() {
        let config = FsWatchConfig::default();
        let mut watcher = FileSystemWatcher::new(config);

        let _rx = watcher.subscribe();
        assert!(watcher.event_tx.is_some());
    }

    #[test]
    fn test_watcher_with_detector() {
        let config = FsWatchConfig::default();
        let detector = SensitiveFileDetector::new(vec![".custom".to_string()]);

        let watcher = FileSystemWatcher::new(config).with_detector(detector);

        // Verify detector is set (indirectly through create_event)
        let event = watcher.create_event(PathBuf::from(".custom"), FileAction::Read);
        assert_eq!(event.risk_level, RiskLevel::Critical);
    }

    #[test]
    fn test_raw_flags_to_action() {
        assert_eq!(
            FileSystemWatcher::raw_flags_to_action(0x00000100),
            FileAction::Create
        );
        assert_eq!(
            FileSystemWatcher::raw_flags_to_action(0x00000200),
            FileAction::Delete
        );
        assert_eq!(
            FileSystemWatcher::raw_flags_to_action(0x00001000),
            FileAction::Write
        );
        assert_eq!(
            FileSystemWatcher::raw_flags_to_action(0x00008000),
            FileAction::Chmod
        );
        assert_eq!(
            FileSystemWatcher::raw_flags_to_action(0x00000000),
            FileAction::Read
        );
    }

    #[test]
    fn test_create_event_normal_file() {
        let config = FsWatchConfig::default();
        let watcher = FileSystemWatcher::new(config);

        let event = watcher.create_event(PathBuf::from("/tmp/normal.txt"), FileAction::Write);

        assert_eq!(event.risk_level, RiskLevel::Low);
        assert!(!event.alert);
    }

    #[test]
    fn test_create_event_sensitive_file() {
        let config = FsWatchConfig::default();
        let watcher = FileSystemWatcher::new(config);

        let event = watcher.create_event(PathBuf::from(".env"), FileAction::Read);

        assert_eq!(event.risk_level, RiskLevel::Critical);
        assert!(event.alert);
    }

    #[test]
    fn test_file_monitor_trait() {
        let config = FsWatchConfig::default();
        let mut watcher = FileSystemWatcher::new(config);

        // Test trait methods
        assert!(!FileMonitor::is_running(&watcher));

        // Start with empty paths should be ok
        FileMonitor::start(&mut watcher).unwrap();

        FileMonitor::stop(&mut watcher);
    }

    #[test]
    fn test_start_with_empty_paths() {
        let config = FsWatchConfig::default();
        let mut watcher = FileSystemWatcher::new(config);

        // Should succeed but do nothing
        let result = watcher.start();
        assert!(result.is_ok());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_watcher_start_stop() {
        let config = FsWatchConfig::new(vec![PathBuf::from("/tmp")]);
        let mut watcher = FileSystemWatcher::new(config);

        watcher.start().unwrap();
        // Give it a moment to start
        std::thread::sleep(Duration::from_millis(50));
        assert!(watcher.is_running());

        watcher.stop();
        assert!(!watcher.is_running());
    }

    #[test]
    fn test_watcher_drop_stops() {
        let config = FsWatchConfig::new(vec![PathBuf::from("/tmp")]);
        let mut watcher = FileSystemWatcher::new(config);
        let _ = watcher.subscribe();

        // Watcher should be stopped when dropped
        drop(watcher);
        // If this doesn't hang, the drop worked correctly
    }
}
