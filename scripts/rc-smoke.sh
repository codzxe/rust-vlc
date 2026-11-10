#!/usr/bin/env bash
# rc-smoke.sh — RC smoke harness (card 8.G2, issue #58; docs/release-checklist.md).
#
# REAL local checks against the current tree, in the shape an RC needs:
#   1. engine-build        cargo build --release --locked of both shipped bins
#   2. engine-version      `lumen --version` prints a version line, exit 0
#   3. engine-info         `lumen engine-info` self-describes (version + decoders)
#   4. headless-decode     `lumen decode-bench <fixture>` — every frame decoded
#   5. command-layer-play  `lumen open <fixture>` reaches Playing, then Stopped
#   6. wasm-web-build      the wasm32 engine module builds; \0asm magic; >1 MiB
#   7. web-ui-build        `npm run build` in ui/ (tsc + vite production build)
#   8. web-ui-browser-load Playwright chromium loads the BUILT app; boots to
#                          the player surface; zero page errors
#
# Exit code: 0 only when every check PASSes. A check that is skipped because
# a check it depends on failed is reported BLOCKED and ALSO fails the run
# (an RC whose web build is red is not shippable, whatever the reason).
#
# Environment overrides (all optional):
#   RC_SMOKE_FIXTURE   media file for checks 4-5
#                      (default: tests/fixtures/generated/valid/avi_mjpeg_pcm.avi
#                      — the first-vertical fixture: MJPEG+PCM in AVI, the one
#                      decode lane the CLI drives end-to-end today)
#   RC_SMOKE_CHROMIUM  explicit chromium executable for check 8 (see discovery
#                      below; the sandbox cache revision can drift from the
#                      ui/package-lock.json @playwright/test pin — documented
#                      in docs/guides/troubleshooting.md "Playwright says the
#                      browser executable doesn't exist")
#   RC_SMOKE_PORT      port for the vite preview server (default 4173)
#   RC_SMOKE_SKIP_WEB  set to 1 to skip checks 7-8 (engine-only smoke; the
#                      summary then says so — it is NOT a full RC smoke)
#
# 3-OS / CI rows are NOT this script's business: they live in
# docs/release-checklist.md as blocked rows (CI billing; user-owned runners).
#
# Style follows scripts/full_gate.sh (set -uo pipefail, honest fail counter).
set -uo pipefail

ROOT=$(cd "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

fail=0
blocked_note=""

FIXTURE=${RC_SMOKE_FIXTURE:-tests/fixtures/generated/valid/avi_mjpeg_pcm.avi}
PORT=${RC_SMOKE_PORT:-4173}

# --- preflight -------------------------------------------------------------
need() { command -v "$1" >/dev/null 2>&1 || { echo "rc-smoke: preflight: '$1' not found in PATH" >&2; exit 2; }; }
need cargo; need node; need npm

if [ ! -f "$FIXTURE" ]; then
  echo "rc-smoke: fixture not found: $FIXTURE (override with RC_SMOKE_FIXTURE)" >&2
  exit 2
fi

# Binary location: cargo's own layout (shared CARGO_TARGET_DIR honored when set).
TARGET_DIR=${CARGO_TARGET_DIR:-$ROOT/target}
LUMEN=$TARGET_DIR/release/lumen
LUMEN_DESKTOP=$TARGET_DIR/release/lumen-desktop
WASM=$TARGET_DIR/wasm32-unknown-unknown/debug/lumen_web.wasm

echo "=== rc-smoke environment ==="
uname -smr
rustc --version 2>/dev/null || cargo --version
node --version
echo "fixture: $FIXTURE"

# --- 1. engine-build -------------------------------------------------------
echo "=== 1. engine-build (cargo build --release --locked -p lumen-cli -p desktop) ==="
if cargo build --release --locked -p lumen-cli -p desktop; then
  echo "engine-build: PASS"
else
  echo "engine-build: FAILED"; fail=1
fi

# --- 2. engine-version -----------------------------------------------------
echo "=== 2. engine-version ==="
if v=$("$LUMEN" --version 2>/dev/null) && printf '%s' "$v" | grep -Eq '^lumen [0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "engine-version: PASS ($v)"
else
  echo "engine-version: FAILED (got: $v)"; fail=1
fi

# --- 3. engine-info --------------------------------------------------------
echo "=== 3. engine-info ==="
info=$("$LUMEN" engine-info </dev/null 2>&1)
if [ "$?" -eq 0 ] && printf '%s' "$info" | grep -q '  version:' && printf '%s' "$info" | grep -q '  decoders:'; then
  echo "engine-info: PASS"
  printf '%s\n' "$info" | sed 's/^/  /'
else
  echo "engine-info: FAILED"; printf '%s\n' "$info" | sed 's/^/  /' | head -8; fail=1
fi

# --- 4. headless-decode ----------------------------------------------------
echo "=== 4. headless-decode (decode-bench; every frame must decode) ==="
bench=$("$LUMEN" decode-bench "$FIXTURE" 2>/dev/null)
if [ "$?" -eq 0 ] && printf '%s' "$bench" | grep -q '^BENCH '; then
  echo "$bench"
  ok=$(printf '%s' "$bench" | awk '{
    vf=-1; vd=-2; af=-1; ad=-2;
    for (i = 1; i <= NF; i++) {
      if ($i ~ /^v_frames=/)   { sub(/^v_frames=/, "", $i);   vf=$i+0 }
      if ($i ~ /^v_decoded=/)  { sub(/^v_decoded=/, "", $i);  vd=$i+0 }
      if ($i ~ /^a_frames=/)   { sub(/^a_frames=/, "", $i);   af=$i+0 }
      if ($i ~ /^a_decoded=/)  { sub(/^a_decoded=/, "", $i);  ad=$i+0 }
    }
    print (vf > 0 && vd == vf && af > 0 && ad == af) ? 1 : 0
  }')
  if [ "$ok" = "1" ]; then
    echo "headless-decode: PASS (v_decoded==v_frames, a_decoded==a_frames)"
  else
    echo "headless-decode: FAILED (decoded counts below container counts in BENCH line)"; fail=1
  fi
else
  echo "headless-decode: FAILED (no exit-0 BENCH line)"; printf '%s\n' "$bench" | head -3; fail=1
fi

# --- 5. command-layer-play -------------------------------------------------
echo "=== 5. command-layer-play (open -> Playing -> Stopped through the catalog) ==="
play=$(printf 'quit\n' | "$LUMEN" open "$FIXTURE" 2>&1)
if [ "$?" -eq 0 ] && printf '%s\n' "$play" | grep -q 'Buffering → Playing' && printf '%s\n' "$play" | grep -q 'Playing → Stopped'; then
  echo "command-layer-play: PASS (reached Playing, ended Stopped, exit 0)"
else
  echo "command-layer-play: FAILED"; printf '%s\n' "$play" | tail -6; fail=1
fi

# --- 6. wasm-web-build -----------------------------------------------------
echo "=== 6. wasm-web-build (cargo build -p lumen-web --target wasm32-unknown-unknown --locked) ==="
if cargo build -p lumen-web --target wasm32-unknown-unknown --locked; then
  if [ -f "$WASM" ] && [ "$(stat -c %s "$WASM" 2>/dev/null || echo 0)" -gt 1048576 ] && [ "$(od -An -tx1 -N4 "$WASM" | tr -d ' \n')" = "0061736d" ]; then
    echo "wasm-web-build: PASS ($(stat -c %s "$WASM") bytes, \\0asm magic)"
  else
    echo "wasm-web-build: FAILED (module missing, too small, or wrong magic: $WASM)"; fail=1
  fi
else
  echo "wasm-web-build: FAILED (cargo build)"; fail=1
fi

# --- 7 + 8. web UI ---------------------------------------------------------
if [ "${RC_SMOKE_SKIP_WEB:-0}" = "1" ]; then
  echo "=== 7/8. web-ui: SKIPPED (RC_SMOKE_SKIP_WEB=1 — engine-only smoke, NOT a full RC smoke) ==="
else
  echo "=== 7. web-ui-build (ui/: npm run build = tsc --noEmit && vite build) ==="
  if [ ! -d "$ROOT/ui/node_modules" ]; then
    echo "web-ui-build: FAILED (ui/node_modules missing — run npm ci in ui/ first)"; fail=1
    ui_built=0
  elif (cd "$ROOT/ui" && npm run build); then
    echo "web-ui-build: PASS"
    ui_built=1
  else
    echo "web-ui-build: FAILED"
    echo "  (known main defects as of 2026-11-10: ui/src/App.tsx merge-conflict"
    echo "   markers + stale generated settings screen — docs/audit/docs-sweep-20261110.md F-1/F-2,"
    echo "   fixes owned outside lane E; re-run when they land)"
    fail=1
    ui_built=0
  fi

  echo "=== 8. web-ui-browser-load (vite preview + Playwright chromium) ==="
  if [ "$ui_built" = "1" ]; then
    # Chromium discovery: never download. Explicit override wins; else the
    # newest chromium-* install under PLAYWRIGHT_BROWSERS_PATH (default
    # ~/.cache/ms-playwright). The ui/ playwright pin may expect a different
    # revision — the mjs bridges with an explicit executablePath (see
    # docs/guides/troubleshooting.md).
    CHROMIUM=${RC_SMOKE_CHROMIUM:-}
    if [ -z "$CHROMIUM" ]; then
      CHROMIUM=$(find "${PLAYWRIGHT_BROWSERS_PATH:-$HOME/.cache/ms-playwright}" \
        -maxdepth 3 -type f -path '*chromium-*/chrome-linux*/chrome' 2>/dev/null \
        | sort -r | head -1)
    fi
    if [ -z "$CHROMIUM" ] || [ ! -x "$CHROMIUM" ]; then
      echo "web-ui-browser-load: FAILED (no chromium executable found — set RC_SMOKE_CHROMIUM)"; fail=1
    elif [ ! -d "$ROOT/ui/dist" ]; then
      echo "web-ui-browser-load: BLOCKED (ui/dist missing after a passing build?)"; fail=1
    else
      (cd "$ROOT/ui" && "./node_modules/.bin/vite" preview --port "$PORT" --strictPort >/tmp/rc-smoke-preview.log 2>&1) &
      preview_pid=$!
      # Wait for the port (bash /dev/tcp probe), bounded at ~15 s.
      up=0
      for _ in $(seq 1 50); do
        if (exec 3<>"/dev/tcp/127.0.0.1/$PORT") 2>/dev/null; then up=1; break; fi
        sleep 0.3
      done
      if [ "$up" = "1" ] && node "$ROOT/scripts/rc-smoke-web.mjs" "http://127.0.0.1:$PORT/" "$CHROMIUM"; then
        echo "web-ui-browser-load: PASS"
      else
        echo "web-ui-browser-load: FAILED"; fail=1
      fi
      kill "$preview_pid" 2>/dev/null
      wait "$preview_pid" 2>/dev/null
    fi
  else
    echo "web-ui-browser-load: BLOCKED (web-ui-build failed — the app cannot be served)"
    blocked_note="blocked-by-web-ui-build"
    fail=1
  fi
fi

echo "=== RC SMOKE RESULT: $([ $fail -eq 0 ] && echo PASS || echo FAIL)${blocked_note:+ ($blocked_note)} ==="
exit $fail
