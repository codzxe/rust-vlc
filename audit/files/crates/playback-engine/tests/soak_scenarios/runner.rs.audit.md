# AUDIT: crates/playback-engine/tests/soak_scenarios/runner.rs
| Field | Value |
|---|---|
| Auditor | W03-A08 |
| Date | 2025-06-27 |
| Lines / functions | 1414 / 34 (+5 unit tests) |
| File verdict | REAL (runs in the default CI lane, drives the real engine stack with real AVI demux + real MJPEG/PCM decoders, real wall-clock memory instruments; one documented assertion contract is violated by its own callers) |
| Reachable from shipped entry point? | The runner itself is test-harness code (NO — never shipped). The stack it drives is the shipped one: dispatch.rs `SoakHost { engine: Engine }` → `demux::avi::AviDemuxer` → `codecs::video::mjpeg::MjpegDecoder` / `codecs::audio::pcm::PcmDecoder` (tests/soak/dispatch.rs:207,218) — the same `playback_engine::Engine` apps/cli/src/host.rs:231 ships. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `sed -n` quotes (exit 0); `rg` over dispatch.rs/scenario.rs/manifest.rs/workflows (exit 0); baseline: cargo-test-workspace.txt:4500-4536 — soak_scenarios binary "32 passed; 0 failed" in 39.22 s incl. `test soak_scenario_suite ... ok` and all 5 `runner::tests` |

## 1. One-paragraph verdict
This is a genuine measurement harness, not a ritual: every op goes through `lumen_soak_dispatch` onto a real `Engine` with the real AVI demuxer and the real MJPEG/PCM codecs (verified in tests/soak/dispatch.rs:199-228 — `MjpegDecoder::new()`, `PcmDecoder::with_format`), the live-bytes instrument is a process-global counting allocator, RSS/fd come from `/proc`, warmup exclusion and least-squares slope math are correct (memory.rs:70-91 is textbook OLS with a degenerate guard), and degenerate measurement windows are recorded as failures rather than passes. The default manifest profile (tests/soak/scenarios.toml) is honestly labeled CI-short: 0.25/0.2/0.2 min and 30/20 cycles — a smoke-tier "soak" whose real tier is the 4 h nightly (soak-nightly.yml job `soak-scenarios`, marked "NOT PROVEN: this 4 h nightly profile itself" — honest). The one real defect: `pump_until`'s documented contract ("the caller asserts the returned truth") is violated at both runner call sites — a stalled play-to-End drain is silently swallowed as long as the subsequent close still succeeds.

## 2. What actually works (verified)
— Baseline cargo-test-workspace.txt:4500-4536: soak_scenarios binary, 32 passed / 0 failed in 39.22 s, including `soak_scenario_suite ... ok` (the real-engine 5-scenario run) and `runner::tests::*` (5/5).
— CI wiring: .github/workflows/ci.yml:225 "test — incl. scenarios_b + soak"; nightly 4 h profile at soak-nightly.yml:193-195 (`scripts/soak-scenarios.sh --profile tests/soak/scenarios-nightly.toml --release`).
— Unit tests are mutation-sensitive: `slope_verdict_flags_budget_violations` pins the exact MB/h arithmetic (0.36 = 100 B/s), `count_baseline_and_span_matches_the_measured_tags` recomputes the committed raw evidence's own shapes (2M/150k/1M series) — these fail on drift.

## 3. Findings
F-001 [S3]
Where: crates/playback-engine/tests/soak_scenarios/runner.rs:596-599 and 787-791
Code:
```
    host.pump_until(
        |state| matches!(state, SessionState::Ended | SessionState::Error),
        PLAY_TO_END_POLLS,
    );
```
What is wrong: `pump_until` returns `bool` and its own doc contract (tests/soak/dispatch.rs:487-488) says "a hang is a failed test — the caller asserts the returned truth". Both runner call sites drop the return value. If a session stalls mid-drain (never reaches Ended/Error in 1024 polls), the runner proceeds to `close_request`; if close succeeds from the stuck state, the cycle still counts, no liveness failure is recorded, and the stall is invisible to the report — a soaked regression that exactly stalls playback-to-end escapes. The per-cycle D-6 check only fires on zero decoded frames, so a stall after the first frame passes. What correct looks like: `assert!(host.pump_until(...), "play-to-end drain stalled ...")` or record a failure. Confidence: HIGH — Verification: read both call sites + the callee contract.

F-002 [S4]
Where: crates/playback-engine/tests/soak_scenarios/runner.rs:248-253 vs tests/soak/scenario.rs:357-359
Code:
```
/// The mid-media rotation slice: every recorded target except the
/// end-adjacent last (the `scenario.rs` rule; a single target stays).
fn mid_media_targets(plan: &FixturePlan) -> &[i64] {
```
What is wrong: `mid_media_targets` is implemented twice — verbatim in the shared module (scenario.rs:358-359, included via `#[path]`) and again here. The rule can drift between the original soak loop and the scenario suite. What correct looks like: reuse the shared module's function (the suite already `#[path]`-includes scenario.rs). Confidence: HIGH — Verification: read both.

F-003 [S4]
Where: crates/playback-engine/tests/soak_scenarios/runner.rs:619 and 971
Code:
```
    let _ = warmup; // timing owned by the driver loop; parameter kept for symmetry
...
        let _ = live_before; // the per-cycle delta folds into the series diff below
```
What is wrong: two dead bindings kept "for symmetry"/documentation; `live_before` (line 913) performs a counting-allocator read whose value is never used. Comments justify intent, but dead reads in a memory-measurement harness are noise. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/playback-engine/tests/soak_scenarios/runner.rs:132-133
Code:
```
    #[allow(dead_code)] // read by the report through the accessor below
    label: String,
```
What is wrong: vestigial `#[allow(dead_code)]` — the field IS read (via the `label()` accessor at lines 482 and 1079); the suppression masks nothing and is one of the tree's lint-suppression debts. Confidence: HIGH — Verification: read + rg call sites.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| run_scenario | 92-105 | REAL | — | kind dispatch |
| fixture_plans | 114-126 | REAL | — | existence-checked, expectation-driven |
| ScenarioPlan::label | 136-142 | REAL | S4 | vestigial allow (F-004) |
| expectation_for | 150-168 | REAL | — | unit-tested mapping, refuses non-generated paths |
| RequestIds::new/allocate | 179-192 | REAL | — | saturating, offset from host allocator |
| open_request/play_request/close_request | 195-223 | REAL | — | wire builders |
| seek_request | 228-246 | REAL | — | absolute/relative/percent rotation |
| mid_media_targets | 250-253 | REAL | S4 | duplicate of scenario.rs:358 (F-002) |
| dispatch | 260-266 | REAL | — | harness-break contextualization |
| in_playback | 269-274 | REAL | — | |
| ScenarioCounts (struct) | 277-304 | REAL | — | report axis |
| ScenarioRun::new/fail/note | 315-336 | REAL | — | record-not-throw discipline |
| DurationClock::new/running/maybe_end_warmup | 349-384 | REAL | — | warmup min 1 s max half; Idle-sighting rule |
| finish_duration_scenario | 389-507 | REAL | — | degenerate window = failure (line 470-476); endpoint forced-sample closes the all-warmup hole |
| run_play_loop | 511-550 | REAL | — | |
| fold_sink_bytes | 557-561 | REAL | — | double-count guards documented at 741-744/951-957 |
| play_loop_cycle | 565-621 | REAL | S3 | pump_until truth swallowed (F-001); dead `warmup` (F-003) |
| run_playlist_cycle | 625-677 | REAL | — | zero-pass liveness check |
| run_seek_storm | 683-846 | REAL | S3 | F-001 at 787-791; skip-rate liveness; last_seek_completed None → fail-fast |
| expected_samples | 851-853 | REAL | — | sample_ms range-checked [1,600000] in manifest.rs:547 → no div-by-zero |
| run_count_scenario | 877-1106 | REAL | — | bounded instrument (stride), baseline/span arithmetic unit-pinned; dead live_before (F-003) |
| count_warmup_cycles | 1110-1113 | REAL | — | unit-pinned clamps |
| count_baseline_and_span | 1124-1134 | REAL | — | unit-pinned against committed evidence |
| count_sample_stride | 1142-1144 | REAL | — | bounds instrument memory |
| pump_until_frames | 1148-1155 | REAL | — | covered by later decoded==0 check |
| read_fd_count (linux) | 1170-1174 | REAL | — | /proc/self/fd |
| read_fd_count (other) | 1184-1186 | REAL | — | honest None, reported as note |
| SlopeVerdict / check_slope | 1195-1225 | REAL | — | pure, unit-tested |
| test slope_verdict_flags... | 1246-1276 | REAL | — | exact arithmetic pinned |
| test count_warmup_cycles_clamps | 1280-1304 | REAL | — | |
| test count_sample_stride_bounds... | 1311-1343 | REAL | — | regression-pinned |
| test count_baseline_and_span... | 1352-1396 | REAL | — | recomputes committed raw evidence |
| test expectation_paths_map... | 1399-1413 | REAL | — | |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| soak_scenario_suite (main.rs) | YES | asserts collected failures empty; baseline-verified 39.22 s real-engine run |
| slope_verdict_flags_budget_violations | YES | exact 0.36 MB/h value assertions |
| count_warmup_cycles_clamps | YES | exact eq on 5 shapes |
| count_sample_stride_bounds_the_instrument_memory | YES | exact stride values incl. 2M → 489 |
| count_baseline_and_span_matches_the_measured_tags | YES | recomputes 3 committed series shapes exactly |
| expectation_paths_map_generated_areas | YES | exact paths + 3 refusals |
| (runner's own liveness oracle) | PARTIAL | drain-stall escape (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "exercise the real engine playback path (access → AVI demux → MJPEG/PCM decode → sink) ... not a mock" | runner.rs:9-12 | REAL | dispatch.rs:207 `MjpegDecoder::new()`, :218 `PcmDecoder::with_format`, :59 `AviDemuxer`; baseline run |
| "Violations are recorded, not thrown ... the test asserts the failure list is empty" | runner.rs:38-43 | REAL | main.rs:237-244 assert; ScenarioRun::fail |
| "a hang is a failed test, never a timeout mystery" (PLAY_TO_END_POLLS) | runner.rs:67-70 | SHALLOW | contract violated at both call sites — the bound exists but the timeout outcome is swallowed (F-001) |
| "warmup, then measure ... min one unit, max half" | runner.rs:29-30 | REAL | DurationClock::new:356-360, count_warmup_cycles, unit tests |
| "soak" (CI-short profile ~1 min) | tests/soak/scenarios.toml:35-39 | REAL (honest) | 0.25/0.2 min + 30/20 cycles; 4 h tier exists in soak-nightly.yml but is self-labeled "NOT PROVEN" (line 160) |

## 7. Dead / duplicated / unreachable code
- `mid_media_targets` duplicated (runner.rs:250 vs scenario.rs:358) — F-002.
- `let _ = warmup` / `let _ = live_before` dead bindings — F-003.
- Nothing unreachable: the whole suite runs in the default lane (baseline).

## 8. Dependencies & flags
Test-only; `#[path = "../soak/..."]` includes share dispatch/memory/scenario verbatim. `read_fd_count` is `#[cfg(target_os = "linux")]`-split with an honest None lane. No feature gates. Counting allocator is process-global (`#[global_allocator]` in soak_scenarios/main.rs:74-75).

## 9. Open questions for the next agent
- scripts/soak-scenarios.sh (wrapper) is not in my assignment — verify its exit-code contract actually fails CI on violations.
- The 4 h nightly profile has never been verified as green (workflow self-admits "NOT PROVEN") — no evidence in the repo of a completed 4 h run; treat any doc claiming "4 h soak passes" as UNVERIFIED.
- `AWAIT_POLLS` (dispatch.rs:79) is 2048 polls of a release-unbounded engine loop — in debug builds each poll is cheap, but if the engine ever blocks in poll, the "bounded" await is a wall-clock unbounded spin; acceptable for now, worth noting.
