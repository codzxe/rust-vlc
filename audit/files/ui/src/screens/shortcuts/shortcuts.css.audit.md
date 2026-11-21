# AUDIT: ui/src/screens/shortcuts/shortcuts.css
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / functions | 476 / 34 selector rules (+1 media block) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (web app build): `ui/src/main.tsx` → App.tsx lazy route (App.tsx:62-63) → `/keyboard-shortcuts` (route registered; title row App.tsx:225) → `ShortcutsScreen.tsx:402` `<Dialog … className="shortcuts-dialog">` imports this sheet (baseline #22 `npm run build` OK; #23 vitest 1705 passed incl. ShortcutsScreen.test.tsx). No GUI binary hosts it (W14-A01: tauri host cannot compile). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `Read` shortcuts.css 1-476 (complete); `rg -o "shortcuts__[a-z-]+" ShortcutsScreen.tsx | sort -u` (26 classes — every selector in the sheet is used except none dead; conversely `.shortcuts__group` in TSX is an unstyled semantic hook, fine); `for t in …` loop `rg -c -- "--$t:" ../themes/generated/tokens.css` (all 20 referenced custom properties exist in the committed generated tokens — zero dangling vars); `rg -n "keyboard-shortcuts|ShortcutsScreen" App.tsx` (route + lazy import verified); `rg -n "shortcuts__edit" -B6 -A4 ShortcutsScreen.tsx` (idle pencil is `aria-hidden` span, active is a real `<Link>`); `rg -n "describe|it(" ShortcutsScreen.test.tsx` (15 behavioral tests); `sed -n` on quoted ranges. |

## 1. One-paragraph verdict
A clean, frame-faithful stylesheet: every color goes through semantic tokens or `color-mix` over tokens (the "Semantic tokens only" contract at line 25 holds — zero raw hex), every geometry constant carries a `frame:` measurement comment, the two light-mode hazards the file itself identifies (near-invisible keycaps on light surface, 1.75:1 disabled-Save ink) are fixed in-sheet with documented AA math, and the decorative idle pencil is properly inert (`aria-hidden` + `pointer-events: none`). Only hygiene debt: a redundant `width`+`max-width` pair, and a Cancel-button recipe the comment itself admits is copy-pasted from network-stream.css. No behavioral defect found; nothing here is wired to a backend (nor should it be — it is presentation).

## 2. What actually works (verified)
— Verified by token existence check: all 20 custom properties referenced (`--color-surface-deep`, `--blur-backdrop`, `--radius-window`, `--radius-control`, `--radius-inset`, `--font-size-sidebar/label/body`, `--motion-duration-fast`, `--opacity-disabled`, `--color-accent-tinted`, `--color-accent`, `--color-surface*`, `--color-content-*`, `--color-stroke-border`, `--font-weight-regular/medium`, `--font-family-sans`) exist in `themes/generated/tokens.css` — no dangling `var()`.
— Verified by `rg` class cross-check: all 26 `shortcuts__*` classes used in ShortcutsScreen.tsx; every selector in this sheet targets a used class (the sheet has zero dead rules).
— Verified by read: a11y surfaces are real — `.shortcuts__search input:focus-visible` draws the computed outline (171-174), `.shortcuts__close` is a 28×28 hit target with `:focus-visible` ring (95-119), the disabled Save keeps its labeled tint rather than dropping text (429-434).
— Verified by baseline #23 (ui-vitest.log.txt, 1705 passed): ShortcutsScreen.test.tsx's 15 cases (search/filter behavior, scrim/✕/Escape dismissal, edit-pencil links) run against this composition.

## 3. Findings

### F-001 [S4]
Where: ui/src/screens/shortcuts/shortcuts.css:33-36
Code:
```css
  width: min(1048px, 92vw);
  max-width: min(1048px, 92vw);
  height: min(800px, 92vh);
  max-height: min(800px, 92vh);
```
What is wrong: `width` and `max-width` (and `height`/`max-height`) carry identical `min()` values — the max-* pair can never further constrain anything, so four declarations do the work of two.
Why it matters: pure duplication; a future edit that changes one member of a pair and not the other creates a silently dead declaration.
What correct looks like: keep `width`/`height` alone (the Dialog primitive's own maxima, if any, already apply), or document why the belt-and-suspenders pair exists (e.g. guarding against a shared-sheet override).
Confidence: HIGH — Verification: read.

### F-002 [S4]
Where: ui/src/screens/shortcuts/shortcuts.css:416-424
Code:
```css
.shortcuts__cancel.lumen-button {
  border-color: transparent;
  background: color-mix(in srgb, var(--color-surface-raised) 22%, var(--color-surface-panel));
}
```
What is wrong: the flat-Cancel recipe is duplicated verbatim in network-stream.css ("Same recipe as the netstream dialog's Cancel (network-stream.css)" — the file's own comment at 420). Two sheets independently maintain the same color-mix magic numbers (22%, transparent border).
Why it matters: the comment admits the copy-paste; when the recipe is retuned again (it already was once, 30%→22%), someone must remember both sites.
What correct looks like: a shared `.lumen-button--flat` modifier (or a `--surface-cancel` token) consumed by both dialogs.
Confidence: HIGH — Verification: read (network-stream.css is my file #9 — the duplicate rule was confirmed there during its audit).

## 4. Per-function (selector) table
| Selector | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-dialog.shortcuts-dialog` | 30-49 | REAL | F-001 | Frame window geometry + panel@95% fill + backdrop blur; shared Dialog provides scrim/trap. |
| `.shortcuts` | 52-63 | REAL | — | 96/1fr/52 grid; token-typed color/font. |
| `.shortcuts__topbar` | 69-76 | REAL | — | 96px strip, 9px bottom pad per frame. |
| `.shortcuts__titlerow` / `__title` | 78-92 | REAL | — | 50px row; secondary ink #8F8F8F→content-secondary. |
| `.shortcuts__close` (+hover/focus) | 95-119 | REAL | — | 28×28 target; hover lift; focus ring. |
| `.shortcuts__filters` | 123-129 | REAL | — | 10px gap, 9px insets — the frame arithmetic (820+10+200=1030) documented and correct. |
| `.shortcuts__search` (+focus-within) | 133-149 | REAL | — | 34px pill; accent ring on focus-within. |
| `.shortcuts__search-icon` | 151-154 | REAL | — | Trivial. |
| `.shortcuts__search input` (+focus-visible/::placeholder) | 156-178 | REAL | — | Computed-outline a11y requirement; placeholder on stroke-border. |
| `:root[data-theme="light"] … ::placeholder` | 183-185 | REAL | — | Light re-pin, documented (#d4d4d4 pill would swallow #d9d9d9). |
| `.shortcuts__filter.lumen-dropdown` (trigger/hover/list/option/selected/value) | 190-231 | REAL | — | Dropdown grammar override; 44px/2px pitch per Dropdown.svg; selected quiet ink per option.png. |
| `.shortcuts__body` | 237-241 | REAL | — | Scroll region; 31px bottom clearance above footer. |
| `.shortcuts__head` | 246-256 | REAL | — | Sticky; opaque composite matching the window fill (no tint seam). |
| `.shortcuts__headrow, .shortcuts__row` | 260-266 | REAL | — | 1fr/270/270 grid, 89px gutter — frame arithmetic correct. |
| `.shortcuts__headrow` / `__headcell` (+nth-child) | 268-286 | REAL | — | Tertiary ink per Lead WCAG ruling (documented deviation from #6B6B6B); centered columns. |
| `.shortcuts__rule` | 293-298 | REAL | — | 1px surface-raised (frame #222222 dark exact). |
| `.shortcuts__group-title` | 301-311 | REAL | — | ≈16px; #969696 via 15/85 mix (token-lane ask documented). |
| `.shortcuts__rows` / `.shortcuts__row + __row` | 315-325 | REAL | — | 47px pitch. |
| `.shortcuts__action` | 327-334 | REAL | — | Ellipsis on overflow. |
| `.shortcuts__keycap` | 338-347 | REAL | — | 270×33 r6; token surface. |
| `.shortcuts__combo` | 349-358 | REAL | — | Tabular-nums for F1..F10 columns — nice detail; max-width clears the pencil. |
| `.shortcuts__edit` (+`--idle`, hover, focus) | 364-390 | REAL | — | 28px pencil; idle variant `pointer-events:none` and rendered `aria-hidden` in TSX (verified) — inert for mouse AND keyboard. |
| `.shortcuts__empty` | 394-398 | REAL | — | Quiet empty state. |
| `.shortcuts__footer` / `.lumen-button` / `__cancel` / `__save:disabled` | 404-434 | REAL | F-002 | 100×32 buttons; flat Cancel (duplicated recipe); disabled Save keeps frame tint. |
| `:root[data-theme="light"] … __save:disabled` | 441-443 | REAL | — | Fixes a documented 1.75:1 light contrast hole. |
| `:root[data-theme="light"] __topbar, __keycap` | 455-458 | REAL | — | Light wells to surface-deep (Δ12 step, AA math documented). |
| `@media (width <= 1120px)` block | 466-476 | REAL | — | Narrow-stage compression; keycap columns minmax(150,220), filter 170px; ellipsis handles the rest. |

## 5. Test honesty (ShortcutsScreen.test.tsx — behavior tests, not CSS)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| 15 cases: route render, friendly keycap labels, one keycap per binding, edit-pencil links, numeric sort, search/filter scoping (incl. pure filterGroups totality), scrim/✕/Escape dismissal | YES | DOM-behavioral assertions over the real screen; they exercise this sheet's class contract (a renamed class breaks them). No visual-regression of the CSS itself in this suite (Playwright visual configs exist elsewhere — w15-g003, another agent). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens only (doc 08 §3.2)" (header 25) | this file | REAL | Zero raw hex/rgb in declarations (all via `var()`/`color-mix` over tokens); all 20 vars exist in generated tokens (rg loop). Raw px is geometry-only, as the header exempts (23-24). |
| "the render surface … Dialog primitive: scrim + trap + Escape/scrim close come from <Dialog>" (header 5-9) | this file | REAL | ShortcutsScreen.tsx:402 mounts `<Dialog open … closeOnOutsideClick>`; tests 260-272 assert scrim/✕/Escape dismissal. |
| "Save … disabled — read-only reference" (header 20-21) | this file | REAL | `.shortcuts__save:disabled` styled; screen renders the shortcuts list read-only (test: pencil LINKS to the editing surface instead). |

## 7. Dead / duplicated / unreachable code
- No dead selectors (all 34 rules target used classes; cross-checked).
- Duplicated recipe: flat-Cancel color-mix shared with network-stream.css (F-002).
- Unreachable as a shipped binary (W14 host non-compiling) — applies to the whole UI; not a defect of this file.

## 8. Dependencies & flags
- Depends on the shared Dialog (`lumen-dialog`) and Dropdown (`lumen-dropdown`) grammar and `.lumen-button`; tokens from themes/generated/tokens.css. No flags, no `@import`, one media query (modern range syntax — vite/esbuild fine).

## 9. Open questions for the next agent
- The two "no token yet" sites (230, 310: #969696 as 15/85 content mix) are a token-lane ask — whoever audits the token CSS should confirm whether a mid-gray token landed since.
