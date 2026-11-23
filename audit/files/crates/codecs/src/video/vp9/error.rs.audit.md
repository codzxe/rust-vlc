# AUDIT: crates/codecs/src/video/vp9/error.rs
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-23 |
| Lines / functions | 187 / 6 (Vp9Error::into_engine_error, From impl, Detail, unsupported, corrupt, 1 test) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — Vp9Decoder is registered: apps/cli/src/pipeline_host.rs:313 `"vp9" =>` (rusty_vp9 =0.1.3 admission); this taxonomy is the decoder's error surface |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg wiring (pipeline_host:66,313); baseline test results cited |

## 1. One-paragraph verdict
A clean typed-error taxonomy: three variants (Unsupported/Corrupt/BadState) mapping one-to-one onto EngineError kinds at stage Decode, with stable dotted codes, structured CorruptDetail, and a Display-only chain-root for the human reason. The single test pins the exact rendered strings, stage, and source chain for all three variants — a real behavioral pin (a rendering regression fails it). No panics, no unwraps; the doc's totality claim matches the code. One hygiene note: the Detail carrier is the Nth verbatim copy of the 2.B4 FeatureDetail pattern (self-acknowledged).

## 2. What actually works (verified)
— verified baseline audit/runs/baseline/cargo-test-workspace.txt:877 -> `video::vp9::error::tests::taxonomy_maps_each_variant_to_its_public_kind ... ok` (plus decoder tests L873-876 that exercise this taxonomy's constructors).
— verified wiring: pipeline_host.rs:66 `use codecs::video::vp9::Vp9Decoder;` and :313 the registry lane.

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/video/vp9/error.rs:113-116
Code: (verbatim)
```
/// Chain-root carrier for the human-readable reason (the 2.B4
/// `FeatureDetail` pattern verbatim).
#[derive(Debug, Clone)]
struct Detail(String);
```
What is wrong: the private Display-carrier struct is duplicated per codec module (the comment admits "verbatim"); a shared media-types/testkit carrier would remove the copy-paste pattern. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Vp9Error (enum) | 42-68 | REAL | - | documented variant→kind table holds in code |
| into_engine_error | 72-104 | REAL | - | stage/kind/code/source mapping; pinned by test |
| From<Vp9Error> | 107-111 | REAL | - | |
| Detail + impls | 115-124 | REAL | S4 | duplicated pattern (F-001) |
| unsupported / corrupt | 127-145 | REAL | - | total constructors |
| tests | 151-186 | REAL | - | exact-string + stage + source-chain pins |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| taxonomy_maps_each_variant_to_its_public_kind | YES | exact rendered Display strings and source chain asserted for all 3 variants |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Nothing panics; every constructor is total; no `unwrap` on any input path" | L27-28 | REAL | no panic/unwrap in the file (read) |
| variant→kind table (L37-41) | doc table | REAL | into_engine_error implements exactly it; test pins |

## 7. Dead / duplicated / unreachable code
Detail duplication (F-001). Nothing dead.

## 8. Dependencies & flags
media_types only. pub(crate) — internal vocabulary, correctly not public API. No feature gates.

## 9. Open questions for the next agent
- vp9/decoder.rs (the consumer, rusty_vp9 wrapper) is not in my assignment — the "engine rejected / plane geometry does not close" Corrupt sites live there.
