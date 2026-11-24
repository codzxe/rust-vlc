# AUDIT: crates/product-config/tests/fixtures/missing-limits-key.toml
| Field | Value |
|---|---|
| Auditor | W11-A05 |
| Date | 2026-11-23 |
| Lines | 224 (stale base + 1 deletion) |
| File verdict | REAL (fixture; serves its rule row) |
| Reachable from shipped entry point? | N/A — consumed by `tests/fixtures_matrix.rs::missing_limits_key_is_reported` (baseline: ok) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 (shared stale base) |
| Commands run | `diff` vs base (exactly 1 deleted line); python3 tomllib parse (PARSE OK, limits keys = 7, `max_open_sessions` absent); `sed -n '200p' \| od -c`; byte-audit of tail lines via od (see note) |

## 1. One-paragraph verdict
Valid fixture for the required-fields rule: `limits.max_open_sessions` deleted (base line 201 gone; the remaining 7 of 8 `[limits]` keys byte-verified present). The test asserts exactly one diagnostic for `limits.max_open_sessions` with rule "required key is missing". I initially suspected a corrupted tail (`[migrations]` rendering as `igrations]` in two viewers) — octal-dump proved the file is intact (`[   m   i   g   r   a   t   i   o   n   s   ]` at L223) and python tomllib parses it cleanly with a `migrations` key: the earlier display was an output-rendering artifact, NOT file damage. Recorded here so no later agent re-chases it.

## 2. What actually works (verified)
— python3 tomllib: `PARSE OK; limits keys: [...7 keys...]` (max_open_sessions absent, migrations present); baseline `missing_limits_key_is_reported ... ok`.

## 3. Findings
F-001 [S4] Stale base — identical to unknown-top-level.toml F-001 (diff-verified: display_version 0.1.0-dev, version_code 1, 480x320 mins, streaming-http=false, no `[web]`). Additionally, this fixture's `[limits]` block carries the OLD probe/queue comments; the deleted-line mutation is otherwise exact.
Confidence: HIGH — Verification: diff + tomllib.

## 4. Per-function table
Data file — N/A. Mutation (byte-verified): `max_open_sessions = 4` deleted; nothing else (diff shows only `201d200` plus the base's `[extra]` absence).

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fixtures_matrix::missing_limits_key_is_reported | YES | `one()` pins exactly 1 diagnostic for `limits.max_open_sessions` + rule "required key is missing"; dropping the required-key walk for nested limits fails it |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`limits.max_open_sessions` removed" (fixture map) | tests/fixtures/README.md:29 | REAL | tomllib key dump + diff |
| "report every violation, not just the first" (§3.4 stage 2) | doc 07 §3.4 / fixtures_matrix.rs header | REAL for the multi-key case | sibling test `missing_window_fields_reports_every_violation` asserts two diagnostics; this fixture is the single-key complement |

## 7. Dead / duplicated / unreachable code
Duplicated stale base (by design).

## 8. Dependencies & flags
None.

## 9. Open questions for the next agent
None. (Note for tooling: line 216's comment `#   [[migrations.entries]]` and line 223 `[migrations]` are intact per od; do not trust non-od display of this file's `[m`-prefixed lines.)
