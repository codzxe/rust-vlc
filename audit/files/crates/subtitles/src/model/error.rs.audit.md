# AUDIT: crates/subtitles/src/model/error.rs
| Field | Value |
|---|---|
| Auditor | W09-A08 |
| Date | 2026-11-23 |
| Lines / functions | 197 / 4 impl blocks (2 enums, 2 Display, 2 Error) |
| File verdict | REAL (data-only error taxonomy; every variant is constructed and rendered); crate UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO (crate-level, see segment.rs audit). In-crate: `ModelError` <- `model/time.rs:50`, `model/position.rs:43,270`, `model/style.rs:66`; `WireError` <- `model/wire.rs` at 25 sites; `tests/roundtrip_matrix.rs` + `tests/fuzz_survival.rs:183-200` assert on these variants. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Grep variant-construction sweep (all 4 ModelError + all 13 WireError variants verified constructed — output above); `sed -n` quote verification; baseline cargo-test-workspace.txt:5828-5833 (fuzz_survival asserts on EndBeforeStart/PercentOutOfRange/EmptyViewport). |

## 1. One-paragraph verdict
A clean, honest error-taxonomy file: two `#[non_exhaustive]` enums, no logic, no panics, every variant carries diagnostic fields (1-based line numbers, offending values), and — verified by exhaustive Grep — every one of the 17 variants is actually constructed by the model/wire layer and asserted on by tests. Nothing here can crash; the only judgment calls are cosmetic.

## 2. What actually works (verified)
— Exhaustive construction sweep: ModelError::{EndBeforeStart (time.rs:50), PercentOutOfRange (position.rs:43), EmptyViewport (position.rs:270), BadColor (style.rs:66)}; WireError::{MissingMagic (wire.rs:197), UnknownTag (285,357), MissingField (213,228,259,277,339,351,478,489), ExtraFields (369), BadInt (238,405,428,499), BadValue (265,411,436,446), BadAlignment (379), BadColor (458), EndBeforeStart (373), PercentOutOfRange (430), MissingPayload (220,291), BadEscape (176), UnterminatedStyled (311)} — zero dead variants.
— Line-number claim cross-checked: `parse_document` (wire.rs:192-197) enumerates `src.split('\n')` from index 0 with `MissingMagic` on the first line — the "1-based, magic line is line 1" doc matches the parser's `enumerate()` + offset usage downstream.
— Baseline: tests/fuzz_survival.rs asserts these error types by name (cargo-test-workspace.txt:5996-6000, `random_buffers_never_panic` etc. ok).

## 3. Findings
None. Checked, with method: (a) variant deadness — exhaustive Grep above, none dead; (b) Display correctness — each of the 17 arms rendered, field interpolation matches variant payloads (read); (c) doc-claim consistency — the two taxonomy claims (§ header) verified as above; (d) panic surface — pure `write!`/data, no indexing/unwrapping (read); (e) exhaustive match — both `match self` arms cover all variants, compiler-checked.

## 4. Per-function table (file < 200 LOC, included for completeness)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ModelError enum | 16-35 | REAL | - | 4 variants, all constructed |
| ModelError::Display/Error | 37-53 | REAL | - | renders µs values, states zero-duration legality |
| WireError enum | 59-142 | REAL | - | 13 variants, all constructed; line_no on every contextual one |
| WireError::Display/Error | 144-197 | REAL | - | `char::from(*tag)` renders bytes 0x80+ as U+0080.. — cosmetic only |

## 5. Test honesty
No tests in this file. Its variants are asserted by name in position.rs tests (percent bounds, empty viewport — ran green earlier), time.rs tests (baseline `end_before_start_is_typed_invalid` ok), and tests/fuzz_survival.rs (baseline ok) — the taxonomy is load-bearing in real assertions.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "failure is data, not a panic" | error.rs:1 | REAL | no panic path; fuzz_survival asserts typed errors (baseline) |
| "both `#[non_exhaustive]`" | error.rs:3-4 | REAL | lines 15, 58 |
| "Line numbers are 1-based ... (the magic line is line 1)" | error.rs:55-56 | REAL | wire.rs:192-197 split/enumerate + MissingMagic on first line |

## 7. Dead / duplicated / unreachable code
None (verified by sweep in §2).

## 8. Dependencies & flags
std-only (`std::fmt`, `std::error::Error`). The `// lumen-allow-literal:` annotation at line 149 is a deliberate, commented exemption for the wire-magic diagnostic string (the repo's lint-literals gate) — legitimate, not an lint-suppression smell.

## 9. Open questions for the next agent
None.
