# AUDIT: crates/codecs/src/audio/alac/error.rs
| Field | Value |
|---|---|
| Auditor | W07-A06 |
| Date | 2026-11-23 |
| Lines / functions | 213 / 7 (incl. 3 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — every ALAC decode error path flows through `into_engine_error` (decoder.rs:184, 515, 617, 631, 651, 676, 685, 705); ALAC itself reachable via pipeline_host.rs:293-296 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | prebuilt `./target/debug/deps/codecs-a0d7a93b7072ff58 alac::error` -> 3 passed; rg conversion-usage sweep over crates/codecs/src/audio/alac/ |

## 1. One-paragraph verdict
A small, clean, real error taxonomy: three variants (Unsupported/Corrupt/BadState), total constructors, structured CorruptDetail carried into `ErrorKind::Corrupt`, human-readable reasons delegated to the error `source` chain, and a pinned rendering test that locks the public strings ("[decode] unsupported in alac (alac.config.missing)" etc.). The 3 tests executed and pass; the "no unwrap on any input path" claim is true by inspection (the only `unwrap_or_default` is in a test). Two defects: the sole production truncation site feeds the structured class a meaningless `expected: 0` (so every truncated-ALAC error renders "expected 0 bytes, found N bytes" — the "observable, not vague" promise is broken by the caller), and the `From<AlacError> for EngineError` impl has zero users (every call site names `into_engine_error()` explicitly).

## 2. What actually works (verified)
- `./target/debug/deps/codecs-a0d7a93b7072ff58 alac::error` -> `test result: ok. 3 passed` — variant→kind mapping incl. rendered Display strings and source-chain text; CorruptDetail::Truncated structure survives into `ErrorKind`; BadState is `ErrorKind::Decoder`, never Corrupt.
- Baseline corroborates: cargo-test-workspace.txt:449-451 (all three tests ok).
- Panic-freedom claim verified by reading: no unwrap/expect/index outside test code; constructors are total.

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/audio/alac/decoder.rs:343-349 (the production consumer of this taxonomy)
Code:
```
                if truncated {
                    media_types::error::CorruptDetail::Truncated {
                        expected: 0,
                        found: packet_len as u64,
                    }
```
What is wrong: this file's contract says the Corrupt variant is "Typed with the exact [`CorruptDetail`] so reporting is observable, not vague" (error.rs:46-47), and the taxonomy's own test pins the honest shape (`Truncated { expected: 16, found: 9 }` renders "truncated data: expected 16 bytes, found 9"). The only production truncation site passes `expected: 0`, so every real truncated-ALAC error renders "truncated data: expected 0 bytes, found 512 bytes (alac.frame.truncated)" — an "expected 0" that conveys nothing. The engine's end-of-bitstream error carries no byte position, but `frame_length`-derived expectations (or at least `expected: 1`) were available. Why it matters: the structured reporting this file exists to provide is fed garbage by its principal caller; downstream UIs/diagnostics surface nonsense. What correct looks like: an expected value from the cookie geometry or a distinct CorruptDetail variant for mid-bitstream exhaustion. Confidence: HIGH — Verification: read + rendered-shape pinned by the in-file test.
F-002 [S4]
Where: crates/codecs/src/audio/alac/error.rs:99-103
Code:
```
impl From<AlacError> for EngineError {
    fn from(value: AlacError) -> EngineError {
        value.into_engine_error()
    }
}
```
What is wrong: dead code. rg over the alac module shows every conversion is the explicit `.into_engine_error()` / `map_err(AlacError::into_engine_error)` form (decoder.rs:184, 515, 617, 631, 676, 685); no `?`-based From conversion exists, and no other crate can use it (`pub(crate)`). Harmless, but it implies a `?` ergonomics path that nothing exercises. What correct looks like: delete, or use `?` in the internal helpers. Confidence: HIGH — Verification: rg usage sweep.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| AlacError (enum) | 33-59 | REAL | — | Three variants, documented public-kind mapping |
| into_engine_error | 64-97 | REAL | — | Stage Decode; FormatInfo "alac" on Unsupported; structured detail on Corrupt; stable `alac.not_configured` on BadState |
| From<AlacError> | 99-103 | REAL (dead) | S4 | F-002 |
| Detail + Display + Error | 108-117 | REAL | — | Source-chain carrier; no secrets in Display surface |
| unsupported | 120-125 | REAL | — | Total constructor |
| corrupt | 128-138 | REAL | — | Total constructor; caller feeds it `expected: 0` (F-001) |
| tests ×3 | 144-213 | REAL | — | Rendering + structure + kind discrimination |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| taxonomy_maps_each_variant_to_its_public_kind | YES | Locks exact Display strings, stage, and source text for all three variants |
| corrupt_variant_carries_the_structured_class | YES | Pattern-matches `ErrorKind::Corrupt`/`Truncated{24,12}` — structure, not just strings |
| bad_state_is_a_decoder_kind_not_corrupt | YES | Discriminates Decoder-vs-Corrupt kind |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Nothing panics; every constructor is total; no `unwrap` on any input path" | error.rs:18-19 | REAL | Read-verified; no unwrap/expect/index outside tests |
| "Typed with the exact CorruptDetail so reporting is observable, not vague" | error.rs:46-47 | SHALLOW | True of the taxonomy; the production truncation caller passes `expected: 0` (F-001) |
| Variant→kind table (Unsupported/Corrupt/Decoder) | error.rs:28-32 | REAL | Pinned by the executed test |

## 7. Dead / duplicated / unreachable code
`From<AlacError> for EngineError` (F-002). Naming hygiene: `Corrupt { detail, detail_text }` — two near-identical field names for different types (structured class vs. string).

## 8. Dependencies & flags
Depends only on media_types (EngineError/ErrorKind/ErrorContext/CorruptDetail/FormatInfo/PipelineStage). Unconditional inside the `codec-alac`-gated alac module. No unsafe.

## 9. Open questions for the next agent
- media_types owner: is `CorruptDetail::Truncated { expected: 0 }` abused elsewhere too (mp3's `expected: 4` at mp3/decoder.rs:204 is at least nonzero)? A `BitstreamExhausted` variant would fix the class of F-001.
