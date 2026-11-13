#!/bin/sh
# build-zip.sh — the flat Windows archive (issue #56, card 8.E1, W24-C).
#
# The 8.E1 config-driven zip lane: both shipped exes + the shipped
# docs list from packaging/packaging.toml [shipped] (never a literal
# file list) at the archive root. Made with `python -m zipfile` (the
# stdlib is on every GitHub runner image) rather than a runner-specific
# zip CLI -- git-bash on windows-latest has no `zip`, GNU tar does not
# write zip, 7z is only guaranteed on the Windows image. One tool,
# three images.
#
# Usage: build-zip.sh
# Inputs: release binaries under TARGET_DIR/release, env from
#         scripts/packaging/packaging-env.py.
# Output: DIST_DIR/<slug>-<version>-<arch>-windows.zip
# POSIX sh (git-bash on the Windows runner); `bash -n` clean; the zip
# creation path is EXECUTED for real in the local gate context (same
# host tools) whenever the release binaries are present.
set -eu

: "${PRODUCT_LOWER:?build-zip.sh: PRODUCT_LOWER not set (run packaging-env.py first)}"
: "${PRODUCT_VERSION:?build-zip.sh: PRODUCT_VERSION not set}"
: "${BIN_CLI:?build-zip.sh: BIN_CLI not set}"
: "${BIN_DESKTOP:?build-zip.sh: BIN_DESKTOP not set}"
: "${OS_ID:?build-zip.sh: OS_ID not set}"
: "${ARCH_ID:?build-zip.sh: ARCH_ID not set}"
: "${PACKAGING_DOCS:?build-zip.sh: PACKAGING_DOCS not set (packaging.toml shipped.docs)}"
TARGET_DIR=${TARGET_DIR:-target}
DIST_DIR=${DIST_DIR:-dist}

ROOT=$(cd "$(dirname -- "$0")/../.." && pwd)
cd "$ROOT"

die() { echo "build-zip.sh: error: $*" >&2; exit 1; }

[ "$OS_ID" = "windows" ] || die "build-zip.sh is the Windows lane (OS_ID=${OS_ID})"

PY=$(command -v python3 || command -v python) ||
    die "neither python3 nor python found on PATH (zip creation needs the stdlib zipfile module)"

release_dir="$TARGET_DIR/release"
cli_exe="${BIN_CLI}.exe"
desktop_exe="${BIN_DESKTOP}.exe"
[ -f "$release_dir/$cli_exe" ] ||
    die "missing packaging input: $release_dir/$cli_exe (build first: cargo build --release --locked)"
[ -f "$release_dir/$desktop_exe" ] ||
    die "missing packaging input: $release_dir/$desktop_exe"

# The shipped list from packaging.toml (existence already asserted by
# packaging-env.py's semantic pass; re-asserted for standalone safety).
for doc in $PACKAGING_DOCS; do
    [ -f "$doc" ] || die "shipped doc not found: $doc (packaging.toml shipped.docs)"
done

mkdir -p "$DIST_DIR"
dist_abs=$(cd "$DIST_DIR" && pwd)
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT INT TERM

install -m 0755 "$release_dir/$cli_exe" "$stage/$cli_exe"
install -m 0755 "$release_dir/$desktop_exe" "$stage/$desktop_exe"
for doc in $PACKAGING_DOCS; do
    install -m 0644 "$doc" "$stage/"
done

base="${PRODUCT_LOWER}-${PRODUCT_VERSION}-${ARCH_ID}-${OS_ID}"
zip_path="$dist_abs/${base}.zip"

# python -m zipfile stores entries under their given (relative) names —
# run from the staging dir so the archive root is flat. The doc list is
# passed as shell-quoted argv (no re-parsing).
set -- "$cli_exe" "$desktop_exe"
for doc in $PACKAGING_DOCS; do
    set -- "$@" "$doc"
done
( cd "$stage" && "$PY" -m zipfile -c "$zip_path" "$@" ) ||
    die "zip creation failed"
[ -s "$zip_path" ] || die "zip archive is missing or empty: $zip_path"
echo "build-zip.sh: wrote $zip_path"
