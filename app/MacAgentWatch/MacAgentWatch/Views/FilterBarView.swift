import SwiftUI

struct FilterBarView: View {
    @Binding var filterRiskLevel: RiskLevel?
    let filteredCount: Int
    @Environment(\.colorSchemeContrast) private var contrast
    @ScaledMetric(relativeTo: .caption) private var chipHorizontalPadding: CGFloat = 10
    @ScaledMetric(relativeTo: .caption) private var chipVerticalPadding: CGFloat = 4

    var body: some View {
        HStack(spacing: 8) {
            Text("filter.label")
                .font(.subheadline)
                .foregroundStyle(.secondary)

            filterChip(label: String(localized: "filter.all"), icon: nil, level: nil)
            ForEach(RiskLevel.allCases, id: \.self) { level in
                filterChip(label: level.label, icon: level.icon, level: level)
            }

            Spacer()

            Text(verbatim: String(format: NSLocalizedString("filter.events.count", comment: ""), filteredCount))
                .font(.caption)
                .foregroundStyle(.tertiary)
        }
    }

    private func filterChip(label: String, icon: String?, level: RiskLevel?) -> some View {
        let isSelected = filterRiskLevel == level
        return Button {
            withAnimation(.easeInOut(duration: 0.15)) {
                filterRiskLevel = level
            }
        } label: {
            HStack(spacing: 4) {
                if let icon {
                    Image(systemName: icon)
                        .font(.caption2)
                        .foregroundStyle(chipColor(level))
                }
                Text(label)
            }
            .font(.caption.weight(isSelected ? .semibold : .regular))
            .padding(.horizontal, chipHorizontalPadding)
            .padding(.vertical, chipVerticalPadding)
            .background(isSelected ? chipColor(level).opacity(0.15) : Color.clear, in: Capsule())
            .overlay(Capsule().strokeBorder(isSelected ? chipColor(level).opacity(0.3) : Color.secondary.opacity(0.2), lineWidth: 1))
        }
        .buttonStyle(.plain)
        .accessibilityLabel(isSelected
            ? String(format: NSLocalizedString("a11y.filter.chip.selected", comment: ""), label)
            : String(format: NSLocalizedString("a11y.filter.chip", comment: ""), label))
        .accessibilityAddTraits(isSelected ? .isSelected : [])
        .accessibilityHint(String(localized: "a11y.filter.chip.hint"))
    }

    private func chipColor(_ level: RiskLevel?) -> Color {
        guard let level else { return .blue }
        if contrast == .increased {
            switch level {
            case .low: return .green
            case .medium: return .orange
            case .high: return .red
            case .critical: return .red
            }
        }
        switch level {
        case .low: return .green
        case .medium: return .yellow
        case .high: return .orange
        case .critical: return .red
        }
    }
}
