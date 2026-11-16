#!/usr/bin/env bash
# T12 - MUTATION TESTING (primary task). Answers: are the tests real?
#
# Usage:
#   12_mutants.sh list              # size the problem, no build
#   12_mutants.sh crate <name>      # full run for one crate
#   12_mutants.sh diff              # changed lines only (requires .git)
#   12_mutants.sh full [--shard i/N]
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

need cargo-mutants || exit 0
mkdir -p "$OUT/mutants"

# Never mutate: tests, generated code, the template member, the excluded tauri app.
EXCLUDE_RE='(^|/)(tests|benches|examples)/|_template|desktop-tauri|schemas/generated'

TIMEOUT="${MUTANTS_TIMEOUT:-60}"
JOBS="${MUTANTS_JOBS:-8}"

cmd="${1:-list}"
shift || true

case "$cmd" in
  list)
    head1 "mutant census (no build)"
    cargo mutants --list --workspace \
      --exclude-re "$EXCLUDE_RE" \
      2>/dev/null | tee "$OUT/mutants/list.txt" | tail -5
    total=$(grep -c ':' "$OUT/mutants/list.txt" 2>/dev/null || echo 0)
    say ""
    say "total mutants: $total"
    say "per crate:"
    grep ':' "$OUT/mutants/list.txt" 2>/dev/null | cut -d: -f1 | sed 's#.*/src/##' | cut -d/ -f1 | sort | uniq -c | sort -rn | head -40
    ;;
  crate)
    pkg="${1:?crate name required}"
    head1 "mutation run: $pkg (timeout ${TIMEOUT}s, jobs $JOBS)"
    mkdir -p "$OUT/mutants/$pkg"
    cargo mutants -p "$pkg" \
      --test-tool nextest \
      --jobs "$JOBS" --timeout "$TIMEOUT" \
      --exclude-re "$EXCLUDE_RE" \
      --json --output "$OUT/mutants/$pkg/outcomes.json" \
      2>&1 | tee "$OUT/mutants/$pkg/run.log" | tail -40
    ;;
  diff)
    if [ ! -d .git ]; then
      not_run "cargo-mutants --in-diff" "no .git in this working copy (blocker B1)"
      say "run this in the real clone:"
      say "  cargo mutants --in-diff <(git diff origin/main...) --test-tool nextest --timeout 60"
      exit 0
    fi
    base="${1:-origin/main}"
    head1 "mutation run: --in-diff against $base"
    cargo mutants --in-diff <(git diff "$base"...) \
      --test-tool nextest --jobs "$JOBS" --timeout "$TIMEOUT" \
      --exclude-re "$EXCLUDE_RE" \
      2>&1 | tee "$OUT/mutants/in-diff.log" | tail -40
    ;;
  full)
    # Per-shard output so a sharded matrix (CI) or a sequential local run
    # never overwrites another shard's evidence.
    shard="all"
    prev=""
    for a in "$@"; do
      [ "$prev" = "--shard" ] && shard="${a%%/*}"
      prev="$a"
    done
    head1 "full workspace mutation run (shard $shard) ${*:-}"
    cargo mutants --workspace --test-tool nextest \
      --jobs "$JOBS" --timeout "$TIMEOUT" \
      --exclude-re "$EXCLUDE_RE" "$@" \
      --output "$OUT/mutants/full-shard-$shard" \
      2>&1 | tee "$OUT/mutants/full-shard-$shard.log" | tail -40
    ;;
  *)
    say "usage: 12_mutants.sh {list|crate <name>|diff [base]|full [--shard i/N]}"
    exit 2
    ;;
esac

head1 "how to read this"
cat <<'TXT'
caught   = a test noticed the behaviour change  -> the test is real
MISSED   = no test noticed                      -> fake test OR dead code
timeout  = the mutant hung                      -> finding: unbounded loop
unviable = did not compile                      -> neutral

Every MISSED mutant is a defect in the test suite, and the line it points at
is a line of production code with no behavioural constraint.
TXT
