# AUDIT: ui/src/screens/playlist/options/options.css
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 397 / 22 selector groups (+1 media block) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as UI code — imported by PlaylistOptionsMenu.tsx:23 (`import "./options.css"`), the options layer of the /playlist screen (browse views, sorting/view menus, media list); builds + tests green (baseline #22/#23 incl. OptionsMenus/PlaylistOptionsMenu/MediaList/ThumbnailGrid tests). Desktop host binary cannot compile at HEAD (W14-A01 F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | per-file `rg -o 'lumen-plo[a-z_-]*(__[a-z-]*)?' options/*.tsx` (full selector↔consumer closure both directions) · `rg -n "menu--below|item--selected|lumen-plo--sorting|lumen-plo--view" options/` (4 marker classes have no CSS rule — JS/test hooks) · per-token `rg -c "\-\-$t:" ui/src/tokens/generated/tokens.css` (all 21 custom properties defined) · `rg -n "data-theme" ui/src/theme/ThemeProvider.tsx` (:4 — attribute name matches the `:root:not([data-theme="light"])` dark-scope at :309) · `rg -n "lumen-vrow" options/MediaList.tsx` + list primitives (row class the :224 hover-reset targets) · baseline: ui-vitest.log.txt (1705 passed), ui-build.log.txt; lint:css NOT in baseline (standing UNVERIFIED, same as equalizer.css) |

## 1. One-paragraph verdict
A clean, frame-documented stylesheet for the playlist options layer (sorting/view menus, thumbnail grid, media rows) with full selector↔consumer closure in both directions: every class in this sheet is applied by a real component (PlaylistOptionsMenu, ThumbnailGrid, MediaList, PlaylistOptionsToolbar), and the handful of TSX classes with no rule (`--below`, `--selected`, `--sorting`, `--view`) are deliberate JS/test markers — the flip/align logic asserts them in PlaylistOptionsMenu.test.tsx:183-292. The one hex (#303030 tile fill, dark theme only) is scoped under `:root:not([data-theme="light"])` — attribute name verified against ThemeProvider — and carries the documented stylelint-disable + token-lane handoff. The interaction geometry is reasoned through: the stretched play target (absolute, z-index 0) overlays the row for real clicks while the kebab (z-index 1) stays reachable, `flex:none` rows prevent the documented h17 flex-crush, and the scrollport cap arrives inline from open-time measurement. The single defect is a comment/geometry mismatch on the duration footer. Everything here styles surfaces whose honest-interaction contracts (inert cards for local scope, play dispatch for engine scope) are test-pinned.

## 2. What actually works (verified)
— Selector closure: all 22 selector groups consumed; consumers' classes map back (verified by class-name extraction from every options/*.tsx).
— Token hygiene: 21 distinct custom properties, all defined in tokens.css (per-token rg); the only raw colors are the one documented hex exception (:310-312).
— Dark-scope correctness: `:root:not([data-theme="light"])` matches ThemeProvider's documented `data-theme` attribute (:4) — the light theme keeps the shared token, dark pins the frame value.
— Hover/reset interplay: `.lumen-plo-media .lumen-vrow:hover { background: transparent }` correctly cancels the VirtualList row fill the comment says the frame omits (row class verified against the list primitives).
— Honest-interaction styles are consumed by tested behavior: `--inert` card/row variants (ThumbnailGrid.test.tsx:76 "no play seam → cards are honest presentation, never dead buttons"; MediaList inert tests) and the `__hint` suffix (OptionsMenus.test.tsx) — green in baseline ui-vitest.log.txt.
— The `.lumen-plo-mrow-wrap { display: contents }` wrapper keeps the absolute row's containing block the VirtualList window (documented LibraryRow precedent) — the right-click + kebab menu subtree mounts without breaking tile x20 geometry.

## 3. Findings

### F-001 [S4]
Where: ui/src/screens/playlist/options/options.css:208-218 (comment at :209 vs geometry at :156-158/:198)
Code:
```
/* Quiet meta footer (37px): the centered h:mm:ss duration, #8F8F8F. */
.lumen-plo-card__duration {
  flex: 1;
```
```
  height: 189px;
  padding: 10px; /* frame artwork inset */
```
```
  height: 142px; /* frame: 252×142 artwork */
```
What is wrong: the comment claims a 37px duration footer, but the geometry yields 27px: card 189 − padding 2×10 − art 142 = 27. The frame's 37px (189 − 142 − 10, i.e. art inset at top only) implies the footer runs to the card's bottom edge with no bottom padding; the uniform `padding: 10px` shaves it to 27px. Either the padding or the comment is frame-wrong; the centered ≈15px text fits either way, so the visible effect is a slightly raised duration line.
Why it matters: frame-measured provenance is this sheet's whole discipline — a self-inconsistent measurement undermines the citation chain for future edits (someone "fixing" toward 37px would double-subtract the padding).
What correct looks like: re-measure thumbnail.svg and either set the comment to 27px or move the footer's 10px to the art's top inset only (`padding: 10px 10px 0`).
Confidence: HIGH — Verification: read + arithmetic from the file's own cited numbers.

## 4. Per-selector table
| Selector group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| .lumen-plo-toolbar / .lumen-plo | 22-32 | REAL | — | positioning context for the popup |
| .lumen-plo__menu (+--end/--above) | 46-65 | REAL | — | 265px frame width; borderless r10 documented; --below needs no rule (base position) |
| .lumen-plo__item (+hover/focus/disabled) | 74-96 | REAL | — | flex:none anti-crush fix (h17); aria-disabled stays focusable (inert, not hidden) |
| __icon svg sizing | 101-104 | REAL | — | compound selector beats shared 22px at equal specificity (documented) |
| __check (+::before glyph) | 108-131 | REAL | — | ✓ is graphical (content "\2713"), not i18n — correct call |
| __hint | 122-127 | REAL | — | honest-degradation suffix, test-pinned |
| .lumen-plo-grid (+__cell) | 137-149 | REAL | — | auto-fill keeps 272px cards frame-exact |
| .lumen-plo-card (+hover/focus/--inert) | 153-186 | REAL | — | inert cards honest (no cursor promise) |
| __art (+glyph opacity) | 194-206 | REAL | — | full-surface-deep well, only glyph muted (g18 P2-2 fix) |
| __duration | 209-218 | REAL | S4 | F-001 comment/geometry mismatch |
| .lumen-plo-media (+vrow hover reset) | 224-231 | REAL | — | cancels the shared row fill per frame |
| .lumen-plo-mrow (+states) | 234-294 | REAL | — | --watched/--current/--inert; current = panel-grammar extension (documented frame extension) |
| __play | 253-266 | REAL | — | stretched target, z-layering documented; blocks title selection (tradeoff) |
| __tile (+dark hex scope) | 300-312 | REAL | — | the single documented hex exception, correctly dark-scoped |
| __progress | 316-323 | REAL | — | 28×3 frame bar, partial-progress capable |
| __title | 325-331 | REAL | — | ellipsis, min-width:0 correct |
| __menu (+__dots) | 337-382 | REAL | — | hover/focus-revealed kebab above the play target (z-index 1) |
| __mrow-wrap display:contents | 389-391 | REAL | — | keeps absolute row's containing block the vrow window |
| @media prefers-reduced-motion | 393-397 | REAL | — | card transition disabled |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| PlaylistOptionsMenu.test.tsx:183-292 (flip/align classes --end/--above/--below) | YES (class-level) | asserts the positioning modifiers the TSX computes; visual flip correctness itself is harness/visual territory |
| ThumbnailGrid.test.tsx (grid anatomy, duration format, play dispatch, inert mode) | YES | asserts cards, titles, playlist.play-item dispatch on click, and the no-cursor inert contract |
| MediaList.test.tsx (row states incl. --current/--watched/--inert, kebab) | YES | class + interaction assertions |
| OptionsMenus.test.tsx (__hint suffix) | YES | asserts the honest-degradation copy renders |
All green: baseline ui-vitest.log.txt (1705 passed / 1 skipped).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Colors: semantic tokens ONLY … Raw px appears solely for frame-measured structural dimensions" | file header :12-14 | REAL | verified per-selector; one documented hex exception |
| "rows that pair a leading icon WITH the check (View Options.svg row 2 — MenuItem can't, hence PlaylistOptionsMenu)" | header :9-10 | REAL | PlaylistOptionsMenu composes both (icon :312 block + __check) — cross-verified in TSX |
| "media.svg tile #303030 → pinned below (dark port, g18 P3-3 fix)" | header :16 | REAL | :309-312 rule present, dark-scoped, token-lane flagged |

## 7. Dead / duplicated / unreachable code
- Marker classes without rules (intentional, but undocumented in the CSS): `lumen-plo__menu--below`, `lumen-plo__item--selected`, `lumen-plo--sorting`, `lumen-plo--view` — the CSS never states "marker only"; a maintainer adding a `--below` rule would silently fight the base position. One comment line in the sheet would close it.
- No orphan selectors.

## 8. Dependencies & flags
- Shared menu.css grammar (lumen-menu*, lumen-vrow from list primitives), tokens.css (21 properties verified), ThemeProvider's data-theme attribute. No feature flags. stylelint compliance UNVERIFIED (lint:css not in baseline — same standing gap as equalizer.css; REQUEST-ORCH-RUN already logged).

## 9. Open questions for the next agent
- ThumbnailGrid.tsx and SortingOptionsMenu.tsx are MY next files — the play dispatch (playlist.play-item) reality gets audited there, not here.
- Whoever owns PlaylistOptionsMenu.tsx: the inline max-height measurement contract (options.css :38-40) — verify the TSX actually sets it (I only read the class list).
