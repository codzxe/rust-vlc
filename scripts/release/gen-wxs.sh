#!/bin/sh
# gen-wxs.sh — generate the WiX v5 installer source (.wxs) from the
# product config environment (card 7.w22g, issue #56 release half;
# docs-list generation added in W24-C review fix, W24-R1 P2-1).
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
#                               paths; the FIRST doc is the component
#                               KeyPath). REQUIRED — the caller must run
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
# Honest scope: the two binaries + the [shipped] docs list under
# Program Files, no Start-Menu shortcut — the desktop shell is the
# ADR-004 std-only skeleton, and shipping a menu entry for a binary
# that opens no window would be a false affordance. Revisit when the
# webview seam lands.
#
# Usage: gen-wxs.sh <output.wxs>
# POSIX sh; validated with `bash -n` + sh syntax parse (see worklog).

set -eu

: "${PRODUCT_NAME:?gen-wxs.sh: PRODUCT_NAME not set (run product-env.py first)}"
: "${PRODUCT_PUBLISHER:?gen-wxs.sh: PRODUCT_PUBLISHER not set}"
: "${PRODUCT_DESCRIPTION:?gen-wxs.sh: PRODUCT_DESCRIPTION not set}"
: "${MSI_VERSION:?gen-wxs.sh: MSI_VERSION not set}"
: "${PRODUCT_MSI_UPGRADE_CODE:?gen-wxs.sh: PRODUCT_MSI_UPGRADE_CODE not set}"
: "${BIN_CLI:?gen-wxs.sh: BIN_CLI not set}"
: "${BIN_DESKTOP:?gen-wxs.sh: BIN_DESKTOP not set}"
: "${PACKAGING_DOCS:?gen-wxs.sh: PACKAGING_DOCS not set (run packaging-env.py first — the docs list is packaging.toml [shipped] docs; a hardcoded LICENSE/README pair here would silently ignore docs-list changes)}"

[ $# -eq 1 ] || { echo "usage: $0 <output.wxs>" >&2; exit 2; }
out=$1

xml_escape() {
    printf '%s' "$1" | sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' \
        -e 's/>/\&gt;/g' -e 's/"/\&quot;/g'
}

esc_name=$(xml_escape "$PRODUCT_NAME")
esc_manufacturer=$(xml_escape "$PRODUCT_PUBLISHER")
esc_description=$(xml_escape "$PRODUCT_DESCRIPTION")

# --- Docs rows from the config list (W24-R1 P2-1) ----------------------------
# One <File> per packaging.toml [shipped] doc; the FIRST is the component
# KeyPath (exactly one File per component may be KeyPath; deterministic).
# File/@Id must be a legal identifier: non-word characters collapse to
# '_' and the positional prefix keeps ids collision-free. Source
# basenames resolve from the staging dir, so two configured docs sharing
# a basename would silently clobber in staging — refused loudly instead
# (mirrors build-msi.ps1's check; change both together).
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
    if [ "$n" -eq 1 ]; then
        docs_xml="${docs_xml}          <File Id=\"${id}\" Source=\"${base}\" KeyPath=\"yes\" />
"
    else
        docs_xml="${docs_xml}          <File Id=\"${id}\" Source=\"${base}\" />
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
