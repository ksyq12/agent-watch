import SwiftUI

struct DashboardView: View {
    @Bindable var viewModel: MonitoringViewModel

    var body: some View {
        NavigationSplitView {
            SessionListView(viewModel: viewModel)
                .navigationSplitViewColumnWidth(min: 200, ideal: 240, max: 300)
        } detail: {
            detailContent
        }
        .navigationTitle(String(localized: "app.name"))
        .frame(minWidth: 800, minHeight: 500)
    }

    // MARK: - Detail Content

    private var detailContent: some View {
        VStack(spacing: 0) {
            ActivityCardsView(summary: viewModel.activitySummary)
                .padding()

            FilterBarView(
                filterRiskLevel: $viewModel.filterRiskLevel,
                filteredCount: viewModel.filteredEvents.count
            )
            .padding(.horizontal)
            .padding(.bottom, 8)

            Divider()

            eventsList
        }
    }

    // MARK: - Events List

    private var eventsList: some View {
        List(viewModel.filteredEvents) { event in
            EventRowView(event: event)
        }
        .listStyle(.inset(alternatesRowBackgrounds: true))
        .accessibilityLabel(Text("a11y.events.list"))
        .accessibilityHint(String(localized: "a11y.events.list.hint"))
    }
}
