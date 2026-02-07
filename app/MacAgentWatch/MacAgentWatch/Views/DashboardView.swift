import SwiftUI

enum DetailTab: String, CaseIterable {
    case events, liveLog, charts

    var label: String {
        switch self {
        case .events: return String(localized: "tab.events")
        case .liveLog: return String(localized: "tab.livelog")
        case .charts: return String(localized: "tab.charts")
        }
    }

    var icon: String {
        switch self {
        case .events: return "list.bullet"
        case .liveLog: return "scroll"
        case .charts: return "chart.bar.xaxis"
        }
    }
}

struct DashboardView: View {
    @Bindable var viewModel: MonitoringViewModel
    @State private var selectedTab: DetailTab = .events

    var body: some View {
        NavigationSplitView {
            SessionListView(viewModel: viewModel)
                .navigationSplitViewColumnWidth(min: 200, ideal: 240, max: 300)
        } detail: {
            detailContent
        }
        .inspector(isPresented: showInspector) {
            if let event = viewModel.selectedEvent {
                EventDetailView(event: event) {
                    viewModel.selectedEvent = nil
                }
            }
        }
        .navigationTitle(String(localized: "app.name"))
        .frame(minWidth: 800, minHeight: 500)
    }

    private var showInspector: Binding<Bool> {
        Binding(
            get: { viewModel.selectedEvent != nil },
            set: { if !$0 { viewModel.selectedEvent = nil } }
        )
    }

    // MARK: - Detail Content

    private var detailContent: some View {
        VStack(spacing: 0) {
            ActivityCardsView(summary: viewModel.activitySummary)
                .padding()

            detailTabPicker
                .padding(.horizontal)
                .padding(.bottom, 8)

            Divider()

            switch selectedTab {
            case .events:
                eventsContent
            case .liveLog:
                LiveLogView(viewModel: viewModel)
            case .charts:
                ChartsView(viewModel: viewModel)
            }
        }
    }

    // MARK: - Detail Tab Picker

    private var detailTabPicker: some View {
        HStack(spacing: 12) {
            Picker(String(localized: "dashboard.tab"), selection: $selectedTab) {
                ForEach(DetailTab.allCases, id: \.self) { tab in
                    Label(tab.label, systemImage: tab.icon).tag(tab)
                }
            }
            .pickerStyle(.segmented)
            .frame(maxWidth: 360)
            .accessibilityLabel(String(localized: "a11y.dashboard.tab.picker"))

            Spacer()
        }
    }

    // MARK: - Events Content

    private var eventsContent: some View {
        VStack(spacing: 0) {
            FilterBarView(
                filterRiskLevel: $viewModel.filterRiskLevel,
                searchQuery: $viewModel.searchQuery,
                eventTypeFilter: $viewModel.eventTypeFilter,
                dateRangePreset: $viewModel.dateRangePreset,
                customStartDate: $viewModel.customStartDate,
                customEndDate: $viewModel.customEndDate,
                filteredCount: viewModel.filteredEvents.count
            )
            .padding(.horizontal)
            .padding(.vertical, 8)

            Divider()

            eventsList
        }
    }

    // MARK: - Events List

    private var eventsList: some View {
        List(viewModel.filteredEvents, selection: selectedEventID) { event in
            EventRowView(event: event)
                .tag(event.id)
        }
        .listStyle(.inset(alternatesRowBackgrounds: true))
        .accessibilityLabel(Text("a11y.events.list"))
        .accessibilityHint(String(localized: "a11y.events.list.hint"))
    }

    private var selectedEventID: Binding<String?> {
        Binding(
            get: { viewModel.selectedEvent?.id },
            set: { newID in
                viewModel.selectedEvent = viewModel.filteredEvents.first { $0.id == newID }
            }
        )
    }
}
