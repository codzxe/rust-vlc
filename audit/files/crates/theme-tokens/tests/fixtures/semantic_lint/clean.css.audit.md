# AUDIT: crates/theme-tokens/tests/fixtures/semantic_lint/clean.css (fixture)
| Field | Value |
|---|---|
| Auditor | W11-A02 |
| Date | 2026-11-23 |
| Lines | 16 |
| File verdict | REAL (negative-control fixture doing its job) |
| Reachable from shipped entry point? | N/A — test-only; consumed by crates/theme-tokens/tests/semantic_lint.rs:94-103 (`clean_fixture_passes_the_check`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | grep of every `--token` name against themes/generated/tokens.css (all present); `cargo test -p theme-tokens --offline` (semantic_lint 2/2 pass — includes this fixture's test) |

## 1. Purpose / consumer / cross-check
Purpose: the NEGATIVE control for the crate-side semantic-consumption lint (`lint_source`, doc 08 §3.2 "Components reference semantic tokens only") — a component-style CSS source that references only semantic custom properties and must therefore produce zero violations. Header (L1-3, verbatim):
```
/* Fixture (card 1.B4): a component-style source that consumes semantic
 * tokens only (doc 08 §3.2) — `lint_source` must report nothing. */
```
Consumer (semantic_lint.rs:94-103) parses the COMMITTED themes/tokens.json (not a fixture token set) and asserts `lint_source(&set, &text).is_empty()`. Cross-check vs lint semantics: every var() name used — `--color-content-primary`, `--color-surface`, `--radius-control`, `--shadow-card`, `--blur-backdrop`, `--color-accent`, `--color-content-on-accent` — exists in themes/generated/tokens.css (grep counts 1-9 each). Per lint.rs's documented semantics, an alias CSS form that coincides with a semantic name is legitimate; all seven here are genuine semantic tokens, so the clean-pass expectation is correct, and the file contains no raw color literals or dotted primitive paths that the lint would flag.

## 2. What actually works (verified)
— `cargo test -p theme-tokens --offline` → semantic_lint suite 2/2 pass (this fixture's test plus the violating.css positive case), i.e. the fixture demonstrably lints clean through the real committed token source.

## 3. Findings
None. Checked: token names resolve (grep), no literals, no primitive references, test asserts emptiness against the real token set (would fail if the lint flagged semantic references or if these tokens were renamed out from under the fixture — a genuine drift tripwire).

## 4. Fixture verdict
REAL: correct negative control, wired to the real committed source, executed green.

## 5. Open questions
- None.
