# AUDIT: crates/video-engine/src/scheduler.rs#part1 (lines 1–570)
| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-21 |
| Lines / functions | 570 audited (1-570; chunk boundary cuts `PresentationScheduler::new` whose body completes at :576-577 — constructor covered for coherence; tick algorithm at :701+ is part2/A01) / 24 items in range |
| File verdict | REAL (chunk) |
| Reachable from shipped entry point? | YES. `lumen` CLI: `apps/cli/src/pipeline_host.rs:84` imports `PresentationScheduler` → `:704` field `scheduler: PresentationScheduler` → ticked in lockstep with the audio quanta (:41-42, :761). Executed: AVI first-vertical lane (INDEX.md: avi-firstvertical-open-session.txt full lifecycle; decode-bench `v_frames=60 v_decoded=60`) and baseline tests (orchrun log :5915-5975: video_engine 48/48 unit, 21/21 scheduling, 5/5 zero-alloc — all pass). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `sed -n '1,290p'/'291,600p'` (chunk read) · `rg -n 'PresentationScheduler' crates apps -l` (11 consumers: apps/cli pipeline_host + apps/desktop tests + video-engine tests/sink/policy/lib + renderer-wgpu presenter_seam) · `rg -n 'DEFAULT_MAX_PENDING' policy.rs` (:174 = 16 — verifies MAX_DROP_TRAIL sizing claim) · `rg -n 'pub fn tick'` (:701 — algorithm in part2) · `rg -n 'DropTrail\|recorded\(\)'` over test module (:1435-1509 — truncation tests exist, ran in baseline) · `sed -n '5915,6010p' orchrun…log` (video_engine test outcomes) |

## 1. One-paragraph verdict
Part 1 is the scheduler's vocabulary and state layer: the `PushOutcome` refusal taxonomy (every refusal hands the frame back — no silent swallowing, enforced by `#[must_use]`), the fixed-capacity `DropTrail` (inline `[Timestamp; 16]`, count-exact/ptrail-truncating under storms, D-1 zero-heap), `TickOutcome`/`SyncSample` observability records, 19 lifetime `SchedulerStats` counters, and the A/V-sync `SchedulerSync` runtime state with tumbling drop/rate budget windows. I traced every invariant the docs claim: `len()` is the min of recorded and the cap, `record()` writes only in-bounds, `truncated()` is saturating, `PartialEq` compares exactly the observable state, the trail sizing claim (`DEFAULT_MAX_PENDING` = 16 = `MAX_DROP_TRAIL`) is verified against policy.rs, and the tumbling-window rotation math (`saturating_sub` against window opens) is correct. The only defect is a single non-saturating counter increment (`recorded += 1`) in a codebase whose stated discipline is saturating arithmetic everywhere — pedantic, but real. Types are reachable from the shipped CLI and executed by committed baseline runs.

## 2. What actually works (verified)
— `DropTrail` (:194-286): traced all five accessors + `record`; the retained-prefix/truncated-count split is correct (`len = min(recorded, 16)`, `truncated = recorded.saturating_sub(16)`, `record` writes at index `len()` only when `< 16`). Truncation behavior pinned by tests at :1435-1509 (`recorded() == 40, truncated` exact) — executed, 48/48 unit pass (orchrun log :5919).
— `MAX_DROP_TRAIL` sizing claim (:170-171): `crate::policy::DEFAULT_MAX_PENDING = 16` confirmed at policy.rs:174 — the doc's "sized to cover the default reorder window exactly" is true.
— `SchedulerSync::rotate_windows` (:526-539): both tumbling windows use `saturating_sub` + `>=` on tick counts — no underflow, no off-by-one (window of N ticks resets on the Nth tick boundary); `note_late_drop` rotates before counting, so a drop can never land in a stale window.
— `PushOutcome` design (:94-148): every refusal variant carries the untouched `VideoFrame` back and the enum is `#[must_use]` with a message saying why — the "never silently swallows fresh data" claim (:90-92) is structural, not aspirational.
— D-1 zero-allocation claim for the tick path's drop records: `tests/zero_alloc_scheduler.rs` ran in baseline — 5 passed / 0 failed (orchrun log :5925 area) — inline arrays, no heap, verified by execution.
— End-to-end execution: AVI first-vertical (MJPEG+PCM) plays 60/60 frames through this scheduler (INDEX.md §"contrast: AVI first-vertical" + mp4-smoke-decode-bench on the avi fixture).

## 3. Findings

### F-001 [S4]
Where: crates/video-engine/src/scheduler.rs:246-252
Code:
```
    fn record(&mut self, pts: Timestamp) {
        let retained = self.len();
        if retained < MAX_DROP_TRAIL {
            self.pts[retained] = pts;
        }
        self.recorded += 1;
    }
```
What is wrong: `self.recorded += 1` is a plain, non-saturating increment on the one counter in this file the rest of the codebase would write as `saturating_add(1)`. Every other counter in the same chunk (`SchedulerStats` fields are updated in part2 with saturating ops; `DropTrail::truncated` itself saturates at :228; `SchedulerSync::note_late_drop` saturates at :521) follows the saturation discipline; this one overflows (panic in debug, wrap in release) at 2^64 records.
Why it matters: Practically unreachable (a u64 of drop records needs 18 quintillion ticks), so S4 — but the inconsistency is exactly the kind of drift the workspace's own D-1/D-2 directives exist to prevent, and in a `--release` build a wrapped `recorded` would corrupt the observable drop accounting (`len()`/`truncated()` silently reset) with no detection.
What correct looks like: `self.recorded = self.recorded.saturating_add(1);`
Confidence: HIGH — Verification: read; sibling counters compared.

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (algorithm contract) | 1-71 | REAL | — | Four-step algorithm + pre-roll hold spec; implementation at tick() :701 (part2, A01) — claims cross-referenced, not verified here (boundary noted). |
| `PushOutcome` enum (7 variants) | 94-148 | REAL | — | Frame-back refusals; `#[must_use]`; counted in SchedulerStats fields that exist (:390-398). |
| `PresentAction` enum | 151-165 | REAL | — | Present/RepeatLast/Background — matches doc 03 §4.10 e vocabulary. |
| `MAX_DROP_TRAIL` | 178 | REAL | — | 16; sizing claim verified vs policy.rs:174. |
| `DropTrail` struct | 193-202 | REAL | — | Inline `[Timestamp; 16]` + u64 count; Copy. |
| `DropTrail::len` | 208-210 | REAL | — | `min(recorded, cap) as usize` — correct. |
| `DropTrail::is_empty` | 213-216 | REAL | — | `recorded == 0` — consistent with `len`. |
| `DropTrail::recorded` | 219-222 | REAL | — | Exact count accessor. |
| `DropTrail::truncated` | 226-229 | REAL | — | Saturating sub — correct. |
| `DropTrail::as_slice` | 232-235 | REAL | — | `&self.pts[..self.len()]` — in-bounds by construction. |
| `DropTrail::iter` | 238-241 | REAL | — | Delegates to as_slice. |
| `DropTrail::record` | 246-252 | REAL | S4 | F-001 (non-saturating increment); write is bounds-checked. |
| `Default for DropTrail` | 255-264 | REAL | — | Zero-fill on NANO_BASE; never observable past len. |
| `Debug for DropTrail` | 266-275 | REAL | — | Prints only retained prefix — honest. |
| `PartialEq/Eq for DropTrail` | 277-286 | REAL | — | Compares count + retained prefix only — fill slots excluded, correct. |
| `TickDrops` | 295-310 | REAL | — | Two trails + stale u64; Copy/Default. |
| `TickOutcome` | 315-328 | REAL | — | action + next_due + drops + sync — consumed by pipeline_host. |
| `SyncSample` + Default | 336-367 | REAL | — | Option-wrapped error/EMA (None when not sampled — "discarded, not smeared" matches the paused-clock doc :46-50); band + snapped + late threshold. |
| `SchedulerStats` (19 fields) | 371-416 | REAL | — | i128 for `sync_error_sum` (documented overflow rationale); all fields written in part2's tick path (fields exist and are read by tests — :1509 asserts `drops.late.recorded() == 40`). |
| `Pending` | 418-422 | REAL | — | pts + frame; window sorted-by-pts invariant is part2's to maintain. |
| `PresentationScheduler` struct | 426-448 | REAL | — | Bounded window (cfg.max_pending), single held-missing slot, last/repeat state, fixed-size sync state — no unbounded field. |
| `SchedulerSync` struct | 454-485 | REAL | — | All fixed-size; zero allocation claim consistent with zero-alloc test passing. |
| `SchedulerSync::new` | 488-503 | REAL | — | Bands from cfg — policy-derived. |
| `SchedulerSync::reset_estimators` | 508-515 | REAL | — | Resets estimator/bands/actuators; window-start fields intentionally kept (harmless: counters are zeroed, next rotation re-keys) — traced. |
| `SchedulerSync::note_late_drop` | 519-522 | REAL | — | Rotate-then-count; saturating. |
| `SchedulerSync::rotate_windows` | 526-539 | REAL | — | Tumbling logic traced correct (see §2). |
| `Debug for PresentationScheduler` | 542-556 | REAL | — | Honest surface (clock handle excluded, documented why). |
| `PresentationScheduler::new` | 563-577 | REAL | — | Generation from clock snapshot (engine mints before wiring — matches playback-engine wiring); body completes 4 lines past the nominal chunk edge (570) — boundary noted. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| DropTrail truncation tests (:1435-1509, part3 module) | YES | Assert `recorded()==40` with `truncated()` exact and retained prefix contents — kills count-only, wrong-prefix, and off-by-one cap mutations. Executed: 48/48 unit pass (orchrun :5919). |
| `tests/scheduling.rs` (21 tests) | YES (for the types) | Integration-level scheduling behavior; 21/21 pass (orchrun :5923). |
| `tests/zero_alloc_scheduler.rs` (5 tests) | YES | Would fail if the tick path allocated (DropTrail's inline-array claim is part of that path); 5/5 pass (orchrun :5927). |
| `apps/cli/tests/sync_scenarios.rs` | YES | End-to-end sync scenarios through the shipped host; passed in baseline. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Sized to cover the default reorder window exactly (DEFAULT_MAX_PENDING = 16)" | scheduler.rs:170-171 | REAL | policy.rs:174 `pub const DEFAULT_MAX_PENDING: usize = 16;` — verified by rg. |
| "a tick allocates nothing… storm beyond MAX_DROP_TRAIL truncates the trail while every count stays exact" | scheduler.rs:292-294 | REAL | DropTrail impl traced; zero-alloc test suite executed in baseline (5/5). |
| "the scheduler never silently swallows fresh data" | scheduler.rs:90-92 | REAL | Every refusal variant returns the frame; `#[must_use]` on the enum. |
| Four-step tick algorithm + pre-roll hold | scheduler.rs:14-71 | REAL (definitions) / boundary | Implementation in part2 `tick()` :701 — A01's chunk to verify; execution evidence (AVI 60/60) exists regardless. |

## 7. Dead / duplicated / unreachable code
None in range. All public types are consumed (`rg -l` consumers: apps/cli pipeline_host, apps/desktop tests, video-engine tests, renderer-wgpu presenter_seam).

## 8. Dependencies & flags
Depends on media-clock (`PlaybackClock`, `ClockState`, `NANO_BASE`, `sync::{DriftEstimator, SyncBand, SyncBands}`), media-types (`VideoFrame`, `GenerationId`, time types), crate::policy (`SchedulerCfg`), crate::sink (`PresentRequest`) — all declared and used. No feature gates. `Arc<dyn PlaybackClock>` — clock injected, single-threaded v0 discipline documented (:6-12).

## 9. Open questions for the next agent
- Part2 (A01): verify `tick()` (:701+) actually implements the module doc's binding order (generation check → late purge with pre-roll exemption → largest-pts-≤-now+lead selection → repeat/background) and maintains the window's PTS-sorted invariant claimed at :432-433.
- Part2: `stats` counter increments — confirm they are saturating (F-001's discipline concern applies to the whole file).
