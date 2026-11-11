# Form input state table (W26 refresh — card 4.B1 / doc 08 §5.5)

Provenance: W26 frames (design/figma/*.svg, W25 spec §0) — Toggle.svg,
Values.svg, Values-1.svg, Settings - interface/global.svg, audio value.svg,
EQ.svg, Onboarding.svg. Enforced by the colocated tests + the lane log's
Chromium computed-style proof.

## TextField / NumberField

| State | Visual | Mechanism |
|---|---|---|
| rest | `--color-surface-input` (#2a2a2a fallback), 1px `--color-stroke-input` (#3a3a3a fallback), `--radius-control` (r6), 34px | class `lumen-field__control` |
| hover | `--color-surface-hover` fill + tertiary border | `:hover` |
| pressed | `--color-surface-selected` fill | `:active` (§4.4 closure 2026-10-24) |
| focus | orange border (`--color-accent`) | `:focus` (W26 mission spec) |
| focus-visible | 2px `--color-accent` ring, 2px offset | `:focus-visible` |
| disabled | `--opacity-disabled` + pointer-events none | `:disabled` |
| error | `--color-record` border + caption message; `aria-invalid` + `aria-describedby` | `[aria-invalid="true"]` |

NumberField adds pointer steppers: `+`/`−` SVG glyphs, `tabIndex=-1` +
`aria-hidden` (keyboard users have native arrow/Home/End), disabled at
min/max bounds, values clamped — never fabricated out of range. The W26
package has no stepper frame (Values.svg became the segmented control), so
steppers reuse the panel/well language.

## Checkbox / Radio

Native inputs (`accent-color: --color-accent`), label wired via
`htmlFor`/`id`; disabled via native attribute (browser-gated); invalid flag
sets `aria-invalid` (Checkbox per-input, RadioGroup on the group). No
checkbox/radio exports exist in the design package — composed from the
accent/icon language (doc 08 §2 interim).

RadioGroup: `role="radiogroup"` + `aria-label`, roving tabindex (selected
option tabbable), arrow keys (all four) move selection with wrap-around,
per-option disabled supported.

## Switch (W26: Toggle.svg port — local component)

68×32 rx6 surface track; ON = accent-tinted 33×26 text segment ("ON" in
accent) + 26×26 accent-strong fill right; OFF = 26×26 stroke-border fill
left + panel 33×26 text segment ("OFF" tertiary). State matrix: hover →
surface-hover track (fill brightens on ON), focus-visible → 2px accent
ring, disabled → `--opacity-disabled`. Micro labels are aria-hidden;
`role="switch"` + `aria-checked` carry state. (Pre-W26 this re-exported the
button-family Toggle; wave 26 ships the local port — the button family
keeps its own for the control-parts lane.)

## SegmentedControl / SwatchGroup / ValueChip (W26)

- SegmentedControl (Values.svg): rx6 surface track, `--space-track-inset`
  2px inset, 104×32 `--radius-inset` segments; active = accent-tinted +
  accent text; inactive = panel + tertiary; hover → raised + secondary.
- SwatchGroup (Settings - global.svg): same track grammar, 32×32 hit
  targets, 20×20 rx1 dots; rest dots at `--opacity-swatch-rest` (0.7, frame
  opacity), hover lifts to 1; selected = accent-tinted rx4 slot. Radiogroup
  semantics (roving tabindex, arrow keys with wrap, per-option disabled).
- ValueChip (audio value.svg / EQ.svg): r6 `--color-surface-deep` chip,
  bright tabular-nums readout. Frame text measures #969696/#8f8f8f
  (secondary gray); spec §2 + lane mission say white — ships
  `--color-content-bright`; one-token swap if verifiers side with the frames.

## §4.4 state-matrix audit (closure, 2026-10-24; W26 re-check)

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

- Input tokens `--color-surface-input` (#2a2a2a) / `--color-stroke-input`
  (#3a3a3a) do not exist — frame-marked `var()` fallbacks ship the values;
  token-seam request to the token lane (lane log decision 2).
- Error color is `--color-record` (only red semantic token) — a dedicated
  `--color-danger` semantic token is a token-seam request, not a local hack.
- ValueChip text: frames measure secondary gray, spec says white — see the
  W26 section above.
- Pre-W26 gap "Export rx5 mapped to --radius-control" is closed: the W26
  grammar is r6 (`--radius-control`) everywhere on these controls.
- NumberField steppers synthesize a ChangeEvent (no `stepUp/stepDown` in
  jsdom); values still flow through the single controlled `onChange`.
- DOM snapshots embed React `useId` values — stable per file order, but a
  future snapshot re-pin may be needed if test order changes.
- i18n: labels/errors are caller-supplied strings (no literals in
  components); wiring to useMsg() MessageKeys happens at screen composition.
