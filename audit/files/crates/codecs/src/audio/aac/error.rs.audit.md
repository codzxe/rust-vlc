# AUDIT: crates/codecs/src/audio/aac/error.rs
| Field | Value |
|---|---|
| Auditor | W07-A08 |
| Date | 2026-11-21 |
| Lines / functions | 187 / 8 (1 enum, 2 conversions, 1 carrier type, 2 constructors, 2 tests) |
| File verdict | REAL (small, correct, tested mapping module on the live AAC lane) |
| Reachable from shipped entry point? | YES (call chain: `lumen` open MP4/ADTS → `AacDecoder` constructs `AacError`s (aac/decoder.rs:159-198 `aac.he_sbr`/`aac.profile`/`aac.channel_config_pce`/`aac.channel_count`) → `into_engine_error` at the contract boundary. The AAC lane is exercised end-to-end in baseline: `sample-aac.mp4` → Buffering → Playing → Stopped (audit/runs/baseline/INDEX.md corrected MP4 matrix); `sample-420.mp4` (H.264+AAC) plays. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg -n "aac\.(he_sbr\|profile\|channel\|object)" aac/decoder.rs` (code usage) · family survey: `rg -c` of `From`/`corrupt`/variants across all 8 audio error.rs files (7 identical 3-variant shapes, pcm the 2-variant outlier) · read media-types error kinds via usage · `sed -n` quote verification |

## 1. One-paragraph verdict
A competent, honest error taxonomy: three variants (`Unsupported`/`Corrupt`/`BadState`) mapping one-to-one onto `ErrorKind::Unsupported`/`Corrupt`/`Decoder{BadState}` at stage `Decode`, with the human-readable reason carried in the `source` chain (not `Display`) and stable dotted codes the decoder actually mints. The mapping is pinned by a test that asserts the exact rendered strings and source texts, and the module doc's taxonomy (HE-AAC refused as `aac.he_sbr`, profiles as `aac.profile`, PCE/channel gates) matches real constructor sites in decoder.rs. The only defect is structural: this file is one of seven near-verbatim copies of the same pattern in the audio tree (plus a drifted eighth), so any fix to the shared shape must be replicated seven times.

## 2. What actually works (verified)
— Mapping verified variant-by-variant against the doc table (lines 32-36): Unsupported → `ErrorKind::Unsupported` with `FormatInfo{name:"aac"}` (66-76), Corrupt → `ErrorKind::Corrupt{detail}` (77-86), BadState → `Decoder{BadState}` with code `aac.not_configured` (87-93). The in-file test asserts exact Display strings ("[decode] unsupported in aac (aac.he_sbr)" etc.) and source texts — ran green in baseline #3 (INDEX.md).
— Doc honesty: "Nothing panics; every constructor is total; no `unwrap` on any input path" (22-23) — verified by reading the whole file (no unwrap/expect/panic/indexing).
— The taxonomy's claimed sites exist: `aac.he_sbr` (decoder.rs:159), `aac.profile` (165/177), `aac.channel_config_pce` (191), `aac.channel_count` (198) — the doc is not describing imaginary codes.

## 3. Findings
### F-001 [S3] Seventh copy of the shared error-taxonomy pattern — family already drifting
Where: crates/codecs/src/audio/aac/error.rs:104-136 (with the family: flac/mp3/mp2/alac/opus/vorbis/error.rs)
Code:
```
/// Chain-root carrier for the human-readable reason (the 2.B4
/// `FeatureDetail` pattern verbatim).
#[derive(Debug, Clone)]
struct Detail(String);
```
What is wrong: the `Detail` newtype + `Display` + `Error` impl, the `into_engine_error` match, the `unsupported`/`corrupt` constructors, and the test skeletons are copy-pasted across seven codec modules (survey: aac/mp3/mp2/flac/alac/opus/vorbis all 3-variant + `From` + `corrupt`, 184-214 lines each, 1534 lines total for the family). The eighth copy (pcm/error.rs) has already drifted: no `Corrupt` variant (by design) but also no `From<PcmError> for EngineError` (98-102 here) — its decoder must spell `.map_err(PcmError::into_engine_error)` at every site instead of `?`.
Why it matters: a change to the shared shape (e.g., attaching structured offsets to `Unsupported`, or fixing a Display format) must be hand-replicated seven times; the pcm drift demonstrates the copies are already diverging silently.
What correct looks like: one `codecs::audio::error` module with a generic `CodecError` (parameterized by codec name), each codec keeping only its code constants.
Confidence: HIGH — Verification: read + rg survey of all 8 files (counts pasted in Commands run).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| enum AacError (3 variants) | 38-58 | REAL | — | `code: &'static str` keeps codes static; detail Strings heap-allocated on error paths only |
| AacError::into_engine_error | 63-95 | REAL | — | mapping matches doc table exactly; stage always Decode |
| impl From<AacError> for EngineError | 98-102 | REAL | — | enables `?` in decoder |
| struct Detail + Display + Error | 106-115 | REAL | F-001 | 7th copy of the pattern |
| fn unsupported | 118-123 | REAL | — | total |
| fn corrupt | 126-136 | REAL | — | total; structured CorruptDetail param |
| test taxonomy_maps_each_variant_to_its_public_kind | 142-174 | REAL | — | exact Display + stage + source-text asserts — fails on any mapping/format drift |
| test bad_state_is_a_decoder_kind_not_corrupt | 176-186 | REAL | — | pins misuse ≠ corruption |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| taxonomy_maps_each_variant_to_its_public_kind | YES | three exact rendered strings + source texts + stage |
| bad_state_is_a_decoder_kind_not_corrupt | YES | kind-shape match + negative match |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| variant→kind mapping table | error.rs:32-36 | REAL | code 63-95; test 142-174 |
| "HE-AAC (SBR/PS), honestly absent — object type 5/29 refused typedly, never silently downgraded" | error.rs:10-13 | REAL | decoder.rs:159 `aac.he_sbr` refusal site |
| "channel configuration 0 (PCE-in-band …); more than 2 channels" refused | error.rs:13 | REAL | decoder.rs:191, 198 |
| "a damaged packet never poisons the next — the walker resyncs at the next syncword" | error.rs:18-20 | REAL-UNPROVEN | decoder-side claim (resync walker is decoder.rs, another agent's file); codes exist, resync behavior not re-verified here |
| "Nothing panics; every constructor is total; no `unwrap`" | error.rs:22-23 | REAL | whole-file read |

## 7. Dead / duplicated / unreachable code
F-001 (the duplication itself). Nothing dead within the file — all variants, constructors, and conversions have live callers in aac/decoder.rs (rg `AacError::` / `unsupported(` / `corrupt(` sites).

## 8. Dependencies & flags
media-types only (ErrorKind/EngineError/CorruptDetail). No feature gate; compiled with the aac module in the shipped build. No unsafe.

## 9. Open questions for the next agent
- aac/decoder.rs (its agent): the resync-walker claim (doc lines 18-20) — is there a real syncword rescan after a damaged ADTS frame, and does the engine's `per_frame_damage` classification (playback-engine pipeline.rs:2378) treat `aac.adts.sync` as per-frame?
- pcm/error.rs (my next file): the missing `From` impl is the family drift canary — if the family is ever unified (F-001), which shape wins?
