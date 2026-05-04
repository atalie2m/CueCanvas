#!/usr/bin/env sh
set -eu

if command -v nix >/dev/null 2>&1; then
  command -v nix
elif [ -x /run/current-system/sw/bin/nix ]; then
  printf '%s\n' /run/current-system/sw/bin/nix
elif [ -x /nix/var/nix/profiles/default/bin/nix ]; then
  printf '%s\n' /nix/var/nix/profiles/default/bin/nix
else
  printf '%s\n' "nix was not found on PATH or the standard macOS Nix locations." >&2
  exit 127
fi
