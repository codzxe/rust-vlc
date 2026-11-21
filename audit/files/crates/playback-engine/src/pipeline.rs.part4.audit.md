# AUDIT: crates/playback-engine/src/pipeline.rs#part4 (lines 2002-2668)
| Field | Value |
|---|---|
| Auditor | W03-A10 |
| Date | 2026-11-21 |
| Lines / functions | 667 (chunk 2002-2668 of 3339) / 14 items in range |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `Pipeline::tick` ← `Session::tick_step` session.rs:2431 ← session pump ← `lumen open/play`; baseline avi-firstvertical-open-session.txt shows Buffering→Playing→Stopped→Idle lifecycle executing this code) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | `rg -n "fn nanos_i64"` → seek.rs:330 saturating conversion; `rg -n "high_packets"` → 1115 `.max(1)` guard; `rg -n "pending"` → seek clears at 1257, switch-retire at 1828-1832; `rg -n "\.close\(\)"` → only teardown:1159/switch-retire:1785; `sed -n` on every quoted range (all verified verbatim); `rg -n "frames_delivered"` → session.rs:861, 928-935 fold_preroll_discards |

## 1. One-paragraph verdict
This chunk is the engine's demux/decode slicing and its starvation/rebuffer core — real, traced, budget-bounded logic with genuine backpressure discipline, executed by the shipped AVI lane (baseline `avi-firstvertical-open-session.txt`, 60/60 frames in `rc-smoke-engine-only.log.txt`). The code quality is high but not clean: one conversion site fabricates a negative play-out bound on file-controlled timestamps (`as i64` where the repo's own policy saturates), the concealment delivery path silently violates the documented F5 play-out invariant, and one narrow path double-counts a discarded frame as both delivered and dropped. The comment claims I could falsify (Stale/Closed unreachability, "caller counts the loss") all traced TRUE.

## 2. What actually works (verified)
- FIFO backpressure with refused-re-offer early return (2116-2133): re-offer → if `pending` re-set → `return` — traced that this prevents the historic single-slot overwrite data loss described in the comment. Verified by reading demux_step + all `pending` write sites (only 2223 sets it, only after 2122's `take()`).
- `push_packet`'s claim (2225-2228) that Stale/Closed are unobservable in synchronous v0: verified by tracing every `queue.close()` (teardown pipeline.rs:1159 clears limbs+pending in the same call; switch-retire 1785 removes the limb before any further demux_step) and every `pending` clear (seek 1257 clears pending BEFORE the generation advance at 1262; switch-retire 1828-1832 clears only the retired track's held packet, and surviving limbs' queues are not re-generationed on switch). No reachable path mints a stale stamp into a live queue.
- No division-by-zero in `buffer_fill`: `high_packets = thresholds.high.packets.max(1)` (pipeline.rs:1115, part2 range) — verified by `rg -n "high_packets"`.
- Starve debounce/episode state machine (2029-2081) is exercised by real-Pipeline tests at pipeline.rs:3056-3129 (part5 range) asserting observable counters — see §5.
- `nanos_i64` (seek.rs:330) saturates `u64→i64` per documented policy — the correct conversion this chunk should have used (see F-001).

## 3. Findings

### F-001 [S3]
Where: crates/playback-engine/src/pipeline.rs:2591-2597
Code:
```rust
    fn observe_video_end(&mut self, pts: Stamp) {
        let Some(end) = pts.map_timestamp(|ts| ts.as_nanos().ok().map(|ns| ns.0 as i64)) else {
            return;
        };
```
What is wrong: `ns.0 as i64` is a truncating u64→i64 cast. `Nanoseconds(pub u64)` (media-types/src/time.rs:781); any ns > i64::MAX (~292 years) reinterprets as a **negative** i64. The repo's own conversion policy, `nanos_i64` (seek.rs:326-331: "a theoretical `u64` overflow saturates rather than wraps — never a fabricated position") and `stamp_ns` in this same file (2536-2537: "None when missing or unrepresentable — absence is data, never a guess") both refuse to fabricate; this one site wraps.
Why it matters: `last_end` → `limb.last_video_end_ns` → `Pipeline::last_video_end_ns()` (1180-1186) → `Session::video_played_out` (session.rs:2741-2750: `clock_position_ns(&self.runtime.clock) >= end`). A file-declared pts with ticks near i64::MAX on a 1/90000 base (demux stamps are file-controlled; as_nanos happily returns u64 > i64::MAX) yields a negative end → the gate passes at clock 0 → end-of-media fires prematurely on crafted input. Two conversion policies for the identical operation in one file is itself drift.
What correct looks like: `map(|ns| i64::try_from(ns.0).ok())` (None = unrepresentable, matching `stamp_ns`) or `nanos_i64(ns)`.
Confidence: HIGH — Verification: read (cast semantics + all downstream consumers traced; nanos_i64/Nanoseconds definitions read at seek.rs:330, time.rs:781). Practical trigger requires a hostile/absurd timestamp; no baseline run exercises it.

### F-002 [S3]
Where: crates/playback-engine/src/pipeline.rs:2615-2637
Code:
```rust
        self.consumer.accept(repeated, self.track)?;
        self.concealed = self.concealed.saturating_add(1);
        Ok(())
```
What is wrong: `conceal_last` presents a video frame (the repeated last-good frame re-stamped at the damaged slot's pts) directly to the consumer without calling `observe_video_end`. Every other delivery path extends the F5 play-out high-water — `accept` does it for "every live video frame — held or delivered" (2683-2686, part5 range; struct doc 2575-2580: "its pts is still media the clock must cover before end-of-media").
Why it matters: the documented F5 invariant is violated by the concealment path: a damage-heavy tail (many concealed slots, no successful decode) leaves `last_video_end_ns` at the last *good* frame's pts, so `video_played_out` (session.rs:2744) passes before the concealed slots present — media ends early relative to the concealed timeline, and the doc's stated invariant is false for exactly the damaged streams the concealment exists for.
What correct looks like: call `observe_video_end(pts)` in `conceal_last` before/after the consumer accept (the presented pts is media the clock must cover), or document the exclusion.
Confidence: HIGH — Verification: read (both paths traced; session.rs:2741-2750 consumer read). Mutation reasoning: adding the call would extend end-of-media past concealment tails — behavior change, so tests would notice only if one covered damaged-tail playback; none does (see §5).

### F-003 [S3]
Where: crates/playback-engine/src/pipeline.rs:2494-2498 (merge sites; source is `accept` 2696-2699, part5/A06 range)
Code:
```rust
            report.frames = report.frames.saturating_add(u64::from(sink_frames));
```
plus (boundary, part5):
```rust
            if !hold.push(frame) {
                self.stale_dropped = self.stale_dropped.saturating_add(1);
            }
            self.frames = self.frames.saturating_add(1);
```
What is wrong: a frame refused by a full pre-roll hold is counted BOTH as a drop (`stale_dropped` → `report.stale_frames` → `PipelineTotals.stale_frames_dropped`, session.rs:865-867) AND as delivered (`frames` → `report.frames` → `frames_delivered`, session.rs:861). The same frame is thus reported as delivered and dropped. Additionally the drop lands in a metric named "stale" — a capacity refusal is not staleness; `PrerollHoldSlot::push`'s doc (2661-2664, this range: "the caller counts the loss") is technically true but the count is semantically misfiled. `Session::fold_preroll_discards` (session.rs:927-935) corrects `frames_delivered` down only for frames actually *held* then discarded (`window.held_count()`), never for refusals — a refused frame is in no hold, so its inflation is never corrected.
Why it matters: engine stats lie on this path (delivered-count inflated, stale-count polluted by a non-stale fact). Reachability is narrow — the per-pop `hold_is_full` check (2324) means it needs one `decoder.decode()` call to burst past the whole hold cap; today's decoders (MJPEG/PCM, ≤1 frame per packet) cannot — but the code path is real and uncorrected.
What correct looks like: on refusal, count a dedicated `preroll_refused`/`discarded` field; do not increment `frames` for a frame that never delivered.
Confidence: HIGH on the code fact (both sites read) / LOW on practical reachability with shipped decoders.

### F-004 [S4]
Where: crates/playback-engine/src/pipeline.rs:2010-2019
Code:
```rust
        // The in-episode progress: the bottleneck limb's fill
        // toward its high watermark (card 3.B4 — doc 05 §3.2's
        // `0.0..1.0` while buffering). The exit bookend `1.0` is
        // the table effect; this is the honest mid-episode view.
        if state == SessionState::Buffering {
            // The in-episode progress: the bottleneck pacing limb's
            // fill toward its high watermark (card 3.B4 — doc 05
            // §3.2's `0.0..1.0` while buffering). The exit bookend
            // `1.0` is the table effect; this is the honest
            // mid-episode view.
```
What is wrong: the same four-line comment appears twice, back to back — once outside the `if`, once inside. Copy-paste residue.
Why it matters: comment rot; the outer copy is stale (its "bottleneck limb's" wording was superseded by the inner "bottleneck pacing limb's").
What correct looks like: delete the outer block.
Confidence: HIGH — Verification: read (sed -n '2010,2020p').

### F-005 [S4]
Where: crates/playback-engine/src/pipeline.rs:2020-2028
Code:
```rust
            report.buffer_fill = self
                .limbs
                .iter()
                .filter(|limb| self.paces(limb))
                .map(|limb| {
                    (limb.queue.usage().packets as f32 / limb.high_packets as f32).clamp(0.0, 1.0)
                })
                .fold(1.0_f32, f32::min);
```
What is wrong: for an empty pacing-limb set (e.g. zero limbs), `fold(1.0, min)` returns 1.0 — buffer_fill reports "full" — while the priming computation in the same tick reports `primed = !self.limbs.is_empty()` = false for the same condition (1971). The two signals contradict: fill=1.0, primed=false.
Why it matters: if a session ever reaches Buffering with an empty limb set (deselect-all during buffering — whether that state is reachable is owned by session.rs, not audited here), the UI would read 100% buffered while never priming. Edge case, contradictory-telemetry class.
What correct looks like: make fill None/0.0 when nothing paces, consistent with `primed` (e.g. compute over the filtered set and only fold when non-empty).
Confidence: MEDIUM — Verification: read; reachability of the empty-limb Buffering state UNVERIFIED (session-layer deselect path not in this chunk — open question Q-1).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Pipeline::tick` (tail: priming loop end, buffer_fill, starvation detect) | 1904-2083 (chunk covers 2002-2083) | REAL | S4 | Budget-bounded; starvation suppression while `producer_parked` (2062) is a traced, documented anti-flap trade-off — see §9 Q-2 for the paused-audio-ring corner; episode/debounce logic test-covered (§5) |
| `Pipeline::starvation_counters` | 2090-2092 | REAL | — | Copy-out snapshot; folded by session.rs:2114 `absorb_pipeline_starvation` |
| `Pipeline::is_drained` | 2100-2107 | REAL | — | Empty-limb vacuous-true by documented design (card 3.A3); requires eof + no pending + all limbs drained & empty — consistent |
| `Pipeline::demux_step` | 2112-2188 | REAL | — | Re-offer-then-pull FIFO; refused re-offer returns (2124-2133) — the 6.FIX-pcm data-loss fix, traced; Err→fatal; Recovered counts are loss-units only (2155-2167) |
| `Pipeline::push_packet` | 2193-2231 | REAL | — | Unwired tracks dropped by policy; Full→hold+axis tallies; Stale/Closed silent-drop claim TRACED TRUE (§2) |
| `Pipeline::decode_step` | 2269-2533 | REAL | S3 | Per-limb generation floor; hold & ring backpressure; per-frame damage concealment with bounded budget (2381: `> VIDEO_CONCEALMENT_BUDGET` → typed terminal); merge sites are F-003's evidence |
| `stamp_ns` | 2538-2544 | REAL | — | Saturating-via-nanos_i64, Option-returning — the correct policy F-001 says observe_video_end should share; used at 911/919/1201/1574 |
| `LimbSink` (struct) | 2557-2585 | REAL | — | Plain state carrier |
| `LimbSink::observe_video_end` | 2591-2597 | REAL | S3 | F-001 (`as i64` truncation); redundant double-`Some`-destructure is cosmetic |
| `LimbSink::conceal_last` | 2615-2637 | REAL | S3 | F-002 (bypasses observe_video_end); stale-generation guard (2625) and hold-holds rule (2617) traced correct; clone+re-stamp is the documented D-1 fixed-field clone |
| `LimbSink::hold_is_full` | 2641-2643 | REAL | — | Trivial, correct |
| `PrerollHoldSlot` (struct) | 2649-2652 | REAL | — | Borrowed slice + cap; no allocation |
| `PrerollHoldSlot::is_full` | 2657-2659 | REAL | — | Trivial, correct |
| `PrerollHoldSlot::push` | 2665-2671 | REAL | S3 | Refusal counted by caller (verified, 2696-2698) but into the misleading `stale_dropped` field — F-003 |

(Chunk boundary note: `impl FrameSink for LimbSink` (2674-2724) and `mod tests` (2727-3338) belong to part5/A06; I read both for verification of cross-boundary claims only.)

## 5. Test honesty
Tests for this chunk's logic live in the part5 test module (pipeline.rs:2727+) — they run the REAL Pipeline (real queues, real demuxer/decoder seams) and assert observables:
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `first_stall_of_a_fresh_episode_fires_at_one_tick` (3058-3071) | YES | Removing the `!limb.episode_open` immediate-fire arm (2066) would make `near.starved` false → assert fails; counters asserted exactly |
| `single_tick_reentry_dips_do_not_refire` (3077-3110) | YES | Deleting the debounce (2066 `low_streak >= self.starve_debounce`) → dips re-fire → `starve_signals != 1` fails |
| `sustained_low_fires_at_the_debounce_count` (3115-3129+) | YES | Off-by-one in streak increment (2065) shifts which tick fires → `t2/t3` asserts flip |
| parked-consumer backpressure test (~3012-3053) | YES | Deleting the ring gate (2343-2348) → `accepts != 0` fails; queue-retention asserted |
| Baseline: these ran green — `orchrun-test-workspace-nofailfast.log.txt` reports no playback-engine binary among the 7 failures | — | cited as execution evidence |

No test covers: damaged-tail concealment + end-of-media interaction (F-002), hold-cap refusal accounting (F-003) — both untested paths.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "fresh data is never dropped" under backpressure | doc 03 §4.4 Q3, cited in comment 2220-2222 | REAL | Traced: Full→`pending` hold (2223); refused re-offer returns before any newer push (2124-2133) |
| "concealment presentations are counted separately so a repeated frame is never reported as decoded output" | comment 2491-2493 | REAL | 2635 `concealed` vs `frames`; merge 2494-2497 separate fields |
| "every live video frame — held or delivered — extends the play-out bound" (F5) | comments 2577-2580, 2683-2684 | FALSE for the concealment path | F-002: `conceal_last` presents without `observe_video_end` |
| "the caller counts the loss (never buffered unboundedly, never silently swallowed)" | `PrerollHoldSlot::push` doc 2661-2664 | REAL (with caveat) | Caller counts at 2697 — but into the misnamed `stale_dropped` (F-003) |
| MP4/AVI playback lifecycle through this code | baseline INDEX §smoke | REAL | avi-firstvertical-open-session.txt: Buffering→Playing→Stopped→Idle, 60/60 frames (rc-smoke-engine-only.log.txt); sample-420.mp4 PLAYS (INDEX CORRECTION v2) |

## 7. Dead / duplicated / unreachable code
- Duplicated comment block (F-004).
- No dead functions in range: `stamp_ns` (file-private) has 4 live call sites (911/919/1201/1574); all `LimbSink`/`PrerollHoldSlot` methods have live callers.
- `PushOutcome::Stale|Closed` arm (2229) is currently unreachable in v0 by traced design (§2) — dead-in-practice but a correct forward-compat seam, not a defect.

## 8. Dependencies & flags
- Uses `packet_queue::{LimitAxis, PushOutcome, PopOutcome}` — cross-crate seams, no feature gates in range.
- No `unsafe`, no allocations on the per-frame path beyond the documented fixed-field clone (2709-2713, part5) — consistent with the zero-alloc claims audited elsewhere (note: the desktop mailbox zero-alloc test FAILS in the no-fail-fast baseline, `orchrun-test-workspace-nofailfast.log.txt` failure #1 — that path is apps/desktop, not this file).

## 9. Open questions for the next agent
- Q-1 (session.rs owner): can a session sit in `Buffering` with an empty limb set (deselect-all during buffering)? If yes, F-005's contradictory fill=1.0/primed=false becomes user-visible.
- Q-2 (session/consumer owner): while `producer_parked` (2049) suppresses starvation detection, a permanently-not-ready audio consumer (ring full, device stalled) with a full audio queue would starve video silently in `Playing` forever. What drains the ring when playback is in the stalled state? If nothing can, this upgrades to S2.
- Q-3 (codecs owner): can any shipped decoder emit >1 frame from a single `decode()` call (F-003's trigger)? MJPEG/PCM cannot (verified per doc 03 §4.5(d) comment 691-695); H.264 sw (sw.rs) — verify B-frame reordering emission counts.
