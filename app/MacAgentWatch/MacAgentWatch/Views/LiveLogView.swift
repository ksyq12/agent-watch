import SwiftUI

struct LiveLogView: View {
    @Bindable var viewModel: MonitoringViewModel
    @State private var logEntries: [LiveLogEntry] = []
    @State private var isLive = true
    @State private var autoScroll = true
    @State private var timer: Timer?
    @State private var searchQuery: String = ""
    @State private var debouncedSearch: String = ""
    @State private var activeRiskFilters: Set<RiskLevel> = Set(RiskLevel.allCases)
    @State private var blinkingEntries: Set<UUID> = []
    @Environment(\.colorSchemeContrast) private var contrast
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @FocusState private var isSearchFocused: Bool

    @ScaledMetric(relativeTo: .caption) private var chipHorizontalPadding: CGFloat = 8
    @ScaledMetric(relativeTo: .caption) private var chipVerticalPadding: CGFloat = 3

    private static let maxLines = 500

    // MARK: - Filtered Entries

    private var filteredEntries: [LiveLogEntry] {
        logEntries.filter { entry in
            guard activeRiskFilters.contains(entry.riskLevel) else { return false }
            guard !debouncedSearch.isEmpty else { return true }
            let query = debouncedSearch.lowercased()
            return entry.message.lowercased().contains(query)
                || entry.process.lowercased().contains(query)
                || entry.typeTag.lowercased().contains(query)
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            toolbar
            Divider()
            riskFilterBar
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

            searchField

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

            lineCountLabel
        }
        .padding(.horizontal)
        .padding(.vertical, 8)
    }

    // MARK: - Search Field

    private var searchField: some View {
        HStack(spacing: 4) {
            Image(systemName: "magnifyingglass")
                .foregroundStyle(.secondary)
                .font(.caption)

            TextField(String(localized: "livelog.search.placeholder"), text: $searchQuery)
                .textFieldStyle(.plain)
                .font(.caption)
                .focused($isSearchFocused)
                .accessibilityLabel(String(localized: "a11y.livelog.search"))
                .accessibilityHint(String(localized: "a11y.livelog.search.hint"))

            if !searchQuery.isEmpty {
                Button {
                    withAnimation(.easeInOut(duration: 0.15)) {
                        searchQuery = ""
                        debouncedSearch = ""
                    }
                } label: {
                    Image(systemName: "xmark.circle.fill")
                        .foregroundStyle(.secondary)
                        .font(.caption)
                }
                .buttonStyle(.plain)
                .accessibilityLabel(String(localized: "a11y.livelog.search.clear"))
            }
        }
        .padding(.horizontal, 6)
        .padding(.vertical, 4)
        .background(Color.secondary.opacity(0.1), in: RoundedRectangle(cornerRadius: 6))
        .frame(maxWidth: 200)
        .task(id: searchQuery) {
            do {
                try await Task.sleep(for: .milliseconds(300))
                debouncedSearch = searchQuery
            } catch {}
        }
        .background {
            Button("") { isSearchFocused = true }
                .keyboardShortcut("f", modifiers: .command)
                .hidden()
        }
    }

    // MARK: - Line Count

    private var lineCountLabel: some View {
        Group {
            if debouncedSearch.isEmpty && activeRiskFilters.count == RiskLevel.allCases.count {
                Text(verbatim: String(format: NSLocalizedString("livelog.line.count", comment: ""), logEntries.count))
            } else {
                Text(verbatim: String(format: NSLocalizedString("livelog.filter.showing", comment: ""), filteredEntries.count, logEntries.count))
            }
        }
        .font(.caption)
        .foregroundStyle(.tertiary)
        .monospacedDigit()
    }

    // MARK: - Risk Filter Bar

    private var riskFilterBar: some View {
        HStack(spacing: 6) {
            ForEach(RiskLevel.allCases, id: \.self) { level in
                riskFilterChip(level)
            }
            Spacer()
        }
        .padding(.horizontal)
        .padding(.vertical, 5)
    }

    private func riskFilterChip(_ level: RiskLevel) -> some View {
        let isActive = activeRiskFilters.contains(level)
        let count = logEntries.filter { $0.riskLevel == level }.count
        let color = riskColor(level)

        return Button {
            withAnimation(.easeInOut(duration: 0.15)) {
                if isActive {
                    activeRiskFilters.remove(level)
                } else {
                    activeRiskFilters.insert(level)
                }
            }
        } label: {
            HStack(spacing: 3) {
                Image(systemName: level.icon)
                    .font(.caption2)
                    .foregroundStyle(color)
                Text(level.label)
                Text(verbatim: "\(count)")
                    .padding(.horizontal, 4)
                    .padding(.vertical, 1)
                    .background(color.opacity(0.15), in: Capsule())
            }
            .font(.caption.weight(isActive ? .semibold : .regular))
            .padding(.horizontal, chipHorizontalPadding)
            .padding(.vertical, chipVerticalPadding)
            .background(isActive ? color.opacity(0.12) : Color.clear, in: Capsule())
            .overlay(
                Capsule().strokeBorder(
                    isActive ? color.opacity(0.3) : Color.secondary.opacity(0.2),
                    lineWidth: 1
                )
            )
            .opacity(isActive ? 1.0 : 0.5)
        }
        .buttonStyle(.plain)
        .accessibilityLabel("\(level.label), \(count)")
        .accessibilityAddTraits(isActive ? .isSelected : [])
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
                        ForEach(filteredEntries) { entry in
                            logLine(entry)
                                .id(entry.id)
                        }
                    }
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)
                }
            }
            .background(Color(nsColor: .textBackgroundColor))
            .onChange(of: filteredEntries.count) {
                if autoScroll, let last = filteredEntries.last {
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
        let isHighRisk = entry.riskLevel >= .high
        let isBlink = blinkingEntries.contains(entry.id)

        return HStack(spacing: 0) {
            // Vertical color bar for high/critical
            if isHighRisk {
                Rectangle()
                    .fill(riskColor(entry.riskLevel))
                    .frame(width: 4)
            }

            HStack(spacing: 8) {
                Text(entry.timeString)
                    .foregroundStyle(.tertiary)

                Image(systemName: entry.riskLevel.icon)
                    .foregroundStyle(riskColor(entry.riskLevel))
                    .frame(width: 16)

                Text(verbatim: "[\(entry.process)]")
                    .foregroundStyle(.secondary)

                Text(entry.typeTag)
                    .foregroundStyle(.tint)

                Text(highlightedText(entry.message, query: debouncedSearch))
                    .foregroundStyle(.primary)
                    .lineLimit(1)

                Spacer()
            }
            .padding(.vertical, 2)
            .padding(.horizontal, 4)
        }
        .font(.system(.caption, design: .monospaced))
        .background(isHighRisk ? riskColor(entry.riskLevel).opacity(0.15) : .clear)
        .overlay(
            isBlink
                ? riskColor(entry.riskLevel).opacity(0.3)
                : Color.clear.opacity(0)
        )
        .animation(.easeOut(duration: 1.0), value: isBlink)
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(entry.timeString), \(entry.riskLevel.label), \(entry.process), \(entry.typeTag), \(entry.message)")
    }

    // MARK: - Search Highlight

    private func highlightedText(_ text: String, query: String) -> AttributedString {
        var attributed = AttributedString(text)
        guard !query.isEmpty else { return attributed }

        let lowercased = text.lowercased()
        let queryLower = query.lowercased()
        var searchStart = lowercased.startIndex

        while searchStart < lowercased.endIndex,
              let range = lowercased.range(of: queryLower, range: searchStart..<lowercased.endIndex) {
            let attrStart = AttributedString.Index(range.lowerBound, within: attributed)
            let attrEnd = AttributedString.Index(range.upperBound, within: attributed)
            if let attrStart, let attrEnd {
                attributed[attrStart..<attrEnd].backgroundColor = .yellow.opacity(0.3)
            }
            searchStart = range.upperBound
        }

        return attributed
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
        blinkingEntries.removeAll()
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
                message: event.eventType.summaryText,
                riskLevel: event.riskLevel,
                typeTag: event.eventType.typeTag
            )
            logEntries.append(entry)

            // Blink animation for critical events
            if entry.riskLevel >= .high && !reduceMotion {
                blinkingEntries.insert(entry.id)
                let entryId = entry.id
                Task { @MainActor in
                    try? await Task.sleep(for: .seconds(1))
                    withAnimation(.easeOut(duration: 0.3)) {
                        blinkingEntries.remove(entryId)
                    }
                }
            }
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
    let typeTag: String

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
