import SwiftUI
import Charts

struct ChartsView: View {
    @Bindable var viewModel: MonitoringViewModel
    @State private var selectedPeriod: ChartPeriod = .day
    @Environment(\.colorSchemeContrast) private var contrast

    // Hover state
    @State private var hoveredTimelinePoint: ChartDataPoint?
    @State private var hoverLocation: CGPoint = .zero

    // Donut selection via angle
    @State private var selectedAngleValue: Int?

    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                periodPicker
                activityTimelineChart
                HStack(spacing: 16) {
                    riskDistributionChart
                    eventTypeChart
                }
            }
            .padding()
        }
        .onChange(of: selectedPeriod) {
            loadChartDataForPeriod()
        }
        .onAppear {
            loadChartDataForPeriod()
        }
    }

    // MARK: - Period Picker

    private var periodPicker: some View {
        Picker(String(localized: "charts.period"), selection: $selectedPeriod) {
            ForEach(ChartPeriod.allCases, id: \.self) { period in
                Text(period.label).tag(period)
            }
        }
        .pickerStyle(.segmented)
        .frame(maxWidth: 300)
        .accessibilityLabel(String(localized: "a11y.charts.period.picker"))
    }

    // MARK: - Activity Timeline

    private var activityTimelineChart: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label(String(localized: "charts.activity.title"), systemImage: "chart.bar.xaxis")
                .font(.headline)

            if timelineData.isEmpty {
                ContentUnavailableView {
                    Label(String(localized: "charts.empty.timeline.title"), systemImage: "chart.bar.xaxis")
                } description: {
                    Text(String(localized: "charts.empty.timeline.description"))
                }
                .frame(height: 220)
            } else {
                Chart(timelineData) { point in
                    BarMark(
                        x: .value(String(localized: "charts.axis.time"), point.timestamp, unit: timelineUnit),
                        y: .value(String(localized: "charts.axis.count"), point.critical)
                    )
                    .foregroundStyle(by: .value(String(localized: "charts.risk"), String(localized: "risk.critical")))
                    .opacity(timelineBarOpacity(for: point))

                    BarMark(
                        x: .value(String(localized: "charts.axis.time"), point.timestamp, unit: timelineUnit),
                        y: .value(String(localized: "charts.axis.count"), point.high)
                    )
                    .foregroundStyle(by: .value(String(localized: "charts.risk"), String(localized: "risk.high")))
                    .opacity(timelineBarOpacity(for: point))

                    BarMark(
                        x: .value(String(localized: "charts.axis.time"), point.timestamp, unit: timelineUnit),
                        y: .value(String(localized: "charts.axis.count"), point.medium)
                    )
                    .foregroundStyle(by: .value(String(localized: "charts.risk"), String(localized: "risk.medium")))
                    .opacity(timelineBarOpacity(for: point))

                    BarMark(
                        x: .value(String(localized: "charts.axis.time"), point.timestamp, unit: timelineUnit),
                        y: .value(String(localized: "charts.axis.count"), point.low)
                    )
                    .foregroundStyle(by: .value(String(localized: "charts.risk"), String(localized: "risk.low")))
                    .opacity(timelineBarOpacity(for: point))

                    // Selection indicator
                    if let selected = viewModel.selectedTimelineBucket,
                       Calendar.current.isDate(point.timestamp, equalTo: selected, toGranularity: timelineUnit) {
                        RuleMark(x: .value("Selected", point.timestamp, unit: timelineUnit))
                            .foregroundStyle(.blue.opacity(0.3))
                            .lineStyle(StrokeStyle(lineWidth: 2, dash: [4, 4]))
                            .annotation(position: .top, alignment: .center) {
                                Text(verbatim: "\(point.total)")
                                    .font(.caption2.bold())
                                    .padding(4)
                                    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 4))
                            }
                    }
                }
                .chartForegroundStyleScale(riskColorMapping)
                .chartOverlay { chart in
                    GeometryReader { geometry in
                        Rectangle()
                            .fill(.clear)
                            .contentShape(Rectangle())
                            .onTapGesture { location in
                                handleTimelineTap(at: location, chart: chart, geometry: geometry)
                            }
                            .onContinuousHover { phase in
                                switch phase {
                                case .active(let location):
                                    handleTimelineHover(at: location, chart: chart, geometry: geometry)
                                case .ended:
                                    hoveredTimelinePoint = nil
                                }
                            }
                    }
                }
                .chartBackground { chart in
                    // Hover tooltip
                    if let hovered = hoveredTimelinePoint {
                        timelineTooltip(for: hovered)
                    }
                }
                .frame(height: 220)
                .accessibilityLabel(String(localized: "a11y.charts.activity.timeline"))
                .accessibilityHint(String(localized: "a11y.charts.timeline.tap.hint"))
            }
        }
        .padding()
        .background(.background.secondary, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
    }

    // MARK: - Risk Distribution (Donut)

    private var riskDistributionChart: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label(String(localized: "charts.risk.distribution"), systemImage: "chart.pie")
                .font(.headline)

            if riskDistribution.allSatisfy({ $0.count == 0 }) {
                ContentUnavailableView {
                    Label(String(localized: "charts.empty.risk.title"), systemImage: "chart.pie")
                } description: {
                    Text(String(localized: "charts.empty.risk.description"))
                }
                .frame(height: 200)
            } else {
                Chart(riskDistribution) { item in
                    SectorMark(
                        angle: .value(String(localized: "charts.axis.count"), item.count),
                        innerRadius: .ratio(0.6),
                        outerRadius: sectorOuterRadius(for: item),
                        angularInset: 1.5
                    )
                    .foregroundStyle(by: .value(String(localized: "charts.risk"), item.level))
                    .cornerRadius(4)
                    .opacity(sectorOpacity(for: item))
                }
                .chartForegroundStyleScale(riskColorMapping)
                .chartAngleSelection(value: $selectedAngleValue)
                .onChange(of: selectedAngleValue) { _, newValue in
                    handleDonutSelection(angleValue: newValue)
                }
                .overlay {
                    donutCenterOverlay
                }
                .frame(height: 200)
                .accessibilityLabel(String(localized: "a11y.charts.risk.distribution"))
                .accessibilityHint(String(localized: "a11y.charts.risk.tap.hint"))
            }
        }
        .padding()
        .background(.background.secondary, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
    }

    // MARK: - Event Type Distribution

    private var eventTypeChart: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label(String(localized: "charts.event.types"), systemImage: "chart.bar")
                .font(.headline)

            if eventTypeCounts.allSatisfy({ $0.count == 0 }) {
                ContentUnavailableView {
                    Label(String(localized: "charts.empty.events.title"), systemImage: "chart.bar")
                } description: {
                    Text(String(localized: "charts.empty.events.description"))
                }
                .frame(height: 200)
            } else {
                Chart(eventTypeCounts) { item in
                    BarMark(
                        x: .value(String(localized: "charts.axis.count"), item.count),
                        y: .value(String(localized: "charts.axis.type"), item.type)
                    )
                    .foregroundStyle(eventTypeBarColor(for: item))
                    .cornerRadius(4)
                    .opacity(eventTypeBarOpacity(for: item))
                    .annotation(position: .trailing, alignment: .leading, spacing: 4) {
                        Text(verbatim: "\(item.count)")
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    }
                }
                .chartOverlay { chart in
                    GeometryReader { geometry in
                        Rectangle()
                            .fill(.clear)
                            .contentShape(Rectangle())
                            .onTapGesture { location in
                                handleEventTypeTap(at: location, chart: chart, geometry: geometry)
                            }
                    }
                }
                .frame(height: 200)
                .accessibilityLabel(String(localized: "a11y.charts.event.types"))
                .accessibilityHint(String(localized: "a11y.charts.eventType.tap.hint"))
            }
        }
        .padding()
        .background(.background.secondary, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
    }

    // MARK: - Timeline Interaction

    private func handleTimelineTap(at location: CGPoint, chart: ChartProxy, geometry: GeometryProxy) {
        let plotFrame = geometry[chart.plotFrame!]
        let relativeX = location.x - plotFrame.origin.x
        guard let tappedDate: Date = chart.value(atX: relativeX) else { return }

        let bucketStart = roundToBucketStart(tappedDate)
        let bucketEnd = bucketStart.addingTimeInterval(Double(bucketMinutes) * 60)

        viewModel.selectedTimelineBucket = bucketStart
        viewModel.applyChartFilter(timeRange: bucketStart...bucketEnd)
    }

    private func handleTimelineHover(at location: CGPoint, chart: ChartProxy, geometry: GeometryProxy) {
        let plotFrame = geometry[chart.plotFrame!]
        let relativeX = location.x - plotFrame.origin.x
        guard let hoveredDate: Date = chart.value(atX: relativeX) else {
            hoveredTimelinePoint = nil
            return
        }

        let bucketStart = roundToBucketStart(hoveredDate)
        hoveredTimelinePoint = timelineData.first { point in
            Calendar.current.isDate(point.timestamp, equalTo: bucketStart, toGranularity: timelineUnit)
        }
        hoverLocation = location
    }

    private func roundToBucketStart(_ date: Date) -> Date {
        let calendar = Calendar.current
        switch selectedPeriod {
        case .day:
            return calendar.dateInterval(of: .hour, for: date)?.start ?? date
        case .week, .month:
            return calendar.startOfDay(for: date)
        }
    }

    private func timelineBarOpacity(for point: ChartDataPoint) -> Double {
        guard let selected = viewModel.selectedTimelineBucket else { return 1.0 }
        return Calendar.current.isDate(point.timestamp, equalTo: selected, toGranularity: timelineUnit) ? 1.0 : 0.3
    }

    private func timelineTooltip(for point: ChartDataPoint) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(point.timestamp, style: .date)
                .font(.caption2.bold())
            Text(point.timestamp, style: .time)
                .font(.caption2)
                .foregroundStyle(.secondary)
            Divider()
            HStack(spacing: 8) {
                VStack(alignment: .leading, spacing: 2) {
                    tooltipRow(label: String(localized: "charts.tooltip.risk.critical"), count: point.critical, color: AppColors.riskColor(.critical))
                    tooltipRow(label: String(localized: "charts.tooltip.risk.high"), count: point.high, color: AppColors.riskColor(.high))
                }
                VStack(alignment: .leading, spacing: 2) {
                    tooltipRow(label: String(localized: "charts.tooltip.risk.medium"), count: point.medium, color: AppColors.riskColor(.medium))
                    tooltipRow(label: String(localized: "charts.tooltip.risk.low"), count: point.low, color: AppColors.riskColor(.low))
                }
            }
            Divider()
            HStack {
                Text(String(localized: "charts.tooltip.total"))
                    .font(.caption2.bold())
                Spacer()
                Text(verbatim: "\(point.total)")
                    .font(.caption2.bold())
            }
        }
        .padding(8)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 8))
        .frame(width: 160)
        .position(x: hoverLocation.x.clamped(to: 80...300), y: 20)
    }

    private func tooltipRow(label: String, count: Int, color: Color) -> some View {
        HStack(spacing: 4) {
            Circle().fill(color).frame(width: 6, height: 6)
            Text(label).font(.caption2)
            Spacer()
            Text(verbatim: "\(count)").font(.caption2.bold())
        }
    }

    // MARK: - Donut Interaction

    private func handleDonutSelection(angleValue: Int?) {
        guard let angleValue else {
            viewModel.selectedRiskSector = nil
            return
        }

        // Walk through cumulative angles to find which sector was selected
        var cumulativeTotal = 0
        let riskLevels: [(level: RiskLevel, count: Int)] = [
            (.critical, viewModel.activitySummary.criticalCount),
            (.high, viewModel.activitySummary.highCount),
            (.medium, viewModel.activitySummary.mediumCount),
            (.low, viewModel.activitySummary.lowCount),
        ]

        for item in riskLevels {
            cumulativeTotal += item.count
            if angleValue <= cumulativeTotal {
                viewModel.selectedRiskSector = item.level
                viewModel.applyChartFilter(riskLevel: item.level)
                return
            }
        }
    }

    private func sectorOuterRadius(for item: RiskDistributionItem) -> MarkDimension {
        guard let selectedRisk = viewModel.selectedRiskSector else { return .ratio(0.95) }
        let isSelected = item.level == selectedRisk.label
        return isSelected ? .ratio(1.0) : .ratio(0.95)
    }

    private func sectorOpacity(for item: RiskDistributionItem) -> Double {
        guard let selectedRisk = viewModel.selectedRiskSector else { return 1.0 }
        return item.level == selectedRisk.label ? 1.0 : 0.4
    }

    @ViewBuilder
    private var donutCenterOverlay: some View {
        if let selectedRisk = viewModel.selectedRiskSector {
            let count = riskCount(for: selectedRisk)
            let total = viewModel.activitySummary.totalEvents
            let percentage = total > 0 ? Double(count) / Double(total) * 100 : 0

            VStack(spacing: 2) {
                Text(verbatim: "\(count)")
                    .font(.title2.bold())
                Text(verbatim: String(format: "%.0f%%", percentage))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Text(selectedRisk.label)
                    .font(.caption2)
                    .foregroundStyle(AppColors.riskColor(selectedRisk))
            }
        }
    }

    private func riskCount(for level: RiskLevel) -> Int {
        switch level {
        case .critical: return viewModel.activitySummary.criticalCount
        case .high: return viewModel.activitySummary.highCount
        case .medium: return viewModel.activitySummary.mediumCount
        case .low: return viewModel.activitySummary.lowCount
        }
    }

    // MARK: - Event Type Interaction

    private func handleEventTypeTap(at location: CGPoint, chart: ChartProxy, geometry: GeometryProxy) {
        let plotFrame = geometry[chart.plotFrame!]
        let relativeY = location.y - plotFrame.origin.y
        guard let tappedType: String = chart.value(atY: relativeY) else { return }

        if let eventTypeFilter = eventTypeFilterFromLabel(tappedType) {
            viewModel.selectedEventTypeBar = eventTypeFilter
            viewModel.applyChartFilter(eventType: eventTypeFilter)
        }
    }

    private func eventTypeFilterFromLabel(_ label: String) -> EventTypeFilter? {
        let mapping: [(label: String, filter: EventTypeFilter)] = [
            (String(localized: "eventType.command"), .command),
            (String(localized: "eventType.file"), .fileAccess),
            (String(localized: "eventType.network"), .network),
            (String(localized: "eventType.process"), .process),
        ]
        return mapping.first { $0.label == label }?.filter
    }

    private func eventTypeBarOpacity(for item: EventTypeCount) -> Double {
        guard let selectedType = viewModel.selectedEventTypeBar else { return 1.0 }
        if let filter = eventTypeFilterFromLabel(item.type) {
            return filter == selectedType ? 1.0 : 0.3
        }
        return 1.0
    }

    private func eventTypeBarColor(for item: EventTypeCount) -> AnyShapeStyle {
        guard let selectedType = viewModel.selectedEventTypeBar,
              let filter = eventTypeFilterFromLabel(item.type),
              filter == selectedType else {
            return AnyShapeStyle(.blue.gradient)
        }
        return AnyShapeStyle(.blue.gradient)
    }

    // MARK: - Data

    private var timelineUnit: Calendar.Component {
        switch selectedPeriod {
        case .day: return .hour
        case .week: return .day
        case .month: return .day
        }
    }

    private var timelineData: [ChartDataPoint] {
        viewModel.chartData
    }

    private var riskDistribution: [RiskDistributionItem] {
        let summary = viewModel.activitySummary
        return [
            RiskDistributionItem(level: String(localized: "risk.critical"), count: summary.criticalCount),
            RiskDistributionItem(level: String(localized: "risk.high"), count: summary.highCount),
            RiskDistributionItem(level: String(localized: "risk.medium"), count: summary.mediumCount),
            RiskDistributionItem(level: String(localized: "risk.low"), count: summary.lowCount),
        ]
    }

    private var eventTypeCounts: [EventTypeCount] {
        let events = viewModel.filteredEvents
        let commandCount = events.filter { if case .command = $0.eventType { return true }; return false }.count
        let fileCount = events.filter { if case .fileAccess = $0.eventType { return true }; return false }.count
        let networkCount = events.filter { if case .network = $0.eventType { return true }; return false }.count
        let processCount = events.filter { if case .process = $0.eventType { return true }; return false }.count
        return [
            EventTypeCount(type: String(localized: "eventType.command"), count: commandCount),
            EventTypeCount(type: String(localized: "eventType.file"), count: fileCount),
            EventTypeCount(type: String(localized: "eventType.network"), count: networkCount),
            EventTypeCount(type: String(localized: "eventType.process"), count: processCount),
        ]
    }

    private var riskColorMapping: KeyValuePairs<String, Color> {
        let criticalColor = contrast == .increased ? AppColors.riskColorHighContrast(.critical) : AppColors.riskColor(.critical)
        let highColor = contrast == .increased ? AppColors.riskColorHighContrast(.high) : AppColors.riskColor(.high)
        let mediumColor = contrast == .increased ? AppColors.riskColorHighContrast(.medium) : AppColors.riskColor(.medium)
        let lowColor = contrast == .increased ? AppColors.riskColorHighContrast(.low) : AppColors.riskColor(.low)
        return [
            String(localized: "risk.critical"): criticalColor,
            String(localized: "risk.high"): highColor,
            String(localized: "risk.medium"): mediumColor,
            String(localized: "risk.low"): lowColor,
        ]
    }

    // MARK: - Chart Data Loading

    private var bucketMinutes: UInt32 {
        switch selectedPeriod {
        case .day: return 60
        case .week: return 360
        case .month: return 1440
        }
    }

    private func loadChartDataForPeriod() {
        viewModel.loadChartData(bucketMinutes: bucketMinutes)
    }
}

// MARK: - Chart Period

enum ChartPeriod: String, CaseIterable {
    case day, week, month

    var label: String {
        switch self {
        case .day: return String(localized: "charts.period.24h")
        case .week: return String(localized: "charts.period.7d")
        case .month: return String(localized: "charts.period.30d")
        }
    }
}

// MARK: - CGFloat Clamped Extension

private extension Comparable {
    func clamped(to range: ClosedRange<Self>) -> Self {
        min(max(self, range.lowerBound), range.upperBound)
    }
}
