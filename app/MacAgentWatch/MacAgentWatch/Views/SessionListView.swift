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
                onSelect: { viewModel.loadSession(session) }
            )
            .onAppear { viewModel.loadSessionEventCount(for: session) }
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
    let onSelect: () -> Void
    @ScaledMetric(relativeTo: .body) private var rowVerticalPadding: CGFloat = 4

    var body: some View {
        Button(action: onSelect) {
            HStack(spacing: 6) {
                if isActive {
                    Circle()
                        .fill(Color.green)
                        .frame(width: 8, height: 8)
                }

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
            }
            .padding(.vertical, rowVerticalPadding)
        }
        .buttonStyle(.plain)
        .listRowBackground(isSelected ? Color.accentColor.opacity(0.15) : Color.clear)
        .accessibilityElement(children: .combine)
        .accessibilityLabel(String(format: NSLocalizedString("a11y.session.row", comment: ""), session.sessionId, displayName))
        .accessibilityHint(Text("a11y.session.hint"))
        .accessibilityValue(displayName)
    }
}
