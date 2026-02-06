import Foundation

@MainActor
final class CoreBridge {
    static let shared = CoreBridge()

    private init() {}

    func loadConfig() -> AppConfig {
        // TODO: Replace with real FFI call to macagentwatch_core.load_config()
        return AppConfig()
    }

    func analyzeCommand(command: String, args: [String]) -> MonitoringEvent {
        // TODO: Replace with real FFI call to macagentwatch_core.analyze_command()
        return MonitoringEvent(
            id: UUID().uuidString,
            timestamp: Date(),
            eventType: .command(command: command, args: args, exitCode: nil),
            process: "mock",
            pid: UInt32(ProcessInfo.processInfo.processIdentifier),
            riskLevel: .low,
            alert: false
        )
    }

    func getVersion() -> String {
        // TODO: Replace with real FFI call to macagentwatch_core.get_version()
        return "0.3.0-mock"
    }

    func readSessionLog(path: String) -> [MonitoringEvent] {
        // TODO: Replace with real FFI call
        return Self.mockEvents
    }

    func listSessionLogs() -> [SessionInfo] {
        // TODO: Replace with real FFI call
        return Self.mockSessions
    }

    func getActivitySummary(events: [MonitoringEvent]) -> ActivitySummary {
        // TODO: Replace with real FFI call
        var summary = ActivitySummary()
        summary.totalEvents = events.count
        for event in events {
            switch event.riskLevel {
            case .critical: summary.criticalCount += 1
            case .high: summary.highCount += 1
            case .medium: summary.mediumCount += 1
            case .low: summary.lowCount += 1
            }
        }
        return summary
    }

    // MARK: - Mock Data

    static let mockEvents: [MonitoringEvent] = [
        MonitoringEvent(
            id: UUID().uuidString,
            timestamp: Date(),
            eventType: .command(command: "ls", args: ["-la"], exitCode: 0),
            process: "zsh", pid: 1234, riskLevel: .low, alert: false
        ),
        MonitoringEvent(
            id: UUID().uuidString,
            timestamp: Date().addingTimeInterval(-60),
            eventType: .command(command: "curl", args: ["https://api.example.com"], exitCode: 0),
            process: "zsh", pid: 1234, riskLevel: .medium, alert: false
        ),
        MonitoringEvent(
            id: UUID().uuidString,
            timestamp: Date().addingTimeInterval(-120),
            eventType: .command(command: "rm", args: ["-rf", "temp/"], exitCode: 0),
            process: "zsh", pid: 1234, riskLevel: .high, alert: true
        ),
        MonitoringEvent(
            id: UUID().uuidString,
            timestamp: Date().addingTimeInterval(-180),
            eventType: .command(command: "chmod", args: ["777", "/etc/passwd"], exitCode: 1),
            process: "zsh", pid: 1234, riskLevel: .critical, alert: true
        ),
        MonitoringEvent(
            id: UUID().uuidString,
            timestamp: Date().addingTimeInterval(-240),
            eventType: .fileAccess(path: "/Users/test/.env", action: .read),
            process: "cat", pid: 5678, riskLevel: .high, alert: true
        ),
        MonitoringEvent(
            id: UUID().uuidString,
            timestamp: Date().addingTimeInterval(-300),
            eventType: .network(host: "suspicious.example.com", port: 443, protocol: "tcp"),
            process: "curl", pid: 9012, riskLevel: .medium, alert: false
        ),
    ]

    static let mockSessions: [SessionInfo] = [
        SessionInfo(
            id: "session-1",
            sessionId: "session-20260206-143022-a1b2c3d4",
            filePath: "~/.macagentwatch/logs/session-20260206-143022-a1b2c3d4.jsonl",
            startTime: Date(),
            startTimeString: "2026-02-06T14:30:22Z"
        ),
        SessionInfo(
            id: "session-2",
            sessionId: "session-20260205-091500-e5f6g7h8",
            filePath: "~/.macagentwatch/logs/session-20260205-091500-e5f6g7h8.jsonl",
            startTime: Date().addingTimeInterval(-86400),
            startTimeString: "2026-02-05T09:15:00Z"
        ),
    ]
}
