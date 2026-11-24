# AUDIT: ui/src/screens/playlist/options/PlaylistOptionsMenu.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 335 / 10 (component + 9 inner fns) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: App.tsx:319 /playlist -> PlaylistScreen.tsx -> PlaylistOptionsToolbar -> SortingOptionsMenu.tsx:17 / ViewOptionsMenu -> PlaylistOptionsMenu |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg consumer/typeahead checks (exit 0); baseline ui-test.txt cited |

## 1. One-paragraph verdict
A solid generic anchored single-select menu: full APG keyboard model (ArrowUp/Down cycling that skips disabled rows, Home/End, Tab focus trap, Escape with focus restore, Enter/Space activation), roving tabindex, role=menu/menuitemradio with aria-checked, outside-pointer dismiss, and a real flip/cap measurement that reads live spacing tokens off the trigger and re-measures on window resize. It is presentational — the data and the onSelect sink come from callers; the sorting caller (SortingOptionsMenu.tsx:26-29) honestly marks metadata-dependent rows with a "Needs media metadata" hint while the meta seam is unwired, so the no-op sort rows are visible as no-ops. Two S4 hygiene notes only.

## 2. What actually works (verified)
— Consumer chain verified by rg: PlaylistScreen -> PlaylistOptionsToolbar -> Sorting/ViewOptionsMenu -> this component.
— Honest-degradation hint consumed: SortingOptionsMenu.tsx:26-29 passes `hint: sortUsesMeta(key) && !metaSeamWired ? t("playlist.options.sort.needs-meta") : undefined` — no-op rows say so (i18n key w26.k15.ts:15).
— Baseline vitest: options suite passes (audit/runs/baseline/ui-test.txt includes src/screens/playlist/options/ tests).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/playlist/options/PlaylistOptionsMenu.tsx:139-147
Code:
```
  const move = (delta: number): void => {
    if (entries.length === 0) return;
    let index = activeIndex;
    for (let step = 0; step < entries.length; step += 1) {
      index = (index + delta + entries.length) % entries.length;
      if (enabled(index)) break;
    }
    focusIndex(index);
```
What is wrong: when EVERY entry is disabled the loop never breaks and focusIndex lands on a disabled row (which then holds tabIndex 0 via L309). Unreachable with current callers (the 16 sort rows always include enabled ones) but the component is exported generic. What correct looks like: track whether an enabled row was found; no-op otherwise. Confidence: HIGH — Verification: read (mutation reasoning; not executed).
F-002 [S4]
Where: ui/src/screens/playlist/options/PlaylistOptionsMenu.tsx:276-296
What is wrong: trigger carries `aria-haspopup="menu"` + `aria-expanded` but no `aria-controls` id linkage to the popup (the popup div has no id); no printable-character type-ahead (verified the shared MenuPopup has neither — rg "typeahead" in MenuPopup.tsx: no match — so the header's "mirrors MenuPopup's APG implementation exactly" claim is consistent, but both omit APG's type-ahead). Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| readTokenPx | 59-63 | REAL | - | live token read w/ frame fallback; jsdom-safe |
| PlaylistOptionsMenu | 88-335 | REAL | - | full composition |
| outside-dismiss effect | 118-127 | REAL | - | cleanup correct |
| enabled / focusIndex | 129-137 | REAL | - | scrollIntoView optional-chained for jsdom |
| move | 139-147 | REAL | S4 | all-disabled edge (F-001) |
| openMenu | 149-156 | REAL | - | selected-else-first-enabled focus |
| closeAndRestore | 158-161 | REAL | - | focus restore to trigger |
| activate | 163-168 | REAL | - | disabled-guarded |
| focus-on-open effect | 172-177 | REAL | - | open-transition only |
| measure + resize effect | 188-222 | REAL | - | flip/cap math; token-driven; resize-aware |
| handleKeyDown | 224-266 | REAL | S4 | no type-ahead (F-002) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| options tests (baseline pass) | YES | keyboard model, flip/cap math (getComputedStyle stubs drive branches), disabled skipping are asserted (per test-file inventory in options/ dir) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The keyboard model below mirrors MenuPopup's APG implementation exactly" | L12-13 | TRUE | rg: MenuPopup has the same key set, no type-ahead in either |
| hint = "honest-degradation marker ... on rows that are stable no-ops until the seam is wired" | L33-36 | TRUE | consumed at SortingOptionsMenu.tsx:26-29 |

## 7. Dead / duplicated / unreachable code
None. (Divergence from shared MenuPopup is justified in-file: leading icon + trailing check on the same row, which MenuItem cannot render.)

## 8. Dependencies & flags
No gates. Uses menu.css + options.css + IconButton/Icon primitives.

## 9. Open questions for the next agent
- PlaylistOptionsProvider.metaSeamWired: what would ever set it true (no engine resolveMeta command exists in the catalog)? If nothing can, the meta-dependent sort rows are permanently hinted no-ops.
