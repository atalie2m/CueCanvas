import AppKit
import Darwin
import Foundation
import Security
import SwiftUI

@MainActor
final class RuntimeController: ObservableObject {
    @Published var selectedSection: AppSection = .operatorCockpit
    @Published var status: RuntimeStatus = .stopped
    @Published var lastMessage = "Runtime has not been started."
    @Published var port = 4317
    @Published var launchesAutomatically = true
    @Published private(set) var credentials = RuntimeCredentials()
    @Published private(set) var currentPackageURL: URL?

    private let launcher: RuntimeProcessLaunching
    private let keychain: KeychainStoring
    private let projectClient: RuntimeProjectClienting
    private var process: RuntimeProcessHandle?
    private var healthTask: Task<Void, Never>?

    init(
        launcher: RuntimeProcessLaunching = ProcessRuntimeLauncher(),
        keychain: KeychainStoring = SystemKeychainStore(service: "com.atalie2m.CueCanvas"),
        projectClient: RuntimeProjectClienting = RuntimeProjectClient()
    ) {
        self.launcher = launcher
        self.keychain = keychain
        self.projectClient = projectClient
        credentials.overlayToken = (try? keychain.read(account: KeychainAccount.overlayToken.rawValue)) ?? ""
    }

    var editorURL: URL {
        var components = URLComponents(string: "http://127.0.0.1:\(port)/editor")!
        var queryItems: [URLQueryItem] = []
        if !credentials.editorToken.isEmpty {
            queryItems.append(URLQueryItem(name: "editorToken", value: credentials.editorToken))
        }
        if !credentials.overlayToken.isEmpty {
            queryItems.append(URLQueryItem(name: "overlayToken", value: credentials.overlayToken))
        }
        components.queryItems = queryItems.isEmpty ? nil : queryItems
        return components.url ?? URL(string: "about:blank")!
    }

    var overlayURL: URL {
        var components = URLComponents(string: "http://127.0.0.1:\(port)/overlay/program")!
        if !credentials.overlayToken.isEmpty {
            components.queryItems = [
                URLQueryItem(name: "token", value: credentials.overlayToken)
            ]
        }
        return components.url!
    }

    var runtimeBaseURL: URL {
        URL(string: "http://127.0.0.1:\(port)")!
    }

    func start() {
        guard launchesAutomatically else {
            status = .stopped
            lastMessage = "Automatic runtime launch is disabled."
            return
        }
        guard process == nil || process?.isRunning == false else { return }

        port = FreePortPicker.pick(preferred: port)
        status = .starting
        lastMessage = "Starting Runtime on port \(port)."

        do {
            process = try launcher.launch(
                port: port,
                output: { [weak self] line in
                    Task { @MainActor in
                        self?.ingestRuntimeOutputLine(line)
                    }
                },
                termination: { [weak self] status in
                    Task { @MainActor in
                        self?.runtimeDidTerminate(status: status)
                    }
                }
            )
            startHealthPolling()
        } catch {
            status = .degraded
            lastMessage = "Runtime launch failed: \(error.localizedDescription)"
        }
    }

    func ingestRuntimeOutputLine(_ line: String) {
        if let port = line.runtimePort {
            self.port = port
            lastMessage = "Runtime is listening on port \(port)."
        } else if let token = line.removingPrefix("Editor token: ") {
            credentials.editorToken = token
            try? keychain.write(token, account: KeychainAccount.editorToken.rawValue)
            lastMessage = "Captured editor token from Runtime."
        } else if let token = line.removingPrefix("Overlay token: ") {
            credentials.overlayToken = token
            try? keychain.write(token, account: KeychainAccount.overlayToken.rawValue)
            lastMessage = "Captured overlay token from Runtime."
        } else if let urlText = line.removingPrefix("Program overlay URL: "),
                  let token = URLComponents(string: urlText)?
                    .queryItems?
                    .first(where: { $0.name == "token" })?
                    .value
        {
            credentials.overlayToken = token
            try? keychain.write(token, account: KeychainAccount.overlayToken.rawValue)
            lastMessage = "Captured overlay URL from Runtime."
        }

        if credentials.isComplete {
            status = .running
            lastMessage = "Runtime credentials captured."
        }
    }

    func restart() {
        stop()
        start()
    }

    func stop() {
        healthTask?.cancel()
        healthTask = nil
        process?.terminate()
        process = nil
        status = .stopped
        lastMessage = "Runtime stopped."
    }

    @MainActor
    func openProjectWithPanel() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.message = "Open a .cuecanvas package"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        openProject(at: url)
    }

    @MainActor
    func saveProjectWithPanel() {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = currentPackageURL?.lastPathComponent ?? "show.cuecanvas"
        panel.canCreateDirectories = true
        panel.message = "Save a .cuecanvas package"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        saveProject(to: url)
    }

    @MainActor
    func saveCurrentProject() {
        guard let currentPackageURL else {
            saveProjectWithPanel()
            return
        }
        saveProject(to: currentPackageURL)
    }

    func openProject(at url: URL) {
        guard credentials.hasEditorToken else {
            lastMessage = "Runtime editor token is not available."
            return
        }
        let client = projectClient
        let baseURL = runtimeBaseURL
        let token = credentials.editorToken
        Task {
            do {
                try await client.openProject(
                    packageURL: url,
                    runtimeBaseURL: baseURL,
                    editorToken: token
                )
                await MainActor.run {
                    currentPackageURL = url
                    lastMessage = "Opened \(url.lastPathComponent)."
                }
            } catch {
                await MainActor.run {
                    status = .degraded
                    lastMessage = "Open failed: \(error.localizedDescription)"
                }
            }
        }
    }

    func saveProject(to url: URL) {
        guard credentials.hasEditorToken else {
            lastMessage = "Runtime editor token is not available."
            return
        }
        let client = projectClient
        let baseURL = runtimeBaseURL
        let token = credentials.editorToken
        Task {
            do {
                try await client.saveProject(
                    packageURL: url,
                    runtimeBaseURL: baseURL,
                    editorToken: token
                )
                await MainActor.run {
                    currentPackageURL = url
                    lastMessage = "Saved \(url.lastPathComponent)."
                }
            } catch {
                await MainActor.run {
                    status = .degraded
                    lastMessage = "Save failed: \(error.localizedDescription)"
                }
            }
        }
    }

    private func startHealthPolling() {
        healthTask?.cancel()
        healthTask = Task { @MainActor [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(for: .seconds(2))
                await self?.pollHealth()
            }
        }
    }

    private func pollHealth() async {
        do {
            let (_, response) = try await URLSession.shared.data(from: runtimeBaseURL.appending(path: "health"))
            if (response as? HTTPURLResponse)?.statusCode == 200, credentials.isComplete {
                status = .running
            }
        } catch {
            if status == .running {
                status = .degraded
                lastMessage = "Runtime health check failed."
            }
        }
    }

    private func runtimeDidTerminate(status terminationStatus: Int32) {
        healthTask?.cancel()
        healthTask = nil
        process = nil
        status = terminationStatus == 0 ? .stopped : .degraded
        lastMessage = terminationStatus == 0
            ? "Runtime exited."
            : "Runtime exited with status \(terminationStatus)."
    }
}

struct RuntimeCredentials: Equatable {
    var editorToken = ""
    var overlayToken = ""

    var hasEditorToken: Bool {
        !editorToken.isEmpty
    }

    var isComplete: Bool {
        !editorToken.isEmpty && !overlayToken.isEmpty
    }
}

enum RuntimeStatus {
    case stopped
    case starting
    case running
    case degraded

    var title: String {
        switch self {
        case .stopped: "Stopped"
        case .starting: "Starting"
        case .running: "Running"
        case .degraded: "Degraded"
        }
    }

    var systemImage: String {
        switch self {
        case .stopped: "pause.circle"
        case .starting: "clock"
        case .running: "checkmark.circle"
        case .degraded: "exclamationmark.triangle"
        }
    }

    var color: Color {
        switch self {
        case .stopped: .secondary
        case .starting: .orange
        case .running: .green
        case .degraded: .red
        }
    }
}

protocol RuntimeProcessHandle: AnyObject {
    var isRunning: Bool { get }
    func terminate()
}

protocol RuntimeProcessLaunching {
    func launch(
        port: Int,
        output: @escaping (String) -> Void,
        termination: @escaping (Int32) -> Void
    ) throws -> RuntimeProcessHandle
}

final class ProcessRuntimeLauncher: RuntimeProcessLaunching {
    func launch(
        port: Int,
        output: @escaping (String) -> Void,
        termination: @escaping (Int32) -> Void
    ) throws -> RuntimeProcessHandle {
        let callbacks = RuntimeProcessCallbacks(output: output, termination: termination)
        let process = Process()
        process.executableURL = try runtimeExecutableURL()
        process.arguments = ["--port", "\(port)"]

        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = pipe
        pipe.fileHandleForReading.readabilityHandler = { handle in
            guard let text = String(data: handle.availableData, encoding: .utf8) else { return }
            text.split(separator: "\n").forEach { callbacks.output(String($0)) }
        }
        process.terminationHandler = { process in
            pipe.fileHandleForReading.readabilityHandler = nil
            callbacks.termination(process.terminationStatus)
        }

        try process.run()
        return ProcessHandle(process: process)
    }

    private func runtimeExecutableURL() throws -> URL {
        if let override = ProcessInfo.processInfo.environment["CUECANVAS_RUNTIME_BINARY"] {
            return URL(fileURLWithPath: override)
        }
        if let bundled = Bundle.main.url(forResource: "cuecanvas-runtime", withExtension: nil) {
            return bundled
        }
        let devPath = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
            .appending(path: "target/debug/cuecanvas-runtime")
        if FileManager.default.isExecutableFile(atPath: devPath.path) {
            return devPath
        }
        throw RuntimeLaunchError.runtimeBinaryMissing
    }
}

final class RuntimeProcessCallbacks: @unchecked Sendable {
    let output: (String) -> Void
    let termination: (Int32) -> Void

    init(output: @escaping (String) -> Void, termination: @escaping (Int32) -> Void) {
        self.output = output
        self.termination = termination
    }
}

final class ProcessHandle: RuntimeProcessHandle {
    private let process: Process

    init(process: Process) {
        self.process = process
    }

    var isRunning: Bool {
        process.isRunning
    }

    func terminate() {
        process.terminate()
    }
}

enum RuntimeLaunchError: LocalizedError {
    case runtimeBinaryMissing

    var errorDescription: String? {
        switch self {
        case .runtimeBinaryMissing:
            "cuecanvas-runtime was not found in the app bundle or target/debug."
        }
    }
}

protocol KeychainStoring {
    func read(account: String) throws -> String?
    func write(_ value: String, account: String) throws
    func delete(account: String) throws
}

enum KeychainAccount: String {
    case editorToken = "editor-session-token"
    case overlayToken = "overlay-token"
    case obsPassword = "obs-password"
    case externalInputToken = "external-input-token"
}

final class SystemKeychainStore: KeychainStoring {
    private let service: String

    init(service: String) {
        self.service = service
    }

    func read(account: String) throws -> String? {
        var query = baseQuery(account: account)
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne

        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        if status == errSecItemNotFound { return nil }
        guard status == errSecSuccess else { throw KeychainError.status(status) }
        guard let data = item as? Data else { return nil }
        return String(data: data, encoding: .utf8)
    }

    func write(_ value: String, account: String) throws {
        try delete(account: account)
        var query = baseQuery(account: account)
        query[kSecValueData as String] = Data(value.utf8)
        query[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else { throw KeychainError.status(status) }
    }

    func delete(account: String) throws {
        let status = SecItemDelete(baseQuery(account: account) as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw KeychainError.status(status)
        }
    }

    private func baseQuery(account: String) -> [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account
        ]
    }
}

enum KeychainError: LocalizedError {
    case status(OSStatus)

    var errorDescription: String? {
        switch self {
        case .status(let status):
            "Keychain returned status \(status)."
        }
    }
}

protocol RuntimeProjectClienting: Sendable {
    func openProject(packageURL: URL, runtimeBaseURL: URL, editorToken: String) async throws
    func saveProject(packageURL: URL, runtimeBaseURL: URL, editorToken: String) async throws
}

struct RuntimeProjectClient: RuntimeProjectClienting {
    func openProject(packageURL: URL, runtimeBaseURL: URL, editorToken: String) async throws {
        try await sendProjectRequest(
            path: "api/project/open",
            packageURL: packageURL,
            runtimeBaseURL: runtimeBaseURL,
            editorToken: editorToken
        )
    }

    func saveProject(packageURL: URL, runtimeBaseURL: URL, editorToken: String) async throws {
        try await sendProjectRequest(
            path: "api/project/save",
            packageURL: packageURL,
            runtimeBaseURL: runtimeBaseURL,
            editorToken: editorToken
        )
    }

    private func sendProjectRequest(
        path: String,
        packageURL: URL,
        runtimeBaseURL: URL,
        editorToken: String
    ) async throws {
        var request = URLRequest(url: runtimeBaseURL.appending(path: path))
        request.httpMethod = "POST"
        request.setValue(editorToken, forHTTPHeaderField: "X-CueCanvas-Editor-Token")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONEncoder().encode(ProjectPersistenceBody(packageDir: packageURL.path))
        let (_, response) = try await URLSession.shared.data(for: request)
        guard (response as? HTTPURLResponse)?.statusCode == 200 else {
            throw RuntimeProjectClientError.requestFailed
        }
    }
}

struct ProjectPersistenceBody: Encodable {
    let packageDir: String
}

enum RuntimeProjectClientError: LocalizedError {
    case requestFailed

    var errorDescription: String? {
        "Runtime project request failed."
    }
}

enum FreePortPicker {
    static func pick(preferred: Int) -> Int {
        if preferred > 0, isAvailable(preferred) {
            return preferred
        }
        let socketDescriptor = socket(AF_INET, SOCK_STREAM, 0)
        guard socketDescriptor >= 0 else { return 4317 }
        defer { close(socketDescriptor) }

        var address = sockaddr_in()
        address.sin_len = UInt8(MemoryLayout<sockaddr_in>.stride)
        address.sin_family = sa_family_t(AF_INET)
        address.sin_port = in_port_t(0).bigEndian
        address.sin_addr = in_addr(s_addr: inet_addr("127.0.0.1"))

        let bindStatus = withUnsafePointer(to: &address) {
            $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                bind(socketDescriptor, $0, socklen_t(MemoryLayout<sockaddr_in>.stride))
            }
        }
        guard bindStatus == 0 else { return 4317 }

        var length = socklen_t(MemoryLayout<sockaddr_in>.stride)
        let nameStatus = withUnsafeMutablePointer(to: &address) {
            $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                getsockname(socketDescriptor, $0, &length)
            }
        }
        guard nameStatus == 0 else { return 4317 }
        return Int(UInt16(bigEndian: address.sin_port))
    }

    private static func isAvailable(_ port: Int) -> Bool {
        let socketDescriptor = socket(AF_INET, SOCK_STREAM, 0)
        guard socketDescriptor >= 0 else { return false }
        defer { close(socketDescriptor) }

        var address = sockaddr_in()
        address.sin_len = UInt8(MemoryLayout<sockaddr_in>.stride)
        address.sin_family = sa_family_t(AF_INET)
        address.sin_port = in_port_t(port).bigEndian
        address.sin_addr = in_addr(s_addr: inet_addr("127.0.0.1"))

        return withUnsafePointer(to: &address) {
            $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                bind(socketDescriptor, $0, socklen_t(MemoryLayout<sockaddr_in>.stride))
            }
        } == 0
    }
}

private extension String {
    func removingPrefix(_ prefix: String) -> String? {
        guard hasPrefix(prefix) else { return nil }
        return String(dropFirst(prefix.count)).trimmingCharacters(in: .whitespacesAndNewlines)
    }

    var runtimePort: Int? {
        guard let urlText = removingPrefix("CueCanvas runtime listening on "),
              let url = URL(string: urlText)
        else {
            return nil
        }
        return url.port
    }
}
