# AUDIT: ui/src/components/list/list.css
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 490 / ~40 selectors + 1 media block |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — imported by the list primitives (components/list/*), consumed by PlaylistScreen and menus via ListRow/VirtualList |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | class-extraction + TSX consumer scan; verification of dynamic class construction (Row.tsx:74) |

## 1. One-paragraph verdict
Clean, token-disciplined primitive stylesheet (tabs, scroll areas, virtual
list/table, rows, empty states). No dead selectors — the two that a naive
literal grep misses (`lumen-lrow--comfortable/--compact`) are constructed
dynamically in Row.tsx:74 `lumen-lrow--${density}`. The one substantive note
is a documented contrast fix that quietly contradicts its own empty-state
comment.

## 2. What actually works (verified)
- Consumer scan: 38/40 top-level classes referenced literally; the 2 remainder
  verified via Row.tsx:74 template construction (rg output above).
- A11y contrast fix at 469-478 replaces tertiary with secondary ink on
  `__body`, citing axe numbers (3.2:1 dark / 2.45:1 light) — the fix direction
  is real (secondary tier is darker than tertiary by token definition).

## 3. Findings
F-001 [S4]
Where: list.css:469-478
Code:
```
  /* w26.d2 a11y sweep (out-of-lane fix, flagged to the list-lane owner +
   * Lead): tertiary failed the axe color-contrast audit on every consumer
   * (3.2:1 dark / 2.45:1 light). One step up to the title's secondary tier —
   * the two-tone calm is kept by size (body vs caption), not ink. */
  color: var(--color-content-secondary);
```
What is wrong: `__title` (line 466) and `__body` now use the SAME
--color-content-secondary — the visual hierarchy the header comment
(451-452: "centered secondary title + tertiary body") describes no longer
exists in code; the comment at 451-452 is stale. Cosmetic-doc drift only.
Confidence: HIGH — Verification: read.

## 4. Per-function table (>100 LOC; CSS blocks as units)
| Block | Lines | Verdict | Notes |
|---|---|---|---|
| Tabs | 19-67 | REAL | focus ring + disabled present |
| ScrollArea + scrollbar spec | 71-109 | REAL | "honest native scrolling" |
| VirtualList/VirtualTable | 113-200 | REAL | dead sticky rule removed (165-168 comment documents the fix) |
| ListRow base + density | 208-279 | REAL | density classes dynamic (Row.tsx:74) |
| drag handle | 284-325 | REAL | hover-revealed, grab cursors |
| title/meta/duration/trailing | 327-379 | REAL | ellipsis + tabular-nums |
| kebab | 384-447 | REAL | 3px dots/6px pitch per frame |
| empty state | 454-482 | REAL | see F-001 |
| reduced-motion | 484-490 | REAL | |

## 5. Test honesty
No tests in this file; List.test.tsx covers the components (Row.tsx audited
separately below).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "quiet gray text ... hover fill" row grammar | list.css:6-7 | REAL | rules 242-246 |
| "empty lists: quiet secondary text" | list.css:12 | REAL | 463-477 (with F-001 caveat) |
| "Semantic tokens only. Raw px is structural" | list.css:14-15 | REAL | all raw px carries `frame:`/`structural` comments (read) |

## 7. Dead / duplicated / unreachable code
None (the naive-grep misses are dynamic; verified Row.tsx:74).

## 8. Dependencies & flags
Design tokens only. No feature flags. No backend surface.

## 9. Open questions for the next agent
None.
