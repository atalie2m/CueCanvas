# Testing Strategy

Testing is a first-class implementation constraint.

## Rust

- Model serde round trips for all public contracts.
- Schema snapshots for package, command, event, and overlay contracts.
- Engine unit tests for deterministic absolute cue resolution.
- Property-style coverage for unknown extension metadata preservation.
- Live command tests for idempotency, immutable ProgramSnapshot, Clear,
  Blackout, and Restore.
- Package tests for atomic save/load and recovery data.

## Web

- TypeScript strict mode for editor and overlay.
- Unit tests for diff and render-model helpers.
- Playwright coverage for Preview -> Take -> Restore and transparent overlay
  rendering once the apps are wired to a dev server.

## macOS

- XcodeGen owns the project file.
- XCTest covers runtime launch configuration, Keychain/token handling, and URL
  construction.
- XCUITest smoke verifies WKWebView-hosted editor launch once the runtime is
  bundled.

## OBS

CI uses mocked OBS WebSocket tests. Release candidates require real OBS Browser
Source verification for create/update, refresh, hide/show, scene switch,
width/height mismatch, and Program restore.
