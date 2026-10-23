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
| off | knob `--color-surface-muted` left, fill width 0 | `aria-checked="false"` |
| on | fill 33x26 `--color-accent-tinted` + knob `--color-accent-strong` right | `aria-checked="true"` |
| hover | track `--color-surface-hover` | `:hover` |
| focus-visible | 2px accent ring, 2px offset | `:focus-visible` |
| disabled | `--opacity-disabled`, inert, state still rendered truthfully | `:disabled` |

ARIA note: the task card mentioned `aria-pressed`, but doc 08 §4.5 mandates
`toggles → switch`; implemented as `role="switch"` + `aria-checked` (a switch
carries checked state, not pressed). Keyboard: native `<button>` gives
Space/Enter activation.

## Honest gaps (next sessions)

- Visual snapshots: 4.G1's harness doesn't exist; vitest stubs `.css` imports,
  so jsdom never cascades the stylesheet. Current "visual snapshot" = DOM
  snapshot + CSS-source→semantic-token assertions (`?raw` import). Swap to
  real screenshot coverage when 4.G1 lands.
- Toggle micro labels (`--font-size-micro`, on=accent / off=content-tertiary
  per Toggle.svg) not yet rendered.
- Toggle knob `border-radius: 3px` is raw px — no 3px semantic radius token
  exists; needs a token-seam decision (token README follow-up), not a local
  constant.
- Button variant list is intentionally minimal (doc 08 exports contain no
  secondary/ghost/danger buttons); extend only with design provenance.
- Form inputs (TextField/NumberField/Checkbox/Radio/RadioGroup/Switch) not
  started — next session, per plan.
