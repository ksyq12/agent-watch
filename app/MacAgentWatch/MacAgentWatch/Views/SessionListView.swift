import SwiftUI

struct SessionListView: View {
    @Bindable var viewModel: MonitoringViewModel

    var body: some View {
        List(viewModel.sessions) { session in
            SessionRowButton(
                session: session,
                isSelected: viewModel.selectedSession?.id == session.id,
                isActive: viewModel.isActiveSession(session),
                displayName: viewModel.sessionDisplayName(for: session),
                eventCount: viewModel.sessionEventCounts[session.id],
                riskSummary: viewModel.sessionRiskSummaries[session.id],
                onSelect: { viewModel.loadSession(session) }
            )
            .onAppear {
                viewModel.loadSessionEventCount(for: session)
                viewModel.loadSessionRiskSummary(for: session)
            }
        }
        .listStyle(.sidebar)
        .navigationTitle(String(localized: "sessions.title"))
    }
}

private struct SessionRowButton: View {
    let session: SessionInfo
    let isSelected: Bool
    let isActive: Bool
    let displayName: String
    let eventCount: Int?
    let riskSummary: ActivitySummary?
    let onSelect: () -> Void

    @Environment(\.colorSchemeContrast) private var contrast
    @ScaledMetric(relativeTo: .caption) private var rowVerticalPadding: CGFloat = 4
    @ScaledMetric(relativeTo: .caption2) private var miniBarHeight: CGFloat = 4

    var body: some View {
        Button(action: onSelect) {
            VStack(alignment: .leading, spacing: 3) {
                topLine
                middleLine
                if let summary = riskSummary, summary.totalEvents > 0 {
                    riskDistributionBar(summary)
                }
            }
            .padding(.vertical, rowVerticalPadding)
        }
        .buttonStyle(.plain)
        .listRowBackground(isSelected ? Color.accentColor.opacity(0.15) : Color.clear)
        .accessibilityElement(children: .combine)
        .accessibilityLabel(accessibilityText)
        .accessibilityHint(Text("a11y.session.hint"))
    }

    // MARK: - Top Line: Agent Name + LIVE Badge

    private var topLine: some View {
        HStack(spacing: 4) {
            Image(systemName: "cpu")
                .font(.caption2)
                .foregroundStyle(.secondary)

            Text(session.agentName ?? String(localized: "session.agent.unknown"))
                .font(.caption2)
                .foregroundStyle(.secondary)
                .lineLimit(1)

            Spacer()

            if isActive {
                liveBadge
            }
        }
    }

    // MARK: - Middle Line: Display Name + Event Count + Risk Badge

    private var middleLine: some View {
        HStack(spacing: 6) {
            Image(systemName: "clock.arrow.circlepath")
                .font(.caption)
                .foregroundStyle(.secondary)

            Text(displayName)
                .font(.caption.weight(.medium))
                .lineLimit(1)

            Spacer()

            if let eventCount, eventCount > 0 {
                Text("\(eventCount)")
                    .font(.caption2)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(.quaternary, in: Capsule())
                    .accessibilityLabel(String(format: NSLocalizedString("session.events.count", comment: ""), eventCount))
            }

            riskBadge
        }
    }

    // MARK: - LIVE Badge

    private var liveBadge: some View {
        Text("LIVE")
            .font(.system(.caption2, weight: .bold))
            .foregroundStyle(.white)
            .padding(.horizontal, 5)
            .padding(.vertical, 1)
            .background(.red, in: Capsule())
            .accessibilityLabel(String(localized: "a11y.session.live.badge"))
    }

    // MARK: - Risk Badge

    private var riskBadge: some View {
        let effectiveColor = contrast == .increased
            ? AppColors.riskColorHighContrast(session.maxRiskLevel)
            : AppColors.riskColor(session.maxRiskLevel)
        return HStack(spacing: 2) {
            Image(systemName: session.maxRiskLevel.icon)
                .font(.caption2)
            Text(session.maxRiskLevel.label)
                .font(.caption2.weight(.medium))
        }
        .foregroundStyle(effectiveColor)
        .padding(.horizontal, 5)
        .padding(.vertical, 1)
        .background(effectiveColor.opacity(0.12), in: Capsule())
    }

    // MARK: - Risk Distribution Mini Bar

    private func riskDistributionBar(_ summary: ActivitySummary) -> some View {
        let total = CGFloat(summary.totalEvents)
        return GeometryReader { geo in
            HStack(spacing: 0) {
                barSegment(count: summary.criticalCount, total: total, level: .critical, width: geo.size.width)
                barSegment(count: summary.highCount, total: total, level: .high, width: geo.size.width)
                barSegment(count: summary.mediumCount, total: total, level: .medium, width: geo.size.width)
                barSegment(count: summary.lowCount, total: total, level: .low, width: geo.size.width)
            }
            .clipShape(Capsule())
        }
        .frame(height: miniBarHeight)
        .accessibilityLabel(riskBarAccessibilityLabel(summary))
    }

    private func barSegment(count: Int, total: CGFloat, level: RiskLevel, width: CGFloat) -> some View {
        let proportion = total > 0 ? CGFloat(count) / total : 0
        let color = contrast == .increased
            ? AppColors.riskColorHighContrast(level)
            : AppColors.riskColor(level)
        return color
            .frame(width: proportion * width)
    }

    // MARK: - Accessibility

    private var accessibilityText: String {
        var parts = [String]()
        if let name = session.agentName {
            parts.append(name)
        }
        if isActive {
            parts.append(String(localized: "a11y.session.live.badge"))
        }
        parts.append(displayName)
        parts.append(session.maxRiskLevel.label)
        return parts.joined(separator: ", ")
    }

    private func riskBarAccessibilityLabel(_ summary: ActivitySummary) -> String {
        String(format: NSLocalizedString("a11y.session.risk.distribution", comment: ""),
               summary.criticalCount, summary.highCount, summary.mediumCount, summary.lowCount)
    }
}
