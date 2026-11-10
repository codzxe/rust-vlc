#!/bin/sh
# gen-wxs.sh — generate the WiX v5 installer source (.wxs) from the
# product config environment (card 7.w22g, issue #56 release half;
# docs-list generation added in W24-C review fix, W24-R1 P2-1;
# doc-basename XML escaping + hostile-fixture self-test added in W24-Q
# for W24-R6 P4-2).
#
# Every MSI identity field is injected from product.toml via the
# environment set up by scripts/release/product-env.py:
#   PRODUCT_NAME             -> Package/@Name (and install dir name)
#   PRODUCT_PUBLISHER       -> Package/@Manufacturer
#   PRODUCT_DESCRIPTION     -> package description
#   MSI_VERSION              -> Package/@Version (numeric-only x.y.z;
#                               display_version's pre-release suffix is
#                               stripped by product-env.py — MSI
#                               ProductVersion rejects "-dev")
#   PRODUCT_MSI_UPGRADE_CODE -> Package/@UpgradeCode (identity.ids.windows
#                               .msi_upgrade_code: frozen, stable across
#                               versions so upgrades replace, not
#                               co-install)
#   BIN_CLI / BIN_DESKTOP    -> the two shipped exe file names
# and the DOCS LIST is injected from packaging.toml [shipped] docs by
# scripts/packaging/packaging-env.py (the W24-C bridge):
#   PACKAGING_DOCS           -> one <File> row per doc in the Docs
#                               component (space-separated repo-relative
#                               paths — whitespace-free by schema, see
#                               packaging.schema.json shipped.docs; the
#                               FIRST doc is the component KeyPath).
#                               REQUIRED — the caller must run
#                               packaging-env.py first (build-msi.sh /
#                               build-msi.ps1 do; the superseded wave-22
#                               scripts/release/package-windows.sh flow
#                               must too, or die loudly here).
#
# The output is WiX v4/v5 schema (wixtoolset.org/schemas/v4/wxs); wix v5
# CLI consumes it with `wix build -arch x64`. File Source paths are
# basenames: run wix from the staging directory that contains the exes
# AND the staged docs (the caller stages both — build-msi.sh stages the
# same PACKAGING_DOCS list into the staging dir).
#
# XML ESCAPING (W24-R6 P4-2): every value interpolated into an XML
# attribute goes through xml_escape — the identity fields always did,
# and the doc BASENAMES in File/@Source now do too (a doc named
# `a&b.md` used to yield malformed XML — a loud wix-build failure, but
# a broken one; File/@Id stays identifier-sanitized instead).
#
# Honest scope: the two binaries + the [shipped] docs list under
# Program Files, no Start-Menu shortcut — the desktop shell is the
# ADR-004 std-only skeleton, and shipping a menu entry for a binary
# that opens no window would be a false affordance. Revisit when the
# webview seam lands.
#
# Usage: gen-wxs.sh <output.wxs>
#        gen-wxs.sh --self-test   (regression test: hostile filenames
#                                  + identity values -> the generated
#                                  XML must PARSE and round-trip;
#                                  needs python3/python for the XML
#                                  proof — the same interpreter CI
#                                  provides for packaging-env.py)
# POSIX sh; validated with `bash -n` + sh syntax parse (see worklog).

set -eu

usage() {
    echo "usage: $0 <output.wxs>" >&2
    echo "       $0 --self-test   (hostile-fixture XML-escaping regression test)" >&2
    exit 2
}

[ $# -eq 1 ] || usage
self_test=0
out=""
if [ "$1" = "--self-test" ]; then
    self_test=1
else
    out=$1
fi

xml_escape() {
    printf '%s' "$1" | sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' \
        -e 's/>/\&gt;/g' -e 's/"/\&quot;/g' -e "s/'/\&apos;/g"
}

# The generation body, one function so --self-test exercises the REAL
# code path (never a copy that could drift from it).
generate_wxs() {
    : "${PRODUCT_NAME:?gen-wxs.sh: PRODUCT_NAME not set (run product-env.py first)}"
    : "${PRODUCT_PUBLISHER:?gen-wxs.sh: PRODUCT_PUBLISHER not set}"
    : "${PRODUCT_DESCRIPTION:?gen-wxs.sh: PRODUCT_DESCRIPTION not set}"
    : "${MSI_VERSION:?gen-wxs.sh: MSI_VERSION not set}"
    : "${PRODUCT_MSI_UPGRADE_CODE:?gen-wxs.sh: PRODUCT_MSI_UPGRADE_CODE not set}"
    : "${BIN_CLI:?gen-wxs.sh: BIN_CLI not set}"
    : "${BIN_DESKTOP:?gen-wxs.sh: BIN_DESKTOP not set}"
    : "${PACKAGING_DOCS:?gen-wxs.sh: PACKAGING_DOCS not set (run packaging-env.py first — the docs list is packaging.toml [shipped] docs; a hardcoded LICENSE/README pair here would silently ignore docs-list changes)}"

    esc_name=$(xml_escape "$PRODUCT_NAME")
    esc_manufacturer=$(xml_escape "$PRODUCT_PUBLISHER")
    esc_description=$(xml_escape "$PRODUCT_DESCRIPTION")

    # --- Docs rows from the config list (W24-R1 P2-1) --------------------
    # One <File> per packaging.toml [shipped] doc; the FIRST is the
    # component KeyPath (exactly one File per component may be KeyPath;
    # deterministic). File/@Id must be a legal identifier: non-word
    # characters collapse to '_' and the positional prefix keeps ids
    # collision-free. File/@Source carries the BASENAME, XML-escaped
    # like every other attribute value (W24-R6 P4-2) — a doc named
    # `a&b<c>"d".md` must produce well-formed XML. Source basenames
    # resolve from the staging dir, so two configured docs sharing a
    # basename would silently clobber in staging — refused loudly
    # instead (mirrors build-msi.ps1's check; change both together).
    docs_xml=""
    n=0
    seen=""
    for doc in $PACKAGING_DOCS; do
        n=$((n + 1))
        base=$(basename -- "$doc")
        case " $seen " in
            *" $base "*)
                echo "gen-wxs.sh: error: packaging.toml shipped.docs lists two docs with the same basename '$base' — the MSI staging dir is flat" >&2
                exit 1
                ;;
        esac
        seen="$seen $base"
        id="Doc_${n}_$(printf '%s' "$base" | sed -e 's/[^0-9A-Za-z_]/_/g')"
        esc_source=$(xml_escape "$base")
        if [ "$n" -eq 1 ]; then
            docs_xml="${docs_xml}          <File Id=\"${id}\" Source=\"${esc_source}\" KeyPath=\"yes\" />
"
        else
            docs_xml="${docs_xml}          <File Id=\"${id}\" Source=\"${esc_source}\" />
"
        fi
    done
    if [ "$n" -eq 0 ]; then
        echo "gen-wxs.sh: error: packaging.toml [shipped] docs is empty — the Docs component needs at least one file" >&2
        exit 1
    fi

    mkdir -p "$(dirname -- "$out")"

    cat >"$out" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!-- Generated by scripts/release/gen-wxs.sh — DO NOT HAND-EDIT.
     All identity fields come from product.toml (via product-env.py);
     regenerate instead of editing (MP §5 single source of truth). -->
<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs">
  <Package Name="${esc_name}"
           Manufacturer="${esc_manufacturer}"
           Version="${MSI_VERSION}"
           UpgradeCode="${PRODUCT_MSI_UPGRADE_CODE}"
           Scope="perMachine"
           Language="1033">
    <MajorUpgrade DowngradeErrorMessage="A newer version of [ProductName] is already installed." />
    <MediaTemplate EmbedCab="yes" />
    <StandardDirectory Id="ProgramFiles64Folder">
      <Directory Id="INSTALLDIR" Name="${esc_name}">
        <Component Id="BinCli" Guid="*">
          <File Id="BinCliExe" Source="${BIN_CLI}.exe" KeyPath="yes" />
        </Component>
        <Component Id="BinDesktop" Guid="*">
          <File Id="BinDesktopExe" Source="${BIN_DESKTOP}.exe" KeyPath="yes" />
        </Component>
        <Component Id="Docs" Guid="*">
${docs_xml}        </Component>
      </Directory>
    </StandardDirectory>
    <Feature Id="Main" Level="1" Title="${esc_name}">
      <ComponentRef RefId="BinCli" />
      <ComponentRef RefId="BinDesktop" />
      <ComponentRef RefId="Docs" />
    </Feature>
  </Package>
</Wix>
EOF

    echo "gen-wxs.sh: wrote $out (name=${esc_name} version=${MSI_VERSION} upgrade-code=${PRODUCT_MSI_UPGRADE_CODE})"
}

# --- --self-test: hostile fixtures must yield WELL-FORMED XML ----------------
# Regression test for W24-R6 P4-2 (a doc named `a&b<c>"d".md` used to
# produce output that no XML parser accepts — gen-wxs exited 0 and the
# failure surfaced only inside `wix build` on the CI runner). Runs the
# REAL generate_wxs with hostile identity values + hostile doc
# basenames, then proves with python's XML parser that the result
# parses AND that the hostile values round-trip (the parser sees the
# original basename back — escaping is lossless for content, while
# File/@Id stays sanitized).
self_test() {
    PY="$(command -v python3 || command -v python)" || {
        echo "gen-wxs.sh: --self-test needs python3/python (XML well-formedness proof — the same interpreter CI provides for packaging-env.py)" >&2
        exit 1
    }
    work=$(mktemp -d)
    trap 'rm -rf "$work"' EXIT INT TERM
    out="$work/hostile.wxs"

    # Hostile fixtures (test data only — never real identity values):
    # every XML metacharacter in the identity fields and BOTH doc
    # basenames (' included, per the escaping contract).
    PRODUCT_NAME='A&B "C" <D>'
    PRODUCT_PUBLISHER='P&Q'
    PRODUCT_DESCRIPTION="it's <a> & \"b\" > c"
    MSI_VERSION="1.2.3"
    PRODUCT_MSI_UPGRADE_CODE="11111111-2222-3333-4444-555555555555"
    BIN_CLI=selftest-cli
    BIN_DESKTOP=selftest-desktop
    PACKAGING_DOCS="a&b<c>\"d\".md it's.md"

    generate_wxs

    "$PY" - "$out" <<'PYEOF'
import sys
import xml.etree.ElementTree as ET

path = sys.argv[1]
root = ET.parse(path).getroot()  # malformed XML -> parser death = test failure
ns = "{http://wixtoolset.org/schemas/v4/wxs}"
files = root.findall(f".//{ns}File")
assert files, "no File elements generated"

sources = [f.get("Source") for f in files]
for hostile in ("a&b<c>\"d\".md", "it's.md"):
    assert hostile in sources, (
        f"doc basename did not round-trip through the XML escaping: "
        f"{hostile!r} not in {sources!r}")

ids = [f.get("Id") for f in files]
assert len(ids) == len(set(ids)), f"File ids collide: {ids!r}"
for i in ids:
    assert i and all(c.isalnum() or c == "_" for c in i), (
        f"File id not sanitized to a legal identifier: {i!r}")

assert root.find(f"{ns}Package").get("Name") == 'A&B "C" <D>', (
    "hostile identity Name did not round-trip")

print(f"gen-wxs.sh: --self-test PASS "
      f"({len(files)} File rows parsed; hostile basenames + identity "
      f"round-trip, ids sanitized)")
PYEOF
}

if [ "$self_test" -eq 1 ]; then
    self_test
else
    generate_wxs
fi
