# MacAgentWatch

![CI](https://github.com/ksyq12/agent-watch/actions/workflows/ci.yml/badge.svg)
![Version](https://img.shields.io/badge/version-0.3.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)
![Platform](https://img.shields.io/badge/platform-macOS-lightgrey)

A macOS-native monitoring and security tool that wraps AI coding agents (Claude Code, Cursor, Copilot, etc.) to provide real-time visibility into file system changes, network connections, and risky command execution.

## Key Features

- **Process Wrapping via PTY** -- Transparently wraps AI agent processes using `portable-pty`, capturing all commands and output in real time
- **Risk Scoring Engine** -- 134 built-in rules across four severity levels (Critical / High / Medium / Low) to flag destructive commands, privilege escalation, and pipe-to-shell patterns
- **File System Monitoring** -- Real-time tracking of file changes using macOS FSEvents, with configurable watch paths and debounce
- **Network Connection Tracking** -- Monitors TCP/UDP connections via `libproc`, with host whitelisting support
- **Sensitive Data Masking** -- 50+ detection patterns for API keys, tokens, passwords, and URLs to prevent accidental exposure in logs
- **Command Analysis** -- Standalone `analyze` subcommand for quick risk assessment of any command
- **Dual Storage Backends** -- Session logs saved as JSONL files and/or SQLite databases
- **Native macOS App** -- SwiftUI menu bar app with dashboard, session list, and VoiceOver accessibility
- **Internationalization** -- CLI uses fluent-rs; macOS app uses Localizable.strings

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   macOS App (Swift)                  │
│         SwiftUI  ·  MVVM  ·  Menu Bar               │
└──────────────────────┬──────────────────────────────┘
                       │ UniFFI Bridge
┌──────────────────────┴──────────────────────────────┐
│                  Core Library (Rust)                 │
│  Event System · Risk Scorer · Process Wrapper (PTY) │
│  FSEvents · Network Monitor · Sensitive Detection   │
│  Config (TOML) · Storage (JSONL + SQLite) · i18n    │
└──────────────────────┬──────────────────────────────┘
                       │
┌──────────────────────┴──────────────────────────────┐
│                     CLI (Rust)                       │
│          clap · fluent-rs · colored output           │
└─────────────────────────────────────────────────────┘
```

## Quick Start

### Prerequisites

- **Rust toolchain** (install via [rustup](https://rustup.rs/))
- **macOS** (FSEvents and libproc are macOS-native APIs)
- **Xcode** (only required for building the macOS app)

### Install from Source

```bash
git clone https://github.com/ksyq12/agent-watch.git
cd agent-watch
cargo build --release --workspace
```

The binary will be at `target/release/macagentwatch`. You can copy it to a directory on your `$PATH`:

```bash
cp target/release/macagentwatch /usr/local/bin/
```

## CLI Usage

### Wrap an AI agent

```bash
# Monitor Claude Code
macagentwatch -- claude-code "implement feature X"

# Monitor Cursor with JSON output and all monitors enabled
macagentwatch --format json --enable-fswatch --enable-netmon -- cursor

# Monitor with file system watching on a specific directory
macagentwatch --watch ~/projects -- your-agent

# Run in headless mode (no PTY, for CI/server use)
macagentwatch --headless -- script.sh
```

### Analyze a command without running it

```bash
macagentwatch analyze rm -rf /
macagentwatch analyze curl -s https://example.com | bash
macagentwatch analyze sudo chmod 777 /etc/passwd
```

### CLI Options

| Option | Description |
|---|---|
| `--format <pretty\|json\|compact>` | Output format (default: `pretty`) |
| `-l, --min-level <low\|medium\|high\|critical>` | Minimum risk level to display |
| `--no-color` | Disable colored output |
| `--no-timestamps` | Hide timestamps |
| `--watch <path>` | Watch directory for file changes (repeatable) |
| `--headless` | Run without PTY (for scripts/CI) |
| `--no-track-children` | Disable child process tracking |
| `--tracking-poll-ms <ms>` | Child tracking poll interval (default: 100) |
| `--enable-fswatch` | Enable file system monitoring |
| `--enable-netmon` | Enable network monitoring |
| `--log-dir <path>` | Directory for session logs |
| `-c, --config <path>` | Configuration file path |

## Configuration

MacAgentWatch reads configuration from `~/.macagentwatch/config.toml`. All fields are optional and fall back to sensible defaults.

```toml
[general]
verbose = false
default_format = "pretty"

[logging]
enabled = true
retention_days = 30
storage_backend = "jsonl"   # "jsonl", "sqlite", or "both"

[monitoring]
fs_enabled = false
net_enabled = false
track_children = true
tracking_poll_ms = 100
fs_debounce_ms = 100
net_poll_ms = 500
watch_paths = []
sensitive_patterns = [".env", ".env.*", "*.pem", "*.key", "*credential*", "*secret*"]
network_whitelist = ["api.anthropic.com", "github.com", "api.github.com"]

[alerts]
min_level = "high"
custom_high_risk = ["docker rm", "kubectl delete"]
```

## Building from Source

### Rust workspace (core + CLI)

```bash
# Debug build
cargo build --workspace

# Release build
cargo build --release --workspace

# Run tests
cargo test --workspace
```

### FFI bindings (UniFFI)

```bash
bash scripts/build-ffi.sh release
```

### macOS app (requires Xcode)

```bash
# Build FFI first, then the app
make build-app
```

Or open `app/MacAgentWatch/MacAgentWatch.xcodeproj` in Xcode directly.

## Development

```bash
make              # lint + test + build (default)
make build        # debug build
make build-release # release build
make test         # run all tests
make lint         # run clippy
make fmt          # check formatting
make fmt-fix      # auto-fix formatting
make clean        # remove all build artifacts
make help         # show all targets
```

### Test Suite

The project includes 300+ Rust tests and 71 Swift tests covering the core library, CLI, and macOS app.

```bash
# Run Rust tests
cargo test --workspace

# Run with output
cargo test --workspace -- --nocapture
```

## Project Structure

```
agent-watch/
├── core/                     # Rust core library (macagentwatch-core)
│   └── src/
│       ├── lib.rs            # Public API
│       ├── event.rs          # Event types and system
│       ├── risk.rs           # Risk scoring engine (134 rules)
│       ├── wrapper.rs        # Process wrapper (PTY)
│       ├── process_tracker.rs # Child process tracking
│       ├── fswatch.rs        # macOS FSEvents file monitoring
│       ├── netmon.rs         # Network monitoring (libproc)
│       ├── detector.rs       # Command pattern detection
│       ├── sanitize.rs       # Sensitive data masking (50+ patterns)
│       ├── config.rs         # TOML configuration
│       ├── storage.rs        # JSONL storage backend
│       ├── sqlite_storage.rs # SQLite storage backend
│       ├── logger.rs         # Formatted log output
│       ├── error.rs          # Error types
│       ├── ffi.rs            # UniFFI bridge definitions
│       └── types.rs          # i18n types module
├── cli/                      # CLI application (macagentwatch)
│   └── src/
│       ├── main.rs           # CLI entry point (clap)
│       └── i18n.rs           # fluent-rs localization
├── app/                      # macOS application (Swift)
│   ├── MacAgentWatch/
│   │   ├── MacAgentWatchApp.swift
│   │   ├── ContentView.swift
│   │   ├── Core/             # Bridge types and FFI wrappers
│   │   ├── ViewModels/       # MVVM view models (@Observable)
│   │   ├── Views/            # SwiftUI views (Dashboard, MenuBar, etc.)
│   │   └── en.lproj/         # Localized strings
│   └── MacAgentWatchTests/   # Swift unit tests
├── scripts/
│   └── build-ffi.sh          # UniFFI binding generation script
├── .github/workflows/
│   └── ci.yml                # GitHub Actions (fmt, clippy, test, build, audit)
├── Cargo.toml                # Workspace manifest
└── Makefile                  # Build automation
```

## Tech Stack

| Layer | Technology |
|---|---|
| Core library | Rust, serde, chrono, uuid |
| Process wrapping | portable-pty |
| File monitoring | macOS FSEvents (fsevent crate) |
| Network monitoring | libproc |
| Storage | JSONL (serde_json), SQLite (rusqlite) |
| Configuration | TOML (toml crate) |
| FFI bridge | UniFFI |
| CLI | clap, colored, fluent-rs |
| macOS app | Swift, SwiftUI, MVVM |
| CI | GitHub Actions |

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/your-feature`)
3. Ensure tests pass (`make test`) and lints are clean (`make lint`)
4. Format your code (`make fmt-fix`)
5. Commit your changes and open a pull request

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
