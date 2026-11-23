# AUDIT: crates/video-engine/tests/common/mod.rs
| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-23 |
| Lines / functions | 273 / 22 (0 `#[test]` — fixture module) |
| File verdict | REAL (with dead fixture machinery hidden by `#![allow(dead_code)]`) |
| Reachable from shipped entry point? | Test lane. Code under test is shipped: apps/cli/src/pipeline_host.rs:84-86 `use video_engine::scheduler::{...}`, `use video_engine::sink::{PresentRequest, Presenter}`, `use video_engine::{EnqueueOutcome, PresentationQueue, SchedulerCfg}` — the CLI host's presentation path. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | Disk full — no cargo runs. Evidence: baseline cargo-test-workspace.txt:6451-6563 video-engine section: scheduling.rs 21 passed, sync_scenarios.rs 5 passed, zero_alloc_scheduler.rs 1 passed (all suites compile this module). Usage counts by rg across the three suites (see F-001). |

## 1. One-paragraph verdict
A competent, sleep-free fixture module: virtual monotonic clock with `fetch_max` monotonic safety, production-shaped audio-master clock over a fake sample counter, and a recording presenter with an in-order assertion helper used by 8 call sites across the suites. The rot is under the blanket `#![allow(dead_code)]`: four helpers — the entire scripted-presenter-failure lane (`failing_at`, `presenter_failure`, and the failure branch of `present`) plus `frame_missing` and `records()` — are used by NO integration suite; they are dead duplicates of `src/test_support.rs` (whose copy IS exercised by src/queue.rs:370). The blanket allow hides exactly this. The `failing_at` doc also misdescribes sticky semantics it never gets called enough to reveal.

## 2. What actually works (verified)
— Nothing executed directly (disk full). Baseline: all three consuming suites green (21+5+1 passed) — the used helpers (`VirtualClock`, `AudioMasterClock`, `frame`, `RecordingPresenter::{new, slots_seen, gens_seen, assert_in_order}`, `set_ns`, `advance_ns`, `now_ns`, `advance_samples`, `set_generation`, `pause`, `resume`, `clock`) drive those 27 passing tests. Usage verified by rg per-suite counts.

## 3. Findings
F-001 [S4]
Where: crates/video-engine/tests/common/mod.rs:8, 185-194, 172-177, 223-228, 241-243
Code:
```
#![allow(dead_code)]
```
and
```
    /// A presenter that refuses the `fail_at`-th request (0-based).
    pub fn failing_at(fail_at: usize) -> RecordingPresenter {
```
What is wrong: the blanket allow hides dead fixtures. rg across the three consuming suites (scheduling.rs, sync_scenarios.rs, zero_alloc_scheduler.rs): `failing_at` 0 uses, `presenter_failure` 0 uses, `frame_missing` 0 uses, `records()` 0 uses. The scripted presenter-failure machinery in THIS file (including the `fail_at` branch at 262-264) is never executed by any test — it is an unused duplicate of src/test_support.rs:136-188, where the same machinery IS used (src/queue.rs:370 `RecordingPresenter::failing_at(1)`). So integration-level presenter-failure coverage of the shipped `Presenter` seam does not exist; only the unit-level copy covers it. What correct looks like: delete the dead helpers (or write the integration failure test they were built for) and drop the blanket allow for `#[cfg]`-narrow ignores. Confidence: HIGH — Verification: rg usage counts (command in log) + read.
F-002 [S4]
Where: crates/video-engine/tests/common/mod.rs:222-228 vs 261-268
Code:
```
    fn present(&mut self, request: &PresentRequest) -> Result<PresentInfo, EngineError> {
        if self.fail_at == Some(self.log.len()) {
            return Err(presenter_failure());
        }
        self.log.push(PresentRecord {
```
What is wrong: `failing_at(n)` does not merely "refuse the fail_at-th request" as documented — the failed request is never pushed to `log`, so `log.len()` stays at `n` and EVERY subsequent present also refuses: it is a sticky from-n-on failure. Latent everywhere: the only user of either copy (src/queue.rs:370) aborts at the first refusal via `drain_to`'s `Err`, so stickiness is never observed. Same shape in src/test_support.rs:182-187. A future test written against the doc wording ("the Nth fails, N+1 succeeds") would silently test the wrong thing and pass only if it never presents past N. Confidence: HIGH — Verification: read + traced both copies.
F-003 [S4]
Where: crates/video-engine/tests/common/mod.rs:1-6 vs crates/video-engine/src/test_support.rs
Code:
```
//! Shared fixtures for the video-engine integration suites (card
//! 2.C4 AC). Integration tests cannot see the crate's cfg(test)
//! `test_support`, so this module carries the production-shaped
//! fakes: virtual clocks over media-clock's injected-truth traits
```
What is wrong: structural duplication forced by Rust's test-crate visibility (documented, legitimate) — but the copies have already drifted in content (this file adds `PresentRecord` struct + `records()`; src's records tuples) and the dead parts (F-001) make the drift worse. Acceptable as-is; flag for the duplication ledger. Confidence: HIGH — Verification: read both files.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FakeMonotonic + MonotonicSource | 27-33 | REAL | — | used via VirtualClock |
| VirtualClock::new/clock | 44-60 | REAL | — | MediaClock::system_monotonic over settable source |
| VirtualClock::set_ns | 62-67 | REAL | — | `fetch_max` — monotonic-safe by construction |
| VirtualClock::advance_ns / now_ns | 69-77 | REAL | — | |
| VirtualClock::set_generation / pause / resume | 79-93 | REAL | — | used by scheduling.rs |
| FakeCounter + SampleCounterSource | 106-119 | REAL | — | 48 kHz |
| AudioMasterClock::new/clock | 121-140 | REAL | — | production shape (audio master) |
| AudioMasterClock::advance_samples / set_generation | 142-150 | REAL | — | used by scheduling.rs |
| frame | 159-169 | REAL | — | 2×2 NV12, nano base — used by all 3 suites |
| frame_missing | 171-177 | DEAD | S4 | 0 uses (F-001) |
| presenter_failure | 185-194 | DEAD (here) | S4 | live twin in src/test_support.rs:136 |
| PresentRecord | 198-204 | REAL | — | consumed by slots_seen/gens_seen |
| RecordingPresenter::new | 214-220 | REAL | — | |
| RecordingPresenter::failing_at | 222-228 | DEAD + doc bug | S4 | F-001 + F-002 |
| slots_seen / gens_seen | 230-238 | REAL | — | scheduling.rs |
| records | 240-243 | DEAD | S4 | 0 uses |
| assert_in_order | 245-257 | REAL | — | 8 call sites; enforces doc 03 §4.10(f) |
| Presenter::present | 260-273 | REAL | — | records `when.ticks` + generation; failure branch dead here |

## 5. Test honesty
No tests in this file (fixture). The fixtures feed 27 baseline-green tests; `assert_in_order` is a real property assertion applied to every recorded run (would fail on any out-of-order present).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no sleeps, ever" (doc 03 §8 discipline) | module docs 4-5 | REAL | whole file is clock/counter injection; the consuming suites' only Instant/Duration use would be in their own bodies (not verified — their files, not mine) |
| "production-shaped fakes" | module docs 3-4 | REAL | VirtualClock = system-monotonic master; AudioMasterClock = audio_master over SampleCounterSource — both real media-clock constructors |
| "refuses the `fail_at`-th request (0-based)" | 222 | FALSE as worded (sticky from N on) | F-002; never observable at present (dead here; src user aborts at first Err) |

## 7. Dead / duplicated / unreachable code
Dead in this file: `failing_at`, `presenter_failure` (this copy), `frame_missing`, `records()` — all hidden by `#![allow(dead_code)]` (F-001). Duplication: forced twin of src/test_support.rs (F-003).

## 8. Dependencies & flags
media_clock (MediaClock, MonotonicSource, SampleCounterSource, PlaybackClock, NANO_BASE), media_types, testkit::Must, video_engine (Presenter/PresentRequest/PresentInfo). One blanket `#![allow(dead_code)]` (F-001). No unsafe, no feature gates.

## 9. Open questions for the next agent
- scheduling.rs / sync_scenarios.rs / zero_alloc_scheduler.rs owners: do those suites need a presenter-failure integration test (the dead machinery suggests one was planned)? If not, prune.
- src/test_support.rs (not my file): same sticky `failing_at` semantics (182-187) — if any future unit test presents past the scripted failure, it will hit the same doc-vs-behavior surprise.
