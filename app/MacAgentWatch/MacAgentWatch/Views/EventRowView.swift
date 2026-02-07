import SwiftUI

struct EventRowView: View {
    let event: MonitoringEvent
    @Environment(\.colorSchemeContrast) private var contrast

    var body: some View {
        HStack(spacing: 10) {
            riskIndicator
            eventIcon
            eventContent
            Spacer()
            trailingInfo
        }
        .padding(.vertical, 4)
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(event.riskLevel.label) risk, \(event.eventType.description), \(event.process), PID \(event.pid)")
    }

    // MARK: - Risk Indicator

    private var riskIndicator: some View {
        Image(systemName: event.riskLevel.icon)
            .font(.caption)
            .foregroundStyle(contrast == .increased ? riskColorHighContrast : riskColor)
            .frame(width: 20)
            .accessibilityLabel(String(format: NSLocalizedString("a11y.risk.indicator", comment: ""), event.riskLevel.label))
    }

    // MARK: - Event Icon

    private var eventIcon: some View {
        Image(systemName: event.eventType.icon)
            .font(.body)
            .foregroundStyle(.secondary)
            .frame(width: 24)
    }

    // MARK: - Content

    private var eventContent: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(event.eventType.description)
                .font(.system(.body, design: .monospaced))
                .lineLimit(1)

            HStack(spacing: 8) {
                Label(event.process, systemImage: "gearshape")
                Label(String(format: NSLocalizedString("event.pid", comment: ""), event.pid), systemImage: "number")
                HStack(spacing: 2) {
                    Image(systemName: "clock")
                    Text(event.timestamp, style: .relative)
                }
            }
            .font(.caption)
            .foregroundStyle(.secondary)
        }
    }

    // MARK: - Trailing Info

    @ViewBuilder
    private var trailingInfo: some View {
        let effectiveColor = contrast == .increased ? riskColorHighContrast : riskColor
        HStack(spacing: 6) {
            if event.alert {
                Image(systemName: "bell.badge.fill")
                    .font(.caption)
                    .foregroundStyle(.red)
                    .symbolEffect(.pulse, options: .repeating)
                    .accessibilityLabel(String(localized: "a11y.alert.badge"))
                    .accessibilityRemoveTraits(.isImage)
            }

            Image(systemName: event.riskLevel.icon)
                .font(.caption)
                .foregroundStyle(effectiveColor)

            Text(event.riskLevel.label)
                .font(.caption.weight(.medium))
                .foregroundStyle(effectiveColor)
                .padding(.horizontal, 6)
                .padding(.vertical, 2)
                .background(effectiveColor.opacity(0.12), in: Capsule())
        }
    }

    // MARK: - Helpers

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
