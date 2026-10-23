# Slider family states (card 4.B2)

Provenance: docs/08-ui-figma-fidelity.md §1.4 (anatomy), §4 (architecture),
§4.5 (ARIA), §5.2 (truthful seek). Enforced by the colocated interaction
tests (doc 08 §5.5, §9.2).

## Interaction contract (all three components)

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

## Slider

| State | Visual (slider.css) | ARIA |
|---|---|---|
| rest | 16px track rx6 `surface-deep`; fill inset 2 `accent-tinted` rx4; 8×12 knob rx4 `accent` | `role=slider`, `aria-valuemin/max/now(/text)`, `aria-orientation`, `tabindex=0` |
| vertical (`orientation="vertical"`) | 16px-wide track; fill 12 wide anchored bottom; 8×24 rx4 knob | `aria-orientation="vertical"`; ArrowUp increases |
| focus-visible | 2px `accent` outline, 2px offset | — |
| dragging | `data-dragging`, grabbing cursor | `aria-valuenow` tracks the preview |
| disabled | `--opacity-disabled`, `pointer-events:none` | `aria-disabled="true"`, `tabindex=-1`, value stays truthful |

## Seekbar (doc 08 §5.2 — the truthful-seek table)

| Phase | Knob | Fill | ARIA / data |
|---|---|---|---|
| rest (engine truth) | at `value` | `--lumen-slider-fill` = `value` | `aria-valuenow` = `value`, valuetext = formatted time |
| 1. scrubbing | follows local preview | **stays at engine position** | `data-dragging`, `aria-valuenow` = preview |
| 2. pending (`pending` prop, caller-owned) | locked at requested target, `accent-strong` | **stays at engine position** | `data-pending="true"`, `aria-valuenow` = requested target |
| 3. confirmed (caller updates `value`, clears `pending`) | at engine truth | animates to engine truth | pending attribute removed |
| failed seek (caller clears `pending` without updating `value`) | snaps back to engine truth | never left it | no fabricated jump |
| no media (`duration <= 0`) | inert | empty | `aria-disabled="true"`, `aria-valuemax="0"` |

Buffer ranges: engine-reported `[{start,end}]` rendered as `surface-muted`
sub-region overlays (**[PROPOSAL]** token mapping — no buffer color exists in
the §1.4 exports). No ARIA is assigned to them: doc 08 §4.5 gives the slider
role to the bar itself, nothing to buffer sub-regions. The component never
invents buffer data.

## VolumeSlider

Vertical, stroke-based anatomy per volume.svg (§1.5): 2px `surface-muted`
stroke track, `accent` fill, 8×24 rx4 `accent-strong` thumb (EQ thumb, §1.4).
Range 0…1, step 0.01, `aria-valuetext` = percent. Mute/mute-pending is the
later `VolumeControl` composite's concern (doc 08 §4.3/§5.5) — the icon +
flyout chrome is not part of this primitive.

## Structural values without tokens (raw px in slider.css)

`8`/`12`/`16`/`24`/`28` px track & knob dimensions, `2px` fill inset, `2px`
stroke track, `2px` focus ring width/offset, `160px` default extent — all
pinned by the §1.4 design exports or inherited from the B1 focus-ring
convention; no semantic token exists for them (token gaps logged in
docs/08 §3).
