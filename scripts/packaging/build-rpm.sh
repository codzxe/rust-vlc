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
#   files       <- PACKAGING_LINUX_BINDIR / _DOCS_DIR /
#                  _APPLICATIONS_DIR / _ICONS_DIR + BIN_CLI/BIN_DESKTOP
#                  (cargo metadata) + PACKAGING_DESKTOP_CATEGORIES /
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

# --- Stage the payload (same tree the deb ships) -----------------------------
cli_bin="$TARGET_DIR/release/$BIN_CLI"
desktop_bin="$TARGET_DIR/release/$BIN_DESKTOP"
for f in "$cli_bin" "$desktop_bin" LICENSE README.md; do
    [ -f "$f" ] || die "missing packaging input: $f (build first: cargo build --release --locked)"
done

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT INT TERM
payload="$stage/${PRODUCT_LOWER}-${PRODUCT_VERSION}"
mkdir -p \
    "$payload/$PACKAGING_LINUX_BINDIR" \
    "$payload/$PACKAGING_LINUX_DOCS_DIR/$PRODUCT_LOWER" \
    "$payload/$PACKAGING_LINUX_APPLICATIONS_DIR" \
    "$payload/$PACKAGING_LINUX_ICONS_DIR"

install -m 0755 "$cli_bin" "$payload/$PACKAGING_LINUX_BINDIR/$BIN_CLI"
install -m 0755 "$desktop_bin" "$payload/$PACKAGING_LINUX_BINDIR/$BIN_DESKTOP"
cp LICENSE README.md "$payload/$PACKAGING_LINUX_DOCS_DIR/$PRODUCT_LOWER/"
chmod 0644 "$payload/$PACKAGING_LINUX_DOCS_DIR/$PRODUCT_LOWER/LICENSE" \
    "$payload/$PACKAGING_LINUX_DOCS_DIR/$PRODUCT_LOWER/README.md"

cat >"$payload/$PACKAGING_LINUX_APPLICATIONS_DIR/${PRODUCT_APPSTREAM_ID}.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=${PRODUCT_NAME}
Comment=${PRODUCT_DESCRIPTION}
Exec=${BIN_DESKTOP}
Icon=${PRODUCT_LOWER}
Terminal=${PACKAGING_DESKTOP_TERMINAL}
Categories=${PACKAGING_DESKTOP_CATEGORIES}
EOF

# Icon: prefer the real brand asset; fall back to the documented
# deterministic placeholder (scripts/release/gen-icon.py) until
# assets/brand exists.
icon_src="$payload/$PACKAGING_LINUX_ICONS_DIR/$PRODUCT_LOWER.png"
if [ -f "$PRODUCT_ICON_PNG_512" ]; then
    cp "$PRODUCT_ICON_PNG_512" "$icon_src"
else
    PY=$(command -v python3 || command -v python) ||
        die "neither python3 nor python found on PATH (icon generation needs the stdlib)"
    "$ROOT/scripts/release/gen-icon.py" "$icon_src" "$PRODUCT_ACCENT_COLOR"
fi
chmod 0644 \
    "$payload/$PACKAGING_LINUX_APPLICATIONS_DIR/${PRODUCT_APPSTREAM_ID}.desktop" \
    "$icon_src"

# The %files entries, generated BEFORE the heredoc so the spec stays
# one clean config-driven template: leading slash = install position
# (buildroot-relative), every path derived from the packaging config.
files_list=""
for entry in "/$PACKAGING_LINUX_BINDIR/$BIN_CLI" \
    "/$PACKAGING_LINUX_BINDIR/$BIN_DESKTOP" \
    "/$PACKAGING_LINUX_DOCS_DIR/$PRODUCT_LOWER/LICENSE" \
    "/$PACKAGING_LINUX_DOCS_DIR/$PRODUCT_LOWER/README.md" \
    "/$PACKAGING_LINUX_APPLICATIONS_DIR/${PRODUCT_APPSTREAM_ID}.desktop" \
    "/$PACKAGING_LINUX_ICONS_DIR/$PRODUCT_LOWER.png"
do
    files_list="$files_list$entry
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

%description
${PRODUCT_DESCRIPTION}
Both shipped binaries (the engine-driven CLI and the desktop
composition-root binary) install under /${PACKAGING_LINUX_BINDIR}; this
package ships files only, no scriptlets.

%prep
%setup -q

%build
# Binary package: the payload is built by cargo (rust toolchain), not
# by rpmbuild. Nothing to compile here.

%install
mkdir -p "%{buildroot}"
cp -a "${PRODUCT_LOWER}-${PRODUCT_VERSION}/." "%{buildroot}/"

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
tar -czf "$tarball" -C "$stage" "${PRODUCT_LOWER}-${PRODUCT_VERSION}"

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
