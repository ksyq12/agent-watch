import SwiftUI

struct ActivityCardsView: View {
    let summary: ActivitySummary
    @Environment(\.colorSchemeContrast) private var contrast
    @ScaledMetric(relativeTo: .body) private var cardSpacing: CGFloat = 12
    @ScaledMetric(relativeTo: .body) private var cardVerticalPadding: CGFloat = 12
    @ScaledMetric(relativeTo: .body) private var cardCornerRadius: CGFloat = 10

    var body: some View {
        HStack(spacing: cardSpacing) {
            activityCard(
                title: String(localized: "summary.total.events"),
                count: summary.totalEvents,
                icon: "list.bullet",
                color: .blue
            )
            activityCard(
                title: String(localized: "summary.critical"),
                count: summary.criticalCount,
                icon: "xmark.octagon.fill",
                color: AppColors.riskColor(.critical)
            )
            activityCard(
                title: String(localized: "summary.high"),
                count: summary.highCount,
                icon: "exclamationmark.octagon.fill",
                color: AppColors.riskColor(.high)
            )
            activityCard(
                title: String(localized: "summary.medium"),
                count: summary.mediumCount,
                icon: "exclamationmark.triangle.fill",
                color: AppColors.riskColor(.medium)
            )
            activityCard(
                title: String(localized: "summary.low"),
                count: summary.lowCount,
                icon: "checkmark.circle.fill",
                color: AppColors.riskColor(.low)
            )
        }
    }

    private func activityCard(title: String, count: Int, icon: String, color: Color) -> some View {
        let fillOpacity = contrast == .increased ? 0.15 : 0.08
        let borderOpacity = contrast == .increased ? 0.3 : 0.15
        let borderWidth: CGFloat = contrast == .increased ? 2 : 1
        return VStack(spacing: 6) {
            HStack(spacing: 6) {
                Image(systemName: icon)
                    .foregroundStyle(color)
                Text(title)
                    .foregroundStyle(.secondary)
            }
            .font(.caption)

            Text("\(count)")
                .font(.system(.title, design: .rounded, weight: .bold))
                .monospacedDigit()
                .foregroundStyle(color)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, cardVerticalPadding)
        .background {
            RoundedRectangle(cornerRadius: cardCornerRadius, style: .continuous)
                .fill(color.opacity(fillOpacity))
                .overlay(
                    RoundedRectangle(cornerRadius: cardCornerRadius, style: .continuous)
                        .strokeBorder(color.opacity(borderOpacity), lineWidth: borderWidth)
                )
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel(String(format: NSLocalizedString("a11y.dashboard.card", comment: ""), title, count))
        .accessibilityHint(String(localized: "a11y.dashboard.card.hint"))
    }
}
