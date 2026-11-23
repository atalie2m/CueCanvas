import SwiftUI

struct RuntimeStatusView: View {
    @ObservedObject var runtime: RuntimeController

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            HStack {
                RuntimeStatusBadge(status: runtime.status)
                Spacer()
                Button("Open") {
                    runtime.openProjectWithPanel()
                }
                Button("Save") {
                    runtime.saveCurrentProject()
                }
                Button("Restart") {
                    runtime.restart()
                }
            }

            Grid(alignment: .leading, horizontalSpacing: 18, verticalSpacing: 10) {
                GridRow {
                    Text("Runtime URL")
                        .foregroundStyle(.secondary)
                    Text(runtime.editorURL.absoluteString)
                        .textSelection(.enabled)
                }
                GridRow {
                    Text("Overlay URL")
                        .foregroundStyle(.secondary)
                    Text(runtime.overlayURL.absoluteString)
                        .textSelection(.enabled)
                }
                GridRow {
                    Text("Package")
                        .foregroundStyle(.secondary)
                    Text(runtime.currentPackageURL?.path ?? "Not saved")
                        .textSelection(.enabled)
                }
                GridRow {
                    Text("Last Message")
                        .foregroundStyle(.secondary)
                    Text(runtime.lastMessage)
                }
            }
            .font(.system(.body, design: .monospaced))

            Spacer()
        }
        .padding(24)
        .navigationTitle("Runtime Status")
    }
}

struct RuntimeStatusBadge: View {
    let status: RuntimeStatus

    var body: some View {
        Label(status.title, systemImage: status.systemImage)
            .font(.caption.weight(.semibold))
            .foregroundStyle(status.color)
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
            .background(.regularMaterial, in: Capsule())
    }
}

struct SettingsView: View {
    @ObservedObject var runtime: RuntimeController

    var body: some View {
        Form {
            TextField("Runtime Port", value: $runtime.port, format: .number)
            Toggle("Launch runtime automatically", isOn: $runtime.launchesAutomatically)
        }
        .padding()
        .frame(width: 420)
    }
}
