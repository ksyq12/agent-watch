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
    var selectedEvent: MonitoringEvent?
    var isMonitoring: Bool = false
    var config: AppConfig = AppConfig()
    var version: String = ""
    var filterRiskLevel: RiskLevel? = nil
    var searchQuery: String = ""
    var eventTypeFilter: EventTypeFilter = .all
    var dateRangePreset: DateRangePreset = .allTime
    var customStartDate: Date? = nil
    var customEndDate: Date? = nil
    var currentSessionId: String?
    var errorMessage: String?
    var chartData: [ChartDataPoint] = []
    var monitoredAgents: [FfiDetectedAgent] = []
    var liveEventIndex: UInt32 = 0
    var selectedTab: DetailTab = .events
    var isSearchFocused: Bool = false

    var selectedTheme: AppThemeMode {
        get {
            let raw = UserDefaults.standard.string(forKey: "selectedTheme") ?? AppThemeMode.system.rawValue
            return AppThemeMode(rawValue: raw) ?? .system
        }
        set {
            UserDefaults.standard.set(newValue.rawValue, forKey: "selectedTheme")
        }
    }

    // MARK: - Settings Bindings

    var loggingEnabled: Bool {
        get { config.logging.enabled }
        set { config.logging.enabled = newValue }
    }

    var logRetentionDays: UInt32 {
        get { config.logging.retentionDays }
        set { config.logging.retentionDays = newValue }
    }

    var defaultFormat: String {
        get { config.general.defaultFormat }
        set { config.general.defaultFormat = newValue }
    }

    var fsEventsEnabled: Bool {
        get { config.monitoring.fsEnabled }
        set { config.monitoring.fsEnabled = newValue }
    }

    var networkMonitorEnabled: Bool {
        get { config.monitoring.netEnabled }
        set { config.monitoring.netEnabled = newValue }
    }

    var trackingPollMs: UInt64 {
        get { config.monitoring.trackingPollMs }
        set { config.monitoring.trackingPollMs = newValue }
    }

    var watchPaths: [String] {
        get { config.monitoring.watchPaths }
        set { config.monitoring.watchPaths = newValue }
    }

    var sensitivePatterns: [String] {
        get { config.monitoring.sensitivePatterns }
        set { config.monitoring.sensitivePatterns = newValue }
    }

    var networkWhitelist: [String] {
        get { config.monitoring.networkWhitelist }
        set { config.monitoring.networkWhitelist = newValue }
    }

    var notificationSoundEnabled: Bool {
        get { config.notifications.soundEnabled }
        set {
            config.notifications.soundEnabled = newValue
            notificationManager.soundEnabled = newValue
        }
    }

    var notificationBadgeEnabled: Bool {
        get { config.notifications.badgeEnabled }
        set {
            config.notifications.badgeEnabled = newValue
            notificationManager.badgeEnabled = newValue
        }
    }

    var notificationMinRiskLevel: RiskLevel {
        get { config.notifications.minRiskLevel }
        set {
            config.notifications.minRiskLevel = newValue
            notificationManager.minRiskLevel = newValue
        }
    }

    func saveSettings() {
        bridge.saveConfig(config)
        applyNotificationConfig()
    }

    private let bridge = CoreBridge.shared
    private let notificationManager = NotificationManager.shared

    init() {
        loadInitialData()
        Task {
            await notificationManager.requestAuthorization()
            applyNotificationConfig()
        }
    }

    var notificationsEnabled: Bool {
        get { notificationManager.notificationsEnabled }
        set {
            notificationManager.notificationsEnabled = newValue
            config.notifications.enabled = newValue
        }
    }

    private func applyNotificationConfig() {
        notificationManager.notificationsEnabled = config.notifications.enabled
        notificationManager.minRiskLevel = config.notifications.minRiskLevel
        notificationManager.soundEnabled = config.notifications.soundEnabled
        notificationManager.badgeEnabled = config.notifications.badgeEnabled
    }

    func loadInitialData() {
        version = bridge.getVersion()
        config = bridge.loadConfig()
        applyNotificationConfig()
        sessions = bridge.listSessionLogs()
        isMonitoring = bridge.isEngineActive()
        if isMonitoring {
            monitoredAgents = bridge.getMonitoredAgents()
        }

        if let latestSession = sessions.first {
            events = bridge.readSessionLog(path: latestSession.filePath)
            selectedSession = latestSession
        } else {
            events = []
        }

        activitySummary = bridge.getActivitySummary(events: events)
        recentAlerts = events.filter { $0.alert }.prefix(5).map { $0 }
        loadChartData()
    }

    func loadChartData(bucketMinutes: UInt32 = 60) {
        guard let session = selectedSession else {
            chartData = []
            return
        }
        chartData = bridge.getChartData(path: session.filePath, bucketMinutes: bucketMinutes)
    }

    func pollLatestEvents() {
        guard let session = selectedSession else { return }
        let newEvents = bridge.getLatestEvents(path: session.filePath, sinceIndex: liveEventIndex)
        if !newEvents.isEmpty {
            events.append(contentsOf: newEvents)
            liveEventIndex += UInt32(newEvents.count)
            trimEvents()
            activitySummary = bridge.getActivitySummary(events: events)
            recentAlerts = events.filter { $0.alert }.prefix(5).map { $0 }

            for event in newEvents {
                notificationManager.sendNotification(for: event)
            }
        }
    }

    func startMonitoring() {
        errorMessage = nil
        do {
            let sessionId = try bridge.startSession(processName: "MacAgentWatch")
            currentSessionId = sessionId
            isMonitoring = true
            monitoredAgents = bridge.getMonitoredAgents()
            sessions = bridge.listSessionLogs()
            // Load the new session
            if let newSession = sessions.first {
                loadSession(newSession)
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func stopMonitoring() {
        errorMessage = nil
        if bridge.stopSession() {
            isMonitoring = false
            currentSessionId = nil
            monitoredAgents = []
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
            self.liveEventIndex = UInt32(loadedEvents.count)
            self.activitySummary = bridge.getActivitySummary(events: self.events)
            self.recentAlerts = self.events.filter { $0.alert }.prefix(5).map { $0 }
            self.loadChartData()
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
        notificationManager.sendNotification(for: event)
    }

    private func trimEvents() {
        if events.count > Self.maxEvents {
            events.removeLast(events.count - Self.maxEvents)
        }
    }

    var filteredEvents: [MonitoringEvent] {
        var result = events

        // Risk level filter
        if let riskFilter = filterRiskLevel {
            result = result.filter { $0.riskLevel == riskFilter }
        }

        // Event type filter
        if eventTypeFilter != .all {
            result = result.filter { matchesEventType($0) }
        }

        // Text search
        if !searchQuery.isEmpty {
            result = result.filter { matchesSearch($0) }
        }

        // Date range
        result = result.filter { matchesDateRange($0) }

        return result
    }

    private func matchesEventType(_ event: MonitoringEvent) -> Bool {
        switch (eventTypeFilter, event.eventType) {
        case (.command, .command): return true
        case (.fileAccess, .fileAccess): return true
        case (.network, .network): return true
        case (.process, .process): return true
        default: return false
        }
    }

    private func matchesSearch(_ event: MonitoringEvent) -> Bool {
        let query = searchQuery.lowercased()
        if event.eventType.description.lowercased().contains(query) { return true }
        if event.process.lowercased().contains(query) { return true }
        if event.riskLevel.rawValue.lowercased().contains(query) { return true }
        return false
    }

    private func matchesDateRange(_ event: MonitoringEvent) -> Bool {
        let now = Date()
        switch dateRangePreset {
        case .allTime:
            return true
        case .today:
            return Calendar.current.isDateInToday(event.timestamp)
        case .lastHour:
            return event.timestamp >= now.addingTimeInterval(-3600)
        case .last24Hours:
            return event.timestamp >= now.addingTimeInterval(-86400)
        case .last7Days:
            return event.timestamp >= now.addingTimeInterval(-604800)
        case .custom:
            let afterStart = customStartDate.map { event.timestamp >= $0 } ?? true
            let beforeEnd = customEndDate.map { event.timestamp <= $0 } ?? true
            return afterStart && beforeEnd
        }
    }
}
