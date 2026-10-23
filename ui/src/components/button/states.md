# Button family state table (card 4.B1, doc 08 §5.5)

Provenance: `design/figma` exports per doc 08 §1.4/§1.5; interim states from
doc 08 §2/§4.4 `[PROPOSAL]`. Enforced by the colocated interaction tests.

## Button

| State | Visual | Mechanism |
|---|---|---|
| rest | `--color-surface-deep` fill, `--color-content-secondary` glyph, `--radius-inset` (Button.svg 360x32 rx4) | class `lumen-button` |
| hover | `--color-surface-hover`, glyph `--color-content-bright` | `:hover` |
| pressed | `--color-surface-selected` | `:active` |
| focus-visible | 2px `--color-accent` ring, 2px offset (doc 08 §2) | `:focus-visible` |
| disabled | `--opacity-disabled` + `pointer-events:none`; native `disabled` => no handlers | `:disabled` |
| loading/pending | `--opacity-emphasis` + CSS spinner glyph + `aria-busy="true"`; inert | `[aria-busy]` |

Variants: `default` (Button.svg), `primary` (onboarding: `--color-accent`
fill, `--color-content-on-accent` glyph, `--radius-control`). Sizes: 28/32/48
(`sm`/`md`/`lg`, doc 08 §1.4 control heights).

## IconButton — same state matrix; rest color `--color-stroke-icon`,
hover `--color-content-bright` on `--color-surface-hover`, pressed
`--color-accent-icon`; `aria-label` mandatory (§4.5).

## Toggle

| State | Visual | Mechanism |
|---|---|---|
| off | knob `--color-surface-muted` left, fill width 0, "OFF" micro label right (`--color-content-tertiary`) | `aria-checked="false"` |
| on | fill 33x26 `--color-accent-tinted` + "ON" micro label (`--color-accent`) + knob `--color-accent-strong` right; OFF label `display:none` | `aria-checked="true"` |
| hover | track `--color-surface-hover` | `:hover` |
| focus-visible | 2px accent ring, 2px offset | `:focus-visible` |
| disabled | `--opacity-disabled`, inert, state still rendered truthfully | `:disabled` |

Micro labels (doc 08 §1.3 row 4.4 + §1.5 Toggle.svg): `--font-size-micro`,
ON = `--color-accent` (#D37918), OFF = `--color-content-tertiary` (#6B6B6B) —
both export hexes map exactly onto existing semantic tokens. Decorative
(`aria-hidden`): the switch conveys state via `aria-checked` alone; label text
is not translated (swap to i18n keys only if ever exposed to the a11y tree).

ARIA note: the task card mentioned `aria-pressed`, but doc 08 §4.5 mandates
`toggles → switch`; implemented as `role="switch"` + `aria-checked` (a switch
carries checked state, not pressed). Keyboard: native `<button>` gives
Space/Enter activation.

§4.4 audit (closure, 2026-10-24): rest/hover/pressed(`:active` on the button
itself — toggle presses read as track hover+fill motion; no separate pressed
skin in Toggle.svg)/focus-visible/disabled/checked all present. Pending: not
required — §4.4 scopes it "where relevant (§5)" and §5's pending states belong
to command-dispatching components (PlayButton, SeekBar, dialogs); Toggle
dispatches no engine command.

## Honest gaps (next sessions)

- Visual snapshots: 4.G1's harness doesn't exist; vitest stubs `.css` imports
  (`?raw` returns EMPTY — use `node:fs` + `import.meta.dirname`), so jsdom
  never cascades the stylesheet. Current "visual snapshot" = DOM snapshot +
  CSS-source→semantic-token assertions. Swap to real screenshot coverage when
  4.G1 lands.
- Toggle knob `border-radius: 3px` is raw px — no 3px semantic radius token
  exists; handoff to the token owner (Lead, themes/tokens.json is Rust-owned),
  not a local constant.
- Button variant list is intentionally minimal (doc 08 exports contain no
  secondary/ghost/danger buttons); extend only with design provenance.
- Form inputs (TextField/NumberField/Checkbox/Radio/RadioGroup/Switch) landed
  in a79e9f7 — form-zone audit in form/states.md.
