//! Risk scoring module for command analysis
//!
//! Analyzes commands and assigns risk levels based on their potential impact.

use crate::event::RiskLevel;
use std::collections::HashSet;

/// Rule for matching commands to risk levels
#[derive(Debug, Clone)]
pub struct RiskRule {
    /// Pattern to match (command name or full pattern)
    pub pattern: RiskPattern,
    /// Risk level to assign
    pub level: RiskLevel,
    /// Description of why this is risky
    pub reason: &'static str,
}

/// Pattern type for matching commands
#[derive(Debug, Clone)]
pub enum RiskPattern {
    /// Exact command name match
    Command(&'static str),
    /// Command with specific arguments
    CommandWithArgs(&'static str, Vec<&'static str>),
    /// Command contains pattern
    Contains(&'static str),
    /// Pipe pattern (command | command)
    PipePattern(&'static str, &'static str),
}

/// Risk scorer that analyzes commands
#[derive(Clone)]
pub struct RiskScorer {
    rules: Vec<RiskRule>,
    custom_high_risk: HashSet<String>,
}

impl Default for RiskScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl RiskScorer {
    /// Create a new risk scorer with default rules
    pub fn new() -> Self {
        Self {
            rules: Self::default_rules(),
            custom_high_risk: HashSet::new(),
        }
    }

    /// Add custom high-risk commands
    pub fn add_custom_high_risk(&mut self, commands: Vec<String>) {
        self.custom_high_risk.extend(commands);
    }

    /// Score a command and return its risk level
    pub fn score(&self, command: &str, args: &[String]) -> (RiskLevel, Option<&'static str>) {
        let full_command = if args.is_empty() {
            command.to_string()
        } else {
            format!("{} {}", command, args.join(" "))
        };

        // Check custom high-risk first
        for custom in &self.custom_high_risk {
            if full_command.starts_with(custom) {
                return (RiskLevel::High, Some("Custom high-risk command"));
            }
        }

        // Check built-in rules (highest risk first)
        let mut critical_rules: Vec<&RiskRule> = Vec::new();
        let mut high_rules: Vec<&RiskRule> = Vec::new();
        let mut medium_rules: Vec<&RiskRule> = Vec::new();

        for rule in &self.rules {
            match rule.level {
                RiskLevel::Critical => critical_rules.push(rule),
                RiskLevel::High => high_rules.push(rule),
                RiskLevel::Medium => medium_rules.push(rule),
                RiskLevel::Low => {}
            }
        }

        // Check critical first
        for rule in critical_rules {
            if self.matches_rule(rule, command, args, &full_command) {
                return (RiskLevel::Critical, Some(rule.reason));
            }
        }

        // Check high
        for rule in high_rules {
            if self.matches_rule(rule, command, args, &full_command) {
                return (RiskLevel::High, Some(rule.reason));
            }
        }

        // Check medium
        for rule in medium_rules {
            if self.matches_rule(rule, command, args, &full_command) {
                return (RiskLevel::Medium, Some(rule.reason));
            }
        }

        (RiskLevel::Low, None)
    }

    fn matches_rule(
        &self,
        rule: &RiskRule,
        command: &str,
        args: &[String],
        full_command: &str,
    ) -> bool {
        match &rule.pattern {
            RiskPattern::Command(cmd) => command == *cmd,
            RiskPattern::CommandWithArgs(cmd, required_args) => {
                command == *cmd
                    && required_args.iter().all(|required| {
                        args.iter()
                            .any(|a| a == *required || a.starts_with(&format!("{}=", required)))
                    })
            }
            RiskPattern::Contains(pattern) => full_command.contains(pattern),
            RiskPattern::PipePattern(first, second) => {
                full_command.contains(first)
                    && full_command.contains("|")
                    && full_command.contains(second)
            }
        }
    }

    fn default_rules() -> Vec<RiskRule> {
        vec![
            // Critical: Extremely dangerous
            RiskRule {
                pattern: RiskPattern::CommandWithArgs("rm", vec!["-rf", "/"]),
                level: RiskLevel::Critical,
                reason: "Recursive force delete of root directory",
            },
            RiskRule {
                pattern: RiskPattern::CommandWithArgs("rm", vec!["-rf", "/*"]),
                level: RiskLevel::Critical,
                reason: "Recursive force delete of root contents",
            },
            RiskRule {
                pattern: RiskPattern::CommandWithArgs("chmod", vec!["777"]),
                level: RiskLevel::Critical,
                reason: "Setting world-writable permissions",
            },
            RiskRule {
                pattern: RiskPattern::CommandWithArgs("chmod", vec!["-R", "777"]),
                level: RiskLevel::Critical,
                reason: "Recursively setting world-writable permissions",
            },
            RiskRule {
                pattern: RiskPattern::PipePattern("curl", "bash"),
                level: RiskLevel::Critical,
                reason: "Piping remote script to shell (curl | bash)",
            },
            RiskRule {
                pattern: RiskPattern::PipePattern("wget", "bash"),
                level: RiskLevel::Critical,
                reason: "Piping remote script to shell (wget | bash)",
            },
            RiskRule {
                pattern: RiskPattern::PipePattern("curl", "sh"),
                level: RiskLevel::Critical,
                reason: "Piping remote script to shell (curl | sh)",
            },
            RiskRule {
                pattern: RiskPattern::Contains(":(){:|:&};:"),
                level: RiskLevel::Critical,
                reason: "Fork bomb detected",
            },
            // High: Destructive or privilege escalation
            RiskRule {
                pattern: RiskPattern::CommandWithArgs("rm", vec!["-rf"]),
                level: RiskLevel::High,
                reason: "Recursive force delete",
            },
            RiskRule {
                pattern: RiskPattern::CommandWithArgs("rm", vec!["-r"]),
                level: RiskLevel::High,
                reason: "Recursive delete",
            },
            RiskRule {
                pattern: RiskPattern::Command("sudo"),
                level: RiskLevel::High,
                reason: "Privilege escalation",
            },
            RiskRule {
                pattern: RiskPattern::Command("su"),
                level: RiskLevel::High,
                reason: "User switch",
            },
            RiskRule {
                pattern: RiskPattern::Command("ssh"),
                level: RiskLevel::High,
                reason: "Remote shell access",
            },
            RiskRule {
                pattern: RiskPattern::Command("scp"),
                level: RiskLevel::High,
                reason: "Remote file copy",
            },
            RiskRule {
                pattern: RiskPattern::Command("rsync"),
                level: RiskLevel::High,
                reason: "Remote sync",
            },
            RiskRule {
                pattern: RiskPattern::CommandWithArgs("chmod", vec!["+x"]),
                level: RiskLevel::High,
                reason: "Adding execute permission",
            },
            RiskRule {
                pattern: RiskPattern::Command("chown"),
                level: RiskLevel::High,
                reason: "Changing file ownership",
            },
            RiskRule {
                pattern: RiskPattern::Command("mkfs"),
                level: RiskLevel::High,
                reason: "Formatting filesystem",
            },
            RiskRule {
                pattern: RiskPattern::Command("dd"),
                level: RiskLevel::High,
                reason: "Low-level disk operation",
            },
            // Medium: Network operations, package management
            RiskRule {
                pattern: RiskPattern::Command("curl"),
                level: RiskLevel::Medium,
                reason: "Network request",
            },
            RiskRule {
                pattern: RiskPattern::Command("wget"),
                level: RiskLevel::Medium,
                reason: "Network download",
            },
            RiskRule {
                pattern: RiskPattern::CommandWithArgs("pip", vec!["install"]),
                level: RiskLevel::Medium,
                reason: "Python package installation",
            },
            RiskRule {
                pattern: RiskPattern::CommandWithArgs("pip3", vec!["install"]),
                level: RiskLevel::Medium,
                reason: "Python package installation",
            },
            RiskRule {
                pattern: RiskPattern::CommandWithArgs("npm", vec!["install"]),
                level: RiskLevel::Medium,
                reason: "NPM package installation",
            },
            RiskRule {
                pattern: RiskPattern::CommandWithArgs("yarn", vec!["add"]),
                level: RiskLevel::Medium,
                reason: "Yarn package installation",
            },
            RiskRule {
                pattern: RiskPattern::CommandWithArgs("brew", vec!["install"]),
                level: RiskLevel::Medium,
                reason: "Homebrew package installation",
            },
            RiskRule {
                pattern: RiskPattern::CommandWithArgs("cargo", vec!["install"]),
                level: RiskLevel::Medium,
                reason: "Cargo package installation",
            },
            RiskRule {
                pattern: RiskPattern::Command("git"),
                level: RiskLevel::Medium,
                reason: "Git operation",
            },
            RiskRule {
                pattern: RiskPattern::Command("docker"),
                level: RiskLevel::Medium,
                reason: "Docker operation",
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_low_risk_commands() {
        let scorer = RiskScorer::new();

        let (level, _) = scorer.score("ls", &["-la".to_string()]);
        assert_eq!(level, RiskLevel::Low);

        let (level, _) = scorer.score("cat", &["file.txt".to_string()]);
        assert_eq!(level, RiskLevel::Low);

        let (level, _) = scorer.score("echo", &["hello".to_string()]);
        assert_eq!(level, RiskLevel::Low);

        let (level, _) = scorer.score("cd", &["/home".to_string()]);
        assert_eq!(level, RiskLevel::Low);
    }

    #[test]
    fn test_medium_risk_commands() {
        let scorer = RiskScorer::new();

        let (level, reason) = scorer.score("curl", &["https://example.com".to_string()]);
        assert_eq!(level, RiskLevel::Medium);
        assert!(reason.is_some());

        let (level, _) = scorer.score("wget", &["https://example.com/file".to_string()]);
        assert_eq!(level, RiskLevel::Medium);

        let (level, _) = scorer.score("pip", &["install".to_string(), "requests".to_string()]);
        assert_eq!(level, RiskLevel::Medium);

        let (level, _) = scorer.score("npm", &["install".to_string(), "lodash".to_string()]);
        assert_eq!(level, RiskLevel::Medium);

        let (level, _) = scorer.score("git", &["clone".to_string(), "repo".to_string()]);
        assert_eq!(level, RiskLevel::Medium);
    }

    #[test]
    fn test_high_risk_commands() {
        let scorer = RiskScorer::new();

        let (level, reason) = scorer.score("rm", &["-rf".to_string(), "directory".to_string()]);
        assert_eq!(level, RiskLevel::High);
        assert_eq!(reason, Some("Recursive force delete"));

        let (level, _) = scorer.score("sudo", &["apt".to_string(), "update".to_string()]);
        assert_eq!(level, RiskLevel::High);

        let (level, _) = scorer.score("ssh", &["user@host".to_string()]);
        assert_eq!(level, RiskLevel::High);

        let (level, _) = scorer.score("chmod", &["+x".to_string(), "script.sh".to_string()]);
        assert_eq!(level, RiskLevel::High);
    }

    #[test]
    fn test_critical_risk_commands() {
        let scorer = RiskScorer::new();

        let (level, reason) = scorer.score("rm", &["-rf".to_string(), "/".to_string()]);
        assert_eq!(level, RiskLevel::Critical);
        assert!(reason.unwrap().contains("root"));

        let (level, _) = scorer.score("chmod", &["777".to_string(), "/etc".to_string()]);
        assert_eq!(level, RiskLevel::Critical);
    }

    #[test]
    fn test_pipe_to_bash_critical() {
        let scorer = RiskScorer::new();

        // Simulating: curl https://example.com/script.sh | bash
        let (level, reason) = scorer.score(
            "curl",
            &[
                "https://example.com/script.sh".to_string(),
                "|".to_string(),
                "bash".to_string(),
            ],
        );
        assert_eq!(level, RiskLevel::Critical);
        assert!(reason.unwrap().contains("curl | bash"));
    }

    #[test]
    fn test_custom_high_risk() {
        let mut scorer = RiskScorer::new();
        scorer.add_custom_high_risk(vec!["docker rm".to_string(), "kubectl delete".to_string()]);

        let (level, reason) = scorer.score("docker", &["rm".to_string(), "container".to_string()]);
        assert_eq!(level, RiskLevel::High);
        assert_eq!(reason, Some("Custom high-risk command"));

        let (level, _) = scorer.score(
            "kubectl",
            &["delete".to_string(), "pod".to_string(), "mypod".to_string()],
        );
        assert_eq!(level, RiskLevel::High);
    }

    #[test]
    fn test_score_returns_reason() {
        let scorer = RiskScorer::new();

        let (_, reason) = scorer.score("sudo", &["rm".to_string()]);
        assert_eq!(reason, Some("Privilege escalation"));

        let (_, reason) = scorer.score("ls", &[]);
        assert!(reason.is_none());
    }

    #[test]
    fn test_fork_bomb_detection() {
        let scorer = RiskScorer::new();

        let (_level, _reason) = scorer.score(":()", &["{:|:&};:".to_string()]);
        // Fork bomb pattern check - the pattern should be in args
        let (level, reason) = scorer.score("bash", &["-c".to_string(), ":(){:|:&};:".to_string()]);
        assert_eq!(level, RiskLevel::Critical);
        assert!(reason.unwrap().contains("Fork bomb"));
    }
}
