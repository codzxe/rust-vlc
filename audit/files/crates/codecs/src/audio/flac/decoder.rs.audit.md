# AUDIT: crates/codecs/src/audio/flac/decoder.rs
| Field | Value |
|---|---|
| Auditor | W07-A09 |
| Date | 2026-11-30 |
| Lines / functions | 716 / 14 (+3 test fns, 1 doc-test) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (see §2 chain) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/codecs-a0d7a93b7072ff58 audio::flac` -> 25 passed, 0 failed (exit 0); `./target/debug/deps/flac_bitexact-0363eae80343785c` -> 4 passed, 0 failed (exit 0); sed/rg quote verification (exit 0) |

## 1. One-paragraph verdict
This is a real, working FLAC decoder built on claxon 0.4.3 (NOT hand-rolled, NOT symphonia-codec-flac — `Cargo.toml:86` pins `claxon = { version = "=0.4.3", optional = true }`, lockfile line 103-104 confirms 0.4.3). claxon owns the full entropy decode: constant/verbatim/fixed(0-4)/LPC(1-32) subframes, Rice partitions, and all three stereo decorrelation modes (verified in vendored src/subframe.rs:17-20, src/frame.rs:34-38,713-740); this file adds STREAMINFO facts, output normalization, pooling, and the codec-api contract. Byte-exactness against ffmpeg MD5s passes on committed fixtures. The defects are a genuine engine subset hole (frames that omit sample size — mandatory for 32-bit FLAC — are undecodable and the 25–32-bit output mapping is dead code), a missing bps consistency gate that truncates hostile audio silently, and two doc lies.

## 2. What actually works (verified)
- `./target/debug/deps/codecs-a0d7a93b7072ff58 audio::flac` -> `test result: ok. 25 passed; 0 failed` (contract, width mapping, i24 byte order, seek planner, metadata).
- `./target/debug/deps/flac_bitexact-0363eae80343785c` -> `test result: ok. 4 passed; 0 failed` — `fixtures_decode_byte_exact_against_the_ffmpeg_reference` decodes 4 ffmpeg-authored fixtures (s16 stereo/mono, s24 48kHz, seektable carrier) to MD5-equal PCM vs ffmpeg dumps AND vs the STREAMINFO MD5 field, plus per-frame first-sample equality against ffprobe pts.
- Reachability (call chain, all read): `apps/cli/src/run.rs`+`dispatch.rs` (CliHost) -> `apps/cli/src/host.rs:948` `HostDecoderProvider::new` -> `apps/cli/src/pipeline_host.rs:253-260` `"flac" => ... self.seeded(track, config, FlacDecoder::new)` -> `FlacDecoder::configure` (STREAMINFO seeded from `apps/cli/src/containers/wav.rs:111-114`; mkv `A_FLAC` and mp4 `fLaC` lanes also mint the descriptor). Feature gate `codec-flac` is in the crate's `default` set (Cargo.toml:52), so the release lane includes it.

## 3. Findings
F-001 [S2]
Where: crates/codecs/src/audio/flac/decoder.rs:41-49 + ~/.cargo/.../claxon-0.4.3/src/frame.rs:686-691,227-236
Code (decoder.rs:44-45):
```
//! bits. The decoder left-justifies into the smallest media-types
//! width that holds the stream — ≤16 → [`SampleFormat::S16`],
```
and claxon frame.rs:691: `None => return Err(Error::Unsupported("header without bits per sample info")),` with frame.rs:229 `0b000 => None, // 000 means 'get from streaminfo block'.`
What is wrong: claxon 0.4.3 refuses any frame whose sample-size code is 000 ("get from STREAMINFO"). The FLAC frame header has explicit codes only for 8/12/16/20/24 bits — 32-bit and every nonstandard depth (17-23, 25-31) MUST use code 000, so every valid 32-bit FLAC stream decodes to `flac.frame.feature` Unsupported. The decoder's `output_sample_format` arm `_ => (SampleFormat::S32, 32 - bits_per_sample.min(32))` (decoder.rs:137) is unreachable through any decodable frame, and the module doc advertises "25–32 → S32" as live coverage. Why it matters: "FLAC (native)" (flac/mod.rs:1-2) is claimed without this subset caveat; the mapping branch is dead code that pretends coverage. What correct looks like: either inject STREAMINFO bps into the claxon read path (fork/patch, or symphonia) or delete/qualify the 25–32 mapping and document the subset. Confidence: HIGH — Verification: read (vendored claxon source); UNVERIFIED: end-to-end decode of a real 32-bit FLAC file (no such fixture in tree; cargo-build-budget).
F-002 [S3]
Where: crates/codecs/src/audio/flac/decoder.rs:253 + 296-307
Code:
```
    let (sample, shift) = output_sample_format(facts.bits_per_sample);
```
```
                2 => {
                    let bytes = (value as i16).to_ne_bytes();
                    dst[cursor..cursor + 2].copy_from_slice(&bytes);
```
What is wrong: output scale is derived from STREAMINFO bps only. A frame that decodes at a different declared bps (e.g. 24-bit frame under a 16-bit STREAMINFO) passes the channels gate (decoder.rs:231-240) and then truncates samples via `as i16` — silently corrupted audio, no typed rejection. The channel-count mismatch got a dedicated gate + fuzz regression; the bps mismatch got nothing. Why it matters: hostile/misauthored streams produce wrong bytes instead of `flac.frame.*` damage. What correct looks like: gate `frame_bps != facts.bits_per_sample` as typed Corrupt before interleave (claxon's `Block` does not expose bps, so reject at header-parse time via `SeekPlanner::parse_header_at`, which already reads the bps code). Confidence: MEDIUM — Verification: read (mutation reasoning; no hostile fixture executed).
F-003 [S4]
Where: crates/codecs/src/audio/flac/decoder.rs:186
Code:
```
                        expected: u64::from(facts.max_frame_size.unwrap_or(u32::MAX)),
```
What is wrong: when max_frame_size is unknown (0 per spec), the Truncated detail reports `expected: 4294967295` — a fabricated 4 GiB expectation. Why it matters: diagnostics lie. What correct looks like: `expected: None`-style detail or omit the field. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: crates/codecs/src/audio/flac/metadata.rs:98 (cross-file, feeds this decoder's TimeBase)
Code:
```
    /// Sample rate in Hz (spec ceiling 65 535; 0 is invalid).
```
What is wrong: the FLAC spec ceiling is 655 350 Hz, and the parse (metadata.rs:182-187) only rejects rate 0 — rates up to 1 048 575 (20-bit field) are accepted. The doc claim "spec ceiling 65 535" is false, and no range cap exists. Answering the hostile-focus question "sample rates to 655350?": no 655350 cap is enforced; any 20-bit value passes into `TimeBase::new(1, rate)`. Blast radius: doc lie + odd-but-tolerated rates (decoder handles them arithmetically; `rate.max(1)` at decoder.rs:582 guards div-by-zero). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| decode_frame (free) | 93-104 | REAL | — | allocates max slot per call; correct truncate to exact len |
| bytes_per | 119-121 | REAL | — | trivial |
| max_output_len | 125-128 | REAL | — | bounded u16 max_block × 8ch × 4B = 2 MiB cap |
| output_sample_format | 133-139 | REAL | — | S32 arm dead via claxon (F-001) |
| decode_into | 150-279 | REAL | — | typed errors all paths; channels gate before plane access; slot-size gate 256 |
| interleave_block | 287-323 | REAL | — | get()-bounded reads, checked_shl; `as i16` truncation = F-002 |
| write_i24 | 328-339 | REAL | — | native-endian low-24; test-pinned |
| FlacDecoder::new | 434-437 | REAL | — | Default |
| FlacDecoder::accepts | 441-443 | REAL | — | exact "flac" match, test-pinned |
| Decoder::configure | 520-549 | REAL | — | typed streaminfo.missing; pool reset |
| Decoder::decode | 551-604 | REAL | — | stale-gen drop, empty→NeedMoreInput, pooled fill, saturating casts |
| Decoder::flush | 606-612 | REAL | — | stateless: Drained; BadState before configure |
| pooled_sample_slot | 629-674 | REAL | — | get_mut exclusivity = no aliasing; bounded SAMPLE_SLOTS=4 |
| contract_tests::channel_count_mismatch_is_typed_not_panic | 454-516 | REAL test | — | would fail if gate removed (panics in claxon channel()) |
| tests::descriptor_gate / output_width_mapping / write_i24 | 680-715 | REAL tests | — | mutation-sensitive asserts |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fixtures_decode_byte_exact_against_the_ffmpeg_reference (integration) | YES | MD5 equality vs external ffmpeg dump AND vs STREAMINFO md5; any decode bug shifts bytes |
| channel_count_mismatch_on_a_decodable_frame_is_typed_not_panic (integration) | YES | decodable mono fixture + doctored facts; Ok(...) is unreachable!-flagged |
| channel_count_mismatch_is_typed_not_panic (lib, contract_tests) | PARTIAL | frame bytes `FF F8 51 ...` are not necessarily decodable — asserts only `is_err()` (structure damage acceptable), the panic guard is the integration twin |
| output_width_mapping_is_left_justifying | YES | exact (format, shift) pairs incl. dead S32 arm (which masks F-001) |
| corpus_flacs_decode_byte_exact_or_reject_typed | SKIP-RISK | prints "SKIP" and returns Ok when LUMEN_CORPUS unset — honest message, but corpus row silently passes as "ok" in logs |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "claxon owns exactly one job here: entropy-coded frame decode (header + subframes + CRC-16 verification, i32 samples)" | decoder.rs:14-16 | REAL | claxon frame.rs read_next_or_eof + crc.rs; tests byte-exact |
| "One packet = one FLAC frame... trailing bytes... are not touched" | decoder.rs:22-28 | REAL | claxon FrameReader stops at frame end; fixtures pass with walk-split packets |
| "zero per-frame allocation after warm-up" | decoder.rs:51-59 | REAL-UNPROVEN | pooled_sample_slot logic read; no allocation-count run executed (audio_bench exists, not run) |
| "25–32 → SampleFormat::S32" | decoder.rs:44 | FALSE (unreachable via decode) | F-001: claxon rejects bps-code-000 frames |
| "Sample rate in Hz (spec ceiling 65 535...)" | metadata.rs:98 | FALSE | spec ceiling is 655 350; no cap in parse_streaminfo |

## 7. Dead / duplicated / unreachable code
- `output_sample_format` S32 arm (25-32) + S24 shift 1-7 (bps 17-23): unreachable through claxon (F-001). The unit test asserting them is the only caller.
- `flac.frame.absent` branch (decoder.rs:170-177): author's own comment says defensive; claxon returns Ok(None) only at EOF which is pre-rejected — harmless.

## 8. Dependencies & flags
claxon =0.4.3 (optional, `codec-flac` in default set — release lane includes it). No symphonia-codec-flac anywhere in Cargo.lock (rg: only symphonia aac/alac/vorbis/bundle-mp3/core). Purity record docs/dependencies/records/claxon-0.4.3.md referenced, not opened (doc-audit territory).

## 9. Open questions for the next agent
- Does any doc/matrix row claim 32-bit or 24-bit-hi-res FLAC support that F-001 makes false? Check the FLAC matrix row's evidence strings.
- metadata.rs is assigned elsewhere — confirm its sample-rate doc lie (F-004) lands in that audit.
- flac/fuzz.rs harness (`fuzz_codec_flac`) wiring status — claimed "honestly pending"; not audited here.
