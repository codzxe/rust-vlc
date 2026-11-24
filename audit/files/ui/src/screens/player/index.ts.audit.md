# AUDIT: ui/src/screens/player/index.ts
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 55 / 0 (barrel re-exports) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — consumed by App.tsx (lazy player import) + variants consumers |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read; export-vs-module existence spot-checks (PlayerScreen, useChromeAutoHide, keymap, layout, variants) |

## 1. One-paragraph verdict
A pure barrel: re-exports PlayerScreen, useChromeAutoHide, useFullscreen, keymap + layout parsers, and the variants surface. Every spot-checked export exists in its source module. NOTE: as a barrel it re-exports useChromeAutoHide — the module with the tree-breaking syntax error (useChromeAutoHide.ts:121, see that audit) — so this barrel is also unbuildable as committed.

## 2. What actually works (verified)
— verified by rg/read: all 5 source modules exist and carry the named exports (PlayerScreen.tsx: PLAYER_OPEN_CUSTOMIZE_EVENT / PLAYER_TOGGLE_SIDEBAR_EVENT present; variants/index carries the variant surface).

## 3. Findings
Zero findings of its own. (The build-blocking defect lives in a re-exported module — counted there.)

## 4. Per-function table
N/A (barrel).

## 5. Test honesty
N/A.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Card 4.C1 — player screen shell public surface" | L1 | REAL | exports match the shell's consumers |

## 7. Dead / duplicated / unreachable code
None (all exports have consumers or are public API of the frozen barrel).

## 8. Dependencies & flags
None.

## 9. Open questions for the next agent
- None.
