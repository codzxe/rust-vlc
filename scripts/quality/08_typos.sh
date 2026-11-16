#!/usr/bin/env bash
# T08 - typos in identifiers, user-facing strings and docs.
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

if ! need typos; then exit 0; fi

head1 "typos"
typos --format brief . 2>&1 | tee "$OUT/typos.txt" | head -60

head1 "categories"
total=$(grep -c '^' "$OUT/typos.txt" 2>/dev/null || echo 0)
code=$(grep -E 'crates/.*\.rs:' "$OUT/typos.txt" 2>/dev/null | wc -l)
docs=$(grep -E '\.md:' "$OUT/typos.txt" 2>/dev/null | wc -l)
say "total=$total  in-rust-sources=$code  in-docs=$docs"

head1 "top misspellings"
sed -E 's/^[^ ]+:[0-9]+:[0-9]+ - //' "$OUT/typos.txt" 2>/dev/null \
  | sed -E 's/error: .*//' | awk '{print $1}' | sort | uniq -c | sort -rn | head -20

head1 "spellcheck (rustdoc only, EVAL)"
if need cargo-spellcheck; then
  cargo spellcheck --code 1 2>&1 | tail -20
else
  not_run "cargo-spellcheck" "not installed (EVAL tier)"
fi
