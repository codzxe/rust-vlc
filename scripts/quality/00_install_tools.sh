#!/usr/bin/env bash
# T00 - install the pinned audit tool set and record versions.
# Every version here is pinned; nothing floats into CI (AGENTS.md 3.2 / doc 11 3.7).
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

# Pinned versions (resolved from crates.io on 2026-11-16, see 01-TOOL-INVENTORY.md)
PKGS=(
  "cargo-machete@0.10.1"
  "cargo-shear@1.14.1"
  "cargo-hack@0.6.46"
  "cargo-nextest@0.9.152"
  "cargo-llvm-cov@0.9.1"
  "cargo-mutants@27.1.0"
  "cargo-modules@0.28.0"
  "cargo-bloat@0.12.1"
  "cargo-llvm-lines@0.4.48"
  "cargo-crap@0.9.0"
  "cargo-deny@0.20.2"      # matches the pin already used by ci.yml
  "typos-cli@1.52.2"
  "ast-grep@0.51.3"
)
# EVAL-only, installed on demand (listed for completeness, not installed here):
#   cargo-udeps, cargo-public-api, cargo-semver-checks, cargo-check-external-types,
#   cargo-geiger, cargo-vet, cargo-about, cargo-hakari, cargo-outdated,
#   cargo-careful, kani-verifier, dylint, cargo-spellcheck, cargo-call-stack,
#   cargo-flamegraph, cargo-expand

head1 "0. rustup components"
rustup component add llvm-tools-preview 2>&1 | tail -1
rustup component add rust-src 2>&1 | tail -1

head1 "1. installing pinned tools"
for p in "${PKGS[@]}"; do
  bin="${p%%@*}"
  if command -v "$bin" >/dev/null 2>&1; then
    say "present: $p"
    continue
  fi
  say "installing: $p"
  cargo install --locked "$p" 2>&1 | tail -3
done

head1 "2. recording versions"
: > "$OUT/tool-versions.txt"
{
  echo "generated: $(stamp)"
  echo "rustc: $(rustc --version)"
  echo "cargo: $(cargo --version)"
} >> "$OUT/tool-versions.txt"
for p in "${PKGS[@]}"; do
  bin="${p%%@*}"
  if command -v "$bin" >/dev/null 2>&1; then
    printf '%-22s %s\n' "$bin" "$("$bin" --version 2>&1 | head -1)" >> "$OUT/tool-versions.txt"
  else
    printf '%-22s NOT INSTALLED\n' "$bin" >> "$OUT/tool-versions.txt"
  fi
done
cat "$OUT/tool-versions.txt"
say ""
say "written: $OUT/tool-versions.txt"
