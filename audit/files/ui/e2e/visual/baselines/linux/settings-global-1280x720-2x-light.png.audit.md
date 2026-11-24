# AUDIT: ui/e2e/visual/baselines/linux/settings-global-1280x720-2x-light.png
| Field | Value |
|---|---|
| Auditor | W16-A05 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Kind | Binary fixture — visual-regression baseline PNG |
| File verdict | REAL-UNPROVEN (real raster baseline, never executed here) |
| Reachable from shipped entry point? | N/A — test asset; consumed by ui/e2e/visual/visual.spec.ts via ui/playwright.visual.config.ts |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | file -b; python3 sha256+PNG-IHDR decode (see audit/runs/W16-RR1.datahashes.tsv); rg consumers; ls baselines/linux \| wc -l |

## 1. Verdict
Real, machine-shaped PNG baseline for case "route /settings/global, 1280x720 @2x, light theme". Lane: NIGHTLY-ONLY (dpi=2).
It is decoded, dimension-checked and content-sane, but no Playwright comparison was executed
in this environment (cargo/npm/playwright not run; no visual lane in audit/runs/baseline/),
so its pass-against-fresh-render status is UNVERIFIED.

## 2. What actually works (verified)
- `file -b` -> PNG image data, non-interlaced; IHDR dims match the filename exactly.
- Full zlib+unfilter decode (python3, this run): mean_lum=188, 31 coarse colors — bright chrome consistent with theme=light.
- Consumer chain real: visual.spec.ts:102 toHaveScreenshot(`${id}.png`) with caseId()
  composing exactly this filename from manifest.ts ROUTES/VIEWPORTS/DPIS/THEMES.
Identity/size/sha256: audit/runs/W16-RR1.datahashes.tsv (batch python3 file+sha256, this run).
Naming: matches caseId() at ui/e2e/visual/manifest.ts:144
  `return `${slug(c.route)}-${c.width}x${c.height}-${c.dpi}x-${c.theme}${accent}`;`
Consumer: ui/e2e/visual/visual.spec.ts:102
  `await expect(page).toHaveScreenshot(`${id}.png`, { clip: box });`
CI wiring: .github/workflows/ui.yml `visual` job (every push/PR, ubuntu, blocking) and
`visual-nightly` job (schedule/dispatch only). Matrix size claim verified:
`ls ui/e2e/visual/baselines/linux | wc -l` -> 216 == MATRIX-20261113.md "holds **216** files".

## 3. Findings
F-001 [S4]
Where: .github/workflows/ui.yml:148,205 (job titles) / ui/e2e/visual/MATRIX-20261113.md
Code: `visual (PR-lite lane — 30 cases vs pinned Linux baselines)` / `visual (full matrix — 136 cases, nightly)`
What is wrong: job titles still claim 30/136 cases; manifest.ts now generates 50 PR cases and
216 nightly cases (25-route inventory). Cosmetic staleness only — the `--mode` invocation is
correct. Also: no committed evidence that `visual-nightly` ever executed on GitHub (MATRIX
run record is self-reported prose; sandbox baseline has no visual run at all).
Confidence: HIGH — Verification: read (ui.yml vs manifest.ts:28-29 "PR lane = 50 / 216 nightly").

## 4. What was checked and how (no other findings)
Type via `file`; sha256+size vs generated table; IHDR width/height vs filename tokens;
full pixel decode (luminance, color diversity) vs claimed theme; filename vs caseId()
grammar; consumer rg across ui/; CI lane wiring rg in .github/workflows/ui.yml; on-disk
count vs MATRIX claim. No code executes from this file; nothing else to check.
