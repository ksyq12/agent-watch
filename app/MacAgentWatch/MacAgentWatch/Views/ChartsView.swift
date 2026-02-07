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
        .padding()
        .background(.background.secondary, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
    }

    // MARK: - Risk Distribution (Donut)

    private var riskDistributionChart: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label(String(localized: "charts.risk.distribution"), systemImage: "chart.pie")
                .font(.headline)

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
        .padding()
        .background(.background.secondary, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
    }

    // MARK: - Event Type Distribution

    private var eventTypeChart: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label(String(localized: "charts.event.types"), systemImage: "chart.bar")
                .font(.headline)

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
        Self.generateMockTimeline(for: selectedPeriod)
    }

    private var riskDistribution: [RiskDistributionItem] {
        let summary = viewModel.activitySummary
        let critical = summary.criticalCount > 0 ? summary.criticalCount : 3
        let high = summary.highCount > 0 ? summary.highCount : 12
        let medium = summary.mediumCount > 0 ? summary.mediumCount : 28
        let low = summary.lowCount > 0 ? summary.lowCount : 57
        return [
            RiskDistributionItem(level: String(localized: "risk.critical"), count: critical),
            RiskDistributionItem(level: String(localized: "risk.high"), count: high),
            RiskDistributionItem(level: String(localized: "risk.medium"), count: medium),
            RiskDistributionItem(level: String(localized: "risk.low"), count: low),
        ]
    }

    private var eventTypeCounts: [EventTypeCount] {
        [
            EventTypeCount(type: String(localized: "eventType.command"), count: 42),
            EventTypeCount(type: String(localized: "eventType.file"), count: 35),
            EventTypeCount(type: String(localized: "eventType.network"), count: 18),
            EventTypeCount(type: String(localized: "eventType.process"), count: 12),
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

    // MARK: - Mock Data Generation

    private static func generateMockTimeline(for period: ChartPeriod) -> [ChartDataPoint] {
        let calendar = Calendar.current
        let now = Date()
        var points: [ChartDataPoint] = []

        let count: Int
        let component: Calendar.Component
        switch period {
        case .day:
            count = 24
            component = .hour
        case .week:
            count = 7
            component = .day
        case .month:
            count = 30
            component = .day
        }

        for i in (0..<count).reversed() {
            guard let date = calendar.date(byAdding: component, value: -i, to: now) else { continue }
            let low = Int.random(in: 5...20)
            let medium = Int.random(in: 2...10)
            let high = Int.random(in: 0...4)
            let critical = Int.random(in: 0...2)
            points.append(ChartDataPoint(
                timestamp: date,
                total: low + medium + high + critical,
                critical: critical,
                high: high,
                medium: medium,
                low: low
            ))
        }
        return points
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
