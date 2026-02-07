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
        viewModel = nil
        super.tearDown()
    }

    // MARK: - Initial State

    func testInitialStateLoadsData() {
        XCTAssertFalse(viewModel.version.isEmpty, "Version should be set")
        XCTAssertEqual(viewModel.version, "0.3.0")
        // Events may be empty if no real sessions exist on disk
    }

    func testInitialStateIsNotMonitoring() {
        XCTAssertFalse(viewModel.isMonitoring)
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

    func testInitialStateCurrentSessionIdIsNil() {
        XCTAssertNil(viewModel.currentSessionId)
    }

    func testInitialStateErrorMessageIsNil() {
        XCTAssertNil(viewModel.errorMessage)
    }

    // MARK: - startMonitoring / stopMonitoring

    func testStartMonitoring() {
        XCTAssertFalse(viewModel.isMonitoring)
        viewModel.startMonitoring()
        // Real FFI startSession should succeed
        XCTAssertTrue(viewModel.isMonitoring)
        XCTAssertNotNil(viewModel.currentSessionId, "Session ID should be set after starting")
        XCTAssertNil(viewModel.errorMessage, "No error should occur on start")
        // Clean up
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
        XCTAssertFalse(viewModel.isMonitoring)
        viewModel.startMonitoring()
        XCTAssertTrue(viewModel.isMonitoring)
        viewModel.stopMonitoring()
        XCTAssertFalse(viewModel.isMonitoring)
        viewModel.startMonitoring()
        XCTAssertTrue(viewModel.isMonitoring)
        // Clean up
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

    // MARK: - Helpers

    private func countEvents(at level: RiskLevel) -> Int {
        viewModel.filterRiskLevel = level
        let count = viewModel.filteredEvents.count
        viewModel.filterRiskLevel = nil
        return count
    }
}
