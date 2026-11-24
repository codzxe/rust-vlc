# AUDIT: ui/src/screens/player/laneMsg.ts
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 24 / 1 hook |
| File verdict | REAL — the shared lane resolver; deduplication claim verified |
| Reachable from shipped entry point? | YES (PlayerScreen.tsx canvas menu, StageOverlays.tsx:49 HomeOverlay, PlaylistsDock, PlaylistBrowse — the consumers its header names) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read full file; Grep `useLaneMsg` consumers (verified in StageOverlays/PlayerScreen reads) |

## 1. One-paragraph verdict
Correct 20-line hook: catalog-first resolution with lane fallback (`m(key) ?? catalog[key]`), honest caveat that the returned closure is per-render (not for effect deps). The dedup story checks out — StageOverlays.tsx:49 and PlayerScreen.tsx:594 both consume this module instead of local copies (the header's "four divergent copies" consolidation is real in the current tree). Zero findings: checked fallback typing, null-safety of `m()` (returns undefined for unregistered keys -> falls through), and consumer wiring.

## 2. What actually works (verified)
— verified by read + consumer reads (StageOverlays.tsx:49 `useLaneMsg(gfixCKeys)`, PlayerScreen.tsx:594 `useLaneMsg(canvasCatalog)`): one shape, two+ live consumers.

## 3. Findings
None. Checked: type union (MessageKey | keyof T), the `??` fallback path, per-render closure semantics, no stale-catalog hazards (catalog is a prop).

## 4. Per-function table
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| useLaneMsg | 18-24 | REAL | catalog-first, lane fallback |

## 5. Test honesty
No tests in-file; consumers' lane tests exercise it. UNVERIFIED beyond baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The wave-J review counted four divergent copies ... this module is the single shared shape" (L6-9) | header | REAL (UNVERIFIED history — no git) | current tree: StageOverlays/PlayerScreen consume the shared hook (reads) |

## 7. Dead / duplicated / unreachable code
None (this file REMOVED duplication).

## 8. Dependencies & flags
i18n/messages. No feature flags.

## 9. Open questions for the next agent
- None.
