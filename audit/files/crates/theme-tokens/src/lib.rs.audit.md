# AUDIT: crates/theme-tokens/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W11-A10 |
| Date | 2026-11-23 |
| Lines / functions | 223 / 9 (+4 tests) |
| File verdict | REAL (build-time tooling crate; UNREACHABLE from shipped entry points) |
| Reachable from shipped entry point? | NO. theme-tokens has zero dependents (the `theme-tokens` mentions in crates/commands/Cargo.toml:29 and crates/settings-schema/Cargo.toml:19 are comments). Runs via `cargo xtask tokens` (tools/xtask/src/cmd/tokens.rs:48-71 shells out to `cargo run -p theme-tokens --bin tokens-gen`) and via crate tests in `cargo test --workspace` (baseline: exit 0). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | cat Cargo.toml (serde_json-only dep confirmed); rg TokenError variant usage with -F counts (parse 27, cycle 1, unknown_reference 2, contrast 1, emit 5, io 2, internal 11); rg theme_tokens::generated consumers (none); rg xtask tokens wiring; baseline cargo-test-workspace.txt:6330 (theme-tokens exit 0) |

## 1. One-paragraph verdict
The crate root does exactly what its header says and every doc claim I could
check reconciles: the parse pipeline and three-layer grammar were verified
real by Wave 1 (parse.rs audit), the committed outputs are diff-tested
(tests/diff.rs — baseline pass), the xtask subprocess bridge exists as
described (tokens.rs:48-71), and the dependency claim "serde_json only" is
literally true (Cargo.toml [dependencies] has one entry). TokenError is a
clean hand-rolled error type — all seven variants are constructed by real
code, Display strings are exact-asserted by tests, source() chains only Io.
The one dishonest note is small: the generated Rust constants module is
claimed to exist "for engine-side needs (doc 01 §2.3)" and no engine code
anywhere consumes it.

## 2. What actually works (verified)
— All 7 TokenError constructors have real call sites (rg -F counts above);
no dead variants.
— xtask bridge verified: tools/xtask/src/cmd/tokens.rs:61-66 spawns
`run -p theme-tokens --bin tokens-gen` — the documented "subprocess bridge".
— Dependency claim: `rg` Cargo.toml → `[dependencies] serde_json = "1.0.154"`
only.
— baseline cargo-test-workspace.txt:6316-6330: theme-tokens unit + diff +
semantic_lint tests all ran and passed, exit 0.

## 3. Findings
F-001 [S4]
Where: crates/theme-tokens/src/lib.rs:16-17, 39-42
Code:
```
/// * `crates/theme-tokens/src/generated/tokens.rs` — Rust constants for
/// engine-side needs (doc 01 §2.3), included below as [`generated`].
```
```
#[path = "generated/tokens.rs"]
pub mod generated;
```
What is wrong: `theme_tokens::generated` has zero consumers — `rg
theme_tokens::generated|generated::` outside theme-tokens/src returns
nothing, and inside the crate only tests/diff.rs compares the FILE (not the
module) and tokens_gen.rs writes it. The 247-line constants module is
compiled into the crate for no reader. Its real functions are (a)
compile-checking the committed Rust output and (b) being the diff-test
target — legitimate, but "for engine-side needs" is aspirational: engine
code consumes zero semantic tokens through Rust constants today (the UI
consumes the CSS side). What correct looks like: state the actual purpose
(compile-check + diff anchor) or delete the claim.
Confidence: HIGH — Verification: rg over crates/, apps/, tools/.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| TokenError::parse | 101-106 | REAL | — | 27 call sites |
| TokenError::cycle | 109-113 | REAL | — | 1 call site |
| TokenError::unknown_reference | 116-121 | REAL | — | 2 call sites |
| TokenError::contrast | 124-128 | REAL | — | 1 call site |
| TokenError::emit | 131-135 | REAL | — | 5 call sites |
| TokenError::io | 138-143 | REAL | — | 2 call sites |
| TokenError::internal | 146-148 | REAL | — | 11 call sites (invariant guards, per Wave 1 parse.rs audit) |
| Display for TokenError | 151-174 | REAL | — | exact-asserted |
| Error::source | 176-183 | REAL | — | Io-only chaining, correct |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| parse_error_names_the_offending_path | YES | exact full-string assert |
| parse_error_without_path_addresses_the_whole_file | YES | exact string |
| unknown_reference_renders_from_and_target | YES | exact string |
| io_error_carries_context_and_source | YES | string + source().is_some() |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| parse() validates three-layer grammar, produces TokenSet | lib.rs:3-8 | REAL | Wave 1 parse.rs audit (REAL, complete recursive-descent validator) |
| emitters render deterministically; outputs COMMITTED and diff-tested | lib.rs:10-20 | REAL | tests/diff.rs byte-equality, baseline pass; Wave 1 tokens.json audit (sha256 pins match) |
| "regeneration ... is idempotent" | lib.rs:20 | REAL | emission is a pure function of tokens.json (deterministic sort/format); diff.rs proves committed == fresh |
| tokens-gen binary is the xtask entry point (subprocess bridge) | lib.rs:21-24 | REAL | tools/xtask/src/cmd/tokens.rs:48-71 |
| "serde_json only" dependency policy | lib.rs:30-31 | REAL | Cargo.toml [dependencies] |
| Rust constants "for engine-side needs (doc 01 §2.3)" | lib.rs:16-17 | SHALLOW | zero engine consumers (F-001) |
| lint_source checks primitive/alias refs + raw literals | lib.rs:26-28 | REAL | my lint.rs audit (with documented hex-only scope) |

## 7. Dead / duplicated / unreachable code
`pub mod generated` — compiled, unconsumed (F-001). Crate itself is
production-unreachable (tooling only).

## 8. Dependencies & flags
serde_json 1.0.154 only. No feature gates. [[bin]] tokens-gen declared.

## 9. Open questions for the next agent
- If the engine ever needs token constants (doc 01 §2.3), the consumer will
  have to appear in a shipped crate — until then the Rust-emission half of
  the pipeline is write-only. (emit.rs owner.)
