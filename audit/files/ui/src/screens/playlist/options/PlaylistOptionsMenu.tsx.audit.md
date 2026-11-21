# AUDIT: ui/src/screens/playlist/options/PlaylistOptionsMenu.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 335 / 10 (OptionsMenuEntry type, VIEWPORT_MARGIN_PX/MENU_CHROME_PX/fallbacks, readTokenPx, PlaylistOptionsMenuProps, PlaylistOptionsMenu [outside-dismiss effect, enabled, focusIndex, move, openMenu, closeAndRestore, activate, open-focus effect, measure + useLayoutEffect, handleKeyDown, render]) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: App.tsx:319 `/playlist` → PlaylistScreen.tsx:53,376 mounts PlaylistOptionsToolbar inside PlaylistOptionsProvider → SortingOptionsMenu.tsx:14 / ViewOptionsMenu.tsx → PlaylistOptionsMenu). Baseline vitest executed its 19 tests (136/136 files, 1705 passed). Product caveat: no desktop binary (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -ln 'PlaylistOptionsMenu'` (consumers: Sorting/ViewOptionsMenu, index barrel, tests); Read SortingOptionsMenu.tsx (real consumer: SORT_KEYS → setSortKey via PlaylistOptionsProvider, meta-hint composition); `rg -n 'PlaylistOptionsToolbar' src/screens/playlist` (PlaylistScreen.tsx:53 — the product mount); `rg -n 'it\(' PlaylistOptionsMenu.test.tsx` (19 behavioral tests incl. flip/cap measurement, live-token reads, resize re-measure); `rg -n 'lumen-plo__' options.css` (all class hooks exist: --end/--above/--check/--hint at 55-129); Read MenuItem.tsx:60-80 (verified the "icon XOR check" justification for not using the shared MenuPopup — TRUE: icon renders only `&& !checkable`, check renders when `checkable \|\| icon === undefined`); static trace of move()/measure() math. |

## 1. One-paragraph verdict
A clean, genuinely accessible generic options-menu component: full APG keyboard model (arrows skipping disabled, Home/End, Enter/Space, Escape-with-focus-restore, Tab trap, roving tabindex, menuitemradio/aria-checked), correct outside-dismiss containment (no portal — rootRef contains the popup), a real flip/scrollport measurement pass that reads live CSS tokens instead of hardcoded pitches and re-runs on resize, and a 19-test behavioral suite that would fail on mutation of every one of those behaviors (executed in baseline). Its one defect is an edge-case in `move()`: with no active row (all entries disabled), ArrowUp/Shift+Tab lands on the second-to-last row rather than the last, and a fully-disabled menu still focuses a disabled row after a full wrap — degenerate states no current consumer produces. The header's justification for not using the shared MenuPopup (icon XOR check) was verified TRUE against MenuItem.tsx's render logic.

## 2. What actually works (verified)
— verified by PlaylistOptionsMenu.test.tsx:44-283 (19 tests: menu semantics, single-check selection, disabled never selects, outside dismiss, keyboard cycle/Home/End/Enter/Space/Escape/Tab-trap, trigger toggle, align/placement classes, flip-above, scrollport cap, LIVE token values stubbed via getComputedStyle, resize re-measure, zero-height guard) + baseline ui-vitest.log.txt (136/136, 1705 passed).
— verified by Read MenuItem.tsx:69-74 → the shared MenuItem cannot render a leading icon AND a check on one row (icon gated on `!checkable`) — the file's "why not MenuPopup" header claim (8-11) is accurate, not an excuse to duplicate.
— verified by `rg 'PlaylistOptionsToolbar' PlaylistScreen.tsx` → :53 import + :376 mount — the component is product-reachable, not a harness-only widget.

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/playlist/options/PlaylistOptionsMenu.tsx:139-147
Code:
```ts
  const move = (delta: number): void => {
    if (entries.length === 0) return;
    let index = activeIndex;
    for (let step = 0; step < entries.length; step += 1) {
      index = (index + delta + entries.length) % entries.length;
      if (enabled(index)) break;
    }
    focusIndex(index);
  };
```
What is wrong: Two degenerate-state defects: (a) when `activeIndex === -1` (which `openMenu` produces when NO entry is enabled — `findIndex` returns -1 at :153), `move(-1)` computes `(-1 - 1 + n) % n = n - 2` — Shift+Tab/ArrowUp from "no active row" lands on the second-to-LAST row instead of the last; (b) when every entry is disabled, the wrap loop never `break`s, ends where it started, and `focusIndex` focuses a disabled row (roving tabindex moves onto an aria-disabled item). Current consumers never produce these states (SORT_KEYS and view modes are all enabled), so the defect is latent.
Why it matters: The component is a exported generic (`PlaylistOptionsMenuProps<K>`) built for reuse; the next consumer with conditional rows inherits a subtly wrong keyboard model, and nothing in the 19 tests pins the all-disabled shape.
What correct looks like: Seed `index` from the selected/first-enabled row when `activeIndex < 0` (mirroring openMenu's rule), and no-op (keep focus on the trigger/first row) when no enabled entry exists.
Confidence: HIGH — Verification: static trace of the arithmetic; consumer entry lists checked (all enabled today).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `OptionsMenuEntry` type | 26-37 | REAL | — | Generic K; hint field documented as honest-degradation marker. |
| layout constants + fallbacks | 43-55 | REAL | — | Frame-measured fallbacks for jsdom's empty computed tokens (documented, tests stub the branches). |
| `readTokenPx` | 59-63 | REAL | — | px-suffix + isFinite guard; falls back cleanly. |
| `PlaylistOptionsMenuProps` | 65-86 | REAL | — | Full generic API; defaults documented. |
| outside-dismiss effect | 118-127 | REAL | — | pointerdown + contains via rootRef; popup renders inside rootRef (no portal) — containment correct. |
| `enabled`/`focusIndex` | 129-137 | REAL | — | scrollIntoView optional-chained (jsdom). |
| `move` | 139-147 | REAL | S4 | F-001 degenerate-state arithmetic. |
| `openMenu`/`closeAndRestore`/`activate` | 149-168 | REAL | — | Selected-else-first-enabled focus; restore-to-trigger; disabled never activates. |
| open-focus effect | 172-177 | REAL | — | Open-transition only; deps eslint-disable is sound (later moves focus themselves). |
| `measure` + useLayoutEffect | 188-222 | REAL | — | Flip-then-cap math traced correct (natural vs below/above with gap+margin; cap floor 160); live token reads; resize listener cleaned up; deps [open, entries.length, placement] cover the closure reads. |
| `handleKeyDown` | 224-266 | REAL | — | Full APG set; handled() also on inactive Enter (prevents page scroll); Tab trap documented (card 4.B3). |
| render | 268-334 | REAL | — | role=menu/menuitemradio, aria-checked/aria-disabled, roving tabIndex, check/icon coexistence (the whole point of the custom component), hint span, trigger haspopup/expanded; maxHeight applied only when capped. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| :112 arrows skip disabled | YES | Asserts focused row id after keydown. |
| :210/:221 flip / scrollport cap | YES | Stubbed getComputedStyle + innerHeight drive the branches; asserts direction class + maxHeight style. |
| :244 live token values | YES | Asserts the measurement used the stubbed token (48px pitch), not the TS fallback. |
| :264 resize re-measure | YES | Fires resize, asserts re-capped height. |
| :148 Tab trap | YES | Asserts wrap + no focus escape. |
| Gap: no all-disabled-entries test | — | F-001's defect is unpinned. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "MenuItem renders either an icon or a check, never both — logged as a fix-needed for the menu lane" | PlaylistOptionsMenu.tsx:8-11 | REAL | MenuItem.tsx:69-74: icon gated on `!checkable`; check on `checkable \|\| icon === undefined`. |
| "the keyboard model below mirrors MenuPopup's APG implementation exactly" | PlaylistOptionsMenu.tsx:12-15 | REAL (traced) | Arrows/Home/End/Enter/Space/Escape/Tab/roving all present and tested. |
| "the math keeps the frame values there (tests stub getComputedStyle to drive the branches)" | PlaylistOptionsMenu.tsx:50-53 | REAL | Test :244 stubs computed style; fallbacks at 54-55. |

## 7. Dead / duplicated / unreachable code
Dead: none. Duplicated: the outside-dismiss effect and focus-restore pattern are copies of MenuPopup/Dropdown/PanelPopover's caller halves (4th copy in the codebase — a shared hook would serve; noted, consistent with the menu-lane's own "Known gaps" sanctioning app-layer overrides). Unreachable: none.

## 8. Dependencies & flags
IconButton/Icon (components), shared menu.css classes + options.css (all hooks verified present), no transport (pure UI — selection semantics owned by PlaylistOptionsProvider). No feature flags. Generic <K extends string> keeps i18n keys type-safe through the lane resolver.

## 9. Open questions for the next agent
- The menu-lane "fix-needed" (MenuItem icon+check coexistence, header line 11): if it lands, can this component collapse back onto MenuPopup, deleting ~200 lines of duplicated keyboard model?
- PlaylistOptionsProvider/metaSeamWired: who owns the resolveMeta seam the sorting hints advertise as missing? (sorting.ts:86-88 claims the hint is "real, not aspirational" — verify in the Provider's audit.)
