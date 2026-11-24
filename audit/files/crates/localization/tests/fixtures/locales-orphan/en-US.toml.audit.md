# AUDIT: crates/localization/tests/fixtures/locales-orphan/en-US.toml
| Field | Value |
|---|---|
| Auditor | W11-A04 |
| Date | 2026-11-13 |
| Lines / functions | 7 / 0 (fixture data: 6 catalog keys) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test data). Consumed by `tests/audit_proof_matrix.rs::rule2_orphan_keys_are_flagged_exactly` (L154-167) via `load_pair("locales-orphan")` (en-US = reference, de.toml = catalog under audit). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `cargo test -p localization --test audit_proof_matrix` -> 13 passed, 1 failed — the failure is `missing_reference_catalog_is_reported`, an environment casualty (see below), NOT this fixture's test (`rule2_orphan_keys_are_flagged_exactly ... ok`); diff vs locales-good/en-US.toml -> IDENTICAL |

## 1. One-paragraph verdict
The reference half of the rule-2 (orphan-key) proof pair: six valid message keys that the sibling de.toml mirrors plus one extra (`fixture.orphan`) — the audit must flag exactly that one key, and it does (fresh test run). All six values parse under the real message-format parser (catalog loading parses every message; a malformed value would surface as a different issue class and break the exact-count assertion). No findings.

## 2. What actually works (verified)
— `cargo test -p localization --test audit_proof_matrix` -> `rule2_orphan_keys_are_flagged_exactly ... ok` (fresh run this session): exactly 1 `OrphanKey { file: "de.toml", key: "fixture.orphan" }` through BOTH the individual `audit_orphan_keys` and the combined `audit_catalogs` entry point.
— Content (verbatim, L1-6):
```
# Fixture: the reference catalog shape (doc 07 §6.2 examples).
"fixture.simple"   = "Plain text"
"fixture.greeting" = "Hello, {name}!"
"fixture.progress" = "Buffering… {percent, number, percent}"
"fixture.source"   = "Opening {source}"
"fixture.items"    = "{count, plural, one {# item} other {# items}}"
```
(plus `"fixture.two" = "Second key"` on L7.)
— Message forms cross-check: the four placeholder forms used here are exactly message.rs's v1 subset (simple / percent / plural with `#`), and `Message::parse` accepts all of them (my message.rs audit ran the parser tests green).
— Byte-identical to locales-good/en-US.toml (diff -> IDENTICAL): deliberate self-contained duplication — each fixture dir carries its own reference copy.

## 3. Findings
None. Checked, with method: (a) file is consumed — rg audit_proof_matrix.rs L155/L162; (b) the test asserts exact issue count, file, and key — not just non-empty; (c) every message value parses (catalog load + exact-count assertion would fail otherwise); (d) the orphan key is deliberately ABSENT from this file (rg `fixture.orphan` -> 0 hits here, 1 hit in de.toml); (e) duplication with locales-good is intentional and harmless (each pair is internally consistent); (f) no secrets, no binaries.

## 4-9. (short fixture audit)
Environment note (not a finding against this file): the same test binary's `missing_reference_catalog_is_reported` fails in this sandbox because `std::env::temp_dir()` is on a 100%-full disk (`mkdir /tmp/<probe>` -> "No space left on device"; the test's `let _ = std::fs::create_dir(&empty);` swallows the error and audits a nonexistent path, yielding 2 issues vs the expected 1). Baseline (cargo-test-workspace.txt:2943) shows this test passing on a healthy disk. The swallowed-create_dir-error is a test-robustness smell in audit_proof_matrix.rs:324-330 (owner: that file's auditor) — an ENOSPC masquerades as an assertion failure with no hint. Test honesty for THIS fixture's test: would fail if orphan detection broke, flagged the wrong key/file, or double-counted. Open questions: none.
