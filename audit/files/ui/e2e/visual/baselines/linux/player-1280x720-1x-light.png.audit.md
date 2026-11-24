# AUDIT: ui/e2e/visual/baselines/linux/player-1280x720-1x-light.png
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 33,373 / 56ca514582110672 |
| Type / dims | PNG 8-bit RGB, 1280×720 (caseId `player-1280x720-1x-light`) |
| File verdict | REAL (generated baseline) |
| Reachable? | YES — PR-lite lane (1280×720 @1×, manifest.ts:157) + nightly; visual.spec.ts:102 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |

## 1. Verdict
Generated baseline for /player (the idle player screen with honest
placeholders — no media surface bytes), 1280×720 @1× light. Dimensions
match; distinct from 2× light twin (33,362 B, 85cf6d637b1a7591).
## 2. Verified
— python3 IHDR: 1280×720 depth 8 colortype 2; sha256 56ca514582110672….
— Consumer: manifest ROUTES "/player" (:62); visual.spec.ts:102.
— Note: this baseline pins the EMPTY player (no session) — the canvas is the
`data-placeholder="engine-wiring-4.H1"` surface (see PlayerScreen.test.tsx
audit); the baseline honestly depicts the shipped placeholder state.
## 3. Findings
None.
