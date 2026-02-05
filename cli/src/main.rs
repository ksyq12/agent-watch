//! MacAgentWatch CLI
//!
//! Command-line interface for monitoring AI agents.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use macagentwatch_core::{
    Config, LogFormat, LoggerConfig, NetworkWhitelist, ProcessWrapper, RiskLevel, RiskScorer,
    WrapperConfig,
};
use std::path::PathBuf;

/// MacAgentWatch - AI Agent Monitoring Tool
#[derive(Parser)]
#[command(name = "macagentwatch")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Output format
    #[arg(short, long, value_enum, default_value = "pretty")]
    format: OutputFormat,

    /// Minimum risk level to display
    #[arg(short = 'l', long, value_enum, default_value = "low")]
    min_level: RiskLevelArg,

    /// Disable colored output
    #[arg(long)]
    no_color: bool,

    /// Hide timestamps
    #[arg(long)]
    no_timestamps: bool,

    /// Watch directory for file changes (can be specified multiple times)
    #[arg(short, long)]
    watch: Vec<String>,

    /// Run in headless mode (no PTY, for server use)
    #[arg(long)]
    headless: bool,

    /// Disable child process tracking
    #[arg(long)]
    no_track_children: bool,

    /// Polling interval for child process tracking (milliseconds)
    #[arg(long, default_value = "100")]
    tracking_poll_ms: u64,

    /// Enable file system monitoring
    #[arg(long)]
    enable_fswatch: bool,

    /// Enable network monitoring
    #[arg(long)]
    enable_netmon: bool,

    /// Directory to save session logs
    #[arg(long)]
    log_dir: Option<PathBuf>,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,

    /// Command to wrap (after --)
    #[arg(last = true, required = false)]
    cmd: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show version information
    Version,
    /// Run a quick risk analysis on a command
    Analyze {
        /// Command to analyze
        command: String,
        /// Command arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Pretty,
    Json,
    Compact,
}

impl From<OutputFormat> for LogFormat {
    fn from(f: OutputFormat) -> Self {
        match f {
            OutputFormat::Pretty => LogFormat::Pretty,
            OutputFormat::Json => LogFormat::JsonLines,
            OutputFormat::Compact => LogFormat::Compact,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
enum RiskLevelArg {
    Low,
    Medium,
    High,
    Critical,
}

impl From<RiskLevelArg> for RiskLevel {
    fn from(l: RiskLevelArg) -> Self {
        match l {
            RiskLevelArg::Low => RiskLevel::Low,
            RiskLevelArg::Medium => RiskLevel::Medium,
            RiskLevelArg::High => RiskLevel::High,
            RiskLevelArg::Critical => RiskLevel::Critical,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Version) => {
            print_version();
            Ok(())
        }
        Some(Commands::Analyze { command, args }) => {
            analyze_command(&command, &args, cli.format, cli.no_color)
        }
        None => {
            if cli.cmd.is_empty() {
                print_usage();
                Ok(())
            } else {
                run_wrapper(cli)
            }
        }
    }
}

fn print_version() {
    println!(
        "{} v{}",
        "MacAgentWatch".cyan().bold(),
        macagentwatch_core::VERSION
    );
    println!("AI Agent Monitoring Tool for macOS");
    println!();
    println!("Core: {}", macagentwatch_core::NAME);
}

fn print_usage() {
    println!("{}", "MacAgentWatch".cyan().bold());
    println!("AI Agent Monitoring Tool");
    println!();
    println!("{}", "USAGE:".yellow());
    println!("    macagentwatch [OPTIONS] -- <COMMAND> [ARGS]...");
    println!("    macagentwatch analyze <COMMAND> [ARGS]...");
    println!();
    println!("{}", "EXAMPLES:".yellow());
    println!("    macagentwatch -- claude-code \"help me with this project\"");
    println!("    macagentwatch --format json -- cursor");
    println!("    macagentwatch analyze rm -rf /tmp/cache");
    println!("    macagentwatch --watch ~/projects -- your-agent");
    println!();
    println!("Run 'macagentwatch --help' for more options.");
}

fn analyze_command(command: &str, args: &[String], format: OutputFormat, no_color: bool) -> Result<()> {
    let scorer = RiskScorer::new();
    let (level, reason) = scorer.score(command, args);

    let full_cmd = if args.is_empty() {
        command.to_string()
    } else {
        format!("{} {}", command, args.join(" "))
    };

    match format {
        OutputFormat::Pretty => {
            println!();
            println!("{}", "Command Analysis".cyan().bold());
            println!("{}", "─".repeat(50));
            println!();
            println!("  {} {}", "Command:".dimmed(), full_cmd);
            println!();

            let level_str = match level {
                RiskLevel::Low => {
                    if no_color {
                        format!("🟢 LOW")
                    } else {
                        format!("🟢 {}", "LOW".green())
                    }
                }
                RiskLevel::Medium => {
                    if no_color {
                        format!("🟡 MEDIUM")
                    } else {
                        format!("🟡 {}", "MEDIUM".yellow())
                    }
                }
                RiskLevel::High => {
                    if no_color {
                        format!("🟠 HIGH")
                    } else {
                        format!("🟠 {}", "HIGH".bright_yellow().bold())
                    }
                }
                RiskLevel::Critical => {
                    if no_color {
                        format!("🔴 CRITICAL")
                    } else {
                        format!("🔴 {}", "CRITICAL".red().bold())
                    }
                }
            };

            println!("  {} {}", "Risk Level:".dimmed(), level_str);

            if let Some(r) = reason {
                println!("  {} {}", "Reason:".dimmed(), r);
            }

            println!();

            if level >= RiskLevel::High {
                let warning = if no_color {
                    "⚠️  This command may be dangerous!".to_string()
                } else {
                    format!("⚠️  {}", "This command may be dangerous!".red())
                };
                println!("  {}", warning);
                println!();
            }
        }
        OutputFormat::Json => {
            let result = serde_json::json!({
                "command": command,
                "args": args,
                "risk_level": level.to_string(),
                "reason": reason,
                "alert": level >= RiskLevel::High,
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Compact => {
            let level_str = match level {
                RiskLevel::Low => "LOW",
                RiskLevel::Medium => "MED",
                RiskLevel::High => "HIGH",
                RiskLevel::Critical => "CRIT",
            };
            println!(
                "[{}] {} {}",
                level_str,
                full_cmd,
                reason.unwrap_or("")
            );
        }
    }

    Ok(())
}

fn run_wrapper(cli: Cli) -> Result<()> {
    let command = cli.cmd.first().context("No command specified")?;
    let args: Vec<String> = cli.cmd.iter().skip(1).cloned().collect();

    // Load config file if specified or use default
    let app_config = if let Some(ref path) = cli.config {
        Config::load_from_path(path).unwrap_or_default()
    } else {
        Config::load().unwrap_or_default()
    };

    // Build logger config
    let logger_config = LoggerConfig {
        format: cli.format.into(),
        min_level: cli.min_level.into(),
        show_timestamps: !cli.no_timestamps,
        use_colors: !cli.no_color,
    };

    // Determine watch paths from CLI and config
    let mut watch_paths: Vec<PathBuf> = cli.watch.iter().map(PathBuf::from).collect();
    if watch_paths.is_empty() {
        watch_paths = app_config.monitoring.watch_paths.clone();
    }

    // Determine log directory
    let log_dir = cli.log_dir.or_else(|| {
        dirs::data_local_dir().map(|d| d.join("macagentwatch").join("logs"))
    });

    // Build network whitelist
    let network_whitelist = NetworkWhitelist::new(
        app_config.monitoring.network_whitelist.clone(),
        vec![],
    );

    // Build wrapper config
    let mut config = WrapperConfig::new(command)
        .args(args)
        .logger_config(logger_config)
        .track_children(!cli.no_track_children)
        .tracking_poll_ms(cli.tracking_poll_ms)
        .enable_fswatch(cli.enable_fswatch)
        .watch_paths(watch_paths)
        .enable_netmon(cli.enable_netmon)
        .network_whitelist(network_whitelist);

    if let Some(dir) = log_dir {
        config = config.session_log_dir(dir);
    }

    // Print banner
    if !cli.no_color {
        println!();
        println!("{}", "╭─────────────────────────────────────╮".cyan());
        println!(
            "{}  {}  {}",
            "│".cyan(),
            "◉ MacAgentWatch Recording".green().bold(),
            "│".cyan()
        );
        println!("{}", "╰─────────────────────────────────────╯".cyan());
        println!();
    } else {
        println!();
        println!("◉ MacAgentWatch Recording");
        println!();
    }

    // Create and run wrapper
    let wrapper = ProcessWrapper::new(config);

    let exit_code = if cli.headless {
        wrapper.run_simple()?
    } else {
        // Try PTY first, fall back to simple if it fails
        wrapper.run().unwrap_or_else(|e| {
            eprintln!("PTY failed ({}), using simple mode", e);
            wrapper.run_simple().unwrap_or(-1)
        })
    };

    // Print footer
    if !cli.no_color {
        println!();
        println!("{}", "╭─────────────────────────────────────╮".cyan());
        println!(
            "{}  {}  {}",
            "│".cyan(),
            format!("Session ended (exit: {})", exit_code).dimmed(),
            "│".cyan()
        );
        println!("{}", "╰─────────────────────────────────────╯".cyan());
    } else {
        println!();
        println!("Session ended (exit: {})", exit_code);
    }

    std::process::exit(exit_code);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_basic() {
        let cli = Cli::parse_from(["macagentwatch", "--", "ls", "-la"]);
        assert_eq!(cli.cmd, vec!["ls", "-la"]);
        assert_eq!(cli.format, OutputFormat::Pretty);
    }

    #[test]
    fn test_cli_parse_with_format() {
        let cli = Cli::parse_from(["macagentwatch", "--format", "json", "--", "echo", "hello"]);
        assert_eq!(cli.format, OutputFormat::Json);
    }

    #[test]
    fn test_cli_parse_analyze() {
        let cli = Cli::parse_from(["macagentwatch", "analyze", "rm", "-rf", "/"]);
        match cli.command {
            Some(Commands::Analyze { command, args }) => {
                assert_eq!(command, "rm");
                assert_eq!(args, vec!["-rf", "/"]);
            }
            _ => panic!("Expected Analyze command"),
        }
    }

    #[test]
    fn test_cli_parse_headless() {
        let cli = Cli::parse_from(["macagentwatch", "--headless", "--", "script.sh"]);
        assert!(cli.headless);
    }

    #[test]
    fn test_cli_parse_min_level() {
        let cli = Cli::parse_from(["macagentwatch", "-l", "high", "--", "cmd"]);
        assert_eq!(cli.min_level, RiskLevelArg::High);
    }

    #[test]
    fn test_output_format_conversion() {
        assert_eq!(LogFormat::from(OutputFormat::Pretty), LogFormat::Pretty);
        assert_eq!(LogFormat::from(OutputFormat::Json), LogFormat::JsonLines);
        assert_eq!(LogFormat::from(OutputFormat::Compact), LogFormat::Compact);
    }

    #[test]
    fn test_risk_level_conversion() {
        assert_eq!(RiskLevel::from(RiskLevelArg::Low), RiskLevel::Low);
        assert_eq!(RiskLevel::from(RiskLevelArg::Critical), RiskLevel::Critical);
    }

    #[test]
    fn test_cli_parse_no_track_children() {
        let cli = Cli::parse_from(["macagentwatch", "--no-track-children", "--", "cmd"]);
        assert!(cli.no_track_children);
    }

    #[test]
    fn test_cli_parse_tracking_poll_ms() {
        let cli = Cli::parse_from(["macagentwatch", "--tracking-poll-ms", "50", "--", "cmd"]);
        assert_eq!(cli.tracking_poll_ms, 50);
    }

    #[test]
    fn test_cli_default_tracking_poll_ms() {
        let cli = Cli::parse_from(["macagentwatch", "--", "cmd"]);
        assert_eq!(cli.tracking_poll_ms, 100);
        assert!(!cli.no_track_children);
    }
}
