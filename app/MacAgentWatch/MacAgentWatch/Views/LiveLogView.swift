import SwiftUI

struct LiveLogView: View {
    @Bindable var viewModel: MonitoringViewModel
    @State private var logEntries: [LiveLogEntry] = []
    @State private var isLive = true
    @State private var autoScroll = true
    @State private var timer: Timer?
    @Environment(\.colorSchemeContrast) private var contrast
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    private static let maxLines = 500

    var body: some View {
        VStack(spacing: 0) {
            toolbar
            Divider()
            logContent
        }
        .onAppear { startPolling() }
        .onDisappear { stopPolling() }
    }

    // MARK: - Toolbar

    private var toolbar: some View {
        HStack(spacing: 12) {
            Image(systemName: "scroll")
                .foregroundStyle(.secondary)
            Text(String(localized: "livelog.title"))
                .font(.headline)

            Spacer()

            if isLive {
                HStack(spacing: 4) {
                    Circle()
                        .fill(.green)
                        .frame(width: 8, height: 8)
                        .symbolEffect(.pulse, options: .repeating, isActive: !reduceMotion)
                    Text(String(localized: "livelog.status.live"))
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(.green)
                }
                .accessibilityElement(children: .combine)
                .accessibilityLabel(String(localized: "a11y.livelog.status.live"))
            }

            Button {
                toggleLive()
            } label: {
                Label(
                    isLive
                        ? String(localized: "livelog.pause")
                        : String(localized: "livelog.resume"),
                    systemImage: isLive ? "pause.fill" : "play.fill"
                )
                .font(.caption.weight(.medium))
            }
            .buttonStyle(.bordered)
            .accessibilityHint(String(localized: "a11y.livelog.toggle.hint"))

            Button {
                clearLog()
            } label: {
                Label(String(localized: "livelog.clear"), systemImage: "trash")
                    .font(.caption.weight(.medium))
            }
            .buttonStyle(.bordered)
            .accessibilityHint(String(localized: "a11y.livelog.clear.hint"))

            Text(verbatim: String(format: NSLocalizedString("livelog.line.count", comment: ""), logEntries.count))
                .font(.caption)
                .foregroundStyle(.tertiary)
                .monospacedDigit()
        }
        .padding(.horizontal)
        .padding(.vertical, 8)
    }

    // MARK: - Log Content

    private var logContent: some View {
        ScrollViewReader { proxy in
            ScrollView {
                if logEntries.isEmpty && viewModel.selectedSession == nil {
                    VStack(spacing: 8) {
                        Image(systemName: "text.justify.leading")
                            .font(.largeTitle)
                            .foregroundStyle(.tertiary)
                        Text(String(localized: "livelog.empty.title"))
                            .font(.headline)
                            .foregroundStyle(.secondary)
                        Text(String(localized: "livelog.empty.subtitle"))
                            .font(.caption)
                            .foregroundStyle(.tertiary)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .padding(.top, 60)
                } else {
                    LazyVStack(alignment: .leading, spacing: 1) {
                        ForEach(logEntries) { entry in
                            logLine(entry)
                                .id(entry.id)
                        }
                    }
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)
                }
            }
            .background(Color(nsColor: .textBackgroundColor))
            .onChange(of: logEntries.count) {
                if autoScroll, let last = logEntries.last {
                    withAnimation(reduceMotion ? nil : .easeOut(duration: 0.2)) {
                        proxy.scrollTo(last.id, anchor: .bottom)
                    }
                }
            }
            .modifier(ScrollAutoScrollModifier(autoScroll: $autoScroll))
        }
    }

    // MARK: - Log Line

    private func logLine(_ entry: LiveLogEntry) -> some View {
        HStack(spacing: 8) {
            Text(entry.timeString)
                .foregroundStyle(.tertiary)

            Image(systemName: entry.riskLevel.icon)
                .foregroundStyle(riskColor(entry.riskLevel))
                .frame(width: 16)

            Text(verbatim: "[\(entry.process)]")
                .foregroundStyle(.secondary)

            Text(entry.message)
                .foregroundStyle(.primary)
                .lineLimit(1)

            Spacer()
        }
        .font(.system(.caption, design: .monospaced))
        .padding(.vertical, 2)
        .padding(.horizontal, 4)
        .background(entry.riskLevel >= .high ? riskColor(entry.riskLevel).opacity(0.06) : .clear)
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(entry.timeString), \(entry.riskLevel.label), \(entry.process), \(entry.message)")
    }

    // MARK: - Actions

    private func toggleLive() {
        isLive.toggle()
        if isLive {
            autoScroll = true
            startPolling()
        } else {
            stopPolling()
        }
    }

    private func clearLog() {
        logEntries.removeAll()
    }

    private func startPolling() {
        stopPolling()
        timer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { _ in
            Task { @MainActor in
                pollNewEvents()
            }
        }
    }

    private func stopPolling() {
        timer?.invalidate()
        timer = nil
    }

    private func pollNewEvents() {
        guard isLive else { return }

        let previousCount = viewModel.events.count
        viewModel.pollLatestEvents()

        // Convert new events to log entries
        let newEvents = viewModel.events.suffix(max(0, viewModel.events.count - previousCount))
        for event in newEvents {
            let entry = LiveLogEntry(
                timestamp: event.timestamp,
                process: event.process,
                message: event.eventType.description,
                riskLevel: event.riskLevel
            )
            logEntries.append(entry)
        }

        // FIFO trim
        if logEntries.count > Self.maxLines {
            logEntries.removeFirst(logEntries.count - Self.maxLines)
        }
    }

    // MARK: - Helpers

    private func riskColor(_ level: RiskLevel) -> Color {
        if contrast == .increased {
            return AppColors.riskColorHighContrast(level)
        }
        return AppColors.riskColor(level)
    }

}

// MARK: - Supporting Types

struct LiveLogEntry: Identifiable {
    let id = UUID()
    let timestamp: Date
    let process: String
    let message: String
    let riskLevel: RiskLevel

    var timeString: String {
        Self.formatter.string(from: timestamp)
    }

    private static let formatter: DateFormatter = {
        let f = DateFormatter()
        f.dateFormat = "HH:mm:ss"
        return f
    }()
}

private struct ScrollAutoScrollModifier: ViewModifier {
    @Binding var autoScroll: Bool

    func body(content: Content) -> some View {
        if #available(macOS 15.0, *) {
            content
                .onScrollGeometryChange(for: Bool.self) { geometry in
                    let atBottom = geometry.contentOffset.y + geometry.containerSize.height >= geometry.contentSize.height - 40
                    return atBottom
                } action: { _, isAtBottom in
                    autoScroll = isAtBottom
                }
        } else {
            content
                .onAppear { autoScroll = true }
        }
    }
}
