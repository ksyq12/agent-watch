import XCTest
@testable import MacAgentWatch

final class MonitoringTypesTests: XCTestCase {

    // MARK: - RiskLevel Ordering

    func testRiskLevelOrdering() {
        XCTAssertTrue(RiskLevel.low < RiskLevel.medium)
        XCTAssertTrue(RiskLevel.medium < RiskLevel.high)
        XCTAssertTrue(RiskLevel.high < RiskLevel.critical)
        XCTAssertFalse(RiskLevel.critical < RiskLevel.low)
    }

    func testRiskLevelSorted() {
        let shuffled: [RiskLevel] = [.critical, .low, .high, .medium]
        let sorted = shuffled.sorted()
        XCTAssertEqual(sorted, [.low, .medium, .high, .critical])
    }

    func testRiskLevelNotLessThanSelf() {
        XCTAssertFalse(RiskLevel.low < RiskLevel.low)
        XCTAssertFalse(RiskLevel.critical < RiskLevel.critical)
    }

    // MARK: - RiskLevel Colors

    func testRiskLevelColors() {
        XCTAssertEqual(RiskLevel.low.color, "green")
        XCTAssertEqual(RiskLevel.medium.color, "yellow")
        XCTAssertEqual(RiskLevel.high.color, "orange")
        XCTAssertEqual(RiskLevel.critical.color, "red")
    }

    // MARK: - RiskLevel Icons

    func testRiskLevelIcons() {
        XCTAssertEqual(RiskLevel.low.icon, "checkmark.circle.fill")
        XCTAssertEqual(RiskLevel.medium.icon, "exclamationmark.triangle.fill")
        XCTAssertEqual(RiskLevel.high.icon, "exclamationmark.octagon.fill")
        XCTAssertEqual(RiskLevel.critical.icon, "xmark.octagon.fill")
    }

    // MARK: - RiskLevel Labels

    func testRiskLevelLabels() {
        XCTAssertEqual(RiskLevel.low.label, "Low")
        XCTAssertEqual(RiskLevel.medium.label, "Medium")
        XCTAssertEqual(RiskLevel.high.label, "High")
        XCTAssertEqual(RiskLevel.critical.label, "Critical")
    }

    // MARK: - RiskLevel CaseIterable

    func testRiskLevelAllCases() {
        XCTAssertEqual(RiskLevel.allCases.count, 4)
        XCTAssertEqual(RiskLevel.allCases, [.low, .medium, .high, .critical])
    }

    // MARK: - RiskLevel Raw Values

    func testRiskLevelRawValues() {
        XCTAssertEqual(RiskLevel.low.rawValue, "low")
        XCTAssertEqual(RiskLevel.medium.rawValue, "medium")
        XCTAssertEqual(RiskLevel.high.rawValue, "high")
        XCTAssertEqual(RiskLevel.critical.rawValue, "critical")
    }

    func testRiskLevelInitFromRawValue() {
        XCTAssertEqual(RiskLevel(rawValue: "low"), .low)
        XCTAssertEqual(RiskLevel(rawValue: "critical"), .critical)
        XCTAssertNil(RiskLevel(rawValue: "unknown"))
    }

    // MARK: - FileAction

    func testFileActionRawValues() {
        XCTAssertEqual(FileAction.read.rawValue, "read")
        XCTAssertEqual(FileAction.write.rawValue, "write")
        XCTAssertEqual(FileAction.delete.rawValue, "delete")
        XCTAssertEqual(FileAction.create.rawValue, "create")
        XCTAssertEqual(FileAction.chmod.rawValue, "chmod")
    }

    // MARK: - ProcessAction

    func testProcessActionRawValues() {
        XCTAssertEqual(ProcessAction.start.rawValue, "start")
        XCTAssertEqual(ProcessAction.exit.rawValue, "exit")
        XCTAssertEqual(ProcessAction.fork.rawValue, "fork")
    }

    // MARK: - SessionAction

    func testSessionActionRawValues() {
        XCTAssertEqual(SessionAction.start.rawValue, "start")
        XCTAssertEqual(SessionAction.end.rawValue, "end")
    }

    // MARK: - EventType Icons

    func testEventTypeCommandIcon() {
        let event = EventType.command(command: "ls", args: [], exitCode: 0)
        XCTAssertEqual(event.icon, "terminal")
    }

    func testEventTypeFileAccessIcon() {
        let event = EventType.fileAccess(path: "/tmp/test", action: .read)
        XCTAssertEqual(event.icon, "doc")
    }

    func testEventTypeNetworkIcon() {
        let event = EventType.network(host: "example.com", port: 443, protocol: "tcp")
        XCTAssertEqual(event.icon, "network")
    }

    func testEventTypeProcessIcon() {
        let event = EventType.process(pid: 100, ppid: 1, action: .start)
        XCTAssertEqual(event.icon, "gearshape.2")
    }

    func testEventTypeSessionIcon() {
        let event = EventType.session(action: .start)
        XCTAssertEqual(event.icon, "play.circle")
    }

    // MARK: - EventType Descriptions

    func testCommandDescriptionNoArgs() {
        let event = EventType.command(command: "ls", args: [], exitCode: 0)
        XCTAssertEqual(event.description, "ls")
    }

    func testCommandDescriptionWithArgs() {
        let event = EventType.command(command: "ls", args: ["-la", "/tmp"], exitCode: 0)
        XCTAssertEqual(event.description, "ls -la /tmp")
    }

    func testCommandDescriptionNilExitCode() {
        let event = EventType.command(command: "sleep", args: ["10"], exitCode: nil)
        XCTAssertEqual(event.description, "sleep 10")
    }

    func testFileAccessDescription() {
        let event = EventType.fileAccess(path: "/Users/test/.env", action: .read)
        XCTAssertEqual(event.description, "read: /Users/test/.env")
    }

    func testNetworkDescription() {
        let event = EventType.network(host: "example.com", port: 443, protocol: "tcp")
        XCTAssertEqual(event.description, "tcp://example.com:443")
    }

    func testProcessDescription() {
        let event = EventType.process(pid: 1234, ppid: 1, action: .start)
        XCTAssertEqual(event.description, "start (PID: 1234)")
    }

    func testSessionDescription() {
        let event = EventType.session(action: .start)
        XCTAssertEqual(event.description, "Session start")
    }

    // MARK: - MonitoringEvent

    func testMonitoringEventProperties() {
        let event = MonitoringEvent(
            id: "test-id-123",
            timestamp: Date(),
            eventType: .command(command: "echo", args: ["hello"], exitCode: 0),
            process: "zsh",
            pid: 1000,
            riskLevel: .low,
            alert: false
        )
        XCTAssertEqual(event.id, "test-id-123")
        XCTAssertEqual(event.process, "zsh")
        XCTAssertEqual(event.pid, 1000)
        XCTAssertEqual(event.riskLevel, .low)
        XCTAssertFalse(event.alert)
    }

    func testMonitoringEventWithAlert() {
        let event = MonitoringEvent(
            id: "alert-event",
            timestamp: Date(),
            eventType: .command(command: "rm", args: ["-rf", "/"], exitCode: nil),
            process: "bash",
            pid: 2000,
            riskLevel: .critical,
            alert: true
        )
        XCTAssertTrue(event.alert)
        XCTAssertEqual(event.riskLevel, .critical)
    }

    // MARK: - ActivitySummary

    func testActivitySummaryEmpty() {
        let summary = ActivitySummary.empty
        XCTAssertEqual(summary.totalEvents, 0)
        XCTAssertEqual(summary.criticalCount, 0)
        XCTAssertEqual(summary.highCount, 0)
        XCTAssertEqual(summary.mediumCount, 0)
        XCTAssertEqual(summary.lowCount, 0)
    }

    func testActivitySummaryMutation() {
        var summary = ActivitySummary()
        summary.totalEvents = 10
        summary.criticalCount = 1
        summary.highCount = 2
        summary.mediumCount = 3
        summary.lowCount = 4
        XCTAssertEqual(summary.totalEvents, 10)
        XCTAssertEqual(summary.criticalCount, 1)
    }

    // MARK: - SessionInfo

    func testSessionInfoProperties() {
        let session = SessionInfo(
            id: "s1",
            sessionId: "session-20260206-143022-a1b2c3d4",
            filePath: "~/.macagentwatch/logs/session.jsonl",
            startTime: Date(),
            startTimeString: "2026-02-06T14:30:22Z"
        )
        XCTAssertEqual(session.id, "s1")
        XCTAssertEqual(session.sessionId, "session-20260206-143022-a1b2c3d4")
    }

    func testSessionInfoHashable() {
        let session1 = SessionInfo(
            id: "s1", sessionId: "abc", filePath: "/a", startTime: nil, startTimeString: "t1"
        )
        let session2 = SessionInfo(
            id: "s1", sessionId: "abc", filePath: "/a", startTime: nil, startTimeString: "t1"
        )
        var set = Set<SessionInfo>()
        set.insert(session1)
        set.insert(session2)
        XCTAssertEqual(set.count, 1)
    }

    func testSessionInfoNilStartTime() {
        let session = SessionInfo(
            id: "s2", sessionId: "nil", filePath: "/dev/null",
            startTime: nil, startTimeString: ""
        )
        XCTAssertNil(session.startTime)
    }

    // MARK: - EventType summaryText

    func testSummaryTextCommand() {
        let event = EventType.command(command: "git", args: ["clone", "https://github.com/repo.git"], exitCode: 0)
        XCTAssertEqual(event.summaryText, "git clone https://github.com/repo.git")
    }

    func testSummaryTextCommandNoArgs() {
        let event = EventType.command(command: "ls", args: [], exitCode: nil)
        XCTAssertEqual(event.summaryText, "ls")
    }

    func testSummaryTextFileAccess() {
        let event = EventType.fileAccess(path: "/Users/test/.env", action: .read)
        XCTAssertEqual(event.summaryText, "read: /Users/test/.env")
    }

    func testSummaryTextNetwork() {
        let event = EventType.network(host: "api.anthropic.com", port: 443, protocol: "tcp")
        XCTAssertEqual(event.summaryText, "tcp://api.anthropic.com:443")
    }

    func testSummaryTextProcess() {
        let event = EventType.process(pid: 5678, ppid: 1, action: .start)
        XCTAssertEqual(event.summaryText, "start (PID: 5678)")
    }

    func testSummaryTextSession() {
        let event = EventType.session(action: .end)
        XCTAssertEqual(event.summaryText, "Session end")
    }

    func testSummaryTextTruncationAt80Chars() {
        let longArgs = (0..<20).map { "arg\($0)" }
        let event = EventType.command(command: "longcommand", args: longArgs, exitCode: 0)
        let summary = event.summaryText
        XCTAssertLessThanOrEqual(summary.count, 80)
        XCTAssertTrue(summary.hasSuffix("…"))
    }

    func testSummaryTextNoTruncationUnder80() {
        let event = EventType.command(command: "ls", args: ["-la"], exitCode: 0)
        let summary = event.summaryText
        XCTAssertEqual(summary, "ls -la")
        XCTAssertFalse(summary.hasSuffix("…"))
    }

    func testSummaryTextExactly80CharsNotTruncated() {
        // "read: /" (7 chars) + 73 "a"s = 80 chars total
        let path = "/" + String(repeating: "a", count: 73)
        let event = EventType.fileAccess(path: path, action: .read)
        let expected = "read: " + path
        XCTAssertEqual(expected.count, 80)
        let summary = event.summaryText
        XCTAssertEqual(summary.count, 80)
        XCTAssertFalse(summary.hasSuffix("…"))
        XCTAssertEqual(summary, expected)
    }

    // MARK: - EventType typeTag

    func testTypeTagCommand() {
        let event = EventType.command(command: "ls", args: [], exitCode: 0)
        XCTAssertEqual(event.typeTag, "[CMD]")
    }

    func testTypeTagFileAccess() {
        let event = EventType.fileAccess(path: "/tmp", action: .read)
        XCTAssertEqual(event.typeTag, "[FILE]")
    }

    func testTypeTagNetwork() {
        let event = EventType.network(host: "example.com", port: 80, protocol: "tcp")
        XCTAssertEqual(event.typeTag, "[NET]")
    }

    func testTypeTagProcess() {
        let event = EventType.process(pid: 1, ppid: nil, action: .fork)
        XCTAssertEqual(event.typeTag, "[PROC]")
    }

    func testTypeTagSession() {
        let event = EventType.session(action: .start)
        XCTAssertEqual(event.typeTag, "[SES]")
    }
}
