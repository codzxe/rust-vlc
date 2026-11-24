# AUDIT: ui/src/components/list/Virtual.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 300 / 5 (ScrollArea, useWindowedRange, VirtualList, VirtualTable + helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — used by AppShell.tsx (sidebar playlists), PlaylistScreen/PlaylistPanel, MiniPlayer, MediaList, ShortcutsScreen (table); verified by rg -l |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | read of range.ts math; rg user census; manual edge-case walk (empty data, shrink lag, align math) |

## 1. One-paragraph verdict
Real fixed-height windowed virtualization over native scrolling: the range math (range.ts computeRange) is pure and correct (clamped scrollTop, overscan both sides, exact spacer height), the state dedupe avoids re-render storms, shrink-lag is double-clamped (in computeRange AND at render), ARIA grid/listbox/table semantics are APG-correct (aria-rowindex with header offset, aria-activedescendant, omitted-not-false aria-selected). This is the opposite of a stub. One caveat: the `semantics="listbox"` default path implements no option focus/selection model (documented in-code); all four production mounts use "grid".

## 2. What actually works (verified)
— verified by manual math walk: computeRange(1000, 36, scrollTop=10000, vh=720, overscan=5) → firstVisible=277, visibleCount=20, window 272..301 — correct; empty data (count=0) → startIndex 0, endIndex -1 → no rows, spacer 0 — correct; scrollToIndex align="center" subtracts (clientHeight-itemHeight)/2 — correct.
— verified by rg: 9+ production consumers (AppShell, PlaylistScreen/Panel, MiniPlayer, MediaList, SettingsScreen escape-dismiss note).
— NOT executed in a browser (no runtime budget) — REAL by code + tests exist in lane (range tests referenced by comments; perf test asserts ~25 nodes for 10k rows — UNVERIFIED run).

## 3. Findings
F-001 [S4]
Where: ui/src/components/list/Virtual.tsx:78-99 (semantics prop doc)
What is wrong: the default `"listbox"` path ships a degenerate a11y model (unfocusable options, container tab stop, no selection surface) — kept "for wave-1 API compatibility" while all real consumers use "grid". A footgun default that a new consumer gets silently. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ScrollArea | 10-17 | REAL | - | labelled region over native scroll |
| useWindowedRange | 20-69 | REAL | - | measure + ResizeObserver + dedupe; jsdom guard |
| VirtualList | 100-246 | REAL | - | grid/listbox branches, clamped window, activedescendant |
| VirtualTable | 248-300 | REAL | - | sticky header row 1, i+2 body rows, rowcount+1 |

## 5. Test honesty
Tests live in the lane's test files (not in my list) — comments reference a perf test asserting O(viewport) node count; UNVERIFIED execution.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a 10k-row list renders ~25 row nodes, not 10k (asserted by the perf test)" | range.ts:10-11 | REAL-UNPROVEN | math supports it; perf test not run |
| "All four production mounts use 'grid'" | L78-80 | REAL | rg census shows grid consumers (AppShell/Playlist/MiniPlayer/MediaList) — exact four-way count not re-verified |

## 7. Dead / duplicated / unreachable code
None found. isItemSelected/activeIndex are optional additive props, consumed by real mounts.

## 8. Dependencies & flags
No deps beyond React + list.css. No env vars.

## 9. Open questions for the next agent
- None material.
