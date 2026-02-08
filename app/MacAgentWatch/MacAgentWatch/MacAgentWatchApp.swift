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
                .preferredColorScheme(viewModel.selectedTheme.colorScheme)
        }
        .commands {
            CommandMenu(String(localized: "menu.monitor")) {
                Button(String(localized: "menu.monitor.start")) {
                    viewModel.startMonitoring()
                }
                .keyboardShortcut("m", modifiers: [.command, .shift])
                .disabled(viewModel.isMonitoring)

                Button(String(localized: "menu.monitor.stop")) {
                    viewModel.stopMonitoring()
                }
                .keyboardShortcut(".", modifiers: [.command, .shift])
                .disabled(!viewModel.isMonitoring)

                Button(String(localized: "menu.monitor.restart")) {
                    viewModel.restartMonitoring()
                }
                .keyboardShortcut("r", modifiers: [.command, .shift])
                .disabled(!viewModel.isMonitoring)

                Divider()

                Button(String(localized: "menu.monitor.refresh")) {
                    viewModel.loadInitialData()
                }
                .keyboardShortcut("r", modifiers: .command)
            }

            CommandGroup(after: .toolbar) {
                Button(String(localized: "menu.view.events")) {
                    viewModel.selectedTab = .events
                }
                .keyboardShortcut("1", modifiers: .command)

                Button(String(localized: "menu.view.livelog")) {
                    viewModel.selectedTab = .liveLog
                }
                .keyboardShortcut("2", modifiers: .command)

                Button(String(localized: "menu.view.charts")) {
                    viewModel.selectedTab = .charts
                }
                .keyboardShortcut("3", modifiers: .command)
            }
        }

        Settings {
            SettingsView(viewModel: viewModel)
                .preferredColorScheme(viewModel.selectedTheme.colorScheme)
        }
    }
}
