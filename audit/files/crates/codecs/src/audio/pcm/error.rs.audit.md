# AUDIT: crates/codecs/src/audio/pcm/error.rs
| Field | Value |
|---|---|
| Auditor | W07-A08 |
| Date | 2026-11-23 |
| Lines / functions | 153 / 4 (enum + impl, Detail, 1 constructor) + 3 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — the error path of the FIRST-VERTICAL decoder: apps/cli/src/pipeline_host.rs:347 `codec if codec.starts_with("pcm_")` → `PcmDecoder` (seeded `with_format` at :359); constructors used 23× in pcm/decoder.rs (rg count); PCM is one of the two decoders the baseline engine-info actually lists (`decoders: mjpeg, pcm_s16le`, rc-smoke.txt:128). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `./target/debug/deps/codecs-a0d7a93b7072ff58 --test-threads=2 pcm::error` → 3 passed, exit 0; `rg -c` usage across pcm/decoder.rs; spot-read pcm/decoder.rs:37-51,185-198 for the concealment cross-reference |

## 1. One-paragraph verdict
The smallest taxonomy in the family and correct for the format: PCM has no in-packet entropy coding, so the only fatal errors are configuration-time (`Unsupported`) and contract misuse (`BadState`); byte damage is concealed by the decoder's whole-sample-frame prefix, which is why there is no `Corrupt` variant at all — a deliberate, documented design decision the decoder actually implements (decoder.rs:37-51, `whole_frame_prefix` at 185-198). The mapping is test-pinned (rendered strings, stage, source chain, kind discrimination), all production lines are panic-free by construction, and everything is on the shipped path. No defects.

## 2. What actually works (verified)
— `./target/debug/deps/codecs-a0d7a93b7072ff58 --test-threads=2 pcm::error` → `test result: ok. 3 passed` (exact Display pins `[decode] unsupported in pcm (pcm.waveformat.tag)` / `[decode] decoder: invalid state transition (pcm.not_configured)`; source-chain text; BadState-is-not-Corrupt).
— Cross-reference verified: decoder.rs:47-51 "A non-empty prefix with a damaged tail is concealed: the prefix is emitted" — the doc's justification for the missing Corrupt variant is real code, not prose.

## 3. Findings
None. What was checked and how:
- Fakeness: no placeholders; the single constructor (88-93) stores its args; `Unsupported`/`BadState` both used (23 hits in pcm/decoder.rs); `Detail` used at line 60.
- Correctness: no unwrap/expect/index/panic in the 93 production lines (read); no `From` impl here (unlike FLAC) — no dual conversion path to diverge; `ErrorContext::new("pcm.not_configured")` is a literal, no allocation risk.
- Mapping fidelity: doc table (21-24) pinned by executed tests.
- Duplication: byte-similar to flac/error.rs and the 2.B4 MJPEG pattern — intentional crate convention (each carries codec-specific codes/format names); not copy-paste rot: the variant sets differ deliberately (no Corrupt here, with the reason documented and cross-verified).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PcmError (enum) | 26-42 | REAL | — | 2 variants, both used |
| PcmError::into_engine_error | 47-69 | REAL | — | stage Decode; kinds pinned by tests |
| Detail (+Display, +Error) | 76-85 | REAL | — | source-chain carrier |
| unsupported (ctor) | 88-93 | REAL | — | used throughout pcm/decoder.rs |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| taxonomy_maps_each_variant_to_its_public_kind | Yes | exact rendered strings + stage for both variants |
| unsupported_detail_rides_in_the_source_chain | Yes | asserts the source chain text, not just the kind |
| bad_state_is_a_decoder_kind_not_corrupt | Yes | misuse never reported as media damage |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Byte-level damage inside a packet is *always* concealable (the whole-sample-frame prefix … see `decoder.rs`)" | error.rs:9-12 | REAL | decoder.rs:37-51 + `whole_frame_prefix` (185-198) read; concealment implemented |
| "Nothing panics; every constructor is total" | error.rs:12 | REAL | read of all production lines |
| "typed, stage-tagged … map one-to-one onto EngineError kinds" | error.rs:2-3 | REAL | executed tests |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
media-types only. No feature gates — PCM is an in-house decoder, unconditional in every build (codecs/Cargo.toml comment, "in-house decoders … stay unconditional").

## 9. Open questions for the next agent
- None for this file. (pcm/decoder.rs itself — the concealment accounting and the `Reported` observability — belongs to its owner; the cross-reference I verified only covers the prefix rule.)
