import SwiftUI

struct MenuBarView: View {
    @Bindable var viewModel: MonitoringViewModel
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            Divider()
            summarySection
            Divider()
            alertsSection
            Divider()
            actionsSection
        }
        .frame(width: 300)
    }

    // MARK: - Header

    private var header: some View {
        HStack(spacing: 8) {
            Image(systemName: "shield.checkered")
                .font(.title2)
                .foregroundStyle(.tint)

            VStack(alignment: .leading, spacing: 2) {
                Text("app.name")
                    .font(.headline)
                Text(String(format: String(localized: "app.version"), viewModel.version))
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Spacer()

            statusBadge
        }
        .padding(12)
    }

    private var statusBadge: some View {
        HStack(spacing: 4) {
            Circle()
                .fill(viewModel.isMonitoring ? Color.green : Color.secondary)
                .frame(width: 8, height: 8)
            Text(viewModel.isMonitoring ? String(localized: "status.active") : String(localized: "status.idle"))
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(.quaternary, in: Capsule())
        .accessibilityLabel(viewModel.isMonitoring ? String(localized: "a11y.status.monitoring") : String(localized: "a11y.status.idle"))
    }

    // MARK: - Summary

    private var summarySection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("summary.title")
                .font(.subheadline.weight(.medium))
                .foregroundStyle(.secondary)

            HStack(spacing: 8) {
                summaryCard(
                    count: viewModel.activitySummary.totalEvents,
                    label: String(localized: "summary.total"),
                    color: .primary
                )
                summaryCard(
                    count: viewModel.activitySummary.criticalCount,
                    label: String(localized: "summary.critical"),
                    color: .red
                )
                summaryCard(
                    count: viewModel.activitySummary.highCount,
                    label: String(localized: "summary.high"),
                    color: .orange
                )
                summaryCard(
                    count: viewModel.activitySummary.mediumCount,
                    label: String(localized: "summary.medium"),
                    color: .yellow
                )
            }
        }
        .padding(12)
    }

    private func summaryCard(count: Int, label: String, color: Color) -> some View {
        VStack(spacing: 2) {
            Text("\(count)")
                .font(.title3.weight(.semibold).monospacedDigit())
                .foregroundStyle(color)
            Text(label)
                .font(.caption2)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 6)
        .background(.quaternary, in: RoundedRectangle(cornerRadius: 6))
        .accessibilityLabel(String(format: String(localized: "a11y.summary.card"), label, count))
    }

    // MARK: - Alerts

    private var alertsSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("alerts.title")
                .font(.subheadline.weight(.medium))
                .foregroundStyle(.secondary)

            if viewModel.recentAlerts.isEmpty {
                HStack {
                    Spacer()
                    Text("alerts.empty")
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                    Spacer()
                }
                .padding(.vertical, 8)
            } else {
                ForEach(viewModel.recentAlerts) { event in
                    alertRow(event)
                }
            }
        }
        .padding(12)
    }

    private func alertRow(_ event: MonitoringEvent) -> some View {
        HStack(spacing: 8) {
            Image(systemName: event.riskLevel.icon)
                .font(.caption)
                .foregroundStyle(riskColor(event.riskLevel))

            VStack(alignment: .leading, spacing: 1) {
                Text(event.eventType.description)
                    .font(.caption)
                    .lineLimit(1)
                Text(event.process)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }

            Spacer()

            Text(event.timestamp, style: .relative)
                .font(.caption2)
                .foregroundStyle(.tertiary)
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel(String(format: String(localized: "a11y.risk.indicator"), String(describing: event.riskLevel)))
    }

    // MARK: - Actions

    private var actionsSection: some View {
        VStack(spacing: 4) {
            Button {
                openWindow(id: "dashboard")
            } label: {
                Label("action.open.dashboard", systemImage: "rectangle.on.rectangle")
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .buttonStyle(.borderless)
            .accessibilityLabel(String(localized: "action.open.dashboard"))
            .accessibilityHint(String(localized: "app.dashboard.title"))

            Divider()

            Button(role: .destructive) {
                NSApplication.shared.terminate(nil)
            } label: {
                Label("action.quit", systemImage: "power")
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .buttonStyle(.borderless)
            .accessibilityLabel(String(localized: "action.quit"))
        }
        .padding(12)
    }

    // MARK: - Helpers

    private func riskColor(_ level: RiskLevel) -> Color {
        switch level {
        case .low: return .green
        case .medium: return .yellow
        case .high: return .orange
        case .critical: return .red
        }
    }
}
