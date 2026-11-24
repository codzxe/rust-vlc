# AUDIT: ui/src/components/menu/Dropdown.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 434 / 11 (clippingBounds, optionDataAttributes, Dropdown + 7 inner helpers, Select alias) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — imported by ShortcutsScreen.tsx:432-area, ConvertScreen.tsx, settings/areas/{audio,codecs,staged}.tsx, settings/SettingField.tsx, language/LocalePicker.tsx (rg -l, list above) -> routed screens -> App -> main.tsx |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | `rg -ln "menu/Dropdown\""` (8 importer files); `rg -n "import \{[^}]*\bSelect\b" ui/src` (empty); `sed -n '92,103p;142,148p;433,435p'` quote checks; ls ui/node_modules/.bin/vitest -> absent |

## 1. One-paragraph verdict
A genuinely implemented combobox: controlled value, roving-tabindex listbox,
Arrow/Home/End/Enter/Space/Escape/Tab handling with focus restoration,
printable-key typeahead with timed reset, disabled-option skipping, outside-
pointer dismiss, density variants, and a measured open-direction flip against
the nearest clipping ancestor (not the naive viewport). No stubs, no fake
seams. The component is UI-local by design (value changes flow through
`onChange` to the host); whether hosts wire `onChange` to anything real is the
hosts' question. Two defects: an admitted clipping residual (neither-side-fits)
and a dead `Select` alias export.

## 2. What actually works (verified)
— Reachability: 8 importer files (rg list, section header). — Behavior is real
per code read: enabledIndex wrap logic (L169-178) handles all-disabled lists
(returns -1); select() refuses disabled options (L195-200); Escape/Tab
stopPropagation prevents host-dialog double-close (L293-301). — Tests exist and
are behavioral (Dropdown.test.tsx: 21 `it` blocks incl. focus movement,
disabled skipping, onChange firing, outside dismiss, direction fallback).
Not executed here: ui/node_modules absent and npm install is banned
(cargo/npm budget) — baseline run (audit/runs/baseline) reports vitest
1705 passed / 1 skipped for the whole UI suite.

## 3. Findings
F-001 [S3]
Where: ui/src/components/menu/Dropdown.tsx:142-148
Code:
```
 * case (j02 J02-7, deferred — see lane log w26.k02): when NEITHER side
 * fits the full 320px scrollport, the measured fallback still clips the
 * list (no dynamic max-height yet); the scrollport cap keeps that window
 * narrow. */
```
What is wrong: in a short clipping container (both above and below < estimated
list height) the list still overflows and clips — options can be painted
nowhere, exactly the class of bug w26.i16 fixed for the one-side-fits case.
Admitted and deferred, with no tracking issue beyond a lane-log reference.
Why it matters: real windows (short dialogs, settings rows near screen bottom)
hit this; 5/6 options invisible was the original repro.
What correct looks like: dynamic max-height = available room when neither side
fits.
Confidence: HIGH — Verification: read (logic L248-265 confirmed: no max-height
path exists).

F-002 [S4]
Where: ui/src/components/menu/Dropdown.tsx:433-434
Code:
```
/** Alias per card 4.B3 ("Dropdown/Select"). */
export const Select = Dropdown;
```
What is wrong: `Select` is re-exported from menu/index.ts:4 but no file in
ui/src imports the alias (`rg -n "import \{[^}]*\bSelect\b[^}]*\} from" ui/src`
-> empty). Dead public API.
Confidence: HIGH — Verification: ran rg.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| clippingBounds | 27-43 | REAL | — | walks to nearest clipping ancestor; viewport fallback; jsdom-safe |
| optionDataAttributes | 113-116 | REAL | — | data map -> data-* spread |
| Dropdown (component) | 118-431 | REAL | S3 (F-001) | full combobox; see helpers |
| — enabledIndex | 169-178 | REAL | — | modulo wrap; all-disabled -> -1 |
| — focusIndex | 180-187 | REAL | — | guarded scrollIntoView (jsdom) |
| — closeAndRestore | 189-193 | REAL | — | focus back to trigger |
| — select | 195-200 | REAL | — | refuses disabled |
| — openList | 202-211 | REAL | — | initial = selected or first enabled |
| — open effect (fold reveal) | 222-231 | REAL | — | eslint-disable for exhaustive-deps is scoped + commented (legit open-only semantics) |
| — scrollTop reset effect | 240-243 | REAL | — | fixes re-render scroll yank (documented) |
| — direction useLayoutEffect | 248-265 | REAL | S3 (F-001) | measured flip vs preferredDirection |
| — handleListKeyDown | 267-324 | REAL | — | arrows/Home/End/Enter/Space/Escape/Tab/typeahead |
| — selectedLabel | 328-329 | REAL | — | "" for unmatched value (controlled hosts must pass a matching value) |
| Select (alias) | 434 | UNREACHABLE | S4 (F-002) | exported, never imported |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| "moves with ArrowDown/ArrowUp, skipping disabled options" | YES | asserts toHaveFocus on the exact next/prev option (L92-103, quote above) |
| "Escape/Tab close ... WITHOUT closing a listening host dialog" | YES | host dialog mock asserts not dismissed (L137+) |
| "the preference never clips: falls back to the measured side" | YES | mocks element rects, asserts data-direction |
| 17 others | YES (spot-read) | role queries + focus/onChange assertions, no tautologies |
Not run in this sandbox (no node_modules); baseline suite cited.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "disabled options skipped and not selectable" | Dropdown.tsx:52-53 | REAL | L175, L197 guards |
| "keep in sync with the CSS cap max-height: 320px" | Dropdown.tsx:12-14 | UNVERIFIED | menu.css not in my assignment; next agent: confirm the 320px cap exists |

## 7. Dead / duplicated / unreachable code
`Select` alias (F-002). Nothing else.

## 8. Dependencies & flags
./Icon, ./types (identityResolveLabel, MenuDensity, ResolveLabel), ./timings
(TYPEAHEAD_RESET_MS), ./menu.css. No feature gates. No backend dependency —
pure presentational component; `onChange` wiring is host responsibility.

## 9. Open questions for the next agent
- Do the settings hosts (audio.tsx, codecs.tsx, LocalePicker.tsx) forward
  `onChange` to a persisted store, and does anything consume the persisted
  `global.language` value at startup? (rg shows setActiveLocale called ONLY
  from main.tsx:51 URL param — the language setting looks unconsumed; see my
  messages.ts audit F-002.)
- Confirm menu.css `.lumen-dropdown__list { max-height: 320px }` matches
  DROPDOWN_LIST_MAX_HEIGHT (L15).
