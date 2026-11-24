# AUDIT: crates/product-config/tests/fixtures/unknown-key-typo.toml
| Field | Value |
|---|---|
| Auditor | W11-A09 |
| Date | 2026-11-23 |
| Lines | 226 (full product.toml copy, one mutation) |
| File verdict | REAL |
| Reachable | Test fixture — consumed by crates/product-config/tests/fixtures_matrix.rs:347-367 `unknown_key_typo_gets_a_did_you_mean_hint` (passes, baseline: `test unknown_key_typo_gets_a_did_you_mean_hint ... ok`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## Verdict
Single-mutation fixture: `cache_demux_mb = 32` (line 195) replaces the real key `cache_demux_mib` — the classic MiB/MB typo the additionalProperties:false typo-detection is designed for. The test asserts the FULL consequence chain, not just the unknown key:
```
cache_demux_mb = 32
```
1. exactly one diagnostic for `limits.cache_demux_mb` with rule "unknown key" and hint containing "did you mean `cache_demux_mib`" (the Levenshtein suggester, crate::suggest) — pinned to the exact line;
2. exactly one diagnostic for `limits.cache_demux_mib` with "required key is missing" — proving the aggregate report-every-violation behavior (doc 07 §3.4 stage 2) instead of first-error-only.
This is the strongest fixture shape in the set: it verifies the suggester, the unknown-key rule, and the required-key rule interact correctly in one file. Would fail if did_you_mean were deleted, if the typo rule were dropped, or if required checking stopped after the first error.

F-001 [S4] Stale base (0.1.0-dev / version_code 1 / 480x320 / streaming-http=false / no `[web.size_budgets]`) — same class as siblings; the structural gate keeps drift loud. Confidence: HIGH — Verification: read + diff.

## Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fixtures_matrix.rs:347 unknown_key_typo_gets_a_did_you_mean_hint | YES | pins both diagnostics (unknown + missing-required), the suggestion text, and the source line |
