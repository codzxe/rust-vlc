# AUDIT: ui/src/components/menu/menu.css
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 551 / 60 rule blocks |
| File verdict | REAL-UNPROVEN (grammar live in 29+ screen files; every frame-pinned dimension unverified by any committed visual run of open-menu states) |
| Reachable from shipped entry point? | YES as web-UI source: `src/main.tsx → App.tsx → PlayerScreen.tsx → ControlBar.tsx` renders MenuBar; Tooltip.tsx:4 / Dropdown.tsx:8 / ContextMenu.tsx:6 / MenuBar.tsx:6 each `import "./menu.css"`; 29 non-test screen files consume Tooltip/ContextMenu/Dropdown (`rg -l` over src/screens). Same W15 caveat as all ui files: no GUI binary exists (W14; baseline #2 exit 101) and the UI mounts no `<canvas>` (W0). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -rho "lumen-(menubar|menu|context-menu|dropdown|tooltip)[a-z_-]*" src/components/menu/*.tsx \| sort -u` (two-way selector check → 2 dead-in-CSS hooks, F-004); `rg -n "DROPDOWN_LIST_MAX_HEIGHT" Dropdown.tsx` (=320 at :15, matches CSS :430 — cross-link intact); python3 WCAG luminance recompute (comment's 3.30/3.09/3.74/2.88 all confirmed exact); `rg -rn "menu.css" src/` (importers + 6 harness.test.tsx CSS-contract readers); `sed -n '140,170p' MenuBar.tsx`, `60,95p ContextMenu.tsx` (hook-only classnames confirmed); baseline #22/#23 (build OK, 1705 tests pass) |

## 1. One-paragraph verdict
The shared menu grammar sheet — 60 rules covering MenuBar, MenuPopup/MenuItem, ContextMenu, Dropdown/Select, Tooltip — is disciplined work: tokens-only coloring, every raw px marked "structural" with frame provenance, honest in-file documentation of two deliberate frame deviations (min-width 400 vs frame 375; hover #1F1F1F via --color-surface-selected), and a two-file grammar (CSS max-height 320px ↔ `DROPDOWN_LIST_MAX_HEIGHT = 320` at Dropdown.tsx:15) that actually matches. The two real defects: the accelerator ink token ships sub-AA text contrast by documented policy (3.09:1 dark / 2.88:1 light on selected rows — my recomputation confirms every number in the comment), and none of the ~25 frame-pinned dimensions for open menu states is verified by anything committed — the visual harness covers routes with closed triggers only, and the "vision harness" tests are env-gated HTML exporters whose only assertion is `innerHTML.length > 0`.

## 2. What actually works (verified)
— verified by `rg -n "DROPDOWN_LIST_MAX_HEIGHT" src/components/menu/Dropdown.tsx` → `const DROPDOWN_LIST_MAX_HEIGHT = 320;` (:15) matches menu.css:430 `max-height: 320px;` — the cross-linked "one grammar in two files" claim holds at HEAD.
— verified by python3 WCAG recompute → `#6b6b6b` on panel `#191919` = 3.30:1, on selected `#1f1f1f` = 3.09:1; light `#5f5f5f` on `#c6c6c6` = 3.74:1, on `#aeaeae` = 2.88:1 — the sheet's comment (:266-271) states exactly these values; no fabricated math.
— verified by two-way class grep → every one of the 60 rules' selectors is produced by menu/*.tsx (e.g. `[aria-checked="true"] > .lumen-menu__check::before` pairs with MenuItem radio rows; `data-direction="up"` pairs with Dropdown.tsx measurement, asserted in useDropdownScrollportFlip.test.ts:133).
— verified by baseline #22/#23 → the sheet compiles and all 7 menu test files pass (MenuBar/MenuPopup/MenuItem/ContextMenu/Dropdown/ProfilesFlyout/Tooltip) in `npm test` 1705-pass run.

## 3. Findings

### F-001 [S3]
Where: ui/src/components/menu/menu.css:281 (token use) and 253-280 (the ruling that pins it)
Code:
```
  color: var(--color-content-accelerator);
```
What is wrong: the accelerator (shortcut-hint) ink ships at #6b6b6b dark / #5f5f5f light — 3.09:1 on dark selected rows and **2.88:1** on light selected rows, against WCAG 1.4.3's 4.5:1 for text < 18.66px bold. The comment's defense ("accelerators are not the reading copy", :276-279) invents an exemption WCAG does not have: shortcut hints are normal text; the only carve-outs are incidental/logotype text. Every menu in the app renders these.
Why it matters: keyboard-shortcut text is the primary affordance for exactly the power users who open menus with the keyboard; in light mode on a selected row it is below 3:1 — below even the large-text bar.
What correct looks like: `--color-content-accelerator` repinned to ≥4.5:1 on both panel and selected surfaces (e.g. dark #8f8f8f-class ink, which measures 5.44:1 on panel — the label token already proves the headroom), or an explicit documented a11y-exception filing rather than an in-CSS ruling.
Confidence: HIGH — Verification: read + recomputed contrast (python3, values above); token values verified in tokens.css:36/:119.

### F-002 [S3]
Where: ui/src/components/menu/menu.css:88 and 124 (representative of ~25 frame-pinned structural values: 88, 111, 124, 136, 305, 416, 419, 453)
Code:
```
  min-width: 400px; /* 400px-class panel (task W26; `menu.svg` frame is 375) — structural */
```
What is wrong: the frame-fidelity program ("SVG is ground truth", :7-8) is unenforced and this very line documents a 25px deviation decided by "task W26" rather than any frame — while `right-click menu.svg` 265px (:305) and `Dropdown.svg` 44px/46px/2px (:416/:453) claim exactness. No committed run renders an open menu: e2e/visual baselines cover routed screens with closed triggers only (manifest.ts grid), the harness.test.tsx files that inline this CSS (equalizer/video/visualizer/dialogs/subtitles/playlist-options) only assert `innerHTML.length > 0` unless a human sets `W26C1_HARNESS_OUT` (equalizer/harness.test.tsx:44-52), and jsdom tests assert classnames, not geometry.
Why it matters: a 25px documented drift proves the pinning is editorial; with zero measurement, the next drift (like the "was 58px pre-w26.f07" row height at :124, or "was 22px" icon at :198) ships silently again.
What correct looks like: one Playwright dialog-state spec asserting computed min-width/row-height/option-pitch on an open Dropdown + MenuBar menu, or committed harness-output screenshots diffed in CI.
Confidence: HIGH — Verification: read (harness test bodies, e2e tree, manifest); baseline #23 cited for what tests actually run.

### F-003 [S4]
Where: ui/src/components/menu/menu.css:523-537
Code:
```
.lumen-tooltip__bubble {
  position: absolute;
  bottom: calc(100% + var(--space-grid));
  left: 50%;
```
What is wrong: the tooltip bubble is `white-space: nowrap` (:532) with no max-width and no viewport clamping — a long localized string (the i18n seam resolves arbitrary translations, Tooltip.tsx:8) overflows the window with no wrap or flip, unlike menus which got the w26.f07 flip/shift treatment (:32-33).
Why it matters: unbounded horizontal paint at trigger-centered 50% means CJK/German long labels can run off both edges; there is no escape hatch (no data-attribute, no width cap).
What correct looks like: `max-width: min(300px, calc(100vw - 16px))` + `white-space: normal`, or the MenuPopup clamp seam reused.
Confidence: HIGH — Verification: read; no clamp rule anywhere in the sheet (grep nowrap/max-width).

### F-004 [S4]
Where: ui/src/components/menu/menu.css (absence) vs MenuBar.tsx:157, ContextMenu.tsx:76, Dropdown.test.tsx:204
Code:
```
              className="lumen-menubar__popup"
```
What is wrong: three classnames are emitted/tested but have no rules in this sheet — `.lumen-menubar__popup` (MenuBar.tsx:157), bare `.lumen-context-menu` (ContextMenu.tsx:76), and `.lumen-dropdown--comfortable` (asserted by Dropdown.test.tsx:204). They are semantic hooks only; nothing in the sheet styles them, so the class inventory over-promises and a future maintainer greps this sheet for rules that do not exist.
Why it matters: dead naming surface; the comfortable/compact density split is real styling (compact rules at :135/:202/:231/:342/:463) so "--comfortable" implying a mirrored rule is actively misleading.
What correct looks like: drop the hooks or add a comment block listing "hook-only classes consumed by tests/hosts" at the sheet top.
Confidence: HIGH — Verification: two-way grep (Commands); sed reads of both TSX sites.

## 4. Per-function table (selector groups)
| Selector(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-menubar` | 40-44 | REAL | — | flex row |
| `__item-wrapper` | 46-48 | REAL | — | popup anchor |
| `__trigger` (+hover/focus/expanded) | 50-78 | REAL | — | 32px structural, focus-visible present |
| `.lumen-menu` | 81-95 | REAL | — | z10 (below dialog scrim z20 — layering verified vs surface.css:35); 400px documented 25px frame drift → F-002 |
| `--compact` / `--submenu` | 97-105 | REAL | — | compact sizing + submenu offsets |
| `--comfortable--submenu` (compound) | 110-112 | REAL | — | 265px flyout; comfortable-alone unstyled by design |
| `__host` | 114-116 | REAL | — | nested submenu anchor |
| `__item` (+compact) | 119-140 | REAL | — | 48px/40px rows; 58px→48px history documented |
| `__item:hover` | 148-150 | REAL | — | #1F1F1F via selected token — token-value claim verified (#1f1f1f in tokens.css) |
| `__item--selected` (+check::before) | 156-167 | REAL | — | ✓ glyph U+2713; graphical, not i18n — correct |
| `__item:focus` / `:focus-visible` | 172-179 | REAL | — | roving cursor = fill; ring on visible focus |
| `__item[aria-disabled]` | 181-184 | REAL | — | opacity + pointer-events:none; "roving focus still reaches it" — inert pattern correct |
| `__icon` (+svg sizing ×2) | 189-205 | REAL | — | 20/16px density-owned sizing; source-order override documented and correct (equal specificity, later wins) |
| `__item--icon-accent …` | 215-218 | REAL | — | deliberately neutral now (two-tone moved to icon.css) — the class is a vestigial hook (see F-004 note) |
| `__check` (+compact, aria-checked::before) | 221-238 | REAL | — | alignment slot; ✓ on checked |
| `__label` | 240-245 | REAL | — | ellipsis |
| `__accelerator` | 247-282 | REAL | S3 | F-001 sub-AA ink, ruling quoted |
| `__submenu-arrow::before` | 286-289 | REAL | — | › chevron |
| `__separator` | 294-298 | REAL | — | 1px on-boundary (frame formula) |
| `.lumen-context-menu__popup` | 301-306 | REAL | — | fixed; inline left/top from ContextMenu.tsx:92 overrides |
| `.lumen-dropdown` (+compact/invalid triggers) | 309-344 | REAL | — | invalid uses --color-record ring |
| `__trigger > span.__value/__arrow` | 352-361 | REAL | — | flex fix documented (v2 audit caught chevron eating half the width) |
| `__trigger` states | 363-379 | REAL | — | hover/expanded/disabled |
| `--invalid __trigger` | 381-384 | REAL | — | 2px record ring |
| `__value` | 386-392 | REAL | — | ellipsis |
| `__arrow::before` (+expanded flip) | 397-406 | REAL | — | › rotated 90°, flips -90° open |
| `__list` (+data-direction="up") | 409-440 | REAL | — | 2px inset, 2px gap, 320px scrollport = Dropdown.tsx:15 constant (verified); overscroll-behavior contain |
| `__option` (+compact) | 448-465 | REAL | — | 44px min-height |
| `__option:hover/:focus/:focus-visible/[aria-selected]` | 467-486 | REAL | — | quiet labels on state; #1F1F1F fill |
| `[aria-selected]::after` check | 490-497 | REAL | — | orange ✓, semibold |
| `[aria-disabled]` | 499-502 | REAL | — | inert pattern |
| `__option-icon` / `__option-label` | 504-515 | REAL | — | slot + ellipsis |
| `.lumen-tooltip` / `__bubble` (+bottom) | 518-542 | REAL | S4 | F-003 no wrap/clamp |
| `@media (prefers-reduced-motion)` | 544-551 | REAL | — | kills the 4 transitions — correct |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| MenuBar/MenuPopup/MenuItem/ContextMenu/Dropdown/ProfilesFlyout/Tooltip .test.tsx (7 files, in baseline 1705-pass) | Only for DOM/behavior breaks | jsdom: no CSS applied; they assert roles, classnames, aria state, keyboard flows — good behavioral coverage, zero style coverage |
| Dropdown.test.tsx:204 `toHaveClass("lumen-dropdown--comfortable")` | NO | asserts a class that has no CSS rule anywhere (F-004) — the test would pass if the entire comfortable grammar were deleted |
| 6 × harness.test.tsx CSS inliners (equalizer/video/visualizer/dialogs/subtitles/playlist-options) | NO | env-gated export; only assertion is non-empty HTML; CSS file list is hand-maintained and would silently miss new sheets |
| e2e/visual route baselines | Only closed-trigger styles | open menus/popups/context menus/tooltips are in-panel states, excluded by manifest.ts:12-14 |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "SVG is ground truth where the spec conflicts" (menu grammar) | menu.css:7-8 header | SHALLOW | 400px min-width vs 375px frame (:88) is a documented hand-waved deviation; no enforcement (F-002) |
| "Colors are semantic tokens ONLY (doc 08 §3.2)" | menu.css:34-36 | REAL | verified: zero raw hexes in declarations; only raw px marked structural |
| "--color-surface-selected carries exactly #1f1f1f" | menu.css:18-19 | REAL | tokens.css :root `--color-surface-selected: #1f1f1f` |
| "keep in sync with DROPDOWN_LIST_MAX_HEIGHT (Dropdown.tsx)" | menu.css:427-429 | REAL | Dropdown.tsx:15 = 320 = CSS :430 |

## 7. Dead / duplicated / unreachable code
Dead-in-CSS hooks: `.lumen-menubar__popup`, bare `.lumen-context-menu`, `.lumen-dropdown--comfortable` (F-004). `.lumen-menu__item--icon-accent` now resolves to the same value as the base rule (deliberately retained as a hook — documented at :214). No duplication.

## 8. Dependencies & flags
Consumes 18 semantic tokens, all defined in generated tokens.css (incl. `--color-content-accelerator` dark/light — verified). `--space-item-pitch` (48px) doubles as row height and trigger height — a structural coupling to tokens.css values, correct today (48px in tokens).

## 9. Open questions for the next agent
- Who owns the promised-but-missing `--color-accelerator` retune lane ("A future retune diverges via the dedicated token", :279-280)? If the tokens crate wave owns tokens.json, F-001 belongs there too.
- `useDropdownScrollportFlip.ts` (video tool) duplicates Dropdown.tsx's flip logic — is there a reason the shared Dropdown cannot host it? (Duplication flagged for the video-tools wave if unowned.)
