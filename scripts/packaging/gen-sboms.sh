#!/bin/sh
# gen-sboms.sh — the ONE per-artifact CycloneDX SBOM writer (issue #56,
# card 8.E3 / doc 11 §9.3 item 1; task W24-C r3).
#
# Wraps `cargo run -p xtask --locked -- sbom` (the wave-23 std-only
# CycloneDX generator) once per release artifact, so every download
# set carries its own SBOM and every release asset is self-contained.
# Content is lockfile-derived, hence byte-identical across platforms;
# the per-artifact copies exist for self-containment, not divergence.
#
# Two cross-checks make this more than a loop:
#   * the output file suffix comes from packaging/packaging.toml
#     [sbom] artifact_suffix (via packaging-env.py) — no literal here;
#   * the specVersion the generator ACTUALLY emitted is asserted
#     against packaging.toml [sbom] spec_version — drift between the
#     config plane and the xtask generator is a hard error, never a
#     silent mismatch.
#
# Entry points: the release workflow calls this directly on the
# Windows/macOS jobs; the Linux lane gets the same writer through
# verify-linux-artifacts.sh step 8 (one writer, two callers).
#
# Usage: gen-sboms.sh <artifact> [artifact...]
# Inputs: env from scripts/packaging/packaging-env.py; a workspace
#         with tools/xtask buildable (cargo on PATH).
# Output: <artifact>.<PACKAGING_SBOM_ARTIFACT_SUFFIX> beside each.
# POSIX sh; `bash -n` clean; EXECUTED for real against the committed
# linux-lane artifacts (docs/packaging/evidence/, r3 log).
set -eu

: "${PACKAGING_SBOM_ARTIFACT_SUFFIX:?gen-sboms.sh: PACKAGING_SBOM_ARTIFACT_SUFFIX not set (run packaging-env.py first)}"
: "${PACKAGING_SBOM_SPEC_VERSION:?gen-sboms.sh: PACKAGING_SBOM_SPEC_VERSION not set (run packaging-env.py first)}"

ROOT=$(cd "$(dirname -- "$0")/../.." && pwd)
cd "$ROOT"

die() { echo "gen-sboms.sh: error: $*" >&2; exit 1; }

[ $# -ge 1 ] || die "usage: gen-sboms.sh <artifact> [artifact...]"

for artifact in "$@"; do
    [ -f "$artifact" ] || die "artifact not found: $artifact"
    out="${artifact}.${PACKAGING_SBOM_ARTIFACT_SUFFIX}"
    cargo run -p xtask --locked -- sbom --out "$out" >/dev/null ||
        die "xtask sbom failed for $out"
    [ -s "$out" ] || die "SBOM missing or empty: $out"
    # Config-vs-code drift check: what xtask emitted must be what
    # packaging.toml declares (pretty-printed "specVersion": "x.y" or
    # the compact one-line form — both accepted, nothing else).
    grep -q "\"specVersion\": *\"${PACKAGING_SBOM_SPEC_VERSION}\"" "$out" ||
        die "SBOM specVersion does not match packaging.toml sbom.spec_version=${PACKAGING_SBOM_SPEC_VERSION}: $out"
    echo "gen-sboms.sh: wrote $out ($(wc -c <"$out") bytes, CycloneDX ${PACKAGING_SBOM_SPEC_VERSION})"
done
