# Form input state table (card 4.B1, doc 08 §5.5)

Provenance: doc 08 §1.4 onboarding text input (469x34, surface-deep fill,
2px surface stroke), §1.5 Controls.svg text_input states + Values.svg
steppers, §2/§4.4 interim states. Enforced by colocated tests.

## TextField / NumberField

| State | Visual | Mechanism |
|---|---|---|
| rest | `--color-surface-deep` fill, 2px `--color-surface` stroke, `--radius-control` | class `lumen-field__control` |
| hover | `--color-surface-hover` fill | `:hover` |
| pressed | `--color-surface-selected` fill | `:active` (§4.4 closure 2026-10-24) |
| focus-visible | 2px `--color-accent` ring, 2px offset | `:focus-visible` |
| disabled | `--opacity-disabled` + pointer-events none | `:disabled` |
| error | `--color-record` stroke + caption message; `aria-invalid` + `aria-describedby` | `[aria-invalid="true"]` |

NumberField adds pointer steppers (Values.svg): `+`/`−` SVG glyphs,
`tabIndex=-1` + `aria-hidden` (keyboard users have native arrow/Home/End),
disabled at min/max bounds, values clamped — never fabricated out of range.

## Checkbox / Radio

Native inputs (`accent-color: --color-accent`), label wired via
`htmlFor`/`id`; disabled via native attribute (browser-gated); invalid flag
sets `aria-invalid` (Checkbox per-input, RadioGroup on the group). No
checkbox/radio exports exist in the design package — composed from the
accent/icon language (doc 08 §2 interim).

RadioGroup: `role="radiogroup"` + `aria-label`, roving tabindex (selected
option tabbable), arrow keys (all four) move selection with wrap-around,
per-option disabled supported.

## Switch

Re-export of the button family's `Toggle` (doc 08 §1.5 ships a single
Toggle.svg; §4.5 assigns it the `switch` role) — see ../button/states.md.

## §4.4 state-matrix audit (closure, 2026-10-24)

Doc 08 §4.4: "Every interactive primitive implements: rest, hover,
active/pressed, focus-visible, disabled, and where relevant selected, checked,
open, dragging, pending (§5)."

- rest/hover/pressed/focus-visible/disabled: present on every interactive
  control — field controls + steppers (`:active` → surface-selected) and
  choice rows (`:active` → accent-icon, IconButton's pressed language) gained
  pressed skins in the closure commit; enforced by the CSS-token tests in
  TextField.test.tsx.
- checked: Checkbox/Radio via native `accent-color`; Switch via Toggle.
- selected: RadioGroup selection = roving tabindex + `aria-checked`
  (Radio.tsx) — no separate visual skin pending a radio export in the design
  package.
- open/dragging: N/A for value-entry primitives.
- **pending/loading on form inputs: NOT required by the doc** — §4.4 scopes
  pending to "where relevant (§5)", and §5's pending states (PlayButton
  command dispatch, SeekBar pending seek, VolumeControl mute-pending,
  TrackMenu disabled-while-loading, PlaylistPanel reorder-pending, dialog
  Save-pending) all belong to command-dispatching components. Form inputs
  dispatch no engine commands; a loading skin here would be fabricated state
  (MP §2). Recorded so the next auditor doesn't re-derive it.
- error: beyond the matrix — wired per §5.4 adjacency (typed errors render,
  never throw).

## Honest gaps (next sessions)

- Error color is `--color-record` (only red semantic token) — a dedicated
  `--color-danger` semantic token is a token-seam request, not a local hack.
- Export rx5 (text input) mapped to `--radius-control` (6px) — no 5px token.
- NumberField steppers synthesize a ChangeEvent (no `stepUp/stepDown` in
  jsdom); values still flow through the single controlled `onChange`.
- DOM snapshots embed React `useId` values — stable per file order, but a
  future snapshot re-pin may be needed if test order changes.
- Visual snapshot adaptation (4.G1 harness pending): DOM snapshot +
  CSS-source→semantic-token assertions via node:fs — see ../button/states.md.
- i18n: labels/errors are caller-supplied strings (no literals in
  components); wiring to useMsg() MessageKeys happens at screen composition.
