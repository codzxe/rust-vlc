# AUDIT: crates/audio-output/src/ring.rs
| Field | Value |
|---|---|
| Auditor | W08-A02 |
| Date | 2026-11-15 |
| Lines / functions | 319 / 8 (+6 tests) |
| File verdict | REAL (Wave-7 claim confirmed: this is a real SPSC ring; pow2 enforced, mask/write semantics correct; 6/6 tests executed fresh) |
| Reachable from shipped entry point? | YES — apps/cli/src/pipeline_host.rs:1598-1599 `NullBackend::open_stream(&format, RING_FRAMES, ...)` → null.rs:85 `SpscRing::new(format.channels, buffer_frames)?`; channels comes from demux probe facts (apps/cli/src/host.rs:914-926) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `cargo test -p audio-output --lib ring::` → 6 passed / 0 failed (fresh run, 0.00s); sed -n quote verification (4 ranges); rg for SpscRing callers, thread::spawn in crate (zero hits), MAX_CHANNELS bounds in demux/codecs |

## 1. One-paragraph verdict
Wave 7's claim is TRUE: this is a real, correctly-built power-of-two SPSC ring — capacity validated as a power of two with a typed refusal, `mask = frames - 1`, monotonic cursors, correct two-segment wrap loops, proper Relaxed-slot / Release-publish / Acquire-load pairing, and whole-frame alignment enforced on both sides. I ran its 6 unit tests fresh: all pass, and by mutation reasoning each would fail if the corresponding code were broken (pow2 check deleted → `SpscRing::new(2,7)` succeeds → shape test fails; mask off-by-one → index panic or conservation failure; wrap accounting wrong → `long_stream_wrap_conservation` fails). The two hostile-focus items check out. What is missing: the S in SPSC is never exercised — no test in the entire crate spawns a thread (rg `thread::spawn|JoinHandle|scope(` over crates/audio-output → 0 hits), and the shipped CLI drives producer and consumer on the SAME thread (apps/cli/src/host.rs:614,726 — `device.render` on the host thread), so the wait-free cross-thread property is correct-by-inspection with zero runtime evidence anywhere.

## 2. What actually works (verified)
— `cargo test -p audio-output --lib ring::` → `test result: ok. 6 passed; 0 failed` (includes shape validation, FIFO roundtrip, full-ring refusal + wrap segments=2, short read, unaligned refusal, 100-iteration wrap conservation).
— Power-of-two math verified by reading + test: `!buffer_frames.is_power_of_two()` refusal (line 71), `mask = buffer_frames - 1` (line 81), `capacity_frames = mask + 1` (line 95).

## 3. Findings
F-001 [S3]
Where: crates/audio-output/src/ring.rs:70-82
Code:
```
    pub fn new(channels: u32, buffer_frames: usize) -> Result<SpscRing, EngineError> {
        if channels == 0 || buffer_frames == 0 || !buffer_frames.is_power_of_two() {
            return Err(error::unsupported("audio.ring_shape_invalid"));
        }
        let slots = buffer_frames
            .checked_mul(channels as usize)
            .ok_or_else(|| error::unsupported("audio.ring_shape_invalid"))?;
```
What is wrong: the only allocation bound is usize multiplication overflow; there is no ceiling on `slots`. The module positions the ring as "the last line of defense" (line 117-118), but a caller passing `channels = u32::MAX, buffer_frames = 2` (1.7e10 slots ≈ 68 GB) passes validation and `Vec::with_capacity` aborts the process (`handle_alloc_error`), it does not return the typed error. Why it matters: abort-on-open from parameters is exactly the D-3 "bounded memory" discipline this crate claims; today's shipped callers are bounded (demux WAV `MAX_CHANNELS = 64` at crates/demux/src/wav/format.rs:41,358; vorbis codec `MAX_MAPPED_CHANNELS = 8` at crates/codecs/src/audio/vorbis/decoder.rs:119,231; CLI `RING_FRAMES = 131_072` at apps/cli/src/pipeline_host.rs:118 → worst case 64×131072×4 B ≈ 33.5 MB), so no live OOM path exists — but the defense the docs advertise is not in the code. What correct looks like: a slots ceiling (e.g. `slots > 1<<24 → audio.ring_shape_invalid`). Confidence: HIGH — Verification: read + caller bounds traced.
F-002 [S3]
Where: crates/audio-output/src/ring.rs (whole file) + crates/audio-output/ (rg evidence)
Code:
```
//! - Two monotonic cursors (`write`, `read`, in frames): the producer
//!   is the only writer of `write`, the consumer the only writer of
//!   `read` — the SPSC discipline.
```
( rg `thread::spawn|JoinHandle|scope(` in crates/audio-output → no matches; CLI drives `device.render` on the host thread: apps/cli/src/host.rs:614. )
What is wrong: the SPSC/wait-free property — the file's entire reason for using atomics — is never exercised with two threads, in tests or in the shipped binary (null.rs documents "The v0 thread model is manual driving on the caller's thread"). A regression downgrading every ordering to `Relaxed` would pass all 33 tests in the crate. The pattern is correct by inspection, but the concurrency claim has zero runtime evidence. What correct looks like: one threaded producer/consumer stress test (or a loom-style interleaving test) in the crate's suite. Confidence: HIGH — Verification: read + rg.
F-003 [S4]
Where: crates/audio-output/src/ring.rs:113-119
Code:
```
    pub fn push(&self, data: &[f32]) -> usize {
        let channels = self.channels;
        if !data.len().is_multiple_of(channels) {
            // A whole-frames contract violation: refuse rather than
            // tear a frame. (Callers validate and type this; the ring
            // itself is the last line of defense.)
            return 0;
```
What is wrong: `push` returns 0 for BOTH "ring full" and "unaligned input" — a contract violation is indistinguishable from backpressure. Documented, and the only production caller (`NullSink::write_bytes`, null.rs:184-189) pre-checks alignment with a typed `audio.partial_frame` error, so not live. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SpscRing::new | 70-86 | REAL | S3 | pow2 + zero checks + overflow check; no slots ceiling (F-001) |
| channels | 89-91 | REAL | — | accessor |
| capacity_frames | 94-96 | REAL | — | mask+1, exact |
| available_frames | 100-102 | REAL | — | Acquire both cursors; monotonic difference |
| free_frames | 105-107 | REAL | — | capacity − available; safe under SPSC |
| push | 113-147 | REAL | S4 | correct segment loop (≤2 for count ≤ capacity), Relaxed slots + Release cursor publish only when count>0; F-003 conflation |
| read | 154-190 | REAL | — | mirrors push; segments ledger; Release publish of read cursor |
| RingRead (struct/Default) | 37-43 | REAL | — | data carrier |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| shape_validation_is_typed | YES | deletes pow2/zero checks → `new(2,7)`/`new(0,8)` succeed → assert fails |
| push_read_roundtrip_in_fifo_order | YES | wrong mask/index math → wrong data or panic |
| full_ring_refuses_without_blocking_or_tearing | YES | overwrite-on-full or tear → out[..4] mismatch; segments=2 asserts wrap splitting |
| short_read_reports_what_was_available | YES | over-read past available → NAN-tail assert fails |
| unaligned_buffers_are_refused_whole | YES | partial-frame write → returned count ≠ 0 |
| long_stream_wrap_conservation | YES | 100 iterations × 6-frame push into capacity 8 with partial pushes; any frame loss/dup/order break fails conservation + FIFO asserts |
All 6 executed fresh: 6 passed. Gap: none of them, and nothing else in the crate, runs push and read on separate threads (F-002).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the SPSC ring's wait-free index swap is the only synchronization on the callback path" | ring.rs:1-3 quoting doc 03 §4.9c; actual text docs/03-engine-plan.md:646 "any lock except the SPSC ring's wait-free index swap" | REAL-UNPROVEN | code matches (atomics only, no locks — read); never exercised cross-thread anywhere (F-002) |
| "The ring never allocates after construction (open time), never blocks" | ring.rs:23-24 | TRUE | push/read touch only atomics and slices (read) |
| "never distinguishes 'empty' from 'full' by luck: available = write − read" | ring.rs:24-25 | TRUE | lines 100-107 verified |
| Wave 7: "ring.rs is the real ring" | audit wave-7 summary | TRUE | this audit: pow2 enforced, semantics correct, tests executed |

## 7. Dead / duplicated / unreachable code
None. Every function is called (render.rs, null.rs, tests, CLI via NullBackend).

## 8. Dependencies & flags
std only (`std::sync::atomic`), `media_types::EngineError`, `crate::error`, dev-dep `testkit`. No feature gates. Bit-cast through `AtomicU32` is exact (f32::to_bits/from_bits are bijective).

## 9. Open questions for the next agent
- Does any wired demux path deliver probe-fact `channels` to `open_audio_session` WITHOUT a codec-side bound (ogg vorbis ID-chain harvest at demux level)? If yes, F-001 upgrades from defense-gap to live abort. (apps/cli/src/host.rs:914-926 `audio_geometry()` path; demux/ogg/mod.rs:247-258 `identify` does not read channels — who harvests it?)
- callback_budget.rs (W08-A08): does it assert the "≤ 2 segments per callback" claim the RingRead ledger exists for?
