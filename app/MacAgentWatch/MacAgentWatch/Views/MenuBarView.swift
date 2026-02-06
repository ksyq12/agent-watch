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
                Text("MacAgentWatch")
                    .font(.headline)
                Text("v\(viewModel.version)")
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
            Text(viewModel.isMonitoring ? "Active" : "Idle")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(.quaternary, in: Capsule())
    }

    // MARK: - Summary

    private var summarySection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Activity Summary")
                .font(.subheadline.weight(.medium))
                .foregroundStyle(.secondary)

            HStack(spacing: 8) {
                summaryCard(
                    count: viewModel.activitySummary.totalEvents,
                    label: "Total",
                    color: .primary
                )
                summaryCard(
                    count: viewModel.activitySummary.criticalCount,
                    label: "Critical",
                    color: .red
                )
                summaryCard(
                    count: viewModel.activitySummary.highCount,
                    label: "High",
                    color: .orange
                )
                summaryCard(
                    count: viewModel.activitySummary.mediumCount,
                    label: "Medium",
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
    }

    // MARK: - Alerts

    private var alertsSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Recent Alerts")
                .font(.subheadline.weight(.medium))
                .foregroundStyle(.secondary)

            if viewModel.recentAlerts.isEmpty {
                HStack {
                    Spacer()
                    Text("No recent alerts")
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
    }

    // MARK: - Actions

    private var actionsSection: some View {
        VStack(spacing: 4) {
            Button {
                openWindow(id: "dashboard")
            } label: {
                Label("Open Dashboard", systemImage: "rectangle.on.rectangle")
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .buttonStyle(.borderless)

            Divider()

            Button(role: .destructive) {
                NSApplication.shared.terminate(nil)
            } label: {
                Label("Quit MacAgentWatch", systemImage: "power")
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .buttonStyle(.borderless)
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
