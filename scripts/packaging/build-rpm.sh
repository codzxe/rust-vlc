#!/bin/sh
# build-rpm.sh — generate (and, where rpmbuild exists, build) the RPM
# package (issue #56, card 8.E3, task W24-C).
#
# Two modes:
#   build-rpm.sh --spec-only <out.spec>
#       Generate the .spec fully from the config environment, stage the
#       payload, and STRUCTURALLY VALIDATE: required tags present, RPM
#       Version has no dash (pre-release mapped to ~), every %files
#       entry exists in the staging tree. Exits 0 — this is the
#       locally-provable half (runs on any POSIX host; it is what
#       scripts/packaging/gate.sh and full_gate step 11 exercise).
#   build-rpm.sh
#       Full binary build: requires rpmbuild (Fedora-family host or a
#       CI lane with the rpm package installed). Stages the same tree
#       the deb ships, tars it as Source0, and runs rpmbuild -bb.
#
# Spec content is 100% config-driven (packaging-env.py env):
#   Name        <- PRODUCT_LOWER (identity.name slug, product.toml)
#   Version     <- PRODUCT_VERSION with "-" -> "~" (RPM pre-release
#                  ordering: 0.1.0~dev < 0.1.0)
#   Release     <- PACKAGING_RPM_RELEASE (packaging.toml)
#   Summary     <- PRODUCT_DESCRIPTION (locale catalog via product.toml)
#   License     <- PACKAGING_RPM_LICENSE (mirrors the repo LICENSE)
#   Requires    <- gtk3 + webkit2gtk4.1 (script-local: the tauri
#                  admission shell's webview runtime — no rpm_requires
#                  config key exists; same decision as build-deb.sh's
#                  Depends rows)
#   files       <- PACKAGING_LINUX_BINDIR / _DOCS_DIR /
#                  _APPLICATIONS_DIR / _ICONS_DIR + BIN_CLI/BIN_DESKTOP
#                  + BIN_TAURI (cargo metadata) +
#                  PACKAGING_DESKTOP_CATEGORIES /
#                  PACKAGING_DESKTOP_TERMINAL for the entry file
#
# HONEST LOCAL-PROOF BOUNDARY: this sandbox has no rpmbuild — the
# binary build is CI-only [VERIFY at first live run]; spec generation
# + validation run here and in the gate. The workflow's Linux jobs
# `apt-get install rpm` first (ubuntu images carry it).
#
# POSIX sh; `bash -n` clean.

set -eu

: "${PRODUCT_LOWER:?build-rpm.sh: PRODUCT_LOWER not set (run packaging-env.py first)}"
: "${PRODUCT_VERSION:?build-rpm.sh: PRODUCT_VERSION not set}"
: "${PRODUCT_DESCRIPTION:?build-rpm.sh: PRODUCT_DESCRIPTION not set}"
: "${PRODUCT_PUBLISHER:?build-rpm.sh: PRODUCT_PUBLISHER not set}"
: "${PRODUCT_NAME:?build-rpm.sh: PRODUCT_NAME not set}"
: "${PRODUCT_APPSTREAM_ID:?build-rpm.sh: PRODUCT_APPSTREAM_ID not set}"
: "${PRODUCT_ACCENT_COLOR:?build-rpm.sh: PRODUCT_ACCENT_COLOR not set}"
: "${BIN_CLI:?build-rpm.sh: BIN_CLI not set}"
: "${BIN_DESKTOP:?build-rpm.sh: BIN_DESKTOP not set}"
: "${BIN_TAURI:?build-rpm.sh: BIN_TAURI not set}"
: "${OS_ID:?build-rpm.sh: OS_ID not set}"
: "${ARCH_ID:?build-rpm.sh: ARCH_ID not set}"
: "${PACKAGING_LINUX_BINDIR:?build-rpm.sh: PACKAGING_LINUX_BINDIR not set}"
: "${PACKAGING_LINUX_DOCS_DIR:?build-rpm.sh: PACKAGING_LINUX_DOCS_DIR not set}"
: "${PACKAGING_LINUX_APPLICATIONS_DIR:?build-rpm.sh: PACKAGING_LINUX_APPLICATIONS_DIR not set}"
: "${PACKAGING_LINUX_ICONS_DIR:?build-rpm.sh: PACKAGING_LINUX_ICONS_DIR not set}"
: "${PACKAGING_RPM_RELEASE:?build-rpm.sh: PACKAGING_RPM_RELEASE not set}"
: "${PACKAGING_RPM_LICENSE:?build-rpm.sh: PACKAGING_RPM_LICENSE not set}"
: "${PACKAGING_DESKTOP_CATEGORIES:?build-rpm.sh: PACKAGING_DESKTOP_CATEGORIES not set}"
: "${PACKAGING_DESKTOP_TERMINAL:?build-rpm.sh: PACKAGING_DESKTOP_TERMINAL not set}"
: "${PACKAGING_DEB_SECTION:?build-rpm.sh: PACKAGING_DEB_SECTION not set (shared staging sanity)}"
: "${PACKAGING_DOCS:?build-rpm.sh: PACKAGING_DOCS not set (packaging.toml shipped.docs — the %files docs rows iterate it)}"
TARGET_DIR=${TARGET_DIR:-target}
DIST_DIR=${DIST_DIR:-dist}

ROOT=$(cd "$(dirname -- "$0")/../.." && pwd)
cd "$ROOT"

die() { echo "build-rpm.sh: error: $*" >&2; exit 1; }

spec_only=0
spec_out=""
case "${1-}" in
    --spec-only)
        spec_only=1
        spec_out=${2-}
        [ -n "$spec_out" ] || die "--spec-only requires an output path"
        ;;
    "") ;;
    *) die "usage: $0 [--spec-only <out.spec>]" ;;
esac

[ "$OS_ID" = "linux" ] || die "build-rpm.sh is the Linux lane (OS_ID=${OS_ID})"

# RPM Version: dashes are illegal; map the semver pre-release marker
# to RPM's tilde (sorts BELOW the release, matching semver intent).
rpm_version=$(printf '%s' "$PRODUCT_VERSION" | sed 's/-/~/g')
case "$rpm_version" in
    *-*) die "internal: rpm version still contains a dash: $rpm_version" ;;
esac

# --- Stage the payload (the ONE shared FHS tree the deb ships) ---------------
# stage-linux-payload.sh owns the layout: bins, shipped docs, desktop
# entry (gen-desktop-entry.sh — the ONE writer) and icon, every path
# from packaging.toml [linux] / [shipped] / [desktop_entry].
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT INT TERM
payload="$stage/${PRODUCT_LOWER}-${PRODUCT_VERSION}"
mkdir -p "$payload"
"$ROOT/scripts/packaging/stage-linux-payload.sh" "$payload"

# The %files entries, generated BEFORE the heredoc so the spec stays
# one clean config-driven template: leading slash = install position
# (buildroot-relative), every path derived from the packaging config;
# the docs rows iterate the SAME packaging.toml [shipped] docs list the
# payload stager installed (single source — they cannot drift).
files_list=""
for entry in "/$PACKAGING_LINUX_BINDIR/$BIN_CLI" \
    "/$PACKAGING_LINUX_BINDIR/$BIN_DESKTOP" \
    "/$PACKAGING_LINUX_BINDIR/$BIN_TAURI" \
    "/$PACKAGING_LINUX_APPLICATIONS_DIR/${PRODUCT_APPSTREAM_ID}.desktop" \
    "/$PACKAGING_LINUX_ICONS_DIR/$PRODUCT_LOWER.png"
do
    files_list="$files_list$entry
"
done
for doc in $PACKAGING_DOCS; do
    files_list="$files_list/$PACKAGING_LINUX_DOCS_DIR/$PRODUCT_LOWER/$doc
"
done

# --- The .spec (generated; every field traced above) --------------------------
# NOTE: the %files list is appended AFTER the heredoc, not spliced into
# its delimiter line — a `<<EOF ... ${var}EOF` splice does NOT terminate
# the here-document (the delimiter is matched on the raw line, before
# expansion), which would silently swallow the rest of this script into
# the spec file. bash -n guards this class (see scripts/packaging/gate.sh).
spec="$stage/${PRODUCT_LOWER}.spec"
cat >"$spec" <<EOF
# Generated by scripts/packaging/build-rpm.sh — DO NOT HAND-EDIT.
# Every field comes from product.toml + packaging/packaging.toml (via
# packaging-env.py); regenerate instead of editing (MP §5).
Name:           ${PRODUCT_LOWER}
Version:        ${rpm_version}
Release:        ${PACKAGING_RPM_RELEASE}
Summary:        ${PRODUCT_DESCRIPTION}
License:        ${PACKAGING_RPM_LICENSE}
Packager:       ${PRODUCT_PUBLISHER}
Source0:        ${PRODUCT_LOWER}-${PRODUCT_VERSION}.tar.gz
# Requires: the tauri admission shell (BIN_TAURI) links WebKitGTK 4.1
# + GTK3 at run time; without these rows a clean Fedora host installs
# an rpm whose tauri binary cannot launch. Fedora package names; gtk3
# arrives transitively via webkit2gtk4.1 but is declared explicitly
# (the tauri binary links it directly). Script-local rows — no
# rpm_requires config key exists (same decision as the deb Depends;
# see scripts/packaging/build-deb.sh + the w26.m.t2 report).
Requires:       gtk3
Requires:       webkit2gtk4.1

%description
${PRODUCT_DESCRIPTION}
The shipped binaries (the engine-driven CLI, the desktop
composition-root binary and the tauri admission shell) install under
/${PACKAGING_LINUX_BINDIR}; this package ships files only, no
scriptlets.

%prep
# The tarball's top-level dir is made to MATCH %{name}-%{version} (the
# ~-mapped rpm_version) via tar --transform below — %setup's default cd
# then lands on the real payload dir. History: round 3 tried %setup -n
# (pinning the dash dir), but the hosted runner's rpm 4.2x ignored -n
# for the cd (run 43406493996: `cd lumen-0.1.0~dev` still fired, line
# 41) — the robust contract is the tarball layout, not macro options.
%setup -q

%build
# Binary package: the payload is built by cargo (rust toolchain), not
# by rpmbuild. Nothing to compile here.

%install
# %setup left cwd INSIDE the extracted top-dir (= the payload root:
# usr/, share/...); `cp -a .` copies the CONTENTS into the buildroot
# (verified locally: `cp -a . dest/` copies children, not the dir
# itself). The old `${name}-${version}/.` path was written for a
# buildroot-relative cwd and died "cannot stat" the moment %prep's cwd
# changed (aarch64, run 43406493996).
cp -a . "%{buildroot}/"

%files
EOF
printf '%s' "$files_list" >>"$spec"

spec_final="$spec"

# --- Structural validation (always — this is the local proof) -----------------
validate_spec() { # $1 = spec path, $2 = staging payload dir
    vspec=$1
    vpayload=$2
    vfail=0
    for tag in Name: Version: Release: Summary: License: %description %files; do
        if ! grep -q "^$tag" "$vspec"; then
            echo "build-rpm.sh: spec validation FAILED — missing $tag" >&2
            vfail=1
        fi
    done
    case $(sed -n 's/^Version:[[:space:]]*//p' "$vspec") in
        *-*) echo "build-rpm.sh: spec validation FAILED — Version contains a dash" >&2; vfail=1 ;;
    esac
    # The tauri shell's runtime-dep rows (the deb carries the same pair
    # in Depends): a dropped row would ship an rpm that installs cleanly
    # but cannot launch BIN_TAURI on a clean host — asserted here so the
    # spec-only local proof (this sandbox + the gate) catches it, not
    # the first CI rpmbuild.
    if ! grep -Eq '^Requires:[[:space:]]+gtk3[[:space:]]*$' "$vspec"; then
        echo "build-rpm.sh: spec validation FAILED — missing 'Requires: gtk3'" >&2
        vfail=1
    fi
    if ! grep -Eq '^Requires:[[:space:]]+webkit2gtk4\.1[[:space:]]*$' "$vspec"; then
        echo "build-rpm.sh: spec validation FAILED — missing 'Requires: webkit2gtk4.1'" >&2
        vfail=1
    fi
    # Every %files path must exist in the staged payload. (The check
    # runs in $(...) — a subshell — so it COLLECTS failures and the
    # parent decides, never the reverse.)
    missing=$(sed -n '/^%files$/,$p' "$vspec" | grep '^/' | while IFS= read -r entry; do
        [ -e "$vpayload$entry" ] || printf '%s\n' "$entry"
    done)
    if [ -n "$missing" ]; then
        echo "build-rpm.sh: spec validation FAILED — %files entries not staged:" >&2
        printf '%s\n' "$missing" >&2
        vfail=1
    fi
    return $vfail
}

validate_spec "$spec_final" "$payload" ||
    die "spec validation failed (see messages above)"

if [ "$spec_only" -eq 1 ]; then
    mkdir -p "$(dirname -- "$spec_out")"
    cp "$spec_final" "$spec_out"
    echo "build-rpm.sh: wrote + validated $spec_out (rpm_version=${rpm_version})"
    echo "build-rpm.sh: spec-only mode — binary build needs rpmbuild (CI lane; local proof covers generation + validation only)"
    exit 0
fi

# --- Full binary build --------------------------------------------------------
command -v rpmbuild >/dev/null 2>&1 ||
    die "rpmbuild not found — the RPM binary build needs a Fedora-family host or the CI lane (workflow installs the rpm package); use --spec-only for the locally-provable half"

case "$ARCH_ID" in
    x86_64|aarch64) rpm_arch="$ARCH_ID" ;;
    *) die "cannot map ARCH_ID=${ARCH_ID} to an rpm architecture" ;;
esac

tarball="$stage/${PRODUCT_LOWER}-${PRODUCT_VERSION}.tar.gz"
# --transform renames the archive's top-level dir to the ~-mapped
# rpm_version so %setup's default `cd %{name}-%{version}` finds the
# payload (see the %prep comment; on-disk staging keeps the dash name
# the deb/AppImage lanes share).
tar -czf "$tarball" -C "$stage" \
    --transform "s,^${PRODUCT_LOWER}-${PRODUCT_VERSION},${PRODUCT_LOWER}-${rpm_version}," \
    "${PRODUCT_LOWER}-${PRODUCT_VERSION}"

mkdir -p "$stage/SOURCES" "$stage/RPMS" "$stage/BUILD" "$stage/SRPMS" "$stage/SPECS"
cp "$tarball" "$stage/SOURCES/"
cp "$spec_final" "$stage/SPECS/${PRODUCT_LOWER}.spec"

rpmbuild --define "_topdir $stage" -bb "$stage/SPECS/${PRODUCT_LOWER}.spec" ||
    die "rpmbuild failed"

mkdir -p "$DIST_DIR"
dist_abs=$(cd "$DIST_DIR" && pwd)
built=$(find "$stage/RPMS" -name "${PRODUCT_LOWER}-${rpm_version}-*.${rpm_arch}.rpm" -print -quit)
[ -n "$built" ] || die "rpmbuild produced no expected rpm for arch ${rpm_arch}"
cp "$built" "$dist_abs/${PRODUCT_LOWER}-${PRODUCT_VERSION}-${ARCH_ID}-${OS_ID}.rpm"
cp "$spec_final" "$dist_abs/${PRODUCT_LOWER}-${PRODUCT_VERSION}-${ARCH_ID}-${OS_ID}.spec"
echo "build-rpm.sh: wrote $dist_abs/${PRODUCT_LOWER}-${PRODUCT_VERSION}-${ARCH_ID}-${OS_ID}.rpm"
