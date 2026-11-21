# AUDIT: ui/src/components/list/Row.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 92 / 1 component (ListRow) + props interface |
| File verdict | REAL (thin, correct primitive; 53-test suite incl. a CSS-source check for the scrollbar lane) |
| Reachable from shipped entry point? | YES as web-UI source: consumers include PlaylistPanel.tsx:339 (queue rows), DiscoveryScreen.tsx:304 (volume rows), PlaylistScreen library rows, settings sidebar — via list primitives imported from "./list". Real shell: no GUI binary (W14). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `grep -rn "data-density" src/` (test-only hook — List.test.tsx:413-424); `grep -n "it(" List.test.tsx` (53 tests: Tabs roving/keyboard, ScrollArea, VirtualList windowing/scrollToIndex/shrink-mid-window, ListRow anatomy/density/selected); class-production cross-check vs list.css (my file-3 audit — all `__lrow` rules produced); baseline #23 |

## 1. One-paragraph verdict
A textbook row primitive: `Omit<…, "title">` cleanly reclaims the label prop from the native tooltip attribute, the Booleanish `draggable` both drives the native attribute and the handle (with `"false"` explicitly not showing the handle, :56), the kebab's click is stopPropagation-wrapped so row onClick cannot fire (:64-67), `actions` replaces the kebab per the documented contract, roles are deliberately delegated to consumers via rest, and every class it emits has a live rule in list.css (verified in my list.css audit). The only nit: the row exposes the same state twice — `--comfortable/--compact` classes AND `data-density`/`data-selected` attributes — where only the classes carry styling and only the data hooks carry tests; two parallel APIs for one truth.

## 2. What actually works (verified)
— verified by List.test.tsx:392-427 (baseline 1705-pass) → anatomy order (icon · title/meta · duration · trailing · kebab), density defaults, selected class + data hook all pinned.
— verified by read → `draggable` prop passes through to the DOM (native drag attribute) while conditionally rendering the handle — one prop, both behaviors, no drift possible.
— verified by cross-ref to my list.css audit → all 15 `lumen-lrow*` selectors in the sheet have their producing sites in this file; conversely this file emits no class the sheet lacks.
— verified by List.test.tsx:128 → the scrollbar token lane has a CSS-source check (B1 pattern) — the list family again carries the strongest CSS verification in the wave.

## 3. Findings

### F-001 [S4]
Where: ui/src/components/list/Row.tsx:74-76
Code:
```
      className={`lumen-lrow lumen-lrow--${density}${selected ? " lumen-lrow--selected" : ""} ${className}`.trim()}
      data-density={density}
      data-selected={selected || undefined}
```
What is wrong: state is exposed twice — density/selected as modifier classes (styled by list.css) AND as data attributes (consumed only by tests). Neither CSS nor production TSX reads the data attributes (grep: List.test.tsx only). Two parallel APIs for the same state invite drift (a consumer styling `[data-selected]` would fight the class rules).
Why it matters: API surface bloat on the most-reused row primitive; the test-only hook should be documented as such or replaced by asserting the class.
What correct looks like: drop the data hooks and assert classes in tests, or a one-line comment marking them test-only.
Confidence: HIGH — Verification: repo-wide grep for both attributes.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ListRowProps` | 5-32 | REAL | — | every slot documented; title-collision Omit correct |
| `ListRow` | 42-91 | REAL | S4 | Booleanish draggable :56; kebab stopPropagation :65; actions-replaces-kebab :57-71; children-after-actions (PlaylistPanel's sr marker rides it); F-001 dual state API |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| ListRow tests (:392-427) | YES | anatomy order, density, selected — DOM assertions |
| Tabs/ScrollArea/VirtualList tests (44 tests) | YES | roving tabindex, window math, no-gaps/no-dupes window, shrink-mid-window, scrollToIndex clamping — behavioral depth |
| scrollbar CSS-source check (:128) | YES | reads list.css, asserts token-colored scrollbar declarations |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Roles and aria wiring belong to the consumer … so the same row serves lists, menus and settings sidebars" | Row.tsx:38-40 | REAL | consumers: playlist grid rows (role=row via VirtualList), discovery rows (tabIndex via rest), settings sidebar |
| "Booleanish: 'false' must NOT show a handle" | Row.tsx:56 | REAL | code + implicit in tests |

## 7. Dead / duplicated / unreachable code
`data-density`/`data-selected` hooks (F-001) — test-only, not dead but unstyled duplicates of the class API.

## 8. Dependencies & flags
list.css, density.ts (asset-group g003). No flags.

## 9. Open questions for the next agent
- VirtualList's grid semantics (my list.css/Virtual.tsx reads) deliberately host interactive children; if any ListRow consumer still mounts `semantics="listbox"` (default) WITH kebab/actions, that's an axe nested-interactive violation — consumer sweep for the playlist/options lane.
