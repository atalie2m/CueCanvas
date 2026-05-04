import SwiftUI
import WebKit

struct ContentView: View {
    @ObservedObject var runtime: RuntimeController

    var body: some View {
        NavigationSplitView {
            List(selection: $runtime.selectedSection) {
                Label("Operator Cockpit", systemImage: "rectangle.3.group")
                    .tag(AppSection.operatorCockpit)
                Label("Program Overlay", systemImage: "display")
                    .tag(AppSection.programOverlay)
                Label("Runtime Status", systemImage: "waveform.path.ecg")
                    .tag(AppSection.runtimeStatus)
            }
            .listStyle(.sidebar)
            .navigationTitle("CueCanvas")
        } detail: {
            switch runtime.selectedSection {
            case .operatorCockpit:
                RuntimeWebView(url: runtime.editorURL)
                    .overlay(alignment: .topTrailing) {
                        RuntimeStatusBadge(status: runtime.status)
                            .padding()
                    }
            case .programOverlay:
                RuntimeWebView(url: runtime.overlayURL)
            case .runtimeStatus:
                RuntimeStatusView(runtime: runtime)
            }
        }
        .task {
            runtime.start()
        }
    }
}

enum AppSection: String, CaseIterable, Identifiable {
    case operatorCockpit
    case programOverlay
    case runtimeStatus

    var id: String { rawValue }
}

struct RuntimeWebView: NSViewRepresentable {
    let url: URL

    func makeNSView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.limitsNavigationsToAppBoundDomains = false
        let webView = WKWebView(frame: .zero, configuration: configuration)
        webView.allowsBackForwardNavigationGestures = false
        return webView
    }

    func updateNSView(_ webView: WKWebView, context: Context) {
        if webView.url != url {
            webView.load(URLRequest(url: url))
        }
    }
}
