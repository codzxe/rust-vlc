#!/bin/sh
# build-targz.sh — the flat tar.gz archive lane (issue #56, cards
# 8.E1–8.E3 share it; task W24-C).
#
# The plain-archive fallback every OS lane ships: the two shipped
# binaries (the CLI and the tauri GUI shell — w26.m.t2 added the
# shell, w30.8 removed the exit-86 lumen-desktop stub: a flat archive
# user who double-clicked the obviously-desktop-named exe got
# "loading, then nothing"; the stub is a CI-only diagnostic now) +
# the docs list from packaging/packaging.toml [shipped] at the
# archive root, packed with the host's native tar. This is the
# archive users on any OS can unpack with nothing but tar.
#
# Usage: build-targz.sh
# Inputs: release binaries under TARGET_DIR/release, env from
#         scripts/packaging/packaging-env.py.
# Output: DIST_DIR/<slug>-<version>-<arch>-<os>.tar.gz
# POSIX sh; `bash -n` clean; EXECUTED for real in the local 8.E3 proof
# context and on every release-lane OS (tar is universal).
set -eu

: "${PRODUCT_LOWER:?build-targz.sh: PRODUCT_LOWER not set (run packaging-env.py first)}"
: "${PRODUCT_VERSION:?build-targz.sh: PRODUCT_VERSION not set}"
: "${BIN_CLI:?build-targz.sh: BIN_CLI not set}"
: "${BIN_TAURI:?build-targz.sh: BIN_TAURI not set}"
: "${OS_ID:?build-targz.sh: OS_ID not set}"
: "${ARCH_ID:?build-targz.sh: ARCH_ID not set}"
: "${PACKAGING_DOCS:?build-targz.sh: PACKAGING_DOCS not set (packaging.toml shipped.docs)}"
TARGET_DIR=${TARGET_DIR:-target}
DIST_DIR=${DIST_DIR:-dist}

ROOT=$(cd "$(dirname -- "$0")/../.." && pwd)
cd "$ROOT"

die() { echo "build-targz.sh: error: $*" >&2; exit 1; }

release_dir="$TARGET_DIR/release"
cli_bin="$release_dir/$BIN_CLI"
tauri_bin="$release_dir/$BIN_TAURI"
[ -f "$cli_bin" ] ||
    die "missing packaging input: $cli_bin (build first: cargo build --release --locked)"
# The GUI shell: same required-input posture as the CLI — it builds
# separately in CI (--manifest-path apps/desktop-tauri/Cargo.toml)
# but lands in the SAME target dir, so from here on it is just a
# second filename.
[ -f "$tauri_bin" ] ||
    die "missing packaging input: $tauri_bin"

# The shipped list from packaging.toml (existence already asserted by
# packaging-env.py's semantic pass; re-asserted for standalone safety).
for doc in $PACKAGING_DOCS; do
    [ -f "$doc" ] || die "shipped doc not found: $doc (packaging.toml shipped.docs)"
done

mkdir -p "$DIST_DIR"
dist_abs=$(cd "$DIST_DIR" && pwd)
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT INT TERM

install -m 0755 "$cli_bin" "$stage/"
install -m 0755 "$tauri_bin" "$stage/"
for doc in $PACKAGING_DOCS; do
    install -m 0644 "$doc" "$stage/"
done

base="${PRODUCT_LOWER}-${PRODUCT_VERSION}-${ARCH_ID}-${OS_ID}"
out="$dist_abs/${base}.tar.gz"

set -- "$BIN_CLI" "$BIN_TAURI"
for doc in $PACKAGING_DOCS; do
    set -- "$@" "$doc"
done
tar -czf "$out" -C "$stage" "$@"
[ -s "$out" ] || die "tar.gz is missing or empty: $out"
echo "build-targz.sh: wrote $out"
