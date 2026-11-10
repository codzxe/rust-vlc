#!/bin/sh
# make-dmg.sh — build the macOS disk image (.dmg) with the NATIVE hdiutil
# (card 7.w22g, issue #56 release half).
#
# No third-party tooling, no downloads, no create-dmg dependency: the
# bundle is staged as a symlink into a scratch folder and packed with
#   hdiutil create -volname <name> -srcfolder <stage> -format UDZO
# which is the documented Apple CLI for read-only compressed images.
#
# Inputs (environment; product fields via scripts/release/product-env.py):
#   PRODUCT_NAME     (volume name + bundle name)
#   PRODUCT_LOWER, PRODUCT_VERSION, OS_ID, ARCH_ID (artifact naming)
#   APP_DIR          (where make-app.sh placed the bundle; default DIST_DIR)
#   DIST_DIR         (dmg output dir, default "dist")
#
# Usage: make-app.sh first (the bundle must exist), then make-dmg.sh.
# macOS only — dies loudly when hdiutil is absent (e.g. Linux dry-runs).
# POSIX sh; `bash -n` clean.

set -eu

: "${PRODUCT_NAME:?make-dmg.sh: PRODUCT_NAME not set}"
: "${PRODUCT_LOWER:?make-dmg.sh: PRODUCT_LOWER not set}"
: "${PRODUCT_VERSION:?make-dmg.sh: PRODUCT_VERSION not set}"
: "${OS_ID:?make-dmg.sh: OS_ID not set}"
: "${ARCH_ID:?make-dmg.sh: ARCH_ID not set}"
DIST_DIR=${DIST_DIR:-dist}
APP_DIR=${APP_DIR:-$DIST_DIR}

ROOT=$(cd "$(dirname -- "$0")/../.." && pwd)
cd "$ROOT"

die() { echo "make-dmg.sh: error: $*" >&2; exit 2; }

command -v hdiutil >/dev/null 2>&1 ||
    die "hdiutil not found — make-dmg.sh must run on macOS (release job: macos runner)"

app="$APP_DIR/${PRODUCT_NAME}.app"
[ -d "$app" ] || die "bundle not found: $app (run make-app.sh first)"

mkdir -p "$DIST_DIR"
dist_abs=$(cd "$DIST_DIR" && pwd)
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT INT TERM

# Symlink (not copy): hdiutil -srcfolder follows it and the image keeps
# the standard "<volume>/<Name>.app" drag-install layout.
ln -s "$(cd "$APP_DIR" && pwd)/${PRODUCT_NAME}.app" "$stage/${PRODUCT_NAME}.app"

out="$dist_abs/${PRODUCT_LOWER}-${PRODUCT_VERSION}-${ARCH_ID}-${OS_ID}.dmg"
hdiutil create -volname "$PRODUCT_NAME" -srcfolder "$stage" -format UDZO -ov "$out"
[ -s "$out" ] || die "dmg is missing or empty: $out"
echo "make-dmg.sh: wrote $out"
