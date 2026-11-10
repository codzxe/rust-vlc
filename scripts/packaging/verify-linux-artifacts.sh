#!/bin/sh
# verify-linux-artifacts.sh — the 8.E3 Linux lane PROOF script (issue
# #56, card 8.E3, task W24-C): build the real packages, then
# LAUNCH-TEST the packaged binaries (real version flag, real
# decode-bench smoke on a real fixture), then wire SBOM + checksums.
#
# Runs LOCALLY (this is the sandbox-provable lane) and in the release
# workflow's Linux jobs (same script, same assertions — no CI-only
# divergence). Everything config-driven via packaging-env.py; the only
# inputs are the release binaries and the pinned AppImage tools.
#
# Steps:
#   1. scripts/packaging/build-deb.sh       — REAL .deb (native dpkg-deb)
#                                            from the shared FHS payload
#                                            (config-driven: dirs, docs,
#                                            desktop entry, section)
#   2. scripts/packaging/build-appimage.sh  — REAL .AppImage (pinned
#                                            appimagetool + type2 runtime
#                                            via APPIMAGETOOL/RUNTIME_FILE,
#                                            shared desktop entry)
#   3. build-rpm.sh --spec-only            — generated + structurally
#                                            validated .spec (the binary
#                                            rpm build is the CI step)
#   4. deb launch tests (dpkg-deb -x, no root needed):
#        <bindir>/<cli> --version          exit 0
#        <bindir>/<cli> decode-bench <fx>  exit 0 + one BENCH line
#        <bindir>/<desktop>                exit = the documented
#                                            skeleton code (config)
#        dpkg-deb -f Package/Version/Architecture == config values
#        shipped docs present per packaging.toml [shipped] docs
#   5. AppImage launch test: APPIMAGE_EXTRACT_AND_RUN=1 <image> exits
#      with the desktop skeleton code (no FUSE needed — same mechanism
#      as the build step).
#   6. Per-artifact-set SBOM: cargo run -p xtask --locked -- sbom
#      --out dist/<base>.<suffix> (CycloneDX 1.5, lockfile-derived —
#      content identical per set, generated per artifact so each
#      download is self-contained).
#   7. Checksum manifest: dist/<manifest> over every artifact, with
#      the algorithm from packaging.toml [checksum].
#
# Usage: verify-linux-artifacts.sh   (env from packaging-env.py +
#        APPIMAGETOOL + RUNTIME_FILE + built release binaries)
# POSIX sh; `bash -n` clean.

set -eu

: "${PRODUCT_LOWER:?verify-linux-artifacts.sh: PRODUCT_LOWER not set (run packaging-env.py first)}"
: "${PRODUCT_VERSION:?verify-linux-artifacts.sh: PRODUCT_VERSION not set}"
: "${PRODUCT_NAME:?verify-linux-artifacts.sh: PRODUCT_NAME not set}"
: "${BIN_CLI:?verify-linux-artifacts.sh: BIN_CLI not set}"
: "${BIN_DESKTOP:?verify-linux-artifacts.sh: BIN_DESKTOP not set}"
: "${OS_ID:?verify-linux-artifacts.sh: OS_ID not set}"
: "${ARCH_ID:?verify-linux-artifacts.sh: ARCH_ID not set}"
: "${PACKAGING_LINUX_BINDIR:?verify-linux-artifacts.sh: PACKAGING_LINUX_BINDIR not set}"
: "${PACKAGING_LINUX_DOCS_DIR:?verify-linux-artifacts.sh: PACKAGING_LINUX_DOCS_DIR not set}"
: "${PACKAGING_DOCS:?verify-linux-artifacts.sh: PACKAGING_DOCS not set (packaging.toml shipped.docs)}"
: "${PACKAGING_LAUNCH_VERSION_FLAG:?verify-linux-artifacts.sh: PACKAGING_LAUNCH_VERSION_FLAG not set}"
: "${PACKAGING_LAUNCH_SMOKE_VERB:?verify-linux-artifacts.sh: PACKAGING_LAUNCH_SMOKE_VERB not set}"
: "${PACKAGING_LAUNCH_SMOKE_FIXTURE:?verify-linux-artifacts.sh: PACKAGING_LAUNCH_SMOKE_FIXTURE not set}"
: "${PACKAGING_LAUNCH_DESKTOP_PENDING_EXIT:?verify-linux-artifacts.sh: PACKAGING_LAUNCH_DESKTOP_PENDING_EXIT not set}"
: "${PACKAGING_CHECKSUM_ALGORITHM:?verify-linux-artifacts.sh: PACKAGING_CHECKSUM_ALGORITHM not set}"
: "${PACKAGING_CHECKSUM_MANIFEST:?verify-linux-artifacts.sh: PACKAGING_CHECKSUM_MANIFEST not set}"
: "${PACKAGING_SBOM_ARTIFACT_SUFFIX:?verify-linux-artifacts.sh: PACKAGING_SBOM_ARTIFACT_SUFFIX not set}"
: "${APPIMAGETOOL:?verify-linux-artifacts.sh: APPIMAGETOOL not set (pinned appimagetool download — see docs/packaging/README.md)}"
: "${RUNTIME_FILE:?verify-linux-artifacts.sh: RUNTIME_FILE not set (pinned AppImage type2 runtime)}"
TARGET_DIR=${TARGET_DIR:-target}
DIST_DIR=${DIST_DIR:-dist}

ROOT=$(cd "$(dirname -- "$0")/../.." && pwd)
cd "$ROOT"

die() { echo "verify-linux-artifacts.sh: error: $*" >&2; exit 1; }

[ "$OS_ID" = "linux" ] || die "this is the Linux lane (OS_ID=${OS_ID})"
[ -x "$APPIMAGETOOL" ] || die "APPIMAGETOOL is not executable: $APPIMAGETOOL"
[ -f "$RUNTIME_FILE" ] || die "RUNTIME_FILE not found: $RUNTIME_FILE"
[ -f "$PACKAGING_LAUNCH_SMOKE_FIXTURE" ] ||
    die "smoke fixture not found: $PACKAGING_LAUNCH_SMOKE_FIXTURE (packaging.toml launch_test.smoke_fixture)"

mkdir -p "$DIST_DIR"
dist_abs=$(cd "$DIST_DIR" && pwd)
base="${PRODUCT_LOWER}-${PRODUCT_VERSION}-${ARCH_ID}-${OS_ID}"

echo "=== 1. build .deb (native dpkg-deb; config-driven payload) ==="
scripts/packaging/build-deb.sh

echo "=== 2. build .AppImage (pinned appimagetool; shared desktop entry) ==="
scripts/packaging/build-appimage.sh

echo "=== 3. generate + validate the rpm .spec ==="
scripts/packaging/build-rpm.sh --spec-only "$dist_abs/${base}.spec"

deb="$dist_abs/${base}.deb"
appimage="$dist_abs/${base}.AppImage"
[ -s "$deb" ] || die "deb missing after build: $deb"
[ -s "$appimage" ] || die "AppImage missing after build: $appimage"

echo "=== 4. deb launch tests (extraction — no root) ==="
deb_root=$(mktemp -d)
trap 'rm -rf "$deb_root"' EXIT INT TERM
dpkg-deb -x "$deb" "$deb_root" || die "dpkg-deb -x failed on $deb"

cli="$deb_root/$PACKAGING_LINUX_BINDIR/$BIN_CLI"
desktop="$deb_root/$PACKAGING_LINUX_BINDIR/$BIN_DESKTOP"
[ -x "$cli" ] || die "packaged CLI not present in the deb: $PACKAGING_LINUX_BINDIR/$BIN_CLI"
[ -x "$desktop" ] || die "packaged desktop binary not present in the deb"

# 4a. version flag — the packaged engine binary launches.
cli_out=$("$cli" "$PACKAGING_LAUNCH_VERSION_FLAG") ||
    die "packaged CLI $BIN_CLI $PACKAGING_LAUNCH_VERSION_FLAG exited non-zero"
echo "  CLI ${PACKAGING_LAUNCH_VERSION_FLAG}: $cli_out"

# 4b. decode smoke — the packaged engine really opens + decodes a real
# fixture through the open path (the decode-bench host verb).
smoke_out=$("$cli" "$PACKAGING_LAUNCH_SMOKE_VERB" "$PACKAGING_LAUNCH_SMOKE_FIXTURE" 2>&1) ||
    die "packaged CLI smoke decode failed:
$smoke_out"
echo "$smoke_out" | grep -q "BENCH" ||
    die "smoke decode produced no BENCH line:
$smoke_out"
echo "  CLI smoke decode: $(echo "$smoke_out" | grep "BENCH" | head -1)"

# 4c. desktop binary — the documented skeleton contract, asserted
# as-is (never faked, never skipped).
desktop_out=$("$desktop" 2>&1) && desktop_rc=0 || desktop_rc=$?
[ "$desktop_rc" -eq "$PACKAGING_LAUNCH_DESKTOP_PENDING_EXIT" ] ||
    die "packaged desktop binary exited $desktop_rc (expected the documented skeleton code $PACKAGING_LAUNCH_DESKTOP_PENDING_EXIT):
$desktop_out"
echo "  desktop skeleton: exit $desktop_rc (as documented)"

# 4d. control metadata matches the config (the deb IS what config says).
case "$ARCH_ID" in
    x86_64) want_arch=amd64 ;;
    aarch64) want_arch=arm64 ;;
    *) die "cannot map ARCH_ID=${ARCH_ID} to a Debian architecture" ;;
esac
for field in "Package:$PRODUCT_LOWER" "Version:$PRODUCT_VERSION" "Architecture:$want_arch"; do
    want_field=${field%%:*}
    want_val=${field#*:}
    got_val=$(dpkg-deb -f "$deb" "$want_field")
    [ "$got_val" = "$want_val" ] ||
        die "deb control field $want_field is '$got_val', config says '$want_val'"
    echo "  deb control: $want_field=$got_val"
done

# 4e. shipped docs present in the extracted tree, per packaging.toml
# [shipped] docs (the same list the stager installed — cross-checked,
# not assumed).
for doc in $PACKAGING_DOCS; do
    [ -f "$deb_root/$PACKAGING_LINUX_DOCS_DIR/$PRODUCT_LOWER/$doc" ] ||
        die "shipped doc missing from the deb payload: $doc (packaging.toml shipped.docs)"
done
echo "  deb payload docs: $PACKAGING_DOCS"

echo "=== 5. AppImage launch test (extract-and-run; no FUSE) ==="
appimage_out=$(APPIMAGE_EXTRACT_AND_RUN=1 "$appimage" 2>&1) && app_rc=0 || app_rc=$?
[ "$app_rc" -eq "$PACKAGING_LAUNCH_DESKTOP_PENDING_EXIT" ] ||
    die "AppImage exited $app_rc (expected the documented skeleton code $PACKAGING_LAUNCH_DESKTOP_PENDING_EXIT):
$appimage_out"
echo "  AppImage skeleton: exit $app_rc (as documented)"

echo "=== 6. per-artifact-set SBOM (xtask sbom, CycloneDX 1.5) ==="
for artifact_base in "$base.deb" "$base.AppImage" "$base.spec"; do
    sbom_out="$dist_abs/${artifact_base}.${PACKAGING_SBOM_ARTIFACT_SUFFIX}"
    cargo run -p xtask --locked -- sbom --out "$sbom_out" >/dev/null ||
        die "xtask sbom failed for $sbom_out"
    [ -s "$sbom_out" ] || die "SBOM missing or empty: $sbom_out"
    echo "  SBOM: $sbom_out ($(wc -c <"$sbom_out") bytes)"
done

echo "=== 7. checksum manifest ==="
case "$PACKAGING_CHECKSUM_ALGORITHM" in
    sha256) hasher=sha256sum ;;
    *) die "no wired hasher for algorithm ${PACKAGING_CHECKSUM_ALGORITHM}" ;;
esac
manifest="$dist_abs/$PACKAGING_CHECKSUM_MANIFEST"
: >"$manifest"
for artifact in "$deb" "$appimage" "$dist_abs/${base}.spec" \
    "$dist_abs/${base}.deb.${PACKAGING_SBOM_ARTIFACT_SUFFIX}" \
    "$dist_abs/${base}.AppImage.${PACKAGING_SBOM_ARTIFACT_SUFFIX}" \
    "$dist_abs/${base}.spec.${PACKAGING_SBOM_ARTIFACT_SUFFIX}"; do
    "$hasher" "$artifact" >>"$manifest"
done
echo "  manifest: $manifest"
cat "$manifest"

echo "verify-linux-artifacts.sh: PASS — deb + AppImage built, packaged binaries launch-tested, SBOMs + checksums written"
