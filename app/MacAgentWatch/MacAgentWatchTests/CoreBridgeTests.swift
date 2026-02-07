import XCTest
@testable import MacAgentWatch

@MainActor
final class CoreBridgeTests: XCTestCase {

    private var bridge: CoreBridge!

    override func setUp() {
        super.setUp()
        bridge = CoreBridge.shared
    }

    // MARK: - analyzeCommand

    func testAnalyzeCommandReturnsValidEvent() {
        let event = bridge.analyzeCommand(command: "ls", args: ["-la"])

        XCTAssertFalse(event.id.isEmpty)
        XCTAssertEqual(event.process, "agent", "Real FFI analyzeCommand returns process 'agent'")
        XCTAssertEqual(event.riskLevel, .low)
        XCTAssertFalse(event.alert)

        if case .command(let cmd, let args, let exitCode) = event.eventType {
            XCTAssertEqual(cmd, "ls")
            XCTAssertEqual(args, ["-la"])
            XCTAssertNil(exitCode)
        } else {
            XCTFail("Expected .command event type")
        }
    }

    func testAnalyzeCommandWithEmptyArgs() {
        let event = bridge.analyzeCommand(command: "pwd", args: [])

        if case .command(let cmd, let args, _) = event.eventType {
            XCTAssertEqual(cmd, "pwd")
            XCTAssertTrue(args.isEmpty)
        } else {
            XCTFail("Expected .command event type")
        }
    }

    func testAnalyzeCommandTimestampIsRecent() {
        let before = Date().addingTimeInterval(-2)
        let event = bridge.analyzeCommand(command: "echo", args: ["hello"])
        let after = Date().addingTimeInterval(2)

        XCTAssertGreaterThanOrEqual(event.timestamp, before)
        XCTAssertLessThanOrEqual(event.timestamp, after)
    }

    func testAnalyzeCommandUniqueIds() {
        let event1 = bridge.analyzeCommand(command: "ls", args: [])
        let event2 = bridge.analyzeCommand(command: "ls", args: [])
        XCTAssertNotEqual(event1.id, event2.id)
    }

    func testAnalyzeCommandPidIsCurrentProcess() {
        let event = bridge.analyzeCommand(command: "test", args: [])
        let expectedPid = UInt32(ProcessInfo.processInfo.processIdentifier)
        XCTAssertEqual(event.pid, expectedPid)
    }

    // MARK: - getVersion

    func testGetVersionReturnsNonEmpty() {
        let version = bridge.getVersion()
        XCTAssertFalse(version.isEmpty)
        XCTAssertEqual(version, "0.3.0")
    }

    // MARK: - loadConfig

    func testLoadConfigReturnsDefault() {
        let config = bridge.loadConfig()
        XCTAssertFalse(config.general.verbose)
        XCTAssertEqual(config.general.defaultFormat, "pretty")
        XCTAssertTrue(config.logging.enabled)
        XCTAssertEqual(config.logging.retentionDays, 30)
        XCTAssertNil(config.logging.logDir)
        XCTAssertFalse(config.monitoring.fsEnabled)
        XCTAssertFalse(config.monitoring.netEnabled)
        XCTAssertTrue(config.monitoring.trackChildren)
        XCTAssertEqual(config.alerts.minLevel, "high")
    }

    // MARK: - getActivitySummary

    func testGetActivitySummaryEmptyEvents() {
        let summary = bridge.getActivitySummary(events: [])
        XCTAssertEqual(summary.totalEvents, 0)
        XCTAssertEqual(summary.criticalCount, 0)
        XCTAssertEqual(summary.highCount, 0)
        XCTAssertEqual(summary.mediumCount, 0)
        XCTAssertEqual(summary.lowCount, 0)
    }

    func testGetActivitySummaryCountsCorrectly() {
        let events = [
            makeEvent(riskLevel: .low),
            makeEvent(riskLevel: .low),
            makeEvent(riskLevel: .medium),
            makeEvent(riskLevel: .high),
            makeEvent(riskLevel: .high),
            makeEvent(riskLevel: .high),
            makeEvent(riskLevel: .critical),
        ]
        let summary = bridge.getActivitySummary(events: events)

        XCTAssertEqual(summary.totalEvents, 7)
        XCTAssertEqual(summary.lowCount, 2)
        XCTAssertEqual(summary.mediumCount, 1)
        XCTAssertEqual(summary.highCount, 3)
        XCTAssertEqual(summary.criticalCount, 1)
    }

    func testGetActivitySummaryAllSameLevel() {
        let events = [
            makeEvent(riskLevel: .critical),
            makeEvent(riskLevel: .critical),
            makeEvent(riskLevel: .critical),
        ]
        let summary = bridge.getActivitySummary(events: events)

        XCTAssertEqual(summary.totalEvents, 3)
        XCTAssertEqual(summary.criticalCount, 3)
        XCTAssertEqual(summary.highCount, 0)
    }

    // MARK: - listSessionLogs

    func testListSessionLogsDoesNotCrash() {
        let sessions = bridge.listSessionLogs()
        // Real FFI may return 0 sessions if no log files exist on disk
        XCTAssertNotNil(sessions, "listSessionLogs should return a valid array")
    }

    func testListSessionLogsHaveValidIds() throws {
        let sessions = bridge.listSessionLogs()
        // Only validate if sessions exist on disk
        try XCTSkipIf(sessions.isEmpty, "No session logs on disk to validate")
        for session in sessions {
            XCTAssertFalse(session.id.isEmpty)
            XCTAssertFalse(session.sessionId.isEmpty)
            XCTAssertFalse(session.filePath.isEmpty)
        }
    }

    func testListSessionLogsSessionIdsAreUnique() throws {
        let sessions = bridge.listSessionLogs()
        // Only validate if sessions exist on disk
        try XCTSkipIf(sessions.isEmpty, "No session logs on disk to validate")
        let ids = sessions.map { $0.id }
        XCTAssertEqual(Set(ids).count, ids.count)
    }

    // MARK: - readSessionLog

    func testReadSessionLogFallsBackForNonExistentPath() {
        let events = bridge.readSessionLog(path: "/nonexistent/path")
        // Real FFI fails for non-existent path, falls back to mockEvents
        XCTAssertFalse(events.isEmpty)
        XCTAssertEqual(events.count, CoreBridge.mockEvents.count)
    }

    // MARK: - Mock Data Variety

    func testMockEventsContainVariousRiskLevels() {
        let levels = Set(CoreBridge.mockEvents.map { $0.riskLevel })
        XCTAssertTrue(levels.contains(.low))
        XCTAssertTrue(levels.contains(.medium))
        XCTAssertTrue(levels.contains(.high))
        XCTAssertTrue(levels.contains(.critical))
    }

    func testMockEventsContainVariousEventTypes() {
        var hasCommand = false, hasFileAccess = false, hasNetwork = false
        for event in CoreBridge.mockEvents {
            switch event.eventType {
            case .command: hasCommand = true
            case .fileAccess: hasFileAccess = true
            case .network: hasNetwork = true
            default: break
            }
        }
        XCTAssertTrue(hasCommand)
        XCTAssertTrue(hasFileAccess)
        XCTAssertTrue(hasNetwork)
    }

    // MARK: - Helpers

    private func makeEvent(riskLevel: RiskLevel) -> MonitoringEvent {
        MonitoringEvent(
            id: UUID().uuidString,
            timestamp: Date(),
            eventType: .command(command: "test", args: [], exitCode: 0),
            process: "test",
            pid: 1,
            riskLevel: riskLevel,
            alert: riskLevel >= .high
        )
    }
}
