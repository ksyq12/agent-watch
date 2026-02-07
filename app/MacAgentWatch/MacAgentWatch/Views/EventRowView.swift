import SwiftUI

struct EventRowView: View {
    let event: MonitoringEvent
    @Environment(\.colorSchemeContrast) private var contrast
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @ScaledMetric(relativeTo: .body) private var rowSpacing: CGFloat = 10
    @ScaledMetric(relativeTo: .caption) private var indicatorWidth: CGFloat = 20
    @ScaledMetric(relativeTo: .body) private var iconWidth: CGFloat = 24

    var body: some View {
        HStack(spacing: rowSpacing) {
            riskIndicator
            eventIcon
            eventContent
            Spacer()
            trailingInfo
        }
        .padding(.vertical, 4)
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(event.riskLevel.label) risk, \(event.eventType.description), \(event.process), PID \(event.pid)")
        .accessibilityHint(String(localized: "a11y.event.row.hint"))
    }

    // MARK: - Risk Indicator

    private var riskIndicator: some View {
        Image(systemName: event.riskLevel.icon)
            .font(.caption)
            .foregroundStyle(contrast == .increased ? riskColorHighContrast : riskColor)
            .frame(width: indicatorWidth)
            .accessibilityLabel(String(format: NSLocalizedString("a11y.risk.indicator", comment: ""), event.riskLevel.label))
    }

    // MARK: - Event Icon

    private var eventIcon: some View {
        Image(systemName: event.eventType.icon)
            .font(.body)
            .foregroundStyle(.secondary)
            .frame(width: iconWidth)
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
                    .symbolEffect(.pulse, options: .repeating, isActive: !reduceMotion)
                    .accessibilityLabel(String(localized: "a11y.alert.badge"))
                    .accessibilityValue(String(localized: "a11y.alert.value"))
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
        AppColors.riskColor(event.riskLevel)
    }

    private var riskColorHighContrast: Color {
        AppColors.riskColorHighContrast(event.riskLevel)
    }
}
