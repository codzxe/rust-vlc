# AUDIT: crates/localization/tests/fixtures/locales-plural-ja/ja.toml
| Field | Value |
|---|---|
| Auditor | W11-A05 |
| Date | 2026-11-23 |
| Lines | 8 (6 keys; intended rule-5 violation on one key) |
| File verdict | REAL (fixture) |
| Reachable from shipped entry point? | N/A — consumed by `tests/audit_proof_matrix.rs::rule5_unused_plural_categories_are_flagged` (baseline: ok) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Read (byte-exact via Read tool); `rg` consumer test (exit 0); baseline cited |

## 1. One-paragraph verdict
Correct fixture for the "unused arm" direction of rule 5: Japanese is other-only in the seed table, and `"fixture.items"` deliberately carries a dead `one` arm (both arms render identical text `# 項目`, so the violation is purely structural). The consuming test pins exactly one PluralNonConformance with file `ja.toml`, key `fixture.items`, and a detail naming `one` and `ja` — precise, and it exercises the STRICT branch of audit.rs (known language, arm set must equal the CLDR set). No defects found; checked: keys are flat quoted-dotted (catalog grammar), the 6-key set matches the en-US reference exactly (no missing/orphan/placeholder issues would mask the target issue), and the `count` placeholder matches the reference.

## 2. What actually works (verified)
— baseline `cargo-test-workspace.txt` localization section: `rule5_unused_plural_categories_are_flagged ... ok`; consumer test body read (asserts issues.len()==1 and pins file/key/detail).

## 3. Findings
None. Checked and clean: mutation is exactly the dead `one` arm (fixture.items L7); all other keys are conformant Japanese text with the reference's placeholder shapes; the fixture header comment (L1-2) truthfully describes the violation.

## 4. Per-function table
Data file — N/A.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| audit_proof_matrix::rule5_unused_plural_categories_are_flagged | YES | exact count (1) + variant match + file/key/detail pins; deleting the strict/`unexpected` branch of plural_conformance_detail fails it |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "5 plural (unused arm) … 1 × PluralNonConformance (ja has no `one`)" | tests/audit_proof_matrix.rs:14 (doc table) | REAL | baseline pass + test body pins |
| "ja … Some(&[PluralCategory::Other])" seed | plural.rs:94 | REAL | this fixture proves the audit enforces it |

## 7. Dead / duplicated / unreachable code
The `one` arm is INTENTIONALLY dead copy — that is the fixture's purpose.

## 8. Dependencies & flags
None (TOML data).

## 9. Open questions for the next agent
None. (Related: plural.rs seed-table currency issues are recorded in the plural.rs audit, not here.)
