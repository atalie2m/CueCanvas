import Foundation
import XCTest
@testable import CueCanvasHost

@MainActor
final class RuntimeControllerTests: XCTestCase {
    func testRuntimeURLsUseLocalhostAndRuntimeServedEditor() {
        let runtime = RuntimeController(keychain: InMemoryKeychainStore())

        XCTAssertEqual(runtime.editorURL.host(), "127.0.0.1")
        XCTAssertEqual(runtime.editorURL.path(), "/editor")
        XCTAssertEqual(runtime.overlayURL.host(), "127.0.0.1")
    }

    func testRuntimeOutputCapturesTokensForEditorAndOverlayURLs() {
        let keychain = InMemoryKeychainStore()
        let runtime = RuntimeController(keychain: keychain)

        runtime.ingestRuntimeOutputLine("CueCanvas runtime listening on http://127.0.0.1:49152")
        runtime.ingestRuntimeOutputLine("Editor token: editor-test")
        runtime.ingestRuntimeOutputLine("Program overlay URL: http://127.0.0.1:49152/overlay/program?token=overlay-test")

        XCTAssertEqual(runtime.status.title, "Running")
        XCTAssertEqual(runtime.port, 49152)
        XCTAssertEqual(runtime.credentials.editorToken, "editor-test")
        XCTAssertEqual(runtime.credentials.overlayToken, "overlay-test")
        XCTAssertEqual(try keychain.read(account: KeychainAccount.overlayToken.rawValue), "overlay-test")
        XCTAssertEqual(
            URLComponents(url: runtime.editorURL, resolvingAgainstBaseURL: false)?
                .queryItems?
                .map(\.name),
            ["editorToken", "overlayToken"]
        )
        XCTAssertEqual(
            URLComponents(url: runtime.overlayURL, resolvingAgainstBaseURL: false)?
                .queryItems?
                .first(where: { $0.name == "token" })?
                .value,
            "overlay-test"
        )
    }

    func testStartLaunchesRuntimeWithAFreePort() {
        let launcher = RecordingLauncher()
        let runtime = RuntimeController(launcher: launcher, keychain: InMemoryKeychainStore())

        runtime.start()

        XCTAssertEqual(runtime.status.title, "Starting")
        XCTAssertNotNil(launcher.launchedPort)
        XCTAssertGreaterThan(launcher.launchedPort ?? 0, 0)
    }

    func testKeychainStoreReadWriteDelete() throws {
        let keychain = InMemoryKeychainStore()

        try keychain.write("secret", account: KeychainAccount.obsPassword.rawValue)
        XCTAssertEqual(try keychain.read(account: KeychainAccount.obsPassword.rawValue), "secret")

        try keychain.delete(account: KeychainAccount.obsPassword.rawValue)
        XCTAssertNil(try keychain.read(account: KeychainAccount.obsPassword.rawValue))
    }
}

final class RecordingLauncher: RuntimeProcessLaunching {
    var launchedPort: Int?

    func launch(
        port: Int,
        output: @escaping (String) -> Void,
        termination: @escaping (Int32) -> Void
    ) throws -> RuntimeProcessHandle {
        launchedPort = port
        return RecordingProcessHandle()
    }
}

final class RecordingProcessHandle: RuntimeProcessHandle {
    var isRunning = true

    func terminate() {
        isRunning = false
    }
}

final class InMemoryKeychainStore: KeychainStoring {
    private var storage: [String: String] = [:]

    func read(account: String) throws -> String? {
        storage[account]
    }

    func write(_ value: String, account: String) throws {
        storage[account] = value
    }

    func delete(account: String) throws {
        storage.removeValue(forKey: account)
    }
}
