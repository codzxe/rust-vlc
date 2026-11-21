# AUDIT: crates/subtitles/src/model/error.rs
| Field | Value |
|---|---|
| Auditor | W09-A02 |
| Date | 2026-11-21 |
| Lines / functions | 197 / 6 items (2 enums, 2 Display impls, 2 Error impls) — no functions, no tests in-file |
| File verdict | UNREACHABLE (real, minimal, correct data module; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — `ModelError` constructed only by `model/{cue,position,style,time}.rs`; `WireError` only by `model/wire.rs` (all in-crate; census below); crate-level proof in ass/tags.rs audit §2 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | variant census: `rg -c 'ModelError::<k>' model/*.rs` (4/4 live) and `rg -c 'WireError::<k>' model/wire.rs` (12/12 live; single-hit variants verified as `Err(...)` constructions at wire.rs:176,197,311,369,379,458) · `rg -n 'WireError|ModelError' ../tests/*.rs` (integration consumers) · `sed -n` quote verification |

## 1. One-paragraph verdict
A pure typed-error vocabulary: `ModelError` (4 variants, model-construction
failures) and `WireError` (12 variants, wire-form parse failures with 1-based
line numbers), both `#[non_exhaustive]`, both with compiler-forced
exhaustive `Display` matches, both `std::error::Error`. Every one of the 16
variants is genuinely constructed in the crate — no paper variants. The one
defect is a diagnostic nit: `UnknownTag` renders its raw tag byte through
`char::from(u8)`, so byte 0xFF reports as `'ÿ'`, implying a UTF-8 scalar
that was never in the byte-oriented wire stream. Unreachable from shipped
binaries like the rest of the crate.

## 2. What actually works (verified)
- Variant liveness: census commands in the header — ModelError:
  EndBeforeStart 4, PercentOutOfRange 5, EmptyViewport 5, BadColor 2 hits
  across model/{cue,position,style,time}.rs; WireError: all 12 variants
  constructed in model/wire.rs (single-hit ones re-verified as `Err(...)`
  sites: `WireError::BadEscape` wire.rs:176, `MissingMagic` :197,
  `UnterminatedStyled` :311, `ExtraFields` :369, `BadAlignment` :379,
  `BadColor` :458).
- Integration consumers exercise the taxonomy: `tests/timing_edges.rs:47`
  asserts `Some(WireError::EndBeforeStart { .. })`, `tests/fuzz_survival.rs:183-200`
  matches `ModelError::EndBeforeStart/PercentOutOfRange/EmptyViewport` —
  the types are load-bearing in the crate's own ACs, not decorative.
- Both `Display` matches are exhaustive without a wildcard arm — adding a
  variant is a compile error until the message is written. Verified by
  reading both matches (37-51, 144-195).
- No panics possible, no allocation beyond Display formatting, no unsafe,
  std-only.

## 3. Findings

### F-001 [S4] UnknownTag diagnostic prints a Latin-1 char for a raw tag byte
Where: crates/subtitles/src/model/error.rs:151-157
Code:
```
            Self::UnknownTag { line_no, tag } => {
                write!(
                    f,
                    "line {line_no}: unknown record tag {:?}",
                    char::from(*tag)
                )
            }
```
What is wrong: `char::from(u8)` maps bytes 0x80..=0xFF onto U+0080..=U+00FF
(Latin-1). The wire form is byte-tagged; a hostile/corrupt record with
leading byte 0xFF renders the diagnostic "unknown record tag 'ÿ'" —
asserting a printable character that does not exist in the byte stream (the
document contained byte 0xFF, not the UTF-8 sequence for ÿ).
Why it matters: a diagnostics module's one job is truth about the input;
this message misrepresents the offending byte. Blast radius: log text only.
What correct looks like: print the byte numerically —
`"unknown record tag byte 0x{tag:02X}"`.
Confidence: HIGH — Verification: read; `char::From<u8>` Latin-1 semantics
are standard.

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ModelError` (enum) | 14-35 | REAL | — | 4 variants, all live; field docs carry units (µs) and the legal/illegal boundary (zero-duration legal, reversal not). |
| `Display for ModelError` | 37-51 | REAL | — | Exhaustive match; messages include the values and the rule. |
| `Error for ModelError` | 53 | REAL | — | Leaf error, no source — correct. |
| `WireError` (enum) | 57-142 | REAL | — | 12 variants, all live; every parse-failure mode carries 1-based `line_no` except `MissingMagic` (document start — justified). |
| `Display for WireError` | 144-195 | REAL | S4 (F-001) | Exhaustive; one diagnostic-fidelity nit on `UnknownTag`. |
| `Error for WireError` | 197 | REAL | — | Leaf error. |

## 5. Test honesty
No tests in this file. The taxonomy is asserted by integration tests
(`timing_edges.rs`, `fuzz_survival.rs`, `roundtrip_matrix.rs` — consumers
listed in §2), which match variants structurally. The `Display` strings
themselves are asserted nowhere (no golden-string test) — acceptable for
diagnostics, noted for completeness.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Typed errors for the cue model (MP §8: failure is data, not a panic)" | error.rs:1 | REAL (this file) / crate-level | The model constructors return these errors instead of panicking (census sites); crate fuzz lanes pass. |
| "WireError — parsing the stable wire form failed, with the 1-based offending line number" | error.rs:8-9 | REAL | 11 of 12 variants carry `line_no`; `MissingMagic` needs none (document start). |
| "both #[non_exhaustive]" | error.rs:3-4 | REAL | Both enums annotated (lines 15, 58). |

## 7. Dead / duplicated / unreachable code
- Unreachable from shipped entry points (crate proof, ass/tags.rs audit §2).
- No dead variants (16/16 constructed — census). `ModelError::BadColor` and
  `WireError::BadColor` share a name across taxa — intentional and
  documented (line 6-9), not rot.

## 8. Dependencies & flags
- std-only (`std::fmt`, `std::error`), no features, no unsafe, no panics.
  Derives (`Debug, Clone, PartialEq, Eq`) suit a comparability-in-tests
  error vocabulary.

## 9. Open questions for the next agent
- model/wire.rs owner: the wire grammar (LUMEN-CUES 1) this taxonomy
  serves — is `UnterminatedStyled` recoverable (skip cue) or fatal (abort
  document)? The variant docs don't say; the disposition lives in wire.rs.
- If Display strings ever become user-visible (CLI diagnostics), add a
  golden test; today they are log-only.
