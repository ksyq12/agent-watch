//! Process tree tracking module
//!
//! Monitors child processes spawned by the wrapped process using libproc.
//! Polls at configurable intervals to detect new and exited processes.

use crate::event::RiskLevel;
use crate::risk::RiskScorer;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
use libproc::bsd_info::BSDInfo;
#[cfg(target_os = "macos")]
use libproc::proc_pid::{pidinfo, pidpath};
#[cfg(target_os = "macos")]
use libproc::processes::{ProcFilter, pids_by_type};

/// Information about a tracked process
#[derive(Debug, Clone)]
pub struct TrackedProcess {
    /// Process ID
    pub pid: u32,
    /// Parent process ID
    pub ppid: u32,
    /// Process name
    pub name: String,
    /// Full command path
    pub path: Option<String>,
    /// When the process was first detected
    pub detected_at: Instant,
    /// Risk level of the command
    pub risk_level: RiskLevel,
}

/// Event emitted by the process tracker
#[derive(Debug, Clone)]
pub enum TrackerEvent {
    /// New child process started
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

/// Configuration for the process tracker
#[derive(Debug, Clone)]
pub struct TrackerConfig {
    /// Root process ID to track children of
    pub root_pid: u32,
    /// Polling interval
    pub poll_interval: Duration,
    /// Maximum tree depth (None for unlimited)
    pub max_depth: Option<usize>,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            root_pid: 0,
            poll_interval: Duration::from_millis(100),
            max_depth: None,
        }
    }
}

impl TrackerConfig {
    /// Create a new tracker config for a root process
    pub fn new(root_pid: u32) -> Self {
        Self {
            root_pid,
            ..Default::default()
        }
    }

    /// Set the polling interval
    pub fn poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Set the maximum tree depth
    pub fn max_depth(mut self, depth: Option<usize>) -> Self {
        self.max_depth = depth;
        self
    }
}

/// Process tree tracker using libproc
pub struct ProcessTracker {
    config: TrackerConfig,
    risk_scorer: RiskScorer,
    /// Currently tracked processes (pid -> TrackedProcess)
    tracked: Arc<Mutex<HashMap<u32, TrackedProcess>>>,
    /// Event sender
    event_tx: Option<Sender<TrackerEvent>>,
    /// Stop flag
    stop_flag: Arc<AtomicBool>,
    /// Worker thread handle
    thread_handle: Option<JoinHandle<()>>,
}

impl ProcessTracker {
    /// Create a new process tracker
    pub fn new(config: TrackerConfig) -> Self {
        Self {
            config,
            risk_scorer: RiskScorer::new(),
            tracked: Arc::new(Mutex::new(HashMap::new())),
            event_tx: None,
            stop_flag: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
        }
    }

    /// Set a custom risk scorer
    pub fn with_risk_scorer(mut self, scorer: RiskScorer) -> Self {
        self.risk_scorer = scorer;
        self
    }

    /// Subscribe to tracker events
    pub fn subscribe(&mut self) -> std::sync::mpsc::Receiver<TrackerEvent> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.event_tx = Some(tx);
        rx
    }

    /// Start the tracking thread
    pub fn start(&mut self) {
        let config = self.config.clone();
        let tracked = Arc::clone(&self.tracked);
        let stop_flag = Arc::clone(&self.stop_flag);
        let event_tx = self.event_tx.clone();
        let risk_scorer = self.risk_scorer.clone();

        let handle = thread::spawn(move || {
            Self::tracking_loop(config, tracked, stop_flag, event_tx, risk_scorer);
        });

        self.thread_handle = Some(handle);
    }

    /// Stop the tracking thread
    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    /// Signal the tracker to stop without waiting for the thread to finish.
    /// Used by MonitoringOrchestrator for two-phase shutdown.
    pub fn signal_stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    /// Get currently tracked processes
    pub fn get_tracked(&self) -> Vec<TrackedProcess> {
        if let Ok(tracked) = self.tracked.lock() {
            tracked.values().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Check if a PID is being tracked
    pub fn is_tracked(&self, pid: u32) -> bool {
        if let Ok(tracked) = self.tracked.lock() {
            tracked.contains_key(&pid)
        } else {
            false
        }
    }

    /// Main tracking loop
    fn tracking_loop(
        config: TrackerConfig,
        tracked: Arc<Mutex<HashMap<u32, TrackedProcess>>>,
        stop_flag: Arc<AtomicBool>,
        event_tx: Option<Sender<TrackerEvent>>,
        risk_scorer: RiskScorer,
    ) {
        loop {
            // Check stop flag
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }

            // Scan for processes
            Self::scan_processes(&config, &tracked, &event_tx, &risk_scorer);

            // Sleep for poll interval
            thread::sleep(config.poll_interval);
        }
    }

    /// Scan for new and exited processes
    #[cfg(target_os = "macos")]
    fn scan_processes(
        config: &TrackerConfig,
        tracked: &Arc<Mutex<HashMap<u32, TrackedProcess>>>,
        event_tx: &Option<Sender<TrackerEvent>>,
        risk_scorer: &RiskScorer,
    ) {
        // Get all descendant PIDs
        let descendants = Self::get_descendants(config.root_pid, config.max_depth);

        let mut tracked_guard = match tracked.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        // Find new processes
        for pid in &descendants {
            if tracked_guard.contains_key(pid) {
                continue;
            }
            if let Some(process) = Self::get_process_info(*pid, risk_scorer) {
                // Emit event
                if let Some(tx) = event_tx {
                    let _ = tx.send(TrackerEvent::ChildStarted {
                        pid: process.pid,
                        ppid: process.ppid,
                        name: process.name.clone(),
                        path: process.path.clone(),
                        risk_level: process.risk_level,
                    });
                }

                tracked_guard.insert(*pid, process);
            }
        }

        // Find exited processes
        let exited: Vec<u32> = tracked_guard
            .keys()
            .filter(|pid| !descendants.contains(pid))
            .copied()
            .collect();

        for pid in exited {
            tracked_guard.remove(&pid);
            if let Some(tx) = event_tx {
                let _ = tx.send(TrackerEvent::ChildExited { pid });
            }
        }
    }

    /// Non-macOS stub
    #[cfg(not(target_os = "macos"))]
    fn scan_processes(
        _config: &TrackerConfig,
        _tracked: &Arc<Mutex<HashMap<u32, TrackedProcess>>>,
        _event_tx: &Option<Sender<TrackerEvent>>,
        _risk_scorer: &RiskScorer,
    ) {
        // No-op on non-macOS platforms
    }

    /// Get all descendant PIDs of a process
    #[cfg(target_os = "macos")]
    fn get_descendants(root_pid: u32, max_depth: Option<usize>) -> Vec<u32> {
        // Fetch all PIDs once and build a parent->children map
        let all_pids = match pids_by_type(ProcFilter::All) {
            Ok(pids) => pids,
            Err(_) => return Vec::new(),
        };

        // Build HashMap: parent_pid -> Vec<child_pid>
        let mut children_map: HashMap<u32, Vec<u32>> = HashMap::new();
        for pid in all_pids {
            if pid == 0 {
                continue;
            }
            if let Ok(info) = pidinfo::<BSDInfo>(pid as i32, 0) {
                children_map.entry(info.pbi_ppid).or_default().push(pid);
            }
        }

        // BFS using the pre-built map
        let mut descendants = Vec::new();
        let mut to_visit = vec![(root_pid, 0usize)];

        while let Some((pid, depth)) = to_visit.pop() {
            if max_depth.map(|max| depth > max).unwrap_or(false) {
                continue;
            }

            if let Some(children) = children_map.get(&pid) {
                for &child_pid in children {
                    descendants.push(child_pid);
                    to_visit.push((child_pid, depth + 1));
                }
            }
        }

        descendants
    }

    #[cfg(not(target_os = "macos"))]
    fn get_descendants(_root_pid: u32, _max_depth: Option<usize>) -> Vec<u32> {
        Vec::new()
    }

    /// Get process information
    #[cfg(target_os = "macos")]
    fn get_process_info(pid: u32, risk_scorer: &RiskScorer) -> Option<TrackedProcess> {
        let info = pidinfo::<BSDInfo>(pid as i32, 0).ok()?;

        // Convert i8 array to string (pbi_name is [i8; 32])
        let name_bytes: Vec<u8> = info
            .pbi_name
            .iter()
            .take_while(|&&c| c != 0)
            .map(|&c| c as u8)
            .collect();
        let name = String::from_utf8_lossy(&name_bytes).to_string();

        let path = pidpath(pid as i32).ok();

        // Score the command
        let (risk_level, _) = risk_scorer.score(&name, &[]);

        Some(TrackedProcess {
            pid,
            ppid: info.pbi_ppid,
            name,
            path,
            detected_at: Instant::now(),
            risk_level,
        })
    }

    #[cfg(not(target_os = "macos"))]
    fn get_process_info(_pid: u32, _risk_scorer: &RiskScorer) -> Option<TrackedProcess> {
        None
    }
}

impl Drop for ProcessTracker {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_tracker_config_default() {
        let config = TrackerConfig::default();
        assert_eq!(config.root_pid, 0);
        assert_eq!(config.poll_interval, Duration::from_millis(100));
        assert!(config.max_depth.is_none());
    }

    #[test]
    fn test_tracker_config_builder() {
        let config = TrackerConfig::new(1234)
            .poll_interval(Duration::from_millis(50))
            .max_depth(Some(3));

        assert_eq!(config.root_pid, 1234);
        assert_eq!(config.poll_interval, Duration::from_millis(50));
        assert_eq!(config.max_depth, Some(3));
    }

    #[test]
    fn test_tracker_creation() {
        let config = TrackerConfig::new(std::process::id());
        let tracker = ProcessTracker::new(config);

        assert!(tracker.get_tracked().is_empty());
    }

    #[test]
    fn test_tracker_subscribe() {
        let config = TrackerConfig::new(std::process::id());
        let mut tracker = ProcessTracker::new(config);

        let _rx = tracker.subscribe();
        assert!(tracker.event_tx.is_some());
    }

    #[test]
    fn test_tracker_start_stop() {
        let config =
            TrackerConfig::new(std::process::id()).poll_interval(Duration::from_millis(10));
        let mut tracker = ProcessTracker::new(config);

        tracker.start();
        thread::sleep(Duration::from_millis(50));
        tracker.stop();

        // Should not hang
    }

    #[test]
    fn test_tracked_process_clone() {
        let process = TrackedProcess {
            pid: 1234,
            ppid: 1,
            name: "test".to_string(),
            path: Some("/usr/bin/test".to_string()),
            detected_at: Instant::now(),
            risk_level: RiskLevel::Low,
        };

        let cloned = process.clone();
        assert_eq!(cloned.pid, 1234);
        assert_eq!(cloned.name, "test");
    }

    #[test]
    fn test_tracker_event_variants() {
        let start_event = TrackerEvent::ChildStarted {
            pid: 1234,
            ppid: 1,
            name: "bash".to_string(),
            path: Some("/bin/bash".to_string()),
            risk_level: RiskLevel::Low,
        };

        let exit_event = TrackerEvent::ChildExited { pid: 1234 };

        // Just verify they can be created and cloned
        let _cloned = start_event.clone();
        let _cloned = exit_event.clone();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_get_descendants_current_process() {
        // Current process should have no children in test
        let pid = std::process::id();
        let descendants = ProcessTracker::get_descendants(pid, None);
        // May or may not have children depending on test runner
        // descendants may or may not have children depending on test runner
        let _ = descendants.len();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_get_process_info() {
        let scorer = RiskScorer::new();
        let pid = std::process::id();

        let info = ProcessTracker::get_process_info(pid, &scorer);
        assert!(info.is_some());

        let info = info.unwrap();
        assert_eq!(info.pid, pid);
        assert!(!info.name.is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_tracker_with_child_process() {
        use std::process::Command;

        // Start a child process
        let mut child = Command::new("sleep")
            .arg("10")
            .spawn()
            .expect("Failed to spawn sleep");

        let child_pid = child.id();

        // Create tracker for current process
        let config =
            TrackerConfig::new(std::process::id()).poll_interval(Duration::from_millis(10));
        let mut tracker = ProcessTracker::new(config);
        let rx = tracker.subscribe();

        tracker.start();

        // Wait for detection
        thread::sleep(Duration::from_millis(50));

        // Kill the child
        let _ = child.kill();
        let _ = child.wait();

        // Wait for exit detection
        thread::sleep(Duration::from_millis(50));

        tracker.stop();

        // Check events
        let mut found_start = false;
        let mut found_exit = false;

        while let Ok(event) = rx.try_recv() {
            match event {
                TrackerEvent::ChildStarted { pid, .. } if pid == child_pid => {
                    found_start = true;
                }
                TrackerEvent::ChildExited { pid } if pid == child_pid => {
                    found_exit = true;
                }
                _ => {}
            }
        }

        assert!(found_start, "Should have detected child start");
        assert!(found_exit, "Should have detected child exit");
    }

    #[test]
    fn test_is_tracked() {
        let config = TrackerConfig::new(std::process::id());
        let tracker = ProcessTracker::new(config);

        // Initially nothing is tracked
        assert!(!tracker.is_tracked(12345));
    }

    #[test]
    fn test_tracker_with_custom_risk_scorer() {
        let config = TrackerConfig::new(std::process::id());
        let mut scorer = RiskScorer::new();
        scorer.add_custom_high_risk(vec!["test_cmd".to_string()]);

        let tracker = ProcessTracker::new(config).with_risk_scorer(scorer);

        // Just verify it compiles and runs
        assert!(tracker.get_tracked().is_empty());
    }
}
