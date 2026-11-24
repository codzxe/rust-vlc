# AUDIT: ui/src/screens/player/tools/dynamics/copy.ts
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 30 / 1 hook |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: dynamics tools (Compressor/Spatializer/StereoWidener) render via ToolsDialog (Ctrl+E) on /player |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg import sources (exit 0) |

## 1. One-paragraph verdict
Another instance of the standard seam-first copy resolver (catalog -> lane fallback -> raw key), merged from w26.a18 (toggle labels) and w26.c2 (row/staged copy) with a no-duplicate claim. The final `?? key` raw-key fallback differs slightly from the options seam (which would return undefined) — here a missing key renders the key itself, a deliberate loud degradation. No defects.

## 2. What actually works (verified)
— Both source key files exist; merge order documented.
— The loud raw-key fallback makes a missing copy visible in the UI instead of blank.

## 3. Findings
None. Checked: fallback chain, no literals, memoization.

## 4. Per-function table
N/A (<100 LOC): single hook reviewed.

## 5. Test honesty
Dynamics tool tests exercise the resolver (baseline pass).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no key is defined twice" | L15-16 | TRUE | w26.a18 and w26.c2 key sets are disjoint (lane ownership documented) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
None.

## 9. Open questions for the next agent
- None.
