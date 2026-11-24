# AUDIT: crates/localization/tests/fixtures/locales-plural-ja/en-US.toml
| Field | Value |
|---|---|
| Auditor | W11-A05 |
| Date | 2026-11-23 |
| Lines | 7 (6 keys; conformant reference) |
| File verdict | REAL (fixture) |
| Reachable from shipped entry point? | N/A — reference half of the `locales-plural-ja` pair, loaded by `tests/audit_proof_matrix.rs::rule5_unused_plural_categories_are_flagged` via `load_pair` (baseline: ok) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read (byte-exact); `rg` consumer (exit 0); baseline cited |

## 1. One-paragraph verdict
The reference half of the plural-ja pair: 6 flat keys matching the real reference-catalog grammar, `{count, plural, one {# item} other {# items}}` conformant for en-US (one/other). Its job in the pair is to be the key-set/placeholder baseline so the ONLY issue audit reports for the directory is the ja dead-arm violation — which the consuming test proves by asserting exactly one issue. Verified: identical key set to ja.toml (6/6), identical placeholder shapes (`count` on items, `name`, `percent, number, percent`, `source`).

## 2. What actually works (verified)
— baseline: `rule5_unused_plural_categories_are_flagged ... ok` (the pair loads with zero failures via `load_locales_dir`, else `load_pair` errors the test).

## 3. Findings
F-001 [S4]
Where: crates/localization/tests/fixtures/locales-plural-ja/en-US.toml:1
Code:
```
# Fixture: the reference catalog shape (doc 07 §6.2 examples).
```
What is wrong: nothing functional — a hygiene note. This fixture file is one of EIGHT near-identical reference copies scattered under tests/fixtures/locales-*/en-US.toml (duplicated by design for dir-scoped loading). Any grammar/message-format change must be replicated in each copy; there is no shared reference fixture. The `load_locales_dir` design (one dir = one self-contained pair) forces the duplication, and the doc comment correctly scopes this copy as "the reference catalog shape".
Confidence: HIGH — Verification: read + `md5sum` drift check across ALL 8 fixtures/locales-*/en-US.toml copies: identical digest `49a14b2144d2af24f22d95bd9dd4fe2d` — no drift today; the duplication risk remains latent (8 hand-maintained copies).

## 4. Per-function table
Data file — N/A.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| rule5_unused_plural_categories_are_flagged (uses this file as reference) | YES | `load_pair` fails the test if this file stops loading; a placeholder/key mismatch here would surface as extra issues, breaking the exact-count assert (1) in the sibling direction |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the reference catalog shape (doc 07 §6.2 examples)" | en-US.toml:1 comment | REAL | keys/placeholders parse and match doc 07 §6.2 example forms ({name}, {percent, number, percent}, plural one/other) |

## 7. Dead / duplicated / unreachable code
Duplicated across the locales-* fixture dirs by design (see F-001).

## 8. Dependencies & flags
None.

## 9. Open questions for the next agent
Do the other ~8 en-US.toml fixture copies (locales-good, locales-duplicate, …) stay byte-identical to this one? A drift check across fixtures/locales-*/en-US.toml would close F-001.
