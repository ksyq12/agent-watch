import SwiftUI

struct EventDetailView: View {
    let event: MonitoringEvent
    let onDismiss: () -> Void

    @Environment(\.colorSchemeContrast) private var contrast
    @ScaledMetric(relativeTo: .body) private var sectionSpacing: CGFloat = 16
    @ScaledMetric(relativeTo: .body) private var fieldSpacing: CGFloat = 8
    @State private var copiedField: String?

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: sectionSpacing) {
                header
                Divider()
                eventDetailsSection
                Divider()
                eventTypeSection
            }
            .padding()
        }
        .frame(minWidth: 300, idealWidth: 360)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(String(localized: "a11y.detail.panel"))
    }

    // MARK: - Header

    private var header: some View {
        HStack {
            Image(systemName: event.riskLevel.icon)
                .foregroundStyle(effectiveRiskColor)
            Text(event.riskLevel.label)
                .font(.headline)
                .foregroundStyle(effectiveRiskColor)
            Text(event.eventType.typeLabel)
                .font(.headline)
                .foregroundStyle(.primary)
            Spacer()
            Button {
                onDismiss()
            } label: {
                Image(systemName: "xmark.circle.fill")
                    .foregroundStyle(.secondary)
            }
            .buttonStyle(.plain)
            .accessibilityLabel(String(localized: "a11y.detail.close"))
        }
    }

    // MARK: - Event Details Section

    private var eventDetailsSection: some View {
        VStack(alignment: .leading, spacing: fieldSpacing) {
            Text(String(localized: "detail.section.event"))
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(.secondary)

            detailRow(
                label: String(localized: "detail.id"),
                value: event.id,
                copyable: true
            )
            detailRow(
                label: String(localized: "detail.time"),
                value: event.timestamp.formatted(date: .abbreviated, time: .standard)
            )
            detailRow(
                label: String(localized: "detail.process"),
                value: event.process
            )
            detailRow(
                label: String(localized: "detail.pid"),
                value: String(event.pid)
            )

            HStack(spacing: 4) {
                Text(String(localized: "detail.risk"))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .frame(width: 80, alignment: .trailing)
                Image(systemName: event.riskLevel.icon)
                    .font(.caption)
                    .foregroundStyle(effectiveRiskColor)
                Text(event.riskLevel.label)
                    .font(.body)
                    .foregroundStyle(effectiveRiskColor)
            }
            .accessibilityElement(children: .combine)

            HStack(spacing: 4) {
                Text(String(localized: "detail.alert"))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .frame(width: 80, alignment: .trailing)
                Image(systemName: event.alert ? "bell.badge.fill" : "bell.slash")
                    .font(.caption)
                    .foregroundStyle(event.alert ? .red : .secondary)
                Text(event.alert
                     ? String(localized: "detail.alert.yes")
                     : String(localized: "detail.alert.no"))
                    .font(.body)
            }
            .accessibilityElement(children: .combine)
        }
    }

    // MARK: - Event Type Section

    @ViewBuilder
    private var eventTypeSection: some View {
        switch event.eventType {
        case .command(let command, let args, let exitCode):
            commandSection(command: command, args: args, exitCode: exitCode)
        case .fileAccess(let path, let action):
            fileAccessSection(path: path, action: action)
        case .network(let host, let port, let proto):
            networkSection(host: host, port: port, proto: proto)
        case .process(let pid, let ppid, let action):
            processSection(pid: pid, ppid: ppid, action: action)
        case .session(let action):
            sessionSection(action: action)
        }
    }

    // MARK: - Command Details

    private func commandSection(command: String, args: [String], exitCode: Int32?) -> some View {
        VStack(alignment: .leading, spacing: fieldSpacing) {
            Text(String(localized: "detail.section.command"))
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(.secondary)

            detailRow(
                label: String(localized: "detail.command"),
                value: command,
                copyable: true
            )
            detailRow(
                label: String(localized: "detail.arguments"),
                value: args.isEmpty
                    ? String(localized: "detail.none")
                    : args.joined(separator: " "),
                copyable: !args.isEmpty
            )
            if let exitCode {
                detailRow(
                    label: String(localized: "detail.exitCode"),
                    value: String(exitCode)
                )
            }
        }
    }

    // MARK: - File Access Details

    private func fileAccessSection(path: String, action: FileAction) -> some View {
        VStack(alignment: .leading, spacing: fieldSpacing) {
            Text(String(localized: "detail.section.file"))
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(.secondary)

            detailRow(
                label: String(localized: "detail.path"),
                value: path,
                copyable: true
            )
            detailRow(
                label: String(localized: "detail.action"),
                value: action.rawValue.capitalized
            )
        }
    }

    // MARK: - Network Details

    private func networkSection(host: String, port: UInt16, proto: String) -> some View {
        VStack(alignment: .leading, spacing: fieldSpacing) {
            Text(String(localized: "detail.section.network"))
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(.secondary)

            detailRow(
                label: String(localized: "detail.host"),
                value: host,
                copyable: true
            )
            detailRow(
                label: String(localized: "detail.port"),
                value: String(port)
            )
            detailRow(
                label: String(localized: "detail.protocol"),
                value: proto
            )
        }
    }

    // MARK: - Process Details

    private func processSection(pid: UInt32, ppid: UInt32?, action: ProcessAction) -> some View {
        VStack(alignment: .leading, spacing: fieldSpacing) {
            Text(String(localized: "detail.section.process"))
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(.secondary)

            detailRow(
                label: String(localized: "detail.pid"),
                value: String(pid)
            )
            if let ppid {
                detailRow(
                    label: String(localized: "detail.ppid"),
                    value: String(ppid)
                )
            }
            detailRow(
                label: String(localized: "detail.action"),
                value: action.rawValue.capitalized
            )
        }
    }

    // MARK: - Session Details

    private func sessionSection(action: SessionAction) -> some View {
        VStack(alignment: .leading, spacing: fieldSpacing) {
            Text(String(localized: "detail.section.session"))
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(.secondary)

            detailRow(
                label: String(localized: "detail.action"),
                value: action.rawValue.capitalized
            )
        }
    }

    // MARK: - Detail Row Helper

    private func detailRow(label: String, value: String, copyable: Bool = false) -> some View {
        HStack(spacing: 4) {
            Text(label)
                .font(.caption)
                .foregroundStyle(.secondary)
                .frame(width: 80, alignment: .trailing)

            Text(value)
                .font(.system(.body, design: .monospaced))
                .textSelection(.enabled)
                .lineLimit(3)

            if copyable {
                Button {
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(value, forType: .string)
                    copiedField = label
                    DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
                        if copiedField == label { copiedField = nil }
                    }
                } label: {
                    Image(systemName: copiedField == label ? "checkmark" : "doc.on.doc")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                .buttonStyle(.plain)
                .accessibilityLabel(String(format: NSLocalizedString("a11y.detail.copy", comment: ""), label))
            }
        }
        .accessibilityElement(children: .combine)
    }

    // MARK: - Helpers

    private var effectiveRiskColor: Color {
        contrast == .increased ? riskColorHighContrast : riskColor
    }

    private var riskColor: Color {
        switch event.riskLevel {
        case .low: return .green
        case .medium: return .yellow
        case .high: return .orange
        case .critical: return .red
        }
    }

    private var riskColorHighContrast: Color {
        switch event.riskLevel {
        case .low: return .green
        case .medium: return .orange
        case .high: return .red
        case .critical: return .red
        }
    }
}

// MARK: - EventType Extension

extension EventType {
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
