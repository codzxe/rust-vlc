#!/usr/bin/env bash
# T10 - public API surface, semver tripwire, external type boundaries.
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

mkdir -p "$OUT/api"

head1 "10a. public API dump (cargo-public-api)"
if need cargo-public-api; then
  : > "$OUT/api/index.txt"
  crates_list | while IFS=$'\t' read -r name path; do
    [ -d "$path/src" ] || continue
    cargo public-api --simplified -p "$name" > "$OUT/api/$name.txt" 2>/dev/null
    n=$(grep -c '^pub' "$OUT/api/$name.txt" 2>/dev/null || echo 0)
    printf '%-24s %5d pub items\n' "$name" "$n" >> "$OUT/api/index.txt"
  done
  cat "$OUT/api/index.txt"
else
  not_run "cargo-public-api" "not installed (EVAL tier): cargo install --locked cargo-public-api@0.52.0"
  head1 "fallback: count pub items with grep"
  crates_list | while IFS=$'\t' read -r name path; do
    n=$(grep -rn --include=*.rs -E '^\s*pub (fn|struct|enum|trait|mod|const|type|use)' "$path/src" 2>/dev/null | wc -l)
    printf '%-24s %5d pub items\n' "$name" "$n"
  done | tee "$OUT/api/index.txt"
fi

head1 "10b. semver-checks (needs a git baseline)"
if [ ! -d .git ]; then
  not_run "cargo-semver-checks" "no .git in this working copy (blocker B1) - no baseline ref"
elif need cargo-semver-checks; then
  cargo semver-checks check-release 2>&1 | tee "$OUT/semver.txt" | tail -30
else
  not_run "cargo-semver-checks" "not installed (EVAL tier)"
fi

head1 "10c. external type boundaries"
if need cargo-check-external-types; then
  crates_list | while IFS=$'\t' read -r name path; do
    [ -d "$path/src" ] || continue
    cargo check-external-types --package "$name" 2>&1 | tail -3
  done | tee "$OUT/external-types.txt"
else
  not_run "cargo-check-external-types" "not installed (EVAL tier)"
fi
