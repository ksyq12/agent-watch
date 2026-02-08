import Foundation

enum CoreBridgeError: LocalizedError {
    case engineInitFailed

    var errorDescription: String? {
        switch self {
        case .engineInitFailed:
            return "Failed to initialize monitoring engine"
        }
    }
}

@MainActor
final class CoreBridge {
    static let shared = CoreBridge()

    private var engine: FfiMonitoringEngine?

    private init() {}

    // MARK: - FFI Functions

    func loadConfig() -> AppConfig {
        do {
            let ffiConfig = try MacAgentWatch.loadConfig()
            return Self.convertConfig(ffiConfig)
        } catch {
            print("[CoreBridge] Warning: FFI loadConfig failed: \(error), using mock")
            return AppConfig()
        }
    }

    func analyzeCommand(command: String, args: [String]) -> MonitoringEvent {
        do {
            let ffiEvent = try MacAgentWatch.analyzeCommand(command: command, args: args)
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
        return MacAgentWatch.getVersion()
    }

    func readSessionLog(path: String) -> [MonitoringEvent] {
        do {
            let ffiEvents = try MacAgentWatch.readSessionLog(path: path)
            return ffiEvents.map { Self.convertEvent($0) }
        } catch {
            print("[CoreBridge] Warning: FFI readSessionLog failed: \(error), using mock")
            return Self.mockEvents
        }
    }

    func listSessionLogs() -> [SessionInfo] {
        do {
            let ffiSessions = try MacAgentWatch.listSessionLogs()
            return ffiSessions.map { Self.convertSessionInfo($0) }
        } catch {
            print("[CoreBridge] Warning: FFI listSessionLogs failed: \(error), using mock")
            return Self.mockSessions
        }
    }

    func getActivitySummary(events: [MonitoringEvent]) -> ActivitySummary {
        do {
            let ffiEvents = events.map { Self.convertToFfiEvent($0) }
            let ffiSummary = try MacAgentWatch.getActivitySummary(events: ffiEvents)
            return Self.convertActivitySummary(ffiSummary)
        } catch {
            print("[CoreBridge] Warning: FFI getActivitySummary failed: \(error), using empty summary")
            return ActivitySummary()
        }
    }

    // MARK: - v0.4.0 FFI Functions

    func readSessionLogPaginated(path: String, offset: UInt32, limit: UInt32) -> [MonitoringEvent] {
        do {
            let ffiEvents = try MacAgentWatch.readSessionLogPaginated(path: path, offset: offset, limit: limit)
            return ffiEvents.map { Self.convertEvent($0) }
        } catch {
            print("[CoreBridge] Warning: FFI readSessionLogPaginated failed: \(error)")
            return []
        }
    }

    func getSessionEventCount(path: String) -> Int {
        do {
            let count = try MacAgentWatch.getSessionEventCount(path: path)
            return Int(count)
        } catch {
            print("[CoreBridge] Warning: FFI getSessionEventCount failed: \(error)")
            return 0
        }
    }

    func getChartData(path: String, bucketMinutes: UInt32 = 60) -> [ChartDataPoint] {
        do {
            let ffiPoints = try MacAgentWatch.getChartData(path: path, bucketMinutes: bucketMinutes)
            return ffiPoints.map { Self.convertChartDataPoint($0) }
        } catch {
            print("[CoreBridge] Warning: FFI getChartData failed: \(error)")
            return []
        }
    }

    func searchEvents(
        path: String,
        query: String,
        eventTypeFilter: String? = nil,
        riskLevelFilter: FfiRiskLevel? = nil,
        startTimeMs: Int64? = nil,
        endTimeMs: Int64? = nil
    ) -> [MonitoringEvent] {
        do {
            let ffiEvents = try MacAgentWatch.searchEvents(
                path: path,
                query: query,
                eventTypeFilter: eventTypeFilter,
                riskLevelFilter: riskLevelFilter,
                startTimeMs: startTimeMs,
                endTimeMs: endTimeMs
            )
            return ffiEvents.map { Self.convertEvent($0) }
        } catch {
            print("[CoreBridge] Warning: FFI searchEvents failed: \(error)")
            return []
        }
    }

    func getLatestEvents(path: String, sinceIndex: UInt32) -> [MonitoringEvent] {
        do {
            let ffiEvents = try MacAgentWatch.getLatestEvents(path: path, sinceIndex: sinceIndex)
            return ffiEvents.map { Self.convertEvent($0) }
        } catch {
            print("[CoreBridge] Warning: FFI getLatestEvents failed: \(error)")
            return []
        }
    }

    // MARK: - v0.5.0 Config Save

    func saveConfig(_ config: AppConfig) {
        do {
            let ffiConfig = Self.convertToFfiConfig(config)
            try MacAgentWatch.saveConfig(config: ffiConfig)
        } catch {
            print("[CoreBridge] Warning: FFI saveConfig failed: \(error)")
        }
    }

    // MARK: - Monitoring Engine Management

    func startSession(processName: String) throws -> String {
        if engine == nil { engine = FfiMonitoringEngine() }
        guard let engine = engine else {
            throw CoreBridgeError.engineInitFailed
        }
        return try engine.startSession(processName: processName)
    }

    func stopSession() -> Bool {
        do {
            try engine?.stopSession()
            return true
        } catch {
            print("[CoreBridge] Warning: FFI stopSession failed: \(error)")
            return false
        }
    }

    func isEngineActive() -> Bool {
        do {
            return try engine?.isActive() ?? false
        } catch {
            return false
        }
    }

    func getMonitoredAgents() -> [FfiDetectedAgent] {
        do {
            guard let engine = engine else { return [] }
            return try engine.getMonitoredAgents()
        } catch {
            print("[CoreBridge] Warning: FFI getMonitoredAgents failed: \(error)")
            return []
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
        config.notifications.enabled = ffiConfig.notification.enabled
        config.notifications.minRiskLevel = RiskLevel(rawValue: ffiConfig.notification.minRiskLevel) ?? .high
        config.notifications.soundEnabled = ffiConfig.notification.soundEnabled
        config.notifications.badgeEnabled = ffiConfig.notification.badgeEnabled
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

    private static func convertChartDataPoint(_ ffiPoint: FfiChartDataPoint) -> ChartDataPoint {
        let timestamp = Date(timeIntervalSince1970: TimeInterval(ffiPoint.timestampMs) / 1000.0)
        return ChartDataPoint(
            timestamp: timestamp,
            total: Int(ffiPoint.total),
            critical: Int(ffiPoint.critical),
            high: Int(ffiPoint.high),
            medium: Int(ffiPoint.medium),
            low: Int(ffiPoint.low)
        )
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

    // MARK: - Swift → FFI Config Conversion

    private static func convertToFfiConfig(_ config: AppConfig) -> FfiConfig {
        let general = FfiGeneralConfig(
            verbose: config.general.verbose,
            defaultFormat: config.general.defaultFormat
        )
        let logging = FfiLoggingConfig(
            enabled: config.logging.enabled,
            logDir: config.logging.logDir,
            retentionDays: config.logging.retentionDays
        )
        let monitoring = FfiMonitoringConfig(
            fsEnabled: config.monitoring.fsEnabled,
            netEnabled: config.monitoring.netEnabled,
            trackChildren: config.monitoring.trackChildren,
            trackingPollMs: config.monitoring.trackingPollMs,
            fsDebounceMs: config.monitoring.fsDebounceMs,
            netPollMs: config.monitoring.netPollMs,
            watchPaths: config.monitoring.watchPaths,
            sensitivePatterns: config.monitoring.sensitivePatterns,
            networkWhitelist: config.monitoring.networkWhitelist
        )
        let alerts = FfiAlertConfig(
            minLevel: config.alerts.minLevel,
            customHighRisk: config.alerts.customHighRisk
        )
        let notification = FfiNotificationConfig(
            enabled: config.notifications.enabled,
            minRiskLevel: config.notifications.minRiskLevel.rawValue,
            soundEnabled: config.notifications.soundEnabled,
            badgeEnabled: config.notifications.badgeEnabled
        )
        return FfiConfig(
            general: general,
            logging: logging,
            monitoring: monitoring,
            alerts: alerts,
            notification: notification
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
