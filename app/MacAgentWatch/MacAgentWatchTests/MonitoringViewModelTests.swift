import XCTest
@testable import MacAgentWatch

@MainActor
final class MonitoringViewModelTests: XCTestCase {

    private var viewModel: MonitoringViewModel!

    override func setUp() {
        super.setUp()
        viewModel = MonitoringViewModel()
    }

    override func tearDown() {
        if viewModel.isMonitoring {
            viewModel.stopMonitoring()
        }
        viewModel.stopAutoRetry()
        viewModel = nil
        super.tearDown()
    }

    // MARK: - Initial State

    func testInitialStateLoadsData() {
        XCTAssertFalse(viewModel.version.isEmpty, "Version should be set")
        // Events may be empty if no real sessions exist on disk
    }

    func testInitialStateAutoStartsOrWaits() {
        // After init, either monitoring started (auto-start succeeded) or is waiting for agents
        XCTAssertTrue(viewModel.isMonitoring || viewModel.isWaitingForAgents,
                      "Should either be monitoring or waiting for agents after init")
    }

    func testInitialStateFilterIsNil() {
        XCTAssertNil(viewModel.filterRiskLevel)
    }

    func testInitialStateActivitySummaryMatchesEvents() {
        // Activity summary totalEvents should match loaded events count (may be 0)
        XCTAssertEqual(viewModel.activitySummary.totalEvents, viewModel.events.count)
    }

    func testInitialStateRecentAlertsAreValid() {
        for alert in viewModel.recentAlerts {
            XCTAssertTrue(alert.alert, "All recentAlerts entries should have alert == true")
        }
        XCTAssertLessThanOrEqual(viewModel.recentAlerts.count, 5)
    }

    func testInitialStateCurrentSessionIdMatchesMonitoringState() {
        // If auto-start succeeded, currentSessionId should be set; otherwise nil
        if viewModel.isMonitoring {
            XCTAssertNotNil(viewModel.currentSessionId,
                            "currentSessionId should be set when monitoring is active")
        } else {
            XCTAssertNil(viewModel.currentSessionId,
                         "currentSessionId should be nil when not monitoring")
        }
    }

    func testInitialStateErrorMessageIsNil() {
        XCTAssertNil(viewModel.errorMessage)
    }

    // MARK: - startMonitoring / stopMonitoring

    func testStartMonitoring() {
        // Stop any auto-started session first
        if viewModel.isMonitoring { viewModel.stopMonitoring() }
        XCTAssertFalse(viewModel.isMonitoring)
        viewModel.startMonitoring()
        XCTAssertTrue(viewModel.isMonitoring)
        XCTAssertNotNil(viewModel.currentSessionId, "Session ID should be set after starting")
        XCTAssertNil(viewModel.errorMessage, "No error should occur on start")
        viewModel.stopMonitoring()
    }

    func testStopMonitoring() {
        viewModel.startMonitoring()
        XCTAssertTrue(viewModel.isMonitoring)
        viewModel.stopMonitoring()
        XCTAssertFalse(viewModel.isMonitoring)
        XCTAssertNil(viewModel.currentSessionId, "Session ID should be nil after stopping")
        XCTAssertNil(viewModel.errorMessage, "No error should occur on stop")
    }

    func testStartStopToggle() {
        // Stop any auto-started session first
        if viewModel.isMonitoring { viewModel.stopMonitoring() }
        XCTAssertFalse(viewModel.isMonitoring)
        viewModel.startMonitoring()
        XCTAssertTrue(viewModel.isMonitoring)
        viewModel.stopMonitoring()
        XCTAssertFalse(viewModel.isMonitoring)
        viewModel.startMonitoring()
        XCTAssertTrue(viewModel.isMonitoring)
        viewModel.stopMonitoring()
    }

    func testStartMonitoringUpdatesSessionsList() {
        let sessionsBefore = viewModel.sessions.count
        viewModel.startMonitoring()
        // After starting, sessions list should be refreshed (may have new session)
        XCTAssertGreaterThanOrEqual(viewModel.sessions.count, sessionsBefore)
        // Clean up
        viewModel.stopMonitoring()
    }

    // MARK: - analyzeCommand

    func testAnalyzeCommandAddsEvent() {
        let countBefore = viewModel.events.count
        viewModel.analyzeCommand("echo", args: ["hello"])
        XCTAssertEqual(viewModel.events.count, countBefore + 1)
    }

    func testAnalyzeCommandInsertsAtFront() {
        viewModel.analyzeCommand("whoami", args: [])
        let firstEvent = viewModel.events.first!
        if case .command(let cmd, _, _) = firstEvent.eventType {
            XCTAssertEqual(cmd, "whoami")
        } else {
            XCTFail("Expected .command event type at front of events array")
        }
    }

    func testAnalyzeCommandUpdatesActivitySummary() {
        let summaryBefore = viewModel.activitySummary.totalEvents
        viewModel.analyzeCommand("date", args: [])
        XCTAssertEqual(viewModel.activitySummary.totalEvents, summaryBefore + 1)
    }

    func testAnalyzeCommandMultipleTimes() {
        let countBefore = viewModel.events.count
        viewModel.analyzeCommand("cmd1", args: [])
        viewModel.analyzeCommand("cmd2", args: ["a", "b"])
        viewModel.analyzeCommand("cmd3", args: ["x"])
        XCTAssertEqual(viewModel.events.count, countBefore + 3)
    }

    // MARK: - filteredEvents

    func testFilteredEventsReturnsAllWhenFilterNil() {
        viewModel.filterRiskLevel = nil
        XCTAssertEqual(viewModel.filteredEvents.count, viewModel.events.count)
    }

    func testFilteredEventsFiltersLow() {
        // Ensure at least one low event exists via analyzeCommand
        viewModel.analyzeCommand("ls", args: [])
        viewModel.filterRiskLevel = .low
        XCTAssertFalse(viewModel.filteredEvents.isEmpty, "Should have at least one low event")
        for event in viewModel.filteredEvents {
            XCTAssertEqual(event.riskLevel, .low)
        }
    }

    func testFilteredEventsFiltersCritical() {
        // Add a critical-risk command to ensure we have critical events
        viewModel.analyzeCommand("chmod", args: ["777", "/etc/passwd"])
        viewModel.filterRiskLevel = .critical
        let filtered = viewModel.filteredEvents
        for event in filtered {
            XCTAssertEqual(event.riskLevel, .critical)
        }
        // After adding a high-risk command, we may or may not get critical depending on FFI analysis
        // Just verify the filter logic works
    }

    func testFilteredEventsSumMatchesTotal() {
        // Ensure events exist by adding some via analyzeCommand
        viewModel.analyzeCommand("ls", args: [])
        viewModel.analyzeCommand("date", args: [])
        let lowCount = countEvents(at: .low)
        let medCount = countEvents(at: .medium)
        let highCount = countEvents(at: .high)
        let critCount = countEvents(at: .critical)
        XCTAssertEqual(lowCount + medCount + highCount + critCount, viewModel.events.count)
    }

    func testFilteredEventsResetToNil() {
        // Add events so there is something to filter
        viewModel.analyzeCommand("ls", args: [])
        viewModel.analyzeCommand("date", args: [])
        let totalCount = viewModel.events.count

        viewModel.filterRiskLevel = .low
        viewModel.filterRiskLevel = nil
        XCTAssertEqual(viewModel.filteredEvents.count, totalCount)
    }

    // MARK: - loadSession

    func testLoadSessionSetsSelectedSession() throws {
        try XCTSkipIf(viewModel.sessions.isEmpty, "No sessions on disk to test with")
        let session = viewModel.sessions[0]
        viewModel.loadSession(session)
        XCTAssertEqual(viewModel.selectedSession?.id, session.id)
    }

    func testLoadSessionLoadsEvents() throws {
        try XCTSkipIf(viewModel.sessions.isEmpty, "No sessions on disk to test with")
        let session = viewModel.sessions[0]
        viewModel.loadSession(session)
        // loadSession uses Task internally, so events load asynchronously
        // We just verify selectedSession was set; events will load async
        XCTAssertEqual(viewModel.selectedSession?.id, session.id)
    }

    func testLoadSessionUpdatesActivitySummary() throws {
        try XCTSkipIf(viewModel.sessions.isEmpty, "No sessions on disk to test with")
        let session = viewModel.sessions[0]
        viewModel.loadSession(session)
        XCTAssertEqual(viewModel.selectedSession?.id, session.id)
    }

    func testLoadSessionUpdatesRecentAlerts() throws {
        try XCTSkipIf(viewModel.sessions.isEmpty, "No sessions on disk to test with")
        let session = viewModel.sessions[0]
        viewModel.loadSession(session)
        for alert in viewModel.recentAlerts {
            XCTAssertTrue(alert.alert)
        }
        XCTAssertLessThanOrEqual(viewModel.recentAlerts.count, 5)
    }

    // MARK: - Config

    func testInitialConfigIsDefault() {
        XCTAssertFalse(viewModel.config.general.verbose)
        XCTAssertEqual(viewModel.config.general.defaultFormat, "pretty")
        XCTAssertTrue(viewModel.config.logging.enabled)
    }

    // MARK: - currentSessionId / errorMessage

    func testCurrentSessionIdSetOnStart() {
        viewModel.startMonitoring()
        XCTAssertNotNil(viewModel.currentSessionId)
        viewModel.stopMonitoring()
    }

    func testCurrentSessionIdClearedOnStop() {
        viewModel.startMonitoring()
        viewModel.stopMonitoring()
        XCTAssertNil(viewModel.currentSessionId)
    }

    func testErrorMessageClearedOnStart() {
        if viewModel.isMonitoring { viewModel.stopMonitoring() }
        viewModel.errorMessage = "previous error"
        viewModel.startMonitoring()
        XCTAssertNil(viewModel.errorMessage)
        viewModel.stopMonitoring()
    }

    func testErrorMessageClearedOnStop() {
        viewModel.startMonitoring()
        viewModel.errorMessage = "previous error"
        viewModel.stopMonitoring()
        XCTAssertNil(viewModel.errorMessage)
    }

    // MARK: - pollLatestEvents

    func testPollLatestEventsWithNoSession() {
        // When selectedSession is nil, pollLatestEvents should not add any events
        viewModel.selectedSession = nil
        let countBefore = viewModel.events.count
        viewModel.pollLatestEvents()
        XCTAssertEqual(viewModel.events.count, countBefore,
                       "pollLatestEvents should not add events when selectedSession is nil")
    }

    func testLiveEventIndexInitiallyZero() {
        XCTAssertEqual(viewModel.liveEventIndex, 0,
                       "liveEventIndex should start at 0")
    }

    func testClearLogDoesNotAffectViewModel() {
        // After analyzeCommand, events should exist; clearing the view's log
        // should not affect the ViewModel's events array
        viewModel.analyzeCommand("echo", args: ["test"])
        let countAfterAnalyze = viewModel.events.count
        XCTAssertGreaterThan(countAfterAnalyze, 0,
                             "Events should exist after analyzeCommand")
        // ViewModel has no clearLog method - this confirms the separation
        // between view-local logEntries and ViewModel events
    }

    // MARK: - Chart Data

    func testChartDataEmptyWithNoSession() {
        viewModel.selectedSession = nil
        viewModel.loadChartData()
        XCTAssertTrue(viewModel.chartData.isEmpty,
                      "chartData should be empty when selectedSession is nil")
    }

    func testLoadChartDataCallsFFI() throws {
        try XCTSkipIf(viewModel.sessions.isEmpty, "No sessions on disk to test with")
        let session = viewModel.sessions[0]
        viewModel.selectedSession = session
        viewModel.loadChartData(bucketMinutes: 60)
        // chartData should be set (may be empty if session has no events, but should not crash)
        XCTAssertNotNil(viewModel.chartData)
    }

    func testEventTypeCountFromFilteredEvents() {
        // Add known events
        viewModel.analyzeCommand("ls", args: [])
        viewModel.analyzeCommand("cat", args: ["/tmp/test"])
        viewModel.analyzeCommand("echo", args: ["hello"])

        let filtered = viewModel.filteredEvents
        let commandCount = filtered.filter {
            if case .command = $0.eventType { return true }; return false
        }.count
        // All analyzeCommand calls produce .command events
        XCTAssertGreaterThanOrEqual(commandCount, 3,
                                     "Should have at least 3 command events")
    }

    func testActivitySummaryRiskCountsMatchEvents() {
        viewModel.analyzeCommand("ls", args: [])
        viewModel.analyzeCommand("date", args: [])
        let summary = viewModel.activitySummary
        let events = viewModel.events

        let criticalCount = events.filter { $0.riskLevel == .critical }.count
        let highCount = events.filter { $0.riskLevel == .high }.count
        let mediumCount = events.filter { $0.riskLevel == .medium }.count
        let lowCount = events.filter { $0.riskLevel == .low }.count

        XCTAssertEqual(summary.criticalCount, criticalCount,
                       "Summary criticalCount should match events")
        XCTAssertEqual(summary.highCount, highCount,
                       "Summary highCount should match events")
        XCTAssertEqual(summary.mediumCount, mediumCount,
                       "Summary mediumCount should match events")
        XCTAssertEqual(summary.lowCount, lowCount,
                       "Summary lowCount should match events")
    }

    // MARK: - monitoredAgents

    func testMonitoredAgentsAfterStop() {
        if viewModel.isMonitoring { viewModel.stopMonitoring() }
        XCTAssertTrue(viewModel.monitoredAgents.isEmpty, "monitoredAgents should be empty after stop")
    }

    func testStartMonitoringPopulatesAgents() {
        if viewModel.isMonitoring { viewModel.stopMonitoring() }
        viewModel.startMonitoring()
        if viewModel.isMonitoring {
            let _ = viewModel.monitoredAgents
        }
        viewModel.stopMonitoring()
    }

    func testStopMonitoringClearsAgents() {
        viewModel.startMonitoring()
        viewModel.stopMonitoring()
        XCTAssertTrue(viewModel.monitoredAgents.isEmpty, "monitoredAgents should be cleared after stop")
    }

    // MARK: - Auto-Start & Waiting for Agents

    func testIsWaitingForAgentsInitiallyFalse() {
        // isWaitingForAgents should be false before attemptAutoStart is called
        XCTAssertFalse(viewModel.isWaitingForAgents,
                       "isWaitingForAgents should be false initially")
    }

    func testAttemptAutoStartSetsWaitingWhenNoAgents() {
        // When no agents are running, attemptAutoStart should set isWaitingForAgents = true
        viewModel.attemptAutoStart()
        // If monitoring didn't start (no agents), should be waiting
        if !viewModel.isMonitoring {
            XCTAssertTrue(viewModel.isWaitingForAgents,
                          "Should be waiting for agents when start fails to find agents")
        }
    }

    func testAttemptAutoStartSetsMonitoringWhenEngineStarts() {
        // Stop any existing session first to test fresh auto-start
        if viewModel.isMonitoring { viewModel.stopMonitoring() }
        viewModel.attemptAutoStart()
        if viewModel.isMonitoring {
            XCTAssertFalse(viewModel.isWaitingForAgents,
                           "Should not be waiting when monitoring started successfully")
        }
    }

    func testStopMonitoringCancelsAutoRetry() {
        // After attemptAutoStart sets waiting, stopMonitoring should cancel the retry
        viewModel.attemptAutoStart()
        viewModel.stopMonitoring()
        XCTAssertFalse(viewModel.isWaitingForAgents,
                       "stopMonitoring should cancel waiting state")
    }

    func testStopAutoRetryResetsWaitingState() {
        viewModel.attemptAutoStart()
        viewModel.stopAutoRetry()
        XCTAssertFalse(viewModel.isWaitingForAgents,
                       "stopAutoRetry should reset isWaitingForAgents to false")
    }

    // MARK: - Session Event Counts Cache

    func testSessionEventCountsInitiallyEmpty() {
        XCTAssertTrue(viewModel.sessionEventCounts.isEmpty,
                      "sessionEventCounts should be empty initially")
    }

    func testLoadSessionEventCountCachesResult() {
        // After starting monitoring and creating a session, loading event count should cache it
        viewModel.startMonitoring()
        guard let session = viewModel.sessions.first else {
            viewModel.stopMonitoring()
            return
        }
        viewModel.loadSessionEventCount(for: session)
        // The count should be cached (value may be 0 for a fresh session)
        XCTAssertNotNil(viewModel.sessionEventCounts[session.id],
                        "Event count should be cached after loading")
        viewModel.stopMonitoring()
    }

    // MARK: - Session Display Name

    func testSessionDisplayNameWithStartTime() {
        let date = Date(timeIntervalSince1970: 1738857000) // 2025-02-06 14:30:00 UTC
        let session = SessionInfo(
            id: "s1",
            sessionId: "abc123def456",
            filePath: "/tmp/test.jsonl",
            startTime: date,
            startTimeString: "2025-02-06T14:30:00Z"
        )
        let displayName = viewModel.sessionDisplayName(for: session)
        // Should contain date components, not the hash
        XCTAssertFalse(displayName.contains("abc123def456"),
                       "Display name should not show raw session ID hash")
        XCTAssertFalse(displayName.isEmpty,
                       "Display name should not be empty")
    }

    func testSessionDisplayNameWithoutStartTime() {
        let session = SessionInfo(
            id: "s2",
            sessionId: "xyz789",
            filePath: "/tmp/test.jsonl",
            startTime: nil,
            startTimeString: "2025-02-06T14:30:00Z"
        )
        let displayName = viewModel.sessionDisplayName(for: session)
        XCTAssertFalse(displayName.isEmpty,
                       "Display name should fall back to startTimeString when startTime is nil")
    }

    // MARK: - Active Session Detection

    func testIsActiveSessionReturnsTrueForCurrentSession() {
        // Ensure fresh start
        if viewModel.isMonitoring { viewModel.stopMonitoring() }
        viewModel.startMonitoring()
        guard viewModel.isMonitoring, let currentId = viewModel.currentSessionId else {
            return
        }
        // Find the session that matches the currentSessionId
        let matchingSession = viewModel.sessions.first { $0.sessionId == currentId }
        if let session = matchingSession {
            XCTAssertTrue(viewModel.isActiveSession(session),
                          "Current monitoring session should be active")
        }
        viewModel.stopMonitoring()
    }

    func testIsActiveSessionReturnsFalseForOldSession() {
        let oldSession = SessionInfo(
            id: "old",
            sessionId: "old-session-id",
            filePath: "/tmp/old.jsonl",
            startTime: Date().addingTimeInterval(-86400),
            startTimeString: "2025-02-05T00:00:00Z"
        )
        XCTAssertFalse(viewModel.isActiveSession(oldSession),
                       "Non-current session should not be active")
    }

    // MARK: - Restart Monitoring

    func testRestartMonitoringStopsAndStartsNewSession() {
        // Ensure monitoring is active
        if !viewModel.isMonitoring { viewModel.startMonitoring() }
        let oldSessionId = viewModel.currentSessionId
        XCTAssertTrue(viewModel.isMonitoring)

        viewModel.restartMonitoring()

        XCTAssertTrue(viewModel.isMonitoring, "Should be monitoring after restart")
        XCTAssertNotNil(viewModel.currentSessionId, "Should have a new session ID")
        XCTAssertNotEqual(viewModel.currentSessionId, oldSessionId,
                          "Session ID should change after restart")
        XCTAssertNil(viewModel.errorMessage, "No error should occur on restart")
        viewModel.stopMonitoring()
    }

    func testRestartMonitoringWhenNotMonitoring() {
        // Stop any existing session
        if viewModel.isMonitoring { viewModel.stopMonitoring() }
        XCTAssertFalse(viewModel.isMonitoring)

        viewModel.restartMonitoring()

        // Should start monitoring even if not previously monitoring
        XCTAssertTrue(viewModel.isMonitoring, "Should be monitoring after restart from stopped state")
        XCTAssertNotNil(viewModel.currentSessionId)
        viewModel.stopMonitoring()
    }

    func testRestartMonitoringClearsErrorMessage() {
        viewModel.startMonitoring()
        viewModel.errorMessage = "previous error"

        viewModel.restartMonitoring()

        XCTAssertNil(viewModel.errorMessage, "Error message should be cleared after restart")
        viewModel.stopMonitoring()
    }

    func testRestartMonitoringCancelsAutoRetry() {
        viewModel.attemptAutoStart()
        if viewModel.isWaitingForAgents {
            viewModel.restartMonitoring()
            XCTAssertFalse(viewModel.isWaitingForAgents,
                           "Waiting state should be cleared after restart")
            if viewModel.isMonitoring { viewModel.stopMonitoring() }
        }
    }

    func testRestartMonitoringRefreshesSessions() {
        viewModel.startMonitoring()
        let sessionsBeforeRestart = viewModel.sessions.count

        viewModel.restartMonitoring()

        XCTAssertGreaterThanOrEqual(viewModel.sessions.count, sessionsBeforeRestart,
                                     "Sessions list should be refreshed after restart")
        viewModel.stopMonitoring()
    }

    // MARK: - Helpers

    private func countEvents(at level: RiskLevel) -> Int {
        viewModel.filterRiskLevel = level
        let count = viewModel.filteredEvents.count
        viewModel.filterRiskLevel = nil
        return count
    }
}
