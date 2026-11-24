# AUDIT: scripts/soak-scenarios.sh
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Date | 2026-06-14 |
| Lines / functions | 103 / 1 (arg-parse + invoke wrapper) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as tooling — referenced by CI/soak lanes (soak-nightly workflow context); it wraps `cargo test -p playback-engine --test soak_scenarios` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |

## 1. One-paragraph verdict
A thin, honest wrapper that does exactly what its header promises: config-driven (no policy in the script — durations/budgets live in the TOML manifests), honest exit-code propagation (rc captured and re-exited), loud refusals (unknown option exit 2, missing manifest exit 2), and `--list` explicitly disclaimed as grep-convenience-not-validation (the loader enforces the contract). I EXECUTED `--list` (ran: printed 5+ scenario ids) and `bash -n` (clean). Two hygiene notes: `set -uo pipefail` without `-e` (deliberate — rc hand-propagated — but the `cd` chain has no guard, so a failed `cd` degrades to a confusing later error rather than a clean exit), and the header's "Shipped profiles" list omits the existing scenarios-forced-threshold.toml.

## 2. What actually works (verified)
— `bash -n scripts/soak-scenarios.sh` -> OK (ran).
— `scripts/soak-scenarios.sh --list` -> prints ids (play-loop, seek-storm, playlist-cycle, open-close-churn, reopen-stress, ...) (ran, exit 0).
— Profiles exist: tests/soak/{scenarios,scenarios-nightly,scenarios-sandbox,scenarios-forced-threshold}.toml + scenarios.schema.json (ls, ran).
— Suite exists: crates/playback-engine/tests/soak_scenarios/ (ls, ran).
— Exit propagation read: L100-103 `rc=$?; echo; exit "${rc}"` — the header's "Exit status: the cargo test exit status" claim is TRUE in code.
— Arg parsing read: every value-option requires an argument (`${2:?}` loud), unknown option exits 2.

## 3. Findings

F-001 [S4]
Where: scripts/soak-scenarios.sh:39-43
Code:
```
set -uo pipefail
...
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "${here}/.." && pwd)"
cd "${root}"
```
What is wrong: no `-e`, and the `cd`s are unguarded — a failed `cd "${root}"` (e.g. deleted cwd) leaves the script running in the caller's directory; the first symptom is the manifest-existence check failing with a misleading path. Fail-loud posture is weakened only in this corner. What correct looks like: `cd "${root}" || exit 2` (or set -e with the rc handoff restructured). Confidence: HIGH — Verification: read.

F-002 [S4]
Where: scripts/soak-scenarios.sh:11-14 (header "Shipped profiles")
What is wrong: the list names 3 profiles; tests/soak/ actually contains 4 (+ scenarios-forced-threshold.toml, a threshold-testing profile). Harmless omission; doc lag. Confidence: HIGH — Verification: ls (ran).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| arg-parse + invoke | 39-103 | REAL | S4 | while/case parser with :? guards; env exports (SOAK_SCENARIOS/_ONLY/_REPORT); cargo invocation; rc handoff (F-001 posture corner) |
| --list / --help | 60, 73-76 | REAL | - | both executed OK; --list disclaimed as non-validation |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| (wrapper; no tests) | n/a | the wrapped suite (soak_scenarios) owns the liveness/memory budget asserts; this script's job is honest rc propagation — verified in code |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "honest exit-code propagation ... a red run is a finding, never noise" | L35-38 | REAL | L100-103 rc capture + exit (read) |
| "every duration, cycle count, budget, threshold and fixture list lives in the selected tests/soak/*.toml manifest, never here" | L7-9 | REAL | no numeric policy constants in the script (read); 4 manifests exist (ran) |
| "--list ... this listing is not validation" | L70-72 | REAL | disclaimed in-band |
| "the 4 h nightly tier (Soak-Nightly)" | L13 | REAL-UNPROVEN | scenarios-nightly.toml exists; the 4 h wall-clock run itself is the baseline-known open follow-up (phase-4 exit §4.3) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
bash, cargo test (host); SOAK_SCENARIOS* env contract. No literals beyond default paths.

## 9. Open questions for the next agent
1. scenarios-forced-threshold.toml — whose lane? (F-002 doc lag.)
2. soak-nightly workflow — does it call this script with --release + the nightly profile, and is the 4 h wall-clock claim anywhere proven (phase4-exit §4.3 says still open)?
