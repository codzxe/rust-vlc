# AUDIT: ui/src/components/list/density.ts
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 22 / 0 (type + constant) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — Row.tsx + PlaylistPanel/PlaylistScreen consume ROW_HEIGHTS/RowDensity |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -ln "ROW_HEIGHTS|RowDensity"` (3 prod consumers) |

## 1. One-paragraph verdict
Two-export design-constants module (density variant + Figma-measured pitches for the VirtualList windowing math), split from Row.tsx for react-refresh cleanliness. Frame citations on both values. Zero findings.

## 2. What actually works (verified)
— verified by consumer sweep; the pitch feeds VirtualList itemHeight (windowing math lock-step, per docstring).

## 3. Findings
None.

## 4. Per-function table
| Export | Lines | Verdict | Notes |
|---|---|---|---|
| RowDensity / ROW_HEIGHTS | 12-22 | REAL | structural px, no token exists (documented) |

## 5. Test honesty
Covered via Row/VirtualList tests (baseline).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| pitches per density (66/40) | comments | REAL (frame-cited) | consumers pass them to VirtualList |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
None.

## 9. Open questions for the next agent
- None.
