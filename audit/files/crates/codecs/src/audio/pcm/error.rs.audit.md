# AUDIT: crates/codecs/src/audio/pcm/error.rs
| Field | Value |
|---|---|
| Auditor | W07-A08 |
| Date | 2026-11-21 |
| Lines / functions | 153 / 7 (1 enum, 1 conversion, 1 carrier type, 1 constructor, 3 tests) |
| File verdict | REAL (correct, tested, intentionally minimal taxonomy; one family-drift hygiene defect) |
| Reachable from shipped entry point? | YES (call chain: `lumen` open → `PcmDecoder` (pcm/decoder.rs — `PcmError` constructed at decoder.rs:624/629/688/715/783) ← descriptor dispatch. PCM is the one codec `lumen engine-info` even admits (`decoders: mjpeg, pcm_s16le`) and the AVI first-vertical lane (`avi_mjpeg_pcm.avi`, 60/60 frames in baseline decode-bench — INDEX.md) runs it end-to-end. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg -n "into_engine_error\|PcmError::" pcm/decoder.rs` (usage sites; explicit `.map_err` everywhere — no `?` conversion) · `rg -n "prefix\|conceal\|whole-sample\|damag" pcm/decoder.rs` (the concealment claim's referent: `whole_frame_prefix` decoder.rs:198, prefix rule 37-48) · family survey (see aac/error.rs audit) · `sed -n` quote verification |

## 1. One-paragraph verdict
The two-variant taxonomy is a deliberate, documented design: unpacked PCM has no in-packet headers, byte-level damage is concealable via the whole-sample-frame prefix (real code: `whole_frame_prefix`, decoder.rs:198), so `Corrupt` genuinely has no production site and is absent rather than stubbed. The `Unsupported` mapping carries the reason in the `source` chain with exact rendered-string tests, `BadState` maps to `Decoder{BadState}` and never to `Corrupt` (pinned twice). The defects are family-level: this is the eighth drifted copy of the shared pattern (no `From<PcmError> for EngineError` — every sibling has one — forcing `.map_err(PcmError::into_engine_error)` at every decoder call site), and the `Detail` carrier is duplicated eight ways.

## 2. What actually works (verified)
— Mapping verified: Unsupported → `ErrorKind::Unsupported` + `FormatInfo{name:"pcm"}` (50-60), BadState → `Decoder{BadState}` + `pcm.not_configured` (61-67). Tests assert exact Display strings (99-119), the source-chain ride (121-133), and the misuse-≠-corruption kind split (135-152) — all green in baseline #3 (INDEX.md: 1681 passed; the PCM lane is also the AVI first-vertical's audio half, 60/60 frames in decode-bench).
— The "damage is always concealable" doc claim (9-12) points at real code: `whole_frame_prefix` (decoder.rs:198) + the prefix/conceal branch (decoder.rs:217-221) — the absent `Corrupt` variant is justified, not lazy.
— "Nothing panics; every constructor is total" (12) — verified by whole-file read (no unwrap/expect/panic).

## 3. Findings
### F-001 [S3] Eighth drifted copy of the shared taxonomy; missing `From` conversion the seven siblings have
Where: crates/codecs/src/audio/pcm/error.rs:25-42 and 47-70 (family evidence: aac/error.rs:98-102)
Code:
```
pub(crate) enum PcmError {
    /// A descriptor, stream-facts blob or parameter combination
    /// outside the supported PCM subset (unknown codec descriptor,
```
(no `impl From<PcmError> for EngineError` exists in this file; contrast aac/error.rs:98-102)
What is wrong: the `Detail` carrier, `unsupported` constructor, and `into_engine_error` match are the family pattern copy-pasted (8th copy; 1534 lines across the audio tree). This copy silently dropped the `From` conversion every sibling carries, so pcm/decoder.rs cannot use `?` — it spells `.map_err(PcmError::into_engine_error)` at 624/629/688/701/715/783 instead (six sites of boilerplate the siblings avoid). The two-variant shape itself is justified (see §2); the drift is not.
Why it matters: same as the aac S3 — the copies diverge (this one already has), and every shared-shape fix is an N-way manual replication.
What correct looks like: one generic `CodecError` in `codecs::audio` (name-parameterized), each codec keeping its code constants and its honest variant subset.
Confidence: HIGH — Verification: read + rg usage sites + family survey.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| enum PcmError (2 variants) | 26-42 | REAL | — | no Corrupt — justified by the concealment design (decoder.rs:37-48,198) |
| PcmError::into_engine_error | 47-69 | REAL | — | stage Decode; detail rides in source |
| struct Detail + Display + Error | 76-85 | REAL | F-001 | 8th copy |
| fn unsupported | 88-93 | REAL | — | total |
| (absent) `From<PcmError>` | — | MISSING | F-001 | siblings have it; six `.map_err` sites in decoder.rs pay for it |
| test taxonomy_maps_each_variant_to_its_public_kind | 99-119 | REAL | — | exact rendered strings |
| test unsupported_detail_rides_in_the_source_chain | 121-133 | REAL | — | source text asserted |
| test bad_state_is_a_decoder_kind_not_corrupt | 135-152 | REAL | — | kind + negative match |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| taxonomy_maps_each_variant_to_its_public_kind | YES | exact Display + stage for both variants |
| unsupported_detail_rides_in_the_source_chain | YES | exact source-chain text |
| bad_state_is_a_decoder_kind_not_corrupt | YES | kind match + negative Corrupt match |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the only fatal conditions are configuration-time … and contract misuse" | error.rs:6-9 | REAL | enum has exactly those two variants; decoder concealment code exists (decoder.rs:217-221) |
| "Byte-level damage inside a packet is *always* concealable (the whole-sample-frame prefix … see decoder.rs)" | error.rs:9-12 | REAL | `whole_frame_prefix` decoder.rs:198; prefix rule documented decoder.rs:37-48 |
| "Nothing panics; every constructor is total" | error.rs:12 | REAL | whole-file read |
| variant→kind table | error.rs:21-24 | REAL | code 47-69; tests |

## 7. Dead / duplicated / unreachable code
F-001 (8th copy + missing From). Nothing dead in-file: both variants and the constructor have live call sites (rg output above).

## 8. Dependencies & flags
media-types only. No feature gate; always compiled. No unsafe. Zero panic paths.

## 9. Open questions for the next agent
- pcm/decoder.rs (its agent): the concealment claim's other half — a damaged-tail packet reports `DecodeOutcome::Recovered { reason: ConcealedFrame }` with the prefix emitted (decoder.rs:47-48 says so); confirm the recovered accounting matches the engine's `per_frame_damage` classification.
- If the family unification (F-001) lands, `pcm.waveformat.*` codes must survive verbatim — the corpus differential and CLI unsupported-path tests pin some of them (apps/cli/tests/unsupported_path.rs).
