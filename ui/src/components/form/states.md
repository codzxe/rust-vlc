# Form input state table (card 4.B1, doc 08 §5.5)

Provenance: doc 08 §1.4 onboarding text input (469x34, surface-deep fill,
2px surface stroke), §1.5 Controls.svg text_input states + Values.svg
steppers, §2/§4.4 interim states. Enforced by colocated tests.

## TextField / NumberField

| State | Visual | Mechanism |
|---|---|---|
| rest | `--color-surface-deep` fill, 2px `--color-surface` stroke, `--radius-control` | class `lumen-field__control` |
| hover | `--color-surface-hover` fill | `:hover` |
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
