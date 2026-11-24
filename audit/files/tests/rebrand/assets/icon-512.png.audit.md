# AUDIT: tests/rebrand/assets/icon-512.png
| Field | Value |
|---|---|
| Auditor | W16-RR4 (recovery; original agent W16-A05) |
| Size / sha256 | 1,818 B / 74657efc1462340401f2a60923e4eeecd85a4102dbe93a7507c86e4b26242077 |
| Type | PNG 512×512 8-bit RGBA non-interlaced — REAL renderable image, solid #FF5500 (decoded: 1 distinct sampled color, all filter-0 rows, IHDR+IDAT+IEND) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture (rebrand xtask lane); the shipped product icon (assets/brand/icon-512.png) does NOT exist in the repo (config-validator RED) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | file(1); python3 sha256 + chunk walk + full zlib pixel decode; rg consumers |

## 1. One-paragraph verdict
A genuine, renderable 512×512 RGBA PNG whose content is one flat accent color
(#FF5500 — the Nova fixture's documented "accent #ff5500 everywhere" claim in
nova.toml:18, TRUE by my decode). It is the icon_png_512 brand asset of the
rebrand test product and is consumed by two green baseline tests: the rebrand
skeleton run copies it into the dest tree and asserts "4 asset(s) copied"
(rebrand_skeleton.rs:460-478, baseline :6788 ok), and the acceptance suite loads
nova.toml through the real product-config pipeline incl. magic-byte validation
(rebrand_acceptance.rs, baseline :5142 ok).

## 2. What actually works (verified)
— python3: PNG signature ok, IHDR 512×512/8/6 (RGBA), IDAT 1,761 B, IEND; zlib
decompressed 1,049,088 B == 512×(512×4+1) exactly; row filters {0}; sampled
colors {ff5500ff} — top-left/top-right/center all ff5500ff. Renders as a solid
orange square.
— Consumers (rg, non-audit): tests/rebrand/nova.toml:65 `icon_png_512 =
"tests/rebrand/assets/icon-512.png"`; rebrand_skeleton.rs:68 NOVA_ASSETS +
:460-478 copied-asset assertions — GREEN baseline cargo-test-workspace.txt:6788;
rebrand_acceptance.rs surface_fixture_load_nova_through_the_real_pipeline —
GREEN baseline :5142. Docs: docs/release-checklist.md:55 cites the missing REAL
asset (validator RED), docs/07-config-driven-system.md:177 shows the schema shape.

## 3. Findings
F-001 [S4]
Where: tests/rebrand/assets/icon-512.png (content) vs product.toml's assets/brand/icon-512.png (absent)
Code: `violation: brand.icon_png_512: asset path `assets/brand/icon-512.png` does not exist under the root` (baseline xtask-config-validator.txt:13)
What is wrong: the file is placeholder artwork (one flat color) — honest for a
rebrand test product, but the shipped product's own icon asset remains missing
from the repo: the config validator is RED with 4 missing brand assets
(baseline xtask-config-validator.txt:11-14) and release-checklist.md:55 records
the waiver ("packaging ships the deterministic generated PNG instead").
Why it matters: a flat orange square is a valid test stand-in; nothing here is
fake — but any doc implying real branding exists would be false.
Confidence: HIGH — Verification: ran python3 decode + rg.

## 4. Matrix evidence
No feature-matrix row cites this exact path; the rebrand lane's matrix rows cite
nova.toml/rebrand xtask generally. Wave-11 audits of the product-config stub
assets (16-B magic-only icon-512.png) are a DIFFERENT file family — this one is
the full renderable fixture.

## 5. Open questions for the next agent
- None. Sibling assets icon.ico / icon.icns / logo.svg of this family ride the
  same pattern (icns/ico authenticity worth one file(1) check each when assigned).
