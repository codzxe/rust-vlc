# AUDIT: ui/src/screens/convert/index.ts
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 17 / barrel (3 re-export groups) |
| File verdict | REAL barrel; the screen it exports is the best-designed dead surface in the app — capability-probed and honestly gated OFF |
| Reachable from shipped entry point? | YES (App.tsx:309 /convert -> lazy ConvertScreen via this barrel) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | wc -l convert files (406+191+64); sed ConvertScreen.tsx:110-175; read full barrel |

## 1. One-paragraph verdict
A correct barrel (ConvertScreen + convertTargets + convertQueue exports; every named export re-exported, no dead members — the sibling modules define all of them, wc/sed verified). The interesting verdict belongs to the screen it fronts: /convert probes the SERVED `system.list-commands`, discovers the catalog carries no convert.*/record.* commands (ConvertScreen.tsx:28-30 documents the verification), and gates the entire lane OFF with an honest aria-wired note — "no fabricated request is ever sent" (L172-175). This is the honest-anti-theater pattern the playlist panel lacked: a dead surface that says so. The queue machinery (convertQueue.ts, 191 lines) ships as library weight until engine commands land.

## 2. What actually works (verified)
— verified by sed + read: probe -> LaneState -> laneGate disables startJob before any dispatch; the failure/absence path resolves to the ABSENT lane ("absence of proof is never proof of support", L120-121).
— verified by wc: all three sibling modules exist with the exported members (spot-checked names in convertTargets/convertQueue via the barrel's import lines resolving).

## 3. Findings
None for the barrel. Checked: export/import parity, no circular re-exports, no side effects beyond CSS import in the screen module (not this file).

## 4. Per-function table
| Item | Lines | Verdict | Notes |
|---|---|---|---|
| barrel exports | 3-16 | REAL | 3 modules, 11 symbols + 3 types |

## 5. Test honesty
convertQueue tests exist in the lane (baseline). The lane-gate behavior is test-covered per module docs (UNVERIFIED beyond baseline).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "card 7.E3 ... export surface" (L1) | header | REAL as route | App.tsx:309 |
| (screen) "catalog carries NO convert.* / record.* commands" (ConvertScreen.tsx:30) | sibling header | REAL | generated/commands.ts CommandRequest union has no convert/record ids (read) |

## 7. Dead / duplicated / unreachable code
None in the barrel. The screen's queue machinery is unreachable-in-product by the honest gate (design intent, not defect).

## 8. Dependencies & flags
Sibling modules only. No feature flags.

## 9. Open questions for the next agent
- ConvertScreen owner (not my assignment): when engine convert commands land, the "single branch" seam at L166-175 needs the lifecycle wiring — audit then.
