import Foundation

enum RiskLevel: String, Codable, CaseIterable, Comparable {
    case low, medium, high, critical

    var color: String {
        switch self {
        case .low: return "green"
        case .medium: return "yellow"
        case .high: return "orange"
        case .critical: return "red"
        }
    }

    var icon: String {
        switch self {
        case .low: return "checkmark.circle.fill"
        case .medium: return "exclamationmark.triangle.fill"
        case .high: return "exclamationmark.octagon.fill"
        case .critical: return "xmark.octagon.fill"
        }
    }

    var label: String {
        switch self {
        case .low: return String(localized: "risk.low")
        case .medium: return String(localized: "risk.medium")
        case .high: return String(localized: "risk.high")
        case .critical: return String(localized: "risk.critical")
        }
    }

    private var sortOrder: Int {
        switch self {
        case .low: return 0
        case .medium: return 1
        case .high: return 2
        case .critical: return 3
        }
    }

    static func < (lhs: RiskLevel, rhs: RiskLevel) -> Bool {
        lhs.sortOrder < rhs.sortOrder
    }
}

enum FileAction: String, Codable {
    case read, write, delete, create, chmod
}

enum ProcessAction: String, Codable {
    case start, exit, fork
}

enum SessionAction: String, Codable {
    case start, end
}

enum EventType {
    case command(command: String, args: [String], exitCode: Int32?)
    case fileAccess(path: String, action: FileAction)
    case network(host: String, port: UInt16, protocol: String)
    case process(pid: UInt32, ppid: UInt32?, action: ProcessAction)
    case session(action: SessionAction)

    var icon: String {
        switch self {
        case .command: return "terminal"
        case .fileAccess: return "doc"
        case .network: return "network"
        case .process: return "gearshape.2"
        case .session: return "play.circle"
        }
    }

    var description: String {
        switch self {
        case .command(let cmd, let args, _):
            return args.isEmpty ? cmd : "\(cmd) \(args.joined(separator: " "))"
        case .fileAccess(let path, let action):
            return "\(action.rawValue): \(path)"
        case .network(let host, let port, let proto):
            return "\(proto)://\(host):\(port)"
        case .process(let pid, _, let action):
            return "\(action.rawValue) (PID: \(pid))"
        case .session(let action):
            return "Session \(action.rawValue)"
        }
    }

    var summaryText: String {
        let text = description
        if text.count <= 80 { return text }
        return String(text.prefix(79)) + "…"
    }

    var typeTag: String {
        switch self {
        case .command: return "[CMD]"
        case .fileAccess: return "[FILE]"
        case .network: return "[NET]"
        case .process: return "[PROC]"
        case .session: return "[SES]"
        }
    }

    var typeLabel: String {
        switch self {
        case .command: return String(localized: "event.type.command")
        case .fileAccess: return String(localized: "event.type.fileAccess")
        case .network: return String(localized: "event.type.network")
        case .process: return String(localized: "event.type.process")
        case .session: return String(localized: "event.type.session")
        }
    }
}

struct MonitoringEvent: Identifiable {
    let id: String
    let timestamp: Date
    let eventType: EventType
    let process: String
    let pid: UInt32
    let riskLevel: RiskLevel
    let alert: Bool
}

struct ActivitySummary {
    var totalEvents: Int = 0
    var criticalCount: Int = 0
    var highCount: Int = 0
    var mediumCount: Int = 0
    var lowCount: Int = 0

    static let empty = ActivitySummary()
}

struct SessionInfo: Identifiable, Hashable {
    let id: String
    let sessionId: String
    let filePath: String
    let startTime: Date?
    let startTimeString: String
    let agentName: String?
    let maxRiskLevel: RiskLevel
}

enum EventTypeFilter: String, CaseIterable {
    case all
    case command
    case fileAccess
    case network
    case process

    var label: String {
        switch self {
        case .all: return String(localized: "eventType.all")
        case .command: return String(localized: "eventType.command")
        case .fileAccess: return String(localized: "eventType.file")
        case .network: return String(localized: "eventType.network")
        case .process: return String(localized: "eventType.process")
        }
    }

    var icon: String {
        switch self {
        case .all: return "list.bullet"
        case .command: return "terminal"
        case .fileAccess: return "doc"
        case .network: return "network"
        case .process: return "gearshape"
        }
    }
}

enum DateRangePreset: String, CaseIterable {
    case allTime
    case today
    case lastHour
    case last24Hours
    case last7Days
    case custom

    var label: String {
        switch self {
        case .allTime: return String(localized: "dateRange.allTime")
        case .today: return String(localized: "dateRange.today")
        case .lastHour: return String(localized: "dateRange.lastHour")
        case .last24Hours: return String(localized: "dateRange.last24Hours")
        case .last7Days: return String(localized: "dateRange.last7Days")
        case .custom: return String(localized: "dateRange.custom")
        }
    }
}

// MARK: - Chart Data Types

struct ChartDataPoint: Identifiable {
    let id = UUID()
    let timestamp: Date
    let total: Int
    let critical: Int
    let high: Int
    let medium: Int
    let low: Int
}

struct RiskDistributionItem: Identifiable {
    let id = UUID()
    let level: String
    let count: Int
}

struct EventTypeCount: Identifiable {
    let id = UUID()
    let type: String
    let count: Int
}
