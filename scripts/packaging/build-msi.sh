#!/bin/sh
# build-msi.sh — build the per-arch Windows MSI with WiX (issue #56,
# card 8.E1, task W24-C).
#
# Refinement of the wave-22 flow (scripts/release/package-windows.sh
# builds zip + MSI in one go with a fixed x64 arch). This script is the
# 8.E1 lane: a STANDALONE per-arch MSI builder that
#   * re-uses scripts/release/gen-wxs.sh as the ONE config-driven .wxs
#     generator (product name, version, publisher, description and the
#     frozen MSI UpgradeCode all come from product.toml through
#     product-env.py — the UpgradeCode is stable across versions so
#     upgrades REPLACE, never co-install);
#   * maps ARCH_ID -> the WiX -arch platform flag (x86_64 -> x64,
#     aarch64 -> arm64), so an arm64 Windows lane is the same script
#     with a different ARCH_ID;
#   * runs `wix build` from a staging dir so File/@Source basenames
#     resolve — the staging dir carries the exes AND the [shipped] docs
#     list (PACKAGING_DOCS, staged below; the .wxs references every doc
#     by basename — W24-R1 P2-1: the docs list reaches the MSI exactly
#     as it reaches tar.gz/zip/deb/rpm/AppImage);
#   * hands the finished MSI to sign-windows.sh (signtool, secrets-
#     driven, explicit .UNSIGNED.txt fallback — card 8.E1's external
#     certificate blocker is documented there).
#
# Inputs (environment; set by the release workflow via
# scripts/packaging/packaging-env.py, which wraps product-env.py):
#   PRODUCT_NAME, PRODUCT_PUBLISHER, PRODUCT_DESCRIPTION, MSI_VERSION,
#   PRODUCT_MSI_UPGRADE_CODE, BIN_CLI, BIN_DESKTOP, BIN_TAURI, OS_ID,
#   ARCH_ID,
#   PACKAGING_WINDOWS_BINDIR (bin dir under ProgramFiles64Folder\<Name>),
#   PACKAGING_DOCS (packaging.toml [shipped] docs — staged + referenced
#                   by the generated .wxs),
#   TARGET_DIR (cargo build dir, default "target"),
#   DIST_DIR   (output dir, default "dist"),
#   WIX_EXE    (path to wix.exe; REQUIRED — the workflow installs the
#               pinned WiX 5.0.2 nupkg and exports this)
#
# Output: DIST_DIR/<slug>-<version>-<arch>-windows.msi (+ the .wxs it
# was generated from, kept beside it for auditability, + signing
# sidecars from sign-windows.sh).
#
# Usage: build-msi.sh
# POSIX sh (git-bash on the Windows runner); `bash -n` clean.
#
# HONEST LOCAL-PROOF BOUNDARY: this sandbox is Linux — the .wxs
# GENERATION and every env assertion are executed locally in
# scripts/packaging/gate.sh (spec generation + XML well-formedness via
# python), but the `wix build` invocation needs Windows + .NET and is
# CI-only [VERIFY at first live run].

set -eu

: "${PRODUCT_NAME:?build-msi.sh: PRODUCT_NAME not set (run packaging-env.py first)}"
: "${PRODUCT_PUBLISHER:?build-msi.sh: PRODUCT_PUBLISHER not set}"
: "${PRODUCT_DESCRIPTION:?build-msi.sh: PRODUCT_DESCRIPTION not set}"
: "${MSI_VERSION:?build-msi.sh: MSI_VERSION not set}"
: "${PRODUCT_MSI_UPGRADE_CODE:?build-msi.sh: PRODUCT_MSI_UPGRADE_CODE not set}"
: "${PRODUCT_LOWER:?build-msi.sh: PRODUCT_LOWER not set}"
: "${PRODUCT_VERSION:?build-msi.sh: PRODUCT_VERSION not set}"
: "${BIN_CLI:?build-msi.sh: BIN_CLI not set}"
: "${BIN_DESKTOP:?build-msi.sh: BIN_DESKTOP not set}"
# The tauri admission bin: REQUIRED like the other two — the generated
# .wxs carries a BinTauri File row, so staging it is not optional.
: "${BIN_TAURI:?build-msi.sh: BIN_TAURI not set (run packaging-env.py first)}"
: "${OS_ID:?build-msi.sh: OS_ID not set}"
: "${ARCH_ID:?build-msi.sh: ARCH_ID not set}"
: "${PACKAGING_WINDOWS_BINDIR:?build-msi.sh: PACKAGING_WINDOWS_BINDIR not set (run packaging-env.py first)}"
: "${PACKAGING_DOCS:?build-msi.sh: PACKAGING_DOCS not set (packaging.toml shipped.docs — staged for the .wxs File rows)}"
: "${WIX_EXE:?build-msi.sh: WIX_EXE not set (the workflow installs the pinned WiX nupkg and exports WIX_EXE)}"
TARGET_DIR=${TARGET_DIR:-target}
DIST_DIR=${DIST_DIR:-dist}

ROOT=$(cd "$(dirname -- "$0")/../.." && pwd)
cd "$ROOT"

die() { echo "build-msi.sh: error: $*" >&2; exit 1; }

[ "$OS_ID" = "windows" ] || die "build-msi.sh is the Windows lane (OS_ID=${OS_ID})"
[ -x "$WIX_EXE" ] || die "WIX_EXE is not executable: $WIX_EXE"

# Per-arch platform flag (WiX v5 -arch): the MSI platform must match
# the binaries it carries.
case "$ARCH_ID" in
    x86_64) wix_arch=x64 ;;
    aarch64) wix_arch=arm64 ;;
    *) die "cannot map ARCH_ID=${ARCH_ID} to a WiX platform" ;;
esac

# staging: exes at the archive root (File/@Source basenames) and under
# the configured install bindir for the MSI layout.
release_dir="$TARGET_DIR/release"
cli_exe="${BIN_CLI}.exe"
desktop_exe="${BIN_DESKTOP}.exe"
tauri_exe="${BIN_TAURI}.exe"
for f in "$release_dir/$cli_exe" "$release_dir/$desktop_exe" "$release_dir/$tauri_exe"; do
    [ -f "$f" ] || die "missing packaging input: $f (build first: cargo build --release --locked)"
done

mkdir -p "$DIST_DIR"
dist_abs=$(cd "$DIST_DIR" && pwd)
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT INT TERM

install -m 0755 "$release_dir/$cli_exe" "$stage/$cli_exe"
install -m 0755 "$release_dir/$desktop_exe" "$stage/$desktop_exe"
install -m 0755 "$release_dir/$tauri_exe" "$stage/$tauri_exe"

# Docs ride beside the exes in the staging dir: the generated .wxs
# references every [shipped] doc by basename (File/@Source basenames
# resolve from the wix build working directory — the same dir).
for doc in $PACKAGING_DOCS; do
    [ -f "$doc" ] || die "packaging.toml shipped.docs lists $doc — not found in the repo root"
    install -m 0644 "$doc" "$stage/"
done

base="${PRODUCT_LOWER}-${PRODUCT_VERSION}-${ARCH_ID}-${OS_ID}"

# ONE .wxs generator for the whole repo: the wave-22 script, config-
# driven end to end (identity from product.toml via the env this lane
# also consumes).
"$ROOT/scripts/release/gen-wxs.sh" "$stage/installer.wxs"

# Keep the generated source beside the artifact: the MSI's provenance
# (which UpgradeCode, which version, which files) stays auditable
# without re-running anything.
cp "$stage/installer.wxs" "$dist_abs/${base}.wxs"

msi_path="$dist_abs/${base}.msi"
( cd "$stage" && "$WIX_EXE" build -arch "$wix_arch" installer.wxs -o "$msi_path" ) ||
    die "wix build failed"
[ -s "$msi_path" ] || die "MSI is missing or empty: $msi_path"
echo "build-msi.sh: wrote $msi_path (arch=${wix_arch}, upgrade-code=${PRODUCT_MSI_UPGRADE_CODE})"

# Signing: real signtool step, secrets-driven; writes an explicit
# .UNSIGNED.txt sidecar when the (user-owned) certificate is absent.
"$ROOT/scripts/packaging/sign-windows.sh" "$msi_path"
