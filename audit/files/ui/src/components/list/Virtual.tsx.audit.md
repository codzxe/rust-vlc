# AUDIT: ui/src/components/list/Virtual.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / functions | 300 / 5 (ScrollArea, useWindowedRange, VirtualList, VirtualColumn type, VirtualTable) |
| File verdict | REAL (VirtualList/ScrollArea: real, 4 production mounts, strongly tested. VirtualTable: REAL code, ZERO production consumers — dead export) |
| Reachable from shipped entry point? | VirtualList/ScrollArea YES: AppShell.tsx:66/799 (shell sidebar), PlaylistScreen.tsx:355, PlaylistPanel.tsx:263, MediaList.tsx:80 — all four mounts use `semantics="grid"` per the doc's own note. VirtualTable NO production mount: only List.test.tsx and the index.ts barrel reference it (rg across all .tsx). Baseline #22 build OK, #23 vitest 1705 passed incl. List.test.tsx (46 cases). No GUI binary hosts it (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `Read` Virtual.tsx 1-300 (complete); `rg -ln "VirtualList|VirtualTable|ScrollArea" src --glob '*.tsx' --glob '!**/list/*'` (consumers); `rg -n "VirtualTable" src` (only test + barrel — dead in production); `rg -n "VirtualTable|VirtualList" ShortcutsScreen.tsx SettingsScreen.tsx AppShell.tsx PlaylistScreen.tsx PlaylistPanel.tsx MiniPlayer.tsx MediaList.tsx` (mount inventory); `rg -n "describe|it(" List.test.tsx` (46 cases: 10k-row AC, window bounded, no gaps/dupes, scrollToIndex clamp, shrink-mid-window, grid semantics, selection/activedescendant API, re-render discipline, VirtualTable APG numbering + 1000-row scroll); `rg -c "vtable" list.css` (12 — the table's CSS exists); `sed -n` quote verification. |

## 1. One-paragraph verdict
The virtualization core is the real thing and it is tested like it matters: a 10k-row acceptance case asserting a bounded DOM, exact spacer geometry, contiguous windows with no gaps or duplicates on scroll, clamp behavior on out-of-range and shrunken datasets, and a re-render-discipline test proving an in-window scroll event re-renders nothing (the j03 P2-6 state-identity fix). The shrink-lag clamp (159-160, 263-264) genuinely prevents stale `data[i]` renders; the a11y surface (grid/row/gridcell, aria-rowcount, per-row aria-rowindex, activedescendant over stable row ids, selection-truth predicate) matches the APG and is pinned by tests. One real finding: `VirtualTable` has ZERO production consumers — its doc comment claims it is "for grids like the Keyboard-Shortcuts screen (doc 08 §1.6)", but the shortcuts screen renders a plain non-windowed flex list (verified in my shortcuts.css audit: `.shortcuts__rows` plain rows, no VirtualTable import) — dead code whose documentation names a consumer that never adopted it.

## 2. What actually works (verified)
— Verified by baseline #23 (ui-vitest.log.txt, 1705 passed): all 46 List.test.tsx cases, including "renders a bounded DOM window, not 10k rows (< 100 nodes)", "scrolling renders exactly the right contiguous window — no gaps, no dupes", "scrollToIndex clamps out-of-range targets", "never renders past the data when the dataset shrinks mid-window", the grid-semantics and selection/activedescendant suites, the re-render-discipline case (360), and VirtualTable's APG-numbering + 1000-row scroll cases (601-634).
— Verified by read: `useWindowedRange.measure` (26-37) keeps the previous state object when start/end/totalHeight are unchanged — the documented P2-6 fix that stops full-window re-renders inside the overscan band; `scrollToIndex` (52-62) clamps the index, handles start/center/end alignment with non-negative floors, and re-measures after the native jump.
— Verified by read: the shrink-lag clamp (159-160) — `startIndex = min(range.startIndex, len-1)`, `endIndex = min(range.endIndex, len-1)` — and the empty-data case (endIndex −1 → zero rows) are correct; AppShell.tsx:806 documents relying on exactly this guard.
— Verified by read + `rg`: all four production `VirtualList` mounts pass `semantics="grid"` (AppShell, PlaylistScreen, PlaylistPanel, MediaList) — the doc's "GRID-FIRST" claim holds.

## 3. Findings

### F-001 [S3]
Where: ui/src/components/list/Virtual.tsx:241-245 (doc claim), 255-300 (the component)
Code:
```tsx
/**
 * VirtualTable — same windowing math as VirtualList, exposed with table
 * semantics (`role=table`/`row`/`columnheader`/`cell`) for grids like the
 * Keyboard-Shortcuts screen (doc 08 §1.6). The header row stays outside the
```
What is wrong: `VirtualTable` has no production consumer — `rg "VirtualTable"` across all .tsx files matches only its own definition, the index.ts barrel re-export, and List.test.tsx. Its doc comment names the Keyboard-Shortcuts screen as its exemplar consumer, but ShortcutsScreen.tsx does not import it (verified: it renders plain `.shortcuts__rows` rows — 292 bindings' worth, unwindowed). The component, its 12 vtable CSS rules, and its two tests are maintained for a consumer that never adopted it.
Why it matters: dead production code with a doc comment that lies about its purpose — a maintainer reading "for grids like the Keyboard-Shortcuts screen" would reasonably believe the shortcuts list is windowed (it is not), and every change to the windowing hook must keep an unused variant compiling and green. Also a missed perf opportunity: the shortcuts screen renders ~292×2 keycap rows unwindowed while this table sits unused.
What correct looks like: either mount it in ShortcutsScreen (its stated purpose — the screen's row pitch is fixed, a fit for the fixed itemHeight contract) or delete the component, the CSS block, and the barrel export, and fix the doc.
Confidence: HIGH — Verification: read + rg (consumer inventory exhaustive over src/**/*.tsx).

### F-002 [S4]
Where: ui/src/components/list/Virtual.tsx:155-158 (effect deps)
Code:
```tsx
  useEffect(() => {
    if (scrollToIndex) scroll(scrollToIndex);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- scroll on target change only
  }, [scrollToIndex?.index, scrollToIndex?.align]);
```
What is wrong: the scroll fires only when index/align CHANGE — if the dataset is replaced (e.g. filter applied, same active index survives) the list does not re-scroll to keep the target visible; the consumer must nudge the prop. Documented ("scroll on target change only") and all four current mounts are compatible, so this is a contract sharp edge, not a bug today.
Why it matters: future consumers will hit it (target survives a data swap, stays out of view); the eslint-disable hides the exhaustive-deps reminder of exactly this.
What correct looks like: include `data.length` in the deps (re-scroll when the window math changes), or document the nudge requirement in the prop's JSDoc (it is only in the effect comment today).
Confidence: HIGH — Verification: read (both call sites' dep arrays; consumer usage not all traced — behavior reasoned).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ScrollArea` | 13-19 | REAL | — | Labelled scroll region over native scrolling; tabIndex 0; used by AppShell. Test-pinned (117-134). |
| `useWindowedRange` | 22-65 | REAL | — | Memo-identity range state (P2-6); ResizeObserver with jsdom guard; effect deps [count,itemHeight,overscan] re-measure on data change. |
| — `measure` | 26-37 | REAL | — | Keeps prev on unchanged window. Correct. |
| — `scrollToIndex` | 52-62 | REAL | — | Clamp + start/center/end alignment + re-measure. Correct; tested. |
| `VirtualList` | 135-218 | REAL | — | Shrink-lag clamp; grid/listbox branches; aria-selected via optional predicate (omitted when unset — P2-3 fix); stable row ids + activedescendant + data-active (P2-4); aria-rowcount on grid. |
| `VirtualColumn`/`VirtualTableProps` (types) | 220-239 | REAL | — | Type surface. |
| `VirtualTable` | 255-300 | REAL / DEAD-IN-PRODUCTION | F-001 | Header-outside-window anatomy; APG row numbering (+2 offset, rowcount +1); tested but zero production mounts. |

## 5. Test honesty (List.test.tsx, 46 cases, ran in baseline #23)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| computeRange math suite (136-165) | YES | Direct unit oracle incl. stale-scrollTop clamp. |
| 10k-row AC + bounded window + no gaps/dupes + scrollToIndex + shrink-mid-window (167-270) | YES | DOM node-count assertions, content windows, spacer heights — real oracle values. |
| grid-vs-listbox semantics (272-298) | YES | Role/attribute assertions per branch. |
| selection + activedescendant API (300-357) | YES | Attribute-presence AND attribute-absence assertions (unset → omitted). |
| re-render discipline (359-389) | YES | Render-count spy asserts zero re-renders on in-window scroll — pins the P2-6 identity fix. |
| VirtualTable suite (601-634) | YES | APG numbering oracle (rowcount 1001, header row 1, first body row 2) + 1000-row scroll window — but tests a component no screen mounts (F-001). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "VirtualList — windowed listbox (doc 08 §4 virtualization requirement)" (126-128) | this file | REAL | 4 production mounts; 10k-row AC test green in baseline #23. |
| "VirtualTable … for grids like the Keyboard-Shortcuts screen (doc 08 §1.6)" (242-245) | this file | FALSE | ShortcutsScreen imports no VirtualTable (rg); plain unwindowed rows render the 292 bindings. |
| "All four production mounts use `"grid"`" (89-90) | this file | REAL | AppShell/PlaylistScreen/PlaylistPanel/MediaList all verified. |
| "the measured range can lag one render behind a shrinking dataset … consumers must never see a stale `data[i]` (AppShell relies on this)" (131-133) | this file | REAL | Clamp at 159-160; shrink-mid-window test (241); AppShell.tsx:806 guard comment cross-verified. |

## 7. Dead / duplicated / unreachable code
- `VirtualTable` — dead in production (F-001): defined, styled (list.css 12 rules), barrel-exported, tested, consumed by nothing.
- Unreachable as a shipped binary (W14) — global UI condition.

## 8. Dependencies & flags
- Depends on `./range` (computeRange — w15-g003 group, another agent) and list.css. No feature flags; jsdom guards (ResizeObserver typeof check) keep tests honest without mocking the subject.

## 9. Open questions for the next agent
- range.ts owner: the tests pin computeRange's contract tightly — any change there must keep the "no gaps, no dupes" and stale-scrollTop cases green (they are the load-bearing oracle for every list in the app).
- ShortcutsScreen owner: adopting VirtualTable (F-001's fix) would window the 292-row list — verify the row pitch (47px) against `itemHeight` before mounting.
