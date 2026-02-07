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

## Errors
error-no-command = No command specified
error-pty-fallback = PTY failed ({ $error }), using simple mode
