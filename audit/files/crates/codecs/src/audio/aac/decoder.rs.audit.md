# AUDIT: crates/codecs/src/audio/aac/decoder.rs
| Field | Value |
|---|---|
| Auditor | W07-A05 |
| Date | 2026-11-23 |
| Lines / functions | 894 / 16 prod + 8 tests |
| File verdict | REAL (with one S2 honesty hole on the MP4/HE-AAC lane) |
| Reachable from shipped entry point? | YES — lumen CLI `open` → apps/cli/src/containers/mp4.rs `b"mp4a" => Some("aac")` + `asc_of_entry` (esds DSI = raw ASC) → apps/cli/src/pipeline_host.rs `"aac" \| "aac-lc" => self.seeded(track, config, AacDecoder::new)` → `AacDecoder::configure/decode` (this file). ADTS lane: demux/src/adts/mod.rs emits `aac-lc` → same arm. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 3 |
| Commands run | `cargo test -p codecs --lib aac::decoder` → 8 passed (exit 0); `cargo test -p codecs --lib aac` → 17 passed; `cargo test -p codecs --test aac_bitexact` → 2 passed; `--test aac_corpus_differential` → 1 passed but stdout `SKIP aac corpus differential: corpus root absent`; `--test aac_fuzz_survival` → 2 passed; `cargo test -p lumen-cli --test container_conformance mp4_h264_family_lane` → 1 passed (committed h264+aac MP4 plays end-to-end). |

## 1. One-paragraph verdict
The AAC-in-MP4 answer: `configure()` receives the **bare AudioSpecificConfig** extracted from the MP4 esds DecoderSpecificInfo (apps/cli/src/containers/mp4.rs `asc_of_entry` → `dsi_of_esds`), sniffs the first 5 bits (AOT) + channel config itself, then hands the raw ASC to **symphonia-codec-aac 0.6.1 (exact-pinned, `=0.6.1` in Cargo.toml)** as `extra_data`; the entropy decode is fully the engine's, the ADTS parse/gates/error taxonomy are hand-rolled here. Accepted object types: **AAC-LC only**. Explicit HE (AOT 5/29) → typed `aac.he_sbr`; Main/SSR/LTP/other → `aac.profile`; chcfg 0 → `aac.channel_config_pce`; >2ch → `aac.channel_count` — all verified by passing tests. The S2 hole: **implicit-signaled HE-AAC in MP4 (AOT 2 + 0x2B7 sync extension — the common real-world HE spelling) passes every gate and silently decodes as half-rate, half-bandwidth LC at the core sample rate** while the container declares 2× that rate, directly contradicting the file's own "never a silent downgrade" claim. ADTS mode is honest and well-tested; the raw/ASC decode mode is executed end-to-end only by a CLI timing-assertion lane that could not detect the downgrade.

## 2. What actually works (verified)
— `cargo test -p codecs --lib aac::decoder` → 8/8 passed (gates: descriptor, AOT sniff, chcfg sniff+escape, 6ch/7.1/PCE typed refusals, HE/LTP/short-ASC typed refusals, BadState, garbage).
— `cargo test -p codecs --test aac_bitexact` → 2/2 passed (real ADTS fixture decode: `lc_frames_are_exactly_1024_samples`, `fixtures_decode_the_recorded_window_exactly`).
— `cargo test -p lumen-cli --test container_conformance mp4_h264_family_lane` → passed: committed h264+aac MP4 walks to `end-of-media`/`Playing → Ended`, `ok: time 1.532517s (100.0%)` — the MP4 esds→ASC→engine lane decodes end to end.
— Engine contract verified by read: symphonia-codec-aac 0.6.1 `decode_ref` expects a **header-less raw data block** (BitReaderLtr over packet data, SCE/CPE/FIL/TERM walk) — the wrapper's ADTS header strip (`data.get(header.header_len..header.frame_len)`) matches.

## 3. Findings
F-001 [S2]
Where: crates/codecs/src/audio/aac/decoder.rs:109-122 + 445-459 (gate), vs doc claim :360-363
Code:
```rust
    match asc[0] >> 3 {
        1 => AscObjectType::Main,
        2 => AscObjectType::Lc,
        3 => AscObjectType::Ssr,
        4 => AscObjectType::Ltp,
        5 => AscObjectType::Sbr,
        29 => AscObjectType::Ps,
        other => AscObjectType::Other(other),
    }
```
and the doc claim:
```rust
/// — the MP4/esds path): one packet = one raw data block. The ASC
/// is sniffed up front: HE (AOT 5/29) refuses `aac.he_sbr`, non-LC
/// refuses `aac.profile` — never a silent downgrade.
```
What is wrong: the sniff reads ONLY the first 5 bits. Implicit HE-AAC signaling (AOT 2 + GASpecificConfig + trailing 0x2B7 sync extension declaring SBR) is invisible to it. The engine is the only remaining line of defense — and it has the same blind spot: symphonia-common-0.6.1/src/mpeg/audio/mod.rs:400 gates the sync-extension parse on `if asc.sbr_ps_info.is_some()` which is set ONLY in the AOT-5/29 branch (:246-261), so for AOT 2 the extension is never parsed and `sbr_present` stays false; the engine gate (`symphonia-codec-aac-0.6.1/src/aac/mod.rs:93-96`: `asc.samples != 1024 { return unsupported_error("aac: aac too complex"); }`) passes. Result: a 48 kHz-declared HE-AAC MP4 decodes as 24 kHz core LC (half bandwidth, `out_rate = engine rate` at :549), and even the in-band SBR payload is skipped by the engine (`bs.ignore_bits` in the ID_FIL arm). The doc's "never a silent downgrade" is false for exactly the realistic MP4 HE spelling (FDK/HLS default). No committed test could catch it: every decode lane is ADTS-LC, and the MP4 conformance lane asserts wall-clock strings only.
Why it matters: wrong rate/bandwidth output and 2× duration desync on a common input class, silently. What correct looks like: sniff the 0x2B7 sync extension after GASpecificConfig (the same bit walk symphonia-common does for AOT 5/29) and refuse `aac.he_sbr`.
Confidence: HIGH — Verification: read (both sides + vendored engine source); no runtime repro (no HE fixture in tree — UNVERIFIED executably).

F-002 [S3]
Where: crates/codecs/src/audio/aac/decoder.rs:489-593 (raw mode) / tests 647-894
Code:
```rust
        let payload: &[u8] = if self.raw_mode {
            data
        } else {
            let header = parse_adts_header(data)?;
```
What is wrong: no test anywhere decodes a packet through the raw/ASC `decode()` path (all integration lanes go through ADTS `decode_stream`; the in-file tests exercise configure() gates only, two of them happy-path configure with no decode). The unique raw-mode lines (engine-from-ASC build + `out_channels == 0` branch at :547-550) are executed only by the CLI conformance lane, which asserts timing strings, not sample counts/rate. Test-honesty gap on the exact lane the hostile focus targets.
Confidence: HIGH — Verification: read (rg over tests/ + apps/cli/tests: zero raw-mode decode assertions).

F-003 [S3]
Where: crates/codecs/src/audio/aac/decoder.rs:45-52 (doc claim), tests/aac_corpus_differential.rs (SKIP observed)
Code:
```rust
//! Such streams decode as **half-rate LC** (bandwidth-limited); the
//! differential records the degradation (`lumen_rate × 2 = ffmpeg_rate`) instead of
//! silently passing it.
```
What is wrong: the claimed degradation-recording differential has never run in any recorded environment: baseline `cargo-test-workspace.txt:897-902` shows `aac_corpus_differential ... finished in 0.06s` (skip-fast), and my run printed `SKIP aac corpus differential: corpus root absent`. The honesty posture for HE rests on a suite that is dead weight in every recorded run, and it only covers ADTS anyway — the MP4 implicit case (F-001) has no differential lane at all.
Confidence: HIGH — Verification: ran (skip observed) + baseline citation.

F-004 [S3]
Where: crates/codecs/src/audio/aac/decoder.rs:261-270 (vs engine behavior)
Code:
```rust
    let frames = decoded.frames();
    let channels = decoded.spec().channels().count();
    if frames == 0 || channels == 0 {
        return Err(corrupt(
            "aac.frame.no_samples",
```
What is wrong: dead check inherited from engine internals: symphonia's `decode_inner` does `self.buf.render_uninit(None)` BEFORE element decode, committing the full 1024 frames (`num_frames = capacity`) with pre-zeroed planes (`AudioBuffer::new` fills `S::MID`), and the ID_TERM element (first 3 bits = 111, i.e. payload byte ≥ 0xE0) breaks the walk immediately. So a corrupted/shifted raw block starting ≥ 0xE0 — or one that under-fills the declared channels — returns Ok + 1024 frames of silence; `aac.frame.no_samples` is unreachable and corruption is swallowed as silence rather than typed. Engine-boundary leniency the wrapper inherits without detection.
Confidence: MEDIUM-HIGH — Verification: read (symphonia-core buf.rs:389-401, engine mod.rs decode_inner); not executed.
Confidence note: no panic/UB — planes are zero-initialized, this is swallowed-error class, not memory-unsafety.

F-005 [S4]
Where: crates/codecs/src/audio/aac/decoder.rs:556-563
Code:
```rust
            let bytes = pooled_sample_slot(&mut self.sample_pool, &mut self.slot_next, |dst| {
                let mut cursor = 0usize;
                for v in interleaved {
                    dst[cursor..cursor + 4].copy_from_slice(&v.to_ne_bytes());
```
What is wrong: per-frame double copy (engine→scratch interleave, then a byte-loop into the pooled slot); the loop is also scalar `to_ne_bytes` per sample where `extend_from_slice`/chunked copy would do. Bounded and small (8 KB/frame) — hygiene/perf only.
Confidence: HIGH — Verification: read.

F-006 [S4]
Where: crates/codecs/src/audio/aac/decoder.rs:608-645
Code:
```rust
    fill: impl FnOnce(&mut [u8]) -> Result<(), AacError>,
) -> Result<SharedBytes, EngineError> {
```
What is wrong: inert error plumbing — the only closure ever passed returns `Ok::<(), AacError>(())` unconditionally (:562); the `Result` signature is theater. Also the `slots[idx].len() != MAX_SLOT_BYTES → continue` guard can never be true (slots are only ever pushed at exactly MAX_SLOT_BYTES).
Confidence: HIGH — Verification: read.

F-007 [S4]
Where: crates/codecs/src/audio/aac/decoder.rs:292-348
What is wrong: `decode_stream` buffers the whole input plus the whole interleaved PCM (`samples: Vec<f32>`) — fine for its documented conformance/differential role and not on the product decode path (product = `Decoder::decode` with the 4×8 KB pool), but it ships in the product crate as `pub`. Note only.
Confidence: HIGH — Verification: read + rg (product callers: none; tests/fuzz only).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `sniff_asc_object_type` | 109-122 | REAL | S2 | first-5-bits only; misses implicit SBR (F-001); unit-tested |
| `asc_channel_count` | 128-135 | REAL | — | 0→0, 1..=6 identity, 7→8, 8-15 None; tested |
| `sniff_asc_channel_config` | 142-151 | REAL | — | index + 24-bit escape both correct (bit math verified + tested) |
| `gate_object_type` | 155-170 | REAL | — | LC pass, 5/29 `aac.he_sbr`, rest `aac.profile`; tested |
| `gate_adts_profile` | 173-184 | REAL | — | tested via decode_stream garbage lanes |
| `gate_channels` | 188-206 | REAL | — | PCE/>2ch typed refusals; tested |
| `adts_engine` | 209-227 | REAL | — | builds engine from header params, no extra_data — matches engine ADTS-less contract |
| `decode_one` | 233-278 | REAL | S3 | shared engine round trip; dead zero-samples check (F-004); executed by bitexact/fuzz suites |
| `decode_stream` | 292-348 | REAL | S4 | ADTS walk + param-switch gate; whole-buffer memory (F-007); executed green |
| `Debug for AacDecoder` | 388-396 | REAL | — | honest (finish_non_exhaustive, no engine introspection) |
| `AacDecoder::new` | 407-409 | REAL | — | Default |
| `AacDecoder::accepts` | 416-418 | REAL | — | "aac"/"aac-lc" only; tested incl. case sensitivity |
| `Decoder::configure` | 422-487 | REAL | S2 | ASC gates good; implicit-SBR hole (F-001); reserved chcfg 8-15 falls through to engine's `aac.engine.init` Unsupported where Corrupt class would be truer |
| `Decoder::decode` | 489-593 | REAL | S3 | ADTS per-packet parse + lazy engine; raw mode lines untested (F-002); pts/duration math verified correct |
| `Decoder::flush` | 595-603 | REAL | — | Drained, honest (engine holds no cross-packet audio state; overlap lives inside one frame) |
| `pooled_sample_slot` | 608-645 | REAL | S4 | 4×8 KB rotation correct; inert Result (F-006) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| descriptor_gate_accepts_aac_only | YES | matches! on 9 strings |
| asc_object_type_sniffing_spots_the_he_family | YES | 7 assertions incl. PS/AOT-escape residue |
| asc_channel_config_sniffing_handles_index_and_escape | YES | bit-exact incl. 24-bit escape window |
| six_channel_asc_refuses_typed_not_engine_opaque | YES | pins code strings `aac.channel_count`/`aac.channel_config_pce` + clean stereo/mono configure |
| he_asc_refuses_typed_and_never_reaches_the_engine | YES | pins `aac.he_sbr` for AOT 5, `aac.profile` for LTP, `aac.asc.length` |
| decode_before_configure_is_typed_bad_state | YES | pins `aac.not_configured` + Decoder kind |
| configure_rejects_foreign_descriptors | YES | pins `aac.codec_descriptor` |
| empty_buffer_and_garbage_refuse_typed_not_panicking | PARTIAL | proves no-panic + refusal code, but 0xFF garbage never exercises the engine; no test anywhere decodes a broken-then-fixed frame sequence (per-frame isolation claim untested) |
| (integration) aac_bitexact 2 tests | YES | real fixtures, 1024-sample pin + recorded window |
| (integration) aac_corpus_differential | NO (vacuous here) | SKIPs on corpus absence in sandbox AND baseline — the HE-degradation recording claim has zero recorded execution |
| (integration) aac_fuzz_survival | PARTIAL | absence-of-panic + typed-code stability; acceptance tally vacuous (prior agent noted same) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "symphonia-codec-aac 0.6.1, exact-pinned" | :7-9 | REAL | codecs/Cargo.toml `symphonia-codec-aac = { version = "=0.6.1", optional = true }`; registry source read |
| "engine implements AAC-LC only (non-LC, sbr_present, >2ch, non-1024 refused)" | :35-36 | REAL | engine aac/mod.rs:93-96 gate (quoted in F-001 context) |
| "Explicit ASC signaling (AOT 5/29) refused as aac.he_sbr before the engine is built" | :39-41 | REAL | gate_object_type before try_new; test passes |
| "Implicit ADTS HE ... decode as half-rate LC; the differential records the degradation" | :45-52 | SHALLOW | half-rate decode: REAL (engine skips SBR payload); differential recording: never executed in any recorded run (F-003) |
| "never a silent downgrade" (ASC path) | :362-363 | FALSE | implicit-SBR ASC passes all gates and downgrades silently (F-001) |
| "ADTS frames are independent ... after a typed Corrupt the next packet decodes unchanged" | :28-31 | REAL-UNPROVEN | no test pins post-corruption recovery; fuzz survival only proves no-panic |
| "one packet = one complete ADTS frame, header included / one raw data block (ASC)" | :22-25 | REAL | code + engine contract verified |

## 7. Dead / duplicated / unreachable code
- `aac.frame.no_samples` error arm (:263-270) — unreachable given engine always commits 1024 frames (F-004).
- `pooled_sample_slot`'s wrong-size-slot guard (:616-618) — impossible by construction (F-006).
- Nothing gated off: `codec-aac` feature is in `default` (codecs/Cargo.toml), pipeline_host dispatch is unconditional in the CLI build.

## 8. Dependencies & flags
`symphonia-codec-aac = "=0.6.1"` + `symphonia-core` (optional, default-on via feature `codec-aac` in `default`). Engine crate is `#![forbid(unsafe_code)]` (verified in its lib.rs). No unsafe in this file. Release lane enables the feature (baseline builds clean).

## 9. Open questions for the next agent
- Does the audio pipeline resample the decoder-declared `out_rate` (24 kHz in F-001's scenario) against the container-declared rate (48 kHz)? If it trusts the decoder, F-001's desync doubles; if it trusts the container, pitch shifts. Worth tracing in audio-engine's sink/resample chain (convert.rs audited by me — no rate-negotiation logic there; the negotiation lives upstream).
- demux/src/es/aac.rs (raw .aac/es lane) and demux/src/mp4 esds parse — do they pre-filter AOT? (apps/cli mp4.rs does not; verified.)
