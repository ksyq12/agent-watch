import Foundation
import SwiftUI

@Observable
@MainActor
final class MonitoringViewModel {
    static let maxEvents = 1000

    var events: [MonitoringEvent] = []
    var recentAlerts: [MonitoringEvent] = []
    var activitySummary: ActivitySummary = .empty
    var sessions: [SessionInfo] = []
    var selectedSession: SessionInfo?
    var isMonitoring: Bool = false
    var config: AppConfig = AppConfig()
    var version: String = ""
    var filterRiskLevel: RiskLevel? = nil

    private let bridge = CoreBridge.shared

    init() {
        loadInitialData()
    }

    func loadInitialData() {
        version = bridge.getVersion()
        config = bridge.loadConfig()
        sessions = bridge.listSessionLogs()
        events = CoreBridge.mockEvents
        activitySummary = bridge.getActivitySummary(events: events)
        recentAlerts = events.filter { $0.alert }.prefix(5).map { $0 }
    }

    func startMonitoring() {
        isMonitoring = true
        // TODO: Real FFI monitoring engine start
    }

    func stopMonitoring() {
        isMonitoring = false
        // TODO: Real FFI monitoring engine stop
    }

    func loadSession(_ session: SessionInfo) {
        selectedSession = session
        Task {
            let bridge = self.bridge
            let filePath = session.filePath
            let loadedEvents = await Task.detached { @MainActor in
                bridge.readSessionLog(path: filePath)
            }.value
            self.events = loadedEvents
            self.activitySummary = bridge.getActivitySummary(events: self.events)
            self.recentAlerts = self.events.filter { $0.alert }.prefix(5).map { $0 }
        }
    }

    func analyzeCommand(_ command: String, args: [String]) {
        let event = bridge.analyzeCommand(command: command, args: args)
        events.insert(event, at: 0)
        trimEvents()
        activitySummary = bridge.getActivitySummary(events: events)
        if event.alert {
            recentAlerts.insert(event, at: 0)
            if recentAlerts.count > 5 { recentAlerts.removeLast() }
        }
    }

    private func trimEvents() {
        if events.count > Self.maxEvents {
            events.removeLast(events.count - Self.maxEvents)
        }
    }

    var filteredEvents: [MonitoringEvent] {
        guard let filter = filterRiskLevel else { return events }
        return events.filter { $0.riskLevel == filter }
    }
}
