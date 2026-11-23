# Implementation Status

The v5.2 proposal is not fully implemented yet. The repository currently covers
the project foundation and a verified vertical slice.

## Implemented

- Nix flake, bootstrap scripts, formatter/check/test apps, and CI skeleton.
- Rust model/protocol contracts with actor, origin, revisions, and extension
  metadata.
- Cue resolution, PreviewSnapshot generation, ProgramSnapshot Take semantics,
  idempotent Take, Clear, Blackout, and Restore.
- Package save/load with atomic JSON writes and extension metadata preservation.
- Runtime routes for project, preview, take, emergency live commands, overlay
  snapshot, overlay HTML, tokenized assets/fonts, editor WebSocket events, and
  overlay WebSocket reconnect/broadcast snapshots.
- Runtime editor/overlay token separation, read-only overlay authorization, and
  package save/open routes.
- Runtime Take gating against show/preview preflight errors, including
  disconnected Program output.
- macOS host token capture and tokenized editor/overlay URLs.
- Core CSV/TSV import for People, Sessions, and Sponsors with typed-entity and
  internal DataTable updates.
- Dedicated preflight, extension manifest, and OBS Browser Source health-check
  crates with tests.
- React operator cockpit scaffold with cue controls, monitor diffing, data
  counts, and CSV/TSV import panel.
- Rehearsal/Live editor controls, grouped Preflight state, warning override
  hashes, expanded diff categories, health summary, stale-preview messaging, and
  LiveRunning edit confirmation.
- Split `.cuecanvas` package save/load with show/run files, autosave snapshots,
  command-log JSONL, recovery info, migration/version scaffolding, and package
  path traversal rejection.
- Runtime-served editor assets, configurable/free runtime port launch, preflight
  and recovery routes, and editor/overlay static asset serving.
- TypeScript overlay renderer scaffold with runtime asset URL rendering and
  rendered ACK messages.
- XcodeGen-managed SwiftUI host with runtime process launch, token capture,
  health polling, Keychain-backed tokens, File Open/Save/Save As plumbing, and a
  project-local build-and-run script.

## Not Yet Implemented

- Full Template, Data, Cue Sheet, Rehearsal, and Live Mode UI.
- Rich import review/diff, manual column remapping controls, and append/merge
  conflict handling.
- OBS WebSocket bridge, setup wizard UI, and real Browser Source create/update.
- Rich recovery candidate UX beyond the runtime recovery route and saved
  autosave metadata.
- Host header/origin checks and bundled release packaging polish.
- Declarative plugin pack import and contribution apply/reject flows.
- Renderer golden screenshots and real OBS verified render tests.
- Bundled runtime launch from the macOS app.

## Current Verification Gates

- `nix flake check`
- `nix run .#format-check`
- `nix run .#check`
- `nix run .#test`
- `nix run .#test-macos`
- `cargo test --workspace`
- `pnpm typecheck`
- `pnpm test`
- `pnpm build`
- `xcodegen generate --spec apps/macos/project.yml`
- `xcodebuild test -project apps/macos/CueCanvas.xcodeproj -scheme CueCanvasHost -destination 'platform=macOS' -derivedDataPath /private/tmp/CueCanvasDerivedData CODE_SIGNING_ALLOWED=NO`
