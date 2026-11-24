# AUDIT: ui/src/screens/player/tools/video/useDropdownScrollportFlip.ts
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 299 / 5 (2 exported pure + 2 exported DOM fns + 1 hook) + 2 private helpers |
| File verdict | REAL (pure placement decision + imperative DOM application, unit-pinned; scoped stopgap by its own admission) |
| Reachable from shipped entry point? | YES — consumed by ColorsTool.tsx and HdrTool.tsx (rg), which render in the player tools dialog → PlayerScreen → App. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg callers (ColorsTool, HdrTool, video.css, 2 test files); baseline ui-test.txt:95 `✓ …/useDropdownScrollportFlip.test.ts (12 tests) 30ms` |

## 1. One-paragraph verdict
Real, careful geometry: `resolveListPlacement` is a pure function (fits clip → inflow; fits viewport → fixed escape below/above; else capped with a 96px two-row floor), `applyPlacement`/`clearPlacement` are extracted DOM seams the tests pin, and the hook observes list mounts via MutationObserver with full re-decision on scroll/resize. The self-declared status ("DELETE THIS MODULE when the primitive gains the escape") is honest — it is a scoped stopgap duplicating placement logic the shared Dropdown also owns, with a documented algorithm disagreement (intersection-of-all-clippers vs nearest-clipper). One hygiene note: the scroll-listener search only considers `overflow-y: auto|scroll` ancestors while `clipBoxAround` also counts `hidden|clip` — a `hidden` scroller that actually scrolls programmatically would move the trigger without firing the re-measure.

## 2. What actually works (verified)
— verified by rg: two production consumers (ColorsTool.tsx, HdrTool.tsx) + video.css consumes the `data-list-escape`/`--video-dropdown-cap` seams.
— verified by baseline: 12 tests PASS (ui-test.txt:95) covering the pure decision and the DOM writes (per the module's k11 note that the seams exist to be jsdom-pinned).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/tools/video/useDropdownScrollportFlip.ts:282-283 vs 157
Code:
```
        const oy = window.getComputedStyle(node).overflowY;
        if (oy === "auto" || oy === "scroll") return node;
```
What is wrong: The scroll-reattach search matches only `auto|scroll`, but `clipBoxAround` (L157) treats `hidden` and `clip` as clippers. An ancestor with `overflow(-y): hidden` that is scrolled programmatically (scrollTop set by focus scrolling, scrollIntoView on a sibling) shifts the trigger geometry without firing this listener; the escaped fixed list then floats at stale coordinates until the next resize. Edge case in a dialog whose body uses `overflow-y: auto` (the actual tools-dialog case), so today's blast radius ~zero. Confidence: MEDIUM — reasoning, not reproduced (UNVERIFIED: no node_modules).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| resolveListPlacement | 98-136 | REAL | — | pure; below/above clip then viewport then cap; floor 96px |
| clipsY | 139-141 | REAL | — | auto/scroll/hidden/clip |
| clipBoxAround | 148-165 | REAL | — | intersection of ALL clipping ancestors + viewport; stops at body |
| clearPlacement | 172-182 | REAL | — | removes markers + 5 list style props |
| applyPlacement | 196-219 | REAL | — | escape: fixed + bottom:auto (defends against menu.css up-rule); capped: data-list-direction + cap var |
| useDropdownScrollportFlip | 228-299 | REAL | S4 | MutationObserver on dropdown childList; resize+scroll re-decide; cleanup correct; F-001 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| useDropdownScrollportFlip.test.ts (12, baseline PASS) | YES | pure decision table + DOM write/clear assertions against the extracted seams (would catch marker renames, wrong side mapping, cap floor) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the video mounts keep this hook for the ONE case the flip cannot solve" (HDR 6-row list) | useDropdownScrollportFlip.ts:10-14 | REAL-UNPROVEN (geometry claim about 1440×900 rest) | not reproduced here; visual-harness territory |
| "DELETE THIS MODULE when the primitive gains the escape" | useDropdownScrollportFlip.ts:51-52 | TRUE status | shared Dropdown has no viewport escape (menu.css audit: only clip flip) — stopgap justified |
| "clipBoxAround intersects EVERY clipping ancestor while the primitive's clippingBounds takes the nearest only — two algorithms that can disagree" | useDropdownScrollportFlip.ts:52-56 | TRUE (self-reported duplication) | documented wrinkle, honest |

## 7. Dead / duplicated / unreachable code
None dead. Duplication vs shared Dropdown placement is self-documented (consolidation pending).

## 8. Dependencies & flags
React hooks only; DOM APIs (MutationObserver, getComputedStyle, getBoundingClientRect). No flags.

## 9. Open questions for the next agent
- Dropdown.tsx (menu lane): confirm `clippingBounds` really takes the nearest clipper only, as claimed here.
