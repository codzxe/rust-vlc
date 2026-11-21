# AUDIT: crates/codecs/src/audio/vorbis/decoder.rs#part1 (lines 1-646)
| Field | Value |
|---|---|
| Auditor | W07-A07 |
| Date | 2026-11-21 |
| Lines / functions | 1-646 of 1140 / 14 items (2 structs, 1 result struct, 4 consts, 7 fns/methods) |
| Chunk boundary | part1 = 1-646 (module docs, consts, `PendingBlock`/`HeadHold`/`DecodedVorbis`, `build_engine`, `decode_block_into`, `decode_block`, `max_output_len`, `slot_samples`, `VorbisDecoder` state + `new`, `release_head_hold`, `drain_head_hold`). Boundary drawn at the END of `drain_head_hold` (646) rather than the raw half-point 570 — splitting a function mid-body would orphan its invariants. part2 (647-1140: `accepts`, accessors, `Decoder::configure/decode/flush`, `PooledBlock`, `pooled_sample_slot`, unit tests) belongs to W07-A08; I read it for context only and attribute its findings there. |
| File verdict | REAL-UNPROVEN (shipped lane) — REAL for the tested path |
| Reachable from shipped entry point? | YES (call chain: `lumen open` → apps/cli/src/pipeline_host.rs:271-275 `HostDecoderProvider::decoder_for` `"vorbis" => self.seeded(track, config, VorbisDecoder::new)` → codec-api `Decoder::decode`/`flush`; vorbis tracks minted by crates/demux/src/ogg/mod.rs:155 `OggCodec::Vorbis => "vorbis"` and mkv). No baseline smoke run opens a Vorbis file (baseline samples: MP4 variants + AVI only), so shipped-lane execution is UNPROVEN; the full demux→decode path IS executed by crates/codecs/tests/vorbis_bitexact.rs in baseline run #3 (1681 passed). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "impl DecoderProvider"` → apps/cli/src/pipeline_host.rs:235 · `sed -n '120,320p' apps/cli/src/pipeline_host.rs` (vorbis dispatch + seeded lane) · `rg -n "OggCodec::Vorbis|A_VORBIS" crates/demux/src` (track emission) · `sed -n '300,409p' crates/codecs/tests/vorbis_bitexact.rs` (headtrim regression) · `rg -n "hold_cap|PrerollHoldSlot|is_full" crates/playback-engine/src/pipeline.rs` + `sed -n '2646,2714p'` (the 16-frame pre-roll hold cap that refuses bursts) · `rg -n "release_head_hold|drain_head_hold"` (call sites: decode:900 give-up None, flush:987 None — the trimmed path never calls release_head_hold) · baseline INDEX.md #3 |

## 1. One-paragraph verdict
The head-hold machinery in this half is genuinely engineered and regression-pinned: the 6.FIX-vorbis-headtrim defect (128-sample encoder delay riding the ring un-trimmed) has a real test asserting exact emission counts against ffmpeg-derived reference sections, and the issue-#38 amortization (4 frames per call under the pipeline's 16-frame pre-roll hold cap) is correctly implemented for the trimmed release. The engine glue (`build_engine`, `decode_block_into`) is honest: typed refusals, overlap-state reset on poison, pooled-slot sizing bounded by the engine-validated block size. Three defects: the give-up release path (stamp-less head past 120 000 samples) still bursts in ONE un-amortized call — the exact burst shape issue #38 fixed for the trimmed path — and after a seek landing (where the pipeline's pre-roll hold IS armed and hard-capped at 16) the excess frames are refused and dropped; `release_head_hold` swallows the hold on an impossible-state error instead of restoring it; and `release_head_hold`'s trimmed-pts arm is dead code that would double-count tail padding if a future caller ever used it (the frozen-shift trap `drain_head_hold` explicitly guards against).

## 2. What actually works (verified)
- Head-trim regression executes in baseline — `audit/runs/baseline/INDEX.md` #3 (`cargo test --workspace` 1681 passed / 1 failed, failure unrelated); the test `stream_head_encoder_delay_is_trimmed_decoder-side` (tests/vorbis_bitexact.rs:339-408) asserts emission == granule exactly, first Present pts == 0, every frame within [0, granule], and the single-data-page un-trimmed pin — pre-fix values (66 278 / 9 856 / 2 902 µs) recorded in-line.
- Reachability — verified by call chain: pipeline_host.rs:271-275 (vorbis dispatch, seeded with the Ogg three-header chain) and demux/src/ogg/mod.rs:155.
- Pre-roll hold cap behavior — verified by reading pipeline.rs:2665-2671 (`push` refuses at `frames.len() >= cap`) and 2688-2704 (refused frames counted `stale_dropped`, never buffered): a single decode call bursting N frames into an armed hold delivers 16 and drops N−16.
- Trimmed-release amortization arithmetic — read: budget 4 divides hold_cap 16; head chunks emitted before the current packet's block, `hold.released` persisted between calls, `release_shift` frozen at trim resolution (lines 556, 562) — the re-stamp trap the comments describe is structurally avoided in `drain_head_hold`.

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/audio/vorbis/decoder.rs:900 (give-up call, part2 line but the burst function is mine: 448-517) and 121-131 (the bound's justification)
Code:
```rust
                let gave_up = self.release_head_hold(out, packet.generation, None)?;
```
with the un-budgeted release loop it calls:
```rust
        while cursor < end_sample {
            let len = (end_sample - cursor).min(slot_frames);
```
What is wrong: `release_head_hold` emits the entire withheld head in ONE call (no `RELEASE_FRAMES_PER_CALL` budget). Issue #38 (documented at lines 133-153) established that a single-call burst crossing the pipeline's pre-roll hold cap (`hold_cap`, product default 16) has the excess **dropped and counted** (`PrerollHoldSlot::push` refuses at capacity — pipeline.rs:2665-2671 — and `LimbSink::accept` counts the refusal as `stale_dropped`, pipeline.rs:2696-2698). The trimmed release was amortized to 4 frames/call to fix exactly this; the give-up path was not. At a plain stream head no seek window is armed (verified: windows open only via `window_open_seek`/`window_arm_switch`), so the burst lands on the ring and the 120 000 < 131 072 sizing holds — but `configure` also re-arms the hold on a seek reanchor (lines 724-730, part2), and a give-up firing while the seek window is armed (a landing into ≥120 000 unstamped samples — hostile/damaged Ogg) bursts up to 120 000/slot_frames ≈ 117 frames: ~101 frames of head audio dropped-and-counted.
Why it matters: the give-up path's own comment (126-131) justifies the 120 000 bound with ring capacity, the wrong constraint — the constraint that actually produced the measured 29 248-sample loss (issue #38) was the hold cap. The same loss mode the fix addressed is still live on one of the three release paths.
What correct looks like: amortize the give-up release through `drain_head_hold`'s budgeted loop too (or at minimum document that seek-landing give-up audio beyond 16 frames is sacrificial), and fix the comment to reason about `hold_cap`, not the ring.
Confidence: MEDIUM — Verification: read (pipeline.rs hold sites read verbatim; no executed run of the seek-landing give-up scenario — would need a crafted >2.7 s unstamped Ogg landing).

### F-002 [S4]
Where: crates/codecs/src/audio/vorbis/decoder.rs:454-459
Code:
```rust
        let Some(hold) = self.head_hold.take() else {
            return Ok(0);
        };
        let Some(headers) = self.headers.as_ref() else {
            return Err(VorbisError::BadState.into_engine_error());
        };
```
What is wrong: the hold is `take`n before the `headers` guard; if the guard ever fires, the hold — up to 120 000 samples of withheld head audio — is silently discarded (never restored, never emitted). `drain_head_hold` (line 536-541) restores the hold on the same guard, so the two halves of the same machinery disagree on the failure shape.
Why it matters: today the state is unreachable (`head_hold = Some` is only set by `configure` after `VorbisHeaders::from_extra_data` succeeded, which also sets `headers = Some`), so nothing is lost — but the pattern is a trap for any refactor that ever clears `headers` independently (e.g. a future `close`/reconfigure seam).
What correct looks like: check `headers` first, or restore the hold in the error arm (mirror `drain_head_hold`).
Confidence: HIGH — Verification: read (invariant traced through `configure`, part2 lines 714-735).

### F-003 [S4]
Where: crates/codecs/src/audio/vorbis/decoder.rs:477, 486-494
Code:
```rust
        let shift = self.shift;
```
```rust
            let pts = match (trim_from, shift) {
                (Some(_), Some(shift)) => Stamp::Present(Timestamp::new(
                    i64::try_from(cursor)
                        .unwrap_or(i64::MAX)
                        .saturating_sub(shift),
                    base,
                )),
                _ => Stamp::Missing,
            };
```
What is wrong: `release_head_hold` stamps a trimmed release with the CURRENT `self.shift`, while `drain_head_hold` deliberately stamps with `hold.release_shift` frozen at trim resolution (line 556, with the comment at 191-195 explaining why: the final page's stamp grows the shift by the tail padding and re-stamping the head with it double-counts). No caller ever passes `trim_from = Some` (verified: the only two call sites are decode:900 and flush:987, both `None`), so the `(Some(_), Some(_))` arm is dead code — but it is exactly the arm a future caller would reach for, and it carries the re-stamp bug the frozen-shift field exists to prevent.
Why it matters: dead-but-wrong code adjacent to the live, carefully-frozen path; the inconsistency invites the regression the design documents as fixed.
What correct looks like: either use `hold.release_shift` (and test it), or delete the arm and assert `trim_from.is_none()` at entry.
Confidence: HIGH — Verification: read + rg for call sites (`release_head_hold` appears only at its definition, decode:900, flush:987).

## 4. Per-function table (part1 items)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-97 | REAL | — | accurate to the code as verified, incl. the measured defects it cites (128 samples/2 902 µs; 29 248 samples) — both pinned by tests/vorbis_bitexact.rs |
| `SAMPLE_SLOTS` | 113-114 | REAL | — | rotation depth 4 |
| `MAX_MAPPED_CHANNELS` | 116-119 | REAL | — | honest gate: symphonia maps 1-8; >8 typed refusal before engine build |
| `HEAD_HOLD_MAX_SAMPLES` | 121-131 | REAL | S3 | the 120 000 < 131 072 ring argument ignores `hold_cap` (F-001) |
| `RELEASE_FRAMES_PER_CALL` | 133-153 | REAL | — | 4 divides 16; sound |
| `PendingBlock` | 161-170 | REAL | — | pts/valid window frozen at decode time — correct shape |
| `HeadHold` | 181-206 | REAL | — | `shared` view minted on first drain, `data` emptied — no copy-back |
| `DecodedVorbis` | 211-222 | REAL | — | free-surface result |
| `build_engine` | 229-258 | REAL | — | channel gate, gapless off with rationale, engine refusal typed; `max_frames_per_packet = blocksize_long/2` is the true max emission (overlap-add `(prev+n)/4 ≤ n_long/2`) |
| `decode_block_into` | 267-307 | REAL | — | poison → `engine.reset()` + typed error (overlap-scoped recovery, one poisoned half-block); scratch reused, grow-only; slot-size typed check; no panics (bounds via `frames*channels*4` ≤ slot) |
| `decode_block` | 318-338 | REAL | — | free surface (fuzz/tests): rebuilds engine per call — documented, not a hot path; `first_sample: 0` documented cold-start |
| `max_output_len` | 343-345 | REAL | — | bounded: blocksize engine-validated at configure (engine try_new rejects illegal blocksizes), channels ≤ 8 gate first → ≤ 8192/2×8×4 = 131 072 bytes |
| `slot_samples` | 351-353 | REAL | — | blocksize_long/2 |
| `VorbisDecoder` state | 390-424 | REAL | — | all state fields documented; `shift: Option<i64>` re-measured per stamp |
| `VorbisDecoder::new` | 428-431 | REAL | — | default |
| `release_head_hold` | 448-517 | REAL | S3/S4 | F-001 (un-budgeted burst), F-002 (hold swallowed on guard), F-003 (dead trimmed-pts arm with latent re-stamp bug); trim clamped to held samples (lying granule cannot cut past emission) ✓; slicing in-bounds (`data.len() == samples × frame_bytes` invariant) ✓ |
| `drain_head_hold` | 530-646 | REAL | — | budgeted; frozen `release_shift` ✓; head chunks before pending FIFO ✓; hold dies exactly when last frame leaves (639-644) ✓; restore-on-guard ✓; pending bytes sliced ≤ slot_len ✓ |

## 5. Test honesty (tests covering part1 live outside it)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `stream_head_encoder_delay_is_trimmed_decoder_side` (tests/vorbis_bitexact.rs:339-408) | YES | exact-count assertions vs reference sections (`total == granule`, `first_present == Some(0)`, per-frame bounds) and the un-trimmed single-page pin (`raw_samples`); pre-fix numbers recorded — mutation of the trim arithmetic changes the totals |
| corruption-recovery test (same file, ~280-329) | YES | exactly 1 typed refusal at the poisoned packet, ≥60 blocks emitted around it — pins `decode_block_into`'s reset-and-continue |
| doctest on `VorbisDecoder` (364-389) | YES | asserts the `vorbis.headers.missing` typed code |
| Gap: NO test exercises the give-up path (stamp-less head ≥ 120 000 samples) — F-001's drop mode is untested | — | would need a hostile fixture |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "status = implemented (decode + comments + granule timeline)" | docs/formats/audio-vorbis.md:7 | REAL (tested lane) / REAL-UNPROVEN (shipped binary lane) | tests/vorbis_bitexact.rs runs in baseline #3; no baseline smoke opens a Vorbis file |
| "Output = f32 interleaved native-endian, no rate/channel conversion" | docs/formats/audio-vorbis.md:24 | REAL | decode_block_into:303-305 `to_ne_bytes` interleave; format stamped with ident rate/channels (part2 lines 929-935) |
| "pts = raw − measured shift on the delay/padding-trimmed timeline; single-data-page streams honestly expose Missing" | docs/formats/audio-vorbis.md:26-27 | REAL | headtrim test (b) pins the all-Missing single-page shape |
| fuzz_target `fuzz_codec_vorbis` "3 surfaces" | docs/formats/audio-vorbis.md:9 | REAL-UNPROVEN | registry pass in baseline #18 (35 targets, diff empty) proves registration, not execution |

## 7. Dead / duplicated / unreachable code
- `release_head_hold`'s `(Some(_), Some(shift))` pts arm (486-492) — dead (F-003).
- Nothing else dead in part1; all paths reachable via configure/decode/flush (part2) which is reachable from the shipped provider.

## 8. Dependencies & flags
- `symphonia-codec-vorbis =0.6.1` exact pin, `codec-vorbis` feature in codecs default set — shipped build compiles it (baseline #1 CLEAN). Admission record docs/dependencies/records/symphonia-vorbis-0.6.1.md (`#![forbid(unsafe_code)]` claim — record's, not verified by me).
- No `unsafe` in this file; no unwrap/expect outside test-visible dead arms (`unwrap_or` saturating forms used on all conversion sites).

## 9. Open questions for the next agent
- W07-A08 (part2): `decode`'s granule tail clamp (822-831) and the pending-block queueing gate (850) are yours; F-001's give-up site (900) sits in your range but the defect is in my range's function — coordinate so it is counted once.
- W07-A04 (vorbis/seek.rs): `measure_shift`'s exact contract (when it returns Some vs None for out-of-order/stale stamps) underpins every shift write here.
- Hostile-question note for the wave: Vorbis output is NATIVE rate, no conversion (module doc line 83-84 "no rate or channel conversion") — sample-rate conversion, if it exists anywhere, is not in the codec layer; the resampler/mixer owner should confirm where the device-rate seam lives.
