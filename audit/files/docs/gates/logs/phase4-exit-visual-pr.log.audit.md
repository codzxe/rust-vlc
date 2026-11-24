# AUDIT: docs/gates/logs/phase4-exit-visual-pr.log
| Field | Value |
|---|---|
| Auditor | W17-A05 |
| Date | 2026-11-24 |
| Lines / functions | 42 / n.a. (Playwright visual PR-lane run: 30 cases, 1 worker) |
| File verdict | REAL (data audit: genuine transcript; citing doc's numbers reproduce exactly) |
| Reachable from shipped entry point? | n.a. (log artifact). The lane it records — `npm run visual -- --mode=pr` — still exists (ui/scripts/visual-run.mjs, audited with ui.yml). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | grep citation in phase4-exit.md (:30, :54, :207); route-name extraction (15 unique); ✓-count vs summary (30/30, 1.1m) |

## 1. One-paragraph verdict
A genuine Playwright visual-diff transcript: npm script header, the real visual-run.mjs banner ("[visual] mode=pr update=false seed=false"), environment warning noise, 15 routes × {dark,light} = 30 cases all ✓, "30 passed (1.1m)". Its citing doc's claim — "Re-verified THIS session: `npm run visual -- --mode=pr` → 30 passed (1.1 m), exit 0" (phase4-exit.md:54) — reproduces byte-for-byte on count and duration. The route set is the 15-route era; today's manifest carries 25 routes (PR lane = 50), so this evidence is period-correct but not current-tree coverage.

## 2. What actually works (verified)
— verified by counting: 30 ✓ lines; 15 unique route names extracted from test ids; summary "  30 passed (1.1m)".
— verified by citation: phase4-exit.md:30/:54 "30 passed (1.1 m), exit 0", :207 "30 passed" — count + duration match; the :30 row also cites the G2-era 136/136 nightly + 43 s PR-lane from MATRIX-20261024.md (historical layers kept distinct from this session's re-run — no conflation).
— verified by lane existence: ui/scripts/visual-run.mjs + playwright.visual.config.ts + baselines/linux/ (216 PNGs, audited with ui.yml).

## 3. Findings
F-001 [S4]
Where: docs/gates/logs/phase4-exit-visual-pr.log (whole file)
Code:
```
> lumen-ui@0.1.0 visual
> node scripts/visual-run.mjs --mode=pr
```
What is wrong: no capture header and no exit marker (grep exit -> 0 hits); the "exit 0" lives only in the citing doc. Same convention gap as the other phase-4 logs. Confidence: HIGH — Verification: read + grep.

## 4. Per-function table
Not applicable (data/log). Composition verified: 15 routes (player, player-music, miniplayer, playlist, media-info, settings×5, onboarding, keyboard-shortcuts, open-network-stream, not-found, dev-tokens) × dark + light.

## 5. Test honesty
Each case asserts "matches pinned baseline (pr)" — pixel comparison against committed baselines; would fail on visual drift (the SEED_2PX=1 mechanism proves the harness CAN fail — see visual-run.mjs audit under ui.yml).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Re-verified THIS session: npm run visual -- --mode=pr → 30 passed (1.1 m), exit 0" | phase4-exit.md:30, :54 | REAL | log summary "30 passed (1.1m)" |
| "3-OS: infra-blocked — Linux-only sandbox; baselines/linux/ only populated" | phase4-exit.md:30 | REAL | baselines/ contains only `linux` (ls, ui.yml audit) |

## 7. Dead / duplicated / unreachable code
n.a.

## 8. Dependencies & flags
Playwright via visual-run.mjs (spawnSync of @playwright/test cli); chromium; deterministic-rendering controls per visual.spec.ts.

## 9. Open questions for the next agent
- Today's PR lane is 50 cases (25 routes per manifest.ts) — no log in docs/gates/logs records a 50-case green run (MATRIX-20261113.md does: "50 passed (1.7m)" capture + 216/216 nightly). The gates/ evidence dir has no post-w26.k20 PR-lane log. UNVERIFIED whether the 50-case lane ever ran green outside MATRIX-20261113.md's own record.
