#!/bin/sh
# stage-linux-payload.sh — stage the ONE FHS payload tree the deb and
# the rpm ship (issue #56, card 8.E3, task W24-C).
#
# Layout (every path from packaging/packaging.toml [linux] via
# PACKAGING_LINUX_* — no directory literal in this script):
#   <dest>/$PACKAGING_LINUX_BINDIR/$BIN_CLI            0755
#   <dest>/$PACKAGING_LINUX_BINDIR/$BIN_DESKTOP        0755
#   <dest>/$PACKAGING_LINUX_DOCS_DIR/$PRODUCT_LOWER/<each shipped doc>  0644
#   <dest>/$PACKAGING_LINUX_APPLICATIONS_DIR/$PRODUCT_APPSTREAM_ID.desktop 0644
#   <dest>/$PACKAGING_LINUX_ICONS_DIR/$PRODUCT_LOWER.png               0644
#
# The desktop entry comes from gen-desktop-entry.sh (the ONE writer,
# shared with the AppImage lane). The icon prefers the real brand asset
# (product.toml [brand] icon_png_512, exported as PRODUCT_ICON_PNG_512
# by packaging-env.py) and falls back to the deterministic placeholder
# (scripts/release/gen-icon.py, accent-colored) until assets/brand
# exists — an honest, clean-room stand-in, never borrowed artwork.
#
# Usage: stage-linux-payload.sh <dest-dir>
# POSIX sh; `bash -n` clean; EXECUTED for real in the local 8.E3 proof
# (docs/packaging/evidence) and in the release workflow's Linux jobs.
set -eu

: "${PRODUCT_LOWER:?stage-linux-payload.sh: PRODUCT_LOWER not set (run packaging-env.py first)}"
: "${PRODUCT_APPSTREAM_ID:?stage-linux-payload.sh: PRODUCT_APPSTREAM_ID not set}"
: "${PRODUCT_ACCENT_COLOR:?stage-linux-payload.sh: PRODUCT_ACCENT_COLOR not set}"
: "${PRODUCT_ICON_PNG_512:?stage-linux-payload.sh: PRODUCT_ICON_PNG_512 not set}"
: "${BIN_CLI:?stage-linux-payload.sh: BIN_CLI not set}"
: "${BIN_DESKTOP:?stage-linux-payload.sh: BIN_DESKTOP not set}"
: "${OS_ID:?stage-linux-payload.sh: OS_ID not set}"
: "${ARCH_ID:?stage-linux-payload.sh: ARCH_ID not set}"
: "${PACKAGING_LINUX_BINDIR:?stage-linux-payload.sh: PACKAGING_LINUX_BINDIR not set (run packaging-env.py first)}"
: "${PACKAGING_LINUX_DOCS_DIR:?stage-linux-payload.sh: PACKAGING_LINUX_DOCS_DIR not set}"
: "${PACKAGING_LINUX_APPLICATIONS_DIR:?stage-linux-payload.sh: PACKAGING_LINUX_APPLICATIONS_DIR not set}"
: "${PACKAGING_LINUX_ICONS_DIR:?stage-linux-payload.sh: PACKAGING_LINUX_ICONS_DIR not set}"
: "${PACKAGING_DOCS:?stage-linux-payload.sh: PACKAGING_DOCS not set (packaging.toml shipped.docs)}"
TARGET_DIR=${TARGET_DIR:-target}

ROOT=$(cd "$(dirname -- "$0")/../.." && pwd)
cd "$ROOT"

die() { echo "stage-linux-payload.sh: error: $*" >&2; exit 1; }

[ "$OS_ID" = "linux" ] || die "this is the Linux lane (OS_ID=${OS_ID})"
[ $# -eq 1 ] || die "usage: $0 <dest-dir>"
dest=$1

cli_bin="$TARGET_DIR/release/$BIN_CLI"
desktop_bin="$TARGET_DIR/release/$BIN_DESKTOP"
[ -f "$cli_bin" ] || die "missing packaging input: $cli_bin (build first: cargo build --release --locked)"
[ -f "$desktop_bin" ] || die "missing packaging input: $desktop_bin"

mkdir -p \
    "$dest/$PACKAGING_LINUX_BINDIR" \
    "$dest/$PACKAGING_LINUX_DOCS_DIR/$PRODUCT_LOWER" \
    "$dest/$PACKAGING_LINUX_APPLICATIONS_DIR" \
    "$dest/$PACKAGING_LINUX_ICONS_DIR"

install -m 0755 "$cli_bin" "$dest/$PACKAGING_LINUX_BINDIR/$BIN_CLI"
install -m 0755 "$desktop_bin" "$dest/$PACKAGING_LINUX_BINDIR/$BIN_DESKTOP"

# Shipped docs: the list lives in packaging.toml [shipped] docs (space
# separated in PACKAGING_DOCS); existence was already asserted by
# packaging-env.py's semantic pass — asserted again here so the script
# is safe standalone.
for doc in $PACKAGING_DOCS; do
    [ -f "$doc" ] || die "shipped doc not found: $doc (packaging.toml shipped.docs)"
    install -m 0644 "$doc" "$dest/$PACKAGING_LINUX_DOCS_DIR/$PRODUCT_LOWER/"
done

"$ROOT/scripts/packaging/gen-desktop-entry.sh" \
    "$dest/$PACKAGING_LINUX_APPLICATIONS_DIR/${PRODUCT_APPSTREAM_ID}.desktop"

# Icon: real brand asset when it exists, documented placeholder if not.
# gen-icon.py is invoked THROUGH the interpreter, not executed
# directly: the wave-22 commit carries it as 100644 (no exec bit), so a
# direct call dies with EACCES on a fresh checkout (reproduced here;
# flagged to the Lead for a mode-only fix in scripts/release/).
icon_dest="$dest/$PACKAGING_LINUX_ICONS_DIR/$PRODUCT_LOWER.png"
if [ -f "$PRODUCT_ICON_PNG_512" ]; then
    install -m 0644 "$PRODUCT_ICON_PNG_512" "$icon_dest"
else
    PY=$(command -v python3 || command -v python) ||
        die "neither python3 nor python found on PATH (icon generation needs the stdlib)"
    "$PY" "$ROOT/scripts/release/gen-icon.py" "$icon_dest" "$PRODUCT_ACCENT_COLOR"
fi
chmod 0644 "$icon_dest"
