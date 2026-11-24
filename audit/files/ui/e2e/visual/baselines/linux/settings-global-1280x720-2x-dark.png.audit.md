# AUDIT: ui/e2e/visual/baselines/linux/settings-global-1280x720-2x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | binary PNG, 71305 B / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (visual-regression baseline asset). Consumed by ui/e2e/visual/visual.spec.ts:44 test(`matches pinned baseline`) -> expect(page).toHaveScreenshot against baselines/<platform>/; route enumerated in ui/e2e/visual/manifest.ts route table. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 PNG IHDR parse + sha256 (batch) |

Data audit: valid PNG (magic 89504e47, IHDR 1280x720, 8-bit, color-type 2 RGB, clean IEND, zero trailing bytes). sha256 1a4cfa88ebe4…, size 71305 B. Pin provenance recorded in ui/e2e/visual/MATRIX-20261113.md (wave-26 re-pin, tree 507e69c, legitimate-change clause documented). Visual-lane green run UNVERIFIED here (lane not in baseline scripts; Playwright browser lane not executed under load discipline) — the asset itself is genuine and referenced by the harness.
