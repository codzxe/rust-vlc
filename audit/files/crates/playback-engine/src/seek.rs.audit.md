# AUDIT: crates/playback-engine/src/seek.rs
| Field | Value |
|---|---|
| Auditor | W03-A07 |
| Date | 2026-11-23 |
| Lines / functions | 545 / 21 (14 production fns/types + 2 manual impls + 6 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: apps/cli/src/host.rs:1041-1107 (`playback.seek` → `SessionHandle::seek`) → crates/playback-engine/src/session.rs:2002-2043 (resolve_target, pipeline_seek) and :2533 (complete_seek) — all in the release lane. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 1 |
| Commands run | `cargo test -p playback-engine --lib seek::` -> 6 passed (exit 0); `cargo test -p playback-engine --test seek_flow` -> 21 passed (exit 0); `cargo test -p playback-engine --test property seek_roundtrip` -> 6 passed (exit 0); sed/rg verifications incl. pipeline.rs seek_flush/duration_ns/stamp_ns and packet-queue queue.rs flush. |

## 1. One-paragraph verdict
Hostile question answered head-on: seek.rs contains NO reopen-at-target — it implements the seven-step IN-PIPELINE seek (accept/resolve → generation bump → queue+decoder flush → demux reanchor → pre-roll hold → clock reanchor on the first audio frame's truth → Seeked terminal), and the reopen composition is explicitly forbidden for the CLI host (apps/cli/src/host.rs:39-45) and survives only as the disclosed old 2.D2 shape in the test harness (tests/integration/first-vertical/src/lib.rs:64-65). The flow is real, wired into the shipped CLI, and heavily tested (my runs: 6 unit + 21 seek_flow + 6 property tests, all pass). Is it honest about its costs? Mostly: decoder-flush stale frames are counted into `stale_frames_dropped`, pre-roll discards into `preroll_frames_discarded`, failed-seek rollback is a disclosed bounded rebuffer, and the Playing-seek rebuffer through the watermark machinery is admitted ("no fast-start seek heuristic"). Three dishonesties/gaps found: (1) a PANIC path — `ns.clamp(0, duration)` panics whenever a demuxer reports a negative duration (min>max), reachable through the plugin Demuxer seam; (2) the seek flush's queued-packet drop cost is computed by `PacketQueue::flush` and then explicitly discarded (`let _ =`), so the packets every seek throws away are counted nowhere; (3) a decode-stage flush failure is mislabeled `SeekFailure::Demux`. Plus a NaN percent fraction silently becoming "seek to zero", and a false "(no subtitles)" doc claim.

## 2. What actually works (verified)
— `cargo test -p playback-engine --lib seek::` -> 6 passed; `--test seek_flow` -> 21 passed; `--test property seek_roundtrip` -> 6 passed (all exit 0, my runs). Verified by read: D4 rollback (session.rs:2075-2095 — flush happened, demuxer never moved, clock generation published, SeekFailed with old position, slot cleared); supersession (session.rs:2066-2073 — the older seek is silently replaced, no terminal, matching doc 05 §4.2); the Paused decode carve-out is real (pipeline.rs:1928-1946); `complete_seek` anchors the clock on the first AUDIO frame's timestamp, not the request (434-450), pinned by seek_flow's `the_audio_frame_timestamp_is_the_landing_truth_not_the_request` (pass). Cost accounting that DOES exist: `outcome.stale_frames_flushed` → `totals.stale_frames_dropped` (session.rs:2047-2050), preroll discards folded at every boundary (session.rs:2097-2107).

## 3. Findings
F-001 [S2]
Where: crates/playback-engine/src/seek.rs:278-283 (and 293 for Percent)
Code:
```
    let clamp = |ns: i64| -> i64 {
        match duration_ns {
            Some(duration) => ns.clamp(0, duration),
            None => ns.max(0),
        }
    };
```
What is wrong: `i64::clamp` PANICS when min > max. `duration_ns` comes from the plugin Demuxer's track timing (session.rs:2001 → pipeline.rs:1198-1203 `duration_ns` = max of `stamp_ns(track.timing.span.end)`, and `stamp_ns` (pipeline.rs:2538-2544) passes negative values through — negative timestamps are legal in this codebase; seek.rs's own tests acknowledge negative ticks). If every track's span end is negative, `Some(duration)` is negative and ANY Absolute/Relative/Percent seek panics: "assertion failed: min <= max". The engine treats plugin data as untrusted everywhere else (typed refusals, landing_missing for contract violations) but trusts the demuxer's duration as a clamp bound without `duration.max(0)`. Why it matters: a seek on such a stream crashes the session thread instead of answering typed — the exact failure class this module's header promises never happens. What correct looks like: `Some(duration) => ns.clamp(0, duration.max(0))` or refuse typed on negative duration. Confidence: HIGH on the code path (clamp panic semantics are certain) — Verification: read; UNVERIFIED: whether any shipped demuxer (mp4/mkv span-end computation, crates/demux/src — another agent's files) can actually emit an all-negative span end; the in-tree containers I sampled anchor spans at 0 or computed positive ends.
Confidence: HIGH — Verification: read / UNVERIFIED (reachability from shipped demuxers)

F-002 [S3]
Where: crates/playback-engine/src/seek.rs:377-385
Code:
```
    // Step 3 — the flush. A decoder flush/configure failure is a
    // decode-stage refusal of the new generation: the seek fails
    // honestly and the pipeline limps on (the next decode error
    // surfaces through the normal fatal path). The frames the flush
    // already purged are still gone — count them.
    if let Err(error) = pipeline.seek_flush(generation, flush_budget, hold_cap) {
        outcome.stale_frames_flushed = pipeline.take_seek_flush_stale();
        outcome.failure = Some(SeekFailure::Demux(error));
```
What is wrong: the comment says the failure is "a decode-stage refusal", and the error carries `PipelineStage::Decode` — but it is wrapped as `SeekFailure::Demux`, whose doc (213-215) says "The demuxer could not position the stream". The `SeekFailed` event thus classifies a decoder failure as a demuxer failure; any consumer switching on the reason variant misdiagnoses the fault. The inner EngineError preserves the true stage, so this is mislabeling, not data loss. What correct looks like: a `SeekFailure::Flush(EngineError)` variant (or reuse the error's own stage). Confidence: HIGH — Verification: read (seek_flush's error comes from decoder flush/configure per pipeline.rs:1239+ and the comment here).
Confidence: HIGH — Verification: read

F-003 [S3]
Where: crates/playback-engine/src/seek.rs:291-294
Code:
```
        SeekTarget::Percent { fraction } => match duration_ns {
            Some(duration) => ResolvedSeek::Position {
                ns: clamp((f64::from(fraction.clamp(0.0, 1.0)) * duration as f64).round() as i64),
            },
```
What is wrong: `f32::clamp` propagates NaN (NaN fails both comparisons), `NaN * duration = NaN`, `NaN.round() = NaN`, and `NaN as i64 = 0` (saturating cast) — so `Percent { fraction: NaN }` with a known duration silently resolves to `Position { ns: 0 }`: a seek-to-start reported as a normal seek, instead of the typed out-of-range this module uses for unresolvable fractions. Unreachable through serde_json hosts (JSON cannot spell NaN) but reachable by any in-process API user (the engine is a library; SeekTarget is public). What correct looks like: `if fraction.is_nan() { ResolvedSeek::Failure(SeekFailure::OutOfRange) }`. Confidence: HIGH — Verification: read (Rust float semantics); no executed check (would need a new test, forbidden).
Confidence: HIGH — Verification: read / UNVERIFIED (executed)

F-004 [S3]
Where: crates/playback-engine/src/seek.rs:21-24 (the claim) vs crates/playback-engine/src/pipeline.rs:1257-1259 and crates/packet-queue/src/queue.rs:334-346 (the reality)
Code (pipeline.rs):
```
        self.end_seek();
        for limb in &self.limbs {
            let _ = limb.queue.flush(generation);
        }
```
What is wrong: `PacketQueue::flush` RETURNS the drop report ("the synchronous report of what this flush dropped" — dropped packets/bytes/duration, queue.rs:334-346) and seek_flush throws it away with `let _ =`. EngineStats (session.rs:766-830) counts decoder stale frames (`stale_frames_dropped`), pre-roll discards, suppressed events, starvation — but has NO counter for the queue-resident packets every seek destroys. So the answer to "is seek honest about its costs": decoder and preroll costs are counted; the largest routine cost (queued packet eviction, up to the full queue watermark per seek) is computed by the queue and discarded at the pipeline layer — invisible to the soak/observability surface that everything else feeds. What correct looks like: fold the Discontinuity drop counts into a `packets_flushed_seek` stat (or into the existing stale accounting). Confidence: HIGH — Verification: read (all three sites).
Confidence: HIGH — Verification: read

F-005 [S4]
Where: crates/playback-engine/src/seek.rs:140-143
Code:
```
//! - The renderer-mailbox and subtitle-active-set clears of doc 03
//!   §6 step 3 have no v0 counterparts (no mailbox, no subtitles);
//!   the [`FrameConsumer`](crate::pipeline::FrameConsumer) seam is
//!   immediate and generation-checked at the sink.
```
What is wrong: "(no subtitles)" is false — subtitle limbs exist and are wired (pipeline.rs:267-304: `pub subtitle: QueueLimits`, `with_subtitle`; session_flow.rs:890-913 proves a subtitle-only source plays and drains). The seek flush does flush the subtitle limb's queue like any other, so the practical behavior is fine; the parenthetical documenting WHY the step-3 subtitle clear is absent states a nonexistent limitation. What correct looks like: "(no renderer mailbox; subtitles are stateless sink deliveries, no active set to clear)". Confidence: HIGH — Verification: read + my session_flow run.
Confidence: HIGH — Verification: read / ran

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SeekTarget (enum) | 155-182 | REAL | — | public command surface; Chapter honestly refuses |
| ResolvedSeek (enum) | 186-196 | REAL | — | position vs typed refusal |
| SeekFailure (enum) | 204-219 | REAL | — | 4 typed reasons |
| Clone for SeekFailure | 221-233 | REAL | — | rebuilds without source chain (event convention) |
| PartialEq for SeekFailure | 235-247 | REAL | — | compares stage/kind/code only |
| shallow | 250-258 | REAL | — | — |
| not_seekable | 263-266 | REAL | — | engine-side policy fact |
| resolve_target | 273-304 | REAL | S2/S3 | clamp panic on negative duration (F-001); NaN percent → silent 0 (F-003) |
| target_span | 308-313 | REAL | — | ns → NANO_BASE span |
| span_start_ns | 318-324 | REAL | — | missing stamp → None → typed failure |
| nanos_i64 | 330-332 | REAL | — | saturating, documented |
| InFlightSeek | 336-347 | REAL | — | target/landing/generation/origin |
| SeekOutcome | 353-362 | REAL | — | inflight vs failure + stale count |
| pipeline_seek | 369-412 | REAL | S3 | flush failure mislabeled Demux (F-002) |
| landing_missing | 416-424 | REAL | — | typed stand-in for trait violation |
| complete_seek | 434-450 | REAL | — | audio-frame truth anchor; Clock failure typed |
| tests: absolute_clamps_to_the_duration_when_known | 457-485 | REAL | — | 3 shapes; ran ok |
| tests: relative_resolves_against_the_position_then_clamps | 488-501 | REAL | — | ran ok |
| tests: percent_needs_a_known_duration | 504-522 | REAL | — | no NaN case (F-003 gap) |
| tests: chapter_refuses_typed_until_a_chapter_source_exists | 525-528 | REAL | — | ran ok |
| tests: target_span_and_back_is_lossless_in_nano_base | 531-536 | REAL | — | ran ok |
| tests: failure_clones_and_compares_without_source_chains | 539-544 | REAL | — | ran ok |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| 6 unit tests here | YES (for what they cover) | exact resolved values / typed refusals |
| (gaps) | — | no negative-duration case (F-001 untested), no NaN case (F-003 untested); pipeline_seek/complete_seek covered downstream in seek_flow.rs (21 tests, my run ok) and property/seek_roundtrip.rs (6 tests, my run ok) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the seven steps, as implemented" (flush → reanchor → preroll → clock reanchor → report) | seek.rs:5-65 | REAL | session.rs:2002-2107 + 2519-2589 implement exactly this; seek_flow 21 tests pass (my run) |
| "never a fake success" (unresolvable/unseekable → immediate SeekFailed) | seek.rs:15-17 | REAL | session.rs:2002-2033; seek_flow tests |
| D4: "On any failure the session rolls back to the prior position ... a rebuffer — honest, bounded" | seek.rs:82-95 | REAL | session.rs:2075-2095; rollback is disclosed as a cost |
| "(no mailbox, no subtitles)" | seek.rs:140-143 | FALSE (subtitles half) | subtitle limbs wired (pipeline.rs:267-304; session_flow subtitle-only test) — F-005 |
| "Clamped to [0, duration] when the duration is known — clamping is truth" | seek.rs:157-159 | REAL except the panic corner | clamps verified by unit tests; negative-duration input panics instead (F-001) |
| "superseded seeks are silent" (no terminal) | seek.rs:97-106 | REAL | session.rs:2066-2073 replaces slot without terminal; source_swap's 25 cut-off seeks assert 0 terminals (my run ok) |
| first-vertical "Seek composes as stop → close → reopen-at-target (the 2.D2 shape)" | tests/integration/first-vertical/src/lib.rs:64-65 | REAL (disclosed legacy, test-only) | the shipped CLI host forbids this composition for seek (apps/cli/src/host.rs:39-45); seek.rs itself never reopens |

## 7. Dead / duplicated / unreachable code
- None dead in this file. `origin: None` at 397 is immediately overwritten by the caller (session.rs:2070) — slightly awkward API (the field exists only for the caller to fill), not dead.
- `not_seekable()` is a one-line constructor exported for session.rs's policy fact — fine.

## 8. Dependencies & flags
media_clock (NANO_BASE, PlaybackClock), media_types (time types, EngineError), commands (RequestId), crate::pipeline. No feature gates; all paths compiled in the release lane (pub API used by apps/cli and apps/web).

## 9. Open questions for the next agent
- F-001 reachability: can any shipped demuxer (crates/demux/src — mp4/mod.rs:902, mkv/mod.rs:306 build spans from parsed data) emit a track span end that is negative? If yes, this is S1 (user-file-triggered panic in `playback.seek`); the demux-auditing agent should trace span-end computation for mp4 edit lists and mkv timecodes.
- The `flush_budget`/`hold_cap` values (session.rs:2040-2042) come from TickBudget — confirm the soak manifests' seeks_per_reopen storm keeps the preroll hold bounded under the pinned budget (soak runner agent).
