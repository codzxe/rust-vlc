# AUDIT: ui/src/components/menu/Dropdown.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / functions | 435 / 10 (incl. `clippingBounds` helper + `Select` alias) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (web app build): consumed by 8+ screens — `rg "menu/Dropdown"` → ConvertScreen, ShortcutsScreen (field filter), LocalePicker (my file #20), settings areas audio/codecs/staged, SettingField, player tools `useDropdownScrollportFlip`. Baseline #22 build OK, #23 vitest 1705 passed incl. Dropdown.test.tsx (21 cases). No GUI binary hosts it (W14-A01 host non-compiling). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `Read` Dropdown.tsx 1-435 (complete); `rg -n "describe|it(" Dropdown.test.tsx` (21 behavioral cases incl. flip-preference regression tests); `rg -ln "menu/Dropdown" src --glob '!**/menu/*'` (8 consumer screens); `sed -n '409,436p' menu.css` (scrollport max-height 320 + overflow-y auto — the residual-clip mitigation for F-004); `sed -n` verification of quoted ranges. |

## 1. One-paragraph verdict
A genuinely well-built accessible listbox combobox: roving tabindex, typeahead with reset timer, disabled-option skipping, focus restoration, outside-pointer dismiss, a clipping-container-aware direction flip (walking ancestors for real overflow, not the viewport), and 21 behavioral tests that pin all of it — including regressions for two past defects (Escape bubbling into host dialogs; scrollport yank on re-render) whose fixes are explained in-line. The defects that remain are peripheral: the closed trigger ignores ArrowDown/ArrowUp as open affordances (APG combobox pattern — Enter/Space still work), Tab is swallowed instead of advancing focus after close, `optionRefs` never shrinks with `options`, and the file's own comment admits the neither-side-fits clipping case is deferred (mitigated by the list's internal scrollport). No fabrication, no dead code, honest comments.

## 2. What actually works (verified)
— Verified by baseline #23 (ui-vitest.log.txt, 1705 passed): all 21 Dropdown.test.tsx cases — combobox ARIA wiring, roving focus, arrow/Home/End/Enter/Escape/Tab, typeahead, disabled skipping, outside dismiss, density classes, icon slots, and the three preferredDirection flip cases — run against this component.
— Verified by read: `enabledIndex` (169-178) is a correct modular wrap that skips disabled options in both directions and returns -1 when all are disabled; `openList` (202-211) seeds the selected option or the first enabled one; Escape/Tab `stopPropagation` (299) is a real regression fix for host-dialog double-close, and Dropdown.test.tsx:137 pins it.
— Verified by read: `clippingBounds` (27-43) walks to the nearest ancestor whose computed overflow actually clips, falling back to the viewport — the h15 HDR repro reasoning in the comment is coherent and the jsdom fallback keeps tests honest.
— Verified by read: the open-transition effect (222-231) focuses the initial option exactly once per open (`deps [open]`, eslint-disable documented); the scrollTop reset moved from an inline ref callback to a keyed effect (240-243) fixes the re-render scroll yank.

## 3. Findings

### F-001 [S3]
Where: ui/src/components/menu/Dropdown.tsx:359-374
Code:
```tsx
      <button
        ref={(element) => {
          triggerRef.current = element;
          if (typeof ref === "function") ref(element);
          else if (ref !== null && ref !== undefined) ref.current = element;
        }}
        type="button"
        className="lumen-dropdown__trigger"
        aria-haspopup="listbox"
```
What is wrong: the closed trigger has NO `onKeyDown` — ArrowDown/ArrowUp on the focused trigger do nothing. The WAI-ARIA APG combobox (select-only) pattern specifies ArrowDown/ArrowUp open the popup and move focus into the list (Alt+ArrowDown = open only). Here only Enter/Space (native button activation) open the list.
Why it matters: keyboard-first and screen-reader users expect arrows to open a listbox popup — the component implements the full listbox keyboard model INSIDE the list but not the entry affordance on the trigger; every host screen inherits the gap (8+ consumers).
What correct looks like: `onKeyDown` on the trigger: ArrowDown/ArrowUp (or Alt+ArrowDown) → `openList()` (the open effect already focuses the initial option).
Confidence: HIGH — Verification: read (no key handler on the trigger; Dropdown.test.tsx has no trigger-arrow case either).

### F-002 [S4]
Where: ui/src/components/menu/Dropdown.tsx:293-301
Code:
```tsx
    if (key === "Escape" || key === "Tab") {
      closeAndRestore();
```
What is wrong: Tab is `preventDefault()`ed (300) and focus is restored to the trigger — the keystroke never advances focus to the next control. The APG listbox pattern closes the popup on Tab and lets focus MOVE ON. Net effect: keyboard users must press Tab twice to leave a closed dropdown.
Why it matters: minor keyboard friction on every dropdown close; deliberate trade (the g19 fix scoped close to the dropdown) taken one step further than the pattern requires.
What correct looks like: Escape → closeAndRestore + preventDefault; Tab → close, restore nothing (let the browser move focus), stopPropagation only.
Confidence: MEDIUM — Verification: read (behavior traced; APG deviation asserted from the published pattern, not run against a screen reader).

### F-003 [S4]
Where: ui/src/components/menu/Dropdown.tsx:397-402
Code:
```tsx
          {options.map((option, index) => (
            <div
              key={option.value}
              ref={(element) => {
                optionRefs.current[index] = element;
              }}
```
What is wrong: `optionRefs.current` is only ever written, never truncated — when `options` shrinks between renders (dynamic option sets, e.g. filtered language lists), stale DOM nodes linger at the tail of the array. No behavior bug: every read (`focusIndex`, the open effect) is bounded by the current `options.length`.
Why it matters: hygiene; a future unbounded read (e.g. an "after last option" focus helper) would resurrect a detached node.
What correct looks like: `optionRefs.current.length = options.length` before the map (or a cleanup effect).
Confidence: HIGH — Verification: read (all read sites bounded).

### F-004 [S4]
Where: ui/src/components/menu/Dropdown.tsx:142-148
Code:
```tsx
  /* w26.g17/g19 P1 Lead fix: flip-up when the viewport has more room above
   * the trigger than below, so lists open on the side with room. Residual
   * case (j02 J02-7, deferred — see lane log w26.k02): when NEITHER side
   * fits the full 320px scrollport, the measured fallback still clips the
   * list (no dynamic max-height yet); the scrollport cap keeps that window
   * narrow. */
```
What is wrong: an acknowledged open defect: when neither side of the clipping container fits the 320px scrollport, the list still opens at the roomier side and CLIPS — no dynamic max-height. Mitigation verified: the list's own `overflow-y: auto` scrollport (menu.css:429-430) lets pointer users scroll clipped rows back into the visible band, and focus-follows-scroll keeps keyboard users whole, so rows remain reachable — the defect is initial visual clipping, not lost options.
Why it matters: honest deferral, real but narrow UX degradation (dense dialogs with centered dropdowns); recorded so it is not re-discovered as a "bug" later.
What correct looks like: `estimated = Math.min(..., bounds.bottom - rect.bottom or above)` — size the list to the available room on the chosen side.
Confidence: HIGH — Verification: read (both the admission and the CSS mitigation).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `clippingBounds` | 27-43 | REAL | — | Ancestor walk for real clipping; viewport fallback; SSR guard. Correct. |
| `optionDataAttributes` | 113-116 | REAL | — | data-* spread; trivial. |
| `Dropdown` (component) | 118-431 | REAL | F-001 | Full combobox; see sub-rows. |
| — `enabledIndex` | 169-178 | REAL | — | Modular wrap skipping disabled; -1 when none. Correct. |
| — `focusIndex` | 180-187 | REAL | — | Focus + `scrollIntoView({block:"nearest"})` guarded for jsdom. |
| — `closeAndRestore` | 189-193 | REAL | — | Resets typed, restores trigger focus. |
| — `select` | 195-200 | REAL | — | Disabled guard; onChange + close. |
| — `openList` | 202-211 | REAL | — | Seeds selected-or-first-enabled; -1 when all disabled (honest inert list). |
| — typeahead cleanup effect | 150-155 | REAL | — | Unmount timer cleanup. Correct. |
| — outside dismiss effect | 158-167 | REAL | — | pointerdown listener gated on `open`; cleanup. Correct. |
| — open-transition effect | 222-231 | REAL | — | Focus initial option once per open; whole-list reveal for scroll hosts; deps `[open]` documented. |
| — scrollTop reset effect | 240-243 | REAL | — | Once per open; replaces the re-render-firing inline ref (documented defect fix). |
| — direction `useLayoutEffect` | 248-265 | REAL | F-004 | Clipping-container measurement; estimate `min(len*pitch+8, 320)`; preferred side wins while it fits; re-measures on density/length change; no trigger-move listener (documented). |
| — `handleListKeyDown` | 267-324 | REAL | F-002 | Arrows/Home/End/Enter/Space/Escape/Tab; typeahead accumulate + timer + wrap-around prefix match (starts AFTER active, wraps). All correct except Tab swallow (F-002). |
| — `selectedLabel` | 328-329 | REAL | — | Empty string for unmatched value (honest). |
| — render (trigger/list/options) | 341-430 | REAL | F-001, F-003 | ARIA: haspopup/expanded/controls(describedby)/listbox/option/selected/disabled; React-19 ref-as-prop forwarding (returns undefined — no cleanup-return hazard); option icons: iconNode > icon; `Select` alias (434) re-exports same component (card 4.B3). |

## 5. Test honesty (Dropdown.test.tsx, 21 cases, ran in baseline #23)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| renders/aria cases (34-73) | YES | Assert label resolution, aria-describedby wiring, aria-controls id link — the J02-2 a11y fix is pinned. |
| open/focus/arrow/Home/End/Enter/Escape cases (74-136) | YES | Real key events, focus assertions, onChange spy. |
| Escape/Tab no-bubble regression (137) | YES | Host dialog with an Escape listener stays open — pins the g19 stopPropagation fix. |
| typeahead (159) | YES | Types a multi-char prefix, asserts the jump. |
| disabled options (178), outside dismiss (190) | YES | Click + pointerdown simulations. |
| density/icons (199, 218) | YES | Class and DOM assertions. |
| preferredDirection suite (240-274) | YES | Mocks getBoundingClientRect; preference-fits vs preference-clips vs default — pins the i16 logic. |
| NOT covered: trigger ArrowDown-open (F-001), options-shrink refs (F-003), neither-side-fits clip (F-004) | — | Gap list, honest. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "combobox trigger with `aria-haspopup="listbox"`, popup `role="listbox"` with roving tabindex options, arrows/Home/End, Enter/Space select, Escape/Tab close with focus restoration … typeahead jump, disabled options skipped" (doc comment 50-53) | this file | REAL | All verified by read + 21 tests (baseline #23). |
| "a popup must never run past the viewport edge" (10-11) | this file | FALSE (partially) | F-004: the file's own comment admits the neither-side-fits case still clips; mitigated by internal scroll. |
| "Portal-free absolute positioning inside a relatively-positioned wrapper" (53-54) | this file | REAL | No createPortal; `.lumen-dropdown__list` absolute (menu.css:410). |

## 7. Dead / duplicated / unreachable code
- No dead code. `Select` alias (434) is a deliberate API alias (card 4.B3), used by consumers.
- Unreachable as a shipped binary (W14) — global UI condition, not this file's defect.

## 8. Dependencies & flags
- React 19 (useId, ref-as-prop), in-tree Icon/timings/types. No feature flags, no unsafe patterns, no external state.

## 9. Open questions for the next agent
- `useDropdownScrollportFlip.ts` (player tools) duplicates direction logic for its own host — check for drift against this component's `clippingBounds` estimate constants (42/46/320).
- The `aria-label={`${label}: ${value}`}` restructure deferred at 336-340 (60 role-query test renames) — still open; whoever touches those tests should do the rename the comment prescribes.
