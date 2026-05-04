#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DERIVED_DATA="${CUECANVAS_DERIVED_DATA:-/private/tmp/CueCanvasDerivedData}"
CONFIGURATION="${CONFIGURATION:-Debug}"
APP_NAME="CueCanvasHost"
SCHEME="CueCanvasHost"
PROJECT="$ROOT/apps/macos/CueCanvas.xcodeproj"
APP_PATH="$DERIVED_DATA/Build/Products/$CONFIGURATION/$APP_NAME.app"
NIX="$("$ROOT/scripts/find-nix.sh")"

cd "$ROOT"

pkill -x "$APP_NAME" 2>/dev/null || true

"$NIX" develop . -c cargo build -p cuecanvas-runtime
"$NIX" develop . -c pnpm --filter @cuecanvas/editor build
"$NIX" develop . -c pnpm --filter @cuecanvas/overlay build
"$NIX" run .#test-macos-generate

xcodebuild build \
  -project "$PROJECT" \
  -scheme "$SCHEME" \
  -configuration "$CONFIGURATION" \
  -derivedDataPath "$DERIVED_DATA" \
  CODE_SIGNING_ALLOWED=NO

mkdir -p "$APP_PATH/Contents/Resources/editor" "$APP_PATH/Contents/Resources/overlay"
cp "$ROOT/target/debug/cuecanvas-runtime" "$APP_PATH/Contents/Resources/cuecanvas-runtime"
rsync -a --delete "$ROOT/apps/editor/dist/" "$APP_PATH/Contents/Resources/editor/"
rsync -a --delete "$ROOT/apps/overlay/dist/" "$APP_PATH/Contents/Resources/overlay/"

if [[ ${1:-} == "--verify" ]]; then
  /usr/bin/open -n "$APP_PATH"
  sleep 2
  pgrep -x "$APP_NAME" >/dev/null
  echo "$APP_NAME launched from $APP_PATH"
else
  /usr/bin/open -n "$APP_PATH"
fi
