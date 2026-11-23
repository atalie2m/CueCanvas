# CueCanvas Development

CueCanvas is implemented as a macOS-first monorepo:

- `crates/*`: Rust model, engine, package persistence, protocol, runtime, and CLI.
- `apps/editor`: React + TypeScript operator cockpit.
- `apps/overlay`: TypeScript overlay renderer for OBS Browser Source.
- `apps/macos`: SwiftUI host app scaffold.
- `fixtures`: project and overlay fixtures used by tests.

## Nix Bootstrap

Some shells on this machine do not expose `nix` on `PATH`. Use:

```sh
scripts/bootstrap-nix.sh
```

The bootstrap script checks `PATH`, `/run/current-system/sw/bin/nix`, and
`/nix/var/nix/profiles/default/bin/nix`. All `just` commands use the same
lookup and evaluate the repository as a Git flake (`.#...`) so Nix only copies
files that belong in version control.

## Common Commands

```sh
just bootstrap
just dev
just format
just check
just test
just runtime
```

The same checks are also exposed as flake apps for CI and shells that should not
depend on `just`:

```sh
nix run .#format-check
nix run .#check
nix run .#test
nix run .#test-macos
```

The flake follows the local dotfiles template style: `flake-parts`,
`treefmt-nix`, `git-hooks.nix`, Rust stable through `rust-overlay`, Node 22 +
pnpm + Playwright tooling, and Darwin-only Swift/Xcode helper tools.
