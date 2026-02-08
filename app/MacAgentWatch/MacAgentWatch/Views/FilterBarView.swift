import SwiftUI

struct FilterBarView: View {
    @Binding var filterRiskLevel: RiskLevel?
    @Binding var searchQuery: String
    @Binding var eventTypeFilter: EventTypeFilter
    @Binding var dateRangePreset: DateRangePreset
    @Binding var customStartDate: Date?
    @Binding var customEndDate: Date?
    let filteredCount: Int
    let isChartFilterActive: Bool
    let onClearChartFilter: () -> Void

    @Environment(\.colorSchemeContrast) private var contrast
    @ScaledMetric(relativeTo: .caption) private var chipHorizontalPadding: CGFloat = 10
    @ScaledMetric(relativeTo: .caption) private var chipVerticalPadding: CGFloat = 4

    @State private var debouncedSearchText: String = ""
    @FocusState private var isSearchFocused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            // Chart filter badge (only shown when chart filter is active)
            if isChartFilterActive {
                HStack(spacing: 8) {
                    Image(systemName: "chart.bar.xaxis")
                        .foregroundStyle(.blue)
                        .font(.caption)
                    Text(String(localized: "filter.chart.applied"))
                        .font(.caption.weight(.medium))
                        .foregroundStyle(.blue)

                    Spacer()

                    Button {
                        withAnimation(.easeInOut(duration: 0.15)) {
                            onClearChartFilter()
                        }
                    } label: {
                        HStack(spacing: 4) {
                            Image(systemName: "xmark.circle.fill")
                                .font(.caption)
                            Text(String(localized: "filter.chart.clear"))
                                .font(.caption)
                        }
                        .foregroundStyle(.secondary)
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel(String(localized: "filter.chart.clear"))
                    .accessibilityHint(String(localized: "a11y.filter.chart.clear.hint"))
                }
                .padding(.horizontal, 10)
                .padding(.vertical, 6)
                .background(Color.blue.opacity(0.08), in: RoundedRectangle(cornerRadius: 8))
                .accessibilityLabel(String(localized: "a11y.filter.chart.badge"))
            }

            // Top row: search bar + date range picker
            HStack(spacing: 12) {
                searchBar
                dateRangePicker
            }

            // Custom date pickers when "Custom" is selected
            if dateRangePreset == .custom {
                customDateRow
            }

            // Risk level filter chips
            HStack(spacing: 8) {
                Text(String(localized: "filter.label"))
                    .font(.subheadline)
                    .foregroundStyle(.secondary)

                riskFilterChip(label: String(localized: "filter.all"), icon: nil, level: nil)
                ForEach(RiskLevel.allCases, id: \.self) { level in
                    riskFilterChip(label: level.label, icon: level.icon, level: level)
                }
            }

            // Event type filter chips
            HStack(spacing: 8) {
                Text(String(localized: "filter.type.label"))
                    .font(.subheadline)
                    .foregroundStyle(.secondary)

                ForEach(EventTypeFilter.allCases, id: \.self) { type in
                    eventTypeChip(type: type)
                }
            }

            // Event count
            HStack {
                Spacer()
                Text(verbatim: String(format: NSLocalizedString("filter.events.count", comment: ""), filteredCount))
                    .font(.caption)
                    .foregroundStyle(.tertiary)
            }
        }
    }

    // MARK: - Search Bar

    private var searchBar: some View {
        HStack(spacing: 6) {
            Image(systemName: "magnifyingglass")
                .foregroundStyle(.secondary)
                .font(.caption)

            TextField(String(localized: "filter.search.placeholder"), text: $debouncedSearchText)
                .textFieldStyle(.plain)
                .font(.caption)
                .focused($isSearchFocused)
                .accessibilityLabel(String(localized: "a11y.filter.search"))
                .accessibilityHint(String(localized: "a11y.filter.search.hint"))

            if !debouncedSearchText.isEmpty {
                Button {
                    withAnimation(.easeInOut(duration: 0.15)) {
                        debouncedSearchText = ""
                        searchQuery = ""
                    }
                } label: {
                    Image(systemName: "xmark.circle.fill")
                        .foregroundStyle(.secondary)
                        .font(.caption)
                }
                .buttonStyle(.plain)
                .accessibilityLabel(String(localized: "a11y.filter.search.clear"))
            }
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 5)
        .background(Color.secondary.opacity(0.1), in: RoundedRectangle(cornerRadius: 8))
        .frame(maxWidth: 280)
        .task(id: debouncedSearchText) {
            do {
                try await Task.sleep(for: .milliseconds(300))
                searchQuery = debouncedSearchText
            } catch {}
        }
        .background {
            Button("") { isSearchFocused = true }
                .keyboardShortcut("f", modifiers: .command)
                .hidden()
        }
        .onAppear {
            debouncedSearchText = searchQuery
        }
    }

    // MARK: - Date Range Picker

    private var dateRangePicker: some View {
        HStack(spacing: 4) {
            Image(systemName: "calendar")
                .foregroundStyle(.secondary)
                .font(.caption)

            Picker(selection: $dateRangePreset) {
                ForEach(DateRangePreset.allCases, id: \.self) { preset in
                    Text(preset.label).tag(preset)
                }
            } label: {
                EmptyView()
            }
            .pickerStyle(.menu)
            .fixedSize()
            .accessibilityLabel(String(localized: "a11y.filter.dateRange"))
            .accessibilityHint(String(localized: "a11y.filter.dateRange.hint"))
        }
    }

    // MARK: - Custom Date Row

    private var customDateRow: some View {
        HStack(spacing: 12) {
            Text(String(localized: "filter.date.from"))
                .font(.caption)
                .foregroundStyle(.secondary)

            DatePicker(
                "",
                selection: Binding(
                    get: { customStartDate ?? Date() },
                    set: { customStartDate = $0 }
                ),
                displayedComponents: [.date, .hourAndMinute]
            )
            .labelsHidden()
            .datePickerStyle(.compact)
            .accessibilityLabel(String(localized: "a11y.filter.date.start"))

            Text(String(localized: "filter.date.to"))
                .font(.caption)
                .foregroundStyle(.secondary)

            DatePicker(
                "",
                selection: Binding(
                    get: { customEndDate ?? Date() },
                    set: { customEndDate = $0 }
                ),
                displayedComponents: [.date, .hourAndMinute]
            )
            .labelsHidden()
            .datePickerStyle(.compact)
            .accessibilityLabel(String(localized: "a11y.filter.date.end"))
        }
        .transition(.opacity.combined(with: .move(edge: .top)))
    }

    // MARK: - Risk Level Chip

    private func riskFilterChip(label: String, icon: String?, level: RiskLevel?) -> some View {
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
                        .foregroundStyle(riskChipColor(level))
                }
                Text(label)
            }
            .font(.caption.weight(isSelected ? .semibold : .regular))
            .padding(.horizontal, chipHorizontalPadding)
            .padding(.vertical, chipVerticalPadding)
            .background(isSelected ? riskChipColor(level).opacity(0.15) : Color.clear, in: Capsule())
            .overlay(Capsule().strokeBorder(isSelected ? riskChipColor(level).opacity(0.3) : Color.secondary.opacity(0.2), lineWidth: 1))
        }
        .buttonStyle(.plain)
        .accessibilityLabel(isSelected
            ? String(format: NSLocalizedString("a11y.filter.chip.selected", comment: ""), label)
            : String(format: NSLocalizedString("a11y.filter.chip", comment: ""), label))
        .accessibilityAddTraits(isSelected ? .isSelected : [])
        .accessibilityHint(String(localized: "a11y.filter.chip.hint"))
    }

    // MARK: - Event Type Chip

    private func eventTypeChip(type: EventTypeFilter) -> some View {
        let isSelected = eventTypeFilter == type
        return Button {
            withAnimation(.easeInOut(duration: 0.15)) {
                eventTypeFilter = type
            }
        } label: {
            HStack(spacing: 4) {
                Image(systemName: type.icon)
                    .font(.caption2)
                    .foregroundStyle(isSelected ? .blue : .secondary)
                Text(type.label)
            }
            .font(.caption.weight(isSelected ? .semibold : .regular))
            .padding(.horizontal, chipHorizontalPadding)
            .padding(.vertical, chipVerticalPadding)
            .background(isSelected ? Color.blue.opacity(0.15) : Color.clear, in: Capsule())
            .overlay(Capsule().strokeBorder(isSelected ? Color.blue.opacity(0.3) : Color.secondary.opacity(0.2), lineWidth: 1))
        }
        .buttonStyle(.plain)
        .accessibilityLabel(isSelected
            ? String(format: NSLocalizedString("a11y.filter.chip.selected", comment: ""), type.label)
            : String(format: NSLocalizedString("a11y.filter.chip", comment: ""), type.label))
        .accessibilityAddTraits(isSelected ? .isSelected : [])
        .accessibilityHint(String(localized: "a11y.filter.eventType.hint"))
    }

    // MARK: - Colors

    private func riskChipColor(_ level: RiskLevel?) -> Color {
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
