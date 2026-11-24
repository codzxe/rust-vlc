# AUDIT: crates/product-config/tests/fixtures/pattern-reverse-dns.toml
| Field | Value |
|---|---|
| Auditor | W11-A04 |
| Date | 2026-11-13 |
| Lines / functions | 226 / 0 (fixture data) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test data). Consumed by `tests/fixtures_matrix.rs::pattern_reverse_dns_is_rejected` (fixtures_matrix.rs:263-269). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | baseline citation: audit/runs/baseline/cargo-test-workspace.txt:5094 `test pattern_reverse_dns_is_rejected ... ok` (my fresh `--test fixtures_matrix` rerun was blocked by sandbox disk-full linker Bus error — environment, UNVERIFIED live); diff vs root product.toml; sed verification |

## 1. One-paragraph verdict
Correct single-mutation pattern fixture: `reverse_dns = "Dev.Lumen"` violates `^[a-z0-9]+(\.[a-z0-9-]+)+$` in two ways (uppercase letters), the test pins key `identity.ids.reverse_dns`, rule "must match the pattern", found `"Dev.Lumen"`, and — importantly for my patterns.rs audit — this fixture is the integration-level proof that the hand-written `is_reverse_dns` matcher (which rejects uppercase via `is_ascii_lowercase`) is actually wired into schema validation. Only defect: shared stale base (S4, README audit).

## 2. What actually works (verified)
— Baseline (wave-0 full workspace run): `pattern_reverse_dns_is_rejected ... ok` (cargo-test-workspace.txt:5094); lib-level matcher tests `reverse_dns_rules ... ok` ran fresh in my patterns.rs audit (11 passed).
— Mutation (verbatim, L37-38):
```
reverse_dns = "Dev.Lumen"
```
— Cross-check to patterns.rs: `is_reverse_dns("Dev.Lumen")` returns false because `first.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())` fails on 'D' — fixture, matcher, and test agree.

## 3. Findings
F-001 [S4] Stale base snapshot (identical to the other big fixtures; see README.md audit). Confidence: HIGH — Verification: diff.

## 4-9. (short fixture audit)
Test honesty: exact-match assertions on key path, rule substring, and found value — would fail under matcher regression, diagnostic-shape change, or fixture drift. No line assertion for this fixture (pattern diagnostics without marker_line — consistent with siblings; only fixtures whose tests assert line numbers carry markers). No dead code, no deps. Open questions: none.
