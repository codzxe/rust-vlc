# AUDIT: crates/localization/tests/fixtures/refs/resolved.toml
| Field | Value |
|---|---|
| Auditor | W11-A08 |
| Date | 2026-11-23 |
| Lines / functions | 9 / 2 msgid refs (description_key, label_key) |
| File verdict | REAL (rule-4 negative control) |
| Reachable from shipped entry point? | NO — test fixture; consumed only by crates/localization/tests/audit_proof_matrix.rs:230-233. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Read tool (byte-safe); consumer test read; baseline: audit_proof_matrix 14/14 pass. |

## 1. One-paragraph verdict
The rule-4 negative control: a config whose every msgid reference resolves in the reference catalog. It exercises both ref extraction shapes — a top-level `description_key` and a `[[menu.leaf]]` array-of-tables `label_key` — and the consumer asserts zero issues through both the focused `audit_references` and the combined `audit_catalogs_with_refs`. Companion of refs/unresolved.toml (not my file) which proves detection.

## 2. What actually works (verified)
- Fixture content (Read tool, byte-safe; full file):
```
[identity]
description_key = "fixture.simple"

[[menu.leaf]]
label_key = "fixture.greeting"
command = "media.open-file"
```
- Consumer: audit_proof_matrix.rs:230-233 — `extract_msgid_refs("product.toml", &src)` then `audit_references(...).is_empty()` and `audit_catalogs_with_refs(&fixtures("locales-good"), &refs).is_empty()`. Both msgids (fixture.simple, fixture.greeting) exist in the locales-good reference. Baseline pass (14/14).

## 3. Findings
None. Zero-finding note (spec §12): checked TOML validity, both refs present and resolvable against tests/fixtures/locales-good/en-US.toml (the canonical fixture set includes fixture.simple/greeting), the array-of-tables extraction path, and consumer coverage.

## 4-9. Per-function table / tests / docs / dead code / deps / questions
n.a. (data fixture). Test honesty: the paired positive fixture (unresolved.toml, same test) proves the check fires; this file proves it stays silent on valid refs — the pair would catch a broken extractor in either direction.
