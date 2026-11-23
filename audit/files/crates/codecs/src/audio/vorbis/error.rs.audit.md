# AUDIT: crates/codecs/src/audio/vorbis/error.rs
| Field | Value |
|---|---|
| Auditor | W07-A02 |
| Date | 2026-11-23 |
| Lines / functions | 214 / 5 production items (VorbisError enum, into_engine_error, From impl, Detail+Display, unsupported/corrupt ctors) + 3 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — headers.rs:31/comments.rs:27/decoder.rs:99 consume it; VorbisDecoder constructed at apps/cli/src/pipeline_host.rs:272-274 (`self.seeded(track, config, VorbisDecoder::new)`) and apps/web/src/host.rs (vorbis arm). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/codecs-a0d7a93b7072ff58 vorbis::error` → `test result: ok. 3 passed` (exit 0); `rg` usage/caller checks; `sed -n` quote verification |

## 1. One-paragraph verdict
A small, honest error-taxonomy module: three variants, a lossless map onto EngineError kinds, total constructors, zero panics, and three executed tests that pin the exact public Display strings, the structured `CorruptDetail` passthrough, and the BadState kind split. Nothing here is fake or shallow. The only debt is pattern-level: the private `Detail(String)` source-carrier is copy-pasted into 12 codec error modules (this is the 12th copy), and the `From<VorbisError> for EngineError` impl appears unused because every call site converts explicitly.

## 2. What actually works (verified)
- `./target/debug/deps/codecs-a0d7a93b7072ff58 vorbis::error` → 3 passed (exit 0): `taxonomy_maps_each_variant_to_its_public_kind`, `corrupt_variant_carries_the_structured_class`, `bad_state_is_a_decoder_kind_not_corrupt`.
- Callers verified real by rg: headers.rs:31 (`use super::error::{corrupt, unsupported}`), comments.rs:27, decoder.rs:99 — with ~15 construction sites (decoder.rs:458, 541, 745, 752, 959 BadState; headers.rs ident.magic/version/channels; comments.rs comment.magic/truncated).
- Mapping table in the doc comment (lines 31-35) matches the code exactly (verified by the executed test asserting Display + stage).

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/audio/vorbis/error.rs:103-107
Code:
```
impl From<VorbisError> for EngineError {
    fn from(value: VorbisError) -> EngineError {
        value.into_engine_error()
    }
}
```
What is wrong: dead in practice — every conversion site in the vorbis module is explicit (`.into_engine_error()` or `.map_err(VorbisError::into_engine_error)`: decoder.rs:320, 324, 715, 802; headers.rs:95-97, 103-105; comments.rs:158). The `fill(bytes)?` at decoder.rs:1022 stays inside `Result<_, VorbisError>` (pooled_sample_slot, decoder.rs:1011-1014), so it does not exercise `From` either.
Why it matters: trivial dead surface; a maintainer may assume `?`-based conversion is in use somewhere and "refactor" the explicit calls away, silently changing nothing.
Confidence: MEDIUM — Verification: read (rg over all VorbisError uses; not proven by compilation with the impl removed — that would need a build).

F-002 [S4]
Where: crates/codecs/src/audio/vorbis/error.rs:112-121
Code:
```
#[derive(Debug, Clone)]
struct Detail(String);

impl std::fmt::Display for Detail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
```
What is wrong: the twelfth byte-identical private `Detail` in the tree (vorbis, opus, flac, pcm, mp2, mp3, alac, aac, adpcm, vp9, dnx, mpeg12 — rg `^struct Detail` = 12 hits). A single shared carrier in codec-api would delete ~120 duplicated lines across codecs. Deliberate per-module isolation is arguable; the count is not.
Confidence: HIGH — Verification: ran rg (12 files listed).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| VorbisError (enum) | 36-63 | REAL | — | 3 variants, doc'd codes |
| into_engine_error | 68-100 | REAL | — | exact kind/stage/context mapping; executed by 3 tests |
| From<VorbisError> for EngineError | 103-107 | REAL (unused) | S4 | F-001 |
| Detail + Display + Error | 112-121 | REAL | S4 | F-002 duplication |
| unsupported / corrupt ctors | 124-142 | REAL | — | total, no panics; used across module |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| taxonomy_maps_each_variant_to_its_public_kind | YES | pins exact Display strings, stage, and source-chain text for all three variants |
| corrupt_variant_carries_the_structured_class | YES | destructures ErrorKind::Corrupt and asserts expected/found values survive |
| bad_state_is_a_decoder_kind_not_corrupt | YES | kind-discrimination assert |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "typed, stage-tagged internal errors that map one-to-one onto EngineError kinds" | error.rs:1-3 | REAL | executed tests assert the 3 mappings |
| "Nothing panics; every constructor is total; no unwrap on any input path" | error.rs:21-22 | REAL | constructors are pure data builders (read); no unwrap/expect/index in file (read, full file) |
| variant↔kind table | error.rs:31-35 | REAL | executed test |
| "Isolation is overlap-scoped ... poisons at most itself and the next packet's overlap half" | error.rs:14-19 | REAL-UNPROVEN (here) | a decoder.rs behavioral claim documented in this file; verification belongs to W07-A03 (vorbis/decoder.rs) — not testable from error.rs |

## 7. Dead / duplicated / unreachable code
- F-001 (From impl), F-002 (12th Detail copy). Nothing else.

## 8. Dependencies & flags
- media_types only (CorruptDetail, ErrorContext, FormatInfo, EngineError, ErrorKind, PipelineStage). std-only otherwise. No feature gates.

## 9. Open questions for the next agent
- W07-A03 (vorbis/decoder.rs): please verify the overlap-scoped isolation claim quoted in this file's header (error.rs:14-19) — it is this file's only borrowed behavioral claim.
