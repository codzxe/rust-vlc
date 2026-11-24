# AUDIT: crates/product-config/tests/golden_s35.rs
| Field | Value |
|---|---|
| Auditor | W11-A08 |
| Date | 2026-11-23 |
| Lines / functions | 65 / 1 (#[test] probe_max_bytes_renders_the_doc_07_s35_example) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — golden test; runs in the workspace test lane. It exercises `product_config::load_str` + `Diagnostic::Display` (production code, also used by apps/control-server). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg/doc cross-check of doc 07 §3.5 example (docs/07-config-driven-system.md:440-457); diff of range-probe-max-bytes.toml vs product.toml (exit 1, stale base); baseline: `Running tests/golden_s35.rs ... 1 passed`. |

## 1. One-paragraph verdict
A byte-exact golden of the doc 07 §3.5 diagnostic contract: location line, `value:` with units expanded, `rule:` with both bounds in bytes+MiB, `expected:` range, and the doc's hint prose (whitespace-normalized because the doc's manual line wrap is not normative). It loads the fixture under the file label `product.toml` so the rendered location line reproduces the doc example verbatim. Honest and strict; the only nit is that the location line is the EXTENDED form (with `:line:`) which the doc's literal example does not carry — the test itself documents this as the 1.B1 contract extension.

## 2. What actually works (verified)
- Doc example (docs/07-config-driven-system.md §3.5, lines ~446-453) cross-checked against the assertions: `error[config]: product.toml:limits.probe_max_bytes`, `value: 1048576 (1.0 MiB)`, `rule:  must be >= 16777216 (16 MiB) and <= 67108864 (64 MiB)`, hint prose — all four asserted by the test (value/rule/expected byte-exact incl. trailing `\n`; hint fragments normalized).
- Baseline: `Running tests/golden_s35.rs (target/debug/deps/golden_s35-ae4ac8719dee1a22) / running 1 test / test result: ok. 1 passed`.

## 3. Findings
F-001 [S4]
Where: crates/product-config/tests/golden_s35.rs:34-37 vs docs/07-config-driven-system.md §3.5
Code:
```
    assert_eq!(
        first,
        format!("error[config]: product.toml:{line}:limits.probe_max_bytes")
    );
```
What is wrong: the doc's mandatory format is `error[config]: product.toml:limits.probe_max_bytes` (no line number); the golden asserts the extended `file:line:key` form. The test's own comment (31-32) declares the extension as the 1.B1 contract, and rule (1) of §3.5 says the key path is `file:key.path` — so doc and code disagree on the literal shape and the test encodes the code's side. Cosmetic doc/code divergence, deliberately acknowledged; the doc is the stale side.
Confidence: HIGH — Verification: read both files.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| probe_max_bytes_renders_the_doc_07_s35_example | 13-65 | REAL | S4 | Exact-one-diagnostic, byte-exact value/rule/expected lines, normalized hint fragments, line number derived from the fixture marker (not hard-coded). |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| probe_max_bytes_renders_the_doc_07_s35_example | YES | Any change to the diagnostic rendering (units expansion, range text, hint prose, location format) breaks a byte-exact or normalized assertion. A validator that accepted 1048576 fails at the rejection step. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "must render the doc 07 §3.5 example diagnostic" | golden_s35.rs:1-2 (self) | REAL (with F-001 extension caveat) | Doc §3.5 example matches all asserted fields. |
| Diagnostic format `error[config]: file:key.path` (no line) | docs/07 §3.5 mandatory format | SHALLOW | Code renders file:line:key; test asserts the extended form (F-001). |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Test-only; uses common::{read_fixture, marker_line, load_fixture_as, for_key} and product_config::render_all. Note the fixture (range-probe-max-bytes.toml) shares the stale pre-0.1.3 base of the other 39 fixtures — does not affect this golden.

## 9. Open questions for the next agent
- Should doc 07 §3.5's format spec be updated to include `:line:`? (The diagnostic.rs implementation already renders it everywhere.)
