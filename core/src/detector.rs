//! Detection module for MacAgentWatch
//!
//! Provides pattern-based detection for sensitive files and network connections.

use crate::event::RiskLevel;
use glob::Pattern;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// Pre-computed lowercase sensitive directory patterns for efficient matching
static SENSITIVE_DIRS_LOWER: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    vec!["/.ssh/", "/.aws/", "/.gnupg/", "/.kube/"]
});

/// Trait for detecting sensitive items
pub trait Detector<T>: Clone + Send {
    /// Check if item is sensitive
    fn is_sensitive(&self, item: &T) -> bool;
    /// Get risk level for item
    fn risk_level(&self, item: &T) -> RiskLevel;
    /// Get reason why item is sensitive
    fn reason(&self, item: &T) -> Option<&'static str>;
}

/// Sensitive file detector using glob patterns
#[derive(Debug, Clone)]
pub struct SensitiveFileDetector {
    patterns: Vec<Pattern>,
    pattern_strings: Vec<String>,
    custom_paths: HashSet<String>,
}

impl Default for SensitiveFileDetector {
    fn default() -> Self {
        Self::new(default_sensitive_patterns())
    }
}

impl SensitiveFileDetector {
    /// Create a new detector with given patterns
    pub fn new(patterns: Vec<String>) -> Self {
        let mut compiled_patterns = Vec::new();
        let mut pattern_strings = Vec::new();

        for pattern_str in patterns {
            if let Ok(pattern) = Pattern::new(&pattern_str) {
                compiled_patterns.push(pattern);
                pattern_strings.push(pattern_str);
            }
        }

        Self {
            patterns: compiled_patterns,
            pattern_strings,
            custom_paths: HashSet::new(),
        }
    }

    /// Add a custom sensitive path
    pub fn add_custom_path(&mut self, path: String) {
        self.custom_paths.insert(path);
    }

    /// Add custom sensitive paths
    pub fn add_custom_paths(&mut self, paths: Vec<String>) {
        self.custom_paths.extend(paths);
    }

    /// Get the list of patterns being used
    pub fn patterns(&self) -> &[String] {
        &self.pattern_strings
    }

    /// Check if a path matches any sensitive pattern
    fn matches_pattern(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Check custom paths first (exact match)
        if self.custom_paths.contains(path_str.as_ref()) {
            return true;
        }

        // Check filename against patterns
        if let Some(filename) = path.file_name() {
            let filename_str = filename.to_string_lossy();
            for pattern in &self.patterns {
                if pattern.matches(&filename_str) {
                    return true;
                }
            }
        }

        // Check full path against patterns
        for pattern in &self.patterns {
            if pattern.matches(&path_str) {
                return true;
            }
        }

        // Check for common sensitive directories (using cached lowercase patterns)
        let path_lower = path_str.to_lowercase();
        for dir in SENSITIVE_DIRS_LOWER.iter() {
            if path_lower.contains(dir) {
                return true;
            }
        }

        false
    }
}

impl Detector<PathBuf> for SensitiveFileDetector {
    fn is_sensitive(&self, item: &PathBuf) -> bool {
        // Check original path first
        if self.matches_pattern(item) {
            return true;
        }

        // Try to resolve symlinks and check the resolved path
        // This prevents symlink-based bypasses
        if let Ok(resolved) = item.canonicalize() {
            if resolved != *item && self.matches_pattern(&resolved) {
                return true;
            }
        }

        false
    }

    fn risk_level(&self, item: &PathBuf) -> RiskLevel {
        if self.is_sensitive(item) {
            RiskLevel::Critical
        } else {
            RiskLevel::Low
        }
    }

    fn reason(&self, item: &PathBuf) -> Option<&'static str> {
        if self.is_sensitive(item) {
            Some("Sensitive file access detected")
        } else {
            None
        }
    }
}

/// Network whitelist for allowed hosts
#[derive(Debug, Clone)]
pub struct NetworkWhitelist {
    allowed_hosts: HashSet<String>,
    allowed_ports: HashSet<u16>,
}

impl Default for NetworkWhitelist {
    fn default() -> Self {
        Self::new(default_network_whitelist(), vec![])
    }
}

impl NetworkWhitelist {
    /// Create a new whitelist with given hosts and ports
    pub fn new(hosts: Vec<String>, ports: Vec<u16>) -> Self {
        Self {
            allowed_hosts: hosts.into_iter().collect(),
            allowed_ports: ports.into_iter().collect(),
        }
    }

    /// Add an allowed host
    pub fn add_host(&mut self, host: String) {
        self.allowed_hosts.insert(host);
    }

    /// Add an allowed port
    pub fn add_port(&mut self, port: u16) {
        self.allowed_ports.insert(port);
    }

    /// Check if a host is whitelisted
    pub fn is_host_allowed(&self, host: &str) -> bool {
        // Check exact match
        if self.allowed_hosts.contains(host) {
            return true;
        }

        // Check if it's a subdomain of an allowed host
        for allowed in &self.allowed_hosts {
            if host.ends_with(&format!(".{}", allowed)) {
                return true;
            }
        }

        false
    }

    /// Check if a port is whitelisted
    pub fn is_port_allowed(&self, port: u16) -> bool {
        self.allowed_ports.is_empty() || self.allowed_ports.contains(&port)
    }

    /// Get allowed hosts
    pub fn hosts(&self) -> &HashSet<String> {
        &self.allowed_hosts
    }
}

/// Network connection info for detection
#[derive(Debug, Clone)]
pub struct NetworkConnection {
    pub host: String,
    pub port: u16,
    pub protocol: String,
}

impl Detector<NetworkConnection> for NetworkWhitelist {
    fn is_sensitive(&self, item: &NetworkConnection) -> bool {
        !self.is_host_allowed(&item.host)
    }

    fn risk_level(&self, item: &NetworkConnection) -> RiskLevel {
        if self.is_sensitive(item) {
            RiskLevel::High
        } else {
            RiskLevel::Medium
        }
    }

    fn reason(&self, item: &NetworkConnection) -> Option<&'static str> {
        if self.is_sensitive(item) {
            Some("Unknown network destination")
        } else {
            None
        }
    }
}

/// Default sensitive file patterns
pub fn default_sensitive_patterns() -> Vec<String> {
    vec![
        // Environment files
        ".env".to_string(),
        ".env.*".to_string(),
        "*.env".to_string(),
        // Key files
        "*.pem".to_string(),
        "*.key".to_string(),
        "*.p12".to_string(),
        "*.pfx".to_string(),
        "id_rsa".to_string(),
        "id_ed25519".to_string(),
        "id_ecdsa".to_string(),
        "id_dsa".to_string(),
        // Credential files
        "*credential*".to_string(),
        "*secret*".to_string(),
        "*password*".to_string(),
        "*token*".to_string(),
        // Config files with secrets
        ".netrc".to_string(),
        ".npmrc".to_string(),
        ".pypirc".to_string(),
        "credentials".to_string(),
        "credentials.json".to_string(),
        // AWS
        "aws_access_key*".to_string(),
        // Database
        "*.sqlite".to_string(),
        "*.db".to_string(),
    ]
}

/// Default network whitelist
pub fn default_network_whitelist() -> Vec<String> {
    vec![
        "api.anthropic.com".to_string(),
        "github.com".to_string(),
        "api.github.com".to_string(),
        "raw.githubusercontent.com".to_string(),
        "registry.npmjs.org".to_string(),
        "pypi.org".to_string(),
        "crates.io".to_string(),
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "::1".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn test_sensitive_file_detector_default() {
        let detector = SensitiveFileDetector::default();
        assert!(!detector.patterns().is_empty());
    }

    #[test]
    fn test_detect_env_file() {
        let detector = SensitiveFileDetector::default();

        assert!(detector.matches_pattern(&p(".env")));
        assert!(detector.matches_pattern(&p(".env.local")));
        assert!(detector.matches_pattern(&p(".env.production")));
        assert!(detector.matches_pattern(&p("/home/user/project/.env")));
    }

    #[test]
    fn test_detect_key_files() {
        let detector = SensitiveFileDetector::default();

        assert!(detector.matches_pattern(&p("server.pem")));
        assert!(detector.matches_pattern(&p("private.key")));
        assert!(detector.matches_pattern(&p("/etc/ssl/private/cert.pem")));
        assert!(detector.matches_pattern(&p("id_rsa")));
        assert!(detector.matches_pattern(&p("id_ed25519")));
    }

    #[test]
    fn test_detect_credential_files() {
        let detector = SensitiveFileDetector::default();

        assert!(detector.matches_pattern(&p("credentials.json")));
        assert!(detector.matches_pattern(&p("my_credentials.txt")));
        assert!(detector.matches_pattern(&p("api_secret.yaml")));
        assert!(detector.matches_pattern(&p("password.txt")));
    }

    #[test]
    fn test_detect_ssh_directory() {
        let detector = SensitiveFileDetector::default();

        assert!(detector.matches_pattern(&p("/home/user/.ssh/config")));
        assert!(detector.matches_pattern(&p("/Users/dev/.ssh/known_hosts")));
    }

    #[test]
    fn test_detect_aws_directory() {
        let detector = SensitiveFileDetector::default();

        assert!(detector.matches_pattern(&p("/home/user/.aws/credentials")));
        assert!(detector.matches_pattern(&p("/Users/dev/.aws/config")));
    }

    #[test]
    fn test_non_sensitive_files() {
        let detector = SensitiveFileDetector::default();

        assert!(!detector.matches_pattern(&p("README.md")));
        assert!(!detector.matches_pattern(&p("package.json")));
        assert!(!detector.matches_pattern(&p("/home/user/code/main.rs")));
        assert!(!detector.matches_pattern(&p("Cargo.toml")));
    }

    #[test]
    fn test_risk_level_sensitive() {
        let detector = SensitiveFileDetector::default();

        assert_eq!(detector.risk_level(&p(".env")), RiskLevel::Critical);
        assert_eq!(detector.risk_level(&p("private.key")), RiskLevel::Critical);
    }

    #[test]
    fn test_risk_level_normal() {
        let detector = SensitiveFileDetector::default();

        assert_eq!(detector.risk_level(&p("README.md")), RiskLevel::Low);
    }

    #[test]
    fn test_custom_paths() {
        let mut detector = SensitiveFileDetector::default();
        detector.add_custom_path("/custom/secret/file.txt".to_string());

        assert!(detector.matches_pattern(&p("/custom/secret/file.txt")));
    }

    #[test]
    fn test_reason_for_sensitive() {
        let detector = SensitiveFileDetector::default();

        assert!(detector.reason(&p(".env")).is_some());
        assert!(detector.reason(&p("README.md")).is_none());
    }

    // Network whitelist tests

    #[test]
    fn test_network_whitelist_default() {
        let whitelist = NetworkWhitelist::default();
        assert!(!whitelist.hosts().is_empty());
    }

    #[test]
    fn test_host_allowed() {
        let whitelist = NetworkWhitelist::default();

        assert!(whitelist.is_host_allowed("api.anthropic.com"));
        assert!(whitelist.is_host_allowed("github.com"));
        assert!(whitelist.is_host_allowed("localhost"));
    }

    #[test]
    fn test_host_not_allowed() {
        let whitelist = NetworkWhitelist::default();

        assert!(!whitelist.is_host_allowed("malicious-site.com"));
        assert!(!whitelist.is_host_allowed("unknown-api.io"));
    }

    #[test]
    fn test_subdomain_allowed() {
        let whitelist = NetworkWhitelist::default();

        assert!(whitelist.is_host_allowed("api.github.com"));
        assert!(whitelist.is_host_allowed("sub.api.anthropic.com"));
    }

    #[test]
    fn test_network_connection_detection() {
        let whitelist = NetworkWhitelist::default();

        let allowed_conn = NetworkConnection {
            host: "api.anthropic.com".to_string(),
            port: 443,
            protocol: "tcp".to_string(),
        };

        let blocked_conn = NetworkConnection {
            host: "suspicious-server.xyz".to_string(),
            port: 8080,
            protocol: "tcp".to_string(),
        };

        assert!(!whitelist.is_sensitive(&allowed_conn));
        assert!(whitelist.is_sensitive(&blocked_conn));

        assert_eq!(whitelist.risk_level(&allowed_conn), RiskLevel::Medium);
        assert_eq!(whitelist.risk_level(&blocked_conn), RiskLevel::High);
    }

    #[test]
    fn test_add_custom_host() {
        let mut whitelist = NetworkWhitelist::default();
        whitelist.add_host("custom-api.example.com".to_string());

        assert!(whitelist.is_host_allowed("custom-api.example.com"));
    }

    #[test]
    fn test_detector_trait_for_pathbuf() {
        let detector = SensitiveFileDetector::default();
        let path = PathBuf::from(".env");

        // Test trait methods
        assert!(Detector::<PathBuf>::is_sensitive(&detector, &path));
        assert_eq!(
            Detector::<PathBuf>::risk_level(&detector, &path),
            RiskLevel::Critical
        );
        assert!(Detector::<PathBuf>::reason(&detector, &path).is_some());
    }

    #[test]
    fn test_detector_trait_for_network() {
        let whitelist = NetworkWhitelist::default();
        let conn = NetworkConnection {
            host: "unknown.com".to_string(),
            port: 443,
            protocol: "tcp".to_string(),
        };

        // Test trait methods
        assert!(Detector::<NetworkConnection>::is_sensitive(
            &whitelist, &conn
        ));
        assert_eq!(
            Detector::<NetworkConnection>::risk_level(&whitelist, &conn),
            RiskLevel::High
        );
        assert!(Detector::<NetworkConnection>::reason(&whitelist, &conn).is_some());
    }

    #[test]
    fn test_detector_is_clone() {
        let detector = SensitiveFileDetector::default();
        let cloned = detector.clone();
        assert_eq!(detector.patterns().len(), cloned.patterns().len());
    }

    #[test]
    fn test_symlink_resolution() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let env_file = temp_dir.path().join(".env");
        let link_path = temp_dir.path().join("config_link");

        // Create the actual sensitive file
        fs::write(&env_file, "SECRET=value").unwrap();

        // Create a symlink pointing to the sensitive file
        #[cfg(unix)]
        std::os::unix::fs::symlink(&env_file, &link_path).unwrap();

        let detector = SensitiveFileDetector::default();

        // Original sensitive file should be detected
        assert!(detector.is_sensitive(&env_file));

        // Symlink should also be detected (via canonicalize)
        #[cfg(unix)]
        assert!(detector.is_sensitive(&link_path));
    }

    #[test]
    fn test_broken_symlink_handling() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let link_path = temp_dir.path().join("broken_link");

        // Create a symlink to a non-existent file
        #[cfg(unix)]
        {
            let _ = std::os::unix::fs::symlink("/nonexistent/.env", &link_path);

            let detector = SensitiveFileDetector::default();

            // Broken symlink should not crash, should return false
            // (the original path "broken_link" doesn't match sensitive patterns)
            let result = detector.is_sensitive(&link_path);
            // Just check it doesn't panic - result depends on whether symlink was created
            let _ = result;
        }
    }
}
