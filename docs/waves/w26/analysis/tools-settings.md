# W26 · Screen anatomy spec — AUDIO/VIDEO TOOLS + SETTINGS

Task `w26.a20` (analysis lane). For wave-b/wave-c implementers building the
audio/video tool panels and the 13-area Settings window.

**Method**: SVG XML in `design/figma/` is ground truth (parsed: every rect,
path bbox, fill/stroke, filter, gradient, opacity). PNG renders
(`design/figma-renders/`, 900px-wide ≈ 2.26× for 400px frames) + `z-ai vision`
used to confirm labels (EQ preset names, HDR modes, sidebar category names,
"FM5 / Peak"). **Where SVG and the W25 spec §2 conflict, SVG wins** (conflicts
flagged inline). Frames are native-scale component exports — the artboard
stroke-scaling factors (0.825/1.125/1.28571, `themes/tokens.json` header) do
not apply, except one deliberate fine stroke: ⓘ info circles use
`stroke-width="1.125"` (see §C0). Purple `#8A38F5` dashed 0.5px-inset
annotation frames (rx 4.5) are export chrome — exclude.

**Font-size derivation**: Figma text is outlined. Bbox heights include
descenders and Bezier-control-point overshoot (+1–2px); sizes below are
cap-corrected (÷0.727, Inter cap ratio) and cross-checked against glyph-count
widths. Reported as `≈Npx`. Weights [VERIFY] (outlined shapes can't prove
400 vs 500; W25 spec says medium for labels).

**Corrections vs W25 spec §2 + task brief** (SVG wins):
1. Tool card radius is **r6, NOT r8** (every `Audio -*`/`Video -*` card: `rx="6"`).
2. Tool slider track is **328×16 rx6**, NOT "14px track r7".
3. Slider fill is **flat #371F06**, NOT a gradient `#371f06→#a5621a`.
4. Slider handle is **#AB6314** (accent.strong) 8×24 rx4, NOT amber #a5621a.
5. EQ value chip = **138×34 rx5 #090909 + 2px #191919 stroke**, value
   **#8F8F8F ≈12px**, NOT "r6 dark #141414, white value". (The graphic-EQ
   variant chip is 60×32 rx6 flat #090909.)
6. Settings top bar is **50px** (not 36–40); search pill 200×34 rx6 #151515.
7. Settings content column is **784px** (x240–1024 in a 1048-wide window),
   not ~640; controls are 320px wide right-aligned at x1024.
8. Settings section headers measure **≈16–17px #C6C6C6** (raw cap 14.9–15.6
   incl. overshoot), not 15px.
9. "Advanced Options ›" is a **disclosure divider in the content flow**
   (chevron rotates ›→∨), not a pinned footer — advanced sections render
   BELOW it (`global-1`, `performance`, `recording` evidence).
10. Settings renders as a **floating window** 1048×865 rx14 #0E0E0E@95% +
    backdrop-blur + shadow 0/+8 σ7.5 — an overlay, not a full-bleed route.

**Color palette observed** (all in the W25 token set unless noted):
`#090909 #0E0E0E #151515 #191919 #1A1611 #1B1B1B #1F1F1F #222222 #371F06
#404040 #6B6B6B #8F8F8F #969696 #AB6314 #C6C6C6 #D37918 #EAEAEA` + accent
swatch set `#D37918 #E15032 #C49E23 #19A234 #18AAD3 #185DD3 #8E42FF #F052B3
#AFAFAF` and color-picker values `#FF0000`/`#FFFFFF` (data, not theme).
`#1A1611` (slider degree ticks) and `#969696` (row labels) are NOT in
`themes/tokens.json` yet (a19 also flagged #969696).

---

# PART A — AUDIO TOOLS (8 frames)

## A1. `Audio - Equalizer` (400×1623) — 10-band EQ, chip-value rows

Two stacked states (annotation inset 20 → real card 360 wide):

```
Property 1=collapsed
├─ card 360×48 @ (20,20) · rx6 · fill #090909        ← darker than expanded!
├─ label "Expand Settings" #6B6B6B ≈14px @ (37.2,39) · 17px left inset,
│  v-centered in 48px row
└─ chevron-RIGHT 5×10 · stroke 1.5 · #6B6B6B @ (356,39) · 24px from right
   edge (380), v-centered (y39–49)

Property 1=expanded
├─ card 360×1048 @ (20,88) · rx6 · fill #1B1B1B
├─ header: "Collapse Settings" #6B6B6B ≈14px @ (36.8,107) + chevron-DOWN
│  10×5 sw1.5 #6B6B6B @ (353,110) · header block = 59px (card top→row1 y147)
└─ 11 rows (Preamp + 10 bands) · pitch 90 · chip rows
   row N (y147 + 90·N):
   ├─ label  #6B6B6B ≈14–15px @ x36.9 (17px inset) — "Preamp", "60 Hz",
   │  "170 Hz", "310 Hz", "600 Hz", "1 KHz", "3 KHz", "6 KHz", "12 KHz",
   │  "14 KHz", "16 KHz"
   ├─ value chip 138×34 · rx5 · fill #090909 + stroke #191919 sw2
   │  @ x225–363 (17px right inset) · value #8F8F8F ≈12px @ 13.7px inset
   │  (Preamp "12.0 dB", bands "0.0 dB")
   └─ slider 15px below chip top: track 328×16 rx6 #090909 @ x36 (16px
      insets both sides) · fill 12px tall rx4 #371F06 inset 2 · thumb 8×24
      rx4 #AB6314 at fill end (±4px overhang)
      Preamp: fill 271w → thumb x301 · bands: fill 166w (0 dB ≈ 50.6% of
      328) → thumb x196 · NO degree ticks in this frame
```

**Tokens needed**: `tool.card.collapsed(360×48 r6 #090909)`,
`tool.card.expanded(#1B1B1B r6)`, `tool.card.header(59)`, `tool.row.pitch(90)`,
`tool.valuechip(138×34 r5 #090909 + 2px #191919)`, `tool.value(#8F8F8F ≈12)`,
`slider.track(328×16 r6 #090909)`, `slider.fill(#371F06 12h r4 inset2)`,
`slider.thumb(8×24 r4 #AB6314)`, `tool.chevron(5×10 / 10×5, sw1.5 #6B6B6B)`.

**Notes**: (1) collapsed and expanded cards have DIFFERENT fills — #090909
recessed vs #1B1B1B panel. (2) Card height = `59 (header) + (N−1)·90 +
69 (last chip row) + 20 (bottom pad)` = 1048 for N=11, exact. (3) The two
states are stacked in the frame 20/68→88; treat as one component's two
states.

## A2. `EQ` (824×628) — graphic EQ, vertical faders

```
Property 1=Default (collapsed bar)
├─ bar 784×36 @ (20,20) · rx2 (square-ish!) · fill #090909
├─ "Open Equalizer" #8F8F8F ≈14px @ (346,33) + chevron-DOWN 10×5 sw1.5
│  #8F8F8F @ (462,36) · text+chevron centered as a group (gap 17px,
│  group center 409 ≈ bar center 412)

Property 1=Variant2 (expanded)
├─ panel 784×489 @ (20,76) · rx2 · fill #191919
├─ header "Close Equalizer" #8F8F8F ≈14px @ (345.1,89) + chevron-DOWN
│  10×5 @ (463,92) (chevron stays DOWN in both states here)
└─ 11 vertical faders · column pitch 68 · first track center x72
   (tracks x64,132,200,268,336,404,472,540,608,676,744)
   fader anatomy (rotated −90°):
   ├─ track 16 wide × 320 tall · rx6 · #090909 · y130–450
   ├─ fill 12 wide · rx4 · #371F06 · 2px inset · grows UP from bottom y448
   │  (Preamp 258 tall, bands 158 tall = 0 dB ≈ 50.6%)
   ├─ thumb 24×8 · rx4 · #AB6314 · horizontal bar at fill top edge,
   │  4px overhang beyond track sides
   ├─ 0 dB degree tick 6px line · #1A1611 · sw2 (bands only, sits at value)
   ├─ label #6B6B6B ≈14–15px centered under fader @ y486.7 ("Preamp",
   │  "60 Hz" … "16 KHz")
   └─ value chip 60×32 · rx6 · #090909 @ y513 · value #8F8F8F ≈12px
      centered ("12.0 dB" / "0.0 dB")
```

**Tokens needed**: `eq.bar.collapsed(784×36 r2 #090909)`, `eq.panel(784×489
r2 #191919)`, `eq.fader.pitch(68)`, `eq.fader.track(16×320 r6)`,
`eq.fader.fill(#371F06 12w r4)`, `eq.fader.thumb(24×8 r4 #AB6314)`,
`eq.chip(60×32 r6 #090909)`, `eq.tick(#1A1611)`.

**Notes**: (1) This is the compact bar + graphic panel variant (rx2!), a
DIFFERENT family from A1's 360px cards (rx6) — do not merge radii. (2) The
same component is embedded full-width in Settings→Audio→Effects (see C6:
784×489 @ x240) — build it as a reusable `<GraphicEQ>` with width 100%.

## A3. `EQ presets` (406×460) — preset dropdown (Dropdown grammar)

```
state=default (closed trigger)
├─ trigger 362×48 @ (22,22) · rx6 · fill #090909
├─ inner 358×44 @ (24,24) · rx4 · fill #191919 (2px "border" effect)
├─ label "None" #8F8F8F ≈15px @ (39.2,41.3) · 15px inset
└─ chevron-DOWN 10×5 sw1.5 #8F8F8F @ (351,44) · 23px inset

state=open
├─ list 362×348 @ (22,90) · rx6 · fill #191919 · shadow 0/+4y σ6.9 black
├─ 8 rows 358×44 · rx4 · 2px inset · 46px pitch (y92,138,184,230,276,
│  322,368,414)
│  ├─ row 1 selected: fill #1F1F1F + check 14×11 sw2 #D37918 @ (349,108)
│  │  (19px from row right) + label #8F8F8F
│  └─ rows 2–8: leading icon 22px wide @ x39–61 (EQ-curve preview: gray
│     baseline #8F8F8F sw2 + orange #D37918 bump fills) + label
│     #8F8F8F ≈15px @ x72.8–73.2
└─ labels (vision-confirmed): None / Flat / Classical / Club / Dance /
   Full bass [VERIFY — render may read "Full base"] / Full bass & treble /
   Full treble (row 8 overflows the 348px list → clipped; scrolling implied)
```

**Tokens needed**: reuses a19 A9 `Dropdown` grammar verbatim
(`dropdown.trigger(362×48 r6 #090909/#191919)`, `dropdown.row(44 r4 pitch46)`,
`dropdown.check(14×11 sw2 #D37918)`, `dropdown.shadow(0 4 6.9 / .35)`).

**Notes**: list height should be computed `2 + 44·n + 2·(n−1)`; the frame's
348px rect clips row 8 — implement as a scrollable list, don't copy 348.

## A4. `Audio - Compressor Settings` (400×821) — 7 rows, plain-text values

Same card shell as A1 (collapsed 360×48 rx6 #090909 + "Expand Settings";
expanded 360×**590** rx6 #1B1B1B + "Collapse Settings" header 59px). Rows are
**plain-value** (no chip), **pitch 76**:

```
row N (label y149.3 + 76·N):
├─ label  #6B6B6B ≈15px @ x36.4–37.2
├─ value  #6B6B6B ≈15px · right-aligned to x≈362.5 (17.5px right inset)
├─ slider 33px below label: track 328×16 rx6 #090909 @ x36 · fill 12 rx4
│  #371F06 inset2 · thumb 8×24 rx4 #AB6314 at fill end · NO degree ticks
rows: FM5 / Peak [sic — design typo for "RMS / Peak"; vision reads
      "FM5 / Peak" verbatim] = 0.2 (fill 53)
      Attack       = 25.0 ms    (fill 30)
      Release      = 100.0 ms   (fill 56)
      Threshold    = -11.0 dB   (fill 207)
      Ratio        = 4.0 : 1    (fill 104)
      Knee Radius  = 5.0 dB     (fill 162)
      Makeup Gain  = 7.0 dB     (fill 110)
```

**Tokens needed**: `tool.row.pitch(76)`, `tool.value.plain(#6B6B6B ≈15px
right-aligned 17.5px inset)` — rest shared with A1.

**Notes**: two value styles exist in the tool family — CHIP (A1/A2) vs PLAIN
TEXT (A4/A5/A6, B1/B3). Value color differs too (#8F8F8F in chips, #6B6B6B
plain — plain matches the label color).

## A5. `Audio - Spatializer Settings` (400×821) — 5 rows + degree ticks

Card 360×**438** rx6 #1B1B1B, header 59px, rows pitch 76, plain values:

```
Size  = 8.0  (fill 240 · thumb x270) · 10 ticks pitch 29.45 (x67.5–332.5)
Width = 10.0 (fill 324 · thumb x354) ·  9 ticks pitch 32.4  (x70.4–329.6)
Wet   = 4.0  (fill 134 · thumb x164) ·  9 ticks
Dry   = 5.0  (fill 166 · thumb x196) ·  9 ticks
Damp  = 5.0  (fill 166 · thumb x196) ·  9 ticks
ticks: 6px vertical lines · #1A1611 · sw2 · round caps · on track centerline
```

**Tokens needed**: `slider.tick(6px #1A1611 sw2)` + shared.

**Notes**: tick counts/pitches vary per row (10×29.45 in row 1 vs 9×32.4 in
rows 2–5) — ticks are data-driven degree markers, not a fixed grid.

## A6. `Audio - Stereo Widener` (400×821) — 4 rows

Card 360×**362** rx6 #1B1B1B, header 59px, rows pitch 76, plain values:

```
Delay Time     = 20.0 ms (fill 89  · thumb x119) · no ticks
Feedback Gain  = 0.2 %   (fill 72  · thumb x106) · 8 ticks pitch 36 (74–326)
Crossfeed      = 0.3 %   (fill 122 · thumb x156) · 7 ticks pitch 40.5 (78.5–321.5)
Dry Mix        = 0.8 %   (fill 263 · thumb x293) · 9 ticks pitch 32.4
```

**Tokens needed**: none beyond shared.

## A7. `Audio Visualizer` (406×369) — visualizer dropdown, 5 options

Exact `Dropdown` component grammar (a19 A9) — identical to A3 but 5 rows and
two-tone leading icons:

```
trigger: 362×48 rx6 #090909 + inner 358×44 rx4 #191919 · label "Disabled"
  #8F8F8F ≈15px @ (39.2,41) · chevron-DOWN 10×5 @ (351,44)
open list: 362×232 rx6 #191919 · shadow 0/+4 σ6.9 · height = 2+5×44+4×2 ✓
rows (46 pitch, y92…276):
 1 Disabled   selected #1F1F1F + check 14×11 sw2 #D37918 @ (349,108)
 2 Spectometer [sic]  icon: concentric circles+arcs 19×20 two-tone
 3 Scope              icon: 6 vertical bars h5–16 two-tone #8F8F8F/#AB614
 4 Spectrum           icon: 6 bars + orange dots above
 5 Waveform           icon: 6 bars mixed two-tone
 icons 20px wide @ x40–61 · sw1.5 · labels #8F8F8F ≈15px @ x72.3–73.2
```

**Tokens needed**: none beyond a19 A9/B13 (two-tone icon grammar).

**Notes**: "Spectometer" is the design's spelling (VLC uses "Spectrometer") —
keep i18n key values correct, don't copy the typo [VERIFY with Lead].

## A8. `audio value` (121×37) — floating value chip

```
chip 121×37 · rx6 · fill #0E0E0E @95% · backdrop blur + drop shadow 0/+8 σ7.5
     (menu-panel family)
content centered (x41.4–78.6): "100" #969696 ≈15px + 6px gap + "%" #6B6B6B
```

**Tokens needed**: `chip.value(121×37 r6 #0E0E0E@.95)`,
`chip.value.number(#969696)`, `chip.value.unit(#6B6B6B)`.

**Notes**: the volume/level readout that floats over the player (menu-panel
surface grammar, NOT the #090909 field grammar). Number and unit are
different colors.

---

# PART B — VIDEO TOOLS (4 frames)

## B1. `Video - Colors` (400×888) — richest card: inputs, toggles, sliders, sub-dropdown

Card shell shared (collapsed 360×48 rx6 #090909; expanded 360×**755** rx6
#1B1B1B, header 59px). Structure top→bottom (y = label baseline block):

```
Color Extraction   y153  label #6B6B6B ≈15px @ x36.8
                   hex input 116×32 rx6 #090909 @ x172 · text "#FF0000"
                   #6B6B6B ≈15px @ 12.2px inset (x184.2) · color swatch
                   22×22 rx2 fill #FF0000 @ x261 (5px from input right edge
                   288) + inner white@21% 1.5sw stroke inset 0.75 ·
                   Toggle OFF 68×32 @ x296 (16px right inset)
—— separator 1px #222222 · x20→380 (full card width) @ y194 ——
Color Threshold    y225  same hex+swatch+toggle row (y214)
Saturation         y265.5 label + slider y298 (fill 66 → thumb x100)
Similarity         y341.5 label + slider y374 (fill 66) · group clip 328×66
—— separator @ y414 ——
Negate Colors      y445.2 label + Toggle OFF (y434)
—— separator @ y476 ——
Posterize          y497   label + Toggle OFF (y486)
—— separator @ y528 ——
Gradient           y554   label + Toggle OFF (y543)
  sub-dropdown     328×48 rx6 #0E0E0E @ (36,590) + inner 324×44 rx4 #191919
                   (inset 2) · label "Gradient" #6B6B6B ≈15px @ 16.8px inset
                   (x54.8) · chevron-DOWN 10×5 #8F8F8F @ x333 (27px inset)
Color              y664.2 label + Toggle OFF (y653)
Cartoon            y711.2 label + Toggle OFF (y700)
—— separator @ y747 ——
Sepia Intensity    y770.5 label + slider y803 (fill 112 → thumb x142) ·
                   2 ticks @ x146/x254
```

**Tokens needed**: `tool.hexinput(116×32 r6 #090909)`,
`tool.swatch(22×22 r2 + white@.21 inner stroke)`, `tool.separator(1px #222222
full-width)`, `tool.subdropdown(328×48 r6 #0E0E0E/#191919)` + shared
(toggle = a19 B3, slider = A1).

**Notes**: (1) sub-dropdown outer is **#0E0E0E** here (vs #090909 in A3/A7/
Settings) — one-off, flag for token unification. (2) Toggle rows are
label-left + toggle-right (x296) with NO sliders. (3) Separators are the
sectioning device inside tool cards (no section header text).

## B2. `Video - Crop` (400×821) — 2×2 number-input grid

Card 360×**152** rx6 #1B1B1B (smallest), header 59px, then 2 rows of paired
inputs, **pitch 49**:

```
row 1 y142–175:  [Top]     input 141×33 rx6 #090909 @ x36
                 [Bottom]  input 141×33 rx6 #090909 @ x185 (8px gutter)
row 2 y191–224:  [Left]    @ x36   ·  [Right] @ x185
input anatomy: label inside-left #6B6B6B ≈15px @ 12.7px inset · value "0"
  #8F8F8F ≈15px + unit "px" #404040 ≈15px (value+unit group ends 41px from
  input right edge [VERIFY — reserved trailing area])
row icons: link/chain icon 20×10 · stroke 1.5 · #8F8F8F @ x339 (21px right
  inset), one per ROW (visually couples Top↔Bottom and Left↔Right)
```

**Tokens needed**: `tool.numinput(141×33 r6 #090909)`,
`tool.numinput.unit(#404040)`, `crop.linkicon(20×10 sw1.5 #8F8F8F)`.

**Notes**: card height = 59 header + 2×33 rows @49 pitch + 16 bottom pad.
The link icon is a row-level toggle (chain = values locked together).

## B3. `Video - Essential` (400×821) — 8 sliders, NO values

Card 360×**666** rx6 #1B1B1B, header 59px, rows pitch 76, **label-only**
(no value text, no chips):

```
Hue                     y149.8 · fill 166 → thumb x196 · 1 tick @ x200
Brightness              y225.5 · fill 166 · 1 tick @ x200
Contrast                y301.7 · fill 166 · 1 tick @ x200
Saturation              y377.5 · fill 112 → thumb x142 · 2 ticks 146/254
Gamma                   y453.7 · fill 36  → thumb x66  · 9 ticks pitch 32.4
Sharpen Sigma           y529.5 · fill 8   → thumb x38  · 3 ticks 119/200/281
Banding Removal Radius  y605.5 · fill 143 → thumb x173 · 6 ticks pitch 46.3
Film Grain Variance     y681.5 · fill 69  → thumb x99  · 9 ticks pitch 32.4
```

**Tokens needed**: none beyond shared.

**Notes**: same rows as A4 but the value column is omitted entirely — the
slider IS the display. (Settings→Video→Picture repeats Hue/Brightness/
Contrast/Saturation/Gamma — see C5; keep the row component shared.)

## B4. `HDR Options` (406×422) — HDR-mode dropdown, 6 options, no icons

`Dropdown` grammar (a19 A9): trigger 362×48 rx6 #090909 + inner 358×44 rx4
#191919, label **"Auto"** #8F8F8F ≈15px @ (38.4,41.3), chevron-DOWN 10×5
@ (351,44). Open list 362×278 rx6 #191919 + shadow 0/+4 σ6.9;
height = 2 + 6×44 + 5×2 ✓. Rows (46 pitch): row 1 "Auto" selected #1F1F1F +
check 14×11 sw2 #D37918; rows 2–6 **plain labels, NO leading icons**:
Off · HDR10 · HDR10+ · Dolby Vision · HLG (vision-confirmed).

**Notes**: this is the only tool dropdown without leading icons — icon slot
is optional in the row component.

---

# PART C — SETTINGS (19 frames, 13 areas)

## C0. Shared shell (identical in all 19 frames)

```
window 1048×865 · rx14 · fill #0E0E0E @95% · backdrop blur + shadow 0/+8 σ7.5
├─ top bar 1048×50 · fill #090909
│  ├─ search pill 200×34 · rx6 · fill #151515 @ (8,8)
│  │  ├─ magnifier 15×15 · stroke 1.5 · #6B6B6B @ (20,17)
│  │  └─ placeholder "Search" #404040 ≈15px @ (50.8,20.2)
│  ├─ window title #969696 ≈16px @ (240.4–241.4, 18.3) — per-area strings
│  │  (see C1–C13), left-aligned at content column x240
│  └─ close ✕ 10×10 · stroke 1.5 · #8F8F8F @ (1013,20) · 25px inset
├─ sidebar 216 wide (x0–216)
│  ├─ panel 200×799 · rx6 · fill #191919 @95% @ (8,58)
│  ├─ group labels #6B6B6B ≈14–15px @ x24.4–25.2 (16px inset):
│  │  App y75.8 · Media y308.5 · System y589.7 (sentence case)
│  └─ 13 category rows · 188×40 · rx2 @ x14 · 48px pitch
│     tops y105,153,201,249 | 338,386,434,482,530 | 619,667,715,763
│     ├─ icon ~20×20 @ x25–45 · stroke 1.5 #8F8F8F (selected: #D37918 sw2)
│     ├─ label ≈15px @ x58.4–59.2 · #8F8F8F (selected: #EAEAEA)
│     └─ selected row: fill #222222 · r2 (a19 A8 grammar)
│     App:    General(global) · Interface · Privacy · Controls
│     Media:  Playback · Video · Audio · Subtitles · Library
│     System: Network · Performance · Recording · Extensions
├─ content viewport 832×815 @ (216,50) · clips + scrolls
│  └─ content column x240–1024 (784 wide)
│     ├─ section headers #C6C6C6 ≈16–17px @ x240.4–241.3 · first at y83
│     │  (33px below top bar) · header→first row 42–43 · section gap ~48
│     ├─ rows · pitch 42 (32px controls) / 46–47 (36px controls):
│     │  label #8F8F8F ≈15px @ x240.4–241.3 + optional ⓘ circle r7.5
│     │  stroke #404040 sw1.125 + i-glyph sw1.5 (~13px after label end)
│     │  control right-aligned to x1024 (320 wide @ x704)
│     └─ "Advanced Options" disclosure bar (see below)
```

**Row control inventory (all settings frames)**:

| Control | Anatomy |
|---|---|
| Dropdown | 320×36 rx6 #090909 + inner 316×32 rx4 #191919 · label #8F8F8F ≈15px @ 15.2px inset (x720.4–721.2) · chevron-DOWN 10×5 #8F8F8F @ x991 (33px inset) |
| Toggle | 68×32 @ x956 (right-aligned) · a19 B3 grammar (ON #371F06+#AB6314+#D37918 / OFF #191919+#404040+#6B6B6B) |
| Text/number input | 318×34 rx5 #090909 + 2px #191919 stroke · value #8F8F8F ≈15px @ 13.9px inset (x718.8–719.1) — full-width variant; or 208×34 @ x705 + Browse button 100×32 rx6 #191919 (#8F8F8F label) @ x924 |
| Full-width action | 320×32 rx6 #191919 @ x704 · centered label #8F8F8F ≈15px ("Edit Keyboard Shortcuts", "Reset", "Rescan") |
| Small action | 100×32 rx6 #191919 (#8F8F8F) @ x924 ("Browse", "Clear", "Add", "Remove") |
| Segmented (Values) | 320×36 rx6 #090909 · 2×157 or 3×104 cells rx4 · 2px gaps · active #371F06+#D37918 · inactive #191919+#6B6B6B (a19 B4) |
| Radio rows w/ input | 320×78 rx6 #090909 · rows 316×36 rx4 flush 2px gaps · selected #371F06+#D37918 · unselected #191919+#6B6B6B + nested input 182×32 rx2 #090909 · disabled nested input @ opacity 0.4 (a19 B5) |
| Accent swatches | 320×36 rx6 #090909 · 9 swatches 19.556×20 rx1 @ 35.556 pitch · unselected @ opacity 0.7 · selected full + #371F06 cell 31.556×32 rx4 behind · colors #D37918 #E15032 #C49E23 #19A234 #18AAD3 #185DD3 #8E42FF #F052B3 #AFAFAF |
| Color input | 320×32 rx6 #090909 · hex "#FFFFFF"/"#000000" #8F8F8F ≈15px @ 12.2px inset · swatch 22×22 rx2 @ x997 (5px from right edge 1024) + white@.21 inner stroke |
| Slider | 320×16 rx6 #090909 @ x704 · fill 12 rx4 #371F06 inset2 · thumb 8×24 rx4 #AB6314 · ticks 6px #1A1611 sw2 · scale labels ≈12px #6B6B6B under slider (left/center/mid right-aligned) |
| Slider layouts | (a) INLINE: label left @ x240 on slider centerline (video Picture); (b) LABELED+SCALE: slider top, label below-left, scale labels under slider (playback speed, interface hide-after) — block ≈ 42–62px |
| Disabled row | whole row group @ opacity 0.3 (library/network) |
| Extensions list | 782×192 rx5 #090909 + 2px #191919 stroke @ x241 (empty in frame) |

**Advanced Options bar** (in 8 of 13 frames): 784×38 · rx2 · fill #191919
@95% · @ x240, in content flow after the last regular section (not pinned);
"Advanced Options" #8F8F8F ≈15px + chevron 5×10 (collapsed ›) / 10×5
(expanded ∨) sw1.5 #8F8F8F, text+chevron centered as a group (text x554.4–
675.2, chevron x696). Expanded state reveals advanced rows/sections below
(global-1, performance "Developer"; recording/network/etc. ship collapsed).

## C1. `Settings - global` (+`-1`) — "Global Settings"

Sections/rows (label → control):
- **Appearance**: Theme → segmented 3 ("1" selected #D37918 / "2" / "3"
  [VERIFY labels: light/dark/system]) · Accent color → 9 swatches (orange
  selected) · Interface language → dropdown "English".
- **Startup** y280: Show confirmation and error dialogs → OFF · Check for
  updates automatically → OFF.
- **Diagnostics** y423: Developer mode → OFF · Save a log → ON · Log file →
  input 208×34 + Browse · Log details → dropdown "Errors only".
- **Advanced Options bar** y645 (collapsed: chevron ›).
- `global-1` = advanced EXPANDED (chevron ∨), rows below the bar: Show minor
  warnings → OFF · Additional command line options → input 318×34 (no
  Browse) · Log format → dropdown "Plain text" · Collect playback
  statistics → ON. Shell identical.

**Notes**: row pitch 46.5 in Appearance (36px controls), 42 in toggles.
Figma "global/General" OWNS Appearance — today's UI puts theme under
`interface` (see Part D3 mapping).

## C2. `Settings - interface` (+`-1`) — "Interface Settings"

- **Skin** y83: Use a custom skin → ON · Skin file → input 208×34 + Browse.
- **Window** y230: Play video inside main window → ON · Resize window to fit
  video → dropdown "Never" · Remember window size and position → ON (no ⓘ)
  · Keep window above other windows → OFF · Show window decorations → ON ·
  Show the file name in the title bar → ON · Open fullscreen on → dropdown
  "Current screen".
- **Playback Controls** y591: Start in compact mode → OFF · Auto-hide the
  controls → dropdown "In fullscreen only" · Hide after → slider w/ 19 ticks
  (pitch 15.8) + scale labels "0 s" @ slider-left / "5 s" center / "10s"
  right-aligned (≈12px #6B6B6B) · Show controls in fullscreen → ON.
- **Notifications and System Tray** y837: header only (content below fold).
- `interface-1`: **visually identical** (group-id reorder only) — duplicate
  export, ignore.

## C3. `Settings - privacy` — "Privacy Settings"

- **History** y83: Save recently played items → ON · Remember the playback
  position → ON · Never remember items matching → input 318×34 (empty) ·
  Clear history now → Clear button 100×32.
- **Network** y314 (last.fm): Fetch titles and cover art from the internet →
  OFF · Submit played tracks to last.fm → OFF · Last.fm username → input ·
  Last.fm password [layer id says username ×2] → input · Last.fm submission
  URL → input.
No Advanced bar; no `-1` frame.

## C4. `Settings - playback` (+`-1`) — "Playback Preferences"

- **Startup** y83: Continue where you left off → dropdown "Always" · Start
  playing as soon as a file opens → ON · Default playback speed → slider
  (fill 139/320, 6 ticks pitch 45.15) + 8 scale labels 0.25/0.5/0.75/1/
  1.25/1.5/1.75/2 (≈12px #6B6B6B) · Pause when another app uses audio → ON.
- **Tracks** y339: Preferred video quality → dropdown "Best available" ·
  Preferred menu language → dropdown "English" · Default audio track → input
  "1" · Default subtitle track → input "1".
- **Order** y582: Repeat → dropdown "Off" · Shuffle → ON · Stop after each
  item → ON · When playback finishes → dropdown "Stop".
- **Seeking** y817: Seek precisely → ON.
`playback-1`: identical ±1–2px jitter (row y-shifts only) — duplicate.

## C5. `Settings - video` (+`-1`) — "Video Defaults"

- **Output** y83: Enable video → ON · Start videos in fullscreen → OFF ·
  Stop the screensaver during playback → ON · Play video as desktop
  wallpaper → OFF.
- **Picture** y310 (INLINE slider rows, pitch 42): Hue · Brightness ·
  Contrast (fills 158 → thumb x860) · Saturation (fill 105, 2 ticks) ·
  Gamma (fill 32, 9 ticks pitch 31.6) — labels on slider centerline; then
  Reset picture → Reset button 100×32.
- **Deinterlacing** y621: Deinterlacing → dropdown "Automatic" · Method →
  dropdown "Automatic".
- **Framing** y772: Aspect ratio → dropdown "Automatic" · Crop → dropdown
  "None".
`video-1`: identical (filter-id only) — duplicate.

## C6. `Settings - audio` (+`-1`) — "Audio Defaults"

`audio.svg` (top of area):
- **Output** y83: Enable audio → ON · Output device → dropdown "System
  default" · Speaker setup → dropdown "Automatic".
- **Volume** y276: Maximum volume → slider (fill 159, 7 ticks pitch 39.5) +
  scale labels 100%/200%/300% · Startup volume → radio-rows component
  (320×78: row1 selected #371F06, label #D37918 ≈15px; row2 "Always starts
  at:" #6B6B6B + input 182×32 rx2 "200%" @ 0.4 opacity) · Volume step →
  input "1".
- **Loudness** y548: Normalize loudness → OFF · Target level → input "1" ·
  Night mode → OFF · Replay gain → dropdown "Off" · Replay gain preamp →
  input "0.0".
- **Effects** y833: header + one toggle at the fold (y864).

`audio-1.svg` = **Effects section scrolled/expanded** (the key variant):
```
Effects y71.3
├─ Equalizer           → Toggle OFF @ y102.5 (w/ ⓘ)
├─ Equalizer preset    → dropdown (A3 trigger, 320×36) @ y145.5
├─ graphic EQ panel 784×489 · rx2 · #191919 @ (240,192.5)   ← A2 embedded
│  full content width; 11 vertical faders pitch 68; "Close Equalizer"
│  centered header; labels + 60×32 value chips below
├─ Two-pass            → OFF @ y691.5
├─ Spatial audio       → OFF @ y733.5
├─ Spatial audio mode  → dropdown "Headphone virtualization" @ y775.5
├─ Stereo widening     → OFF @ y821.5
└─ (one more toggle clipped @ y863.5 [VERIFY label])
```

**Notes**: the graphic EQ is a CHILD of Settings→Audio→Effects — one
component reused at 360 (A2) and 784 (audio-1) widths.

## C7. `Settings - subtitles` (+`-1`) — "Subtitle Defaults"

- **Tracks & Languages** y83: Enable subtitles → ON · Preferred language →
  dropdown "System language" · Text encoding → dropdown "Automatic".
- **Loading Subtitles** y276: Load subtitle files automatically → ON · File
  name matching → dropdown "Title with extra characters".
- **Appearance** y423 (subtitle look): Font → input "System, sans-serif" ·
  Size values → segmented "Relative"/"Pixels" (2×157 cells) · Font size →
  dropdown "Automatic" · Bold → OFF · Text color → color input "#FFFFFF" ·
  Text opacity → slider (fill 159, 9 ticks) + scale 0%/50%/100% · Outline
  thickness → dropdown "Normal" · Outline color → color input "#000000" ·
  Shadow → ON.
`subtitles-1`: identical — duplicate.

## C8. `Settings - controls` — "Mouse, Keyboard & Remote Controls"

- **Keyboard Shortcuts** y83: Defaults → full-width action button 320×32
  "Edit Keyboard Shortcuts" (@ x704, centered label) · Reset all shortcuts
  → full-width action button "Reset".
- **Skip Amounts** y226: Very short skip → input "3 s" · Short skip →
  "10 s" · Medium skip → "60 s" · Long skip → "300 s" (inputs 318×34,
  pitch 48).
- **Mouse** y477: Single click on the video → dropdown "Play / Pause" ·
  Double click on the video → dropdown "Toggle fullscreen" · Scroll wheel →
  dropdown "Volume" · Horizontal scroll → dropdown "Seek" · Reverse scroll
  direction → OFF.
- **Remotes** y762: Web remote control → OFF · Port → input (empty).
No `-1` frame; no Advanced bar.

## C9. `Settings - library` — "Playlists, Folders & Files"

- **Opening Files** y83: Only allow one VLC window → OFF · Apply to files
  opened from the file manager → OFF **@ row opacity 0.3 (disabled)** ·
  Add to the playlist instead of replacing it → OFF @ 0.3 · When opening a
  folder → dropdown "Include subfolders" · Show hidden files → OFF.
- **Metadata & Artwork** y356: Fetch titles and artwork → dropdown "From the
  file only" · Use the media library → ON.
- **Advanced Options bar** y490 (collapsed ›; nothing below in frame).

## C10. `Settings - network` — "Network Connections & Security"

- **Buffering** y83: Buffering → dropdown "Normal" · Local files → input
  (empty) @ 0.3 · Discs → input @ 0.3 · Network streams → input @ 0.3 ·
  Live capture → input @ 0.3.
- **Proxy** y380: HTTP proxy → input "System proxy" · HTTP proxy password →
  input (empty).
- **Security** y535: Verify certificates against the system trust store →
  ON · Cipher preference → dropdown "Balanced".
- **Advanced Options bar** y669 (collapsed ›).

## C11. `Settings - performance` — "Performance Settings"

- **Decoding** y83: Hardware-accelerated decoding → dropdown "Automatic" ·
  Video post-processing → dropdown "Off" · Skip the H.264 loop filter →
  dropdown "Never" · Decoding threads → input "0".
- **Smooth Playback** y328: Drop frames when the system can't keep up → ON
  · Give VLC real-time priority → OFF · Process priority → input "0" ·
  Rescan plugins → full-width action "Rescan".
- **Advanced Options bar** y548 — **EXPANDED (chevron ∨)** with the advanced
  section visible below:
- **Developer** y619: Encoder → dropdown "Automatic" · Decoder → dropdown
  "Automatic" · Clock synchronisation → dropdown "Default" · Clock jitter →
  input "5000 ms" · Clock reference averaging → input "40".

## C12. `Settings - recording` — "Recording, Casting & Streaming Out"

- **Recording** y83: Save recordings to → input "~/Videos" (208×34) +
  Browse · Record without re-encoding when possible → ON.
- **Casting** y232: Discover cast devices → ON · Cast quality → dropdown
  "Automatic".
- **Streaming** y379: Play locally while streaming → ON · Include audio →
  ON · Include video → ON · include subtitles [sic lowercase in layer] →
  ON · Encoding preset → dropdown "Medium" · Tuning → dropdown "None".
- **Advanced Options bar** y685 (collapsed ›).

## C13. `Settings - extensions` — "Add-ons & Scripts"

- **Add-ons** y83: extensions LIST 782×192 rx5 #090909 + 2px #191919 stroke
  @ (241,110) — empty in frame (no rows exported; rows [VERIFY]) ·
  toolbar row y311: search input 208×34 rx5 w/ "Search" placeholder #404040
  @ x241 + "Find more extensions" 185×32 rx6 #191919 @ x619 + "Add" 100×32
  @ x814 + "Remove" 100×32 @ x924 (10px gaps, right-aligned to x1024) ·
  Load add-ons on startup → ON @ y361.
- **Advanced Options bar** y418 (collapsed ›).

---

# PART D — SHARED GRAMMAR + MAPPING

## D1. Tool-card grammar (audio + video tools)

| Element | Value |
|---|---|
| Collapsed card | 360×48 · r6 · #090909 · "Expand Settings" #6B6B6B ≈14px @ 17px inset · chevron-› 5×10 sw1.5 #6B6B6B @ 24px right inset |
| Expanded card | 360×H · r6 · **#1B1B1B** · "Collapse Settings" + chevron-∨ 10×5 @ (353,110) · header 59px |
| Card height | `59 (header) + (N−1)·pitch + last-row block + 20 (bottom pad)`; pitch: 90 chip rows (A1) · 76 text rows (A4/A5/A6/B1, B3) · 49 input rows (B2); last-row block ≈ 69 (chip) / ≈ 53 (text) |
| Row label | #6B6B6B ≈14–15px · x36.4–37.2 (16–17px inset) |
| Row value (chip) | 138×34 r5 #090909 + 2px #191919 stroke @ x225 (17px right inset) · text #8F8F8F ≈12px @ 13.7px inset |
| Row value (plain) | #6B6B6B ≈15px right-aligned to x≈362.5 (17.5px inset) |
| Slider | track 328×16 r6 #090909 (16px insets) · fill 12h r4 #371F06 inset 2 · thumb 8×24 r4 #AB6314 at fill end · ticks 6px #1A1611 sw2 (optional) |
| Toggle row | label left + Toggle 68×32 @ x296 (16px right inset) · a19 B3 |
| Hex/color row | input 116×32 r6 #090909 + swatch 22×22 r2 (5px from input right) + toggle |
| Separator | 1px #222222 · full card width (x20→380) |
| Sub-dropdown | 328×48 r6 #0E0E0E + inner 324×44 r4 #191919 |
| Vertical fader | track 16×320 r6 · fill 12w r4 #371F06 from bottom · thumb 24×8 r4 #AB6314 · pitch 68 · label below + 60×32 r6 chip |
| Dropdowns (A3/A7/B4) | a19 A9 Dropdown grammar: 362×48 trigger, 44px rows @ 46 pitch, check 14×11 sw2 #D37918, shadow 0/+4 σ6.9; leading icons optional (22px two-tone @ x39) |

**Pitfalls**: (1) r6 not r8; (2) flat #371F06 fill not gradient; (3) thumb
#AB6314 not #D37918; (4) collapsed ≠ expanded fill; (5) two value styles
(chip vs plain) — decide per tool per frame; (6) no hover/pressed states in
frames — use a19 B14 icon-button hover (white@6% r4) and keep keyboard
focus rings; (7) chevron direction encodes state (› collapsed / ∨ expanded)
in cards AND the Settings Advanced bar.

## D2. Settings shared grammar

| Element | Value |
|---|---|
| Window | 1048×865 · r14 · #0E0E0E@.95 · blur + shadow 0/+8 σ7.5 · floating overlay |
| Top bar | 50px #090909 · search pill 200×34 r6 #151515 (magnifier 15×15 #6B6B6B sw1.5, "Search" #404040 ≈15px) · title #969696 ≈16px @ x240 · ✕ 10×10 sw1.5 #8F8F8F @ 25px right inset |
| Sidebar | 216 total · panel 200×799 r6 #191919@.95 @ (8,58) · groups App/Media/System #6B6B6B ≈14–15px · 13 rows 188×40 r2 @ 48 pitch · selected #222222 + icon #D37918 (sw2) + label #EAEAEA |
| Content | column x240–1024 (784) · scrollable viewport 832×815 · section headers #C6C6C6 ≈16–17px · rows label-left (#8F8F8F ≈15px + optional ⓘ r7.5 #404040 sw1.125) / control-right 320px @ x704–1024 · pitch 42/46 · section gap ~48 |
| Controls | dropdown 320×36 · toggle 68×32 · input 318×34 r5 (+2px #191919) or 208×34+Browse · actions 320×32 / 100×32 / 185×32 r6 #191919 · segmented 320×36 · swatches 9×19.556×20 r1 · color input 320×32 · slider 320×16 + scale ≈12px #6B6B6B · radio rows 320×78 · disabled rows @ 0.3 |
| Advanced Options | 784×38 r2 #191919@.95 · "Advanced Options" #8F8F8F ≈15px + chevron (›/∨) centered · disclosure IN FLOW — advanced sections render below |

**Pitfalls**: (1) it is a floating window (overlay), current UI renders a
full route — shell rebuild needed but route/behavior contracts stay; (2)
keep existing Apply/Revert dirty-state bar (no frame shows it — restyle to
this grammar, don't delete; behavior contract); (3) sidebar label is
"General" while the area id stays `global`; (4) 13 areas = 4+5+4 groups
exactly; (5) section headers ≈16–17px #C6C6C6 (content.primary), NOT 15px;
(6) `#1A1611`/`#969696` missing from tokens; (7) only `global-1` and
`audio-1` are real content variants — `interface-1`, `playback-1`,
`video-1`, `subtitles-1` are duplicate exports (±2px).

## D3. Figma area → existing UI mapping

Existing registry (`ui/src/screens/settings/areas/index.tsx` +
`generated/Screens.tsx`): generated ids `global, interface, privacy,
playback, performance`; hand-registered `hotkeys, codecs, library, network,
about, advanced, effects`. `VideoArea`/`AudioArea`/`SubtitlesArea` components
EXIST in `areas/` but are NOT registered (unreachable). Current shell =
flat nav (no groups), no top-bar search pill, no Advanced Options bar.

| Figma area (group) | Figma window title | Existing UI | Verdict |
|---|---|---|---|
| General (App) | Global Settings | `global` generated — 1 row (language); Figma wants Appearance+Startup+Diagnostics+advanced; Appearance currently lives under `interface` | **NEEDS-MERGE** (area + route exist; content re-section; move Appearance) |
| Interface (App) | Interface Settings | `interface` generated — Appearance+Window; Figma wants Skin+Window+Playback Controls+Notifications | **NEEDS-MERGE** |
| Privacy (App) | Privacy Settings | `privacy` generated — Data section (telemetry etc.); Figma wants History+Network(last.fm) | **NEEDS-MERGE** |
| Controls (App) | Mouse, Keyboard & Remote Controls | none; `hotkeys` area (HotkeysEditor) is the natural target of its "Edit Keyboard Shortcuts" button | **NEW** (fold hotkeys in as the edit surface) |
| Playback (Media) | Playback Preferences | `playback` generated — Tracks+Playback; Figma wants Startup+Tracks+Order+Seeking | **NEEDS-MERGE** |
| Video (Media) | Video Defaults | `VideoArea` exists, unregistered, empty state | **NEEDS-MERGE** (register + 4 sections) |
| Audio (Media) | Audio Defaults | `AudioArea` exists, unregistered (output-device picker ≈ Figma Output section) | **NEEDS-MERGE** (register + Volume/Loudness/Effects+EQ) |
| Subtitles (Media) | Subtitle Defaults | `SubtitlesArea` exists, unregistered, empty | **NEEDS-MERGE** (register + 3 sections) |
| Library (Media) | Playlists, Folders & Files | `library` registered → PlaylistServicesArea (empty state) | **NEEDS-MERGE** (registered; compose Opening Files+Metadata rows) |
| Network (System) | Network Connections & Security | `network` registered → NetworkArea (empty state) | **NEEDS-MERGE** (registered; compose Buffering+Proxy+Security) |
| Performance (System) | Performance Settings | `performance` generated — Resources; Figma wants Decoding+Smooth Playback+Developer | **NEEDS-MERGE** |
| Recording (System) | Recording, Casting & Streaming Out | none | **NEW** |
| Extensions (System) | Add-ons & Scripts | none | **NEW** |

**Counts**: EXISTS 0 · NEEDS-MERGE 10 · NEW 3 (Controls, Recording,
Extensions). Closest to EXISTS: `playback`/`privacy`/`performance`
(registered + real rows, wrong sections).

**Orphaned existing areas with no Figma frame** (Lead decision needed):
`hotkeys` (→ Controls), `effects` (→ Audio→Effects hosts the EQ panel per
audio-1), `advanced` (→ the Advanced Options disclosure), `codecs`
(→ Performance candidate), `about` (no frame — keep, restyle to grammar).

## D4. Open questions for the Lead / token lane

- Add `#969696` (settings row labels / a19 sidebar rows) and `#1A1611`
  (slider degree ticks) to `themes/tokens.json`? Both absent.
- Tool-card family radii: cards r6 + EQ bar/panel r2 — confirm r2 is
  intentional (it reads as a deliberately sharper sub-family).
- Sub-dropdown outer #0E0E0E (B1 Gradient) vs #090909 everywhere else —
  unify to one `field.bg`?
- Section-header size: measured ≈16–17px #C6C6C6 (spec said 15px semibold) —
  confirm 16 or 17.
- EQ preset names: "Full bass" vs render-reading "Full base" [VERIFY];
  "Spectometer" vs "Spectrometer"; compressor "FM5 / Peak" is a design typo
  for "RMS / Peak" — i18n values should use correct words.
- Advanced-Options sections NOT exported for: interface (Notifications
  rows), video, subtitles, controls, library, network, recording, extensions
  — content below those bars is [VERIFY]; compose from design language if
  absent.
- Does the 1048×865 floating-window Settings overlay a scrim over the
  player, and does the top-bar ✕ close to `/player`? [VERIFY — no frame
  shows the backdrop.]
