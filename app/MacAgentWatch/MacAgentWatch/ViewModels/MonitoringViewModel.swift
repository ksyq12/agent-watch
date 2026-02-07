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
    var currentSessionId: String?
    var errorMessage: String?

    private let bridge = CoreBridge.shared

    init() {
        loadInitialData()
    }

    func loadInitialData() {
        version = bridge.getVersion()
        config = bridge.loadConfig()
        sessions = bridge.listSessionLogs()
        isMonitoring = bridge.isEngineActive()

        if let latestSession = sessions.first {
            events = bridge.readSessionLog(path: latestSession.filePath)
            selectedSession = latestSession
        } else {
            events = []
        }

        activitySummary = bridge.getActivitySummary(events: events)
        recentAlerts = events.filter { $0.alert }.prefix(5).map { $0 }
    }

    func startMonitoring() {
        errorMessage = nil
        if let sessionId = bridge.startSession(processName: "MacAgentWatch") {
            currentSessionId = sessionId
            isMonitoring = true
            sessions = bridge.listSessionLogs()
        } else {
            errorMessage = "Failed to start monitoring session"
        }
    }

    func stopMonitoring() {
        errorMessage = nil
        if bridge.stopSession() {
            isMonitoring = false
            currentSessionId = nil
            sessions = bridge.listSessionLogs()
        } else {
            errorMessage = "Failed to stop monitoring session"
        }
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
