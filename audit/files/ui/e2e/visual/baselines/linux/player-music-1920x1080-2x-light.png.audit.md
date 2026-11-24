# AUDIT: ui/e2e/visual/baselines/linux/player-music-1920x1080-2x-light.png
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | binary asset, 307667 bytes (10 assigned chunks L1-7512) |
| File verdict | REAL-UNPROVEN (committed baseline; harness real; no committed run evidence) |
| Reachable from shipped entry point? | NO (test artifact; consumed by Playwright visual.spec.ts) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `file` (exit 0); python3 PNG header parse (exit 0); rg manifest/docs (exit 0) |

## 1. One-paragraph verdict
Committed screenshot baseline for visual case `/player/music` @1920x1080, DPI 2x, light, default
accent (per caseId format in ui/e2e/visual/manifest.ts:142-146). `file` confirms genuine PNG,
1920x1080 8-bit RGB non-interlaced; PNG IHDR parsed by me (bitdepth 8, colortype 2). Dimensions
are CSS-pixel dims because playwright.visual.config.ts pins `scale: "css"` in
expect.toHaveScreenshot — so 2x baselines carry the same pixel size as 1x. The capture harness
is real and elaborate (216-case manifest, deterministic-render flags, seeded 2-px regression
proof), but no committed evidence of any visual-lane run exists.

## 2. What actually works (verified)
- `file` -> "PNG image data, 1920 x 1080, 8-bit/color RGB, non-interlaced".
- Case id derivation: manifest.ts:142-146 `caseId` -> `player-music-1920x1080-2x-light`
  (route slug + viewport + dpi + theme); the filename encodes real case axes.
- Consumer: ui/e2e/visual/visual.spec.ts:104 `await expect(page).toHaveScreenshot(`${id}.png`, { clip: box })`
  with clip = live #root boundingBox; snapshotPathTemplate baselines/{platform}.

## 3. Findings
F-001 [S3]
Where: this baseline + audit/runs/baseline/* (absent) + scripts/full_gate.sh (no visual section)
What is wrong: the visual lane never ran in any committed baseline (rg "visual" over
baseline/full_gate.txt + rc-smoke.txt -> zero matches; ui-test.txt is vitest only). All 216
committed baselines are therefore REAL-UNPROVEN as regression evidence: nobody has shown the
harness passing against them in the recorded environment.
Why it matters: a screenshot suite whose baselines have no passing run on record protects
nothing; per-pixel drift between baseline and current tree is unmeasured.
What correct looks like: a committed visual-run log (mode=pr at minimum) in the gate.
Confidence: HIGH — Verification: read (baseline logs + gate script contents).

F-002 [S4]
Where: ui/README.md:135-136
Code: "`--mode=nightly` (`npm run visual:nightly`) = the full 32-case matrix."
What is wrong: manifest GRID = 25 routes × 2 viewports × 2 DPI × 2 themes = 200 + 16 accent
samples = 216 nightly cases (manifest.ts:148-176; MATRIX-20261113.md:45). "32" is the pre-4.G2
4-route count. Doc rot.
Confidence: HIGH — Verification: read (manifest arithmetic + README).

## 4. Provenance
Baseline provenance claim: docs/waves/w26/baselines/MATRIX-20261112.md (w26.k20 re-shoot of the
216-case matrix; player-music re-audited in docs/waves/w26/logs/w26.d2.md:70 "Post-fix re-shoot
+ re-audit: player, player-music, miniplayer → all"). Run-verified status: UNVERIFIED (no
Playwright in sandbox; no committed run log).

## 5. Open questions
- dev-tokens baseline is 1280x244 (content-clipped) — see its audit; confirm the clip-based
  capture makes short pages intentionally variable-height (design intent, not a defect).
