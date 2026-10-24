#!/usr/bin/env bash
# fuzz_registry_check.sh — the doc 04 §8.5 registry gate (card 5.Q1).
#
# Walks four sets and requires them IDENTICAL:
#   1. [[bin]] names in fuzz/Cargo.toml
#   2. target files in fuzz/fuzz_targets/*.rs
#   3. registry rows in fuzz/README.md (the | `name` | table)
#   4. the in-crate harness fn each target's shim calls (rg-verified)
# …plus the coverage gate: every target has a non-empty seed corpus
# (doc 04 §8.5: "new parser without corpus = CI failure").
#
# Exit 0 = registry-vs-implementation diff EMPTY. Any drift fails.
set -uo pipefail
cd "$(dirname "$0")/.."

fail=0
say() { printf '%s\n' "$*"; }

# 1. bins declared in the manifest.
bins=$(sed -n 's/^name = "\(fuzz_[a-z0-9_]*\)"$/\1/p' fuzz/Cargo.toml | sort -u)

# 2. target files on disk.
files=$(cd fuzz/fuzz_targets && ls fuzz_*.rs 2>/dev/null | sed 's/\.rs$//' | sort -u)

# 3. registry rows in the README table.
readme=$(sed -n 's/^| `\(fuzz_[a-z0-9_]*\)` *|.*/\1/p' fuzz/README.md | sort -u)

# 4. harness fns: each shim calls exactly one <crate>::<path>::<fn>(data);
#    verify the fn exists in that crate's source.
harness_missing=0
while IFS= read -r target; do
    call=$(sed -n 's/^    \(.*::fuzz_[a-z0-9_]*\)(data);$/\1/p' \
        "fuzz/fuzz_targets/$target.rs" | head -1)
    if [ -z "$call" ]; then
        say "REGISTRY-GATE: $target: shim does not call an in-crate harness"
        harness_missing=1
        continue
    fi
    crate="${call%%::*}"
    fn="${call##*::}"
    if ! rg -q "fn $fn" "crates/$crate/src" 2>/dev/null; then
        say "REGISTRY-GATE: $target: harness fn $fn not found in crates/$crate/src"
        harness_missing=1
    fi
done <<<"$bins"

# Set diffs (bins vs files vs readme).
for pair in "bins:files" "bins:readme"; do
    a="${pair%%:*}"; b="${pair##*:}"
    diff_out=$(diff <(printf '%s\n' "${!a}") <(printf '%s\n' "${!b}") || true)
    if [ -n "$diff_out" ]; then
        say "REGISTRY-GATE: registry-vs-implementation drift ($a vs $b):"
        say "$diff_out"
        fail=1
    fi
done
[ "$harness_missing" -eq 1 ] && fail=1

# The coverage gate: every target has a non-empty seed corpus.
corpus_root="${LUMEN_FUZZ_CORPUS:-fuzz/corpus}"
while IFS= read -r target; do
    dir="$corpus_root/$target"
    if [ ! -d "$dir" ] || [ -z "$(ls -A "$dir" 2>/dev/null)" ]; then
        say "REGISTRY-GATE: $target: no seed corpus in $dir (doc 04 §8.5: parser without corpus = CI failure)"
        fail=1
    fi
done <<<"$bins"

count=$(printf '%s\n' "$bins" | wc -l)
if [ "$fail" -eq 0 ]; then
    say "fuzz-registry: CLEAN — $count targets, diff empty, corpora present"
else
    say "fuzz-registry: FAILED"
fi
exit $fail
