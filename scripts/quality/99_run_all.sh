#!/usr/bin/env bash
# 99_run_all.sh - the whole audit program in one command.
#
# Usage:
#   ../../scripts/quality/99_run_all.sh static   # cheap, no full build   (~10 min)
#   ../../scripts/quality/99_run_all.sh quality  # + coverage, mutation    (hours)
#   ../../scripts/quality/99_run_all.sh all      # everything            (nights)
#   ../../scripts/quality/99_run_all.sh report   # just regenerate BASELINE/FINDINGS
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$HERE/common.sh"

MODE="${1:-static}"

# Failure plumbing.
#   J  = run a REPORTER: its exit code is recorded but does not fail the tier.
#   JG = run a GATE: its exit code fails the tier (and therefore the CI job).
# Without this, every tier exited 0 no matter what a script reported - a CI that
# cannot fail is theatre.
FAILED=()
GATES=()
J()  { bash "$HERE/$1" "${@:2}"; local rc=$?
       [ "$rc" -ne 0 ] && FAILED+=("$1 rc=$rc"); return 0; }
JG() { bash "$HERE/$1" "${@:2}"; local rc=$?
       if [ "$rc" -ne 0 ]; then FAILED+=("$1 rc=$rc (GATE)"); GATES+=("$1 rc=$rc"); fi; return 0; }

case "$MODE" in
  static)
    : > "$OUT/not-run.txt"
    J 01_clippy.sh
    J 03_astgrep.sh
    J 05_deadcode.sh
    J 06_duplication.sh
    J 08_typos.sh
    J 07_supply_chain.sh
    J 15_test_infra.sh
    J 17_proptests.sh
    J 19_concurrency.sh
    J 13_test_hygiene.sh
    J 28_format_coverage.sh
    ;;
  quality)
    J 02_rustc_lints.sh
    J 03_astgrep.sh
    J 10_api_surface.sh
    J 11_coverage.sh
    J 14_crap.sh
    J 13_test_hygiene.sh
    J 12_mutants.sh list
    ;;
  dynamic)
    J 16_fuzz.sh list
    J 18_ub_checks.sh overflow
    J 09_features.sh
    J 21_perf.sh
    JG 26_complexity.sh
    ;;
  security)
    # Attacker-mode: what hostile input can do, not what the code looks like.
    J 22_attack_surface.sh
    JG 24_panic_budget.sh
    JG 23_alloc_budget.sh
    ;;
  speed)
    # Every corpus in the tree vs ffmpeg. Needs ffmpeg + a release build.
    JG 27_speed_parity.sh "${@:2}"
    ;;
  all)
    bash "$HERE/99_run_all.sh" static
    bash "$HERE/99_run_all.sh" security
    bash "$HERE/99_run_all.sh" quality
    bash "$HERE/99_run_all.sh" dynamic
    exit $?
    ;;
  report)
    ;;
  *)
    echo "usage: 99_run_all.sh {static|security|quality|dynamic|speed|all|report}"
    exit 2
    ;;
esac

head1 "tier result: $MODE"
if [ "${#FAILED[@]}" -gt 0 ]; then
  say "non-zero reporters (recorded, do not fail the tier):"
  printf '  - %s\n' "${FAILED[@]}"
fi
if [ "${#GATES[@]}" -gt 0 ]; then
  say ""
  say "GATE FAILURES (these fail the CI job):"
  printf '  FAIL %s\n' "${GATES[@]}"
fi

head1 "generating report"
$PY "$HERE/report.py" 2>&1 | tail -30
say ""
say "BASELINE : docs/audit/BASELINE.md"
say "FINDINGS : docs/audit/FINDINGS.md"
say "RAW      : docs/audit/reports/"

if [ "${#GATES[@]}" -gt 0 ]; then
  exit 1
fi
exit 0
