# AUDIT: crates/video-engine/src/policy.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 747 / 13 production fns (incl. 2 Display/Default impls) + 12 test fns + 5 types |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — CLI: run.rs open → host.rs:890 `open_session` → pipeline_host.rs:936 `CliFrameConsumer::new` → pipeline_host.rs:1300 `SchedulerCfg::for_frame_duration` → :1324 `PresentationScheduler::new(cfg, clock)` → `cfg.decide(...)` per tick. Desktop adopts the same host (apps/desktop/src/shell.rs). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | rg for `SchedulerCfg|PresentationScheduler|SyncPolicy|sync\.` across apps/ and video-engine/src; sed verification of all quotes. No cargo (adaptation); test execution UNVERIFIED, suite part of W00's 1751-passing baseline. |

## 1. One-paragraph verdict

This is a pure, dependency-free classifier module (PTS vs clock position → Present / WaitUntil / SkipLate) plus the configuration vocabulary for the presentation scheduler, and it is the honest kind: i128 arithmetic at every boundary, typed config errors instead of panics, saturating `due_at`, and an exact-boundary grid test that pins the decision table (`pts == now − threshold` presents, `pts == now + lead` presents, one nanosecond past either flips). Every `SyncPolicy` knob is consumed by `scheduler.rs` (verified: drop_window_ticks scheduler.rs:528, threshold_window_ticks:534, decide_every:877, repeat_extension:922/977, drop_budget:952, widened_threshold:958, threshold_change_limit:967) — no declared-but-never-read config here. The three findings are a truncating `as u64` fallback on an absurd-input path and two hygiene items. The doc-vs-code decision order (late → due → wait) matches the module doc's binding spec (lines 19-28) and the tests prove it.

## 2. What actually works (verified)

- `decide` boundary semantics (policy.rs:271-288): verified against the grid test at 495-523 — the 12-case table exercises far-late, bound−1, ON-bound, bound+1, now, now+lead, now+lead+1, and both i64 extremes; these would fail under any off-by-one in the comparisons or an i64 (non-i128) intermediate.
- Typed config validation (206-231): `Auto` without frame duration and zero duration are rejected with data, not panics; both error paths have dedicated tests (645-678).
- Saturation discipline: `due_at` (296-300) clamps through i128 into the i64 tick range and is tested at `i64::MIN` (610).
- Consumption of every SyncPolicy field by scheduler.rs (grep list above) — the ladder is not paper config.
- Const guard `DEFAULT_MAX_PENDING_NZ` (186-190): the `loop {}` arm is a compile-time dead end — if the literal 16 were zeroed, const evaluation fails the build. The scoped `#[allow(clippy::empty_loop)]` is documented with a reason (clippy's suggested fix is `panic!()`, denied by workspace lints). Legitimate.

## 3. Findings

### F-001 [S3] Truncating `as u64` fallback in `widened_threshold` on absurd frame durations
Where: crates/video-engine/src/policy.rs:455
Code:
```rust
        let width = doubled.clamp(i128::from(duration.0), cap);
        let ns = u64::try_from(width).unwrap_or(cap.max(0) as u64);
        Some(Nanoseconds(ns))
```
What is wrong: `cap = duration.0 × widen_cap_frames` is an i128 that can exceed `u64::MAX` (duration > ~2^64/3 ns ≈ 193 years/frame with the default cap 3). In that case `width` can also exceed `u64::MAX`, `u64::try_from` fails, and the fallback `cap.max(0) as u64` **wraps modulo 2^64** — producing an effectively arbitrary late threshold instead of the intended cap. The comment two lines above says "extreme inputs clamp to the cap"; the code does not do that in this arm.
Why it matters: `frame_duration` is file-controlled (pipeline_host.rs:1296-1298 reads `fact.frame_ns` straight from demuxer track facts, with no upper bound before `SchedulerCfg::for_frame_duration`). A crafted container claiming a frame period of centuries yields a wrapped threshold → the late-drop policy misbehaves (either drops everything or nothing) on that file. No memory-safety impact; behavior defect on malicious/absurd input.
What correct looks like: `let ns = u64::try_from(width).unwrap_or(u64::MAX);` (or clamp `width` to `u64::MAX` as i128 before converting) — saturation, not wrap, matching the module's own stated discipline.
Confidence: HIGH on the code path (read), MEDIUM on end-to-end reachability (requires a demuxer that emits such a frame_ns — W13/W4 verify the bound upstream; marked UNVERIFIED).
### F-002 [S4] `extreme_positions_never_overflow` sweep portion asserts only "does not panic"
Where: crates/video-engine/src/policy.rs:600-604
Code:
```rust
        for now in [i64::MIN, -1, 0, 1, i64::MAX] {
            for pts in [i64::MIN, -1, 0, i64::MAX] {
                let _ = c.decide(ns(pts), ns(now));
            }
        }
```
What is wrong: The 25-pair sweep discards the decision; only the three asserts after it (606-620) check outcomes. A regression that returned `Present` for `i64::MIN` pts under a `u64::MAX` threshold would not be caught by the sweep itself.
Why it matters: The test name promises overflow-safety classification; the sweep proves only absence of panic. Weakens the paperwork for an invariant the doc calls out (§4.7 f saturation discipline).
What correct looks like: Assert the expected `FrameDecision` for each extreme pair (at minimum: late-bound comparisons at `i64::MIN`).
Confidence: HIGH — Verification: read.
### F-003 [S4] `hold_band` truncation makes the "H = target_lead" invariant off-by-one for odd durations
Where: crates/video-engine/src/policy.rs:417 and 247
Code (417):
```rust
            hold_band: Nanoseconds(duration.0 / 2),
```
Code (247):
```rust
            Nanoseconds(duration.0 / 2),
```
What is wrong: For an odd `frame_duration`, `target_lead = duration/2` and `hold_band = duration/2` are both floored, so the doc-invariant "H equals the default target_lead" (line 349-350, asserted at 703-704) holds, but the *stated* semantic "½ frame duration" is floor(duration/2) — a 1 ns discrepancy from the real half. Self-consistent (the test asserts equality of the two floors), so no behavior bug; the naming/doc claim "½ frame" is approximate.
Why it matters: Hygiene only; nanosecond-level pedantry that the module otherwise takes seriously (its grid tests flip on 1 ns).
What correct looks like: Document "floor(½ frame)" in the field docs, or round half-up consistently.
Confidence: HIGH — Verification: read.

## 4. Per-function table (production code)

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `DropPolicy::late_threshold` | 73-79 | REAL | — | Exhaustive 3-arm match; Auto→duration, None→never. Tested (626-640). |
| `Display for SchedulerCfgError::fmt` | 121-132 | REAL | — | Messages contain the searchable tokens the tests assert (658-659, 677). |
| `SchedulerCfg::new` | 206-231 | REAL | — | Rejects Auto-without-duration and zero duration; derives SyncPolicy. |
| `SchedulerCfg::for_frame_duration` | 241-251 | REAL | — | Doc defaults; reachable from pipeline_host.rs:1300 with file-controlled duration. |
| `SchedulerCfg::late_threshold` | 255-257 | REAL | — | Delegates. |
| `SchedulerCfg::decide` | 271-288 | REAL | — | i128 math; strict-late/inclusive-due order; grid-tested to the nanosecond. |
| `SchedulerCfg::due_at` | 296-300 | REAL | — | Saturating clamp; tested at i64::MIN. |
| `SchedulerCfg::sync` | 305-307 | REAL | — | Accessor. |
| `SchedulerCfg::with_sync` | 313-316 | REAL | — | Builder; no callers outside tests (the "session wiring" the doc mentions does not exist — see §7). |
| `SyncPolicy::for_frame_duration` | 415-428 | REAL | — | All constants consumed by scheduler.rs (grep proof). |
| `SyncPolicy::bands` | 434-436 | REAL | — | Constructs SyncBands; used scheduler.rs:491, 510. |
| `SyncPolicy::widened_threshold` | 443-457 | REAL | S3 | F-001 truncating fallback. Clamp logic itself correct and tested (726-746). |
| `Default for SyncPolicy` | 460-464 | REAL | — | 30 fps fallback constant, documented. |

Consts: `DEFAULT_MAX_PENDING` (174), `DEFAULT_MAX_PENDING_NZ` (186-190, compile-time guard, legit), `DEFAULT_MAX_REPEAT` (196), `DEFAULT_SYNC_FRAME_NS` (404).

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| grid_late_policy_decisions_are_exact (495-523) | YES | 12-case exact-boundary table incl. ±1 ns flips and i64 extremes — kills off-by-ones. |
| grid_never_policy_never_skips (526-539) | YES | Never-drop invariant + exact due at zero lead. |
| zero_threshold_drops_anything_not_yet_on_time (542-551) | YES | Zero-threshold boundary triple. |
| drops_are_monotone_in_lateness (554-578) | YES | Real property sweep (step 7 grid) with unreachability on violation. |
| waits_are_monotone_in_earliness (581-592) | YES | due_at exactness across a grid. |
| extreme_positions_never_overflow (595-621) | PARTIAL | Sweep portion is panic-only (F-002); the three explicit asserts are real. |
| policy_threshold_resolution (626-640) | YES | All three policy arms. |
| auto_without_duration_is_a_typed_config_error (645-660) | YES | Error identity + message. |
| zero_duration_is_a_typed_config_error (663-678) | YES | Error identity + message. |
| for_frame_duration_applies_the_doc_defaults (681-694) | YES | Field-by-field default pinning. |
| sync_policy_defaults_follow_the_frame_duration (699-723) | YES | All 10 knobs pinned; with_sync replacement checked. |
| widened_threshold_clamps_between_one_and_three_frames (726-746) | YES | 1.4-frame and cap cases, i64::MIN input, None case. Does NOT cover the >u64::MAX cap arm (F-001 untested). |

Strongest test suite of the scheduling cluster. No mocks-of-subject, no sleeps, no ignored tests.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "assert exact present/drop/repeat decisions for every policy parameter combination, exhaustive over small grids" (doc 03 §4.10 f, quoted lines 7-9) | module doc | REAL | Grid tests 495-551 pin exact decisions at every boundary. |
| "late wins" when a frame is both late and due (lines 21-23) | module doc | REAL | decide() checks late first (274-280); grid case (0, SkipLate) confirms. |
| "extreme inputs clamp to the cap" (line 451) | widened_threshold comment | SHALLOW | True for i64-scale inputs; false for durations > ~6.1e18 ns where the fallback wraps (F-001). |
| "Audio is never dropped for video's sake" (line 38, quoting doc 03 §4.10 e) | module doc | UNVERIFIED | This module governs video only by construction (no audio types present); whether audio is ever dropped is decided in audio-engine (W07). |
| Ladder defaults "all W1-18 [PROPOSAL] numbers" (409-413) | SyncPolicy docs | REAL | Constants present, consumed by scheduler.rs; the [PROPOSAL] tagging is honest. |

## 7. Dead / duplicated / unreachable code

- `SchedulerCfg::with_sync` (313-316): reachable only from the in-file test (714-722). The doc-comment's "session wiring may override from its QueuePlan" scenario does not exist in any shipped app (rg `with_sync` over apps/: zero hits). Not dead API (public, plausible), but currently unused in production — S4-level note, folded into this section rather than a numbered finding.
- No duplicated logic; no dead branches.

## 8. Dependencies & flags

- No feature flags; no `unsafe`; deps = media-types + media-clock only (Cargo.toml, std-only by C5). dev-dep testkit dev-only. Verified: `use` list at 30-34.

## 9. Open questions for the next agent

- W13 (pipeline_host.rs:1296): is `fact.frame_ns` bounded anywhere upstream (demuxer track rate sanity)? F-001's reachability depends on it.
- W03 (whoever has scheduler.rs): does `TickOutcome`/`PresentAction` honor `WaitUntil.due_at` (the doc's timer-driver contract), or busy-poll?
