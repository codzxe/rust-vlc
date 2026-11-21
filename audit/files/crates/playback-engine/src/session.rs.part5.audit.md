# AUDIT: crates/playback-engine/src/session.rs#part5 (lines 2445-3058)
| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-21 |
| Lines / functions | 614 (chunk of 3058-line file) / 16 (11 prod incl. 2 helpers + 3 free fns, 9 test fns) |
| File verdict | REAL (chunk) |
| Reachable from shipped entry point? | YES (call chain: `lumen` bin → apps/cli/src/host.rs:231 `Engine::new()` → host.rs:448 `engine.poll()` → session.rs:1173-1176 `poll` dispatches Buffering/Playing/Paused → `tick_step` (2428) → all chunk functions) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 5 |
| Commands run | `rg -n "fn translate_pipeline_signal"` → state.rs:931 (legality source for F-001); `sed -n 920,990p state.rs` → Fatal/Drained/Eof/Primed/Starved table, tick states always map Fatal→Failed; `rg -n "buffering_last\|episode_open"` → resets at 1673-1675 (episode discipline verified correct); `rg -n "SessionEngine"` → none (type is `Engine`, session.rs:949); `rg -ln "playback_engine"` → apps/cli host/dispatch/pipeline_host + apps/web; `rg -n "transition_table\(\)"` → only tests/table.rs:156; `rg -n "internal_missing_*"` → defensive callers 1584/2351; `rg -n "ClockMaster"` media-clock → SystemMonotonic fallback exists (video_played_out design holds); baseline run check `rg -n "session::tests::(the_hub|a_newer|capacity|receipts|the_inbox)"` → all 5 ok |

## 1. One-paragraph verdict
This chunk is the tick engine's emission half — pipeline tick translation, seek
and track-switch completion, open-facts emission, and the bounded-hub unit
tests — and it is genuinely real: reachable from `engine.poll()` in the
shipped CLI, executed by the no-fail-fast baseline (all 5 in-file tests ok),
and causally coherent (side events before signal translation; audio switch
terminal before video; seek terminal before the same tick's state work). The
seek-completion half at 2526-2591 is exactly the session-side path of the
baseline's still-failing `journey_seek` (decode accounting, observed 90 vs
154 — orchestrator summary item 4), though the accounting defect is not
provable from this chunk alone. The real defects are discipline breaches:
one silently swallowed state-transition rejection, an undocumented swallow
twin, a 0-position fallback that contradicts its own "never a guess" comment
on the wire, fabricated `ResourceLimit` error kinds for internal breaches,
and test-only public API.

## 2. What actually works (verified)
— All 5 chunk tests executed ok: baseline
`audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3806-3824`
(`a_newer_tick_replaces... ok`, `capacity_pressure... ok`,
`receipts_are_pinned... ok`, `the_inbox_is_bounded... ok`,
`the_hub_stays_bounded... ok`).
— Buffering-episode discipline verified by reading: episode opens reset
`buffering_last`/window (session.rs:1673-1675), exits close with the 1.0
bookend (1678-1680, 1703-1713) — no stranded 0.0 episode.
— `video_played_out`'s no-audio design verified: `ClockMaster::SystemMonotonic`
fallback exists (media-clock clock.rs:262-268), so a masterless clock still
advances to `last_video_end_ns`; audio-limb sessions exempt per F5/issue #16
rationale (pipeline.rs:1165-1188).
— Mutation check on `the_hub_stays_bounded...`: switching drop-oldest to
drop-newest fails the survivor assertion `(10_000-8..10_000)` (2948);
breaking the resident-tick replacement counter fails 2961. Tests are
load-bearing.

## 3. Findings
### F-001 [S3]
Where: crates/playback-engine/src/session.rs:2631-2638
Code:
```rust
            match self.apply(input, context) {
                Ok(mut transition_outputs) => outputs.append(&mut transition_outputs),
                Err(_) => {
                    // Unreachable: the exhaustive tests prove the
                    // translation only yields table-legal inputs.
                    // Honored honestly (dropped) rather than
                    // panicked on.
                }
            }
```
What is wrong: A rejected state transition inside the tick is swallowed with
no counter, no diagnostic event, no log. The unreachability claim leans on
`translate_pipeline_signal` (state.rs:931) only ever returning table-legal
inputs — a property pinned by tests in another file, not enforced by types
here. The crate's own discipline everywhere else is "drops are counted, never
silent" (e.g. video-engine policy.rs:104, the hub's `dropped_events`).
Why it matters: If a new `PipelineSignal`/state combination is ever added to
`tick_step`'s priority ladder (2608-2621) without a matching translate row,
the session silently freezes in its old state — no error, no observability,
an undiagnosable hang.
What correct looks like: Count the rejection into a session/engine stat
(like `dropped_events`) or emit a typed internal-error event.
Confidence: HIGH — Verification: read + translate table read at state.rs:931-966.

### F-002 [S4]
Where: crates/playback-engine/src/session.rs:2812-2822
Code:
```rust
/// The clock's current position in nanoseconds — the seek target's
/// resolution base and the failed-seek report's rollback position.
/// The clock answers in the nanosecond base, so the `Err` arm is the
/// i64-exhaustion corner only; 0 is the honest floor there, never a
/// guess presented as truth.
pub(crate) fn clock_position_ns(clock: &MediaClock) -> i64 {
    match clock.now().as_nanos() {
        Ok(ns) => nanos_i64(ns),
        Err(_) => 0,
    }
}
```
What is wrong: On the (practically unreachable — `MediaClock::now` returns
NANO_BASE timestamps, clock.rs:772-774, whose `as_nanos` fast-path cannot
overflow) error arm, 0 IS then presented as truth: `TimeTick { position_ns: 0 }`
(2653-2659) and the failed-seek `position` (2567) both go on the wire, and
`video_played_out` (2746) compares `0 >= end` → gate closes → end-of-media
never fires. The doc comment's "never a guess presented as truth" is
contradicted by its own callers.
Why it matters: Low (corner is ~292-year positions), but the comment claims a
guarantee the code does not provide; the honest shape is a typed error or
saturating to the last known position.
What correct looks like: Propagate `Result` or clamp to the last emitted
position; or fix the comment to admit 0 goes on the wire.
Confidence: HIGH — Verification: read + media-types time.rs:192-196/461-477
(overflow-only Err) + clock.rs:772 (NANO_BASE now).

### F-003 [S4]
Where: crates/playback-engine/src/session.rs:2704-2706
Code:
```rust
            if let Ok(mut transition_outputs) = self.apply(SessionInput::Failed, context) {
                outputs.append(&mut transition_outputs);
            }
```
What is wrong: The third `apply` swallow in the chunk — and the only one
with NO comment. Its twins document their unreachability (2633-2638;
fail_with at 2805-2808 "honored honestly"). Here, if `Failed` were rejected,
the Fatal event (2697) already went out but the `state-changed → error`
transition is silently lost: host sees a fatal error with the session
reporting a non-error state.
Why it matters: Inconsistent defensive discipline; a reviewer cannot tell
whether this swallow was reasoned about or forgotten.
What correct looks like: Same comment as the twins, or a counted rejection.
Confidence: HIGH — Verification: read; `translate` maps all tick states'
Fatal → `Failed` (state.rs:941), so unreachable today.

### F-004 [S4]
Where: crates/playback-engine/src/session.rs:2826-2836
Code:
```rust
fn internal_missing_fatal() -> EngineError {
    EngineError::new(
        media_types::PipelineStage::Control,
        media_types::ErrorKind::ResourceLimit {
            limit: media_types::error::LimitKind::Memory,
            budget: 0,
            attempted: 0,
        },
        media_types::error::ErrorContext::new("engine.fatal-missing"),
    )
}
```
What is wrong: An internal invariant breach (a `Failed` input without its
payload, 1584; a tick/open with no pipeline, 2351) is reported as a **memory
resource limit** with budget 0 / attempted 0 — a fabricated taxonomy. A user
seeing this error would look for OOM conditions, not an engine bug.
`internal_missing_pipeline` (2840-2850) repeats the shape.
Why it matters: Wrong error kinds misdirect diagnosis; the context string
(`engine.fatal-missing`) is honest but the kind lies.
What correct looks like: An `ErrorKind::Internal`/`Contract` variant (or
whatever media-types prescribes for invariant breaches), not ResourceLimit.
Confidence: HIGH — Verification: read + callers at 1584/2351.

### F-005 [S4]
Where: crates/playback-engine/src/session.rs:2852-2858
Code:
```rust
/// The whole transition table is reachable from the pure model —
/// exposed for the exhaustive tests (and for future model checkers
/// per doc 03 §4.6(f)).
#[must_use]
pub fn transition_table() -> &'static [Transition] {
    TRANSITIONS
}
```
What is wrong: Public API whose only consumer is
`crates/playback-engine/tests/table.rs:156-157` (plus model tests). "Future
model checkers" is speculative. It exports the table raw with no contract.
Why it matters: Surface area with no production caller; the model tests could
use a `#[cfg(test)]`/feature-gated path.
What correct looks like: cfg-gate or move to a test-support module.
Confidence: HIGH — Verification: `rg -n "transition_table\(\)"` → only
tests/table.rs.

### F-006 [S4]
Where: crates/playback-engine/src/session.rs:2559-2561 (also 2471, 2587, 2707, 2722)
Code:
```rust
                    if let Some(pipeline) = self.pipeline.as_mut() {
                        pipeline.release_preroll_hold();
                    }
```
What is wrong: Dead defensive re-checks. `tick_step` early-returns when
`self.pipeline` is `None` (2430-2433), and `complete_switch_kind` is only
called from `tick_step` (2600-2601) — so every later `self.pipeline.as_mut()`
in these paths is provably `Some`. Five copies of the same impossible arm.
Why it matters: Noise that hides the real invariant (single live pipeline
within a tick) and implies a liveness the code does not model.
What correct looks like: One binding of the pipeline after the early return,
or a debug_assert.
Confidence: HIGH — Verification: read of 2430-2433 and all five sites.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `tick_step` | 2428-2665 (chunk from 2445; signature/doc in part4) | REAL | S3 (F-001) | Priority ladder fatal>drained>eof>primed>starved verified against translate table; fatal always translates in tick states; end-of-track suppressed on the draining tick (by design, comment 2460-2466); seek completion emits terminal + immediate tick + preroll release; dead defensive re-checks F-006 |
| `complete_switch_kind` | 2674-2725 | REAL | S4 (F-003) | Audio-first order enforced by caller (2600-2601); re-anchor failure path emits Fatal + Failed; `let ... else` at 2685 type-system-required, provably Ready; switch terminal under switch's own generation (emit_terminal) |
| `video_played_out` | 2740-2749 | REAL | — | F5/issue-16 gate; audio-limb exemption sound (DrainGatedDecoder seam, SystemMonotonic master); None ⇒ open gate correct |
| `inflight_switch_of` / `take_inflight_switch_of` | 2752-2765 | REAL | — | Trivial accessors, correct per-kind slots |
| `emit_open_facts` | 2771-2794 | REAL | — | Once-per-open flags reset only at Session::new (1456-1457) — correct: one open per session (opening_attempted gate 2343); duration from declared extents (pipeline.rs:1198-1206) known at open |
| `fail_with` | 2798-2809 | REAL | — | Documented swallow twin of F-003; `unwrap_or_default()` |
| `clock_position_ns` | 2817-2822 | REAL | S4 (F-002) | Err→0 fallback; unreachable-in-practice corner; comment contradicted by wire usage |
| `internal_missing_fatal` | 2826-2836 | REAL | S4 (F-004) | Fabricated ResourceLimit/Memory kind |
| `internal_missing_pipeline` | 2840-2850 | REAL | S4 (F-004) | Same shape |
| `transition_table` | 2856-2858 | REAL | S4 (F-005) | Test-only export |
| test helpers `pair_at`/`marker_event`/`tick_event`/`receipt` | 2873-2911 | REAL | — | White-box constructors |
| test `the_hub_stays_bounded_and_counts_every_drop_at_capacity` | 2922-2949 | REAL | — | Ran ok (baseline :3824); drop-oldest + compaction bound pinned |
| test `a_newer_tick_replaces_the_undelivered_older_one` | 2956-2977 | REAL | — | Ran ok (:3806); resident-tick replacement semantics |
| test `capacity_pressure_may_drop_the_resident_tick_and_counts_it` | 2983-2993 | REAL | — | Ran ok (:3807) |
| test `receipts_are_pinned_from_events_but_hard_capped` | 3000-3032 | REAL | — | Ran ok (:3808); oldest-receipt drop + ordering |
| test `the_inbox_is_bounded_and_answers_typed_busy` | 3038-3057 | REAL | — | Ran ok (:3809); sync_channel(2) typed busy refusal |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| the_hub_stays_bounded... | YES | Exact live/dropped counts + newest-survivors seq range (2948); compaction slot bound asserted |
| a_newer_tick_replaces... | YES | `time_ticks_replaced == 1` and drain order `[2, 3]`; post-drain freshness re-assert |
| capacity_pressure_may_drop... | YES | dropped_events == 1 and `resident_tick.is_none()` |
| receipts_are_pinned... | YES | 16 dropped events, 4 pinned receipts, then oldest-receipt drop `[1,2,3,100]` |
| the_inbox_is_bounded... | YES | Third submit must match `Err(SessionError::InboxBusy)`; queued order + recovery asserted |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "at most one translated input per poll, priority Fatal > Drained > EofReached > Primed > Starved" | doc 03 §4.6 / state.rs:923-927 | REAL | tick_step 2608-2621 implements exactly; translate table state.rs:941-965 |
| "immediate `time-tick` after `seek-completed`" (doc 05 §3.2) | session.rs:2540-2552 comment | REAL | Emitted at 2546-2552 with `last_tick_ns` baseline to prevent double-tick (2649-2651) — logic read and verified |
| "exactly one terminal event per completed generation" (doc 05 §4.3 inv 4) | session.rs:2583-2585 comment | REAL | Seek: terminal + `inflight_seek = None` + `end_seek` (2586-2589); switch: terminal under switch generation (2714-2721) |
| "monotonic within the episode, ≤ 5 Hz" buffering progress (doc 05 §3.4 rule 2) | session.rs:2484-2496 | REAL | Strict `>` + `ProgressWindow::permits`; episode reset verified at 1673-1675 |
| Seek path end-to-end health | baseline orchrun summary item 4 | FALSE (currently broken) | `journey_seek` FAILED: "decode accounting keeps the pre-seek deliveries — observed 90, expected 154" (orchrun log :2521-2533); session-side completion logic in this chunk is coherent, but the seek path's accounting defect is live somewhere (totals/pipeline side — not locatable from this chunk) |

## 7. Dead / duplicated / unreachable code
- `transition_table()` — test-only export (F-005).
- Five dead `pipeline.as_mut()` guards in tick paths (F-006).
- `internal_missing_*` — defensive stand-ins for impossible states (callers
  1584/2351); reachable only via engine bug.

## 8. Dependencies & flags
Uses `commands::envelope::RequestId`, `media_types` (events/errors),
`media_clock::MediaClock`, `testkit::Must` (tests only). No feature gates, no
unsafe, no I/O in this chunk. All emission is through the shared `seq`
AtomicU64 (Relaxed fetch_add) — single-writer discipline per the module docs.

## 9. Open questions for the next agent
- Who owns the `journey_seek` accounting defect (baseline failure #4): audit
  `PipelineTotals::merge_tick` (part4) and the pipeline's decode accounting
  against the journey scenario's expected 154 pre-seek deliveries.
- `OutputHub` implementation (part1/2) — this chunk's tests pin its policy;
  the implementation audit should cross-check `drain_into`'s epoch rule
  against the R9 suppression claim (session.rs:1030-1033).
