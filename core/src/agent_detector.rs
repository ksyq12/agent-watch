//! Agent auto-detection module
//!
//! Scans running processes to detect known AI coding agents.
//! Uses libproc to enumerate system processes and matches against
//! configurable name/path patterns.

#[cfg(target_os = "macos")]
use libproc::bsd_info::BSDInfo;
#[cfg(target_os = "macos")]
use libproc::proc_pid::{pidinfo, pidpath};
#[cfg(target_os = "macos")]
use libproc::processes::{pids_by_type, ProcFilter};

/// A detected AI agent process
#[derive(Debug, Clone)]
pub struct DetectedAgent {
    /// Process ID
    pub pid: u32,
    /// Process name
    pub name: String,
    /// Executable path
    pub path: String,
}

/// Scans running processes for known AI agent patterns
pub struct AgentDetector {
    patterns: Vec<String>,
}

/// Returns the default set of AI agent name patterns
pub fn default_patterns() -> Vec<String> {
    vec![
        "claude".to_string(),
        "cursor".to_string(),
        "copilot".to_string(),
        "aider".to_string(),
        "windsurf".to_string(),
        "cody".to_string(),
    ]
}

impl AgentDetector {
    /// Create a new detector with default patterns
    pub fn new() -> Self {
        Self {
            patterns: default_patterns(),
        }
    }

    /// Create a detector with custom patterns
    pub fn with_patterns(patterns: Vec<String>) -> Self {
        Self { patterns }
    }

    /// Get the current patterns
    pub fn patterns(&self) -> &[String] {
        &self.patterns
    }

    /// Scan all running processes for agents matching the configured patterns
    #[cfg(target_os = "macos")]
    pub fn scan_for_agents(&self) -> Vec<DetectedAgent> {
        let all_pids = match pids_by_type(ProcFilter::All) {
            Ok(pids) => pids,
            Err(_) => return Vec::new(),
        };

        let mut detected = Vec::new();

        for pid in all_pids {
            if pid == 0 {
                continue;
            }

            let name = match pidinfo::<BSDInfo>(pid as i32, 0) {
                Ok(info) => {
                    let name_bytes: Vec<u8> = info
                        .pbi_name
                        .iter()
                        .take_while(|&&c| c != 0)
                        .map(|&c| c as u8)
                        .collect();
                    String::from_utf8_lossy(&name_bytes).to_string()
                }
                Err(_) => continue,
            };

            let path = pidpath(pid as i32).unwrap_or_default();

            let name_lower = name.to_lowercase();
            let path_lower = path.to_lowercase();

            let matched = self.patterns.iter().any(|pattern| {
                let pat_lower = pattern.to_lowercase();
                name_lower.contains(&pat_lower) || path_lower.contains(&pat_lower)
            });

            if matched {
                detected.push(DetectedAgent { pid, name, path });
            }
        }

        detected
    }

    /// Non-macOS stub
    #[cfg(not(target_os = "macos"))]
    pub fn scan_for_agents(&self) -> Vec<DetectedAgent> {
        Vec::new()
    }
}

impl Default for AgentDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_patterns_not_empty() {
        let patterns = default_patterns();
        assert!(!patterns.is_empty());
        assert!(patterns.contains(&"claude".to_string()));
        assert!(patterns.contains(&"cursor".to_string()));
        assert!(patterns.contains(&"copilot".to_string()));
        assert!(patterns.contains(&"aider".to_string()));
        assert!(patterns.contains(&"windsurf".to_string()));
        assert!(patterns.contains(&"cody".to_string()));
    }

    #[test]
    fn test_agent_detector_new() {
        let detector = AgentDetector::new();
        assert!(!detector.patterns().is_empty());
        assert_eq!(detector.patterns().len(), default_patterns().len());
    }

    #[test]
    fn test_with_custom_patterns() {
        let custom = vec!["my-agent".to_string(), "other-agent".to_string()];
        let detector = AgentDetector::with_patterns(custom.clone());
        assert_eq!(detector.patterns().len(), 2);
        assert_eq!(detector.patterns(), &custom);
    }

    #[test]
    fn test_scan_returns_vec() {
        let detector = AgentDetector::new();
        let agents = detector.scan_for_agents();
        // Result is a Vec (may be empty if no agents are running)
        let _ = agents.len();
    }

    #[test]
    fn test_detected_agent_fields() {
        let agent = DetectedAgent {
            pid: 12345,
            name: "claude".to_string(),
            path: "/usr/local/bin/claude".to_string(),
        };
        assert_eq!(agent.pid, 12345);
        assert_eq!(agent.name, "claude");
        assert_eq!(agent.path, "/usr/local/bin/claude");

        // Test clone
        let cloned = agent.clone();
        assert_eq!(cloned.pid, agent.pid);
        assert_eq!(cloned.name, agent.name);
        assert_eq!(cloned.path, agent.path);
    }
}
