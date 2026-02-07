import SwiftUI
import Charts

struct ChartsView: View {
    @Bindable var viewModel: MonitoringViewModel
    @State private var selectedPeriod: ChartPeriod = .day
    @Environment(\.colorSchemeContrast) private var contrast

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

                    BarMark(
                        x: .value(String(localized: "charts.axis.time"), point.timestamp, unit: timelineUnit),
                        y: .value(String(localized: "charts.axis.count"), point.high)
                    )
                    .foregroundStyle(by: .value(String(localized: "charts.risk"), String(localized: "risk.high")))

                    BarMark(
                        x: .value(String(localized: "charts.axis.time"), point.timestamp, unit: timelineUnit),
                        y: .value(String(localized: "charts.axis.count"), point.medium)
                    )
                    .foregroundStyle(by: .value(String(localized: "charts.risk"), String(localized: "risk.medium")))

                    BarMark(
                        x: .value(String(localized: "charts.axis.time"), point.timestamp, unit: timelineUnit),
                        y: .value(String(localized: "charts.axis.count"), point.low)
                    )
                    .foregroundStyle(by: .value(String(localized: "charts.risk"), String(localized: "risk.low")))
                }
                .chartForegroundStyleScale(riskColorMapping)
                .frame(height: 220)
                .accessibilityLabel(String(localized: "a11y.charts.activity.timeline"))
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
                        angularInset: 1.5
                    )
                    .foregroundStyle(by: .value(String(localized: "charts.risk"), item.level))
                    .cornerRadius(4)
                }
                .chartForegroundStyleScale(riskColorMapping)
                .frame(height: 200)
                .accessibilityLabel(String(localized: "a11y.charts.risk.distribution"))
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
                    .foregroundStyle(.blue.gradient)
                    .cornerRadius(4)
                    .annotation(position: .trailing, alignment: .leading, spacing: 4) {
                        Text(verbatim: "\(item.count)")
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    }
                }
                .frame(height: 200)
                .accessibilityLabel(String(localized: "a11y.charts.event.types"))
            }
        }
        .padding()
        .background(.background.secondary, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
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
