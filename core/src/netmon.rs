//! Network monitoring module for MacAgentWatch
//!
//! Uses libproc to monitor network connections from tracked processes.
//! Detects connections to non-whitelisted hosts.

use crate::detector::{Detector, NetworkConnection, NetworkWhitelist};
use crate::event::{Event, EventType};
use anyhow::Result;
use std::collections::HashSet;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Network monitor configuration
#[derive(Debug, Clone)]
pub struct NetMonConfig {
    /// Root process ID to monitor (and its children)
    pub root_pid: u32,
    /// Polling interval
    pub poll_interval: Duration,
    /// Track TCP connections
    pub track_tcp: bool,
    /// Track UDP connections
    pub track_udp: bool,
}

impl Default for NetMonConfig {
    fn default() -> Self {
        Self {
            root_pid: std::process::id(),
            poll_interval: Duration::from_millis(500),
            track_tcp: true,
            track_udp: true,
        }
    }
}

impl NetMonConfig {
    /// Create a new config for a specific process
    pub fn new(root_pid: u32) -> Self {
        Self {
            root_pid,
            ..Default::default()
        }
    }

    /// Set poll interval
    pub fn poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Enable/disable TCP tracking
    pub fn track_tcp(mut self, enabled: bool) -> Self {
        self.track_tcp = enabled;
        self
    }

    /// Enable/disable UDP tracking
    pub fn track_udp(mut self, enabled: bool) -> Self {
        self.track_udp = enabled;
        self
    }
}

/// Tracked network connection with metadata
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TrackedConnection {
    /// Process ID
    pub pid: u32,
    /// Remote host
    pub host: String,
    /// Remote port
    pub port: u16,
    /// Protocol
    pub protocol: String,
}

impl TrackedConnection {
    /// Create a new tracked connection
    pub fn new(pid: u32, host: String, port: u16, protocol: String) -> Self {
        Self {
            pid,
            host,
            port,
            protocol,
        }
    }

    fn to_network_connection(&self) -> NetworkConnection {
        NetworkConnection {
            host: self.host.clone(),
            port: self.port,
            protocol: self.protocol.clone(),
        }
    }
}

/// Network monitor using libproc
pub struct NetworkMonitor {
    config: NetMonConfig,
    whitelist: NetworkWhitelist,
    event_tx: Option<Sender<Event>>,
    stop_flag: Arc<Mutex<bool>>,
    monitor_thread: Option<JoinHandle<()>>,
    tracked_pids: Arc<Mutex<HashSet<u32>>>,
    seen_connections: Arc<Mutex<HashSet<TrackedConnection>>>,
}

impl NetworkMonitor {
    /// Create a new network monitor
    pub fn new(config: NetMonConfig) -> Self {
        let mut tracked = HashSet::new();
        tracked.insert(config.root_pid);

        Self {
            config,
            whitelist: NetworkWhitelist::default(),
            event_tx: None,
            stop_flag: Arc::new(Mutex::new(false)),
            monitor_thread: None,
            tracked_pids: Arc::new(Mutex::new(tracked)),
            seen_connections: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Create with custom whitelist
    pub fn with_whitelist(mut self, whitelist: NetworkWhitelist) -> Self {
        self.whitelist = whitelist;
        self
    }

    /// Subscribe to network events
    pub fn subscribe(&mut self) -> Receiver<Event> {
        let (tx, rx) = channel();
        self.event_tx = Some(tx);
        rx
    }

    /// Add a PID to track
    pub fn add_pid(&self, pid: u32) {
        self.tracked_pids.lock().unwrap().insert(pid);
    }

    /// Remove a PID from tracking
    pub fn remove_pid(&self, pid: u32) {
        self.tracked_pids.lock().unwrap().remove(&pid);
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.monitor_thread.is_some() && !*self.stop_flag.lock().unwrap()
    }

    /// Start monitoring
    #[cfg(target_os = "macos")]
    pub fn start(&mut self) -> Result<()> {
        *self.stop_flag.lock().unwrap() = false;

        let config = self.config.clone();
        let whitelist = self.whitelist.clone();
        let event_tx = self.event_tx.clone();
        let stop_flag = self.stop_flag.clone();
        let tracked_pids = self.tracked_pids.clone();
        let seen_connections = self.seen_connections.clone();

        let handle = thread::spawn(move || {
            Self::monitor_loop(
                config,
                whitelist,
                event_tx,
                stop_flag,
                tracked_pids,
                seen_connections,
            );
        });

        self.monitor_thread = Some(handle);
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn start(&mut self) -> Result<()> {
        // No-op on non-macOS
        Ok(())
    }

    /// Stop monitoring
    pub fn stop(&mut self) {
        *self.stop_flag.lock().unwrap() = true;
        if let Some(handle) = self.monitor_thread.take() {
            let _ = handle.join();
        }
    }

    /// Main monitoring loop
    #[cfg(target_os = "macos")]
    fn monitor_loop(
        config: NetMonConfig,
        whitelist: NetworkWhitelist,
        event_tx: Option<Sender<Event>>,
        stop_flag: Arc<Mutex<bool>>,
        tracked_pids: Arc<Mutex<HashSet<u32>>>,
        seen_connections: Arc<Mutex<HashSet<TrackedConnection>>>,
    ) {
        loop {
            if *stop_flag.lock().unwrap() {
                break;
            }

            // Get current PIDs to check
            let pids: Vec<u32> = tracked_pids.lock().unwrap().iter().cloned().collect();

            for pid in pids {
                // Get connections for this PID
                let connections = Self::get_connections_for_pid(pid, &config);

                for conn in connections {
                    // Check if we've seen this connection before
                    {
                        let mut seen = seen_connections.lock().unwrap();
                        if seen.contains(&conn) {
                            continue;
                        }
                        seen.insert(conn.clone());
                    }

                    // Determine risk level
                    let net_conn = conn.to_network_connection();
                    let risk_level = whitelist.risk_level(&net_conn);

                    // Create event
                    let event = Event::new(
                        EventType::Network {
                            host: conn.host.clone(),
                            port: conn.port,
                            protocol: conn.protocol.clone(),
                        },
                        format!("pid:{}", pid),
                        pid,
                        risk_level,
                    );

                    if let Some(ref tx) = event_tx {
                        let _ = tx.send(event);
                    }
                }
            }

            thread::sleep(config.poll_interval);
        }
    }

    /// Get network connections for a specific PID using libproc
    /// Note: This is a simplified implementation that returns empty for now
    /// Full implementation requires careful handling of libproc socket APIs
    #[cfg(target_os = "macos")]
    fn get_connections_for_pid(pid: u32, _config: &NetMonConfig) -> Vec<TrackedConnection> {
        use libproc::libproc::file_info::ListFDs;
        use libproc::libproc::file_info::ProcFDType;
        use libproc::libproc::proc_pid::listpidinfo;

        let connections = Vec::new();

        // Get file descriptors for the process
        let fds = match listpidinfo::<ListFDs>(pid as i32, 256) {
            Ok(fds) => fds,
            Err(_) => return connections,
        };

        // Count socket file descriptors (actual connection extraction is complex)
        let socket_count = fds
            .iter()
            .filter(|fd| fd.proc_fdtype == ProcFDType::Socket as u32)
            .count();

        // For now, we just log that sockets exist
        // Full implementation would extract remote addresses using pidfdinfo
        if socket_count > 0 {
            // Placeholder: in production, we would extract actual socket info
            // using libproc's pidfdinfo with SocketFDInfo
        }

        connections
    }

    /// Create network event (for testing)
    pub fn create_event(&self, conn: &TrackedConnection) -> Event {
        let net_conn = conn.to_network_connection();
        let risk_level = self.whitelist.risk_level(&net_conn);

        Event::new(
            EventType::Network {
                host: conn.host.clone(),
                port: conn.port,
                protocol: conn.protocol.clone(),
            },
            format!("pid:{}", conn.pid),
            conn.pid,
            risk_level,
        )
    }

    /// Manually report a connection (for integration with external tools)
    pub fn report_connection(&self, conn: TrackedConnection) {
        // Check if we've seen this connection before
        {
            let mut seen = self.seen_connections.lock().unwrap();
            if seen.contains(&conn) {
                return;
            }
            seen.insert(conn.clone());
        }

        // Determine risk level and send event
        let net_conn = conn.to_network_connection();
        let risk_level = self.whitelist.risk_level(&net_conn);

        let event = Event::new(
            EventType::Network {
                host: conn.host.clone(),
                port: conn.port,
                protocol: conn.protocol.clone(),
            },
            format!("pid:{}", conn.pid),
            conn.pid,
            risk_level,
        );

        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(event);
        }
    }
}

impl Drop for NetworkMonitor {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Trait for network trackers
pub trait NetworkTracker {
    /// Start tracking
    fn start(&mut self) -> Result<()>;
    /// Stop tracking
    fn stop(&mut self);
    /// Add a PID to track
    fn add_pid(&mut self, pid: u32);
    /// Remove a PID
    fn remove_pid(&mut self, pid: u32);
    /// Check if running
    fn is_running(&self) -> bool;
}

impl NetworkTracker for NetworkMonitor {
    fn start(&mut self) -> Result<()> {
        NetworkMonitor::start(self)
    }

    fn stop(&mut self) {
        NetworkMonitor::stop(self)
    }

    fn add_pid(&mut self, pid: u32) {
        NetworkMonitor::add_pid(self, pid)
    }

    fn remove_pid(&mut self, pid: u32) {
        NetworkMonitor::remove_pid(self, pid)
    }

    fn is_running(&self) -> bool {
        NetworkMonitor::is_running(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::RiskLevel;

    #[test]
    fn test_netmon_config_default() {
        let config = NetMonConfig::default();
        assert_eq!(config.poll_interval, Duration::from_millis(500));
        assert!(config.track_tcp);
        assert!(config.track_udp);
    }

    #[test]
    fn test_netmon_config_builder() {
        let config = NetMonConfig::new(1234)
            .poll_interval(Duration::from_millis(100))
            .track_tcp(true)
            .track_udp(false);

        assert_eq!(config.root_pid, 1234);
        assert_eq!(config.poll_interval, Duration::from_millis(100));
        assert!(config.track_tcp);
        assert!(!config.track_udp);
    }

    #[test]
    fn test_tracked_connection() {
        let conn = TrackedConnection::new(1234, "example.com".to_string(), 443, "tcp".to_string());

        let net_conn = conn.to_network_connection();
        assert_eq!(net_conn.host, "example.com");
        assert_eq!(net_conn.port, 443);
    }

    #[test]
    fn test_monitor_creation() {
        let config = NetMonConfig::new(1234);
        let monitor = NetworkMonitor::new(config);

        assert!(!monitor.is_running());
        assert!(monitor.tracked_pids.lock().unwrap().contains(&1234));
    }

    #[test]
    fn test_monitor_subscribe() {
        let config = NetMonConfig::default();
        let mut monitor = NetworkMonitor::new(config);

        let _rx = monitor.subscribe();
        assert!(monitor.event_tx.is_some());
    }

    #[test]
    fn test_add_remove_pid() {
        let config = NetMonConfig::new(1);
        let monitor = NetworkMonitor::new(config);

        monitor.add_pid(100);
        assert!(monitor.tracked_pids.lock().unwrap().contains(&100));

        monitor.remove_pid(100);
        assert!(!monitor.tracked_pids.lock().unwrap().contains(&100));
    }

    #[test]
    fn test_with_whitelist() {
        let config = NetMonConfig::default();
        let whitelist = NetworkWhitelist::new(vec!["custom.com".to_string()], vec![]);

        let monitor = NetworkMonitor::new(config).with_whitelist(whitelist);

        assert!(monitor.whitelist.is_host_allowed("custom.com"));
    }

    #[test]
    fn test_create_event_allowed_host() {
        let config = NetMonConfig::default();
        let monitor = NetworkMonitor::new(config);

        let conn =
            TrackedConnection::new(1234, "api.anthropic.com".to_string(), 443, "tcp".to_string());

        let event = monitor.create_event(&conn);
        assert_eq!(event.risk_level, RiskLevel::Medium);
    }

    #[test]
    fn test_create_event_blocked_host() {
        let config = NetMonConfig::default();
        let monitor = NetworkMonitor::new(config);

        let conn =
            TrackedConnection::new(1234, "unknown-server.xyz".to_string(), 8080, "tcp".to_string());

        let event = monitor.create_event(&conn);
        assert_eq!(event.risk_level, RiskLevel::High);
    }

    #[test]
    fn test_network_tracker_trait() {
        let config = NetMonConfig::default();
        let mut monitor = NetworkMonitor::new(config);

        assert!(!NetworkTracker::is_running(&monitor));

        NetworkTracker::add_pid(&mut monitor, 999);
        assert!(monitor.tracked_pids.lock().unwrap().contains(&999));

        NetworkTracker::remove_pid(&mut monitor, 999);
        assert!(!monitor.tracked_pids.lock().unwrap().contains(&999));
    }

    #[test]
    fn test_report_connection() {
        let config = NetMonConfig::default();
        let mut monitor = NetworkMonitor::new(config);
        let rx = monitor.subscribe();

        let conn =
            TrackedConnection::new(1234, "test.example.com".to_string(), 80, "tcp".to_string());

        monitor.report_connection(conn.clone());

        // Should receive the event
        let event = rx.try_recv().unwrap();
        assert_eq!(
            match event.event_type {
                EventType::Network { host, .. } => host,
                _ => panic!("Wrong event type"),
            },
            "test.example.com"
        );

        // Report same connection again - should be ignored
        monitor.report_connection(conn);
        assert!(rx.try_recv().is_err());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_monitor_start_stop() {
        let config = NetMonConfig::new(std::process::id());
        let mut monitor = NetworkMonitor::new(config);

        monitor.start().unwrap();
        std::thread::sleep(Duration::from_millis(50));
        assert!(monitor.is_running());

        monitor.stop();
        assert!(!monitor.is_running());
    }

    #[test]
    fn test_monitor_drop_stops() {
        let config = NetMonConfig::default();
        let mut monitor = NetworkMonitor::new(config);
        let _ = monitor.subscribe();

        drop(monitor);
        // If this doesn't hang, drop worked correctly
    }
}
