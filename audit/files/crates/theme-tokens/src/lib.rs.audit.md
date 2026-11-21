# AUDIT: crates/theme-tokens/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 223 / 14 (7 TokenError constructors, Display::fmt, Error::source, 5 module decls + generated include, 4 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (build-graph): workspace release build compiles it (baseline row 1); dependents crates/commands + crates/settings-schema (rg Cargo.toml) link it into the shipped `lumen` CLI graph. Runtime reachability of the PIPELINE (parse/emit) is via `cargo xtask tokens` / `tokens-gen` (dev/CI lane; ui `tokens:check` green in baseline row 24). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg `theme-tokens` in all Cargo.toml (dependents: commands, settings-schema); read crates/theme-tokens/Cargo.toml (serde_json 1.0.154 ONLY — doc-11 policy honored); read ui/scripts/gen-tokens.mjs:1-30 (single-generator pipeline confirmed: Rust emits themes/generated/tokens.css, node script copies verbatim → no dual-generator drift); ls crates/theme-tokens/tests/ (diff.rs, pipeline.rs, semantic_lint.rs) |

## 1. One-paragraph verdict
A small, disciplined crate root: module wiring, one error enum with fully documented variants, Display rendering one diagnostic line per variant, and an Error impl that chains only the Io source. The dependency policy claim (serde_json only) is exactly true, the generated-constants include is marked do-not-edit, and the pipeline this file fronts is diff-tested (tests/diff.rs) and green in the baseline (ui tokens:check row 24 + workspace build row 1). The only defect is test coverage of the error surface itself: 4 of 8 Display arms (Cycle, Contrast, Emit, Internal) have no test, so the documented "single diagnostic line" contract is unverified for half the failure modes.

## 2. What actually works (verified)
— verified by rg Cargo.toml sweep → exactly two in-tree dependents (crates/commands, crates/settings-schema) plus the crate's own bin; external deps = serde_json only, matching the doc-11 claim at l.30-31.
— verified by baseline row 1 (`cargo build --release --locked --workspace` exit 0) → this file compiles in the shipped graph; row 24 (`npm run tokens:check` PASS) → the committed generated outputs are consistent with the source at HEAD.
— verified by reading ui/scripts/gen-tokens.mjs:5-17 → the UI consumes the crate's committed output verbatim; there is exactly ONE generator for tokens (the Rust binary), killing the dual-generator drift hypothesis I carried into this audit.
— verified by reading tests/diff.rs presence + Cargo.toml `[[bin]] tokens-gen` → the committed-output diff test and the xtask bridge exist as documented (their internals belong to other agents' files).

## 3. Findings
### F-001 [S4]
Where: crates/theme-tokens/src/lib.rs:151-174 vs tests at 190-222
Code:
```
            TokenError::Cycle { chain } => write!(f, "reference cycle: {chain}"),
            TokenError::UnknownReference { from, target } => {
                write!(f, "`{from}` references unknown token `{target}`")
            }
            TokenError::Contrast { message } => write!(f, "accent contrast: {message}"),
            TokenError::Emit { message } => write!(f, "emission: {message}"),
```
What is wrong: The unit tests cover 4 of the 8 Display arms (Parse ×2, UnknownReference, Io). Cycle, Contrast, Emit, and Internal have zero direct tests at this site — their format strings could drift (or panic on a bad format arg) without any test noticing here. (The integration suites in tests/ may hit some arms via real failure fixtures — I did not trace which; UNVERIFIED.)
Why it matters: The crate's contract (l.50-51: "All variants carry actionable context … `Display` renders a single diagnostic line") is the user-facing surface of every pipeline failure; an untested arm is an unverified contract half.
What correct looks like: four more 3-line tests mirroring the existing pattern (construct, to_string, assert).
Confidence: HIGH on what is/isn't tested in this file; UNVERIFIED whether tests/pipeline.rs exercises those arms incidentally.
Note: TokenError is also not `#[non_exhaustive]` — a new variant later breaks exhaustive downstream matches; harmless inside the workspace, worth flagging only as a note.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module decls + `#[path] generated` include | 33-45 | REAL | — | do-not-edit provenance comment; generated/tokens.rs audited by W15-A06 |
| `TokenError` enum | 52-97 | REAL | — | 8 variants, all documented; Contrast variant encodes the ≥4.5:1 on-accent rule (doc 08 §3.3) |
| `TokenError::parse` | 100-106 | REAL | — | tested (190-197) |
| `cycle` | 108-112 | REAL | — | constructor fine; Display arm untested (F-001) |
| `unknown_reference` | 114-120 | REAL | — | tested (205-212) |
| `contrast` | 122-128 | REAL | — | Display arm untested |
| `emit` | 130-135 | REAL | — | Display arm untested |
| `io` | 137-143 | REAL | — | tested incl. `source().is_some()` (214-222) — the only variant with a chained source |
| `internal` | 145-148 | REAL | — | Display arm untested; "please report it" guidance good |
| `Display::fmt` | 151-174 | REAL | S4 | F-001 — 4/8 arms covered |
| `Error::source` | 176-183 | REAL | — | correct: chains Io only, None elsewhere |
| tests ×4 | 185-223 | REAL | — | exact-string assertions; would fail on format drift in the covered arms |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| parse_error_names_the_offending_path (190) | YES | exact to_string equality incl. backtick placement |
| parse_error_without_path (199) | YES | the empty-path branch asserted |
| unknown_reference_renders (205) | YES | exact string |
| io_error_carries_context_and_source (214) | YES | string + source-chain assertion |
Mutation check (reasoned): swap the Cycle format string → no test in THIS file fails (F-001); tests/pipeline.rs may or may not catch it via integration paths.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Dependency policy (doc 11): `serde_json` only (pre-audit PASS)" | l.30-31 | REAL | Cargo.toml [dependencies] = serde_json 1.0.154 only |
| "Both outputs are COMMITTED and diff-tested (`tests/diff.rs`)" | l.19-20 | REAL | tests/diff.rs exists; ui tokens:check green (baseline row 24) |
| "regeneration through `cargo xtask tokens` is idempotent" | l.20-21 | REAL-UNPROVEN | no baseline run of `cargo xtask tokens` twice exists (row set has no such log); the diff test implies single-run consistency only |
| "the `tokens-gen` binary … the xtask `tokens` subcommand shells out to" | l.21-24 | REAL | Cargo.toml [[bin]] tokens-gen; the frozen-xtask bridge rationale recorded |
| "All variants carry actionable context … Display renders a single diagnostic line" | l.49-51 | REAL for 4/8 arms / UNVERIFIED for the rest | F-001 |

## 7. Dead / duplicated / unreachable code
None in this file. Every constructor is called by parse.rs/emit.rs/lint.rs/color.rs (the module tree this file fronts — spot-checked by rg `TokenError::contrast|cycle|emit` across the crate: all used).

## 8. Dependencies & flags
serde_json 1.0.154 (only). `[lints] workspace = true` — inherits the 39/39 unsafe-lint policy (baseline row 11). No feature flags. No unsafe.

## 9. Open questions for the next agent
- tests/pipeline.rs / tests/diff.rs (not in any W15 list I can see): do they drive the Cycle/Contrast/Emit/Internal arms? If yes, F-001 downgrades to a note; if no, the error surface is half-untested crate-wide.
- parse.rs (W15-A02) / color.rs (W15-A04): the Contrast rule's threshold (≥4.5:1) is enforced where? Verify the checker actually recomputes WCAG, not a hardcoded pass.

---

## ERRATUM (W15-A01, session 2, 2026-11-21)
The reachability row above claims "dependents crates/commands + crates/settings-schema (rg
Cargo.toml) link it into the shipped `lumen` CLI graph". That is WRONG. Re-verified by reading
the hit lines: both mentions are COMMENTS (crates/commands/Cargo.toml:29 "…the same versions
settings-schema/theme-tokens use"; crates/settings-schema/Cargo.toml:19 "first recorded by card
1.B4 (theme-tokens)…") — neither crate lists theme-tokens in [dependencies]. theme-tokens has
ZERO in-tree dependents; this crate is dev/codegen tooling (xtask tokens / cargo test gates)
only, exactly as W15-A02's parse.rs audit states. The file verdict REAL is unchanged (the
pipeline runs green in baseline rows 3/24); the "shipped `lumen` CLI graph" clause is retracted.
Cause: session-1 `rg -l theme-tokens -g Cargo.toml` was trusted without reading the matched
lines. Chunk audits (tokens.json#part1/#part2) state the corrected reachability.
