import SwiftUI

// MARK: - Theme Preference

enum AppThemeMode: String, CaseIterable {
    case system
    case light
    case dark

    var colorScheme: ColorScheme? {
        switch self {
        case .system: return nil
        case .light: return .light
        case .dark: return .dark
        }
    }

    var localizedName: String {
        switch self {
        case .system: return NSLocalizedString("settings.appearance.system", comment: "")
        case .light: return NSLocalizedString("settings.appearance.light", comment: "")
        case .dark: return NSLocalizedString("settings.appearance.dark", comment: "")
        }
    }
}

// MARK: - Semantic Colors

enum AppColors {
    // Risk level colors that work in both light and dark mode
    static func riskColor(_ level: RiskLevel) -> Color {
        switch level {
        case .critical: return .red
        case .high: return .orange
        case .medium: return .yellow
        case .low: return .green
        }
    }

    // High contrast risk colors
    static func riskColorHighContrast(_ level: RiskLevel) -> Color {
        switch level {
        case .critical: return .red
        case .high: return .red
        case .medium: return .orange
        case .low: return .green
        }
    }

    // Event type colors
    static func eventTypeColor(_ type: EventType) -> Color {
        switch type {
        case .command: return .blue
        case .fileAccess: return .purple
        case .network: return .cyan
        case .process: return .indigo
        case .session: return .gray
        }
    }

    // UI semantic colors
    static let cardBackground = Color(.controlBackgroundColor)
    static let sidebarBackground = Color(.windowBackgroundColor)
    static let divider = Color(.separatorColor)
    static let secondaryLabel = Color(.secondaryLabelColor)
    static let tertiaryLabel = Color(.tertiaryLabelColor)
}
