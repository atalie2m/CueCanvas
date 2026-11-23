# CueCanvas

CueCanvas is a macOS-first show graphics system for safely preparing,
previewing, validating, and taking OBS Browser Source graphics.

This repository currently contains the first implementation slice:

- Rust model/protocol contracts with `origin`, `actor`, revision, and
  `extensions` metadata fields.
- Rust cue engine for deterministic absolute cue preview, immutable
  `ProgramSnapshot` creation, idempotent Take, Clear, Blackout, and Restore.
- Local Axum runtime skeleton exposing project, preview, take, overlay snapshot,
  and overlay WebSocket routes.
- React + TypeScript operator cockpit scaffold.
- TypeScript overlay renderer scaffold.
- SwiftUI macOS host scaffold managed by XcodeGen.
- Nix flake and `just` commands for a reproducible development shell.

## Bootstrap

Some shells may not have `nix` on `PATH`. Start with:

```sh
scripts/bootstrap-nix.sh
```

Then use:

```sh
just dev
just test
just runtime
just editor
```

See [docs/development.md](docs/development.md) for the development workflow and
[docs/product-contract.md](docs/product-contract.md) for the live-safety
invariants. Current completion status is tracked in
[docs/implementation-status.md](docs/implementation-status.md).
