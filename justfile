set dotenv-load := false

nix := `scripts/find-nix.sh`

bootstrap:
  @scripts/bootstrap-nix.sh

dev:
  @{{nix}} develop . -c zsh

format:
  @{{nix}} run .#format

check:
  @{{nix}} develop . -c cargo check --workspace --all-targets
  @{{nix}} develop . -c pnpm typecheck

test:
  @{{nix}} develop . -c cargo test --workspace
  @{{nix}} develop . -c pnpm test

runtime:
  @{{nix}} develop . -c cargo run -p cuecanvas-runtime

editor:
  @{{nix}} develop . -c pnpm --filter @cuecanvas/editor dev

overlay:
  @{{nix}} develop . -c pnpm --filter @cuecanvas/overlay dev

macos-generate:
  @{{nix}} develop . -c xcodegen generate --spec apps/macos/project.yml
