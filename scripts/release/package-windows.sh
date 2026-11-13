#!/bin/sh
# package-windows.sh — stage + package the Windows release artifacts
# (card 7.w22g, issue #56 release half).
#
# Inputs (environment; set by the release workflow via product-env.py):
#   PRODUCT_LOWER, PRODUCT_VERSION, PRODUCT_DESCRIPTION,
#   PRODUCT_PUBLISHER, MSI_VERSION, PRODUCT_MSI_UPGRADE_CODE,
#   BIN_CLI, BIN_DESKTOP, BIN_TAURI (all three shipped exes),
#   OS_ID (windows), ARCH_ID (e.g. x86_64),
#   TARGET_DIR (cargo build dir, default "target"),
#   DIST_DIR   (output dir, default "dist"),
#   WIX_EXE    (optional path to wix.exe; when set an MSI is also built)
#
# Outputs in DIST_DIR:
#   <slug>-<version>-<arch>-windows.zip   (the CLI, desktop and tauri
#                                          exes + LICENSE + README.md)
#   <slug>-<version>-<arch>-windows.msi   (only when WIX_EXE is set)
#
# The zip is made with `python -m zipfile` (stdlib, present on every
# GitHub runner image) rather than a runner-specific zip CLI — git-bash
# on windows-latest has no `zip`, GNU tar does not write zip, and 7z is
# only guaranteed on the Windows image. One tool, three images.
#
# POSIX sh (git-bash on Windows runs it); `bash -n` clean.

set -eu

: "${PRODUCT_LOWER:?package-windows.sh: PRODUCT_LOWER not set}"
: "${PRODUCT_VERSION:?package-windows.sh: PRODUCT_VERSION not set}"
: "${PRODUCT_DESCRIPTION:?package-windows.sh: PRODUCT_DESCRIPTION not set}"
: "${PRODUCT_PUBLISHER:?package-windows.sh: PRODUCT_PUBLISHER not set}"
: "${BIN_CLI:?package-windows.sh: BIN_CLI not set}"
: "${BIN_DESKTOP:?package-windows.sh: BIN_DESKTOP not set}"
# Third shipped binary: gen-wxs.sh requires it for the BinTauri File
# row, so the zip stages it too (REQUIRED like the other two).
: "${BIN_TAURI:?package-windows.sh: BIN_TAURI not set}"
: "${OS_ID:?package-windows.sh: OS_ID not set}"
: "${ARCH_ID:?package-windows.sh: ARCH_ID not set}"
MSI_VERSION=${MSI_VERSION:-}
PRODUCT_MSI_UPGRADE_CODE=${PRODUCT_MSI_UPGRADE_CODE:-}
WIX_EXE=${WIX_EXE:-}
TARGET_DIR=${TARGET_DIR:-target}
DIST_DIR=${DIST_DIR:-dist}

ROOT=$(cd "$(dirname -- "$0")/../.." && pwd)
cd "$ROOT"

die() { echo "package-windows.sh: error: $*" >&2; exit 1; }

PY=$(command -v python3 || command -v python) ||
    die "neither python3 nor python found on PATH (zip creation needs the stdlib zipfile module)"

release_dir="$TARGET_DIR/release"
cli_exe="${BIN_CLI}.exe"
desktop_exe="${BIN_DESKTOP}.exe"
tauri_exe="${BIN_TAURI}.exe"
for f in "$release_dir/$cli_exe" "$release_dir/$desktop_exe" "$release_dir/$tauri_exe" LICENSE README.md; do
    [ -f "$f" ] || die "missing packaging input: $f (build first: cargo build --release --locked)"
done

mkdir -p "$DIST_DIR"
dist_abs=$(cd "$DIST_DIR" && pwd)
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT INT TERM

install -m 0755 "$release_dir/$cli_exe" "$stage/$cli_exe"
install -m 0755 "$release_dir/$desktop_exe" "$stage/$desktop_exe"
install -m 0755 "$release_dir/$tauri_exe" "$stage/$tauri_exe"
cp LICENSE README.md "$stage/"

base="${PRODUCT_LOWER}-${PRODUCT_VERSION}-${ARCH_ID}-${OS_ID}"
zip_path="$dist_abs/${base}.zip"

# python -m zipfile stores entries under their given (relative) names —
# run from the staging dir so the archive root is flat.
( cd "$stage" && "$PY" -m zipfile -c "$zip_path" "$cli_exe" "$desktop_exe" "$tauri_exe" LICENSE README.md ) ||
    die "zip creation failed"
[ -s "$zip_path" ] || die "zip archive is missing or empty: $zip_path"
echo "package-windows.sh: wrote $zip_path"

if [ -n "$WIX_EXE" ]; then
    # MSI via WiX v5. The .wxs identity comes from product.toml (see
    # gen-wxs.sh); build from the staging dir so File/@Source basenames
    # resolve. -arch x64 selects ProgramFiles64Folder components.
    [ -n "$MSI_VERSION" ] || die "WIX_EXE set but MSI_VERSION not set"
    [ -n "$PRODUCT_MSI_UPGRADE_CODE" ] || die "WIX_EXE set but PRODUCT_MSI_UPGRADE_CODE not set"
    [ -x "$WIX_EXE" ] || die "WIX_EXE is not executable: $WIX_EXE"

    here=$(dirname -- "$0")
    "$here/gen-wxs.sh" "$stage/installer.wxs"

    msi_path="$dist_abs/${base}.msi"
    ( cd "$stage" && "$WIX_EXE" build -arch x64 installer.wxs -o "$msi_path" ) ||
        die "wix build failed"
    [ -s "$msi_path" ] || die "MSI is missing or empty: $msi_path"
    echo "package-windows.sh: wrote $msi_path"
else
    echo "package-windows.sh: WIX_EXE not set - MSI skipped (zip only)"
fi
