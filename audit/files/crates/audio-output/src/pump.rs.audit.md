# AUDIT: crates/audio-output/src/pump.rs
| Field | Value |
|---|---|
| Auditor | W08-A04 |
| Date | 2026-11-23 |
| Lines / functions | 338 / 12 production fns + 4 tests + 2 helpers |
| File verdict | REAL-UNPROVEN as product code — real, tested, but UNREACHABLE from any shipped entry point (test-only proof for the ADR-005 seam) |
| Reachable from shipped entry point? | NO. `PcmPump` is referenced only by crates/audio-output itself (src/error.rs docs, src/lib.rs re-export, tests/pcm_pipeline.rs). rg over apps/ finds zero uses. The shipped "pump" is a different, hand-rolled one: apps/cli/src/host.rs `CliHost::pump/pump_wallclock` writes the sink directly via `AudioLimb` (pipeline_host.rs) with its own park fill (`AUDIO_PARK_FILL`), and apps/desktop/src/shell.rs:280 `pump_session` drives it from the GUI ticker |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `cargo test -p audio-output` → tests/pcm_pipeline 4/4 PASS (exit 0); rg PcmPump usage; sed quote verification |

## 1. One-paragraph verdict
The code is genuine and its tests genuinely pass (I ran the 4-test
pcm_pipeline suite: bit-exact sine delivery, drop ledger, typed
backpressure, cpal marker refusal) — but nothing shipped uses it. The
answer to "who pumps": nobody, in production. The CLI and desktop
lanes bypass `PcmPump` entirely; their pump is `CliHost`'s own loop on
the host ticker, and the decode-side pacing is `AUDIO_PARK_FILL` in
pipeline_host.rs, a second, parallel implementation of the same
bounded-queue idea. So this file is a well-tested prototype whose
invariants are nevertheless wrong for the environment it was built for
(a real device callback thread): it treats silence consumption as
clock corruption (F-001 family) and contains one raw `u64` subtraction
that can underflow the moment the consumer advances between the guard
and the outcome (F-001).

## 2. What actually works (verified)
— verified by `cargo test -p audio-output` (exit 0):
`clock_synced_pump_delivers_the_exact_sine_stream`,
`drop_policy_bounds_memory_and_keeps_the_prefix`,
`backpressure_is_typed_and_clears_as_the_clock_advances`,
`the_cpal_seam_refuses_with_the_typed_marker` — all pass; plus the 4
in-file unit tests (high-water, partial-frame, foreign-clock, policy).

## 3. Findings
F-001 [S2]
Where: crates/audio-output/src/pump.rs:196-201 and 223.
Code:
```
        let consumed = self.consumed();
        if consumed > self.written {
            // The clock advanced past this pump's writes: it is not
            // this session's truth source.
            return Err(error::unsupported("audio.pump_clock_mismatch"));
        }
```
and line 223:
```
            in_flight: self.written - self.consumed(),
```
What is wrong: two symptoms, one root cause — the pump's clock
(`SampleCounterSource`) counts DATA + SILENCE (render.rs:106-108
advances by `frames_requested`; the null tests assert counter=3 after
1 data frame), while `self.written` counts data only. (a) On any real
device that renders an underrun quantum of silence, `consumed` passes
`written` and the pump diagnoses its OWN session's clock as foreign
(`audio.pump_clock_mismatch` is a wiring error per the doc at
179-181: "refuse the wiring, never pace on a lie") — the pump bricks
on the first underrun, the most normal event in real audio. (b) The
raw subtraction at 223 (not the `saturating_sub` of `in_flight`, whose
doc at 143-145 claims "saturating by construction — see the mismatch
guard") underflows if the consumer advances between the guard (line
196) and the outcome (line 223) — which a concurrent device callback
(the SPSC consumer this file is designed for, module docs line 14-15)
can always do. Debug build: panic; release: `in_flight` wraps to a
u64 near 2^64, so every subsequent `fits()` is false forever
(backpressure livelock) or the drop path fires. Today the tests drive
device and pump on one thread with sized renders, so neither symptom
fires — the defects are exactly what ADR-005 would inherit at flip.
What correct looks like: track data-consumed separately from silence
(the counter already knows `silence_filled` per render — accumulate
it), or pace on ring occupancy (`sink.buffered_frames()`), and use
`saturating_sub` at 223. Confidence: HIGH — Verification: read +
executed the passing tests (which cannot reach the racy window —
single-threaded, sized renders).
F-002 [S3]
Where: crates/audio-output/src/pump.rs:1-4 and 84-92 vs the shipped
lanes.
Code:
```
//! The PCM pump: decoded frames → the bounded device queue → the
//! sink, paced by the playback clock (wave 14, issue #1 — the PCM
//! pipeline proof behind the ADR-005 seam; std-only, no `unsafe`, no
//! new dependencies).
```
What is wrong: the crate's story ("the PcmPump integration", lib.rs
docs 47-51) presents this as the engine-side pump; the shipped engine
sides (CLI host, desktop shell) re-implement pacing independently
(AUDIO_PARK_FILL ring parking + wall anchor, host.rs/pipeline_host.rs)
and never instantiate `PcmPump`. Two bounded-queue policies now exist
with different semantics (park-on-fill vs high-water/policy); the
tested one is the one nobody runs. Why it matters: divergence — the
tested invariants (drop ledger, exact prefix) do not govern the
shipped path, and fixes to one will miss the other. Confidence: HIGH —
Verification: rg (zero PcmPump refs outside crates/audio-output).
F-003 [S4]
Where: crates/audio-output/src/pump.rs:35-38.
Code:
```
//! The pump runs on the **write side** (it may compute, allocate for
//! its own bookkeeping is still avoided — D-1: the push path moves
//! only integers and a `Copy` outcome); it never touches the render
//! callback, which stays [`crate::render::render_into`] alone.
```
What is wrong: garbled sentence ("it may compute, allocate for its own
bookkeeping is still avoided"); the D-1 claim is true for the pump but
the doc is unreadable where it states it. Confidence: HIGH —
Verification: read.
F-004 [S4]
Where: crates/audio-output/src/pump.rs:187.
Code:
```
        let frame_bytes = buf.format.frame_bytes().max(1) as usize;
```
What is wrong: the `.max(1)` silently converts a degenerate format
(channels=0 → frame_bytes 0) into "every byte is a frame" instead of
refusing the format; the frames count then includes garbage until the
sink's own format check rejects. Harmless today (sink refuses), but the
pump validates shape for partial-frame purposes while laundering the
zero case. `AudioFormat::frames_in` (media-types/src/frame.rs:76-79)
does this correctly (`frame == 0 → None`) — the pump does not use it.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DropPolicy (enum) | 58-69 | REAL | — | two honest policies |
| PushOutcome (struct) | 74-82 | REAL | — | accepted/dropped/in_flight |
| PcmPump::Debug | 94-106 | REAL | — | clock presence only |
| PcmPump::new | 116-128 | REAL | — | typed zero-budget refusal |
| PcmPump::attach_clock | 135-137 | REAL | — | |
| PcmPump::consumed (private) | 139-141 | REAL | — | 0 when no clock = "device consumes nothing" |
| PcmPump::in_flight | 146-148 | REAL | F-001 | doc claims saturation "by construction" — false at 223 |
| PcmPump::fits | 152-154 | REAL | — | ready test |
| PcmPump::written_frames | 157-159 | REAL | — | |
| PcmPump::dropped_frames | 163-165 | REAL | — | loss ledger |
| PcmPump::high_water | 168-170 | REAL | — | |
| PcmPump::push | 186-225 | REAL | F-001, F-004 | guard + raw subtraction; policy branch exact |
| PcmPump::sink / sink_mut / into_sink | 228-240 | REAL | — | seam accessors |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| zero_high_water_is_refused_at_construction | YES | error code pinned |
| partial_frames_are_refused_before_the_sink_is_touched | YES | written stays 0 |
| a_foreign_clock_is_a_typed_mismatch | YES | with_base(100) counter → typed mismatch (but cannot distinguish foreign from same-session underrun — see F-001) |
| drop_policy_counts_and_backpressure_is_typed | YES | (0,2,4) outcome + kind pinned |
| pcm_pipeline 4 tests (external) | YES | bit-exact oracle, drop ledger, retry-after-consume, marker refusal |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "exactly one producer (this), exactly one consumer (the device callback)" | pump.rs:14-15 | REAL-UNPROVEN | true in tests (same thread); a real concurrent callback is untested and breaks F-001 |
| "in_flight = written − consumed … bounded by media time" | pump.rs:16-21 | SHALLOW | holds only while no silence is consumed (render.rs:106-108) |
| "Nothing is ever silently discarded or repeated" | pump.rs:29-31 | REAL | drops counted; prefix in-order (drop test) |
| "wave-14 PCM pipeline proof … proven in tests/pcm_pipeline.rs" | lib.rs:47-51, matrix row platform-aout-cpal evidence | REAL as proof, UNREACHABLE as product | tests pass (ran); zero shipped callers |
| "saturating by construction — see the mismatch guard" | pump.rs:143-145 | FALSE at line 223 | raw `-` after an unrelated earlier snapshot |

## 7. Dead / duplicated / unreachable code
The entire type is production-dead (F-002): only tests construct it.
`crate::error::pump_backpressure` exists solely for this file.

## 8. Dependencies & flags
None beyond crate siblings; std-only; no unsafe; no feature gate — it
compiles into every audio-output build, including the release lane,
where it is never called.

## 9. Open questions for the next agent
- If ADR-005 ever flips, will the CLI host adopt PcmPump or keep
AUDIO_PARK_FILL? Whoever owns apps/cli/src/pipeline_host.rs should
reconcile the two pacing implementations before the flip.
- Does `media_clock::PlaybackClock` (imported by pcm_pipeline.rs)
provide a data-only counter that would fix F-001 cleanly?
