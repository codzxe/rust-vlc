#!/usr/bin/env bash
# T21 - perf and memory regression baselines: bloat, monomorphization, allocations.
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

mkdir -p "$OUT/perf"

head1 "21a. binary bloat (cargo-bloat)"
if need cargo-bloat; then
  cargo bloat --release --workspace -n 25 2>&1 | tee "$OUT/perf/bloat.txt" | tail -30
else
  not_run "cargo-bloat" "not installed"
fi

head1 "21b. monomorphization (cargo-llvm-lines)"
if need cargo-llvm-lines; then
  cargo llvm-lines -p playback-engine 2>&1 | head -25 | tee "$OUT/perf/llvm-lines.txt"
else
  not_run "cargo-llvm-lines" "not installed"
fi

head1 "21c. benchmark harness present?"
n=$(grep -rln --include=Cargo.toml -E 'criterion|iai-callgrind|divan' crates apps tools 2>/dev/null | wc -l)
say "crates with a benchmark harness: $n"
if [ "$n" -eq 0 ]; then
  say "-> no benchmark harness exists: perf regressions are currently undetectable (finding F-T21-01)"
fi

head1 "21d. iai-callgrind (deterministic, Linux+valgrind only)"
if command -v valgrind >/dev/null 2>&1; then
  say "valgrind present; run: cargo bench --bench <name> (iai-callgrind)"
else
  not_run "iai-callgrind" "valgrind not available on this host (Linux+valgrind only)"
fi

head1 "21e. allocation hot-path check (dhat)"
n=$(grep -rn --include=*.rs -E 'dhat::' crates apps tools 2>/dev/null | wc -l)
say "dhat usages: $n (0 = zero-allocation directive D-1 is unmeasured)"
