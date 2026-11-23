# AUDIT: crates/codecs/src/audio/mp3/decoder.rs
| Field | Value |
|---|---|
| Auditor | W07-A08 |
| Date | 2026-11-23 |
| Lines / functions | 828 / 13 (+6 tests, 2 test helpers) |
| File verdict | REAL (with S2 wiring/PTS defects on the shipped path) |
| Reachable from shipped entry point? | YES — `lumen open x.mp3` → apps/cli/src/containers/mod.rs:65-66 `es::open_mpeg_audio` → crates/demux/src/es/mp3.rs `RawMp3Demuxer` (track codec `mp3`) → apps/cli/src/pipeline_host.rs:261-265 `HostDecoderProvider::decoder_for` → `Mp3Decoder::configure/decode` (provider wired at apps/cli/src/host.rs:948). Release lane compiles it: audit/runs/baseline/rc-smoke.txt:100 `Compiling symphonia-bundle-mp3 v0.6.1`. |
| Findings | S0: 0 S1: 0 S2: 4 S3: 3 S4: 3 |
| Commands run | `./target/debug/deps/mp3_bitexact-a1d379f06feffc22 --test-threads=2` → 2 passed (live ffmpeg oracle present, exit 0); `./target/debug/deps/mp3_corpus_differential-7377bbfd607a4401` → SKIP "corpus root absent", 2 passed/exit 0; python3 Xing-semantics walk over 7 fixtures + 8 fuzz seeds (all: claimed==audio frames excl. header frame) |

## 1. One-paragraph verdict
Engine: **symphonia, not hand-rolled** — `symphonia-bundle-mp3 = "=0.6.1"` + `symphonia-core = "=0.6.1"` behind feature `codec-mp3`, which `default` enables (crates/codecs/Cargo.toml:47-59, 78, 90, 94). The decode core is real and the differential evidence is real: I executed the committed bit-exact suite against a live ffmpeg oracle and it passed. But the shipped wiring guts two of the module's headline features: the LAME gapless facts blob is never fed by any shipped lane (the demuxer harvest hard-codes `extra_data: None`), so every LAME-tagged MP3 plays its encoder-delay silence, and the packet PTS sequence is internally inconsistent (a 529-sample timeline hole after frame 1 on every decode). The 529-sample Layer III latency trim is applied unconditionally even for Layer I/II descriptors the same gate accepts, contradicting the module's own measured claim that Layer II aligns at shift 0.

## 2. What actually works (verified)
— `./target/debug/deps/mp3_bitexact-a1d379f06feffc22 --test-threads=2` → `test result: ok. 2 passed` (fixtures_decode_the_recorded_window_exactly runs the full walk→Xing→decode→gapless pipeline on 6 real ffmpeg-authored fixtures, MD5-pinned, ±1 LSB vs live ffmpeg; would fail if the window arithmetic, Xing parse, or engine glue broke).
— Python walk of all 7 committed fixtures + 8 fuzz seeds: every Xing/Info `frames` value equals the walked audio-frame count EXCLUDING the header frame — the gate's convention matches ffmpeg-authored files (`claimed==audio:True` on all 15 files).
— Layer I/II/III: `codec_id` maps L1→CODEC_ID_MP1, L2→CODEC_ID_MP2, L3→CODEC_ID_MP3 (105-112); `accepts` takes `mp1|mp2|mp3` (437-439). Free-format bitrate (index 0) is a typed refusal `mp3.free_format` (header.rs:303-311) — honest, documented gap; free-format files do NOT play. All 4 stereo modes parsed (header.rs:346-351), dual channel = 2 channels.

## 3. Findings
F-001 [S2]
Where: decoder.rs:580-589
Code:
```
                pts: if self.samples_fed < u64::from(self.delay) {
                    // Still inside the delay window: the first emitted
                    // sample's true index is the delay boundary.
                    Stamp::Present(Timestamp::new(i64::from(self.delay), base))
                } else {
                    Stamp::Present(Timestamp::new(
                        i64::try_from(self.samples_fed).unwrap_or(i64::MAX),
```
What is wrong: two different timelines in one sequence. Packet 1's pts is on the ffmpeg/content timeline (first emitted sample = raw `delay+529` → content index `delay`), but packets 2+ get `samples_fed` — the RAW pipeline counter that still contains the 529 trimmed priming samples. Concrete no-facts case: frame 1 emits 623 samples (raw 529..1152) at pts 0 with matching duration; frame 2 gets pts 1152 although its first sample is content index 623. A 529-sample (~12 ms @44.1 kHz) hole is punched into every MP3's output timeline. Why it matters: any sink scheduling audio by pts inserts a click/gap and drifts 12 ms against video. What correct looks like: `samples_fed.saturating_sub(LAYER3_PIPELINE_LATENCY)` for the general branch. Confidence: HIGH — Verification: read (arithmetic on the code paths; not executed against the sink).

F-002 [S2]
Where: decoder.rs:29-32 (module doc) vs apps/cli/src/containers/es.rs:153-163
Code (doc):
```
//! `DecoderConfig::extra_data` carries OUR 12-byte facts blob —
//! `delay u32 LE | padding u32 LE | total_frames u32 LE` (zeros =
//! no trim) — which the MP3 demuxer will distill from the Xing/Info
```
Code (harvest, apps/cli/src/containers/es.rs:160): `extra_data: None,`
What is wrong: the doc says the MP3 demuxer will distill the LAME facts into `extra_data`; the demuxer's facts harvest (`harvest_mpeg_audio`) hard-codes `extra_data: None` and never parses the LAME tag (`RawMp3Demuxer::first_frame_is_metadata`, crates/demux/src/es/mp3.rs:599-633, only packet-skips the header frame). The CLI's `"mp3"` row is the unseeded lane (pipeline_host.rs:261-265). Result: through the shipped `lumen open`, `delay`/`padding`/`total_frames` are always 0 — the LAME encoder delay (576 samples on every committed fixture, measured) plays as leading silence and the end padding plays too; the end trim can never fire (`total_frames == 0`). The gapless feature the module advertises ("sample-accurate trim") is dead on the shipped path; only `decode_stream` (tests/fuzz) applies it. Why it matters: every real-world LAME/ffmpeg-encoded MP3 plays ~13-25 ms of silence it should trim, and the doc claims wiring that does not exist. What correct looks like: harvest delay/padding/total_frames from the skipped Xing/LAME frame into the TrackFact. Confidence: HIGH — Verification: read (call chain traced; not executed end-to-end).

F-003 [S2]
Where: decoder.rs:493 (with 59-64 and 437-447)
Code:
```
        self.pending_start_trim = delay.saturating_add(LAYER3_PIPELINE_LATENCY as u32);
```
What is wrong: the 529-sample Layer III synthesis latency is added to the start trim for EVERY descriptor the gate accepts — including `mp1` (Layer I) and `mp2` (Layer II), where the module's own measured claim says the offset does not exist:
```
//! to ±1 LSB once aligned (Layer II/MP2 has no such offset — its
//! output aligns at shift 0). Our window therefore applies
```
Layer I was never measured at all (the shift search covered L3 + L2 fixtures). The family gate accepts `mp2` (437-439) and the public API allows configuring it here, so a Layer I/II stream through `Mp3Decoder` loses 529 real samples/channel (~12 ms) at the head. The sibling `Mp2Decoder` applies no such trim (no `529`/`LATENCY`/`pending_start` match in crates/codecs/src/audio/mp2/decoder.rs) — the two decoders disagree about the same Layer II bytes. Why it matters: silent audio loss + a wrong-by-construction path for accepted descriptors. What correct looks like: gate `LAYER3_PIPELINE_LATENCY` on `header.layer == Layer::L3` (engine descriptor), not unconditionally. Confidence: HIGH — Verification: read.

F-004 [S2]
Where: decoder.rs:236-252
Code:
```
    // The Xing consistency gate (the card's typed error path): the
    // claimed frame count must equal the walked audio frames.
    if let Some(claimed) = xing.as_ref().and_then(|x| x.frames)
        && claimed as usize != audio.len()
    {
```
What is wrong (two parts): (a) the gate assumes Xing `frames` EXCLUDES the header frame. I verified this holds for all 15 in-repo files (ffmpeg-authored; `ffmpeg -write_xing 0` produces no tag frame at all, so ffmpeg's muxer wrote every tag in-repo). Whether genuine LAME-CLI-authored files (which the wild is full of) satisfy it is UNVERIFIED — no lame binary in the sandbox; if LAME counts the header frame (a long-documented ambiguity of the Xing spec), every such VBR file hard-fails with `mp3.xing.frame_count_mismatch`. (b) Even under the verified convention, any truncated VBR file (a cut download — walked frames < claimed) refuses to decode AT ALL instead of decoding the available prefix; VLC plays those. `decode_stream` is the differential/fuzz workhorse, not the shipped packet path, so blast radius is the free surface + any future consumer. Confidence: MEDIUM — Verification: read + python3 empirical walk (part a verified for ffmpeg-authored files; LAME-CLI semantics UNVERIFIED).

F-005 [S3]
Where: decoder.rs:300 (and 529-530)
Code:
```
        samples.extend_from_slice(&scratch[..frames_decoded * spec_channels]);
```
What is wrong: `spec_channels` is frozen from the first decoded frame; `decode_one` gates only on LAYER switch (143-150), never on channel-mode/rate change. A mid-stream mode change (hostile or corrupt stream) makes the slice read stale scratch bytes (if channels shrink) or truncate channels (if they grow) — silent garbage, no typed error; the contract path (529-530) slices `self.scratch[..frame_samples * channels]` with the same frozen count and ignores `packet.discontinuity`, which the demuxer does emit for exactly this case (crates/demux/src/es/mp3.rs:571-576). What correct looks like: compare `decoded.spec()` per frame and emit a typed error or re-negotiate. Confidence: MEDIUM — Verification: read (engine's per-frame spec behavior not executed).

F-006 [S3]
Where: decoder.rs:548-549
Code:
```
            let keep = frame_samples - drop;
            emit = emit.get(..keep * channels).unwrap_or(emit);
```
What is wrong: when start and end trim land on the same frame (short stream: the delay window still open on `total_frames−1`), `keep * channels` exceeds the already-start-trimmed `emit.len()`, `get` returns `None`, and the fallback silently keeps the untrimmed `emit` — the end trim is skipped with no error and no accounting. What correct looks like: clamp `keep` to `emit.len()/channels` and account the shortfall. Confidence: HIGH — Verification: read (arithmetic).

F-007 [S3]
Where: decoder.rs:554-565
Code:
```
            let slot_len = MAX_SAMPLES_PER_FRAME * channels * 4;
            let bytes = pooled_sample_slot(
                &mut self.sample_pool,
                &mut self.slot_next,
                slot_len,
                |dst| {
                    let mut cursor = 0usize;
                    for v in emit {
                        dst[cursor..cursor + 4].copy_from_slice(&v.to_ne_bytes());
```
What is wrong: `fill` writes `emit.len()*4` bytes with no bounds check into a slot sized from the constant 1152, not from the actual frame. If the engine ever reports >1152 frames for one packet, `dst[cursor..cursor+4]` panics — "no panic on any input" (mod.rs:39-42) depends on an engine contract this file never checks. What correct looks like: size the slot from `emit.len()` or assert `frame_samples <= MAX_SAMPLES_PER_FRAME` typed. Confidence: MEDIUM — Verification: read (symphonia returns ≤1152 for MPA in practice; not executed).

F-008 [S4]
Where: decoder.rs:29-31
Code:
```
//! `delay u32 LE | padding u32 LE | total_frames u32 LE` (zeros =
//! no trim) — which the MP3 demuxer will distill from the Xing/Info
```
What is wrong: "(zeros = no trim)" is false for the contract path — zeros still trim 529 samples (F-003). The doc contradicts the code it documents. Confidence: HIGH — Verification: read.

F-009 [S4]
Where: decoder.rs:425-427
Code:
```
/// Max samples per frame across MPEG versions/layers (Layer I/II:
/// 1152; Layer III MPEG-1: 1152; MPEG-2 L3: 576 — bounded by 1152).
```
What is wrong: Layer I is 384 samples/frame (header.rs:190), not 1152. The bound is still correct; the comment misstates the spec. Confidence: HIGH — Verification: read (header.rs:188-197 cross-checked).

F-010 [S4]
Where: decoder.rs:163-168
Code:
```
    let frames = decoded.frames();
    let (rate, channels) = {
        let spec = decoded.spec();
        (spec.rate(), spec.channels().count())
    };
    let _ = rate;
```
What is wrong: the engine's per-frame rate is fetched then discarded; a rate switch (or header/engine disagreement) is never detected on any path — `out_rate` is frozen from frame 1 (523-528) and durations/pts are computed with it. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| codec_id | 105-112 | REAL | — | L1/L2/L3→engine ids; Reserved→None |
| decode_one | 126-176 | REAL | S3/S4 | engine glue; rate discarded (F-010); no channel-change gate (F-005) |
| decode_stream | 188-341 | REAL | S2 | Xing gate convention (F-004); verified by executed bitexact suite |
| facts::encode / facts::decode | 350-356 / 360-368 | REAL | — | round-trip pinned by test |
| Mp3Decoder::new / accepts / engine_codec | 432-447 | REAL | — | accepts mp1/mp2/mp3, case-sensitive |
| configure | 451-500 | REAL | S2 | unconditional 529 trim (F-003) |
| decode | 502-603 | REAL | S2 | pts hole (F-001), same-frame trim skip (F-006), unbounded fill (F-007) |
| flush | 605-611 | REAL | — | correct: MPEG frames stateless |
| pooled_sample_slot | 616-654 | REAL | S3 | rotation sound; fill unbounded (F-007) |
| Debug::fmt | 409-420 | REAL | — | finish_non_exhaustive |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| descriptor_gate_accepts_the_mpeg_audio_family | Yes | direct matches! on accepts |
| facts_blob_round_trips_and_rejects_bad_lengths | Yes | round-trip + 2 negative cases |
| decode_before_configure_is_typed_bad_state | Yes | asserts error code `mp3.not_configured`; unreachable! on Ok |
| configure_rejects_foreign_descriptors_and_bad_facts | Yes | asserts `mp3.codec_descriptor` / `mp3.facts.length` |
| xing_frame_count_inconsistency_is_typed | Yes, but | proves the gate fires on a lying count; does NOT prove the convention matches real encoders (self-authored blob, claimed=7 vs 1 audio frame) |
| gapless_window_applies_the_pipeline_latency_shift | Yes, but | pins window length 452*2 — arithmetic against their own 529 model; the model itself is only validated by the bitexact differential (which I ran, PASSED, live ffmpeg) |
| gapless_window_clamps_when_padding_undercuts | Yes | length 573*2 pinned |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "symphonia-bundle-mp3 0.6.1, exact-pinned … engine owns exactly one job" | decoder.rs:7-16 | REAL | Cargo.toml:90 `symphonia-bundle-mp3 = { version = "=0.6.1", optional = true }`; decode_one delegates to `MpaDecoder::decode_ref` |
| "the MP3 demuxer will distill [the facts] from the Xing/Info LAME tag" | decoder.rs:31-32 | FALSE | es.rs:160 `extra_data: None`; demuxer never parses LAME tag (F-002) |
| "zeros = no trim" | decoder.rs:30-31 | FALSE | decoder.rs:493 trims 529 with zeros (F-008) |
| "Layer II/MP2 has no such offset — shift 0" | decoder.rs:59-60 | contradicted by own code | decoder.rs:493 applies the 529 to the mp2 descriptor too (F-003) |
| "differential … compare against ffmpeg … ±1 LSB" (module doc) | decoder.rs:38-46 | REAL | executed mp3_bitexact binary: 2 passed with live ffmpeg |
| "after a typed Corrupt the next packet decodes unchanged" | decoder.rs:22-24 | REAL-UNPROVEN | per-frame isolation is structural (stateless engine config); not executed by me (baseline cargo test covers fuzz survival) |
| mp3 `seek` machinery ("CBR arithmetic, VBR TOC") | mod.rs:12-13, seek.rs | UNREACHABLE (shipped) | crates/demux/src/es/mp3.rs:683-689 `seek` → `Err("demux.es.mp3.seek.unsupported")`; cbr_seek/vbr_toc_seek/refine_to_frame have no shipped caller (only tests/fuzz) |

## 7. Dead / duplicated / unreachable code
- `decode_stream`: NOT on the shipped path (tests/fuzz/bench + mp2 sibling only) — the shipped lane is the packet contract. Real code, test-reachable.
- mp3 `seek.rs` surface: unreachable from any shipped entry point (demuxer refuses seek).
- `let _ = rate` (168) — dead read.

## 8. Dependencies & flags
`codec-mp3 = ["dep:symphonia-bundle-mp3", "dep:symphonia-core"]`, both `=0.6.1` exact-pinned, both in `default` (crates/codecs/Cargo.toml:47-59) → the release lane (lumen-cli links codecs default features) ships them. Web/wasm lane gates it behind `codec-mp3` (apps/web/Cargo.toml:106) which the shipped wasm module does NOT enable (stub path) — native-only reachability. `codec-mp2 = ["codec-mp3"]` — Layer II rides the same engine.

## 9. Open questions for the next agent
- Does a genuine LAME-CLI-authored VBR file satisfy the Xing gate (frames excluding header frame)? Needs a lame binary or a LAME-authored corpus file — currently UNVERIFIED (F-004).
- Does any downstream sink schedule audio frames by pts (making F-001's 529-hole audible)? Check audio-engine/audio-output consumption.
- mp3/fuzz.rs wiring status claim ("+FB; wiring status documented there") — separate file, not audited here.
