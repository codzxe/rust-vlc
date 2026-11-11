# Slider family states (card 4.B2; w26.a04 restyle)

Provenance: W25 figma-fidelity spec §1/§2 + measured frames (Player.svg,
Controls.svg, EQ.svg, Audio - Equalizer.svg, volume.svg, audio value.svg);
interaction contract doc 08 §4/§4.5/§5 (unchanged). Enforced by the
colocated tests (Slider / Seekbar / Volume / EQSlider).

## Interaction contract (all four components — unchanged, doc 08 §5)

| Gesture / key | Fires | Renders |
|---|---|---|
| pointer drag | `onChangeLive` continuously | preview position (`--lumen-slider-thumb`), `data-dragging` |
| pointer release | `onCommit` (final value, once) | reverts to controlled `value` |
| `pointercancel` | nothing (abort) | reverts to controlled `value` |
| ArrowRight/Up, ArrowLeft/Down | `onChangeLive` → `onCommit` (that order) | controlled `value` (caller must feed back) |
| Shift+Arrow, PageUp/PageDown | idem, coarse step (×10) | idem |
| Home / End | idem, min / max | idem |

The component is fully controlled: after commit it renders only what the
caller passes back via `value` — never an optimistic jump (doc 08 §5).
Hit areas are ≥16px on the interaction axis (roots 20–24px tall/wide over
4–14px painted tracks).

## Anatomy (slider.css; w26.a04)

| Component | Track | Fill | Thumb |
|---|---|---|---|
| `Slider` (base, horizontal) | 14px r7 pill, `surface` (#090909 on the frames' #1b1b1b panels; frame: Audio - Equalizer.svg) | full-height pill, amber gradient `accent-tinted`→`accent-strong` (== frame #371f06→#a5621a to Δ≤6/255 per channel; SVG rects are flat #371f06 — gradient per W25 spec §2/brief VLM measurement), width = value | 8×20 r4 `accent-strong`, overhangs track |
| `Slider` vertical | 14px-wide pill | bottom-anchored, gradient to-top | 20×8 r4 |
| `EQSlider` (additive) | base pill, full width | base gradient | base handle |
| `Seekbar` | 4px pill-ended, `surface-deep` (brief/W25 §1 recessed well) | `accent`, pill ends, width = engine position | circular 12px `accent`, opacity 0 at rest |
| `VolumeSlider` | 4px pill-ended, `surface-deep` (seekbar grammar), compact 96px | level fill: `--gradient-volume-level` token if present, else `accent` | circular 12px `accent`, opacity 0 at rest |

Value chip (EQ rows, audio value.svg) is rendered by CONSUMERS — outside
the slider primitive.

## States

| State | Visual (slider.css) | ARIA / data |
|---|---|---|
| rest | above; seekbar/volume thumbs hidden (quiet bars) | `role=slider`, `aria-valuemin/max/now(/text)`, `aria-orientation`, `tabindex=0` |
| hover | handle → accent-strong lifted toward content-bright (color-mix; a real signal in dark, where accent-bright == accent); track well lifts one step (per-well mix toward surface-hover: `surface` base / `surface-deep` slim bars); seekbar/volume circular thumb fades in | — |
| dragging (active) | `data-dragging`, grabbing cursor, handle stays `accent-bright`, thumb visible | `aria-valuenow` tracks the preview |
| focus-visible | 2px `accent` outline, 2px offset; hidden thumbs revealed | — |
| disabled | `--opacity-disabled`, `pointer-events:none` | `aria-disabled="true"`, `tabindex=-1`, value stays truthful |

## Seekbar truth table (doc 08 §5.2 — unchanged semantics)

| Phase | Knob | Fill | ARIA / data |
|---|---|---|---|
| rest (engine truth) | at `value` (hidden until hover) | `--lumen-slider-fill` = `value` | `aria-valuenow` = `value`, valuetext = formatted time |
| 1. scrubbing | follows local preview | **stays at engine position** | `data-dragging`, `aria-valuenow` = preview |
| 2. pending (`pending` prop) | locked at requested target, `accent-strong`, visible | **stays at engine position** | `data-pending="true"`, `aria-valuenow` = target |
| 3. confirmed | animates to engine truth | animates to engine truth | pending attribute removed |
| failed seek | snaps back to engine truth | never left it | no fabricated jump |
| no media (`duration <= 0` / caller `data-empty`) | inert, hidden (calm unplayed track) | empty | `aria-disabled="true"`, `aria-valuemax="0"` |

Buffer ranges: engine-reported `[{start,end}]` rendered as translucent
`content-secondary` sub-region overlays ([PROPOSAL] token mapping — no
buffer color exists in the frames). No ARIA on them (doc 08 §4.5 gives the
slider role to the bar itself). The component never invents buffer data.

## Token gaps logged for the token lane (do not hand-code in consumers)

- `--color-eq-fill-start` / `--color-eq-fill-end` — EQ amber gradient stops
  (spec/brief: `#371f06` → `#a5621a`; the SVG rect fills are FLAT `#371F06`
  with an `#AB6314` handle — EQ.svg / Audio - Equalizer.svg source rects; the
  gradient reading is the W25-spec/brief VLM measurement of the renders).
  slider.css falls back to `--color-accent-tinted` → `--color-accent-strong`
  (#371f06 exactly; #ab6314 is Δ≤6/255 per channel from #a5621a) — visually
  equivalent, tokens only.
- `--gradient-volume-level` — the volume signal gradient
  (`#95D318 → #B2A918 → #D37918 → #FF3737`, revealed to the level;
  Player.svg/volume.svg). slider.css falls back to `--color-accent`.

## Structural values without tokens (raw px in slider.css)

`4`/`12`/`14`/`20`/`24`px track & knob dimensions, `7px`/`4px`/pill radii,
`96`/`160`px default extents, `2px` focus ring width/offset — pinned by the
design exports (W25 spec §2); no semantic token exists for them.
