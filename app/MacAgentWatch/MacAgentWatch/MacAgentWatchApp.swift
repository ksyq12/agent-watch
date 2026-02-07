import SwiftUI

@main
struct MacAgentWatchApp: App {
    @State private var viewModel = MonitoringViewModel()

    var body: some Scene {
        MenuBarExtra(String(localized: "app.name"), systemImage: "shield.checkered") {
            MenuBarView(viewModel: viewModel)
        }
        .menuBarExtraStyle(.window)

        WindowGroup(String(localized: "app.dashboard.title"), id: "dashboard") {
            DashboardView(viewModel: viewModel)
        }
    }
}
