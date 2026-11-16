#!/usr/bin/env bash
# T07 - dependency graph, supply chain, unsafe census, SBOM drift.
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

head1 "7a. duplicate crate versions"
cargo tree --workspace --duplicates 2>&1 | tee "$OUT/duplicates.txt" | head -60
say "duplicate-version entries: $(grep -c '^[a-zA-Z]' "$OUT/duplicates.txt" 2>/dev/null || echo 0)"

head1 "7b. cargo-deny (licenses/sources/bans/advisories)"
if need cargo-deny; then
  cargo deny --workspace check licenses sources bans advisories 2>&1 | tee "$OUT/deny.txt" | tail -40
  say "deny exit=$?"
else
  say "cargo-deny missing - the repo already pins 0.20.2 in ci.yml; install with:"
  say "  cargo install --locked cargo-deny@0.20.2"
fi

head1 "7c. SBOM drift (xtask, same generator as ci.yml)"
cargo run -q -p xtask --locked -- sbom --check 2>&1 | tail -5
say "sbom exit=$?"

head1 "7d. unsafe census"
if need cargo-geiger; then
  cargo geiger --output-format Ascii 2>&1 | tee "$OUT/geiger.txt" | tail -40
else
  not_run "cargo-geiger" "not installed (EVAL in 01-TOOL-INVENTORY.md)"
  say "fallback: count unsafe blocks in first-party crates"
  grep -rn --include=*.rs -E '\bunsafe\b' crates apps tools 2>/dev/null \
    | grep -v '/tests/' | awk -F: '{print $1}' | sort | uniq -c | sort -rn \
    | tee "$OUT/unsafe-first-party.txt" | head -20
fi

head1 "7e. dependency graph (DOT)"
if need cargo-modules; then
  cargo modules structure --workspace 2>&1 | tail -5
  cargo modules dependencies --workspace > "$OUT/depgraph.dot" 2>/dev/null \
    && say "dot: $OUT/depgraph.dot ($(wc -l < "$OUT/depgraph.dot") lines)"
fi
