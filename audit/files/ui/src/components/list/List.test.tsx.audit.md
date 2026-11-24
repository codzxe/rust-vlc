# AUDIT: ui/src/components/list/List.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A05 |
| Date | 2026-11-24 |
| Lines / functions | 638 / 53 tests in 12 describes (+helpers: TabbedScreen, makeRows, shimViewportHeight, screenByTop, cssRule) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — tests ui/src/components/list/index.ts primitives consumed by shipped UI (e.g. ui/src/components/appshell/AppShell.tsx uses VirtualList grid; ui/src/screens/player/variants/MiniPlayer.tsx, ui/src/screens/discovery/DiscoveryScreen.tsx) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; rg inventory (53 it(); earlier `rg -c "  it\("` = 43 was an indent artifact); baseline citation |

## 1. One-paragraph verdict
The strongest kind of component test suite: exact-value pure-function tests (computeRange clamping incl. stale-scrollTop and empty-data edge 0..-1), real DOM-window virtualization tests over 10k rows with contiguity/no-dupes checks, scrollToIndex clamp semantics, re-render discipline proven by a render counter (bail-out AND still-renders-on-real-move), aria semantics for grid vs listbox (aria-rowcount/rowindex/activedescendant with out-of-range and null cases), ListRow anatomy/density/kebab-bubbling/booleanish-edge, and natural-sort behavior with tie-breaks and immutability. CSS assertions are honestly labeled "source check (B1 pattern)" — reading list.css text, which fails on token drift, without pretending jsdom computes styles. All 53 pass in the committed baseline.

## 2. What actually works (verified)
— Committed baseline: audit/runs/baseline/ui-test.txt:33 `✓ src/components/list/List.test.tsx (53 tests) 363ms`; totals :178-179 (1705 passed / 1 skipped).
— Structural: all 12 describes map to real exports of ui/src/components/list/index.ts (rg: `export { naturalCompare, naturalSort } from "./natural"` at index.ts:13; VirtualList/Tabs/ListRow/ScrollArea/VirtualTable/ListEmpty all exported and consumed by shipped screens).

## 3. Findings
None. Zero findings — what was checked and how: (a) every describe read line-by-line and cross-checked against exported primitives (index.ts export list, rg consumers in appshell/player/discovery); (b) mutation reasoning per test below — no tautologies, no asserted-nothing tests, no mocks of the thing under test (the only mock is a clientHeight property shim, a jsdom layout limitation, clearly commented at :162-165); (c) CSS checks read the real committed list.css (:21) and assert token/rule content; (d) the one weak assertion (`queryByText("Network panel")).toBeInTheDocument()` at :65 — presence not visibility) is intentional and commented (hidden panels stay in the DOM); the strong half of that test asserts `Home panel` not.toBeVisible.

## 4. Per-function table (helpers; tests itemized in §5)
| Helper | Lines | Verdict | Notes |
|---|---|---|---|
| cssRule | 23-26 | REAL | regex-extracts a rule body from list.css; selector escaped |
| TabbedScreen | 28-41 | REAL | real Tabs composition |
| makeRows | 160 | REAL | |
| shimViewportHeight | 163-165 | REAL | jsdom clientHeight shim, honest comment |
| screenByTop | 264-266 | REAL | position-based row lookup |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| tabs aria wiring / roving tabindex (44,55) | YES | tabIndex array [-1,0,-1], aria-controls/labelledby cross-ids |
| arrow/Home/End key nav with wrap (68,81,90) | YES | selected tab + activeElement assertions |
| scrollarea token check (128) | YES | fails if list.css drops scrollbar-width:thin / token vars |
| computeRange window/edges/stale (137-157) | YES | exact startIndex/endIndex/totalHeight incl. 0-row edge (endIndex -1) |
| bounded DOM <100 nodes (170) | YES | node count on 10k rows |
| spacer 280000px (181) | YES | exact geometry |
| contiguous window no gaps/dupes (189) | YES | sorted index run + Row 500 content |
| scrollToIndex land/clamp (211,226) | YES | scrollTop exact 56000px / 279972px |
| shrink mid-window (241) | YES | 3 rows exact content, empty → 0 |
| grid vs listbox semantics (273,288) | YES | rowcount/rowindex arrays, absence assertions |
| aria-selected omit vs false (301) | YES | ["false","true","false"] then attribute-absent loop |
| activeIndex wiring (332) | YES | stable ids, activedescendant, null/out-of-range |
| re-render discipline (360) | YES | render counter equality then growth |
| ListRow anatomy/density/selected/kebab (392-466) | YES | class/data attrs, kebab no-bubble via two vi.fn()s |
| row CSS checks (482-518) | YES | token drift fails |
| ListEmpty quiet state (523-548) | YES | no img/svg assertion, optional body/action |
| naturalSort family (552-598) | YES | exact arrays, comparator signs, immutability, stability |
| VirtualTable rowcount/header numbering (608,624) | YES | aria-rowcount 1001, rowindex 97, spacer 32000px |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "visual fidelity ... via dev-shots + VLM" style split (CSS checks are source checks) | List.test.tsx:128,481 comments | REAL | checks read list.css text at :21; they guard drift, not pixels — exactly as labeled |

## 7. Dead / duplicated / unreachable code
None — every exported primitive under test (ListEmpty, ListRow, ROW_HEIGHTS, naturalCompare, naturalSort, ScrollArea, Tab(s/List/Panel), VirtualList, VirtualTable, computeRange) has at least one test.

## 8. Dependencies & flags
Dev-only: @testing-library/react, vitest, node:fs readFileSync for the CSS source checks (import.meta.dirname at :20 — requires Node >= 20.11; repo's vitest runtime provides it, baseline passes).

## 9. Open questions for the next agent
- `cssRule` regex takes the FIRST match per selector — a later overriding rule (media query) would be missed; acceptable for a drift guard, worth noting if list.css grows cascades.
