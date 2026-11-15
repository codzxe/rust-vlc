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
#   1. build-targz.sh    — the flat archive (the two shipped binaries
#                         — CLI + GUI shell — + the [shipped] docs
#                         list) — content cross-checked.
#   2. build-deb.sh       — REAL .deb (native dpkg-deb) from the shared
#                         FHS payload (config-driven: dirs, docs,
#                         desktop entry, section).
#   3. build-appimage.sh  — REAL .AppImage (pinned appimagetool + type2
#                         runtime via APPIMAGETOOL/RUNTIME_FILE,
#                         shared desktop entry).
#   4. rpm lane           — with rpmbuild (CI): REAL binary .rpm +
#                         header/payload assertions vs config; without
#                         (this Debian-family sandbox): spec generation
#                         + structural validation, honestly labeled.
#   5. deb launch tests (dpkg-deb -x, no root needed):
#        <bindir>/<cli> --version          exit 0
#        <bindir>/<cli> decode-bench <fx>  exit 0 + one BENCH line
#        <bindir>/<tauri> PRESENT          exec bit + path (no launch:
#                                            the GUI shell needs the
#                                            host's WebKitGTK, which
#                                            the deb declares in
#                                            Depends — launching it
#                                            here would test this
#                                            sandbox's display
#                                            stack, not the package)
#        <bindir>/<desktop> ABSENT         the exit-86 stub left the
#                                            payload (w30.8: it read
#                                            as "the desktop app" and
#                                            exited with nothing)
#        Depends carries the webkit pair   libgtk-3-0 +
#                                            libwebkit2gtk-4.1-0
#        dpkg-deb -f Package/Version/Architecture == config values
#        shipped docs present per packaging.toml [shipped] docs
#   6. AppImage payload contract (extract-and-run's successor, w30.8):
#      --appimage-extract (the type2 runtime's own extraction, no
#      FUSE) proves AppRun EXECS THE GUI SHELL (the re-point away
#      from the exit-86 stub) + the tauri payload row in the bindir.
#      Launching the GUI shell here would need a display + host
#      WebKitGTK (this lane has neither) — the launch verdicts live
#      in .github/workflows/tauri-launch.yml's xvfb lane, not here.
#   7. tar.gz member check against the same [shipped] list + the two
#      binary names (and the stub's ABSENCE — same w30.8 contract).
#   8. Per-artifact-set SBOM: gen-sboms.sh (the ONE writer — wraps
#      `cargo run -p xtask --locked -- sbom`, CycloneDX per config,
#      lockfile-derived; content identical per set, generated per
#      artifact so each download is self-contained).
#   9. Checksum manifest: dist/<manifest> over every artifact (+ each
#      SBOM), with the algorithm from packaging.toml [checksum].
#
# NOTE for CI (release.yml Linux jobs): the per-set manifest this
# script writes (PACKAGING_CHECKSUM_MANIFEST name) is RENAMED to
# <base>.<manifest> by the workflow right after this script, so the
# x86_64 and aarch64 sets do not collide when the release job
# flat-merges the dist artifacts; the release job then writes THE
# global manifest under the plain configured name.
#
# Usage: verify-linux-artifacts.sh   (env from packaging-env.py +
#        APPIMAGETOOL + RUNTIME_FILE + built release binaries)
# POSIX sh; `bash -n` clean.

set -eu

: "${PRODUCT_LOWER:?verify-linux-artifacts.sh: PRODUCT_LOWER not set (run packaging-env.py first)}"
: "${PRODUCT_VERSION:?verify-linux-artifacts.sh: PRODUCT_VERSION not set}"
: "${PRODUCT_NAME:?verify-linux-artifacts.sh: PRODUCT_NAME not set}"
: "${BIN_CLI:?verify-linux-artifacts.sh: BIN_CLI not set}"
# BIN_DESKTOP is asserted even though the stub is NOT a payload member
# since w30.8: the absence checks below pin that contract, and a future
# lane that re-adds the stub must do it deliberately, not by drift.
: "${BIN_DESKTOP:?verify-linux-artifacts.sh: BIN_DESKTOP not set}"
: "${BIN_TAURI:?verify-linux-artifacts.sh: BIN_TAURI not set}"
: "${OS_ID:?verify-linux-artifacts.sh: OS_ID not set}"
: "${ARCH_ID:?verify-linux-artifacts.sh: ARCH_ID not set}"
: "${PACKAGING_LINUX_BINDIR:?verify-linux-artifacts.sh: PACKAGING_LINUX_BINDIR not set}"
: "${PACKAGING_LINUX_DOCS_DIR:?verify-linux-artifacts.sh: PACKAGING_LINUX_DOCS_DIR not set}"
: "${PACKAGING_DOCS:?verify-linux-artifacts.sh: PACKAGING_DOCS not set (packaging.toml shipped.docs)}"
: "${PACKAGING_LAUNCH_VERSION_FLAG:?verify-linux-artifacts.sh: PACKAGING_LAUNCH_VERSION_FLAG not set}"
: "${PACKAGING_LAUNCH_SMOKE_VERB:?verify-linux-artifacts.sh: PACKAGING_LAUNCH_SMOKE_VERB not set}"
: "${PACKAGING_LAUNCH_SMOKE_FIXTURE:?verify-linux-artifacts.sh: PACKAGING_LAUNCH_SMOKE_FIXTURE not set}"
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

echo "=== 1. build .tar.gz (plain archive lane) ==="
scripts/packaging/build-targz.sh

echo "=== 2. build .deb (native dpkg-deb; config-driven payload) ==="
scripts/packaging/build-deb.sh

echo "=== 3. build .AppImage (pinned appimagetool; shared desktop entry) ==="
scripts/packaging/build-appimage.sh

echo "=== 4. rpm lane ==="
rpm_path=""
if command -v rpmbuild >/dev/null 2>&1; then
    # Full binary build (CI lane: the workflow installs the rpm package).
    scripts/packaging/build-rpm.sh
    rpm_path="$dist_abs/${base}.rpm"
    [ -s "$rpm_path" ] || die "rpm missing after build: $rpm_path"
    # Header assertions, no root needed: the rpm IS what config says.
    # The queryformat TAG is not always the display label uppercased:
    # rpm's architecture tag is ARCH, not ARCHITECTURE (round-4 dry run
    # 43410309730: %{ARCHITECTURE} expanded empty and the assert died on
    # a perfectly good rpm). Explicit label->tag pairs below.
    for field in "Name:NAME:$PRODUCT_LOWER" \
        "Version:VERSION:$(printf '%s' "$PRODUCT_VERSION" | sed 's/-/~/g')" \
        "Release:RELEASE:$PACKAGING_RPM_RELEASE" \
        "Summary:SUMMARY:$PRODUCT_DESCRIPTION" \
        "License:LICENSE:$PACKAGING_RPM_LICENSE" \
        "Architecture:ARCH:$ARCH_ID"; do
        want_field=${field%%:*}
        rest=${field#*:}
        tag=${rest%%:*}
        want_val=${rest#*:}
        got_val=$(rpm -qp --queryformat "%{$tag}" "$rpm_path" 2>/dev/null) ||
            die "rpm -qp failed on $rpm_path"
        [ "$got_val" = "$want_val" ] ||
            die "rpm header field $want_field is '$got_val', config says '$want_val'"
        echo "  rpm header: $want_field=$got_val"
    done
    # Payload rows: every shipped binary must be listed by the package
    # (rpm -qlp needs no root) — the CLI + the GUI shell (w30.8: the
    # exit-86 stub left the payload); a missing row means
    # build-rpm.sh's %files list drifted from the stager.
    rpm_qlp=$(rpm -qlp "$rpm_path") || die "rpm -qlp failed on $rpm_path"
    for bin_row in "$BIN_CLI" "$BIN_TAURI"; do
        printf '%s\n' "$rpm_qlp" | grep -qx "/$PACKAGING_LINUX_BINDIR/$bin_row" ||
            die "rpm payload is missing /$PACKAGING_LINUX_BINDIR/$bin_row"
        echo "  rpm payload: /$PACKAGING_LINUX_BINDIR/$bin_row"
    done
    if printf '%s\n' "$rpm_qlp" | grep -qx "/$PACKAGING_LINUX_BINDIR/$BIN_DESKTOP"; then
        die "rpm payload still carries the exit-86 stub /$PACKAGING_LINUX_BINDIR/$BIN_DESKTOP (w30.8: user payloads ship the CLI + the GUI shell only — the stub is a CI-only diagnostic)"
    fi
else
    # Honest local half: spec generation + structural validation only.
    echo "  rpmbuild absent on this host -- spec-only (the binary .rpm"
    echo "  build runs in the CI lane where rpm is installed; labeled,"
    echo "  never faked)"
    scripts/packaging/build-rpm.sh --spec-only "$dist_abs/${base}.spec"
    # Spec-only half: the %files rows are the payload contract — the
    # GUI shell row must be present exactly like the CLI's.
    # build-rpm.sh's validate_spec already proved every row exists in
    # the staged tree AND that the Requires pair is intact; this greps
    # the same contract back from the consumer side.
    for bin_row in "$BIN_CLI" "$BIN_TAURI"; do
        grep -Fxq "/$PACKAGING_LINUX_BINDIR/$bin_row" "$dist_abs/${base}.spec" ||
            die "rpm spec %files is missing the /$PACKAGING_LINUX_BINDIR/$bin_row row"
        echo "  rpm spec row: /$PACKAGING_LINUX_BINDIR/$bin_row"
    done
    if grep -Fxq "/$PACKAGING_LINUX_BINDIR/$BIN_DESKTOP" "$dist_abs/${base}.spec"; then
        die "rpm spec %files still lists the exit-86 stub /$PACKAGING_LINUX_BINDIR/$BIN_DESKTOP (w30.8: user payloads ship the CLI + the GUI shell only)"
    fi
fi

targz="$dist_abs/${base}.tar.gz"
deb="$dist_abs/${base}.deb"
appimage="$dist_abs/${base}.AppImage"
[ -s "$targz" ] || die "tar.gz missing after build: $targz"
[ -s "$deb" ] || die "deb missing after build: $deb"
[ -s "$appimage" ] || die "AppImage missing after build: $appimage"

echo "=== 5. deb launch tests (extraction — no root) ==="
deb_root=$(mktemp -d)
appimage_extract_dir=$(mktemp -d)
trap 'rm -rf "$deb_root" "$appimage_extract_dir"' EXIT INT TERM
dpkg-deb -x "$deb" "$deb_root" || die "dpkg-deb -x failed on $deb"

cli="$deb_root/$PACKAGING_LINUX_BINDIR/$BIN_CLI"
tauri="$deb_root/$PACKAGING_LINUX_BINDIR/$BIN_TAURI"
[ -x "$cli" ] || die "packaged CLI not present in the deb: $PACKAGING_LINUX_BINDIR/$BIN_CLI"
# The GUI shell rides in the same bindir. Presence + exec bit is the
# honest row here: it is a GUI binary that needs WebKitGTK 4.1 + GTK3
# from the HOST (the pair below asserts the deb DECLARES them) —
# launching it in this sandbox would test the sandbox's display
# stack, not the package. That boundary is stated, not silently
# skipped.
[ -x "$tauri" ] || die "packaged tauri binary not present in the deb: $PACKAGING_LINUX_BINDIR/$BIN_TAURI"
# w30.8: the exit-86 stub must NOT be in the payload (it read as
# "the desktop app" to v0.1.2 users and exited with nothing).
[ ! -e "$deb_root/$PACKAGING_LINUX_BINDIR/$BIN_DESKTOP" ] ||
    die "deb payload still carries the exit-86 stub $BIN_DESKTOP (w30.8: user payloads ship the CLI + the GUI shell only — the stub is a CI-only diagnostic)"

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

# 4c. desktop stub — ABSENT, asserted (w30.8): the v0.1.2 payload
# confusion ("download, install, loading, nothing") was users
# double-clicking the exit-86 skeleton; the stub is a CI-only
# diagnostic now, so a payload that re-stages it is a regression.
# (The deb-level absence is asserted above with the presence rows;
# this label documents why there is no launch assert here anymore.)
echo "  desktop stub: absent from the deb payload (as designed, w30.8)"

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

# 4f. the webview runtime rows the tauri shell needs (the Depends pair
# build-deb.sh declares, script-local like the rows themselves): a
# dropped row would install cleanly and break BIN_TAURI at launch time
# on a clean host.
deb_depends=$(dpkg-deb -f "$deb" Depends)
printf '%s\n' "$deb_depends" | grep -q "libgtk-3-0" ||
    die "deb Depends is missing libgtk-3-0 (the tauri shell's GTK runtime), got: '$deb_depends'"
printf '%s\n' "$deb_depends" | grep -q "libwebkit2gtk-4.1-0" ||
    die "deb Depends is missing libwebkit2gtk-4.1-0 (the tauri shell's webview), got: '$deb_depends'"
echo "  deb Depends: $deb_depends"

echo "=== 6. AppImage payload contract (extract; no FUSE) ==="
# w30.8: AppRun execs the REAL GUI shell (it used to exec the exit-86
# stub — the AppImage "launch test" was really a stub-exit assert).
# Launching the GUI shell here would need a display + host WebKitGTK
# (this lane has neither); the launch verdicts live in
# .github/workflows/tauri-launch.yml's xvfb lane. The honest release
# gate is therefore structural: extract the image (the type2
# runtime's own --appimage-extract works without FUSE; the arg is
# consumed by the runtime, not the app) and pin BOTH the AppRun
# exec target and the payload row.
(cd "$appimage_extract_dir" && "$appimage" --appimage-extract) >/dev/null ||
    die "AppImage --appimage-extract failed on $appimage"
apprun="$appimage_extract_dir/squashfs-root/AppRun"
[ -x "$apprun" ] || die "AppRun missing or not executable in the AppImage payload"
apprun_exec_line="exec \"\$HERE/$PACKAGING_LINUX_BINDIR/$BIN_TAURI\""
grep -Fq "$apprun_exec_line" "$apprun" ||
    die "AppRun does not exec the GUI shell (expected the line: $apprun_exec_line) — the w30.8 re-point away from the exit-86 stub regressed"
echo "  AppRun exec: $apprun_exec_line (the REAL GUI shell)"
[ -x "$appimage_extract_dir/squashfs-root/$PACKAGING_LINUX_BINDIR/$BIN_TAURI" ] ||
    die "tauri binary missing from the AppImage payload: $PACKAGING_LINUX_BINDIR/$BIN_TAURI"
echo "  AppImage payload: $PACKAGING_LINUX_BINDIR/$BIN_TAURI present"
if [ -e "$appimage_extract_dir/squashfs-root/$PACKAGING_LINUX_BINDIR/$BIN_DESKTOP" ]; then
    die "AppImage payload still carries the exit-86 stub $BIN_DESKTOP (w30.8: user payloads ship the GUI shell only)"
fi
echo "  AppImage stub: absent (as designed, w30.8)"

echo "=== 7. tar.gz content check (the shipped list, cross-checked) ==="
tar_list=$(tar -tzf "$targz")
for member in "$BIN_CLI" "$BIN_TAURI" $PACKAGING_DOCS; do
    printf '%s\n' "$tar_list" | grep -qx "$member" ||
        die "tar.gz member missing: $member (expected: the CLI + the GUI shell + packaging.toml [shipped] docs)"
done
if printf '%s\n' "$tar_list" | grep -qx "$BIN_DESKTOP"; then
    die "tar.gz still stages the exit-86 stub $BIN_DESKTOP (w30.8: user payloads ship the CLI + the GUI shell only — the stub is a CI-only diagnostic)"
fi
echo "  tar.gz members: $(printf '%s\n' "$tar_list" | tr '\n' ' ')"

echo "=== 8. per-artifact-set SBOM (gen-sboms.sh — the ONE writer) ==="
# The subjects are exactly this set's artifacts: tar.gz + deb +
# AppImage + the rpm (CI) or the spec (rpmbuild-less hosts).
set -- "$dist_abs/$base.tar.gz" "$dist_abs/$base.deb" "$dist_abs/$base.AppImage"
if [ -n "$rpm_path" ]; then
    set -- "$@" "$rpm_path"
else
    set -- "$@" "$dist_abs/$base.spec"
fi
scripts/packaging/gen-sboms.sh "$@"

echo "=== 9. checksum manifest ==="
case "$PACKAGING_CHECKSUM_ALGORITHM" in
    sha256) hasher=sha256sum ;;
    *) die "no wired hasher for algorithm ${PACKAGING_CHECKSUM_ALGORITHM}" ;;
esac
manifest="$dist_abs/$PACKAGING_CHECKSUM_MANIFEST"
: >"$manifest"
for artifact in $targz $deb $appimage ${rpm_path:-"$dist_abs/${base}.spec"} \
    "$dist_abs/${base}.tar.gz.${PACKAGING_SBOM_ARTIFACT_SUFFIX}" \
    "$dist_abs/${base}.deb.${PACKAGING_SBOM_ARTIFACT_SUFFIX}" \
    "$dist_abs/${base}.AppImage.${PACKAGING_SBOM_ARTIFACT_SUFFIX}" \
    "$dist_abs/${base}.$( [ -n "$rpm_path" ] && printf rpm || printf spec ).${PACKAGING_SBOM_ARTIFACT_SUFFIX}"; do
    "$hasher" "$artifact" >>"$manifest"
done
echo "  manifest: $manifest"
cat "$manifest"

echo "verify-linux-artifacts.sh: PASS — tar.gz + deb + AppImage (+ rpm where rpmbuild exists) built, the two shipped binaries (CLI launch-tested, GUI shell presence-asserted with the webkit runtime deps declared) verified per format, the exit-86 stub verifiably ABSENT from every payload (w30.8), AppRun execs the real GUI shell, SBOMs + checksums written"
