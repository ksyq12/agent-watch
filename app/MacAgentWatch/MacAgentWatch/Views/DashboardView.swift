import SwiftUI

struct DashboardView: View {
    @Bindable var viewModel: MonitoringViewModel

    var body: some View {
        NavigationSplitView {
            SessionListView(viewModel: viewModel)
                .navigationSplitViewColumnWidth(min: 200, ideal: 240, max: 300)
        } detail: {
            detailContent
        }
        .navigationTitle(String(localized: "app.name"))
        .frame(minWidth: 800, minHeight: 500)
    }

    // MARK: - Detail Content

    private var detailContent: some View {
        VStack(spacing: 0) {
            activityCards
                .padding()

            filterBar
                .padding(.horizontal)
                .padding(.bottom, 8)

            Divider()

            eventsList
        }
    }

    // MARK: - Activity Cards

    private var activityCards: some View {
        HStack(spacing: 12) {
            activityCard(
                title: String(localized: "summary.total.events"),
                count: viewModel.activitySummary.totalEvents,
                icon: "list.bullet",
                color: .blue
            )
            activityCard(
                title: String(localized: "summary.critical"),
                count: viewModel.activitySummary.criticalCount,
                icon: "xmark.octagon.fill",
                color: .red
            )
            activityCard(
                title: String(localized: "summary.high"),
                count: viewModel.activitySummary.highCount,
                icon: "exclamationmark.octagon.fill",
                color: .orange
            )
            activityCard(
                title: String(localized: "summary.medium"),
                count: viewModel.activitySummary.mediumCount,
                icon: "exclamationmark.triangle.fill",
                color: .yellow
            )
            activityCard(
                title: String(localized: "summary.low"),
                count: viewModel.activitySummary.lowCount,
                icon: "checkmark.circle.fill",
                color: .green
            )
        }
    }

    private func activityCard(title: String, count: Int, icon: String, color: Color) -> some View {
        VStack(spacing: 6) {
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
        .padding(.vertical, 12)
        .background {
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .fill(color.opacity(0.08))
                .overlay(
                    RoundedRectangle(cornerRadius: 10, style: .continuous)
                        .strokeBorder(color.opacity(0.15), lineWidth: 1)
                )
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel(String(format: NSLocalizedString("a11y.dashboard.card", comment: ""), title, count))
    }

    // MARK: - Filter Bar

    private var filterBar: some View {
        HStack(spacing: 8) {
            Text("filter.label")
                .font(.subheadline)
                .foregroundStyle(.secondary)

            filterChip(label: String(localized: "filter.all"), level: nil)
            ForEach(RiskLevel.allCases, id: \.self) { level in
                filterChip(label: level.label, level: level)
            }

            Spacer()

            Text(verbatim: String(format: NSLocalizedString("filter.events.count", comment: ""), viewModel.filteredEvents.count))
                .font(.caption)
                .foregroundStyle(.tertiary)
        }
    }

    private func filterChip(label: String, level: RiskLevel?) -> some View {
        let isSelected = viewModel.filterRiskLevel == level
        return Button {
            withAnimation(.easeInOut(duration: 0.15)) {
                viewModel.filterRiskLevel = level
            }
        } label: {
            Text(label)
                .font(.caption.weight(isSelected ? .semibold : .regular))
                .padding(.horizontal, 10)
                .padding(.vertical, 4)
                .background(isSelected ? chipColor(level).opacity(0.15) : Color.clear, in: Capsule())
                .overlay(Capsule().strokeBorder(isSelected ? chipColor(level).opacity(0.3) : Color.secondary.opacity(0.2), lineWidth: 1))
        }
        .buttonStyle(.plain)
        .accessibilityLabel(isSelected
            ? String(format: NSLocalizedString("a11y.filter.chip.selected", comment: ""), label)
            : String(format: NSLocalizedString("a11y.filter.chip", comment: ""), label))
        .accessibilityAddTraits(isSelected ? .isSelected : [])
    }

    private func chipColor(_ level: RiskLevel?) -> Color {
        guard let level else { return .blue }
        switch level {
        case .low: return .green
        case .medium: return .yellow
        case .high: return .orange
        case .critical: return .red
        }
    }

    // MARK: - Events List

    private var eventsList: some View {
        List(viewModel.filteredEvents) { event in
            EventRowView(event: event)
        }
        .listStyle(.inset(alternatesRowBackgrounds: true))
        .accessibilityLabel(Text("a11y.events.list"))
    }
}
