import AppKit
import Foundation
import UserNotifications

@MainActor
class NotificationManager: NSObject, UNUserNotificationCenterDelegate {
    static let shared = NotificationManager()

    private(set) var isAuthorized = false
    var minRiskLevel: RiskLevel = .high
    var soundEnabled = true
    var badgeEnabled = true
    var notificationsEnabled = true

    private override init() {
        super.init()
        UNUserNotificationCenter.current().delegate = self
    }

    // MARK: - Authorization

    func requestAuthorization() async {
        do {
            let options: UNAuthorizationOptions = [.alert, .sound, .badge]
            isAuthorized = try await UNUserNotificationCenter.current().requestAuthorization(options: options)
            if isAuthorized {
                await registerCategories()
            }
        } catch {
            print("[NotificationManager] Authorization failed: \(error)")
        }
    }

    // MARK: - Categories

    private func registerCategories() async {
        let viewAction = UNNotificationAction(
            identifier: "VIEW_DETAILS",
            title: NSLocalizedString("notification.action.view", comment: ""),
            options: .foreground
        )
        let dismissAction = UNNotificationAction(
            identifier: "DISMISS",
            title: NSLocalizedString("notification.action.dismiss", comment: ""),
            options: .destructive
        )

        let criticalCategory = UNNotificationCategory(
            identifier: "CRITICAL_ALERT",
            actions: [viewAction, dismissAction],
            intentIdentifiers: []
        )
        let highCategory = UNNotificationCategory(
            identifier: "HIGH_ALERT",
            actions: [viewAction, dismissAction],
            intentIdentifiers: []
        )

        UNUserNotificationCenter.current().setNotificationCategories([criticalCategory, highCategory])
    }

    // MARK: - Send Notification

    func sendNotification(for event: MonitoringEvent) {
        guard notificationsEnabled, isAuthorized else { return }
        guard event.riskLevel >= minRiskLevel else { return }

        let content = UNMutableNotificationContent()
        content.title = notificationTitle(for: event)
        content.body = notificationBody(for: event)
        content.categoryIdentifier = event.riskLevel == .critical ? "CRITICAL_ALERT" : "HIGH_ALERT"
        if soundEnabled {
            content.sound = event.riskLevel == .critical ? .defaultCritical : .default
        }

        let request = UNNotificationRequest(
            identifier: event.id,
            content: content,
            trigger: nil
        )

        UNUserNotificationCenter.current().add(request)
    }

    private func notificationTitle(for event: MonitoringEvent) -> String {
        switch event.riskLevel {
        case .critical:
            return NSLocalizedString("notification.title.critical", comment: "")
        case .high:
            return NSLocalizedString("notification.title.high", comment: "")
        default:
            return NSLocalizedString("notification.title.medium", comment: "")
        }
    }

    private func notificationBody(for event: MonitoringEvent) -> String {
        return "\(event.process): \(event.eventType.description)"
    }

    // MARK: - UNUserNotificationCenterDelegate

    nonisolated func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification
    ) async -> UNNotificationPresentationOptions {
        return [.banner, .sound, .badge]
    }

    nonisolated func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse
    ) async {
        if response.actionIdentifier == "VIEW_DETAILS" {
            await MainActor.run {
                NSApp.activate(ignoringOtherApps: true)
                if let window = NSApp.windows.first(where: { $0.identifier?.rawValue == "dashboard" }) {
                    window.makeKeyAndOrderFront(nil)
                }
            }
        }
    }
}
