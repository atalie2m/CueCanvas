# CueCanvas Agent Guidance

## Nix Store Safety

CueCanvas has previously caused the local Nix Store to grow past 100 GB because
agents repeatedly ran the project as an unfiltered local path flake. The main
failure mode is copying large workspace directories into the flake source, not
the main Darwin profile from `dotfiles`.

Do not run CueCanvas with unfiltered local path flakes:

```sh
nix run path:$PWD#...
nix build path:$PWD#...
```

Use the repository as a Git flake instead:

```sh
nix run .#...
nix build .#...
```

The Git flake source should only include files that belong in version control.
The following directories must not be copied into the Nix flake source:

- `target/`
- `node_modules/`
- `.git/`
- `.direnv/`

If any Nix command or flake configuration needs to reference the local checkout
as a path, ensure the source is explicitly filtered so those directories are
excluded. Do not depend on ad hoc cleanup after the fact.

## Cleanup

Before deleting anything, estimate reclaimable store space:

```sh
nix store gc --dry-run
```

To perform a full garbage collection of old generations and unreachable store
paths:

```sh
sudo nix-collect-garbage -d
```

Use cleanup carefully: it can remove cached build outputs and force later
commands to rebuild or redownload dependencies.
