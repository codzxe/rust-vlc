# AUDIT: crates/product-config/tests/fixtures/assets/assets/brand/logo.svg
| Field | Value |
|---|---|
| Auditor | W11-A10 |
| Date | 2026-11-23 |
| File | SVG (file: "SVG Scalable Vector Graphics image, ASCII text"), 29 bytes, no dimensions (empty `<svg/>`) |
| File verdict | REAL (fixture stub, purpose-built) |
| Reachable from shipped entry point? | NO — test asset via `asset_base()` (tests/semantic.rs:243-245): `brand_icon_with_wrong_magic_bytes_is_rejected` (:247-263, strict `diags.len() == 1`) and `missing_brand_asset_is_rejected` (:264-275). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | file; od hex dumps (logo.svg + 3 siblings); sed test bodies; ls assets/brand (real tree: MISSING); baseline xtask-config-validator.txt:11-14 |

## 1. Verdict
A 29-byte magic-byte stub: `<?xml version="1.0"?>\n<svg/>\n` (hex-verified
first byte `3c` = '<'), exactly what `AssetKind::Svg::magic_ok` requires
(semantic.rs:508-516). It is one of the "other three assets are valid" that
the wrong-magic test's strict `assert_eq!(diags.len(), 1)` depends on:
sibling icon.ico is deliberately corrupt ("NOT-ICO" bytes) while icon-512.png
(89 50 4E 47 0D 0A 1A 0A) and icon.icns ("icns") carry exact valid
signatures (all hex-verified). The doubled `assets/assets/brand/` layout
mirrors base_dir resolution (fixture product.tomls reference repo-relative
`assets/brand/logo.svg` against base_dir `fixtures/assets`). No doc or
matrix row cites this fixture (rg: assignments/manifest only). Context: the
REAL `assets/brand/` does not exist — baseline config-validator reports all
four brand assets missing (xtask-config-validator.txt:11-14) — so this
fixture is currently the only place in the repo with valid brand-asset magic
bytes.

## 2. Findings
None. Zero findings — checked: file type, magic bytes, consumption path
(both tests, exact-count asserts), sibling signatures, doc/matrix citation
scan (none).
