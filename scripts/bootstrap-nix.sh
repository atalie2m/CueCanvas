#!/usr/bin/env sh
set -eu

NIX_BIN="$(dirname "$("$PWD/scripts/find-nix.sh")")"
export PATH="$NIX_BIN:$PATH"

printf 'Using Nix: %s\n' "$(command -v nix)"
nix --version
printf '\nRun: %s develop\n' "$(command -v nix)"
