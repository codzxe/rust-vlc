# AUDIT: crates/demux/src/wav/format.rs
| Field | Value |
|---|---|
| Auditor | W04-A04 |
| Date | 2026-11-23 |
| Lines / functions | 880 / 16 source fns + 10 tests |
| File verdict | REAL (tight, length-guarded parser; 1 validation-bypass arm + 1 false parity doc claim) |
| Reachable from shipped entry point? | YES: lumen-cli `containers/mod.rs` "wav" -> `containers/wav.rs::open` -> `WavDemuxer` -> `wav/mod.rs:464 parse_fmt_chunk` -> `wav/mod.rs:721 format::parse_fmt` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `./target/debug/deps/demux-5e0c7194d741938b --test-threads=2 wav::` -> 43 passed, 0 failed (exit 0); cross-read of `crates/codecs/src/audio/pcm/decoder.rs:374-512` and `apps/cli/src/containers/wav.rs:90-130` + `apps/cli/src/demux_host.rs:201-216` (blob handoff) |

## 1. One-paragraph verdict
A real, careful `fmt ` chunk parser: every field read is behind an explicit length guard (16/20/40/74-byte thresholds), the extensible GUID resolution, valid-bits rule, and PCM align cross-check are all implemented and typed, ADPCM block geometry uses the correct IMA/MS formulas, and nothing here allocates from header values (the 4096-byte clamp lives in `wav/mod.rs:712`; the FLAC STREAMINFO is a fixed `[u8;34]` copy). Two defects: the ADPCM declared-vs-derived validation is bypassable exactly when the block geometry is broken (a declared `wSamplesPerBlock` is accepted unconditionally when the formula cannot derive a ceiling — a 4-byte "block" claiming 4000 samples/block parses fine and inflates duration/stamps), and the module doc's "accepts exactly the blobs the engine's PCM decoder accepts / no demux-ok/decode-fail divergence" claim is false at the blob level (the decoder only accepts lengths {16, 18} ∪ ≥40; `parse_fmt` accepts any length ≥ 16) — it is not user-visible today only because the WAV PCM path hands the decoder a channels/rate pair instead of the blob, which I verified by reading the handoff.

## 2. What actually works (verified)
- 43 wav unit tests pass, including all 10 in this file: `./target/debug/deps/demux-5e0c7194d741938b --test-threads=2 wav::` -> `test result: ok. 43 passed; 0 failed` (binary newer than source).
- Panic-path trace (read, all reads guarded): `u16le(bytes,18)`/`u32le(bytes,20)`/`bytes[24..40]` only inside the `bytes.len() >= EXTENSIBLE_BYTES(40)` arm (368-404); `bytes[40..74]` only under `>= FLAC_WAV_BYTES(74)` (383-385); ADPCM `u16le(bytes,18)` under `>= WAVEFORMATEX_BYTES+2 (20)` (420-421); floor `< MIN_FMT_BYTES(16)` at 346. No unguarded index.
- IMA formula `((1024−4·2)·2)/2+1 = 1017` and MS `(1024−7·2)/2·2+2 = 1012` recomputed by hand — both correct, both pinned by tests (706-719, 785-802).
- No allocation is sized by header values in this file; `MAX_FMT_BYTES` clamp verified at the caller (wav/mod.rs:710-712).

## 3. Findings

F-001 [S3]
Where: crates/demux/src/wav/format.rs:430-452 (samples_per_block match), with 306-313
Code:
```rust
            (Some(0), None) | (None, None) => {
                return Err(error::bad_format_params(offset));
            }
            (Some(declared), None) => {
                if declared == 0 {
                    return Err(error::bad_format_params(offset));
                }
                declared
            }
```
What is wrong: `derived` is `None` precisely when the block geometry is impossible (`block_align <= 4·channels` for IMA, `<= 7·channels` for MS — lines 308-310, 321-323). In that case the `(Some(declared), Some(derived))` ceiling check (`declared > derived` refuses) never runs, and ANY nonzero declared `wSamplesPerBlock` is accepted. Example: IMA, channels=2, block_align=4 (8-byte header does not fit), declared=4000 at fmt[18] -> `Ok` with `samples_per_block: 4000`. The demuxer then stamps 4000 sample frames per 4-byte block (wav/mod.rs:659-664 span_frames = units × spu; per-packet stamps at mod.rs:957), inflating duration ~1000× on a malformed file. The code's own comment says "a declaration that disagrees with the block geometry is a lie about arithmetic" — but the lie is only caught when the geometry is derivable; the most broken geometries escape the check entirely. Untested arm. What correct looks like: refuse when `derived.is_none()` (the geometry cannot legally carry any samples), or clamp declared to 0. Confidence: HIGH — Verification: read (arm traced; no run — no ADPCM lane fixture in-tree).
Confidence: HIGH — Verification: read

F-002 [S3]
Where: crates/demux/src/wav/format.rs:4-10 (module doc) vs parse_fmt:346-348, and crates/codecs/src/audio/pcm/decoder.rs:375-385
Code:
```rust
//! accepts (`codecs::audio::pcm::decoder::parse_waveformatex`'s rules,
//! read and mirrored — the demux layer must not depend on codecs,
//! `boundaries.toml`; **one accepted-set, no demux-ok/decode-fail
//! divergence** for PCM).
```
```rust
pub fn parse_fmt(bytes: &[u8], offset: u64) -> Result<WaveFormat, EngineError> {
    if bytes.len() < MIN_FMT_BYTES {
        return Err(error::bad_structure(offset));
    }
```
What is wrong: the mirror is not exact. The decoder's length gate is `!matches!(bytes.len(), 16 | 18) && bytes.len() < 40` (decoder.rs:375) — it accepts ONLY 16, 18, or ≥40-byte blobs. `parse_fmt` accepts any length ≥ 16: a 17-, 19-, or 25-byte PCM `fmt ` payload parses `Ok` here and is refused by `parse_waveformatex`. The "one accepted-set" claim is false at the blob level. It is not user-visible on the current WAV path — I verified `containers/wav.rs:104-108` harvests PCM as a `(channels, rate)` pair with `extra_data: None`, and `demux_host.rs:206-208` answers geometry from that pair without ever re-parsing the blob — but the doc's safety argument ("no demux-ok/decode-fail divergence") rests on a parity that does not exist and is one refactor (handing the blob through) away from breaking. What correct looks like: enforce the same `{16, 18} ∪ ≥40` shape here, or fix the doc. Confidence: HIGH — Verification: read (both files + handoff chain).
Confidence: HIGH — Verification: read

F-003 [S4]
Where: crates/demux/src/wav/format.rs:383-389 (FLAC tail) and 458-463 (ADPCM bits)
Code:
```rust
            let streaminfo = if bytes.len() >= FLAC_WAV_BYTES {
                let mut block = [0u8; FLAC_STREAMINFO_BYTES];
                block.copy_from_slice(&bytes[EXTENSIBLE_BYTES..FLAC_WAV_BYTES]);
                Some(block)
            } else {
                None
            };
```
What is wrong: `cbSize` (fmt[16..18]) is never read anywhere in the file — the FLAC STREAMINFO is taken positionally at [40..74] regardless of what cbSize declares, and ADPCM's `wBitsPerSample` is recorded unvalidated (any 16-bit value passes). ffmpeg-compatible tolerance, no memory-safety impact, but the declared extension size is silently ignored (a cbSize lie is indistinguishable from a positional read). Confidence: HIGH — Verification: read (rg for cbSize reads: none).
Confidence: HIGH — Verification: read

F-004 [S4]
Where: crates/demux/src/wav/format.rs:391-400 (FLAC early return)
Code:
```rust
            let codec = WavCodec::Flac { streaminfo };
            return Ok(WaveFormat {
                format_tag: TAG_EXTENSIBLE,
                channels,
                samples_per_sec: rate,
                block_align,
```
What is wrong: the FLAC arm returns before the `valid_bits` oddity check (481-488) and before any block_align sanity that the PCM path enforces (492-495) — an extensible FLAC declaration with `wValidBitsPerSample = 0` or a contradictory `nBlockAlign` parses `Ok` where the PCM twin refuses. Impact nil today (FLAC `unit_size` is 1; no FLAC-in-WAV decoder landed), but the validation surface is inconsistent between arms of the same tag family. Confidence: HIGH — Verification: read.
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PcmSample::descriptor | 109-118 | REAL | — | static vocabulary strings |
| PcmSample::bits_per_sample | 121-129 | REAL | — | total function |
| PcmSample::bytes_per_sample | 132-140 | REAL | — | total function |
| PcmSample::from_wave_tag | 146-162 | REAL | — | tag 1: 8/16/24/32; tag 3: 32/64; else None |
| WavCodec::descriptor | 204-211 | REAL | — | registry names |
| WavCodec::unit_size | 217-223 | REAL | — | PCM frame / ADPCM block_align / FLAC 1 — block_align==0 ADPCM refused earlier (414-416) |
| WavCodec::samples_per_unit | 228-235 | REAL | — | FLAC None (honest unknown) |
| WaveFormat::codec_descriptor / unit_size / samples_per_unit | 273-288 | REAL | — | delegates |
| u16le / u32le | 291-297 | REAL | — | panics only if caller skips guards — none do (traced) |
| ima_samples_per_block | 306-313 | REAL | — | formula correct; checked_mul; div by channels ≥ 1 |
| ms_samples_per_block | 319-326 | REAL | — | formula correct; bounded (≤ ~131k) |
| parse_fmt | 345-507 | REAL | S3 | F-001 bypass arm; F-002 length-set divergence vs decoder doc claim; F-003/F-004 inconsistent validation surface |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| parses_plain_pcm16 | YES | field-by-field assertions incl. descriptor string |
| parses_extensible_pcm24_like_ffmpeg_writes | YES | GUID resolution + validBits + mask |
| extensible_float_guid_maps_to_f32 | YES | float GUID -> pcm_f32le |
| extensible_flac_guid_exposes_streaminfo_and_descriptor | YES | 74-byte tail copy verified byte-exact; tailless variant covered |
| extensible_without_body_is_a_structural_lie | YES | 0xFFFE short -> bad-structure code |
| unknown_tags_and_depths_are_typed_refusals | YES | 20-bit / float-8 / tag 0x55 -> unsupported-format |
| adpcm_tags_parse_their_block_framing | PARTLY | covers declared==derived, declared>derived refused, both derivations — does NOT cover F-001's (Some(declared), None) acceptance arm (the gap is untested) |
| valid_bits_below_container_bits_are_refused | YES | 20-in-24 -> unsupported |
| zero_or_inconsistent_params_are_bad_fmt | YES | zero rate/channels, lying align, 65 channels |
| short_fmt_is_structural | YES | 15 bytes -> bad-structure |
All 10 ran green in my run (43-pass wav filter includes them).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "accepts exactly the blobs the engine's PCM decoder accepts ... no demux-ok/decode-fail divergence for PCM" | format.rs:4-8 | FALSE at blob level (not user-visible: pair handoff) | F-002: decoder.rs:375 `16|18|>=40` vs parse_fmt `>=16`; handoff verified containers/wav.rs:104-108 + demux_host.rs:206-208 |
| "a cbSize vendor tail beyond this is corruption, skipped positionally" | format.rs:32-36 | REAL | clamp at wav/mod.rs:710-712; but cbSize itself never read (F-003) |
| "IMA ... (block 1024, stereo -> 1017; block 1024, mono -> 2041)" | format.rs:304-305 | REAL | recomputed: ((1024−8)·2)/2+1=1017; ((1024−4)·2)/1+1=2041 |
| "MS ... (block 1024, stereo -> 1012, the corpus 0102 declaration)" | format.rs:317-318 | REAL | recomputed: (1024−14)/2·2+2=1012 |

## 7. Dead / duplicated / unreachable code
- `(Some(0), None)` arm at 442-444 is subsumed by 445-450's `declared == 0` check — dead duplication (harmless).
- No todo!/unimplemented!/unreachable! placeholders. No #[allow]. No dead pub items (all consts/types consumed by mod.rs, fuzz.rs, or the CLI harvest).

## 8. Dependencies & flags
Only `media_types::EngineError` + `super::error` (typed WAV error family, codes verified by tests: `demux.wav.bad-structure` / `unsupported-format` / `bad-fmt`). No codec-crate dependency (boundary honored — the "mirror" is by reading, not importing, which is exactly why F-002 could drift). No feature gates; release lane builds it.

## 9. Open questions for the next agent
- Run a crafted IMA blob (block_align=4, ch=2, declared=4000) through the full WavDemuxer to confirm the F-001 duration inflation end-to-end (I verified the parse arm by reading; no ADPCM fixture in-tree to run).
- wav/mod.rs (not my file): confirm the per-packet stamp path uses samples_per_block the same way I traced at 957 — if it re-derives, F-001's impact shrinks.
- If the engine ever starts handing the WAV PCM blob to the decoder (the AVI strf shape), F-002 becomes user-visible: fmt sizes 17/19-39 open fine then fail at decode.
