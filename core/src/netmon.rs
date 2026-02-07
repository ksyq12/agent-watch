//! Network monitoring module for MacAgentWatch
//!
//! Uses libproc to monitor network connections from tracked processes.
//! Detects connections to non-whitelisted hosts.

use crate::detector::{Detector, NetworkConnection, NetworkWhitelist};
use crate::error::CoreError;
use crate::event::{Event, EventType};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

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
    /// Maximum number of seen connections before resetting (0 = unlimited)
    pub max_seen_connections: usize,
}

impl Default for NetMonConfig {
    fn default() -> Self {
        Self {
            root_pid: std::process::id(),
            poll_interval: Duration::from_secs(1),
            track_tcp: true,
            track_udp: true,
            max_seen_connections: 10_000,
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

    /// Set maximum seen connections cache size (0 = unlimited)
    pub fn max_seen_connections(mut self, max: usize) -> Self {
        self.max_seen_connections = max;
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

/// Two-generation cache for seen connections.
/// When the current generation fills up, it rotates: the previous generation
/// is discarded and the current becomes the previous. This avoids the
/// "clear everything → duplicate event flood" problem.
struct SeenConnectionsCache {
    current: HashSet<TrackedConnection>,
    previous: HashSet<TrackedConnection>,
    max_size: usize,
}

impl SeenConnectionsCache {
    fn new(max_size: usize) -> Self {
        Self {
            current: HashSet::new(),
            previous: HashSet::new(),
            max_size,
        }
    }

    fn contains(&self, conn: &TrackedConnection) -> bool {
        self.current.contains(conn) || self.previous.contains(conn)
    }

    fn insert(&mut self, conn: TrackedConnection) {
        self.current.insert(conn);
        if self.max_size > 0 && self.current.len() > self.max_size {
            // Rotate: discard previous, current becomes previous
            self.previous = std::mem::take(&mut self.current);
        }
    }

    fn clear(&mut self) {
        self.current.clear();
        self.previous.clear();
    }
}

/// Network monitor using libproc
pub struct NetworkMonitor {
    config: NetMonConfig,
    whitelist: NetworkWhitelist,
    event_tx: Option<Sender<Event>>,
    stop_flag: Arc<AtomicBool>,
    monitor_thread: Option<JoinHandle<()>>,
    tracked_pids: Arc<Mutex<HashSet<u32>>>,
    seen_connections: Arc<Mutex<SeenConnectionsCache>>,
}

impl NetworkMonitor {
    /// Create a new network monitor
    pub fn new(config: NetMonConfig) -> Self {
        let mut tracked = HashSet::new();
        tracked.insert(config.root_pid);
        let max_seen = config.max_seen_connections;

        Self {
            config,
            whitelist: NetworkWhitelist::default(),
            event_tx: None,
            stop_flag: Arc::new(AtomicBool::new(false)),
            monitor_thread: None,
            tracked_pids: Arc::new(Mutex::new(tracked)),
            seen_connections: Arc::new(Mutex::new(SeenConnectionsCache::new(max_seen))),
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
        if let Ok(mut pids) = self.tracked_pids.lock() {
            pids.insert(pid);
        }
    }

    /// Remove a PID from tracking
    pub fn remove_pid(&self, pid: u32) {
        if let Ok(mut pids) = self.tracked_pids.lock() {
            pids.remove(&pid);
        }
    }

    /// Clear the seen connections cache
    pub fn clear_seen_connections(&self) {
        if let Ok(mut seen) = self.seen_connections.lock() {
            seen.clear();
        }
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.monitor_thread.is_some() && !self.stop_flag.load(Ordering::Relaxed)
    }

    /// Start monitoring
    #[cfg(target_os = "macos")]
    pub fn start(&mut self) -> std::result::Result<(), CoreError> {
        self.stop_flag.store(false, Ordering::Relaxed);

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
    pub fn start(&mut self) -> std::result::Result<(), CoreError> {
        // No-op on non-macOS
        Ok(())
    }

    /// Stop monitoring
    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(handle) = self.monitor_thread.take() {
            let _ = handle.join();
        }
    }

    /// Signal the monitor to stop without waiting for the thread to finish.
    /// Used by MonitoringOrchestrator for two-phase shutdown.
    pub fn signal_stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    /// Main monitoring loop
    #[cfg(target_os = "macos")]
    fn monitor_loop(
        config: NetMonConfig,
        whitelist: NetworkWhitelist,
        event_tx: Option<Sender<Event>>,
        stop_flag: Arc<AtomicBool>,
        tracked_pids: Arc<Mutex<HashSet<u32>>>,
        seen_connections: Arc<Mutex<SeenConnectionsCache>>,
    ) {
        loop {
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }

            let iteration_start = Instant::now();

            // Get current PIDs to check
            let pids: Vec<u32> = tracked_pids
                .lock()
                .map(|g| g.iter().cloned().collect())
                .unwrap_or_default();

            for pid in pids {
                // Get connections for this PID
                let connections = Self::get_connections_for_pid(pid, &config);

                for conn in connections {
                    // Check if we've seen this connection before
                    {
                        let Ok(mut seen) = seen_connections.lock() else {
                            continue;
                        };
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

            // Sleep for the remaining time in the poll interval, accounting for processing time
            let elapsed = iteration_start.elapsed();
            if let Some(remaining) = config.poll_interval.checked_sub(elapsed) {
                thread::sleep(remaining);
            }
        }
    }

    /// Get network connections for a specific PID using libproc
    #[cfg(target_os = "macos")]
    fn get_connections_for_pid(pid: u32, config: &NetMonConfig) -> Vec<TrackedConnection> {
        use libproc::libproc::file_info::{pidfdinfo, ListFDs, ProcFDType};
        use libproc::libproc::net_info::{SocketFDInfo, SocketInfoKind, TcpSIState};
        use libproc::libproc::proc_pid::listpidinfo;

        let mut connections = Vec::new();

        // Get all file descriptors for the process
        let fds = match listpidinfo::<ListFDs>(pid as i32, 256) {
            Ok(fds) => fds,
            Err(e) => {
                // Check errno to distinguish error causes
                let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                match errno {
                    3 /* ESRCH */ => {} // Process exited, normal
                    1 /* EPERM */ => {
                        eprintln!("[agent-watch] Permission denied listing FDs for pid {}: {}", pid, e);
                    }
                    _ => {
                        eprintln!("[agent-watch] Failed to list FDs for pid {}: {} (errno={})", pid, e, errno);
                    }
                }
                return connections;
            }
        };

        // Iterate through socket file descriptors
        for fd_info in fds.iter() {
            if fd_info.proc_fdtype != ProcFDType::Socket as u32 {
                continue;
            }

            // Get detailed socket information
            let socket_info = match pidfdinfo::<SocketFDInfo>(pid as i32, fd_info.proc_fd) {
                Ok(info) => info,
                Err(_) => continue,
            };

            let kind: SocketInfoKind = socket_info.psi.soi_kind.into();

            match kind {
                SocketInfoKind::Tcp if config.track_tcp => {
                    let tcp = libproc_safe::tcp_info(&socket_info.psi);
                    let state: TcpSIState = tcp.tcpsi_state.into();

                    // Only track established connections (not listening sockets)
                    if !matches!(
                        state,
                        TcpSIState::Established | TcpSIState::SynSent | TcpSIState::SynReceived
                    ) {
                        continue;
                    }

                    // Extract remote address and port
                    let remote_port = tcp.tcpsi_ini.insi_fport as u16;
                    if remote_port == 0 {
                        continue;
                    }

                    let Some(remote_addr) =
                        extract_ip_address(&tcp.tcpsi_ini, tcp.tcpsi_ini.insi_vflag)
                    else {
                        continue;
                    };

                    let host = remote_addr.to_string();

                    // Skip loopback addresses
                    if host == "127.0.0.1" || host == "::1" || host == "0.0.0.0" {
                        continue;
                    }

                    connections.push(TrackedConnection::new(
                        pid,
                        host,
                        remote_port,
                        "tcp".to_string(),
                    ));
                }
                SocketInfoKind::In if config.track_udp => {
                    let in_sock = libproc_safe::in_sock_info(&socket_info.psi);

                    // Extract remote address and port for UDP
                    let remote_port = in_sock.insi_fport as u16;
                    if remote_port == 0 {
                        continue;
                    }

                    let Some(remote_addr) = extract_ip_address(&in_sock, in_sock.insi_vflag) else {
                        continue;
                    };

                    let host = remote_addr.to_string();

                    // Skip loopback addresses
                    if host == "127.0.0.1" || host == "::1" || host == "0.0.0.0" {
                        continue;
                    }

                    connections.push(TrackedConnection::new(
                        pid,
                        host,
                        remote_port,
                        "udp".to_string(),
                    ));
                }
                _ => {}
            }
        }

        connections
    }
}

/// Safe wrappers for libproc union field access.
///
/// The libproc crate exposes socket protocol info and address info as C unions.
/// These helpers encapsulate the unsafe access with documented invariants.
#[cfg(target_os = "macos")]
mod libproc_safe {
    use libproc::libproc::net_info::{InSockInfo, SocketInfo};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    /// Extract TCP info from a socket whose `soi_kind` is `Tcp`.
    ///
    /// # Safety invariant
    /// Caller must have verified that `soi_kind` matches `SocketInfoKind::Tcp`.
    pub fn tcp_info(socket: &SocketInfo) -> libproc::libproc::net_info::TcpSockInfo {
        // SAFETY: Caller guarantees soi_kind is Tcp, so pri_tcp is the active variant.
        unsafe { socket.soi_proto.pri_tcp }
    }

    /// Extract UDP/generic-IP info from a socket whose `soi_kind` is `In`.
    ///
    /// # Safety invariant
    /// Caller must have verified that `soi_kind` matches `SocketInfoKind::In`.
    pub fn in_sock_info(socket: &SocketInfo) -> InSockInfo {
        // SAFETY: Caller guarantees soi_kind is In, so pri_in is the active variant.
        unsafe { socket.soi_proto.pri_in }
    }

    /// Extract the remote IPv4 address from an InSockInfo whose `insi_vflag` is 1.
    pub fn extract_ipv4(in_sock: &InSockInfo) -> Option<IpAddr> {
        // SAFETY: vflag == 1 confirms IPv4, so ina_46.i46a_addr4 is the active variant.
        let addr = unsafe { in_sock.insi_faddr.ina_46.i46a_addr4 };
        let ip = Ipv4Addr::from(u32::from_be(addr.s_addr));
        if ip.is_unspecified() {
            None
        } else {
            Some(IpAddr::V4(ip))
        }
    }

    /// Extract the remote IPv6 address from an InSockInfo whose `insi_vflag` is 2.
    pub fn extract_ipv6(in_sock: &InSockInfo) -> Option<IpAddr> {
        // SAFETY: vflag == 2 confirms IPv6, so ina_6 is the active variant.
        let addr = unsafe { in_sock.insi_faddr.ina_6 };
        let ip = Ipv6Addr::from(addr.s6_addr);
        if ip.is_unspecified() {
            None
        } else {
            Some(IpAddr::V6(ip))
        }
    }
}

/// Extract IP address from InSockInfo based on the vflag
#[cfg(target_os = "macos")]
fn extract_ip_address(
    in_sock: &libproc::libproc::net_info::InSockInfo,
    vflag: u8,
) -> Option<std::net::IpAddr> {
    match vflag {
        1 => libproc_safe::extract_ipv4(in_sock),
        2 => libproc_safe::extract_ipv6(in_sock),
        _ => None,
    }
}

impl NetworkMonitor {
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
            let Ok(mut seen) = self.seen_connections.lock() else {
                return;
            };
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

impl crate::types::MonitoringSubsystem for NetworkMonitor {
    fn start(&mut self) -> std::result::Result<(), crate::error::CoreError> {
        NetworkMonitor::start(self)
    }

    fn stop(&mut self) {
        NetworkMonitor::stop(self)
    }

    fn signal_stop(&self) {
        NetworkMonitor::signal_stop(self)
    }

    fn is_running(&self) -> bool {
        NetworkMonitor::is_running(self)
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
    fn start(&mut self) -> std::result::Result<(), CoreError>;
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
    fn start(&mut self) -> std::result::Result<(), CoreError> {
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
        assert_eq!(config.poll_interval, Duration::from_secs(1));
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

        let conn = TrackedConnection::new(
            1234,
            "api.anthropic.com".to_string(),
            443,
            "tcp".to_string(),
        );

        let event = monitor.create_event(&conn);
        assert_eq!(event.risk_level, RiskLevel::Medium);
    }

    #[test]
    fn test_create_event_blocked_host() {
        let config = NetMonConfig::default();
        let monitor = NetworkMonitor::new(config);

        let conn = TrackedConnection::new(
            1234,
            "unknown-server.xyz".to_string(),
            8080,
            "tcp".to_string(),
        );

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

    // --- Integration tests ---

    #[test]
    fn test_seen_connections_cache_deduplication() {
        let mut cache = SeenConnectionsCache::new(100);

        let conn1 = TrackedConnection::new(1, "example.com".to_string(), 443, "tcp".to_string());
        let conn2 = TrackedConnection::new(1, "example.com".to_string(), 443, "tcp".to_string());
        let conn3 = TrackedConnection::new(1, "other.com".to_string(), 80, "tcp".to_string());

        assert!(!cache.contains(&conn1));
        cache.insert(conn1.clone());
        assert!(cache.contains(&conn1));

        // Duplicate connection should be detected
        assert!(cache.contains(&conn2));

        // Different connection should not be detected
        assert!(!cache.contains(&conn3));
        cache.insert(conn3.clone());
        assert!(cache.contains(&conn3));
    }

    #[test]
    fn test_seen_connections_cache_rotation() {
        // Small max_size to trigger rotation
        let mut cache = SeenConnectionsCache::new(2);

        let conn1 = TrackedConnection::new(1, "a.com".to_string(), 80, "tcp".to_string());
        let conn2 = TrackedConnection::new(1, "b.com".to_string(), 80, "tcp".to_string());
        let conn3 = TrackedConnection::new(1, "c.com".to_string(), 80, "tcp".to_string());

        cache.insert(conn1.clone());
        cache.insert(conn2.clone());
        assert!(cache.contains(&conn1));
        assert!(cache.contains(&conn2));

        // Inserting third triggers rotation (current becomes previous, current cleared)
        cache.insert(conn3.clone());

        // After rotation: previous = {conn1, conn2, conn3}, current = {}
        // All three should still be found in previous
        assert!(cache.contains(&conn1));
        assert!(cache.contains(&conn2));
        assert!(cache.contains(&conn3));
    }

    #[test]
    fn test_seen_connections_cache_clear() {
        let mut cache = SeenConnectionsCache::new(100);

        let conn = TrackedConnection::new(1, "example.com".to_string(), 443, "tcp".to_string());
        cache.insert(conn.clone());
        assert!(cache.contains(&conn));

        cache.clear();
        assert!(!cache.contains(&conn));
    }

    #[test]
    fn test_seen_connections_cache_unlimited() {
        // max_size = 0 means unlimited, no rotation
        let mut cache = SeenConnectionsCache::new(0);

        for i in 0..100 {
            let conn = TrackedConnection::new(1, format!("host{}.com", i), 80, "tcp".to_string());
            cache.insert(conn);
        }

        // All 100 should still be in current (no rotation)
        for i in 0..100 {
            let conn = TrackedConnection::new(1, format!("host{}.com", i), 80, "tcp".to_string());
            assert!(cache.contains(&conn));
        }
    }

    #[test]
    fn test_report_connection_with_whitelist_filtering() {
        let config = NetMonConfig::default();
        let whitelist = NetworkWhitelist::new(vec!["allowed.com".to_string()], vec![]);
        let mut monitor = NetworkMonitor::new(config).with_whitelist(whitelist);
        let rx = monitor.subscribe();

        // Allowed host
        let allowed_conn =
            TrackedConnection::new(1, "allowed.com".to_string(), 443, "tcp".to_string());
        monitor.report_connection(allowed_conn);

        let event = rx.try_recv().unwrap();
        assert_eq!(event.risk_level, RiskLevel::Medium);

        // Non-whitelisted host
        let blocked_conn =
            TrackedConnection::new(1, "suspicious.xyz".to_string(), 8080, "tcp".to_string());
        monitor.report_connection(blocked_conn);

        let event = rx.try_recv().unwrap();
        assert_eq!(event.risk_level, RiskLevel::High);
    }

    #[test]
    fn test_report_connection_deduplication() {
        let config = NetMonConfig::default();
        let mut monitor = NetworkMonitor::new(config);
        let rx = monitor.subscribe();

        let conn =
            TrackedConnection::new(1, "test.example.com".to_string(), 443, "tcp".to_string());

        // First report should generate an event
        monitor.report_connection(conn.clone());
        assert!(rx.try_recv().is_ok());

        // Second report of same connection should be deduplicated
        monitor.report_connection(conn.clone());
        assert!(rx.try_recv().is_err());

        // Clear seen connections and report again - should generate event
        monitor.clear_seen_connections();
        monitor.report_connection(conn);
        assert!(rx.try_recv().is_ok());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_monitor_signal_stop() {
        let config = NetMonConfig::new(std::process::id()).poll_interval(Duration::from_millis(50));
        let mut monitor = NetworkMonitor::new(config);

        monitor.start().unwrap();
        std::thread::sleep(Duration::from_millis(50));
        assert!(monitor.is_running());

        // signal_stop sets the flag without joining the thread
        monitor.signal_stop();
        assert!(!monitor.is_running());

        // Full stop should work without hanging
        monitor.stop();
        assert!(!monitor.is_running());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_monitor_lifecycle_with_pid_management() {
        let config =
            NetMonConfig::new(std::process::id()).poll_interval(Duration::from_millis(100));
        let mut monitor = NetworkMonitor::new(config);
        let _rx = monitor.subscribe();

        // Add extra PIDs before starting
        monitor.add_pid(99999);
        assert!(monitor.tracked_pids.lock().unwrap().contains(&99999));

        monitor.start().unwrap();
        std::thread::sleep(Duration::from_millis(50));
        assert!(monitor.is_running());

        // Can add/remove PIDs while running
        monitor.add_pid(99998);
        assert!(monitor.tracked_pids.lock().unwrap().contains(&99998));

        monitor.remove_pid(99999);
        assert!(!monitor.tracked_pids.lock().unwrap().contains(&99999));

        monitor.stop();
        assert!(!monitor.is_running());
    }

    #[test]
    fn test_max_seen_connections_config() {
        let config = NetMonConfig::new(1).max_seen_connections(5);
        let monitor = NetworkMonitor::new(config);

        // Verify the cache was created with the configured max size
        let seen = monitor.seen_connections.lock().unwrap();
        assert_eq!(seen.max_size, 5);
    }
}
