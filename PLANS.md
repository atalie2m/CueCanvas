# CueCanvas v5.2 Remaining Implementation Plan

This file tracks the remaining work required to complete
`docs/Cue Canvas Proposal v5.2.md` from the current verified vertical slice.

## Current Baseline

Implemented:

- Reproducible Nix development environment with formatter/check/test flake
  apps and CI entrypoints.
- Rust model/protocol contracts with actor, origin, revisions, and extension
  metadata.
- Deterministic absolute cue preview, immutable `ProgramSnapshot` Take
  semantics, idempotent Take/Clear/Blackout/Restore, and package save/load.
- Runtime routes for project, import, preview, Take, emergency commands,
  tokenized overlay/assets/fonts, editor events, and overlay broadcast/reconnect.
- Take gating against show/preview preflight errors, including disconnected
  Program output.
- Core CSV/TSV import for People, Sessions, and Sponsors.
- React operator cockpit scaffold and TypeScript overlay renderer scaffold.
- XcodeGen-managed SwiftUI macOS host scaffold.

Not complete:

- Full product UI, OBS setup, bundled runtime launch, full persistence/recovery,
  security hardening, renderer verification, and MVP release packaging.

## Phase 1: Runtime Product Core

Goal: make Runtime the explicit command/state owner for all MVP operations.

- Add a runtime command dispatcher/state actor for all mutations.
- Route every mutating operation through actor validation, expected revision
  checks, idempotency, live-state policy, operation logging, derived-state
  rebuild, preflight update, autosave scheduling, and event broadcast.
- Add explicit commands for show creation, stage/output updates, run session
  creation/opening, cue CRUD/reorder, template instance CRUD, typed entity CRUD,
  warning overrides, and live state transitions.
- Persist and broadcast `preflight.changed`, `operation.logged`,
  `overlay.connectionChanged`, `overlay.rendered`, `runtime.error`, and
  `autosave.changed` events.
- Enforce Live Mode restrictions for destructive template, data, cue, asset, and
  output edits.
- Add Runtime API schema snapshots and generated TypeScript contract outputs, or
  another deterministic contract-checking path.

Verification gate:

- `nix run .#check`
- `nix run .#test`
- Runtime integration tests for stale revisions, idempotency, command actors,
  live-state restrictions, and no implicit Program mutation from edits.

## Phase 2: Template System and Setup UI

Goal: let users build a basic show from templates without freeform board editing.

- Add built-in TemplateDefinitions for Lower Third, Session Title, Sponsor Bug,
  Break Screen, and Speaker Card.
- Add Template Library UI with origin badges and missing-plugin warnings.
- Add Stage/Template Mode with TemplateInstance placement, size, z-index,
  variant selection, local slot binding, safe area preview, and inspector.
- Keep TemplateDefinition local slots independent of People/Sessions/Sponsors
  field names.
- Add TemplateInstance validation for missing definitions, missing slots,
  invalid variants, unsafe frames, and deleted assets.
- Add test fixtures for every built-in template.

Verification gate:

- Rust template resolver tests for all built-in templates.
- Web component tests for Template Library and Inspector state.
- Playwright smoke for adding a Lower Third, binding slots, previewing, and
  seeing expected overlay output.

## Phase 3: Data and Cue Sheet

Goal: make People/Sessions/Sponsors and the Cue Sheet usable for a real small
event.

- Build People, Sessions, and Sponsors editors with create, edit, delete,
  validation, origin display, and asset reference handling.
- Expand CSV/TSV import into an import review flow with manual column remapping,
  append/replace/merge choices, duplicate detection, rejected row display, and
  conflict handling.
- Add Cue Sheet CRUD: create, edit, reorder, delete, entity selectors, template
  state controls, slot overrides, operator notes, and segment grouping.
- Add stale preview warnings when Cue/Data/Template changes after Preview.
- Add thumbnail generation or deterministic preview stills for cues.
- Add bulk cue generation from current typed entities for MVP-supported
  templates only.

Verification gate:

- Rust import tests for remapping, merge conflicts, duplicates, and no Program
  mutation.
- Web tests for import review, entity editors, and cue edit flows.
- Playwright flow: import demo data, create cue, preview, edit data, confirm
  Program remains unchanged, refresh preview, Take.

## Phase 4: Preflight and Live Operations UI

Goal: make preflight a visible operational gate, not hidden backend logic.

- Add full Preflight panel with Error/Warning/Info grouping, scope navigation,
  fix actions, origin/source display, and missing plugin origin warnings.
- Add warning override hashes so warnings reappear when the underlying value
  changes.
- Add Rehearsal Mode and Live Mode screens with Cue List, Preview monitor,
  Program monitor, Diff Preview, Take, Clear, Blackout, Restore, health status,
  and operation log.
- Disable or confirm dangerous edits in LiveRunning.
- Add diff categories beyond resolved data: visibility, assets, style, output
  state, revision, origin, and warning changes.
- Add user-facing stale-preview message: Take uses the existing
  `PreviewSnapshot`, not a re-resolved Cue.

Verification gate:

- Rust tests for preflight blocking, warning override hashes, stale preview,
  plugin-origin warnings, and fix action allowlists.
- Web tests for disabled Take, override behavior, and Live Mode restrictions.
- Playwright Preview -> Take -> Clear -> Blackout -> Restore flow.

## Phase 5: Persistence, Autosave, Recovery, and Migrations

Goal: make `.cuecanvas` project packages robust enough for event use.

- Expand storage to the proposed split package layout:
  `manifest.json`, `project.json`, `show-definitions/`, `run-sessions/`,
  `extensions/registry.json`, `assets/`, `fonts/`, `thumbnails/`, `renders/`,
  and `autosave/`.
- Persist latest `ProgramSnapshot`, retained `ProgramSnapshot`, run session,
  command log JSONL, and recovery info.
- Add migration framework with format-version checks and unknown extension
  metadata preservation.
- Add autosave rotation, stale temp cleanup, and recovery candidate selection.
- Add project open/save/save-as UX in the macOS host and editor.
- Add canonical path validation for package assets, fonts, imports, and saves.
- Avoid storing OBS passwords or local tokens in project JSON.

Verification gate:

- Rust package round-trip tests for split files, autosave, recovery, migrations,
  unknown extension metadata, and path traversal rejection.
- Manual recovery drill: save, Take, kill runtime, reopen, restore Program.
- `nix run .#test`

## Phase 6: macOS Host Integration

Goal: ship as a macOS app, not a loose dev-server collection.

- Build and bundle the Rust runtime binary into the app.
- Build and bundle editor/overlay web assets and serve them from Runtime.
- Replace the editor dev-server URL with the Runtime-served editor URL.
- Launch Runtime automatically on a free localhost port, capture tokens, monitor
  process state, restart on request, and shut down cleanly.
- Add Keychain storage for overlay token, editor session token where needed, OBS
  password, and future external input tokens.
- Add File Open, Save, Save As, Recent Files, Preferences, and runtime health UI.
- Add a project-local `script/build_and_run.sh` and Codex Run action once the
  bundled launch path exists.
- Keep XcodeGen as the project source of truth.

Verification gate:

- `nix run .#test-macos`
- XCTest for runtime launch, process monitoring, token parsing, Keychain
  storage, file command plumbing, and failure states.
- XCUITest smoke for app launch, editor load, local Preview/Take, and runtime
  restart.

## Phase 7: OBS Setup Wizard and OBS Health

Goal: make OBS Browser Source setup part of the product experience.

- Add real obs-websocket client and authentication flow.
- Add setup wizard: connect, authenticate, select/create CueCanvas Graphics
  scene, create/select Browser Source, set URL, set width/height, apply
  recommended CSS, and verify source settings.
- Detect refresh/shutdown risks, URL mismatch, size mismatch, missing source,
  missing scene, visibility path issues, and overlay WebSocket disconnects.
- Add Test Pattern command and UI.
- Keep OBS Scene Binding out of MVP.
- Add mocked OBS WebSocket tests in CI.
- Add release-candidate manual OBS verification checklist.

Verification gate:

- Mocked OBS tests for create/update/verify flows.
- Real OBS manual pass for create/update, Take display, Browser Source refresh
  restore, hide/show, scene switch, and width/height mismatch.

## Phase 8: Renderer Confidence and Visual Testing

Goal: prove renderer behavior without claiming exact OBS equivalence.

- Add Chromium golden render harness for overlay fixtures.
- Add fixtures for transparent background, CJK line breaking, font fallback,
  text overflow, image fit, blackout/clear, and reconnect recovery.
- Add overflow detection output to Preflight.
- Add asset/font loading tests against tokenized runtime routes.
- Add OBS Verified Render checklist for release candidates.
- Keep UI wording to "high-confidence preview" and "OBS verified when
  connected"; do not claim perfect match.

Verification gate:

- Playwright screenshot tests for overlay fixtures.
- Canvas/pixel checks for transparent output and nonblank rendered assets.
- Release-candidate OBS Verified Render pass.

## Phase 9: Security Hardening

Goal: close local-app security gaps before MVP release.

- Bind Runtime only to `127.0.0.1`.
- Add Host header and Origin validation.
- Keep CORS disabled by default.
- Add request payload size limits and basic rate limits.
- Keep editor and overlay tokens separate; overlay token remains read-only.
- Require token checks for overlay HTML, snapshot, WebSocket, assets, and fonts.
- Add dedicated external input token model, disabled by default.
- Add canonical path checks for all filesystem reads/writes.
- Add safe error responses that do not leak secrets.
- Add secret scanning and dependency audit CI jobs once noise is manageable.

Verification gate:

- Runtime security integration tests for wrong token, missing token, token
  confusion, path traversal, large payloads, and invalid Host/Origin.
- `nix run .#check`

## Phase 10: Extension Readiness, Plugin-Free MVP

Goal: preserve future plugin compatibility without implementing plugin
execution.

- Keep manifest, capability, origin, contribution, and extension metadata
  schemas in the model.
- Add namespace and size validation for unknown extension metadata.
- Add read-only Extension Registry UI.
- Add origin/source badges for plugin-created artifacts.
- Add missing plugin warnings in Preflight and inspectors.
- Add contribution validator skeleton tests for declarative packs only.
- Do not add plugin loader, marketplace, native plugin execution, JS/WASM
  sandbox, renderer plugins, or plugin install UI for MVP.

Verification gate:

- Round-trip tests for plugin origins and unknown extension metadata.
- UI tests for source badges and missing plugin warnings.
- Manifest validator tests for capabilities, path traversal, unsupported
  contributions, semantic versions, and plugin ID rules.

## Phase 11: Release Readiness

Goal: make MVP releasable for small OBS-based events.

- Add release checklist covering clean checkout, Nix checks, macOS build,
  bundled runtime launch, OBS setup, save/reopen, recovery, and real demo show.
- Add demo `.cuecanvas` package with People, Sessions, Sponsors, built-in
  templates, and cue sheet from the proposal.
- Add user docs for first-run setup, OBS wizard, import, cue creation,
  Preview/Take, recovery, and troubleshooting.
- Add developer docs for architecture, command model, package format, runtime
  API, testing, and release process.
- Decide signing/notarization packaging path and document required credentials.
- Tag MVP only after all gates in this file pass.

Verification gate:

- `nix flake check`
- `nix run .#format-check`
- `nix run .#check`
- `nix run .#test`
- `nix run .#test-macos`
- `pnpm build`
- Built `.app` smoke test
- Real OBS release-candidate checklist

## Work Explicitly Out of MVP

- Advanced freeform Board Designer.
- Plugin loader, plugin marketplace, native plugins, JS/WASM plugin execution,
  renderer plugins, and plugin installation UI.
- OBS Scene Binding or auto-Take behavior.
- External connector plugins.
- General spreadsheet product features beyond People/Sessions/Sponsors import
  and editing.
- Windows host.

## Implementation Rule

Proceed phase by phase. Do not start a later high-risk area until the preceding
phase's verification gate passes. If a phase requires a narrower vertical slice,
keep the slice end-to-end and prove it with automated tests plus one manual
operator flow before broadening the surface area.
