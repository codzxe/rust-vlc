# AUDIT: crates/product-config/tests/fixtures/missing-section-identity.toml
| Field | Value |
|---|---|
| Auditor | W11-A03 |
| Date | 2026-11-13 |
| Lines | 186 (negative fixture: product.toml fork, the whole `[identity]` subtree L17-55 removed) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test-only asset; consumed by fixtures_matrix.rs:59-67 (`missing_identity_section_is_reported`). Not shipped. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `diff product.toml <fixture>` (exit 0: entire [identity]/[identity.ids.*] block deleted); baseline fixtures_matrix 27 passed; direct execution blocked (disk full) — baseline cited |

## 1. One-paragraph verdict
Mutation matches README.md:24 ("whole `[identity]` subtree removed"). The fixture jumps from the §2.1 header prose straight to the §2.2 window comment (sed-verified at L13-18: the "# --- 2.1 Identity ---" table is gone entirely). The consuming test asserts key `identity`, rule "required key is missing", found "absent", and — notably honest — `assert!(d.line.is_none())` with the comment "A wholly missing top-level table has no line to point at": the test pins the None-line edge case instead of pretending a line exists. Baseline green. Two S4s: the sibling fork rot, plus this fixture keeps the §2.1 prose comment block while deleting the table it describes (cosmetic confusion).

## 2. What actually works (verified)
- diff: `17,55d16` — the entire identity subtree (name, short_name, publisher, display_version, version_code, description_key, [identity.ids] and all four OS id tables) removed.
- Test read at fixtures_matrix.rs:59-67: `assert!(d.line.is_none(), "{}", d)` — the None-line contract.
- Baseline: fixtures_matrix `test result: ok. 27 passed; 0 failed`.

## 3. Findings
F-001 [S4] — inherited fork rot (same class as siblings; base predates w28/w31 product.toml).
F-002 [S4] — the "# --- 2.1 Identity ---" section prose above the deletion site survives while its table is deleted; a reader of the fixture sees an identity header with no identity content. Cosmetic; the diff is still unambiguous.
Confidence: HIGH — Verification: ran diff + sed.

## 4-9. (short audit)
Per-function table: N/A (data). Test honesty: exactly-one + None-line pin. Doc claims: README.md:24 accurate. Dead code: none. Dependencies: none. Open questions: none.
