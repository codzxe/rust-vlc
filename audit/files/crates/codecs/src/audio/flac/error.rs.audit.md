# AUDIT: crates/codecs/src/audio/flac/error.rs
| Field | Value |
|---|---|
| Auditor | W07-A08 |
| Date | 2026-11-23 |
| Lines / functions | 210 / 5 (enum + impl, From, Detail, 2 constructors) + 3 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (indirectly, as the error path of every shipped FLAC decode): flac/decoder.rs:558,609 `FlacError::BadState`, and the `corrupt`/`unsupported` constructors are called 13×/11×/3× across decoder.rs / metadata.rs / seek.rs (rg counts), all behind `FlacDecoder` ← pipeline_host.rs:253-260 ("flac" row, flac-in-wav/mp4/mkv lanes). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `./target/debug/deps/codecs-a0d7a93b7072ff58 --test-threads=2 flac::error` → 3 passed, exit 0; `rg -c` constructor usage across the flac module |

## 1. One-paragraph verdict
A small, correct, fully-exercised error taxonomy: three internal variants (Unsupported/Corrupt/BadState) mapped one-to-one onto `EngineError` kinds at stage `Decode`, with the human-readable reason riding in the `source` chain (never in `Display`), structured `CorruptDetail` preserved, and stable dotted context codes. Every constructor and variant is used by the shipped decode paths; the three tests pin the exact rendered strings, the stage, the source text, and the structured class — and they pass under my execution. No defects found; the checklist run is documented below because zero findings demands it.

## 2. What actually works (verified)
— `./target/debug/deps/codecs-a0d7a93b7072ff58 --test-threads=2 flac::error` → `test result: ok. 3 passed` (rendered-string pinning incl. `[decode] unsupported in flac (flac.streaminfo.missing)`; structured `CorruptDetail::Truncated { expected: 34, found: 10 }` round-trip; BadState is `Decoder` kind, not `Corrupt`).

## 3. Findings
None. Exactly what was checked and how:
- Fakeness: no todo!/unimplemented!/unreachable! outside test `unreachable!` guards on impossible arms; no constant-returning bodies; both shorthand constructors (120-138) store their arguments (usage counts above prove they are not dead).
- Correctness: no unwrap/expect/index/panic in any production line (read all 97 non-test production lines); `Detail` is a plain String carrier with Display+Error (108-117) — no panics possible; the `From` impl (99-103) delegates to `into_engine_error` (no divergence between the two conversion paths).
- Mapping fidelity: the doc table (27-31) claims Unsupported→Unsupported, Corrupt→Corrupt, BadState→Decoder(BadState), all at stage Decode — each pinned by an executed test assertion (not just read).
- Severity scale: nothing rises above hygiene. The only cosmetic candidate (String allocation per error in `FormatInfo { name: Some("flac".to_string()) }`, line 71) is on the error path of a decode failure — not a hot path.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FlacError (enum) | 32-59 | REAL | — | 3 variants, all used |
| FlacError::into_engine_error | 64-96 | REAL | — | stage/kind/context/source mapping test-pinned |
| From<FlacError> for EngineError | 99-103 | REAL | — | delegates, no divergence |
| Detail (+Display, +Error) | 105-117 | REAL | — | source-chain carrier |
| unsupported / corrupt (ctors) | 120-138 | REAL | — | used 11×/13× in decoder/metadata/seek |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| taxonomy_maps_each_variant_to_its_public_kind | Yes | asserts exact Display strings, stage, and source text for all 3 variants |
| corrupt_variant_carries_the_structured_class | Yes | pattern-matches the structured CorruptDetail back out of the public error |
| bad_state_is_a_decoder_kind_not_corrupt | Yes | kind discrimination pinned |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "typed, stage-tagged internal errors that map one-to-one onto EngineError kinds" | error.rs:2-3 | REAL | executed tests pin all three mappings |
| "the engine treats these as terminal, correctly" | error.rs:9-10 | CLAIMS ANOTHER COMPONENT | this file only sets kind+stage; whether playback-engine treats Unsupported as terminal is unverifiable here (outside my files) |
| "FLAC frames are independent … a damaged packet never poisons the next" | error.rs:12-15 | REAL (format fact) | each FLAC frame carries its own header+subframes+CRC (spec §6); consistent with flac decoder's per-packet design |
| "Nothing panics; every constructor is total; no unwrap on any input path" | error.rs:17-18 | REAL | read of all production lines; workspace clippy -D warnings CLEAN (baseline cargo-clippy.txt) |

## 7. Dead / duplicated / unreachable code
None. (Pattern-duplication note: mp3/error.rs and pcm/error.rs follow the same 2.B4/2.B5 shape by design — a deliberate crate convention, not accidental copy-paste; the naming and codes differ per codec.)

## 8. Dependencies & flags
media-types only (ErrorKind/EngineError/ErrorContext/CorruptDetail/FormatInfo). No feature gates.

## 9. Open questions for the next agent
- Verify the "engine treats Unsupported as terminal" policy in playback-engine (whoever owns session.rs/pipeline.rs) — the claim originates here but the behavior lives there.
