# AUDIT: crates/codecs/src/video/vp9/decoder.rs
| Field | Value |
|---|---|
| Auditor | W06-A08 |
| Date | 2025-06-27 |
| Lines / functions | 577 / 8 fns + 4 tests |
| File verdict | REAL — real VP9 decode (rusty_vp9 engine), wrapper verified by executed differential |
| Reachable from shipped entry point? | YES: lumen-cli main → apps/cli/src/pipeline_host.rs:313 `"vp9" => Vp9Decoder::new()` → this Decoder impl → rusty_vp9; codecs default features include `codec-vp9` (crates/codecs/Cargo.toml:43-58,81) and apps/cli/Cargo.toml:51 takes defaults |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 4 |
| Commands run | `./target/debug/deps/vp9_differential-4ec38b32b2dc3e15` -> 4/4 ok incl. `differential_lanes_vs_ffmpeg_libvpx_vp9` (22.03s); `./target/debug/deps/codecs-bae0f1756cc0a809 video::vp9` -> 5/5 ok; rg/sed verification |

## 1. One-paragraph verdict
The hostile-focus question — "real VP9 decode or parser + rav1d?" — answers:
neither parser-only nor rav1d. This is a real decode implementation built on the
admitted pure-Rust engine `rusty_vp9 = "=0.1.3"` (rav1d is the *av1* module's
engine; this module contains zero `unsafe` — verified: both rg hits for
"unsafe" are the doc comment itself). I executed the wrapper's differential
suite: bit-exact lanes against ffmpeg `libvpx-vp9` PASS (my run, 4/4), the
10-bit lane correctly refuses typed, garbage/truncation hostile lanes pass, and
all 4 in-file contract tests pass. The wrapper code is careful: checked plane
geometry before any copy, pooled slots, typed refusals for bitdepth/layout it
cannot name, damage counting that never panics. Real defects: the slot pool
silently degrades to per-frame allocation forever after one resolution change
(scalable VP9 changes resolution by design), the drain loop's termination
relies on an engine consumption contract I could not verify, unchecked `w*h`
arithmetic would overflow 32-bit targets at VP9's max geometry, and a sink
error mid-drain drops already-consumed frames.

## 2. What actually works (verified)
— `./target/debug/deps/vp9_differential-4ec38b32b2dc3e15` -> "4 passed; 0
failed", including `differential_lanes_vs_ffmpeg_libvpx_vp9 ... ok` and
`tenbit_lane_is_a_typed_refusal_not_a_misdecode ... ok` (22.03s, my run).
— `./target/debug/deps/codecs-bae0f1756cc0a809 video::vp9` -> 5/5 ok
(wrong_descriptor, decode_before_configure, garbage, zero_length, error
taxonomy).
— Baseline corroborates: cargo-test-workspace.txt:870-876 and :1384-1393.

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/video/vp9/decoder.rs:404-437
Code:
    let count = slots.len();
    for i in 0..count {
        let idx = (*next + i) % count;
        if slots[idx].len() != len {
            continue;
        }
    ...
    if slots.len() < FRAME_SLOTS {
        slots.push(fresh.clone());
    }
What is wrong: once the pool holds FRAME_SLOTS (4) slots of one frame size, a
stream that changes resolution (VP9 spatial/SVC streams do this by design;
`frame.width/height` is per-frame) never finds a matching slot, and the fresh
allocation is NOT stored (`slots.len() < FRAME_SLOTS` is false) — every frame
from then on allocates a new SharedBytes. The doc at L76-79 promises "without
per-frame allocation in steady state"; after one size change there is no steady
state ever again. What correct looks like: replace an old-len slot (or track
the current geometry and recycle).
Confidence: HIGH — Verification: read (code path is unconditional).

F-002 [S4]
Where: crates/codecs/src/video/vp9/decoder.rs:151-159
Code:
        loop {
            match engine.next_frame() {
                Ok(frame) => frames.push(frame),
                Err(rusty_vp9::Error::Again) if stop_on_again => break,
                Err(rusty_vp9::Error::Again) => continue,
                Err(rusty_vp9::Error::Eof) => break,
                Err(_) => damage += 1,
            }
        }
What is wrong: termination depends entirely on rusty_vp9's contract that every
Err consumed a queued frame and that post-flush Again always eventually yields
Eof. An engine that repeats an error without consuming spins this loop forever
(no iteration cap, no budget). Why it matters: a hostile/corrupt stream would
hang the decode thread instead of failing. Mitigation: the hostile differential
lanes (garbage, truncation sweep) pass, so the contract holds on tested inputs.
What correct looks like: a max-iterations guard. UNVERIFIED against engine
source (rusty_vp9 not in my file list).
Confidence: MEDIUM — Verification: read + ran hostile lanes.

F-003 [S4]
Where: crates/codecs/src/video/vp9/decoder.rs:242
Code:
        let len = w * h + 2 * cw * ch;
What is wrong: unchecked multiply — at VP9's maximum geometry (65536×65536)
`w*h` is 2^32, overflowing on 32-bit targets (panic in debug, wrap in release).
64-bit hosts are safe (max ~6.4e9). The wasm lane disables codec-vp9
(default-features=false in apps/web), so the realistic blast radius is nil
today. What correct looks like: u64/checked math like the stride check 20 lines
above already does.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/codecs/src/video/vp9/decoder.rs:328-336, 261-264
Code:
        if let Stamp::Present(ts) = packet.pts {
            self.base = Some(ts.base);
        }
...
        let pts = match (frame.pts, self.base) {
            (Some(ticks), Some(base)) => Stamp::Present(Timestamp::new(ticks, base)),
            _ => Stamp::Missing,
        };
What is wrong: a mid-stream timeline-base change (discontinuity, new source
segment) re-stamps frames still queued behind an alt-ref with the NEW base —
the ticks rode in under the old one. The doc admits the assumption ("the
timeline base is a per-stream constant in practice") but nothing detects or
flags a violation. Wrong timestamps on at most a few frames; S4 blast radius.
Confidence: MEDIUM — Verification: read.

F-005 [S4]
Where: crates/codecs/src/video/vp9/decoder.rs:347-352
Code:
        let (frames, damage) = self.drain(true);
        let mut emitted = 0usize;
        for frame in &frames {
            self.emit_frame(frame, out, generation)?;
            emitted += 1;
        }
What is wrong: if `out.accept` errors mid-batch, `?` aborts decode and the
remaining drained frames — already consumed from the engine queue — are
dropped with no record. A sink backpressure/error loses frames silently.
Confidence: MEDIUM — Verification: read (no execution of a failing sink).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Vp9Decoder::new | 110-112 | REAL | — | |
| Vp9Decoder::accepts | 116-118 | REAL | — | "vp9"/"VP90" |
| Vp9Decoder::take_damage | 122-133 | REAL | — | folds damage into Recovered, saturating |
| Vp9Decoder::drain | 145-161 | REAL | S4 | termination relies on engine contract (F-002) |
| Vp9Decoder::emit_frame | 166-275 | REAL | — | geometry/bitdepth/layout/plane-bounds gates all typed; copy math proven by the pre-check |
| Decoder::configure | 279-301 | REAL | — | fresh engine per generation; extradata ignored (in-band config, documented) |
| Decoder::decode | 310-362 | REAL | S4 | sink-error frame drop (F-005); stale-gen drop delegated to shared helper |
| Decoder::flush | 364-391 | REAL | — | Eof latch + Drained semantics; garbage test covers |
| pooled_frame_slot | 398-438 | REAL | S3 | permanent per-frame alloc after resolution change (F-001) |
| tests (4) | 489-577 | REAL | — | all executed, pass (§2) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| wrong_descriptor_is_a_typed_refusal | Yes | asserts code + stage |
| decode_before_configure_is_typed_bad_state | Yes | both decode and flush paths |
| garbage_packet_is_recovered_not_fatal | Yes | Recovered vocabulary + flush Drained |
| zero_length_packet_is_recovered_not_fatal | Yes | Recovered vocabulary |
In-file tests cover contract edges only; decode truth lives in
tests/vp9_differential.rs (executed by me, 4/4 — bit-exact vs libvpx-vp9).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "rusty_vp9 = "=0.1.3", default features ... pure Rust, zero dependencies" | L6-9 | REAL-UNPROVEN | Cargo.toml:81 gate + exact pin confirmed; purity is the dependency record's claim (UNVERIFIED here) |
| "bit-exact against ffmpeg libvpx-vp9 on six lanes (431/431 frames)" | L10-12 | REAL | my run of vp9_differential: differential_lanes_vs_ffmpeg_libvpx_vp9 ok |
| "This module contains zero `unsafe`" | L22 | REAL | rg "unsafe" -> only the two doc lines |
| "8-bit 4:2:0 → I420 ... typed refusals" | L46-53 | REAL | tenbit_lane_is_a_typed_refusal_not_a_misdecode ok (my run) |
| "a truncated keyframe may decode to a full frame ... honest Ok" | L41-44 | honest limitation | recorded, not hidden |
| "without per-frame allocation in steady state" | L76-79 | SHALLOW | false after any resolution change (F-001) |

## 7. Dead / duplicated / unreachable code
None. `vp9` / `VP90` descriptor both accepted and demuxers map to them.

## 8. Dependencies & flags
rusty_vp9 "=0.1.3" behind `codec-vp9` (default ON). Reachable in the release
lane via apps/cli pipeline_host.rs:313. apps/web disables it
(default-features=false) — consistent with its size report.

## 9. Open questions for the next agent
- rusty_vp9 0.1.3 source: does next_frame() ever repeat an Err without
  consuming (F-002)? Is push() superframe-splitting correct for SVC streams
  that change resolution (F-001 trigger)?
- The engine's DecodedFrame pts passthrough: does a hidden alt-ref's *successor*
  carry its own coding packet's ticks (module doc claims so)?
