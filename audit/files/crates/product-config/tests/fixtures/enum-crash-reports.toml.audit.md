# AUDIT: crates/product-config/tests/fixtures/enum-crash-reports.toml
| Field | Value |
|---|---|
| Auditor | W11-A06 |
| Date | 2026-11-23 |
| Lines / functions | 226 / n/a (test fixture) |
| File verdict | REAL (stale copy) |
| Reachable from shipped entry point? | NO — test input only; consumed by tests/fixtures_matrix.rs:162-170 `enum_crash_reports_is_rejected`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `diff product.toml <fixture>` (mutation + drift identified); rg marker lines; baseline cargo-test-workspace.txt:5040 (fixtures_matrix 27 passed, exit 0). |

## 1. One-paragraph verdict
Real negative fixture: a full product.toml copy carrying the documented mutation `crash_reports = "always"` (line 123; legal set off/ask/send), asserted by an exact test (rule text + found value). The defect is that the fixture is a STALE copy — it diverges from the current repo-root product.toml in ~5 undocumented places because nobody re-applied the README's regeneration procedure after w28 (min sizes), w31.3 (streaming-http), ADR-001 acceptance, the 0.1.3/4 version bump and card 7.K1 ([web.size_budgets]).

## 2. What actually works (verified)
— verified by baseline fixtures_matrix run (27 passed, exit 0) covering this fixture: rejected with key `telemetry.crash_reports`, rule "must be one of: off, ask, send", found "\"always\"".
— mutation confirmed at line 123 (sed); all other enum keys in the file are legal values.

## 3. Findings
F-001 [S4]
Where: enum-crash-reports.toml:27,29,66-67,154 vs repo-root product.toml
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
What is wrong: The fixtures README (tests/fixtures/README.md:5-6) states each fixture "is the repo-root `product.toml` ... with exactly one documented mutation". False for this file: diff vs product.toml shows display_version "0.1.0-dev" (root: "0.1.3"), version_code 1 (root: 4), min_width/min_height 480/320 (root: 940/600, w28 UI-floor fix), streaming-http=false "Phase 6" (root: true, w31.3 shipped), pre-ADR-001 comment blocks, and the whole `[web.size_budgets]` section missing (card 7.K1). ~20 lines of undocumented drift. Works today (schema minimum for min_width is 320 — product.schema.json:116; the mutation is independent), but any future schema tightening (e.g. min_width floor 940) breaks this fixture for a reason unrelated to its documented mutation, and the README's regeneration contract is already violated. Confidence: HIGH — Verification: ran (diff).
Mutations verified: `crash_reports = "always"` at :123 — matches README:32.

## 4. Per-function table
N/A (fixture).
## 5. Test honesty
`enum_crash_reports_is_rejected` (fixtures_matrix.rs:162-170): YES it would fail if the enum check broke — asserts exact rule text and found value.
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "exactly one documented mutation" | fixtures/README.md:5-6 | FALSE (~5 undocumented drifts) | diff vs product.toml |
| Mutation `crash_reports = "always"` | fixtures/README.md:32 | REAL | fixture:123, test asserts rejection |
## 7. Dead / duplicated / unreachable code
The duplicated 220 lines of product.toml per fixture (41 fixtures) — accepted pattern per README, but regeneration is manual and was skipped.
## 8. Dependencies & flags
None (TOML test data).
## 9. Open questions for the next agent
None specific; the drift finding applies to the whole fixture set — a regenerated-fixture sweep would close all of them at once.
