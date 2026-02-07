import SwiftUI

#if DEBUG

// MARK: - RTL Preview Helpers

struct RTLPreviewModifier: ViewModifier {
    func body(content: Content) -> some View {
        content
            .environment(\.layoutDirection, .rightToLeft)
            .environment(\.locale, Locale(identifier: "ar"))
    }
}

extension View {
    func rtlPreview() -> some View {
        modifier(RTLPreviewModifier())
    }
}

// MARK: - RTL Previews

#Preview("MenuBarView - RTL") {
    MenuBarView(viewModel: MonitoringViewModel())
        .rtlPreview()
}

#Preview("DashboardView - RTL") {
    DashboardView(viewModel: MonitoringViewModel())
        .rtlPreview()
        .frame(width: 900, height: 600)
}

#Preview("EventRowView - RTL") {
    let event = MonitoringEvent(
        id: "preview-rtl-1",
        timestamp: Date(),
        eventType: .command(command: "npm", args: ["install"], exitCode: 0),
        process: "node",
        pid: 1234,
        riskLevel: .medium,
        alert: false
    )
    List {
        EventRowView(event: event)
    }
    .rtlPreview()
}

#Preview("SessionListView - RTL") {
    SessionListView(viewModel: MonitoringViewModel())
        .rtlPreview()
        .frame(width: 280)
}

// MARK: - Dynamic Type Previews

#Preview("EventRowView - Large Text") {
    let event = MonitoringEvent(
        id: "preview-dt-1",
        timestamp: Date(),
        eventType: .command(command: "npm", args: ["install"], exitCode: 0),
        process: "node",
        pid: 1234,
        riskLevel: .high,
        alert: true
    )
    List {
        EventRowView(event: event)
    }
    .environment(\.dynamicTypeSize, .accessibility3)
}

#Preview("MenuBarView - Large Text") {
    MenuBarView(viewModel: MonitoringViewModel())
        .environment(\.dynamicTypeSize, .accessibility3)
}

#Preview("DashboardView - Large Text") {
    DashboardView(viewModel: MonitoringViewModel())
        .environment(\.dynamicTypeSize, .accessibility3)
        .frame(width: 1000, height: 700)
}

// MARK: - Reduce Motion Preview
// Note: accessibilityReduceMotion and colorSchemeContrast are read-only
// environment values in macOS 15+. Test these via System Settings instead.

#Preview("EventRowView - Reduce Motion") {
    let event = MonitoringEvent(
        id: "preview-rm-1",
        timestamp: Date(),
        eventType: .command(command: "rm", args: ["-rf", "/tmp"], exitCode: nil),
        process: "bash",
        pid: 5678,
        riskLevel: .critical,
        alert: true
    )
    List {
        EventRowView(event: event)
    }
}

// MARK: - High Contrast Preview

#Preview("DashboardView - High Contrast") {
    DashboardView(viewModel: MonitoringViewModel())
        .frame(width: 900, height: 600)
}

#Preview("EventRowView - High Contrast") {
    let event = MonitoringEvent(
        id: "preview-hc-1",
        timestamp: Date(),
        eventType: .network(host: "api.example.com", port: 443, protocol: "tcp"),
        process: "curl",
        pid: 9012,
        riskLevel: .high,
        alert: true
    )
    List {
        EventRowView(event: event)
    }
}

#endif
