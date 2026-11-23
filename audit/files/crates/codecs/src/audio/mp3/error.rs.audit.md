# AUDIT: crates/codecs/src/audio/mp3/error.rs
| Field | Value |
|---|---|
| Auditor | W07-A04 |
| Date | 2026-11-23 |
| Lines / functions | 186 / 5 (+2 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — CLI `.mp3` open lane (apps/cli/src/containers/mod.rs:66 → es::open_mpeg_audio → provider "mp3" arm, pipeline_host.rs:261-265 → Mp3Decoder) — a damaged frame surfaces `corrupt("mp3.frame.decode", …)` (decoder.rs:150-160) and decode-before-configure surfaces `Mp3Error::BadState` → "mp3.not_configured" (decoder.rs:505-516) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cargo test -p codecs --lib audio::mp3::error` → 2 passed (0.00s) |

## 1. One-paragraph verdict
A clean, total, panic-free error taxonomy: three variants (Unsupported/Corrupt/BadState) mapping one-to-one onto EngineError kinds at stage Decode, structured `CorruptDetail`, human-readable reason riding in the error `source` chain (never in `Display`), stable dotted context codes, and a test that pins the exact rendered strings, stages, kinds, and source text for all three variants (executed: PASS). The single defect is cosmetic misattribution: `FormatInfo.name` is hard-coded `"mp3"` while the owning `Mp3Decoder` also accepts descriptors `"mp1"` and `"mp2"` (decoder.rs:437-439) — an MP1/MP2 stream configured through this decoder reports "unsupported in mp3". Note this file's taxonomy claims are contradicted elsewhere in the family: xing.rs's `parse_vbri` panics on a 14–21-byte tail (see xing.rs audit F-001), violating this module's "Nothing panics" contract — the contract is honored here but not by its sibling.

## 2. What actually works (verified)
— `cargo test -p codecs --lib audio::mp3::error` → 2 passed (`taxonomy_maps_each_variant_to_its_public_kind` asserts exact Display + stage + source for all three variants; `bad_state_is_a_decoder_kind_not_corrupt` asserts the kind discrimination).

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/audio/mp3/error.rs:64-70 + crates/codecs/src/audio/mp3/decoder.rs:437-439
Code:
```
                ErrorKind::Unsupported {
                    format: Some(FormatInfo {
                        name: Some("mp3".to_string()),
                        sniffed: Vec::new(),
                    }),
                },
```
What is wrong: `Mp3Decoder::accepts` matches `"mp1" | "mp2" | "mp3"`, but every Unsupported error this module mints names the format `"mp3"` — an `mp1`/`mp2` descriptor configured through this decoder mis-reports its format. (The shipped CLI routes `mp2` to the separate `Mp2Decoder`, which has its own `Mp2Error` — crates/codecs/src/audio/mp2/decoder.rs:36 — so the shipped path is unaffected; the misattribution is reachable only through the pub codec API with an `mp1`/`mp2` descriptor on `Mp3Decoder`.) What correct looks like: thread the configured descriptor into the error or use a family name. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Mp3Error (enum) | 33-56 | REAL | — | 3 variants, documented kind/stage table |
| into_engine_error | 61-93 | REAL | S4 | hard-coded "mp3" FormatInfo (F-001) |
| From<Mp3Error> | 96-100 | REAL | — | delegates |
| Detail + Display + Error | 105-114 | REAL | — | chain-root carrier, no-secrets Display |
| unsupported / corrupt ctors | 117-135 | REAL | — | total, no panics |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| taxonomy_maps_each_variant_to_its_public_kind | YES | exact Display strings, stage, and source text per variant |
| bad_state_is_a_decoder_kind_not_corrupt | YES | kind discrimination via matches! |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Nothing panics; every constructor is total; no `unwrap` on any input path" (L18-19) | error.rs | REAL for this file | constructors are pure struct literals; FALSE for the family — xing.rs parse_vbri panics (xing.rs audit F-001) |
| "typed, stage-tagged internal errors that map one-to-one onto EngineError kinds" (L2-3) | error.rs | REAL | test pins all three mappings |
| "free-format bitrate stream (… honestly refused rather than guessed)" (L8-10) | error.rs | REAL-UNPROVEN | the refusal exists in header.rs's parse (mp3.free_format codes); not executed in this audit's runs |

## 7. Dead / duplicated / unreachable code
None. The `From` impl and both constructors are used throughout the mp3 module (header.rs, xing.rs, decoder.rs).

## 8. Dependencies & flags
media-types (EngineError/ErrorKind/CorruptDetail/ErrorContext/FormatInfo/PipelineStage). pub(crate) only — correct scoping. No feature gates.

## 9. Open questions for the next agent
- Does `Mp2Error` (mp2/error.rs) duplicate this file's structure wholesale (the "pattern" admits it)? If the two are near-identical, a family-shared taxonomy is the obvious dedup — cost/benefit call for the owner.
- xing.rs F-001 (parse_vbri panic) should be fixed before any consumer adopts `decode_stream`; this module's contract is the reason the panic is a violation, not a curiosity.
