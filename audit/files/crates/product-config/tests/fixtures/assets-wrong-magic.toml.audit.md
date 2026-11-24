# AUDIT: crates/product-config/tests/fixtures/assets-wrong-magic.toml
| Field | Value |
|---|---|
| Auditor | W11-A08 |
| Date | 2026-11-23 |
| Lines / functions | 225 / n.a. (TOML fixture + companion asset files) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture; loaded only by crates/product-config/tests/semantic.rs:248-261 with `base_dir = tests/fixtures/assets`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | diff vs product.toml (exit 1 — no textual mutation beyond the stale-base block); od hexdumps of all four assets (exit 0); `file` on assets; baseline: semantic 17/17 pass. |

## 1. One-paragraph verdict
The path-existence + magic-bytes fixture. The .toml itself carries NO textual mutation — it is the (stale) product.toml with the stock brand paths; the mutation lives in the companion `assets/` miniature repo root: `assets/brand/icon.ico` deliberately begins with ASCII "NOT-ICO" instead of the ICO magic `00 00 01 00`, while the other three assets carry genuine magics. The consumer test asserts exactly ONE diagnostic total, which also proves the three valid assets pass.

## 2. What actually works (verified)
- Asset bytes verified by od (byte-safe, no display mangling):
  - icon.ico: `4e 4f 54 2d 49 43 4f 00 …` = "NOT-ICO" — fails AssetKind::Ico magic `00 00 01 00` (src/semantic.rs:505).
  - icon-512.png: `89 50 4e 47 0d 0a 1a 0a …` — passes Png magic (src/semantic.rs:508).
  - icon.icns: `69 63 6e 73 …` — passes Icns magic (src/semantic.rs:506).
  - logo.svg: starts `<?xml` — passes Svg magic (first non-whitespace/BOM byte `<`, src/semantic.rs:509-517).
- Fixture keeps the stock paths (Read tool, byte-safe; lines 93-97):
```
icon_ico = "assets/brand/icon.ico"
icon_icns = "assets/brand/icon.icns"
icon_png_512 = "assets/brand/icon-512.png"
```
- Consumer: semantic.rs:248-261 `brand_icon_with_wrong_magic_bytes_is_rejected` — one `brand.icon_ico` diagnostic, rule contains "ICO" and "magic", found starts with "file starts with", `assert_eq!(diags.len(), 1)`. Baseline pass.

## 3. Findings
F-001 [S4]
Where: whole file vs tests/fixtures/README.md:3-4
What is wrong: stale pre-0.1.3 base copy (same 39/40 fixture family issue; diff-verified). The README's "exactly one documented mutation" prose is additionally confusing HERE because the mutation is in the assets/ directory, not the .toml — the README's own map row does say "`assets-wrong-magic.toml` + `assets/`", so the map is honest; the regeneration contract is still false.
Confidence: HIGH — Verification: diff.

## 4-9. Per-function table / tests / docs / dead code / deps / questions
n.a. (data fixture + binary companions; `file` reports icon-512.png as "data" because the 16-byte file is a truncated PNG — magic-only validation is the documented contract, so this is correct-by-design).
