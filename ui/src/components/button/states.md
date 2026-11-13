# Button family state table (card 4.B1, doc 08 §5.5; W26 Figma restyle)

Provenance: W26 frames (`design/figma/*.svg`, w26.a03 lane log) + doc 08 §4.4
interim states. Enforced by the colocated interaction tests (CSS-source →
semantic-token assertions + DOM snapshots).

## Button

| State | Visual | Mechanism |
|---|---|---|
| rest | secondary look: `--color-surface-raised` fill + 1px `--color-stroke-border`, `--radius-control` (r6), label `--color-content-secondary`, 12–13px medium (`--font-size-body`) | class `lumen-button` |
| hover | `--color-surface-hover` fill, label `--color-content-bright` | `:hover` |
| pressed | `--color-surface-selected` | `:active` |
| focus-visible | 2px `--color-accent` ring, 2px offset | `:focus-visible` |
| disabled | label `--color-content-tertiary` on `--color-surface-muted` well + `--color-stroke-border`; `pointer-events:none`; native `disabled` ⇒ no handlers | `:disabled` |
| loading/pending | `--opacity-emphasis` + CSS spinner glyph + `aria-busy="true"`; inert | `[aria-busy]` |

Variants (W26):

- `default` / `secondary` (alias, no modifier class) — dialog "Cancel"/
  "Browse": raised + border, r6 (frames: 100x32 r6, fill #1B1B1B, label
  #8F8F8F — `Open network stream.svg`, `Keyboard Shortcuts.svg`,
  `new playlist.svg`, `Settings - global.svg`).
- `primary` — accent fill + `--color-content-on-accent` (dark) glyph, r6
  (`Onboarding.svg` "Next" 100x32 r6 #D37918 + #161616; `play.svg` pill
  54x28 r6). Hover `--color-accent-bright`, pressed `--color-accent-strong`
  (spec §1 accent roles). NO frame shows an r30 button pill (r30 = onboarding
  swatch ring art only) — every variant is r6.
- `tinted` — dialog confirm ("Save"): `--color-accent-tinted` fill +
  `--color-accent` label (`#371F06` + `#D37918` in every dialog frame);
  hover/pressed walk the label accent-bright/strong.
- `ghost` — transparent + 1px transparent border (no layout shift vs the
  bordered secondary), hover `--color-surface-hover`.

Sizes: 28 / 32 / 36 (`sm`/`md`/`lg`) — compact desktop heights per frames
(play pill 28, dialog buttons 32, segmented/footer 36–38). The old 48px `lg`
is retired; 48 survives only as the IconButton `lg` hit wrapper.

Nested icon glyphs: icon.css paints `.lumen-icon` from `--color-stroke-icon`,
so the button-level label color cannot reach a nested glyph. `button.css`
remaps the token in-subtree so glyphs follow the label token at every state
(rest secondary → hover/active/focus bright; primary = on-accent DARK at every
state per play.svg/Controls.svg `#161616` triangle on `#D37918`; tinted =
accent→bright→strong; disabled = tertiary). Without the remap a primary play
pill renders a GRAY glyph (caught in the w26.a03 relaunch vision loop, fixed
2026-11-11; regression-tested in Button.test.tsx).

## IconButton — ghost icon action

Transparent rest + 1px transparent border, `--radius-control`; glyph
`--color-stroke-icon` (Icon defaults to 16px); hover `--color-surface-hover`
chip (Icon Wrapper.svg: white 6% ≈ #262626) + bright glyph; pressed
`--color-accent-icon` (Icon.svg active glyphs run accent); focus-visible 2px
accent ring; disabled = `--color-content-tertiary` (the `.lumen-icon`
subtree is reached by remapping `--color-stroke-icon` locally). Hit areas
28/32/48 (`sm`/`md`/`lg` — all ≥ 28px; `lg` = the ~48px Icon Wrapper).
Carries `lumen-icon-host` so nested `.lumen-icon` glyphs pick up icon.css
state tints. `aria-label` mandatory (§4.5).

## Toggle (w26.k01: the shared switch core)

| State | Visual | Mechanism |
|---|---|---|
| off | knob 26x26 rx3 `--color-stroke-border` (#404040) left + 33x26 rx3 `--color-surface-panel` segment (#191919) with "OFF" micro label | `aria-checked="false"` |
| on | fill 33x26 rx3 `--color-accent-tinted` (#371F06) with "ON" micro label (`--color-accent`) + knob 26x26 rx3 `--color-accent-strong` (#AB6314) right | `aria-checked="true"` |
| hover | track `--color-surface-hover`; the knob walks too (accent-bright when on, stroke-border mix when off — the w26.a06 form-port hover, shared by both grammars since w26.k01) | `:hover` |
| focus-visible | 2px accent ring, 2px offset | `:focus-visible` |
| disabled | `--opacity-disabled`, inert, state still rendered truthfully | `:disabled` |

Track: 68x32 rx6 `--color-surface` (#090909 per Toggle.svg — was surface-deep
pre-W26), 3px padding/radius/pitch (frame-measured structural px, no token).

**j01-3 (w26.k01, Lead ruling):** ONE core implementation —
`switchCore.tsx` — now backs BOTH exported names: this `Toggle`
(`.lumen-toggle*` grammar, control-parts lane) and the form family's
`Switch` (`.lumen-switch*` grammar, settings rows — screens keep CSS hooks on
that set, so both class grammars ship). `button.css` styles the two through
shared selector lists (one rule, both sets), which is what closed j01-9:
the form copy's OFF label had drifted to tertiary (4.57:1) while this one
kept the 4.F4 secondary fix; both are secondary now, in ONE rule.

Micro labels (doc 08 §1.3 row 4.4 + §1.5 Toggle.svg): `--font-size-caption`
(11px; frame-measured ON/OFF cap-height 9.0px ⇒ ~12px type — the doc-08
"micro" 6px was half-size, fixed in the w26.a03 relaunch), ON =
`--color-accent` (#D37918), OFF = `--color-content-secondary` — the frame's
#6B6B6B tertiary is 3.4:1 on the #191919 segment; secondary clears 4.5:1
(4.F4 a11y deviation, kept; ported to the form grammar in w26.k01).
Decorative (`aria-hidden`): the switch conveys state via `aria-checked`
alone. **j01-4 (w26.k01): the label TEXT is localized** — `useSwitchLabels()`
resolves the `switch.on` / `switch.off` msgids (lane file
`ui/src/i18n/keys/w26.k01.ts` until the Lead registers them); the old
"decorative artwork, not translated" allowlist entries are removed, so a
regression to literals now fails `local/i18n-no-literals` (probe-verified).

ARIA note: the task card mentioned `aria-pressed`, but doc 08 §4.5 mandates
`toggles → switch`; implemented as `role="switch"` + `aria-checked` (a switch
carries checked state, not pressed). Keyboard: native `<button>` gives
Space/Enter activation.

§4.4 audit (closure, 2026-10-24): rest/hover/pressed/focus-visible/disabled/
checked all present. Pending: not required for Toggle (§4.4 scopes it to
command-dispatching components).

## Honest gaps (w26.a03 relaunch state)

- Visual snapshots: 4.G1's harness doesn't exist; vitest stubs `.css` imports
  (`?raw` returns EMPTY — use `node:fs` + `import.meta.dirname`), so jsdom
  never cascades the stylesheet. Current "visual snapshot" = DOM snapshot +
  CSS-source→semantic-token assertions. (The comparison loop runs against a
  `/tmp` static harness + live routes — see the lane log.)
- Toggle 3px radii/pitch are raw px — no 3px semantic radius/space token
  exists; handoff to the token owner (themes/tokens.json is Rust-owned).
- RESOLVED (a01 landed): `--radius-control` = 6px and the Inter font stack
  now resolve to the W25 values; button labels measure ~14px in the dialog
  frames (cap-height 10.5px), matching `--font-size-body` — the "12–13px"
  task-card figure is approximated by the frame at ~14px, kept on the token.
- `form/Switch.tsx` is UNIFIED with this family since w26.k01 (j01-3): it is
  the same `switchCore.tsx` in the `.lumen-switch*` class grammar; the skin
  is the shared selector-list block in this sheet (one rule styles both
  grammars — the j01-9 OFF-ink divergence is structurally impossible now).
  The task card's "32×16 settings toggle" does not exist in the frames —
  every settings toggle measures 68×32 (Settings - global.svg /
  playback.svg).
- IconButton `loading` (w26.k01 j01-12): the pending cue is REAL now — a
  14px currentcolor spinner ring (`::before`) replaces the glyph
  (visibility-hidden, no chip resize), motion off under
  prefers-reduced-motion; CSS-source-tested in IconButton.test.tsx (no route
  renders a loading IconButton yet, so the proof is the CSS-source
  assertion, not a screenshot).
- Frame residue, documented (not local fixes): dialog-frame secondary button
  fill is #1B1B1B with no border (this lane ships raised #222222 + 1px
  stroke-border per the task card); `Button.svg`'s 360×32 bars are rx4 with
  surface/surface-panel fills and tertiary/secondary labels — that anatomy
  is the settings "Advanced Options" footer composition (consumer CSS), not
  a primitive variant; dialog frames pin button width at 100px (consumers
  should set width/min-width — the primitive stays auto-width).
