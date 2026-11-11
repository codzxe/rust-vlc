#!/bin/sh
# build-dmg.sh — the macOS 8.E2 lane orchestrator (issue #56, card
# 8.E2, task W24-C): .app bundle -> codesign -> .dmg -> sign + notarize
# + staple.
#
# Pipeline (order matters — a notarized dmg proves its signed .app):
#   1. scripts/release/make-app.sh     bundle layout from product.toml
#                                      (Info.plist fields, MacOS/
#                                      Resources structure) — the
#                                      wave-22 generator, REUSED, not
#                                      duplicated
#   2. scripts/packaging/sign-macos.sh --app  (codesign + hardened
#                                      runtime; R-35-driven unsigned
#                                      fallback with .UNSIGNED.txt
#                                      sidecar when credentials are
#                                      absent)
#   3. scripts/release/make-dmg.sh     hdiutil UDZO disk image — the
#                                      wave-22 script, REUSED
#   4. scripts/packaging/sign-macos.sh --dmg  (codesign the image,
#                                      notarytool submit --wait,
#                                      stapler staple; each stage
#                                      honest about R-35)
#
# UNIVERSAL-BINARY DECISION (8.E2, packaging.toml [macos]):
# v1 ships TWO single-arch artifact sets (aarch64 on macos-latest,
# x86_64 on the macos-*-intel family), not one universal2 binary —
# documented at length in docs/packaging/README.md. packaging.toml
# [macos] universal = false encodes that decision; this script REFUSES
# to run with universal = true until a card implements the lipo build
# (config can never enable a missing capability — the product.toml
# rule-F1 posture, applied to packaging).
#
# Inputs (environment; set by the release workflow via
# scripts/packaging/packaging-env.py): PRODUCT_*, BIN_DESKTOP,
# OS_ID=macos, ARCH_ID (aarch64|x86_64), PACKAGING_MACOS_UNIVERSAL,
# TARGET_DIR (default "target"), DIST_DIR (default "dist"),
# APP_DIR (where the .app is staged; default "bundle").
#
# Output: DIST_DIR/<slug>-<version>-<arch>-macos.dmg (+ signing
# sidecars). POSIX sh; `bash -n` clean.
#
# HONEST LOCAL-PROOF BOUNDARY: authored on Linux — the .app staging is
# pure file operations (locally provable), the codesign/notarize/
# hdiutil steps need macOS and are CI-only [VERIFY at first live run].

set -eu

: "${PRODUCT_NAME:?build-dmg.sh: PRODUCT_NAME not set (run packaging-env.py first)}"
: "${PRODUCT_LOWER:?build-dmg.sh: PRODUCT_LOWER not set}"
: "${PRODUCT_VERSION:?build-dmg.sh: PRODUCT_VERSION not set}"
: "${BIN_DESKTOP:?build-dmg.sh: BIN_DESKTOP not set}"
: "${OS_ID:?build-dmg.sh: OS_ID not set}"
: "${ARCH_ID:?build-dmg.sh: ARCH_ID not set}"
: "${PACKAGING_MACOS_UNIVERSAL:?build-dmg.sh: PACKAGING_MACOS_UNIVERSAL not set (run packaging-env.py first)}"
TARGET_DIR=${TARGET_DIR:-target}
DIST_DIR=${DIST_DIR:-dist}
APP_DIR=${APP_DIR:-bundle}

ROOT=$(cd "$(dirname -- "$0")/../.." && pwd)
cd "$ROOT"

die() { echo "build-dmg.sh: error: $*" >&2; exit 1; }

[ "$OS_ID" = "macos" ] || die "build-dmg.sh is the macOS lane (OS_ID=${OS_ID})"

case "$ARCH_ID" in
    aarch64|x86_64) ;;
    *) die "unexpected macOS ARCH_ID=${ARCH_ID}" ;;
esac

# The universal-binary decision must match what this script can build.
if [ "$PACKAGING_MACOS_UNIVERSAL" != "false" ]; then
    die "packaging.toml [macos] universal = ${PACKAGING_MACOS_UNIVERSAL}, but the universal2 (lipo) build is NOT implemented in v1 — flip the config only together with the implementing card (docs/packaging/README.md §macOS)"
fi

mkdir -p "$DIST_DIR" "$APP_DIR"

# 1. Bundle layout (config-driven; wave-22 generator).
APP_DIR="$APP_DIR" scripts/release/make-app.sh

# 2. codesign (real when credentials exist; R-35 sidecar when not).
scripts/packaging/sign-macos.sh --app "$APP_DIR/${PRODUCT_NAME}.app"

# 3. Disk image (hdiutil; wave-22 generator).
APP_DIR="$APP_DIR" scripts/release/make-dmg.sh

# 4. Sign + notarize + staple the image (carries the signed .app).
dmg="$DIST_DIR/${PRODUCT_LOWER}-${PRODUCT_VERSION}-${ARCH_ID}-${OS_ID}.dmg"
[ -s "$dmg" ] || die "expected dmg not found after make-dmg.sh: $dmg"
scripts/packaging/sign-macos.sh --dmg "$dmg"

echo "build-dmg.sh: lane complete — $dmg (+ sidecars if unsigned: see above)"
