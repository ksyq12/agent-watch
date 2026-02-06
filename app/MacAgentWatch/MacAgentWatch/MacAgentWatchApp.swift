import SwiftUI

@main
struct MacAgentWatchApp: App {
    @State private var viewModel = MonitoringViewModel()

    var body: some Scene {
        MenuBarExtra("MacAgentWatch", systemImage: "shield.checkered") {
            MenuBarView(viewModel: viewModel)
        }
        .menuBarExtraStyle(.window)

        WindowGroup("MacAgentWatch Dashboard", id: "dashboard") {
            DashboardView(viewModel: viewModel)
        }
    }
}
