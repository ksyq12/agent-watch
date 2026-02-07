import Foundation

struct GeneralConfig {
    var verbose: Bool = false
    var defaultFormat: String = "pretty"
}

struct LoggingConfig {
    var enabled: Bool = true
    var logDir: String? = nil
    var retentionDays: UInt32 = 30
}

struct MonitoringConfig {
    var fsEnabled: Bool = false
    var netEnabled: Bool = false
    var trackChildren: Bool = true
    var trackingPollMs: UInt64 = 100
    var fsDebounceMs: UInt64 = 100
    var netPollMs: UInt64 = 500
    var watchPaths: [String] = []
    var sensitivePatterns: [String] = [".env", ".env.*", "*.pem", "*.key", "*credential*", "*secret*"]
    var networkWhitelist: [String] = ["api.anthropic.com", "github.com", "api.github.com"]
}

struct AlertConfig {
    var minLevel: String = "high"
    var customHighRisk: [String] = []
}

struct NotificationConfig {
    var enabled: Bool = true
    var minRiskLevel: RiskLevel = .high
    var soundEnabled: Bool = true
    var badgeEnabled: Bool = true
}

struct AppConfig {
    var general: GeneralConfig = GeneralConfig()
    var logging: LoggingConfig = LoggingConfig()
    var monitoring: MonitoringConfig = MonitoringConfig()
    var alerts: AlertConfig = AlertConfig()
    var notifications: NotificationConfig = NotificationConfig()
}
