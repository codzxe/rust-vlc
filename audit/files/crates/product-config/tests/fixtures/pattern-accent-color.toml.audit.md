# AUDIT: crates/product-config/tests/fixtures/pattern-accent-color.toml
| Field | Value |
|---|---|
| Auditor | W11-A06 |
| Date | 2026-11-23 |
| Lines / functions | 226 / n/a (test fixture) |
| File verdict | REAL (stale copy) |
| Reachable from shipped entry point? | NO — test input only; consumed by tests/fixtures_matrix.rs:254-261 `pattern_accent_color_is_rejected`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `diff product.toml <fixture>`; rg marker lines; baseline cargo-test-workspace.txt:5040 (fixtures_matrix 27 passed, exit 0). |

## 1. One-paragraph verdict
Real negative fixture: `accent_color = "4f8cff"` at line 91 (missing the leading `#`) triggers the brand-color pattern rejection, asserted exactly by the matrix test. Same stale-copy defect as the rest of the fixture set: ~5 undocumented divergences from the current repo-root product.toml, contradicting the README's "exactly one documented mutation" contract.

## 2. What actually works (verified)
— verified by baseline fixtures_matrix run (27 passed, exit 0): rejected at `brand.accent_color`, rule mentions `#`, found "\"4f8cff\"".
— mutation confirmed at line 91 (sed): the only pattern-violating value in the file.

## 3. Findings
F-001 [S4]
Where: pattern-accent-color.toml:27,29,66-67,154 vs repo-root product.toml
Code:
```
display_version = "0.1.0-dev"
```
```
min_width = 480               # schema: min_width < width
```
```
streaming-http = false          # Phase 6
```
What is wrong: Same undocumented drift as every fixture (README:5-6 claims "exactly one documented mutation"): version 0.1.0-dev/1 vs root 0.1.3/4, min sizes 480/320 vs root 940/600 (w28), streaming-http=false "Phase 6" vs root true (w31.3), pre-ADR-001 comments, `[web.size_budgets]` missing (7.K1). Harmless to the mutation's purpose today; a latent spurious-failure source on any schema/product tightening. Confidence: HIGH — Verification: ran (diff).
Mutation verified: `accent_color = "4f8cff"` at :91 — matches README:41 ("no `#`").

## 4. Per-function table
N/A (fixture).
## 5. Test honesty
`pattern_accent_color_is_rejected` (fixtures_matrix.rs:254-261): YES — asserts key path, rule contains "#", found value exactly.
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "exactly one documented mutation" | fixtures/README.md:5-6 | FALSE (drift) | diff vs product.toml |
| Mutation `accent_color = "4f8cff"` (no #) | fixtures/README.md:41 | REAL | fixture:91 + rejecting test |
## 7. Dead / duplicated / unreachable code
Duplicated product.toml body — see fixture-set note in enum-crash-reports.toml.audit.md F-001.
## 8. Dependencies & flags
None (TOML test data).
## 9. Open questions for the next agent
None.
