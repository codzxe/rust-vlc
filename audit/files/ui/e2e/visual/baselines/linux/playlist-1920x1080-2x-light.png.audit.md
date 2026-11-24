# AUDIT: ui/e2e/visual/baselines/linux/playlist-1920x1080-2x-light.png
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A02) |
| Date | 2026-06-14 |
| File verdict | REAL — committed visual-regression baseline of the nightly screenshot-diff lane |
| Reachable from shipped entry point? | NO — test artifact; consumed by ui/e2e/visual/visual.spec.ts:102 `await expect(page).toHaveScreenshot(`${id}.png`, { clip: box })` with id == caseId of manifest.ts:142-145 (route /playlist × 1920×1080 @ 2x light) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256 batch (this run): 32,821 bytes, sha256 5bed730be93bd3d583ca8bd1ef8c9513f498016644aa1759f1a541fbe083a48b, head 89504e47…; `file -b` -> PNG 1920 x 1080, 8-bit RGB non-interlaced; rg `playlist-1920` audit/runs/W16-A08.fingerprints.txt -> 0 hits (no prior pin) |

Generated-vs-handmade: browser-captured golden (Playwright chromium, clip to #root, visual.spec.ts:79-102); provenance ledger docs/waves/w26/baselines/MATRIX-20261112.md (manifest.ts:108). Lane: nightly only (1920×1080 and dpi 2 → pr false on both axes, manifest.ts:157).
Honesty: load-bearing byte golden (threshold 0.1 / maxDiffPixelRatio 0.001, visual.spec.ts:96-101) — the playlist screen's light-mode 2x rendering is diff-gated here. No feature-matrix.toml row cites it. Nightly-lane execution UNVERIFIED (no playwright in this environment; the nightly schedule exists at .github/workflows/ui.yml:58).
