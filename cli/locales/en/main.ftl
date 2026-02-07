# MacAgentWatch CLI Messages

## Version
version-title = MacAgentWatch
version-subtitle = AI Agent Monitoring Tool for macOS
version-core-label = Core

## Usage / Banner
usage-title = MacAgentWatch
usage-subtitle = AI Agent Monitoring Tool
usage-label = USAGE:
usage-line1 = macagentwatch [OPTIONS] -- <COMMAND> [ARGS]...
usage-line2 = macagentwatch analyze <COMMAND> [ARGS]...
examples-label = EXAMPLES:
example-claude = macagentwatch -- claude-code "help me with this project"
example-json = macagentwatch --format json -- cursor
example-analyze = macagentwatch analyze rm -rf /tmp/cache
example-watch = macagentwatch --watch ~/projects -- your-agent
usage-help-hint = Run 'macagentwatch --help' for more options.

## Recording banner / footer
banner-recording = MacAgentWatch Recording
session-ended = Session ended (exit: { $exit_code })

## Analyze command
analyze-title = Command Analysis
analyze-command-label = Command:
analyze-risk-label = Risk Level:
analyze-reason-label = Reason:
analyze-danger-warning = This command may be dangerous!

## Risk levels
risk-low = LOW
risk-medium = MEDIUM
risk-high = HIGH
risk-critical = CRITICAL
risk-med-compact = MED
risk-crit-compact = CRIT

## Risk rule reasons (i18n keys from core/src/risk.rs)
risk-rm-rf-root = Recursive force delete of root directory
risk-rm-rf-root-contents = Recursive force delete of root contents
risk-chmod-world-writable = Setting world-writable permissions
risk-chmod-recursive-world-writable = Recursively setting world-writable permissions
risk-curl-to-bash = Piping remote script to shell (curl | bash)
risk-wget-to-bash = Piping remote script to shell (wget | bash)
risk-curl-to-sh = Piping remote script to shell (curl | sh)
risk-fork-bomb = Fork bomb detected
risk-rm-rf = Recursive force delete
risk-rm-recursive = Recursive delete
risk-sudo = Privilege escalation
risk-su = User switch
risk-ssh = Remote shell access
risk-scp = Remote file copy
risk-rsync = Remote sync
risk-chmod-exec = Adding execute permission
risk-chown = Changing file ownership
risk-mkfs = Formatting filesystem
risk-dd = Low-level disk operation
risk-curl = Network request
risk-wget = Network download
risk-pip-install = Python package installation
risk-npm-install = NPM package installation
risk-yarn-add = Yarn package installation
risk-brew-install = Homebrew package installation
risk-cargo-install = Cargo package installation
risk-git = Git operation
risk-docker = Docker operation
risk-custom-high = Custom high-risk command

## Errors
error-no-command = No command specified
error-pty-fallback = PTY failed ({ $error }), using simple mode
