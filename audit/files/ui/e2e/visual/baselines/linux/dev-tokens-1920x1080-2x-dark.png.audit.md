# AUDIT: ui/e2e/visual/baselines/linux/dev-tokens-1920x1080-2x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | binary PNG, 19774 B / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (visual-regression baseline asset). Consumed by ui/e2e/visual/visual.spec.ts:44 test(`matches pinned baseline`) -> expect(page).toHaveScreenshot against baselines/<platform>/; route enumerated in ui/e2e/visual/manifest.ts route table. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 PNG IHDR parse + sha256 (batch) |

Data audit: valid PNG (magic 89504e47, IHDR 1920x244, 8-bit, color-type 2 RGB, clean IEND, zero trailing bytes). sha256 5329bfdaac92…, size 19774 B. Pin provenance recorded in ui/e2e/visual/MATRIX-20261113.md (wave-26 re-pin, tree 507e69c, legitimate-change clause documented). Visual-lane green run UNVERIFIED here (lane not in baseline scripts; Playwright browser lane not executed under load discipline) — the asset itself is genuine and referenced by the harness.

F-001 [S4]: filename encodes the VIEWPORT (1920x1080) but the actual image is 1920x244 — the harness clips to #root whose height is content-driven on the /dev/tokens route. Mechanism consistent (fresh capture clips identically) but the name misleads humans. Note the 1280x720-1x-dark and -2x-dark dev-tokens baselines are byte-identical (sha 62a81d78d01d both) — DPR-insensitive swatch content.