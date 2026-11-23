# AUDIT: crates/codecs/src/audio/aac/error.rs
| Field | Value |
|---|---|
| Auditor | W07-A05 |
| Date | 2026-11-23 |
| Lines / functions | 187 / 5 prod + 2 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `unsupported`/`corrupt`/`into_engine_error` are the error constructors used by every refusal/damage path in aac/decoder.rs and aac/adts.rs, which are reached from apps/cli pipeline_host.rs's `"aac" \| "aac-lc"` arm. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `cargo test -p codecs --lib aac::error` → 2 passed (exit 0). |

## 1. One-paragraph verdict
A small, correct, total error taxonomy: three variants (`Unsupported`/`Corrupt`/`BadState`) mapping one-to-one onto `ErrorKind::Unsupported`/`Corrupt`/`Decoder{BadState}` at stage `Decode`, exactly as the doc table promises; both tests are real (they pin the exact rendered public strings, the stage, the source chain, and the kind match, including a negative `!matches!(Corrupt)` check). No unwrap, no panic, constructors total — the doc's "nothing panics" claim holds by read. Three hygiene findings: the honesty doc repeats the decoder.rs blind spot (it claims SBR/PS is "never silently downgraded" for object type 5/29 — true — but implicit-signaled SBR IS silently downgraded, see decoder.rs F-001); a genuinely corrupt ASC (e.g. reserved channel configuration, which the engine rejects as decode_error) surfaces as kind `Unsupported` (`aac.engine.init`) instead of the taxonomy's own `Corrupt` class; and the `Detail` chain-root struct is a verbatim copy of the same ~10-line struct duplicated across the codec modules.

## 2. What actually works (verified)
— `cargo test -p codecs --lib aac::error` → 2/2 passed: `taxonomy_maps_each_variant_to_its_public_kind` (pins `"[decode] unsupported in aac (aac.he_sbr)"`, `"[decode] corrupt structure at byte 1 (aac.adts.sync)"`, `"[decode] decoder: invalid state transition (aac.not_configured)"` + source texts), `bad_state_is_a_decoder_kind_not_corrupt`.
— All 14 distinct `aac.*` context codes emitted by decoder.rs/adts.rs route through these two constructors (rg-verified: every site uses `unsupported(...)`/`corrupt(...)` or `AacError::BadState`).

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/audio/aac/error.rs:10-12
Code:
```rust
//!   SSR, LTP, reserved — ADTS profile field or the ASC's audio
//!   object type); **HE-AAC (SBR/PS), honestly absent** — an ASC that
//!   signals object type 5/29 is refused typedly, never silently
//!   downgraded (doc 04 §3.2's honesty note); channel configuration 0
```
What is wrong: the honesty claim is scoped to "object type 5/29" and is true for exactly that scope, but as established in decoder.rs F-001 (this audit), an AOT-2 ASC with implicit SBR sync extension (0x2B7) passes every gate and IS silently downgraded to half-rate LC. The taxonomy doc's "never silently downgraded" framing, read on its own, overstates the guarantee.
Confidence: HIGH — Verification: read + the decoder.rs/engine-source analysis (see decoder.rs F-001).
F-002 [S4]
Where: crates/codecs/src/audio/aac/error.rs:14-17 (doc) vs decoder.rs:463-471 (the user of this taxonomy)
Code:
```rust
//! - **Packet/stream damage** (`Corrupt`): a bad ADTS syncword, a
//!   reserved/non-zero layer field, a reserved sampling-frequency
//!   index, a frame shorter than its own header claims, a frame whose
```
What is wrong: a genuinely malformed ASC (reserved channel configuration 8-15, or an index the engine's parser rejects as decode_error) is classified `Unsupported` (`aac.engine.init`, message "engine refused the configuration: aac: aac too complex") — the taxonomy's own doc places stream damage in `Corrupt`, and a corrupt-ASC refusal lands in the wrong kind. Misclassification of the KIND only (code + message are honest); affects consumers matching on `ErrorKind`.
Confidence: HIGH — Verification: read of decoder.rs:463-471 + symphonia-common's `decode_error` returns (registry source).
F-003 [S4]
Where: crates/codecs/src/audio/aac/error.rs:104-115
Code:
```rust
/// Chain-root carrier for the human-readable reason (the 2.B4
/// `FeatureDetail` pattern verbatim).
#[derive(Debug, Clone)]
struct Detail(String);
```
What is wrong: the doc admits verbatim duplication — the same private `Detail(String)` + Display + Error impl is copy-pasted per codec module (aac, flac, mp3, mp2, ...). Documented pattern, still N copies of identical code that a shared `media-types` helper would collapse.
Confidence: HIGH — Verification: read (cross-module duplication asserted by the doc itself; not re-counted across the crate — UNVERIFIED count).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `AacError` (enum) | 38-58 | REAL | — | 3 variants, documented mapping matches code |
| `into_engine_error` | 63-96 | REAL | S4 | correct kinds/stage; hard-codes `FormatInfo{name: Some("aac")}` even on the `aac-lc` ADTS lane (cosmetic); F-002 classification gap originates in callers |
| `From<AacError>` | 98-102 | REAL | — | delegates |
| `Detail` + impls | 106-115 | REAL | S4 | duplicated pattern (F-003) |
| `unsupported` | 118-123 | REAL | — | total |
| `corrupt` | 126-136 | REAL | — | total |
| tests | 142-186 | REAL | — | both pin real public-rendered strings |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| taxonomy_maps_each_variant_to_its_public_kind | YES | exact Display strings + stage + source-chain text for all 3 variants — any mapping drift fails |
| bad_state_is_a_decoder_kind_not_corrupt | YES | positive and negative kind match |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| variant→kind→stage mapping table | :32-36 | REAL | code :66-93 + both tests executed |
| "HE-AAC (SBR/PS) ... object type 5/29 ... never silently downgraded" | :10-12 | SHALLOW | true for 5/29 (decoder.rs test executed); false for implicit-SBR ASC (decoder.rs F-001) |
| "a damaged packet never poisons the next — the walker resyncs" | :18-20 | REAL-UNPROVEN | no test pins post-corruption recovery (see decoder.rs §5) |
| "Nothing panics; every constructor is total; no unwrap" | :22-23 | REAL | read-verified |

## 7. Dead / duplicated / unreachable code
No dead code. Duplication: `Detail` (F-003).

## 8. Dependencies & flags
media-types only. No unsafe, no features, pub(crate) surface.

## 9. Open questions for the next agent
- Should `aac.engine.init` split into `Unsupported` (feature) vs `Corrupt` (malformed ASC)? Requires distinguishing symphonia's unsupported_error vs decode_error at decoder.rs:465 — the engine's error type does carry that distinction.
