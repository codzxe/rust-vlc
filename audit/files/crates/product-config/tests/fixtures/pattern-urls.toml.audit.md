# AUDIT: crates/product-config/tests/fixtures/pattern-urls.toml
| Field | Value |
|---|---|
| Auditor | W11-A08 |
| Date | 2026-11-23 |
| Lines / functions | 211 / n.a. (TOML fixture) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture; loaded only by crates/product-config/tests/fixtures_matrix.rs:306-312. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | diff vs product.toml (exit 1); Read tool bytes (display-safe); baseline: fixtures_matrix 27/27 pass. |

## 1. One-paragraph verdict
The §3.3 pattern fixture for the https-only URL rule: `urls.help` downgraded from `https://` to `http://`, everything else untouched. Does its job; the consuming assertion is the weakest in the matrix (see F-002), and the base copy is stale like the other 39 fixtures.

## 2. What actually works (verified)
- Mutation verified (Read tool, byte-safe; fixture lines 206-209):
```
[urls]
# Rendered in the Help menu and About (doc 05 §8.3); schema enforces https://.
# [PROPOSAL] placeholder domains until a distribution identity exists.
help = "http://lumen-player.example.org/docs"
```
(root product.toml keeps `help = "https://lumen-player.example.org/docs"`; the other three urls stay https.)
- Consumer: fixtures_matrix.rs:306-312 `pattern_https_url_is_rejected` — exactly one `urls.help` diagnostic, rule contains "https", found starts with `"http://`. Baseline pass (27/27).

## 3. Findings
F-001 [S4]
Where: whole file vs tests/fixtures/README.md:3-4
What is wrong: stale pre-0.1.3 base copy (diff: display_version 0.1.0-dev, min_width 480, streaming-http false, no [web.size_budgets]) — README one-mutation regeneration contract false. Test unaffected.
Confidence: HIGH — Verification: diff.

F-002 [S4]
Where: crates/product-config/tests/fixtures_matrix.rs:310 (consumer)
Code:
```
    assert!(d.rule.contains("https"), "{}", d);
```
What is wrong: the assertion would match any rule text merely containing "https" (e.g. a hint mentioning the scheme), unlike sibling pattern tests that require "must match the pattern". The fixture is fine; the consumer is loose. Recorded here because this fixture is the only one whose rejection is under-asserted.
Confidence: HIGH — Verification: read.

## 4-9. Per-function table / tests / docs / dead code / deps / questions
n.a. (data fixture).
