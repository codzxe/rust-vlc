#!/usr/bin/env bash
# soak-scenarios.sh — the bounded soak scenario suite wrapper (card 8.C1,
# issue #54 / P8-3).
#
# Runs the config-driven soak scenario suite
# (crates/playback-engine/tests/soak_scenarios/) with honest exit-code
# propagation: every duration, cycle count, budget, threshold and fixture
# list lives in the selected tests/soak/*.toml manifest, never here (the
# SOAK_MINUTES dirty-world rule — this script is plumbing, not policy).
#
# Shipped profiles:
#   tests/soak/scenarios.toml          CI-short (~1 min; the merge-gate default)
#   tests/soak/scenarios-nightly.toml  the 4 h nightly tier (Soak-Nightly)
#   tests/soak/scenarios-sandbox.toml  the bounded sandbox measurement (~34 min)
#
# Usage:
#   scripts/soak-scenarios.sh [--profile PATH] [--only ID] [--report PATH]
#                             [--report-dir DIR] [--release] [--list] [-h]
#
#   --profile PATH   manifest to run (default: tests/soak/scenarios-sandbox.toml)
#   --only ID        run only the named scenario (SOAK_SCENARIOS_ONLY — the
#                    sandbox chunking knob: a wall-capped sandbox drives the
#                    scenarios as separate processes, one per scenario, each
#                    with a clean allocator baseline; CI and the nightly never
#                    set it). Unknown ids fail loudly inside the suite.
#   --report PATH    write the suite markdown report to PATH (with --only, the
#                    default is <report-dir>/<profile-stem>__<id>.md)
#   --report-dir DIR per-scenario report directory (default: docs/soak/raw)
#   --release        --release build (the sandbox/nightly tier; default is the
#                    debug build, matching the CI-short merge gate)
#   --list           print the profile's scenario ids (grep convenience; the
#                    loader inside the suite is the contract enforcer) and exit
#   -h, --help       this text
#
# Exit status: the cargo test exit status — non-zero when any configured
# liveness or memory budget is violated (the runner records violations,
# completes every scenario for triage, then asserts; a red run is a
# finding, never noise).
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "${here}/.." && pwd)"
cd "${root}"

profile="tests/soak/scenarios-sandbox.toml"
only=""
report=""
report_dir="docs/soak/raw"
profile_arg="debug"
list=0

while [ $# -gt 0 ]; do
  case "$1" in
    --profile)    profile="${2:?--profile needs a PATH}"; shift 2 ;;
    --only)       only="${2:?--only needs a scenario ID}"; shift 2 ;;
    --report)     report="${2:?--report needs a PATH}"; shift 2 ;;
    --report-dir) report_dir="${2:?--report-dir needs a DIR}"; shift 2 ;;
    --release)    profile_arg="release"; shift ;;
    --list)       list=1; shift ;;
    -h|--help)    sed -n '2,40p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "soak-scenarios.sh: unknown option: $1 (see --help)" >&2; exit 2 ;;
  esac
done

if [ ! -f "${profile}" ]; then
  echo "soak-scenarios.sh: manifest not found: ${profile}" >&2
  exit 2
fi

# --list: the ids in manifest order (grep convenience ONLY — the suite's
# loader (soak_scenarios/manifest.rs) enforces the schema contract and
# refuses loud on unknown keys/types/ranges; this listing is not validation).
if [ "${list}" -eq 1 ]; then
  sed -n 's/^id = "\(.*\)"$/\1/p' "${profile}"
  exit 0
fi

# The per-scenario report default (flat, greppable, commit-friendly).
if [ -z "${report}" ] && [ -n "${only}" ]; then
  stem="$(basename "${profile}" .toml)"
  report="${report_dir}/${stem}__${only}.md"
fi
mkdir -p "$(dirname "${report:-/dev/null}")" 2>/dev/null || true

# The invocation (the suite's own documented convention,
# tests/soak_scenarios/main.rs header). --nocapture so the markdown report
# reaches the log even when SOAK_SCENARIOS_REPORT is unset.
cmd=(cargo test -p playback-engine --test soak_scenarios)
[ "${profile_arg}" = "release" ] && cmd+=(--release)

export SOAK_SCENARIOS="${profile}"
[ -n "${only}" ] && export SOAK_SCENARIOS_ONLY="${only}"
if [ -n "${report}" ]; then
  export SOAK_SCENARIOS_REPORT="${report}"
  echo "soak-scenarios: profile=${profile} only=${only:-<all>} build=${profile_arg} report=${report}"
else
  echo "soak-scenarios: profile=${profile} only=${only:-<all>} build=${profile_arg} report=<stdout only>"
fi

"${cmd[@]}" -- --nocapture
rc=$?
echo "soak-scenarios: exit ${rc}"
exit "${rc}"
