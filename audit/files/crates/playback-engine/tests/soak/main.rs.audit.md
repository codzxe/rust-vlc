# AUDIT: crates/playback-engine/tests/soak/main.rs
| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2025-06-14 |
| Lines / functions | 435 / 11 (7 config/instr fns + the soak test + triage + 3 parsing tests) |
| File verdict | REAL — a genuinely env-configured soak with dual instruments (counting-allocator live bytes + VmRSS), warmup exclusion, least-squares slope assertion, and anti-ritual liveness checks; the 2-min default executed green in the baseline (120.04s) |
| Reachable from shipped entry point? | N/A (test harness). The nightly lane it documents EXISTS and is honestly marked: `.github/workflows/soak-nightly.yml` verified present, "First-run status [PENDING first real 4 h run]" — the 4 h/5 MB/h tier has NEVER RUN (per its own header). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `ls .github/workflows/` + read soak-nightly.yml header (exists, non-blocking schedule-only, continue-on-error: false); own cargo re-run BLOCKED (disk full) — baseline cited: audit/runs/baseline/cargo-test-workspace.txt soak section: `12 passed; 0 failed ... finished in 120.04s` incl. `soak_loops_stay_memory_bounded ... ok` |

## 1. One-paragraph verdict
The soak harness is the most measurement-honest test infrastructure in this crate: warmup window excluded from the fit, two independent instruments, garbage env values refuse loudly instead of defaulting, liveness is asserted (minimum cycles, ≤25% op-skip rates, at least one completed seek and switch — the scenario docs even record the measured 100%-skip ritual they caught and fixed, "1597/1597 cycles skipped the seek"), and the assertion message prints the top-suspect phase for triage. Two scope notes temper it: "driven through the command layer" (line 3) means the in-test `lumen_soak_dispatch` — a local shape one layer below the shipped CLI dispatch, again serving `tracks.set-audio-track` which every shipped host refuses — and the headline 4 h/5 MB/h nightly tier has never actually run (the workflow itself says PENDING first real 4 h run). A smoke-tier invocation (`SOAK_MINUTES < 0.5`) can pass with no slope measured at all.

## 2. What actually works (verified)
- verified by baseline audit/runs/baseline/cargo-test-workspace.txt (soak section): `soak_loops_stay_memory_bounded ... ok`, `12 passed; 0 failed ... 120.04s` — the 2-min default really loops, samples, fits, and passes; parsing tests + memory slope-math test (`slope_math_is_exact_on_clean_lines`) + dispatch refusal tests all green.
- verified by reading `.github/workflows/soak-nightly.yml`: exists; schedule + workflow_dispatch only; explicit "do NOT add to required checks"; 240 min / 5 MB/h config present.
- Config parsers hand-checked: range and positivity guards on all three env knobs (minutes ≤ 1 week, sample ms ≤ 600 s, ceiling ≤ 100 000 MB/h).

## 3. Findings
F-001 [S3]
Where: crates/playback-engine/tests/soak/main.rs:1-6 + dispatch.rs:16-27
Code:
```
//! The 4 h loop of open → play → seek → track-switch → reopen, driven
//! **through the command layer** (ARCH-5 — see
//! `dispatch.rs` for the recorded lineage of the in-test
//! `lumen_soak_dispatch`)
```
What is wrong: "through the command layer" is the LOCAL soak dispatch, not the shipped `lumen_cli::dispatch::lumen_dispatch` (unreachable from this crate for the same cycle reason as scenarios_b — dispatch.rs:7-15 is honest about it). The local route serves `tracks.set-audio-track`, which the shipped CLI refuses ("track switching is card 3.A3") — so the soak's memory-bounded evidence for the switch op covers a composition no shipped host offers. The lineage IS recorded in dispatch.rs, but main.rs's headline phrase overstates.
Why it matters: soak evidence is cited as engine+command-layer health; the command layer exercised is a test-only superset (same family as scenarios_b host.rs audit F-001).
Confidence: HIGH — Verification: read dispatch.rs:1-27 + the CLI route (prior audit).
F-002 [S4]
Where: crates/playback-engine/tests/soak/main.rs:300-306, 309-320
Code:
```
    if minutes >= 0.5 {
        assert!(
            measured.len() >= 2,
```
and
```
    if let Some(slope) = live_slope {
```
What is wrong: below 0.5 minutes the measured-sample-count assertion is skipped and a `None` slope (fewer than 2 measured samples) silently skips THE assertion — a fractional smoke run can pass `soak_loops_stay_memory_bounded` having measured nothing about memory (liveness still enforced). Documented as the smoke tier, but the test name promises bounded memory on every configuration.
Confidence: HIGH — Verification: read (logic), baseline shows the default tier asserts.
F-003 [S4]
Where: crates/playback-engine/tests/soak/main.rs:223, 350
Code:
```
        .unwrap_or_else(|error| unreachable!("soak cycle {cycle_index} failed: {error}"));
```
What is wrong: cycle failure and report-write failure abort via `unreachable!` panic instead of a structured assertion — inside a test this is effectively a loud failure (acceptable), but it is the tree-wide unreachable-pattern; a mid-soak failure loses the report the run was producing.
Confidence: HIGH — Verification: read.
F-004 [S4]
Where: crates/playback-engine/tests/soak/main.rs:19-22 vs the workflow's own header
Code:
```
//! — wired as the scheduled job `.github/workflows/soak-nightly.yml`
//! (issue #18; its 5 MB/h ceiling is `[PROPOSAL]` until the first
//! real 4 h measurement says otherwise).
```
What is wrong: nothing false — the workflow exists and the proposal status is candidly recorded in both places; but the 4 h/5 MB/h tier the file headline advertises ("THE soak ... 4 h loop") has never executed (workflow: "[PENDING first real 4 h run]"). The design target remains unmeasured.
Confidence: HIGH — Verification: read workflow header.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ALLOC / live_bytes | 69-76 | REAL | — | process-global counting allocator; snapshot reader allocates nothing |
| parse_minutes | 97-114 | REAL | — | unset→2.0; garbage/zero/negative/>week refuse; fractional legal; tested |
| parse_sample_ms | 117-131 | REAL | — | 0 and >600 s refuse; tested |
| soak_minutes / soak_sample_ms | 134-163 | REAL | — | .must() panic on bad env (documented fail-loud) |
| parse_leak_mb_per_h | 141-158 | REAL | — | unset→50; bounds + positivity; tested |
| leak_mb_per_h | 166-168 | REAL | — | |
| report_path | 171-176 | REAL | — | empty string = unset |
| soak_loops_stay_memory_bounded | 187-354 | REAL | S3/S4 | the soak: warmup exclusion (max(1s, min(15%, 50%))), cadence sampling with pre-reserved capacity, liveness (MIN_CYCLES 3, skip rates ≤ 25%, ≥1 seek + ≥1 switch), slope fit over measured window only, ceiling assert with triage summary, markdown report (printed + optional file); F-001/F-002/F-003 |
| triage_summary | 357-367 | REAL | — | top-suspect phase in the failure message |
| tests::minutes_default_and_valid_shapes | 378-403 | REAL | — | 7 shapes incl. garbage/zero/negative/overflow |
| tests::leak_mb_per_h_default_and_valid_shapes | 405-424 | REAL | — | default/nightly/garbage/zero/negative |
| tests::sample_ms_default_and_valid_shapes | 426-434 | REAL | — | default/bounds/garbage |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| soak_loops_stay_memory_bounded | YES (default tier) | slope assertion over the measured window + liveness counters; a leak > ceiling or a ritual (skipped ops) fails it — the scenario docs record it actually caught a 100%-skip ritual during development |
| parsing tests (3) | YES | exact-value and error-shape assertions |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "wired as the scheduled job .github/workflows/soak-nightly.yml" | main.rs:19-20 | REAL | file exists, read |
| "nightly tightens it to the 5 MB/h design target" | main.rs:15 | REAL (config) / UNMEASURED (target) | workflow carries SOAK_LEAK_MB_PER_H=5; first real 4 h run PENDING per workflow header |
| "sampling memory at a fixed wall-clock cadence" | main.rs:5-6 | REAL | CadenceSampler ticks from inside pump (dispatch.rs); baseline run sampled |
| "Liveness is asserted (D-6)" | main.rs:40-42 | REAL | lines 246-286: cycles/skip-rates/op-count asserts |

## 7. Dead / duplicated / unreachable code
- None in this file. (dispatch.rs/scenario.rs/memory.rs/report.rs are siblings, not in my assignment; their lineage notes were read for context.)

## 8. Dependencies & flags
- testkit (CountingGlobalAlloc, serialize_tests, Must); sibling modules only. Env knobs: SOAK_MINUTES/SOAK_SAMPLE_MS/SOAK_LEAK_MB_PER_H/SOAK_REPORT + build-time PROFILE. No features. `--release` recommended for the nightly (doc), baseline ran debug.

## 9. Open questions for the next agent
- The first REAL 4 h run has never happened (workflow PENDING) — any doc/matrix claim of "4 h soak green" would be FALSE; only the 2-min CI-short profile is proven (docs/perf/ per the workflow comment — worth verifying that profile exists).
- dispatch.rs (SoakHost) and scenario.rs (fixture plans parsed from expectation TOMLs) are unaudited by me — the skip-rate denominators (2 hops/cycle) and the TOML line-scanner are their responsibility.
