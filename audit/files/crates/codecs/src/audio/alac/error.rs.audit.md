# AUDIT: crates/codecs/src/audio/alac/error.rs
| Field | Value |
|---|---|
| Auditor | W07-A10 |
| Date | 2026-11-21 |
| Lines / functions | 213 / 7 prod items (enum, into_engine_error, From, Detail+Display+Error, 2 ctors) + 3 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open *.m4a` → `containers/mp4.rs:235` alac child-box cookie → `pipeline_host.rs:293` seeded `AlacDecoder` → decoder.rs raises `AlacError` through these ctors (37 use sites in alac/decoder.rs) → `into_engine_error` at the contract boundary; MKV lane `containers/mkv.rs:251`. Damage paths exercised by `crates/codecs/tests/alac_fuzz_survival.rs`; happy path by `alac_bitexact.rs` + the mkv 0098 lane (88,200 samples) — all in baseline run #3.) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -c 'unsupported(\|corrupt(\|AlacError' crates/codecs/src/audio/alac/*.rs` (consumer map: decoder.rs 37 sites); `rg -n 'alac' apps/cli/src/containers/mp4.rs mkv.rs apps/cli/tests` (shipped lanes + e2e lanes); line-by-line read of the mapping code vs the doc table; baseline runs cited for execution. |

## 1. One-paragraph verdict
Clean, correct, fully-consumed error taxonomy: three variants map exactly as the doc table promises (Unsupported→`ErrorKind::Unsupported` with format "alac", Corrupt→`ErrorKind::Corrupt` carrying the structured `CorruptDetail`, BadState→`ErrorKind::Decoder`/`alac.not_configured`), the human-readable reason rides in `source` (never `Display`), every constructor is total, and nothing on any path can panic. Zero findings — the negative result is backed by the itemized checks in §2.

## 2. What actually works (verified) — exhaustive check list for the zero-finding claim
— verified by reading :34-96 against the doc table :26-32: all three variant→kind→stage mappings match one-to-one (stage always `PipelineStage::Decode`); `Corrupt`'s structured `CorruptDetail` passes through unmodified (asserted again by test 2).
— verified by reading every expression :61-138: no `unwrap`/`expect`/index/slice/arith on input; the only allocations are two `to_string()`s per error on cold damage paths; `ErrorContext::new(&'static str)` is total — the module's "nothing panics; every constructor is total" claim (:18-19) holds.
— verified `From<AlacError>` (:99-103) delegates to `into_engine_error` (no divergent duplicate).
— verified `Detail` (:108-117): plain Display/Error wrapper, no secret leakage surface beyond the caller-supplied string.
— verified callers: `unsupported`/`corrupt`/`AlacError::BadState` used at 37 sites in `alac/decoder.rs`; nothing else in the crate re-implements this pattern for ALAC (no duplication).
— verified baseline execution: run #3 (1681 passed) includes the alac suites (bitexact 5 fixtures, fuzz survival, mkv 0098 lane 88,200 samples decoded) — this file's conversions ran in both happy and damage shapes.
— verified workspace hygiene: clippy `-D warnings` clean (run #5), fmt clean (run #7 lists only `plugin-api/tests/testkit_wiring.rs` dirty).

## 3. Findings
None. (Checked for: dead variants, unused constructors, kind mismatches, stage mislabeling, panic paths, allocation in hot paths, taxonomy duplication across codecs — the AAC/FLAC siblings use the same pattern deliberately, per the :1-2 doc note, and are separate files.)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| AlacError (enum) | 33-59 | REAL | — | 3 variants, all consumed; doc table accurate. |
| AlacError::into_engine_error | 61-97 | REAL | — | Mapping verified line-by-line vs doc; total, no panics. |
| From<AlacError> | 99-103 | REAL | — | Delegates; no divergence. |
| Detail + Display + Error | 105-117 | REAL | — | Carrier only. |
| unsupported (ctor) | 119-125 | REAL | — | Total; 1 alloc. |
| corrupt (ctor) | 127-138 | REAL | — | Total; structured class preserved. |
| tests ×3 | 140-213 | REAL tests | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| taxonomy_maps_each_variant_to_its_public_kind | YES | Exact rendered `Display` strings + stage + `source` text asserted for all three variants — change the format name, code, or source routing and it fails. |
| corrupt_variant_carries_the_structured_class | YES | Destructures `ErrorKind::Corrupt{detail}` and asserts the Truncated pair — swallowing or re-typing the detail fails it. |
| bad_state_is_a_decoder_kind_not_corrupt | YES | Kind discrimination pinned (a Corrupt re-mapping of BadState fails it). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "typed, stage-tagged internal errors that map one-to-one onto EngineError kinds" | error.rs:1-3 | REAL | §2 mapping check. |
| "Nothing panics; every constructor is total; no unwrap on any input path" | error.rs:18-19 | REAL | §2 expression-by-expression read. |
| Per-packet isolation note (frames independent) | error.rs:12-16 | REAL-UNPROVEN | Contract claim about decoder.rs behavior; the isolation itself is decoder.rs's to prove (alac/decoder.rs auditor). |

## 7. Dead / duplicated / unreachable code
None in this file. (The pattern is intentionally duplicated across codec modules — flac/error.rs, aac's taxonomy, mp3/error.rs — per the :1-2 note; cross-module consolidation would be a workspace-level S4, not this file's defect.)

## 8. Dependencies & flags
- media-types error taxonomy only; `codec-alac` default ON. No unsafe, no allows, std-only. Zero external deps beyond media-types.

## 9. Open questions for the next agent
- alac/decoder.rs auditor: does the per-packet isolation claim (:12-16) hold in `decode` (no state carry-over between packets)? This file only documents it.
