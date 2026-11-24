# AUDIT: tests/soak/scenarios.toml
| Field | Value |
|---|---|
| Auditor | W16-A03 |
| Date | 2026-11-24 |
| Lines / functions | 128 / 8 defaults keys + 5 scenario blocks |
| File verdict | REAL |
| Reachable from shipped entry point? | YES from the test lane: `cargo test -p playback-engine --test soak_scenarios` — ran green in the committed baseline (:4502-4534 `test soak_scenario_suite ... ok`); also the ci.yml merge-gate default (comment :35-37) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read; rg baseline + .github; fixture existence checks |

## 1. One-paragraph verdict
A well-disciplined test-config plane: every budget/threshold/fixture list
lives here, the loader refuses unknown keys/wrong types/out-of-range values
loudly (comment :26-33, enforced by the Rust loader at
crates/playback-engine/tests/soak_scenarios/manifest.rs), and the file is
honest about its own softness — the 5 MB/h target is marked
"[PROPOSAL until the first real 4 h measurement blesses it]" with
`assert_target = false` on this CI profile (:60-65), and the fd budget is
"reported as not-measurable, never silently passed" on non-Linux (:70-72).
All 5 scenarios' fixtures exist on disk (verified). The suite ran green in
the baseline. The one quibble: the CI profile asserts only a 50 MB/h leak
ceiling — 10× the proposal — so the merge gate is nearly leak-blind by the
file's own admission.

## 2. What actually works (verified)
— Baseline: audit/runs/baseline/cargo-test-workspace.txt:4502-4534 —
soak_scenarios binary ran, `test soak_scenario_suite ... ok` (and the
sibling tests/soak/main.rs `soak_loops_stay_memory_bounded ... ok` at
:4495, which ran >60s per the harness's own note).
— Fixtures: all 5 referenced files exist (avi_mjpeg_pcm.avi, 2pcm.avi,
trunc3.avi, noidx0.avi — verified with test -f).
— Anti-ritual liveness knobs present: min_cycles=3 (:52), max_skip_rate=0.25
(:55) — a scenario that skips its ops fails.
— 3 shipped profiles exist (scenarios.toml, -nightly, -sandbox) matching the
loader's validation test claim (:31-33).

## 3. Findings
F-001 [S4]
Where: :59 vs :64 — `leak_ceiling_mb_per_h = 50.0` (ASSERTED here) vs
`target_mb_per_h = 5.0` (assert_target=false).
What is wrong: the CI merge gate can only catch leaks ≥ 50 MB/h — 10× the
design target; a real 5–50 MB/h regression is green on every PR.
Why it matters: the config names this honestly (generous to avoid flake),
but the practical consequence is the merge gate's leak sensitivity is
1/10th of the design intent. What correct looks like: a mid tier (e.g. 20
MB/h) on the sandbox profile with flake data behind it.
Confidence: HIGH — Verification: read.

## 4. Per-key table
| Key | Lines | Verdict | Notes |
|---|---|---|---|
| sample_ms / warmup_fraction | 45-49 | REAL | warmup excluded from slope fit; min 1 s / max 50 % enforced by loader |
| min_cycles / max_skip_rate | 52-55 | REAL | anti-ritual liveness (D-6) |
| leak_ceiling_mb_per_h | 59 | REAL | asserted; 10× target (F-001) |
| target_mb_per_h / assert_target | 64-65 | REAL | honest [PROPOSAL] marker; asserted only on nightly |
| rss_growth_budget_mb / fd_growth_budget / live_growth_budget_bytes | 69-76 | REAL | count-driven budgets; fd None off-Linux reported, never silently passed |
| scenario play-loop | 98-102 | REAL | 0.25 min; 3-fixture rotation incl. one malformed (trunc3) |
| scenario seek-storm | 104-109 | REAL | 0.2 min; advance_polls=1 |
| scenario playlist-cycle | 111-115 | REAL | 0.2 min; includes noidx0 (spec-violating variant) |
| scenario open-close-churn | 117-121 | REAL | 30 cycles; fd/RSS/live growth |
| scenario reopen-stress | 123-127 | REAL | 20 cycles full open→play→close |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| soak_scenario_suite (loads this file) | YES | loader refuses bad config; scenarios assert budgets/cycles/skip-rate; ran green in baseline |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "runs as part of the plain `cargo test --workspace` merge gate (ci.yml step 3)" | :35-37 | REAL | baseline ran the binary in the per-package sweep; soak-nightly.yml exists for the 4 h tier |
| "[PROPOSAL until the first real 4 h measurement blesses it]" | :60-62 | REAL (honest marker) | target not asserted on this profile (assert_target=false) |

## 7. Dead / duplicated / unreachable code
None; every key is loader-consumed (unknown keys are refused, per :29-30).

## 8. Dependencies & flags
Consumed via SOAK_SCENARIOS env override (:10-24) by the
playback-engine soak_scenarios test binary; schema twin at
tests/soak/scenarios.schema.json.

## 9. Open questions for the next agent
Has the 4 h nightly tier (scenarios-nightly.toml + soak-nightly.yml) ever
completed? No committed soak-report-*.md was checked by this audit (docs/soak
referenced at :62-63) — verify before citing the 5 MB/h target.
