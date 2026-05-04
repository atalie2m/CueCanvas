import SwiftUI

@main
struct CueCanvasHostApp: App {
    @StateObject private var runtime = RuntimeController()

    var body: some Scene {
        WindowGroup("CueCanvas", id: "main") {
            ContentView(runtime: runtime)
                .frame(minWidth: 1100, minHeight: 720)
        }
        .commands {
            CommandGroup(replacing: .newItem) {}
            CommandGroup(after: .newItem) {
                Button("Open...") {
                    runtime.openProjectWithPanel()
                }
                .keyboardShortcut("o", modifiers: .command)

                Button("Save") {
                    runtime.saveCurrentProject()
                }
                .keyboardShortcut("s", modifiers: .command)

                Button("Save As...") {
                    runtime.saveProjectWithPanel()
                }
                .keyboardShortcut("s", modifiers: [.command, .shift])
            }
            CommandGroup(after: .appInfo) {
                Button("Restart Runtime") {
                    runtime.restart()
                }
                .keyboardShortcut("r", modifiers: [.command, .shift])
            }
        }

        Settings {
            SettingsView(runtime: runtime)
        }
    }
}
