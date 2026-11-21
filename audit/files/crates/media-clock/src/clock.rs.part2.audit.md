# AUDIT: crates/media-clock/src/clock.rs#part2 (lines 585-1178)
| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-21 |
| Lines / functions | 594 (chunk of 1179) / 11 prod items + 4 helpers/fakes + 18 tests |
| Chunk boundaries | part2 ≈ 585-1178 (assignments split 589/589): `MediaClock::anchor` tail, lock/snapshot helpers, `anchor_core`, `adopt_anchor`, `read_source`, `adopt_if_awaiting`, `now_locked`, `try_now_locked`, `impl PlaybackClock for MediaClock`, the whole test module. Part 1 (types, trait, Source/Core/extrapolate, constructors, pause/resume) = another agent. |
| File verdict (chunk) | REAL |
| Reachable from shipped entry point? | YES. `lumen` CLI → playback-engine → media-clock: playback-engine/src/seek.rs:442 `clock.reanchor(...)`, tracks.rs:415 `clock.reanchor(...)`, session.rs:1655 `self.runtime.clock.resume()`; constructors used across playback-engine/audio-output (audio-output tests cite `MediaClock`, `PlaybackClock` imports). Baseline execution: all 18 chunk tests passed — orchrun-test-workspace-nofailfast.log.txt:3204+ (`media_clock` lib `92 passed`, `clock::tests::*` all ok). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | sed -n quotes (verified) · Read 1-580 (part-1 context: Core/extrapolate/checked_div_round/pause/resume) to verify part-2 behavior claims · rg `awaiting_anchor` (set at backends.rs:908, 993 — switch_locked + seek_back) · rg `\.reanchor\(` (playback-engine/src/seek.rs:442, tracks.rs:415 — real callers) · baseline orchrun log grep `clock::tests` (18/18 ok) · hand-simulation of extrapolate/now_locked/checked_div_round over the 18 scenarios |

## 1. One-paragraph verdict
The clock core is genuinely engineered: extrapolation is checked i128 with
saturation-into-i64 positions, the regression baseline compares against the
last sound reading (not the anchor), rate changes re-anchor without jumps,
pause/resume semantics are correct against a source that keeps running, and
the 18 unit tests drive the real `MediaClock` through injected fake sources
(the crate's own doctrine), all passing in the no-fail-fast baseline. The one
real defect is an interaction hole: `PlaybackClock::reanchor` (and
`MediaClock::resume`) adopt a usable reading without clearing the
`awaiting_anchor` latch set by master switches / `seek_back`, so the next read
re-adopts and silently swallows the samples consumed in between. Plus two
hygiene nits: an introspection saturation distortion and one dead test
disjunct.

## 2. What actually works (verified)
— Anchor arithmetic incl. rate scaling: hand-checked `anchor_arithmetic_scales_by_rate` (4 counter-seconds × 5/4 → 5 media-seconds, exact through `checked_div_round`) against extrapolate (clock.rs:388-399); ran ok in baseline.
— Monotonicity clamp: `now_locked` (725-744) holds `last_ns` across Discontinuity and Paused arms; `counter_regression_holds_position_never_backwards` (955-982) proves hold-then-reanchor-cure end to end.
— Scale-change detection: `sample_rate_change_is_a_discontinuity` (985-1011) — reading ns (1.0884e9) is NOT below `last_source_ns` (1e9), so only the `rate_hz != anchor_rate_hz` arm (clock.rs:379) fires; the test pins exactly that arm.
— Overflow: `extreme_positions_saturate_monotonically` (1035-1051) — anchor i64::MAX−100 + 1e12 counter-ns extrapolates in i128, `now()` saturates to i64::MAX twice (monotone at the clamp), `try_now` → `TimeError::Overflow` (756-760). Hand-verified.
— `reanchor` atomicity-on-error: `position_nanos(media)?` runs before any mutation (789), so the overflow rejection leaves the clock untouched — asserted by `reanchor_overflow_leaves_clock_unchanged` (943-950).
— set_rate no-jump: `now_locked` re-anchor at clamped current position (814-815) — asserted by `set_rate_never_jumps_and_rescales` (1056-1068).
— Zero-rate source (broken device): `read_source` → Unusable (679-681) → hold + typed error, never a guess — `broken_source_is_unusable_not_panic` (1014-1030).
— All 18 chunk tests executed and passed: orchrun log (media_clock lib 92/92; every `clock::tests::` name listed ok).

## 3. Findings

### F-001 [S3] — `reanchor` adopts a usable reading without clearing `awaiting_anchor`: the next read re-adopts and silently swallows elapsed progress
Where: crates/media-clock/src/clock.rs:788-805 (and sibling site `MediaClock::resume`, 558-568, part 1) vs 706-720
Code:
```rust
    fn reanchor(&self, media: Timestamp) -> Result<(), TimeError> {
        let media_ns = position_nanos(media)?;
        let mut core = self.lock_core();
        let reading = read_source(&core.source);
        core.anchor_media_ns = media_ns;
        adopt_anchor(&mut core, &reading);
```
```rust
pub(crate) fn adopt_if_awaiting(core: &mut Core, reading: &Reading) {
    if core.awaiting_anchor
        && let Reading::Usable {
            ns,
            sample,
            rate_hz,
        } = reading
    {
        core.anchor_source_ns = *ns;
```
What is wrong: `switch_locked` (backends.rs:993) and `seek_back`
(backends.rs:908) set `awaiting_anchor = true` when the new/current source is
unreadable, deferring the anchor baseline to the **first usable reading**.
But when the source *is* readable at `reanchor` time, `reanchor` calls
`adopt_anchor` (which sets `anchor_source_ns`/`last_source_ns` to the current
reading R1) while leaving `awaiting_anchor == true`. The next
`now`/`try_now`/`snapshot` enters `adopt_if_awaiting`, which re-adopts the
then-current reading R2 as the source baseline — the R2−R1 interval of
consumed samples is never counted: the position silently holds at `media_ns`
instead of advancing by (R2−R1)×rate. `set_rate` is immune (it goes through
`now_locked`, which calls `adopt_if_awaiting` first); `MediaClock::resume`
(558-568) has the same hole as `reanchor`.
Why it matters: the reachable sequence — master loss / end-of-media lands on
an unreadable source (latch set), the source recovers, then the engine
seek-resumes: playback-engine really calls `clock.reanchor` (seek.rs:442,
tracks.rs:415) and `clock.resume()` (session.rs:1655) — silently drops up to
one read-interval of progress on the audio-master clock. No jump, no error,
no counter: pure unobservable drift on the A/V sync reference.
What correct looks like: `reanchor`/`resume` clear the latch when they adopt
(`core.awaiting_anchor = false;` next to `adopt_anchor`), or route both
through a shared helper that resolves the latch exactly once.
Confidence: MEDIUM — Verification: read + traced both latch-set sites and
both adopt sites; the swallowing derivation is by hand. UNVERIFIED: an
end-to-end run exhibiting the interleaving (needs a recovery scenario with a
late-readable source — no such test exists; REQUEST-ORCH-RUN not essential,
the code path is unambiguous).

### F-002 [S4] — `anchor()` introspection saturates monotonic anchors through i64
Where: crates/media-clock/src/clock.rs:588-593
Code:
```rust
        let (at_sample, at_wall_ns) = match &core.source {
            Source::Samples(_) => (Some(core.anchor_sample), None),
            Source::Monotonic { .. } => (
                None,
                Some(saturate_position(core.anchor_source_ns.max(0)) as u64),
            ),
```
What is wrong: `anchor_source_ns` for a monotonic master is the raw
`source.now_ns()` (u64, up to 1.8e19) stored in i128. The introspection path
clamps it to i64::MAX (9.2e18) and casts to u64 — a monotonic source past
~292 years of nanoseconds reports a wrong (clamped) `at_wall_ns` with no
indication. Positions elsewhere saturate by documented design; this
introspection field has no such documented saturation semantics.
Why it matters: `ClockAnchor` is the debugging surface for drift analysis; a
silently clamped anchor misleads exactly when the numbers get interesting.
Blast radius: introspection only; no control path reads `at_wall_ns`.
What correct looks like: widen `ClockAnchor::at_wall_ns` to u64 faithfully
(the i128 value is always ≥ 0 and ≤ u64::MAX for monotonic sources) or
document the clamp.
Confidence: HIGH — Verification: read (types traced; no consumer of
`at_wall_ns` beyond tests).

### F-003 [S4] — dead assertion disjunct in the regression test
Where: crates/media-clock/src/clock.rs:967-975
Code:
```rust
        match clock.try_now() {
            Err(TimeError::Discontinuity {
                expected, found, ..
            }) => {
                assert_eq!(expected.map(|t| t.ticks), Some(0));
                assert!(found.ticks < 0 || found.ticks < held.ticks);
```
What is wrong: for this scenario (anchor at counter 0, counter reset to 0
then advanced 24 000), `found_ns = naive_position` = anchor + (0.5 s − 0) =
0.5e9 ≥ 0, so the first disjunct `found.ticks < 0` can never fire; the
assertion is effectively `found.ticks < held.ticks` (0.5e9 < 2e9 — true and
meaningful). The `|| found.ticks < 0` arm is dead code that suggests a
negative-position case the test never constructs.
Why it matters: dead disjuncts in tests imply coverage that does not exist.
What correct looks like: `assert!(found.ticks < held.ticks);` alone, or a
second scenario that actually produces a negative `found`.
Confidence: HIGH — Verification: read + hand-computed found_ns = 5e8.

## 4. Per-function table (chunk)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MediaClock::anchor | 585-602 | REAL | S4 | F-002 i64 saturation on at_wall_ns; samples arm exact |
| MediaClock::lock_core | 604-612 | REAL | — | poison-recovery `into_inner`; critical sections panic-free by construction (checked arithmetic only) — claim verified by reading every lock body in file |
| snapshot_locked | 615-629 | REAL | — | consistent read under lock; uses now_locked (clamp-updating read — documented) |
| anchor_core | 631-650 | REAL | — | zeroed anchor baseline; build() seeds via adopt_anchor (524-530) |
| adopt_anchor | 652-668 | REAL | S3 | F-001: adopts without touching awaiting_anchor — safe in build(), the hole is at reanchor/resume call sites |
| read_source | 670-700 | REAL | — | zero rate → Unusable; checked i128 sample→ns (overflow arm dead for u64 counters — harmless); monotonic always Usable |
| adopt_if_awaiting | 702-720 | REAL | — | first-usable-reading adoption; F-001's second half |
| now_locked | 722-744 | REAL | — | monotone clamp; Discontinuity holds last_ns; Paused arm defensive raise |
| try_now_locked | 746-769 | REAL | — | typed Overflow (i64::try_from) / Discontinuity (anchor context); no clamp — by design |
| PlaybackClock::now | 771-776 | REAL | — | lock + read + now_locked |
| PlaybackClock::try_now | 778-782 | REAL | — | as above |
| PlaybackClock::master | 784-786 | REAL | — | label from source |
| PlaybackClock::reanchor | 788-805 | REAL | S3 | F-001 latch hole; error-before-mutation verified; Ended latch survives (comment + backends tests) |
| PlaybackClock::set_rate | 807-823 | REAL | — | validates, re-anchors at clamped pos, epoch bump; immune to F-001 (goes through now_locked) |
| PlaybackClock::snapshot | 825-828 | REAL | — | delegates |
| tests: FakeCounter/FakeMonotonic/ts/audio_clock | 838-886 | REAL | — | injected-source seams (crate doctrine), not subject mocks |
| 18 tests | 890-1177 | REAL | S4 | see §5; F-003 dead disjunct |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| anchor_arithmetic_tracks_the_counter (891) | YES | 1 s anchor + 48 k samples = exactly 2 s |
| anchor_arithmetic_scales_by_rate (901) | YES | 5/4 rate hand-verified through checked_div_round |
| monotonic_master_tracks_injected_wall_time (915) | YES | wall +2.5 s → position 2.5 s |
| reanchor_jumps_are_expected_and_epoch_bounded (927) | YES | forward+backward jump values + epoch +2 |
| reanchor_overflow_leaves_clock_unchanged (943) | YES | err + position equality |
| counter_regression_holds_position_never_backwards (955) | YES | hold across reset; Discontinuity payload; reanchor cure advances to 3 s |
| sample_rate_change_is_a_discontinuity (985) | YES | pins the scale-change arm specifically (ns NOT regressed) |
| broken_source_is_unusable_not_panic (1014) | YES | holds at 5 ns + try_now err |
| extreme_positions_saturate_monotonically (1035) | YES | i64::MAX twice + typed Overflow |
| set_rate_never_jumps_and_rescales (1056) | YES | no-jump + 2× rescale |
| set_rate_rejects_zero_components (1071) | YES | pub-field Rate bypass rejected; position unchanged |
| pause_freezes_resume_continues_without_jump (1081) | YES | source keeps running while paused; resume continues |
| pause_and_resume_are_idempotent (1099) | YES | state assertions |
| reanchor_while_paused_moves_the_freeze (1110) | YES | frozen moves to 9 s, resume holds |
| rate_survives_pause_resume (1123) | YES | 2× rate across pause cycle |
| snapshot_is_consistent_and_carries_context (1143) | YES | all six fields pinned |
| anchor_introspection_reflects_the_master (1156) | YES | samples vs wall arms |
| send_sync_object_safe (1172) | YES (compile-time) | trait-bound assertion — real check for the Arc<dyn PlaybackClock> usage |
Gap: no test covers `awaiting_anchor` × reanchor/resume interleavings (F-001)
— the latch's happy path is only exercised via backends.rs tests (part 3/4
agents' scope).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "now() = anchor + (counter_elapsed × rate) within one anchor epoch" | clock.rs:15-17 (module doc, part 1) | REAL | extrapolate 388-399 + tests 891-912 |
| "Monotonic: within one anchor epoch now() never goes backwards — equal at worst" | clock.rs:18-21 | REAL | now_locked clamp 738-739 + regression test |
| "On reanchor a jump is expected; the epoch … separates the stretches" | clock.rs:19-21 | REAL | test 927-940 (epoch +2 asserted) |
| "a poisoned lock is recovered via PoisonError::into_inner" | clock.rs:43-45 | REAL | lock_core 607-611 |
| "extreme sample counts and rates saturate now() … reported by try_now() as TimeError::Overflow" | clock.rs:34-36 | REAL | test 1035-1051 |
| "the clock holds the carried position and adopts the source's **first** usable reading as the fresh anchor baseline" | clock.rs:341-347 (awaiting_anchor doc) | PARTLY FALSE in interleavings | F-001: after a reanchor/resume adoption, the "first" reading is re-adopted — the baseline is the *second* reading and the interval is swallowed |
| "critical sections are one source read plus pure checked arithmetic — never held across I/O" | clock.rs:40-42 | REAL | every lock body in this chunk verified: read_source + arithmetic only |

## 7. Dead / duplicated / unreachable code
- `read_source`'s overflow → Unusable arm (691): dead for u64 counters
  (u64::MAX × 1e9 ≈ 1.8e28 < i128::MAX) — harmless defensive branch.
- F-003's `found.ticks < 0` disjunct: dead.
- No duplication; part-1/part-2 split is clean (helpers shared via pub(crate)).

## 8. Dependencies & flags
media-types only (L2); no features; no unsafe; tests use std atomics for
fakes. All code ships in the default CLI build.

## 9. Open questions for the next agent
- backends.rs parts: does any backends test construct the F-001 interleaving
  (switch → recover → reanchor/resume before first read)? If yes and it
  passes, re-derive — my reading says the swallow is invisible to position
  assertions unless the test asserts exact post-resume progress.
- playback-engine seek.rs auditor: confirm seek.rs:442's reanchor is not
  called while a master-switch latch is pending (if the session guards this
  ordering, F-001's reachability narrows to seek_back+resume only).
