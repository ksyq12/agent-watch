import Foundation
import macagentwatch_core

@MainActor
final class CoreBridge {
    static let shared = CoreBridge()

    private init() {}

    // MARK: - FFI Functions

    func loadConfig() -> AppConfig {
        do {
            let ffiConfig = try macagentwatch_core.loadConfig()
            return Self.convertConfig(ffiConfig)
        } catch {
            print("[CoreBridge] Warning: FFI loadConfig failed: \(error), using mock")
            return AppConfig()
        }
    }

    func analyzeCommand(command: String, args: [String]) -> MonitoringEvent {
        do {
            let ffiEvent = try macagentwatch_core.analyzeCommand(command: command, args: args)
            return Self.convertEvent(ffiEvent)
        } catch {
            print("[CoreBridge] Warning: FFI analyzeCommand failed: \(error), using fallback")
            return MonitoringEvent(
                id: UUID().uuidString,
                timestamp: Date(),
                eventType: .command(command: command, args: args, exitCode: nil),
                process: "unknown", pid: 0, riskLevel: .low, alert: false
            )
        }
    }

    func getVersion() -> String {
        return macagentwatch_core.getVersion()
    }

    func readSessionLog(path: String) -> [MonitoringEvent] {
        do {
            let ffiEvents = try macagentwatch_core.readSessionLog(path: path)
            return ffiEvents.map { Self.convertEvent($0) }
        } catch {
            print("[CoreBridge] Warning: FFI readSessionLog failed: \(error), using mock")
            return Self.mockEvents
        }
    }

    func listSessionLogs() -> [SessionInfo] {
        do {
            let ffiSessions = try macagentwatch_core.listSessionLogs()
            return ffiSessions.map { Self.convertSessionInfo($0) }
        } catch {
            print("[CoreBridge] Warning: FFI listSessionLogs failed: \(error), using mock")
            return Self.mockSessions
        }
    }

    func getActivitySummary(events: [MonitoringEvent]) -> ActivitySummary {
        do {
            let ffiEvents = events.map { Self.convertToFfiEvent($0) }
            let ffiSummary = try macagentwatch_core.getActivitySummary(events: ffiEvents)
            return Self.convertActivitySummary(ffiSummary)
        } catch {
            print("[CoreBridge] Warning: FFI getActivitySummary failed: \(error), using empty summary")
            return ActivitySummary()
        }
    }

    // MARK: - FFI → Swift Type Conversions

    private static func convertRiskLevel(_ ffiLevel: FfiRiskLevel) -> RiskLevel {
        switch ffiLevel {
        case .low: return .low
        case .medium: return .medium
        case .high: return .high
        case .critical: return .critical
        }
    }

    private static func convertFileAction(_ ffiAction: FfiFileAction) -> FileAction {
        switch ffiAction {
        case .read: return .read
        case .write: return .write
        case .delete: return .delete
        case .create: return .create
        case .chmod: return .chmod
        }
    }

    private static func convertProcessAction(_ ffiAction: FfiProcessAction) -> ProcessAction {
        switch ffiAction {
        case .start: return .start
        case .exit: return .exit
        case .fork: return .fork
        }
    }

    private static func convertSessionAction(_ ffiAction: FfiSessionAction) -> SessionAction {
        switch ffiAction {
        case .start: return .start
        case .end: return .end
        }
    }

    private static func convertEventType(_ ffiEventType: FfiEventType) -> EventType {
        switch ffiEventType {
        case .command(let command, let args, let exitCode):
            return .command(command: command, args: args, exitCode: exitCode)
        case .fileAccess(let path, let action):
            return .fileAccess(path: path, action: convertFileAction(action))
        case .network(let host, let port, let proto):
            return .network(host: host, port: port, protocol: proto)
        case .process(let pid, let ppid, let action):
            return .process(pid: pid, ppid: ppid, action: convertProcessAction(action))
        case .session(let action):
            return .session(action: convertSessionAction(action))
        }
    }

    private static func convertEvent(_ ffiEvent: FfiEvent) -> MonitoringEvent {
        let timestamp = Date(timeIntervalSince1970: TimeInterval(ffiEvent.timestampMs) / 1000.0)
        return MonitoringEvent(
            id: ffiEvent.id,
            timestamp: timestamp,
            eventType: convertEventType(ffiEvent.eventType),
            process: ffiEvent.process,
            pid: ffiEvent.pid,
            riskLevel: convertRiskLevel(ffiEvent.riskLevel),
            alert: ffiEvent.alert
        )
    }

    private static func convertConfig(_ ffiConfig: FfiConfig) -> AppConfig {
        var config = AppConfig()
        config.general.verbose = ffiConfig.general.verbose
        config.general.defaultFormat = ffiConfig.general.defaultFormat
        config.logging.enabled = ffiConfig.logging.enabled
        config.logging.logDir = ffiConfig.logging.logDir
        config.logging.retentionDays = ffiConfig.logging.retentionDays
        config.monitoring.fsEnabled = ffiConfig.monitoring.fsEnabled
        config.monitoring.netEnabled = ffiConfig.monitoring.netEnabled
        config.monitoring.trackChildren = ffiConfig.monitoring.trackChildren
        config.monitoring.trackingPollMs = ffiConfig.monitoring.trackingPollMs
        config.monitoring.fsDebounceMs = ffiConfig.monitoring.fsDebounceMs
        config.monitoring.netPollMs = ffiConfig.monitoring.netPollMs
        config.monitoring.watchPaths = ffiConfig.monitoring.watchPaths
        config.monitoring.sensitivePatterns = ffiConfig.monitoring.sensitivePatterns
        config.monitoring.networkWhitelist = ffiConfig.monitoring.networkWhitelist
        config.alerts.minLevel = ffiConfig.alerts.minLevel
        config.alerts.customHighRisk = ffiConfig.alerts.customHighRisk
        return config
    }

    private static func convertSessionInfo(_ ffiSession: FfiSessionInfo) -> SessionInfo {
        let iso8601Formatter = ISO8601DateFormatter()
        let startTime = iso8601Formatter.date(from: ffiSession.startTimeStr)

        return SessionInfo(
            id: ffiSession.sessionId,
            sessionId: ffiSession.sessionId,
            filePath: ffiSession.filePath,
            startTime: startTime,
            startTimeString: ffiSession.startTimeStr
        )
    }

    private static func convertActivitySummary(_ ffiSummary: FfiActivitySummary) -> ActivitySummary {
        var summary = ActivitySummary()
        summary.totalEvents = Int(ffiSummary.totalEvents)
        summary.criticalCount = Int(ffiSummary.criticalCount)
        summary.highCount = Int(ffiSummary.highCount)
        summary.mediumCount = Int(ffiSummary.mediumCount)
        summary.lowCount = Int(ffiSummary.lowCount)
        return summary
    }

    // MARK: - Swift → FFI Type Conversions (for getActivitySummary)

    private static func convertToFfiRiskLevel(_ level: RiskLevel) -> FfiRiskLevel {
        switch level {
        case .low: return .low
        case .medium: return .medium
        case .high: return .high
        case .critical: return .critical
        }
    }

    private static func convertToFfiFileAction(_ action: FileAction) -> FfiFileAction {
        switch action {
        case .read: return .read
        case .write: return .write
        case .delete: return .delete
        case .create: return .create
        case .chmod: return .chmod
        }
    }

    private static func convertToFfiProcessAction(_ action: ProcessAction) -> FfiProcessAction {
        switch action {
        case .start: return .start
        case .exit: return .exit
        case .fork: return .fork
        }
    }

    private static func convertToFfiSessionAction(_ action: SessionAction) -> FfiSessionAction {
        switch action {
        case .start: return .start
        case .end: return .end
        }
    }

    private static func convertToFfiEventType(_ eventType: EventType) -> FfiEventType {
        switch eventType {
        case .command(let command, let args, let exitCode):
            return .command(command: command, args: args, exitCode: exitCode)
        case .fileAccess(let path, let action):
            return .fileAccess(path: path, action: convertToFfiFileAction(action))
        case .network(let host, let port, let proto):
            return .network(host: host, port: port, protocol: proto)
        case .process(let pid, let ppid, let action):
            return .process(pid: pid, ppid: ppid, action: convertToFfiProcessAction(action))
        case .session(let action):
            return .session(action: convertToFfiSessionAction(action))
        }
    }

    private static func convertToFfiEvent(_ event: MonitoringEvent) -> FfiEvent {
        let timestampMs = Int64(event.timestamp.timeIntervalSince1970 * 1000.0)
        let iso8601Formatter = ISO8601DateFormatter()
        let timestampStr = iso8601Formatter.string(from: event.timestamp)

        return FfiEvent(
            id: event.id,
            timestampMs: timestampMs,
            timestampStr: timestampStr,
            eventType: convertToFfiEventType(event.eventType),
            process: event.process,
            pid: event.pid,
            riskLevel: convertToFfiRiskLevel(event.riskLevel),
            alert: event.alert
        )
    }

    // MARK: - Fallback Mock Data

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
