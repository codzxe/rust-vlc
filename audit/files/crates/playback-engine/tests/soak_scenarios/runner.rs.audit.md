# AUDIT: crates/playback-engine/tests/soak_scenarios/runner.rs

| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-22 |
| Lines / functions | 1414 / 5 scenario drivers + ~25 helpers + 5 unit tests |
| File verdict | REAL (drivers + pure logic; the CI-short profile ran in the W00 baseline) |
| Reachable from shipped entry point? | N/A (test harness). It drives the real engine playback path (access → AVI demux → MJPEG/PCM decode → sink) over committed fixtures at the workspace `tests/fixtures/generated/` — the same first-vertical path the shipped CLI plays. |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:3 |
| Commands run | Read (full file, 3 chunks); Grep: `frames_delivered\|fn stats` (soak/dispatch.rs:397 — engine-lifetime stats), `pump_until` (scenario.rs:487 checks the bool; runner.rs:596/788 discard it), `seek_targets` (soak/scenario.rs:92-107 — empty-target validation), `SOAK_SCENARIOS` in .github/workflows + scripts (soak-nightly.yml:264, scripts/soak-scenarios.sh:88-91); Read tests/soak/scenarios.toml (CI-short default: 0.25/0.2/0.2 min + 30/20 cycles); fixtures verified present (`tests/fixtures/generated/{valid,malformed}/avi_mjpeg_*`). sed verification of every quoted range. No cargo — run claims cite W00 baseline. |

## 1. One-paragraph verdict

The scenario-runner half of the card 8.C1 soak suite: five config-driven
drivers (play-loop, seek-storm, playlist-cycle, open-close-churn,
reopen-stress) that hammer the REAL engine — real AVI fixtures, real
demux/decode, the counting allocator, /proc/self/status RSS and
/proc/self/fd instruments — with a genuinely serious measurement
discipline: warmup exclusion, bounded instrument memory (the
`MAX_COUNT_SAMPLES` stride, with the 2026-11-10 regression documented),
recorded-not-thrown violations, D-6 liveness counters, and honest
absence notes for off-Linux instruments. The default manifest
(`tests/soak/scenarios.toml`, CI-short, ~1 min total: 0.25/0.2/0.2 min
durations, 30/20 cycles) makes `soak_scenario_suite` a plain ungated
`#[test]`, so the drivers RAN in the W00 baseline's 1751; the 4-hour
nightly tier (48-min scenarios, 24M/1.95M cycles) is env-gated
(`SOAK_SCENARIOS=scenarios-nightly.toml`, soak-nightly.yml:264) and did
NOT run — REAL-UNPROVEN at that scale. One real honesty hole: both
play-to-end drains discard `pump_until`'s success bool, so a wedged
session that never reaches `Ended` closes silently and the cycle still
counts — the original `scenario.rs` checks exactly this bool. The rest
is hygiene: dead params, a forked helper, and inconsistent
frames-decoded bookkeeping in seek-storm.

## 2. What actually works (verified)

— The suite is wired to run by default: `#[test] fn soak_scenario_suite()` (main.rs:184, ungated) loads workspace-root `tests/soak/scenarios.toml` (CI-short, verified present with durations 0.25/0.2/0.2 min and cycles 30/20) → the five drivers in this file executed in W00's `cargo test --workspace` (part of the 1751 passing; the 2 baseline failures were first-vertical golden drift, not this suite).
— Real media, not mocks: fixtures resolve to committed files (`tests/fixtures/generated/valid/avi_mjpeg_pcm.avi`, `avi_mjpeg_2pcm.avi`, malformed `__trunc3`/`__noidx0` — verified present on disk), expectations read from recorded TOMLs, never transcribed (`FixturePlan::from_expectation`).
— Liveness is multi-layered and would fail on a ritual run: per-cycle decode evidence (:609-614), whole-run zero-frame check (:445-451), min_cycles (:439-444), seek-storm skip-rate cap (:824-834), zero-playlist-passes check (:661-667).
— Budgets are real arithmetic over real instruments: least-squares slope of live bytes vs the MB/h ceiling (:425-476, `check_slope` unit-tested at :1246-1276 including negative slopes and the None-line rule), count-scenario baseline-vs-final growth budgets for live/RSS/fd (:993-1021) with warmup-bounded baselines (:984, `count_baseline_and_span` recomputed from the committed raw evidence's own tag shapes at :1353-1396).
— The instrument's own memory is bounded and unit-tested (`count_sample_stride` :1142-1144, test :1312-1343 — the 2M-cycle 48 MB instrument regression is documented at :859-871 and fixed by the stride).
— Error-lane accounting is thought through: the double-fold hazard of a failed open leaving the previous sink in place is documented and handled at :741-744 and :951-957.
— Mutation reasoning (UNVERIFIED, no cargo): delete the leak assertion → a leaking engine still fails via `live_growth_budget_bytes`/slope; make the harness skip ops → D-6 catches it; break `check_slope`'s unit conversion → `slope_verdict_flags_budget_violations` fails (100 B/s must equal 0.36 MB/h).

## 3. Findings

### F-001 [S3] Both play-to-end drains discard `pump_until`'s result — a wedged session closes silently and the cycle still counts

Where: crates/playback-engine/tests/soak_scenarios/runner.rs:596-599, 787-791
Code:
```rust
    host.pump_until(
        |state| matches!(state, SessionState::Ended | SessionState::Error),
        PLAY_TO_END_POLLS,
    );
    let error_lane = host.state() == SessionState::Error;
```
What is wrong: `pump_until` (soak/dispatch.rs:489-496) returns `bool` — whether the predicate ever held. The original soak checks it: `let ended = host.pump_until(|state| state == SessionState::Ended, PLAY_TO_END_POLLS);` (soak/scenario.rs:487) and branches `CycleExit` on it. This file — which the header (:15) claims is "a second DRIVER over them, not a fork" — drops the check at both call sites. If the engine wedges in `Playing`/`Buffering` and 1024 polls exhaust without `Ended`/`Error`, the code proceeds to `SessionClose` (legal from active states), the cycle is counted (`run.counts.cycles += 1`, :617), decode evidence from before the wedge satisfies D-6, and nothing anywhere records that the `Ended` transition was never reached. There is no `ends_ok` counter in `ScenarioCounts` (:278-304) either — the report cannot reveal it.
Why it matters: the suite's selling point (:35-37) is "Liveness is asserted (D-6): cycles must complete, ops must run" — but a cycle whose drain never completed counts as complete. An engine regression that wedges mid-drain would sail through the soak (its leak numbers are still valid, its cycles still churn), and the Ended-path behavior the soak exists to exercise is silently never exercised. The 4-hour nightly tier would run 48 minutes against a wedged drain and pass.
What correct looks like: `if !host.pump_until(...) { run.fail(format!("drain: cycle {cycle_index} never reached Ended/Error within {PLAY_TO_END_POLLS} polls")); }` — or an `ends_ok`/`drain_timeouts` counter asserted at the end.
Confidence: HIGH — Verification: read (both call sites + the contrast in scenario.rs:487-491 + ScenarioCounts has no end counter).

### F-002 [S4] seek-storm's `frames_decoded` bookkeeping is internally inconsistent (delta-sum vs absolute overwrite)

Where: crates/playback-engine/tests/soak_scenarios/runner.rs:746-747, 782
Code:
```rust
                let decoded = host.stats().frames_delivered.saturating_sub(frames_before);
                run.counts.frames_decoded += decoded;
```
```rust
        run.counts.frames_decoded = host.stats().frames_delivered;
```
What is wrong: play-loop and the count scenarios accumulate per-cycle deltas (`+= decoded`, :615, :950); seek-storm adds the reopen-time delta at :746-747 and then, in the SAME loop iteration, overwrites the field with the engine's lifetime absolute at :782. The `+=` at :746-747 is dead-on-arrival (always overwritten ~20 lines later). The field's doc (:299-301) says "lifetime counter delta" — for seek-storm it holds an absolute. Correct today only because `EngineStats::frames_delivered` is engine-lifetime monotonic (session.rs:847, :861 — `saturating_add`) and the `SoakHost` is per-scenario; if stats were ever per-session, the whole-run zero-frame liveness check (:445-451) would silently weaken for seek-storm.
Why it matters: a measurement harness whose counters have two different semantics for the same field is one refactor away from a false pass; the dead `+=` misleads readers into thinking reopen deltas accumulate.
What correct looks like: delete :746-747's add (keep the count at :782's absolute), or use the delta-sum everywhere and document one semantics.
Confidence: HIGH — Verification: read (both sites + session.rs:861 monotonic add + dispatch.rs:397 `engine.stats()`).

### F-003 [S4] Forked helper + panic seam: `mid_media_targets` duplicates scenario.rs and slices without an empty guard

Where: crates/playback-engine/tests/soak_scenarios/runner.rs:250-253 (vs soak/scenario.rs:358-359)
Code:
```rust
fn mid_media_targets(plan: &FixturePlan) -> &[i64] {
    let len = plan.seek_targets_us.len();
    &plan.seek_targets_us[..len.saturating_sub(1).max(1)]
}
```
What is wrong: (1) byte-identical logic to `soak/scenario.rs:358-359` while the module header (:13-15) advertises "the manifest config plane, the dispatch seam and the memory instruments are one codebase; the scenario suite is a second DRIVER over them, not a fork" — the seek-rotation math IS forked. (2) On an empty `seek_targets_us` the slice `[..1]` panics ("range end index 1 out of range") instead of a typed sentence. Unreachable through the manifest path (from_expectation errors on empty targets after the parent fallback, scenario.rs:92-107) — but this file also calls itself the "not a fork" layer, and a forked panic seam is exactly what the strict-loader discipline exists to avoid.
Why it matters: duplication between two soak layers will drift (the `scenario.rs` rule comment is already copy-pasted); the panic turns a config error into an index-out-of-bounds stack trace.
What correct looks like: share one helper (or `plan.mid_media_targets()` on `FixturePlan`), and error, not panic, on empty.
Confidence: HIGH — Verification: read (both files' identical bodies; scenario.rs:92-107 guard).

### F-004 [S4] Dead parameters and ritual `let _ =` discards

Where: crates/playback-engine/tests/soak_scenarios/runner.rs:619, 913+971, 132-133
Code:
```rust
    let _ = warmup; // timing owned by the driver loop; parameter kept for symmetry
```
```rust
        let live_before = live();
```
```rust
        let _ = live_before; // the per-cycle delta folds into the series diff below
```
What is wrong: `play_loop_cycle` takes a `warmup` parameter it never uses; `run_count_scenario` captures `live_before` every cycle (:913) and discards it every cycle (:971) — the comment's "folds into the series diff" rationalization describes the series sampling, not this dead local. `ScenarioPlan::label` carries `#[allow(dead_code)]` (:132) although the field IS read through the accessor used at :482/:612/:947 — a vestigial allow that suppresses nothing today and would mask a real regression to unread.
Why it matters: dead state in a measurement harness is noise at best and masked lints at worst; `#[allow]` that no longer applies is lint debt by definition (§3.5).
What correct looks like: delete the parameter, the local, and the allow.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `run_scenario` | 92-105 | REAL | — | dispatch on Kind; harness breaks = Err |
| `fixture_plans` | 114-126 | REAL | — | existence-checked plans; per-fixture error sentences |
| `ScenarioPlan` (+accessor) | 130-142 | REAL | S4 | vestigial `#[allow(dead_code)]` (F-004) |
| `expectation_for` | 150-168 | REAL | — | strict area mapping; error paths unit-tested (:1399-1413) |
| `RequestIds` | 175-192 | REAL | — | saturating; collision-harmless design documented |
| `open_request`/`play_request`/`close_request` | 195-223 | REAL | — | thin wire builders |
| `seek_request` | 228-246 | REAL | — | 4-way rotation (absolute/relative/percent/absolute) |
| `mid_media_targets` | 250-253 | REAL | S4 | F-003: fork of scenario.rs:358 + empty-vec panic seam |
| `dispatch` | 260-266 | REAL | — | refusal → contextualized harness break |
| `in_playback` | 269-274 | REAL | — | Playing|Buffering |
| `ScenarioCounts` | 277-304 | REAL | S3 | no `ends_ok` counter — feeds F-001's silence |
| `ScenarioRun` | 309-336 | REAL | — | violations recorded, notes never failed |
| `DurationClock` | 343-384 | REAL | — | warmup min 1 s max half; idle-sighting end |
| `finish_duration_scenario` | 389-507 | REAL | — | slope fits, liveness, degenerate-window failure (:470-476); force-ends warmup |
| `run_play_loop` | 511-550 | REAL | — | fixture rotation; F-001 drain hole |
| `fold_sink_bytes` | 557-561 | REAL | — | close-site read point; double-fold hazard documented |
| `play_loop_cycle` | 565-621 | REAL | S3/S4 | F-001 (drain), F-004 (`let _ = warmup`); error lane clean; per-cycle D-6 real |
| `run_playlist_cycle` | 625-677 | REAL | — | strict sequential passes; zero-pass liveness |
| `run_seek_storm` | 683-846 | REAL | S4 | F-001 (epoch drain :788), F-002 (bookkeeping); skip-rate cap; no-fold-on-error-lane correctness argued |
| `expected_samples` | 851-853 | REAL | — | capacity only, not a growth term |
| `MAX_COUNT_SAMPLES` const | 859-871 | REAL | — | documents the 2026-11-10 instrument-memory regression |
| `run_count_scenario` | 877-1106 | REAL | S4 | bounded stride sampling; baseline-vs-final budgets; honest-absent notes; F-004 `live_before`; wall-cap shortfall is a note, not a fail |
| `count_warmup_cycles` | 1110-1113 | REAL | — | clamps unit-tested |
| `count_baseline_and_span` | 1124-1134 | REAL | — | W24-R4 P2-3 fix; recomputed from committed evidence shapes |
| `count_sample_stride` | 1142-1144 | REAL | — | div_ceil; bounds unit-tested incl. u32::MAX |
| `pump_until_frames` | 1148-1155 | REAL | — | bounded decode-evidence wait; no failure signal on exhaustion (silence acceptable: the per-cycle D-6 check at :940-949 is the assert) |
| `read_fd_count` (linux) | 1168-1174 | REAL | — | counts /proc/self/fd; read_dir's own fd constant — documented |
| `read_fd_count` (non-linux) | 1182-1186 | REAL | — | honest None, never a fake zero |
| `SlopeVerdict`/`check_slope` | 1195-1225 | REAL | — | pure; unit-tested both directions |
| tests: `slope_verdict_flags_budget_violations` | 1246-1276 | REAL | — | concrete arithmetic (0.36 MB/h), negative slope, None line |
| tests: `count_warmup_cycles_clamps` | 1280-1304 | REAL | — | boundary cases incl. n=1→0 |
| tests: `count_sample_stride_bounds_the_instrument_memory` | 1311-1343 | REAL | — | 2M→stride 489; u32::MAX safe |
| tests: `count_baseline_and_span_matches_the_measured_tags` | 1352-1396 | REAL | — | recomputes the committed raw reports' own shapes (P2-3 regression) |
| tests: `expectation_paths_map_generated_areas` | 1399-1413 | REAL | — | mapping + 3 refusal paths |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| slope_verdict_flags_budget_violations | YES | exact MB/h arithmetic, both flags, assert_target gating, None/negative rules |
| count_warmup_cycles_clamps | YES | exact boundary values |
| count_sample_stride_bounds_the_instrument_memory | YES | stride/capacity bounds at 1, 1000, 4096, 4097, 2M, u32::MAX |
| count_baseline_and_span_matches_the_measured_tags | YES | exact idx/span from the committed reports' tag shapes |
| expectation_paths_map_generated_areas | YES | 2 mappings + 3 refusals |
| (via main.rs) soak_scenario_suite, CI-short profile | YES for budget/liveness violations; NO for wedged drains | assert on collected failures (:234-240 of main.rs); F-001 hole: a never-Ended cycle passes |

The five unit tests are ungated and ran in the W00 baseline. The scenario suite ran too (default manifest, ~1 min); the nightly/sandbox/forced-threshold tiers are env-gated and did NOT run in the baseline — their passing status is REAL-UNPROVEN (nightly workflow exists: soak-nightly.yml:264; run history unverified).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the scenarios exercise the real engine playback path … not a mock" | header :7-12 | REAL | fixtures on disk; dispatch over SoakHost → real Engine; ran in W00 baseline via the default manifest |
| "not a fork" (one codebase, second driver) | header :13-15 | PARTLY FALSE | `mid_media_targets` and `PLAY_TO_END_POLLS` are forked from scenario.rs — and the fork dropped scenario.rs's `ended` check (F-001, F-003) |
| "Liveness is asserted (D-6): cycles must complete, ops must run, frames must decode" | header :35-37 | SHALLOW at the margin | per-cycle frames + min_cycles + skip-rate are real; "complete" does not include reaching `Ended` — the drain-bound exhaustion is silent (F-001) |
| "Violations are recorded, not thrown … the test asserts the failure list is empty at the end" | header :38-43 | REAL | `run.fail` accumulation; main.rs:234-240 asserts emptiness |

## 7. Dead / duplicated / unreachable code

- `mid_media_targets` duplicates soak/scenario.rs:358-359 (F-003).
- `let _ = warmup;` (:619), `live_before` captured-and-discarded (:913/:971), vestigial `#[allow(dead_code)]` (:132) — F-004.
- `run.counts.frames_decoded += decoded;` (:746-747) always overwritten at :782 — F-002.

## 8. Dependencies & flags

No feature flags. The suite runs as plain `cargo test -p playback-engine --test soak_scenarios`; env knobs (`SOAK_SCENARIOS`, `SOAK_SCENARIOS_ONLY`, `SOAK_SCENARIOS_REPORT`) select manifest/filter/report — defaults make the CI-short profile part of the W00 baseline merge-gate run. Nightly tier: soak-nightly.yml:264 (env-gated; not in the baseline). Platform gating: `#[cfg(target_os = "linux")]` for the fd instrument with an honest None lane elsewhere (:1168-1186).

## 9. Open questions for the next agent

- F-001's hole interacts with the nightly 4 h tier: if a wedge ever appears mid-drain, the 48-min scenarios pass. The claim ledger should check whether any "soak green" evidence predates/postdates the `ended` check's removal (files: docs/soak/soak-report-*.md, worklog).
- `ScenarioCounts.frames_decoded` semantics (F-002) should be reconciled before any report cross-check compares engine counters against sink counters.
- Whether `soak_scenario_suite`'s ~1 min wall budget holds on the CI runners the W00 baseline used (baseline log has the suite's timing — not re-derived here).
