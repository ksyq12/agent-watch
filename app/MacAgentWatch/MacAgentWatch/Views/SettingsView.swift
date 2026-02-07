import SwiftUI

struct SettingsView: View {
    @Bindable var viewModel: MonitoringViewModel

    var body: some View {
        TabView {
            GeneralSettingsTab(viewModel: viewModel)
                .tabItem {
                    Label(String(localized: "settings.tab.general"), systemImage: "gearshape")
                }

            MonitoringSettingsTab(viewModel: viewModel)
                .tabItem {
                    Label(String(localized: "settings.tab.monitoring"), systemImage: "eye")
                }

            SensitiveFilesSettingsTab(viewModel: viewModel)
                .tabItem {
                    Label(String(localized: "settings.tab.sensitive"), systemImage: "lock.shield")
                }

            NotificationSettingsTab(viewModel: viewModel)
                .tabItem {
                    Label(String(localized: "settings.tab.notifications"), systemImage: "bell")
                }
        }
        .frame(width: 500, height: 400)
    }
}

// MARK: - General Settings

struct GeneralSettingsTab: View {
    @Bindable var viewModel: MonitoringViewModel

    var body: some View {
        Form {
            Section(String(localized: "settings.appearance.title")) {
                Picker(String(localized: "settings.appearance.title"), selection: $viewModel.selectedTheme) {
                    ForEach(AppThemeMode.allCases, id: \.self) { theme in
                        Text(theme.localizedName).tag(theme)
                    }
                }
                .pickerStyle(.segmented)
            }

            Section(String(localized: "settings.general.logging")) {
                Toggle(String(localized: "settings.general.logging.enabled"), isOn: $viewModel.loggingEnabled)

                Stepper(
                    value: $viewModel.logRetentionDays, in: 1...365
                ) {
                    HStack {
                        Text(String(localized: "settings.general.logging.retention"))
                        Spacer()
                        Text("\(viewModel.logRetentionDays)")
                            .foregroundStyle(.secondary)
                            .monospacedDigit()
                    }
                }

                Picker(String(localized: "settings.general.format"), selection: $viewModel.defaultFormat) {
                    Text(String(localized: "settings.general.format.pretty")).tag("pretty")
                    Text(String(localized: "settings.general.format.json")).tag("json")
                    Text(String(localized: "settings.general.format.compact")).tag("compact")
                }
            }

            HStack {
                Spacer()
                Button(String(localized: "action.save")) {
                    viewModel.saveSettings()
                }
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding()
    }
}

// MARK: - Monitoring Settings

struct MonitoringSettingsTab: View {
    @Bindable var viewModel: MonitoringViewModel
    @State private var newPath: String = ""

    var body: some View {
        Form {
            Section(String(localized: "settings.monitoring.fsevents")) {
                Toggle(String(localized: "settings.monitoring.fsevents.enabled"), isOn: $viewModel.fsEventsEnabled)
            }

            Section(String(localized: "settings.monitoring.network")) {
                Toggle(String(localized: "settings.monitoring.network.enabled"), isOn: $viewModel.networkMonitorEnabled)
            }

            Section(String(localized: "settings.monitoring.polling")) {
                HStack {
                    Text(String(localized: "settings.monitoring.polling"))
                    Spacer()
                    TextField("", value: $viewModel.trackingPollMs, format: .number)
                        .frame(width: 80)
                        .textFieldStyle(.roundedBorder)
                        .multilineTextAlignment(.trailing)
                    Text("ms")
                        .foregroundStyle(.secondary)
                }
            }

            Section(String(localized: "settings.monitoring.watchpaths")) {
                ForEach(viewModel.watchPaths, id: \.self) { path in
                    HStack {
                        Text(path)
                            .lineLimit(1)
                            .truncationMode(.middle)
                        Spacer()
                        Button(role: .destructive) {
                            viewModel.watchPaths.removeAll { $0 == path }
                        } label: {
                            Image(systemName: "minus.circle")
                        }
                        .buttonStyle(.borderless)
                    }
                }

                HStack {
                    TextField(String(localized: "settings.monitoring.watchpaths.add"), text: $newPath)
                        .textFieldStyle(.roundedBorder)
                    Button {
                        let trimmed = newPath.trimmingCharacters(in: .whitespaces)
                        if !trimmed.isEmpty {
                            viewModel.watchPaths.append(trimmed)
                            newPath = ""
                        }
                    } label: {
                        Image(systemName: "plus.circle")
                    }
                    .buttonStyle(.borderless)
                    .disabled(newPath.trimmingCharacters(in: .whitespaces).isEmpty)
                }
            }

            HStack {
                Spacer()
                Button(String(localized: "action.save")) {
                    viewModel.saveSettings()
                }
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding()
    }
}

// MARK: - Sensitive Files Settings

struct SensitiveFilesSettingsTab: View {
    @Bindable var viewModel: MonitoringViewModel
    @State private var newPattern: String = ""
    @State private var newDomain: String = ""

    private let defaultPatterns: [String] = [".env", ".env.*", "*.pem", "*.key", "*credential*", "*secret*"]

    var body: some View {
        Form {
            Section(String(localized: "settings.sensitive.default")) {
                ForEach(defaultPatterns, id: \.self) { pattern in
                    Text(pattern)
                        .foregroundStyle(.secondary)
                }
            }

            Section(String(localized: "settings.sensitive.custom")) {
                let customPatterns = viewModel.sensitivePatterns.filter { !defaultPatterns.contains($0) }
                ForEach(customPatterns, id: \.self) { pattern in
                    HStack {
                        Text(pattern)
                        Spacer()
                        Button(String(localized: "settings.sensitive.remove"), role: .destructive) {
                            viewModel.sensitivePatterns.removeAll { $0 == pattern }
                        }
                        .buttonStyle(.borderless)
                    }
                }

                HStack {
                    TextField(String(localized: "settings.sensitive.add"), text: $newPattern)
                        .textFieldStyle(.roundedBorder)
                    Button {
                        let trimmed = newPattern.trimmingCharacters(in: .whitespaces)
                        if !trimmed.isEmpty {
                            viewModel.sensitivePatterns.append(trimmed)
                            newPattern = ""
                        }
                    } label: {
                        Image(systemName: "plus.circle")
                    }
                    .buttonStyle(.borderless)
                    .disabled(newPattern.trimmingCharacters(in: .whitespaces).isEmpty)
                }
            }

            Section(String(localized: "settings.sensitive.whitelist")) {
                ForEach(viewModel.networkWhitelist, id: \.self) { domain in
                    HStack {
                        Text(domain)
                        Spacer()
                        Button(String(localized: "settings.sensitive.remove"), role: .destructive) {
                            viewModel.networkWhitelist.removeAll { $0 == domain }
                        }
                        .buttonStyle(.borderless)
                    }
                }

                HStack {
                    TextField(String(localized: "settings.sensitive.whitelist.add"), text: $newDomain)
                        .textFieldStyle(.roundedBorder)
                    Button {
                        let trimmed = newDomain.trimmingCharacters(in: .whitespaces)
                        if !trimmed.isEmpty {
                            viewModel.networkWhitelist.append(trimmed)
                            newDomain = ""
                        }
                    } label: {
                        Image(systemName: "plus.circle")
                    }
                    .buttonStyle(.borderless)
                    .disabled(newDomain.trimmingCharacters(in: .whitespaces).isEmpty)
                }
            }

            HStack {
                Spacer()
                Button(String(localized: "action.save")) {
                    viewModel.saveSettings()
                }
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding()
    }
}

// MARK: - Notification Settings

struct NotificationSettingsTab: View {
    @Bindable var viewModel: MonitoringViewModel

    var body: some View {
        Form {
            Section {
                Toggle(String(localized: "settings.notifications.enabled"), isOn: $viewModel.notificationsEnabled)

                Picker(String(localized: "settings.notifications.minlevel"), selection: $viewModel.notificationMinRiskLevel) {
                    ForEach(RiskLevel.allCases, id: \.self) { level in
                        Text(level.label).tag(level)
                    }
                }

                Toggle(String(localized: "settings.notifications.sound"), isOn: $viewModel.notificationSoundEnabled)
                Toggle(String(localized: "settings.notifications.badge"), isOn: $viewModel.notificationBadgeEnabled)
            }

            HStack {
                Spacer()
                Button(String(localized: "action.save")) {
                    viewModel.saveSettings()
                }
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding()
    }
}
