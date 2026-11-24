# AUDIT: crates/theme-tokens/tests/semantic_lint.rs
| Field | Value |
|---|---|
| Auditor | W11-A09 |
| Date | 2026-11-23 |
| Lines / functions | 103 / 3 helpers + 2 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test file). The code under test (`theme_tokens::lint::lint_source`) is invoked by NOTHING except this test file and lint.rs's own unit tests — no xtask command, no CI step, no shipped binary calls it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg "lint_source" repo-wide (callers: lint.rs itself + this file only); ls + cat of both fixture CSS files; rg eslint config for the UI-side rule (ui/eslint.config.js:110 `local/semantic-tokens-only: error`); baseline citation (semantic_lint 2/2 pass, cargo-test-workspace.txt theme-tokens section) |

## 1. One-paragraph verdict
Two honest integration tests that parse the REAL committed `themes/tokens.json` (not a synthetic token set) and lint two fixture CSS files: `violating.css` deliberately breaks the semantic-only rule in all five detectable ways, and the test asserts each violation with exact `kind` + `token` + `snippet` (e.g. RawValue/color.gray.300/"#9A9A9A"), that every message cites "doc 08 §3.2", and that output is deterministically sorted; `clean.css` must produce zero violations. Mutation reasoning: delete any detector arm in lint.rs → the matching assertion fails; make the lint over-eager → clean fixture fails. The finding is about scope, not honesty: the checker these tests prove is wired to no pipeline — the header of lint.rs admits the real UI enforcement is the ESLint rule, but still claims lint_source is "available to CI", and no CI step exists that calls it.

## 2. What actually works (verified)
— verified by baseline cargo-test-workspace.txt (theme-tokens package): `test result: ok. 2 passed` for tests/semantic_lint.rs.
— verified by cat of both fixtures: violating.css contains all five violation shapes (raw `#9A9A9A`, `var(--color-gray-900)`, `var(--stroke-width-icon)`, `primitive.color.orange.500`, `alias.color.surface`); clean.css uses only semantic vars (`--color-surface`, `--color-accent`, …).
— verified by rg: `lint_source` has no caller outside crates/theme-tokens (src/lint.rs unit tests + this file).

## 3. Findings
F-001 [S3]
Where: crates/theme-tokens/src/lint.rs:4-7 (claim), consumer: this test file only
Code:
```
//! `ui/` enforcement is the ESLint/Stylelint rule set (doc 08 §3.4, Phase
//! 4); [`lint_source`] is the crate-side checker used by this crate's
//! tests and available to CI. Given any source text (CSS, TSX, Rust, …)
```
What is wrong: "available to CI" is false — no CI or gate step invokes `lint_source`: `cargo xtask tokens` only shells out to `tokens-gen` (regeneration; tools/xtask/src/cmd/tokens.rs:54-66 has no lint flag), full_gate's lint sections are lint-literals/unsafe-lints (different checks), and `tokens-gen` itself contains no lint call (rg: zero hits). The Rust-side semantic-only checker is therefore exercised exclusively by its own tests against two synthetic CSS stand-ins; the real enforcement burden rests entirely on the UI's `local/semantic-tokens-only` ESLint rule (ui/eslint.config.js:110). A Rust/TSX source in crates/ or apps/ that references `--color-gray-900` or a raw primitive hex would pass every gate: this checker never sees non-fixture input. What correct looks like: either wire `lint_source` into a gate step over crates/ + apps/ sources, or delete the claim and mark the checker test-only. Confidence: HIGH — Verification: read + rg (exhaustive caller scan).
Test-honesty note (no severity): the dotted-path violations are planted inside CSS comments (`/* reads primitive.color.orange.500 directly */`) — the lint deliberately scans comments; a future maintainer "fixing" the fixture by moving the paths into real property values would still pass, so the comment placement is load-bearing and undocumented in the test. Confidence: HIGH — Verification: read of fixtures.

## 4. Per-function table (helpers)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| workspace_root | 16-18 | REAL | - | MANIFEST_DIR/../.. |
| fixture | 20-25 | REAL | - | reads fixture CSS; `?` propagates IO errors |
| parse_committed_source | 27-31 | REAL | - | parses the REAL themes/tokens.json — real data, not synthetic |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| violating_fixture_fails_the_check (34) | YES | five exact (kind, token, snippet) triples + rule-citation check + sorted-order check; any detector regression or nondeterminism fails it |
| clean_fixture_passes_the_check (94) | YES | over-eager lint (e.g. flagging semantic vars) fails it |
Gap: no test lints real repo sources (ui/src, crates/) — only synthetic fixtures (see F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`violating.css` is a stand-in for component code ... every such reference must FAIL the check" | semantic_lint.rs:3-6 | REAL | test passes against committed tokens.json + fixture |
| "[`lint_source`] is ... used by this crate's tests and available to CI" | lint.rs:5-6 | FALSE in part | no CI step calls it (F-001) |
| "An alias CSS form that coincides with a semantic name ... is *not* flagged" | lint.rs:8-10 | REAL | covered by lint.rs unit tests (lint.rs:319-387) and clean.css's `var(--color-surface)` |

## 7. Dead / duplicated / unreachable code
- `lint_source` itself is CI-dead (F-001) — the test file is its only non-unit consumer.

## 8. Dependencies & flags
theme_tokens::{lint, parse}; std::fs. No features.

## 9. Open questions for the next agent
- Does the UI ESLint rule (`local/semantic-tokens-only`) also catch raw hex duplicating primitives, or only name-form references? If it misses raw hex, the one detector that exists Rust-side is precisely the one not wired — worth a UI-side audit row.
