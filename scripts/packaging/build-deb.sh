#!/bin/sh
# build-deb.sh — build the REAL Debian package with the native dpkg-deb
# (issue #56, card 8.E3, task W24-C).
#
# The 8.E3 config-driven deb lane. It supersedes the wave-22
# scripts/release/package-deb.sh flow for release purposes: every
# packaging decision below comes from packaging/packaging.toml through
# packaging-env.py (install dirs, docs list, desktop-entry categories,
# deb Section), while product identity still comes from product.toml
# through the same bridge. No fpm, no cargo-deb — dpkg-deb only.
#
#   Package       <- PRODUCT_LOWER     (product.toml identity.name slug)
#   Version       <- PRODUCT_VERSION   (identity.display_version; Debian
#                     version syntax tolerates pre-release suffixes)
#   Section       <- PACKAGING_DEB_SECTION      (packaging.toml)
#   Architecture  <- ARCH_ID mapped (x86_64->amd64, aarch64->arm64)
#   Maintainer    <- PRODUCT_PUBLISHER
#   Description   <- PRODUCT_DESCRIPTION (locale catalog via the key in
#                     product.toml identity.description_key)
#   files         <- stage-linux-payload.sh (the ONE shared FHS tree)
#
# No maintainer scripts (postinst & co.): the package ships files only;
# desktop-entry caches refresh lazily.
#
# Usage: build-deb.sh
# Inputs: release binaries under TARGET_DIR/release, env from
#         scripts/packaging/packaging-env.py.
# Output: DIST_DIR/<slug>-<version>-<arch>-linux.deb
# POSIX sh; `bash -n` clean; EXECUTED for real in the local 8.E3 proof
# (docs/packaging/evidence) and in the release workflow's Linux jobs.
set -eu

: "${PRODUCT_LOWER:?build-deb.sh: PRODUCT_LOWER not set (run packaging-env.py first)}"
: "${PRODUCT_VERSION:?build-deb.sh: PRODUCT_VERSION not set}"
: "${PRODUCT_PUBLISHER:?build-deb.sh: PRODUCT_PUBLISHER not set}"
: "${PRODUCT_DESCRIPTION:?build-deb.sh: PRODUCT_DESCRIPTION not set}"
: "${OS_ID:?build-deb.sh: OS_ID not set}"
: "${ARCH_ID:?build-deb.sh: ARCH_ID not set}"
: "${PACKAGING_LINUX_BINDIR:?build-deb.sh: PACKAGING_LINUX_BINDIR not set (run packaging-env.py first)}"
: "${PACKAGING_DEB_SECTION:?build-deb.sh: PACKAGING_DEB_SECTION not set}"
TARGET_DIR=${TARGET_DIR:-target}
DIST_DIR=${DIST_DIR:-dist}

ROOT=$(cd "$(dirname -- "$0")/../.." && pwd)
cd "$ROOT"

die() { echo "build-deb.sh: error: $*" >&2; exit 1; }

[ "$OS_ID" = "linux" ] || die "build-deb.sh is the Linux lane (OS_ID=${OS_ID})"
command -v dpkg-deb >/dev/null 2>&1 ||
    die "dpkg-deb not found — build-deb.sh needs a Debian-family host"

case "$ARCH_ID" in
    x86_64) deb_arch=amd64 ;;
    aarch64) deb_arch=arm64 ;;
    *) die "cannot map ARCH_ID=${ARCH_ID} to a Debian architecture" ;;
esac

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT INT TERM

# 1. The shared FHS payload (bins + docs + desktop entry + icon).
"$ROOT/scripts/packaging/stage-linux-payload.sh" "$stage/root"

# 2. Control metadata (config-driven fields only).
mkdir -p "$stage/root/DEBIAN"
cat >"$stage/root/DEBIAN/control" <<EOF
Package: ${PRODUCT_LOWER}
Version: ${PRODUCT_VERSION}
Section: ${PACKAGING_DEB_SECTION}
Priority: optional
Architecture: ${deb_arch}
Maintainer: ${PRODUCT_PUBLISHER}
Description: ${PRODUCT_DESCRIPTION}
 Both shipped binaries (the engine-driven CLI and the desktop
 composition-root binary) install under /${PACKAGING_LINUX_BINDIR};
 this package ships files only, no maintainer scripts.
EOF

# 3. Pack (root-owner-group: ownership independent of the build user).
mkdir -p "$DIST_DIR"
dist_abs=$(cd "$DIST_DIR" && pwd)
out="$dist_abs/${PRODUCT_LOWER}-${PRODUCT_VERSION}-${ARCH_ID}-${OS_ID}.deb"
dpkg-deb --root-owner-group --build "$stage/root" "$out"
[ -s "$out" ] || die "deb is missing or empty: $out"
echo "build-deb.sh: wrote $out (arch=${deb_arch}, section=${PACKAGING_DEB_SECTION})"
