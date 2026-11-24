# AUDIT: tests/soak/scenarios-sandbox.toml
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A05) |
| Date | 2026-06-14 |
| Lines / functions | 77 / config: defaults block + 5 [[scenario]] tables |
| File verdict | REAL — measured, evidence-committed soak profile |
| Reachable from shipped entry point? | NO — test config; consumed via env `SOAK_SCENARIOS` by crates/playback-engine/tests/soak_scenarios/main.rs:87-94 (loader refuses loudly, never defaults); wrapped by scripts/soak-scenarios.sh (default profile = this file, header :18) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cat scripts/soak-scenarios.sh` (wrapper contract + `set -uo pipefail` :34); `ls docs/soak/raw/` -> 5 scenario reports + calibration + forced-threshold; `head -30 docs/soak/raw/scenarios-sandbox__play-loop.md` (measured evidence against THIS manifest); `rg -n "SOAK_SCENARIOS" crates/playback-engine/tests/soak_scenarios/main.rs` -> :23-25/:87/:94; fixture existence ls |

## 1. One-paragraph verdict
A coherent bounded soak profile that does what its header claims: five scenarios summing to ~34 min (24 duration-driven + two count-driven capped at 5 min each — the "~35 minutes" header rounds up by 1); every duration, cycle count, budget and fixture list lives here, not in the runner; fixtures (avi parents + trunc3 + noidx0) all exist on disk and all four are registered fixtures. Committed measured evidence exists for all five scenarios (docs/soak/raw/scenarios-sandbox__*.md, captured 2026-11-10) with the honest extrapolation disclosure (50 MB/h asserted ceiling; 5 MB/h target reported, not asserted). Two hygiene defects: the committed reports record a FOREIGN working-tree manifest path, and the wrapper script omits `set -e`.

## 2. What actually works (verified)
— verified by command: all 5 scenario report files exist (ls docs/soak/raw); the play-loop report's manifest line names this file, its fixture list matches the TOML's play-loop row, wall 480.0 s matches duration_min 8.0, ceiling/target match leak_ceiling_mb_per_h 50.0 / target_mb_per_h 5.0, and the warmup exclusion matches warmup_fraction 0.15 (73/482 samples ≈ 15%). — consumer wiring: main.rs:87-94 resolves SOAK_SCENARIOS with loud refusal. — calibration numbers recorded in-TOML match the committed calibration doc (reopen-stress 1500 cycles in 3.0 s -> 150k cycles ≈ 5-min cap: report exists at docs/soak/raw/scenarios-sandbox__reopen-stress__calibration.md). Execution itself: UNVERIFIED in this environment (35-min release run; no cargo here) but the committed evidence is the run's own output.

## 3. Findings
F-001 [S4]
Where: docs/soak/raw/scenarios-sandbox__play-loop.md:3 (cross-ref — evidence generated from THIS config)
Code:
```
- Manifest: `/home/z/my-project/wt/w24b/tests/soak/scenarios-sandbox.toml` (5 scenario(s))
```
What is wrong: every committed scenario report records the manifest path of a FOREIGN working tree (`/wt/w24b/…`), not this repository's root — the evidence's byte-equivalence to this tree's fixtures/manifest is unproven from the reports alone. Why it matters: provenance traceability: an auditor re-running this profile here cannot confirm the committed numbers came from the same bytes the current ledger pins. What correct looks like: re-capture in the canonical checkout or record fixture sha256s in the report. Confidence: HIGH — Verification: read (path is printed verbatim). Fixture-byte equivalence UNVERIFIED.
F-002 [S4]
Where: scripts/soak-scenarios.sh:34 (cross-ref — the wrapper that selects this profile)
Code:
```
set -uo pipefail
```
What is wrong: no `-e`; the script relies on the final cargo exit status while intermediate failures (report-dir creation, tooling) continue silently. The header (:27-33) documents deliberate exit-code propagation, so this is a documented trade-off, but a failing pre-run step would mask itself. Confidence: MEDIUM — Verification: read.

## 4. Per-function table
File <100 LOC — [defaults] (sample_ms 1000, warmup 0.15, min_cycles 3, max_skip_rate 0.25, ceiling/target, assert_target false, rss/fd/live budgets) + 5 scenarios (play-loop 8 min, seek-storm 8 min + advance_polls 1, playlist-cycle 8 min, open-close-churn 2M cycles/5 min, reopen-stress 150k cycles/5 min). All values coherent with the header and the committed reports.

## 5. Test honesty
N/A (config). The honesty posture is good: `assert_target = false` + the freeze-or-revise note refuse to assert an extrapolated 5 MB/h from a 34-minute window.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "~35 minutes ... 24 min duration-driven + two 5-min caps" | header :4-6 | REAL | 8+8+8 + 5+5 = 34 min; reports show 480 s walls |
| "every duration and cycle count set HERE — the runner is identical, only this file differs" | header :8-9 | REAL | main.rs:23 documents env-driven manifest; wrapper is plumbing (:9-11) |
| "the 5 MB/h target is reported with a freeze-or-revise verdict ... rather than asserted" | header :18-20 | REAL | assert_target = false :28; report line "reported, not asserted" |

## 7. Dead / duplicated / unreachable code
None. All 4 referenced fixtures exist; trunc3/noidx0 are MANIFEST-registered rows (CORPUS.md table).

## 8. Dependencies & flags
TOML schema 1 parsed by soak_scenarios/manifest.rs; no feature gates.

## 9. Open questions for the next agent
Re-run this profile in the canonical checkout to bind the evidence to this tree's bytes (F-001).
