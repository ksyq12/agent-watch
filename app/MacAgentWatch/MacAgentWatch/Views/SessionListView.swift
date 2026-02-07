import SwiftUI

struct SessionListView: View {
    @Bindable var viewModel: MonitoringViewModel

    var body: some View {
        List(viewModel.sessions) { session in
            SessionRowButton(
                session: session,
                isSelected: viewModel.selectedSession?.id == session.id,
                onSelect: { viewModel.loadSession(session) }
            )
        }
        .listStyle(.sidebar)
        .navigationTitle(String(localized: "sessions.title"))
    }
}

private struct SessionRowButton: View {
    let session: SessionInfo
    let isSelected: Bool
    let onSelect: () -> Void
    @ScaledMetric(relativeTo: .body) private var rowVerticalPadding: CGFloat = 4

    private static let dateTimeFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.locale = Locale.autoupdatingCurrent
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter
    }()

    var body: some View {
        Button(action: onSelect) {
            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 6) {
                    Image(systemName: "clock.arrow.circlepath")
                        .font(.caption)
                        .foregroundStyle(.secondary)

                    Text(truncatedId)
                        .font(.caption.weight(.medium).monospaced())
                        .lineLimit(1)
                }

                timestampView
            }
            .padding(.vertical, rowVerticalPadding)
        }
        .buttonStyle(.plain)
        .listRowBackground(isSelected ? Color.accentColor.opacity(0.15) : Color.clear)
        .accessibilityElement(children: .combine)
        .accessibilityLabel(String(format: NSLocalizedString("a11y.session.row", comment: ""), session.sessionId, session.startTimeString))
        .accessibilityHint(Text("a11y.session.hint"))
        .accessibilityValue(session.startTimeString)
    }

    @ViewBuilder
    private var timestampView: some View {
        if let startTime = session.startTime {
            Text(Self.dateTimeFormatter.string(from: startTime))
                .font(.caption2)
                .foregroundStyle(.tertiary)
        } else {
            Text(session.startTimeString)
                .font(.caption2)
                .foregroundStyle(.tertiary)
        }
    }

    private var truncatedId: String {
        let id = session.sessionId
        if id.count > 24 {
            return String(id.prefix(24)) + "..."
        }
        return id
    }
}
