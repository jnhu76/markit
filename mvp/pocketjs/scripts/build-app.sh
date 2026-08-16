#!/usr/bin/env bash
# Build the Markit PocketJS guest bundle into mvp/pocketjs/dist.
#
# Uses the vendored PocketJS build pipeline (tools/build.ts) with the
# Markit app entry; fonts override to Consolas (the GPUI MVP's face) so
# the two MVPs measure the same typography. Run from the repo root, or
# set POCKETJS_ROOT to the vendored checkout:
#
#   mvp/pocketjs/scripts/build-app.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
POCKETJS="${POCKETJS_ROOT:-$ROOT/vendor/pocketjs}"
APP_SRC="$ROOT/mvp/pocketjs/app/main.tsx"
OUT="$ROOT/mvp/pocketjs/dist"
FONT="${MARKIT_FONT:-/mnt/c/Windows/Fonts/consola.ttf}"

mkdir -p "$OUT"

(cd "$POCKETJS" && bun tools/build.ts "$APP_SRC" \
  --density=1 \
  --font-regular="$FONT" \
  --outdir="$OUT")

# build.ts names the bundle after the entry file ("main"); the host looks
# for markit-editor.{js,pak}.
mv -f "$OUT/main.js" "$OUT/markit-editor.js"
mv -f "$OUT/main.pak" "$OUT/markit-editor.pak"

echo "markit: guest bundle -> $OUT/markit-editor.{js,pak} ($(du -sh "$OUT" | cut -f1))"
