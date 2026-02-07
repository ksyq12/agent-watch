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
        XCTAssertFalse(viewModel.events.isEmpty, "Events should be loaded from mock data")
        XCTAssertFalse(viewModel.sessions.isEmpty, "Sessions should be loaded")
        XCTAssertFalse(viewModel.version.isEmpty, "Version should be set")
        XCTAssertEqual(viewModel.version, "0.3.0-mock")
    }

    func testInitialStateIsNotMonitoring() {
        XCTAssertFalse(viewModel.isMonitoring)
    }

    func testInitialStateNoSelectedSession() {
        XCTAssertNil(viewModel.selectedSession)
    }

    func testInitialStateFilterIsNil() {
        XCTAssertNil(viewModel.filterRiskLevel)
    }

    func testInitialStateActivitySummaryPopulated() {
        XCTAssertGreaterThan(viewModel.activitySummary.totalEvents, 0)
    }

    func testInitialStateRecentAlertsPopulated() {
        XCTAssertFalse(viewModel.recentAlerts.isEmpty)
        for alert in viewModel.recentAlerts {
            XCTAssertTrue(alert.alert, "All recentAlerts entries should have alert == true")
        }
    }

    func testInitialStateRecentAlertsMaxFive() {
        XCTAssertLessThanOrEqual(viewModel.recentAlerts.count, 5)
    }

    // MARK: - startMonitoring / stopMonitoring

    func testStartMonitoring() {
        XCTAssertFalse(viewModel.isMonitoring)
        viewModel.startMonitoring()
        XCTAssertTrue(viewModel.isMonitoring)
    }

    func testStopMonitoring() {
        viewModel.startMonitoring()
        XCTAssertTrue(viewModel.isMonitoring)
        viewModel.stopMonitoring()
        XCTAssertFalse(viewModel.isMonitoring)
    }

    func testStartStopToggle() {
        XCTAssertFalse(viewModel.isMonitoring)
        viewModel.startMonitoring()
        XCTAssertTrue(viewModel.isMonitoring)
        viewModel.stopMonitoring()
        XCTAssertFalse(viewModel.isMonitoring)
        viewModel.startMonitoring()
        XCTAssertTrue(viewModel.isMonitoring)
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
        viewModel.filterRiskLevel = .low
        for event in viewModel.filteredEvents {
            XCTAssertEqual(event.riskLevel, .low)
        }
    }

    func testFilteredEventsFiltersCritical() {
        viewModel.filterRiskLevel = .critical
        let filtered = viewModel.filteredEvents
        for event in filtered {
            XCTAssertEqual(event.riskLevel, .critical)
        }
        XCTAssertFalse(filtered.isEmpty, "Mock data should contain critical events")
    }

    func testFilteredEventsSumMatchesTotal() {
        let lowCount = countEvents(at: .low)
        let medCount = countEvents(at: .medium)
        let highCount = countEvents(at: .high)
        let critCount = countEvents(at: .critical)
        XCTAssertEqual(lowCount + medCount + highCount + critCount, viewModel.events.count)
    }

    func testFilteredEventsResetToNil() {
        viewModel.filterRiskLevel = .critical
        let filteredCount = viewModel.filteredEvents.count
        XCTAssertLessThan(filteredCount, viewModel.events.count)

        viewModel.filterRiskLevel = nil
        XCTAssertEqual(viewModel.filteredEvents.count, viewModel.events.count)
    }

    // MARK: - loadSession

    func testLoadSessionSetsSelectedSession() {
        let session = viewModel.sessions[0]
        viewModel.loadSession(session)
        XCTAssertEqual(viewModel.selectedSession?.id, session.id)
    }

    func testLoadSessionLoadsEvents() {
        let session = viewModel.sessions[0]
        viewModel.loadSession(session)
        XCTAssertFalse(viewModel.events.isEmpty)
    }

    func testLoadSessionUpdatesActivitySummary() {
        let session = viewModel.sessions[0]
        viewModel.loadSession(session)
        XCTAssertEqual(viewModel.activitySummary.totalEvents, viewModel.events.count)
    }

    func testLoadSessionUpdatesRecentAlerts() {
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

    // MARK: - Helpers

    private func countEvents(at level: RiskLevel) -> Int {
        viewModel.filterRiskLevel = level
        let count = viewModel.filteredEvents.count
        viewModel.filterRiskLevel = nil
        return count
    }
}
