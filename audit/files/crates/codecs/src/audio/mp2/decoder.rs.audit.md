# AUDIT: crates/codecs/src/audio/mp2/decoder.rs
| Field | Value |
|---|---|
| Auditor | W07-A10 |
| Date | 2026-11-23 |
| Lines / functions | 678 / 8 prod fns (decode_one, decode_stream, new, accepts, configure, decode, flush, pooled_sample_slot) + Debug::fmt; 7 tests + 3 helpers |
| File verdict | REAL (symphonia-backed Layer II decoder; unit + real-corpus ffmpeg-differential tests executed, pass) |
| Reachable from shipped entry point? | YES — apps/cli/src/pipeline_host.rs:267 `"mp2" => { let mut decoder = Mp2Decoder::new(); decoder.configure(config)?; }` in the CLI decoder factory; codecs default features include `codec-mp2` (crates/codecs/Cargo.toml:47-58) so the release lane compiles it. PERF-VS-VLC.md:140 records real MP2 corpus decode at 105 655 092 B/s. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `cargo test -p codecs --lib mp2 --offline` -> 27 passed / 0 failed; `cargo test -p codecs --test mp2_corpus_differential --offline` -> 3 passed / 0 failed (ffmpeg oracle present); sed verifications |

## 1. One-paragraph verdict
A real, working Layer II decoder built on the pinned `symphonia-bundle-mp3` 0.6.1 engine with an honest architecture: typed truncation/layer/corruption gates, sample-accurate PTS bookkeeping, pooled output slots with exclusive-ownership checks, no unsafe, and a genuine ffmpeg-differential corpus test with pinned PCM MD5s that I ran and that passes. The stream pipeline (`decode_stream`) enforces CBR geometry (version/bitrate/rate/channel-mode agreement across frames, header.rs:268-283), so it refuses inconsistent streams. The one behavioral hole is on the packet path: `Mp2Decoder::decode` freezes `out_channels`/`out_rate` from the first decoded frame and performs no per-packet consistency check, so a mid-stream channel/rate change in fed packets — which `decode_stream` refuses typed — here silently emits wrong-format audio (stale scratch bytes or half a frame) with wrong PTS scaling instead of a typed error. Smaller issues: `Emitted { frames: 1 }` is returned even when zero samples were emitted; `packet_index` is write-only bookkeeping; the 1152-samples-per-frame constant is an unasserted invariant that would panic `SharedBytes::slice` if the engine ever changed.

## 2. What actually works (verified)
— `cargo test -p codecs --lib mp2 --offline` -> `test result: ok. 27 passed; 0 failed` (decoder + header + seek + fuzz-survival unit tests).
— `cargo test -p codecs --test mp2_corpus_differential --offline` -> `corpus_and_fixture_differential_vs_ffmpeg ... ok` (3 passed): real corpus MP2s (rows 0005/0119 with pinned ffmpeg MD5s `fe0d2097…`, `d5c54678…`) decoded by both Lumen and ffmpeg, equal-length, per-sample delta, whole-decode MD5.
— Verified safe by read: `parse_frame_header` typed-errors on <4 bytes (mp3/header.rs:243-260); `walk_frames` is bounds-bounded (`while pos + 4 <= data.len()`, `pos.saturating_add`, truncated tail yields no frame — mp3/header.rs:424-450); the ID3 path (`parse_id3v2` → `id3v2_len`) validates ≥10 header bytes and `total ≤ data.len()` before any slicing (mp3/id3.rs:189-241), so `decode_stream`'s ID3 skip cannot panic on short/truncated tags.
— `pooled_sample_slot` (414-452): replacement only drops the local handle; exclusivity via `get_mut` — no aliased mutation of in-flight frames.

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/audio/mp2/decoder.rs:346-353 (and the same freeze at 188-193 in `decode_stream`, where it is safe because `geometry()` gates it)
Code (346-353):
```
        if self.out_channels == 0 {
            let decoded = engine.last_decoded();
            let spec = decoded.spec();
            self.out_channels = spec.channels().count() as u32;
            self.out_rate = spec.rate();
        }
        let channels = self.out_channels as usize;
        let interleaved = &self.scratch[..frame_samples * channels];
```
What is wrong: the packet path freezes output geometry from the first decoded frame and never re-checks. `decode_one` sizes/copies `scratch` with the *current* frame's channel count (lines 106-112), but `decode` slices with the *first* frame's count. A mono-after-stereo packet emits a frame whose second half is stale bytes from the previous frame (scratch never shrinks); a stereo-after-mono packet emits half the decoded data still labeled stereo. A mid-stream rate change keeps PTS arithmetic (`samples_fed` in `TimeBase::new(1, self.out_rate)`, line 372-386) on the wrong rate. The stream pipeline refuses exactly this input typed (`mp2.stream.cbr_mismatch`, header.rs:268-283), so the module's own standard is that geometry disagreement is a typed refusal — the packet path silently violates it. No panic occurs (Layer II is always 1152 samples and scratch only grows). Why it matters: a spliced or corrupted raw stream fed through a demuxer that frames packets independently produces wrong audio with no error, contradicting the damage-posture. What correct looks like: compare each packet's parsed header geometry (already available — `decode_one` returns the header and discards it as `_header`) against the frozen shape; disagree → typed `mp2.stream.cbr_mismatch`. Confidence: HIGH — Verification: read (data-flow analysis; no execution of a channel-switching packet stream was possible without a fixture generator).
F-002 [S4]
Where: crates/codecs/src/audio/mp2/decoder.rs:356-399
Code (396-399):
```
            out.accept(frame)?;
        }
        self.samples_fed += frame_samples as u64;
        Ok(DecodeOutcome::Emitted { frames: 1 })
```
What is wrong: when the engine decodes 0 samples (`emitted_samples == 0`, line 356 guard skips emission), the outcome still claims `Emitted { frames: 1 }` — a frame count for a frame no sink ever received. Unreachable today (Layer II decodes 1152 or errors), but the contract lies if it ever triggers. Confidence: MEDIUM — Verification: read.
F-003 [S4]
Where: crates/codecs/src/audio/mp2/decoder.rs:262-264, 357, 382; crates/media-types/src/packet.rs:170-175
Code (262-264):
```
/// Layer II samples per frame, every MPEG version (the 5.A3 bound
/// comment holds 1152 across the family).
const MAX_SAMPLES_PER_FRAME: usize = 1152;
```
What is wrong: `slot_len = MAX_SAMPLES_PER_FRAME * channels * 4` sizes the pooled slot, and `bytes.slice(0..exact)` (line 382) with `exact = emitted_samples * channels * 4` hits `SharedBytes::slice`'s `assert!` (packet.rs:171-175) if the engine ever returns >1152 samples for a Layer II frame. The invariant is spec-true and the engine is pinned (=0.6.1), so this is not triggerable today — but it is an unasserted cross-crate assumption; a `debug_assert!(frame_samples <= MAX_SAMPLES_PER_FRAME)` after `decode_one` would turn a future engine change into a test failure instead of a decode-thread panic. Confidence: HIGH — Verification: read (assert semantics verified in packet.rs).
F-004 [S4]
Where: crates/codecs/src/audio/mp2/decoder.rs:235-236, 253, 316, 354
Code (235-236):
```
    /// Packets fed so far (0-based index of the next).
    packet_index: u32,
```
What is wrong: `packet_index` is incremented (line 354) and reset (line 316) but never read except in the `Debug` impl — dead bookkeeping state. Confidence: HIGH — Verification: read (rg for `packet_index` shows only fmt/increment/reset).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `decode_one` | 74-114 | REAL | — | Header gate + typed truncation check + engine decode + exact scratch copy; discards the parsed header (`_header`) that could gate F-001 |
| `decode_stream` | 126-217 | REAL | — | ID3 skip (safe, validated upstream), walk, geometry gate, decode all; first-frame spec freeze is guarded by `geometry()` so safe here |
| `Mp2Decoder::new` | 268-271 | REAL | — | Default |
| `accepts` | 274-276 | REAL | — | Exactly "mp2"; pinned by test |
| `configure` | 280-323 | REAL | — | Descriptor gate, extra-data refusal (honest Layer II no-gapless posture), engine init, full state reset |
| `decode` | 325-400 | REAL | S3 | Frozen-geometry hole (F-001), outcome lie (F-002), slot assert coupling (F-003); otherwise: typed BadState, stale-generation drop, exact PTS/duration |
| `flush` | 402-408 | REAL | — | Honest: Layer II has no cross-frame state → Drained |
| `pooled_sample_slot` | 414-452 | REAL | — | Exclusive-ownership pooling; len-mismatch skip; replacement drops only the local handle |
| `Debug::fmt` | 250-257 | REAL | — | finish_non_exhaustive |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| descriptor_gate_is_exactly_mp2 | YES | Positive + 4 negative descriptors incl. case sensitivity |
| decode_before_configure_is_typed_bad_state | YES | Error code + kind asserted |
| configure_rejects_foreign_descriptors_and_unexpected_facts | YES | Both refusals asserted with codes |
| a_minimal_synthetic_stream_decodes_end_to_end | YES | Shape (1152/16 kHz/mono/720-byte frame) pinned on a synthetic frame |
| a_layer_iii_stream_is_refused_not_misdecoded | YES | Layer gate asserted (`mp2.stream.layer_mismatch`) |
| a_truncated_frame_packet_is_a_typed_truncation | YES | Truncation detail (expected 720/found 100) + per-frame isolation follow-up decode |
| cbr_seek_plan_from_the_decoded_geometry_lands_on_frames | YES | decode_stream ∘ cbr_seek composition on 3-frame stream |
| (external) mp2_corpus_differential.rs | YES | Real corpus + committed fixtures, ffmpeg as oracle, pinned MD5s fail loudly on corpus re-imaging — ran: 3 passed |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "One packet = one complete frame … per-frame isolation holds: after a typed Corrupt the next packet decodes unchanged" | decoder.rs:20-24 | REAL | Truncation test proves isolation; but geometry disagreement (not Corrupt) silently mis-decodes (F-001) — the claim covers damage, not shape drift |
| "ffmpeg's Layer II decode emits exactly frames × 1152 samples and so does this module — the differential is sample-locked" | decoder.rs:31-34 | REAL | Ran `mp2_corpus_differential`: 3 passed vs ffmpeg 7.1.5 oracle with pinned MD5s |
| "No gapless facts — … extra_data accepts nothing: a non-empty blob is a typed refusal" | decoder.rs:26-31 | REAL | configure test asserts `mp2.facts.unexpected` |
| "Damage is never a panic; garbage between frames is resynced past" | decoder.rs:121-125 | REAL | walk_frames resync verified; ID3 validation verified; no panic path found in decode_stream |

## 7. Dead / duplicated / unreachable code
- `packet_index` field: write-only (F-004).
- `decode_one`'s returned `Mp2FrameHeader` is discarded at both call sites (`_header`, lines 187, 344) — the data needed to close F-001 is parsed and thrown away.
- No unreachable code otherwise; module is feature-gated `codec-mp2` which is in the default set (shipped).

## 8. Dependencies & flags
- `symphonia-bundle-mp3 =0.6.1` (pinned, Lead-admitted per docs/dependencies/records/symphonia-mp3-0.6.1.md), `symphonia-core =0.6.1` — optional behind `codec-mp2` → `codec-mp3` (codecs/Cargo.toml:78-79); default feature set includes both, release lane ships them. `Layer II rides the bundle-mp3 family engine AND mp3's sync/header glue` (Cargo.toml:75-77) — mp2 shares mp3's walk/ID3, not duplicated.

## 9. Open questions for the next agent
- Baseline engine-info lists `decoders: mjpeg, pcm_s16le` while the CLI factory (pipeline_host.rs:246-300) dispatches flac/mp3/mp2/vorbis/opus/aac/alac/adpcm and the HEVC error text claims "landed: … mp2 …" — which registry does engine-info read, and why does it disagree with the factory? (Spec §13 open trace.)
- F-001's exploitability depends on whether any shipped demuxer frames raw MP2 packets from a byte stream without a geometry gate — the MP2 demuxer/registry row was "future" per header.rs:299-301; whoever audits that demuxer should check.
