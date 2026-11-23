# AUDIT: crates/codecs/src/audio/pcm/mod.rs
| Field | Value |
|---|---|
| Auditor | W07-A09 |
| Date | 2026-11-30 |
| Lines / functions | 551 / 9 (+8 test fns) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — CLI `decoder_for` `pcm_*` descriptors (apps/cli/src/pipeline_host.rs:355-361) -> `PcmDecoder::with_format`/configure -> `PcmFormat::from_descriptor`/`PcmSource` (this file); `decode_pcm` free surface is the fuzz/corpus entry (`crates/codecs/src/audio/pcm/fuzz.rs`, `tests/corpus.rs:46`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/codecs-a0d7a93b7072ff58 audio::pcm::tests` -> 9 passed (exit 0); full `audio::pcm` -> 39 passed (exit 0); sed/rg verification (exit 0) |

## 1. One-paragraph verdict
The vocabulary and free surface of the in-house PCM decoder (no symphonia-codec-pcm — the dependency decision is recorded in the module doc and Cargo.toml confirms no such dep). Every mapping (bytes-per-sample, descriptor strings, endianness, output SampleFormat) is exact and round-trip tested across all 11 formats × interleaved/planar × 1-3 channels; byte-order normalization is byte-for-byte pinned (s24 3-byte group reversal, f32/f64 native re-read, u8 identity, ragged-tail accounting). The one nit: the module doc's zero-copy claim is true only of the contract path (`packet.data.clone()` Arc clone, decoder.rs:741) — the `decode_pcm` free surface always copies (`data.to_vec()`), which the doc does not distinguish.

## 2. What actually works (verified)
- `./target/debug/deps/codecs-a0d7a93b7072ff58 audio::pcm::tests` -> `test result: ok. 9 passed; 0 failed`; `audio::pcm` (whole family) -> `39 passed; 0 failed`.
- `every_format_decodes_a_clean_buffer_value_exactly` sweeps 11 formats × 2 layouts × 3 channel counts and asserts per-group byte equality against the expected swap — any byte-order or width bug fails it.
- Descriptor vocabulary cross-checked against the demuxer: `crates/demux/src/wav/format.rs` / `avi` emit `pcm_s16le` etc.; `from_descriptor` rejects `pcm`, `pcm_s20le`, `pcm_u16le`, `pcm_f80le` (test-pinned).
- Zero-copy claim spot-verified: contract path decoder.rs:740-741 `if !plan.swap && prefix.cut == data.len() { packet.data.clone() // zero-copy: the bytes ARE the frame }` — Arc clone, real.

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/audio/pcm/mod.rs:27-30
Code:
```
//! exactly that (and can pass a clean native-endian packet
//! through zero-copy, which a generic crate has no reason to
//! promise).
```
What is wrong: the sentence attaches zero-copy to "the in-house decoder" generally; in this very file the only public decode entry, `decode_pcm`, returns `samples: payload.converted.unwrap_or_else(|| data.to_vec())` (line 286) — always an owned copy. Zero-copy holds only for the contract path in `decoder.rs` (Arc clone). Why it matters: the doc overstates the free surface; a reader auditing `decode_pcm` finds a mandatory copy. What correct looks like: scope the claim to `PcmDecoder` (the contract surface) or return a borrowed/cow type here. Confidence: HIGH — Verification: read + ran (both surfaces read; tests run).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PcmFormat::ALL/bytes_per_sample | 133-157 | REAL | — | 1/2/3/4/8 exact, test-pinned |
| PcmFormat::descriptor | 163-177 | REAL | — | matches demuxer vocabulary |
| PcmFormat::from_descriptor | 183-188 | REAL | — | linear scan; foreign descriptors None (tested) |
| PcmFormat::is_little_endian | 193-207 | REAL | — | None for U8 correct |
| PcmFormat::output_sample_format | 212-221 | REAL | — | U8/S16/S24/S32/F32/F64 mapping exact |
| PcmSource::descriptor | 243-249 | REAL | — | `_planar` suffix round-trip tested |
| DecodedPcm | 256-267 | REAL | — | dropped_tail observability, tested |
| decode_pcm | 281-289 | REAL | — | typed config errors only (channels/rate zero); copy on passthrough = F-001 |
| 8 tests | 295-550 | REAL tests | — | mutation-sensitive (byte-level asserts) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| descriptor_vocabulary_roundtrips_every_format | YES | exact Some(format) + widths + 7 foreign rejections |
| every_format_decodes_a_clean_buffer_value_exactly | YES | per-sample-group byte equality incl. swap direction on any host |
| s24_groups_reverse_as_three_byte_units | YES | exact expected byte triples per host endianness |
| f32/f64_values_survive_the_byte_order_normalization | YES | semantic values re-read via from_ne_bytes |
| u8_is_identity_and_byte_order_free | YES | identity assert |
| damaged_tail_is_reported_not_failed | YES | len 4 + dropped 1 exact |
| decode_pcm_rejects_broken_configurations_typed | YES | exact context codes pcm.channels.zero / pcm.rate.zero |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| in-house decision vs symphonia-codec-pcm | mod.rs:5-37 | REAL | no symphonia-codec-pcm in Cargo.toml/lock |
| vocabulary s16/s24/s32/f32/f64 × LE/BE × planar + u8 | mod.rs:39-56 | REAL | enum + tests; demuxer emits same names |
| "emits AudioFrames in the source's own layout ... byte order normalized to the host" | mod.rs:69-78 | REAL | PcmPlan swap logic + byte-exact tests |
| "every decoded byte is a permuted input byte" (no depth/rate/channel conversion) | mod.rs:75-78 | REAL | decode_payload only swaps groups |
| zero-copy claim | mod.rs:27-30 | PARTLY | true for contract path only — F-001 |
| gaps: bare `pcm`, EXTENSIBLE masks, mid-stream changes typed Unsupported | mod.rs:59-67 | REAL-UNPROVEN | typed-refusal code paths seen in decoder.rs configure; EXTENSIBLE mask path not executed here |

## 7. Dead / duplicated / unreachable code
None found. All public items have callers (decoder.rs, fuzz.rs, tests/corpus.rs, demuxer vocabulary mirror).

## 8. Dependencies & flags
Std-only (no external deps — module is unconditional, not feature-gated). Consumers: pcm/decoder.rs (contract), pcm/fuzz.rs, tests/corpus.rs, tests/pcm_*.rs; demux wav/avi mirror the descriptor strings.

## 9. Open questions for the next agent
- pcm/decoder.rs (1.5k+ LOC: pooled conversion, ReseedingDecoder interplay, with_format parse) is a separate assignment — its `swap_groups_into` differential test and WAVEFORMATEX parsing were only spot-read here.
- Does the demuxer ever emit a planar descriptor the AVI/WAV lanes never produce (dead half of the matrix in practice)?
