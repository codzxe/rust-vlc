# AUDIT: crates/product-config/tests/fixtures/missing-brand-asset.toml
| Field | Value |
|---|---|
| Auditor | W11-A07 | Date | 2025-06-14 |
| Lines | 224 (product.toml snapshot + 1 deletion mutation) |
| File verdict | REAL (fixture doing its one job) |
| Reachable from shipped entry point? | NO — test fixture; consumed by tests/fixtures_matrix.rs:88-93 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | sed verification of the deleted key; grep icon_png_512; baseline citation |

## 1. Verdict
Required-field fixture: `brand.icon_png_512` is deleted; the validator must report exactly one `brand.icon_png_512` "required key is missing" diagnostic. Verified passing in baseline cargo-test-workspace.txt:5072 (`missing_brand_asset_is_reported ... ok`). NOTE the naming trap: this fixture is the STRUCTURAL case; the SEMANTIC asset-existence case is a different fixture (`assets-missing.toml`, semantic.rs:265) — and the REAL repo is in exactly that semantic state: root `assets/` does not exist, so `cargo xtask config-validator` fails with 4 missing-asset violations (baseline xtask-config-validator.txt:11-15, "result: FAILURE (4 violation(s))"), acknowledged in docs/release-checklist.md:55 as RED today.

## 2. What actually works (verified)
— baseline cargo-test-workspace.txt:5075-5107 (fixtures_matrix 27/27); baseline xtask-config-validator.txt:11-15.

## 3. Findings
F-001 [S4]
Where: fixtures/missing-brand-asset.toml:93-95 (the [brand] block without icon_png_512)
Code:
```
icon_ico = "assets/brand/icon.ico"
icon_icns = "assets/brand/icon.icns"
```
(icon_png_512 deleted — verified absent by grep). Plus the shared whole-file staleness vs root product.toml (32 diff lines). The structural check is field-local, so the fixture still fires exactly once. Confidence: HIGH — Verification: read (grep + diff run).

## 7. Dead/dup
Two fixtures named "missing brand asset" (this structural one + semantic assets-missing.toml) — intentional two-stage coverage, not duplication.

## 9. Open questions
- The real product.toml ships 4 nonexistent asset paths (known RED): does any release lane gate block on it? (release-checklist says full_gate.sh does NOT run config-validator.)
