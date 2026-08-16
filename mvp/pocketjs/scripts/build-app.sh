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
# APP_ENTRY selects the guest entry (default "main" — the Solid editor):
#   APP_ENTRY=cf         A4-R1 diagnostic counterfactual (no Solid)
#   APP_ENTRY=cf-notext  A4-R1 counterfactual, empty text nodes
#   APP_ENTRY=main-ops   op-count diagnostic (wraps ui.* ops; slow)
#   APP_ENTRY=cf-ops     op-count diagnostic for the counterfactual
# The bundle is renamed <entry>.js/.pak for the host --js/--pak flags.
#
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
POCKETJS="${POCKETJS_ROOT:-$ROOT/vendor/pocketjs}"
NAME="${APP_ENTRY:-main}"
# The default bundle keeps the host's historic name (markit-editor);
# diagnostic entries keep their own names for --js/--pak selection.
if [ -z "${APP_ENTRY:-}" ] || [ "$NAME" = "main" ]; then
  OUT_NAME="markit-editor"
else
  OUT_NAME="$NAME"
fi
APP_SRC="$ROOT/mvp/pocketjs/app/$NAME.tsx"
[ -f "$APP_SRC" ] || APP_SRC="$ROOT/mvp/pocketjs/app/$NAME.ts"
OUT="$ROOT/mvp/pocketjs/dist"
FONT="${MARKIT_FONT:-/mnt/c/Windows/Fonts/consola.ttf}"

mkdir -p "$OUT"

(cd "$POCKETJS" && bun tools/build.ts "$APP_SRC" \
  --density=1 \
  --font-regular="$FONT" \
  --outdir="$OUT")

# build.ts names the bundle after the entry file; rename to the name the
# host resolves (skip when the entry already produced it).
[ -f "$OUT/main.js" ] && mv -f "$OUT/main.js" "$OUT/$OUT_NAME.js"
[ -f "$OUT/main.pak" ] && mv -f "$OUT/main.pak" "$OUT/$OUT_NAME.pak"

echo "markit: guest bundle -> $OUT/$OUT_NAME.{js,pak} ($(du -sh "$OUT" | cut -f1))"
