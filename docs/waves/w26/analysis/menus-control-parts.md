# W26 · Component anatomy spec — MENUS + CONTROL PARTS

Task `w26.a19` (analysis lane). For wave-c implementers building the menu /
control-part component library.

**Method**: SVG XML in `design/figma/` is ground truth (parsed: every rect,
path bbox, fill/stroke, filter, gradient). PNG renders (`design/figma-renders/`,
900px-wide, ≈2.4× scale for 375px frames) + `z-ai vision` used only to confirm
labels/icon readings. **Where SVG and the W25 spec §2 conflict, SVG wins**
(conflicts flagged). All frames here are native-scale component exports — the
artboard stroke-scaling factors (0.825/1.125/1.28571, `themes/tokens.json`
header) do NOT apply, except stray `stroke-width="1.28571"` icon copies inside
`Icon.svg` (scaled duplicates — exclude, see §Icon).

**Font-size derivation**: Figma text is outlined. Measured cap-heights
(digit/cap glyph bbox) ÷ 0.727 (Inter cap ratio) → sizes below are reported as
`cap 10.5px → ≈15px`. Weights are [VERIFY] (outlined shapes can't prove 400 vs
500; fidelity spec says medium for menu labels).

**Color palette observed in these frames** (all match the W25 token set unless
noted): `#090909 #0E0E0E #151515 #161616 #191919 #1A1611 #1B1B1B #1F1F1F
#222222 #262626 #303030 #371F06 #404040 #6B6B6B #8F8F8F #969696 #AB6314
#B8B8B8 #D37918 #EAEAEA #0C0C0C` + volume gradient `#95D318 #B2A918 #FF3737`.
`#969696` (sidebar rows, compact-dropdown labels) is NOT yet in the token set.

---

# PART A — MENUS (11 frames)

## A1. `menu` — app "Media" menu (375×585)

```
menu (panel)
├─ size 375 × 585 · rx 10 · fill #191919 @ 95% opacity
├─ backdrop blur 50 (data-figma-bg-blur-radius) — heavy; 95% opaque
├─ drop shadow: 0/+8y · blur σ7.5 · black @ 46%
└─ 12 option rows + 1 separator  (height = 4 + 48×12 + 1 + 4 = 585)
   ├─ row: 48px tall · row-rect x=4 w=367 (4px inset from panel) · rx 6
   │        hover/selected fill #1F1F1F (default = transparent)
   ├─ icon 20×20 @ x20–40 (16px from row left edge) · stroke 1.5
   │        two-tone: base #8F8F8F + accent detail #AB6314
   ├─ label @ x52.8 (12.8px gap after icon) · #8F8F8F · cap 10.4–10.7 → ≈15px
   ├─ shortcut right-aligned, right edge @ x355–356 (19px from panel edge,
   │  15px from row edge) · #6B6B6B · cap 10.4 → ≈15px (same size as label)
   ├─ submenu chevron "›" 5×10 @ x343–348 (27px from panel edge) · #8F8F8F · sw1.5
   └─ separator: 1px #222222 · x4→371 (full row width, 4px inset) · round caps
      placed ON the row-7/8 boundary (adds 1px to total height)

rows (labels are REAL text in this frame, not placeholders):
 1  Open File(s)         Ctrl + O    icon: rounded doc, orange corner fold
 2  Open Folder          Ctrl + F    folder, orange tab
 3  Open Network Stream  Ctrl + N    broadcast dot + arcs
 4  Open Disc            Ctrl + D    disc, orange center dot
 5  Open Capture Device  Ctrl + C    monitor w/ orange play triangle
 6  Convert              Ctrl + R    swap arrows (one orange)
 7  Open Recent          ›           clock + ccw arrow
 —— separator ——
 8  Profile (Movies)     ›           person, orange head dot
 9  Keyboard Shortcuts   —           keyboard, orange keys
10  Customize Player     —           paintbrush, orange tip
11  Settings             Ctrl + P    gear, orange center
12  About VLC            Shift + F1  info circle, orange dot
```

**Tokens needed**: `menu.panel.radius(10)`, `menu.panel.fill(#191919@.95)`,
`menu.row.height(48)`, `menu.row.inset(4)`, `menu.row.radius(6)`,
`menu.row.hover(#1F1F1F)`, `menu.icon.size(20)`, `menu.icon.stroke(1.5)`,
`menu.label.color(#8F8F8F)`, `menu.shortcut.color(#6B6B6B)`,
`menu.separator(#222222)`, `menu.shadow(0 8 7.5 / .46)`.

**Notes**: (1) W25 spec §2 says menu labels `#e8e8e8` — **SVG says #8F8F8F;
use #8F8F8F** (only `setting category` selected label uses #EAEAEA).
(2) Shortcut text is the SAME size as the label (not smaller) — spec §2's
"13–14px shortcut" is wrong, cap-height 10.4 → ≈15px. (3) Height formula
`8 + 48·rows + separators` reproduces every menu frame exactly — build the
panel from row count, don't hardcode heights.

## A2. `context menu` (265×296)

Same panel grammar (r10, #191919@.95, blur 50, shadow 0/+8/σ7.5/46%).
6 rows × 48 + 8 = 296. Row 1 shows the hover/selected state:
row-rect `x4 y4 257×48 rx6 #1F1F1F`. Labels are literally placeholder
"Option" ×6 (cap 13.4 w/ descender → ≈15px, #8F8F8F, x52.8). No icons,
no shortcuts, no separators in this frame. → the generic context-menu shell;
real labels come from i18n at implementation.

## A3. `file context menu` (265×584)

Panel grammar identical. 12 rows × 48 + 8 = 584. Two-tone icons in every row
(#8F8F8F + #AB6314), labels "Option" placeholders @ x52.4–53.2, #8F8F8F,
cap 10.4–10.8 → ≈15px. NO separators, NO chevrons, NO shortcuts.

## A4. `folder context menu` (265×296)

6 rows × 48 + 8 = 296. Placeholder labels. Icons (from path shapes):
row1 `+` (new folder, #8F8F8F) · row2 folder outline #8F8F8F + orange `+`
#AB6314 · row3 list card w/ orange lines · row4 waveform bars (heights 2–18,
two-tone) · row5 scissors · row6 info circle #8F8F8F + orange dot.

## A5. `playlist context menu` (265×392)

8 rows × 48 + 8 = 392. Placeholder labels. Icons: play-in-rounded-square
(#AB6314), link, arrows, etc. No separators/chevrons/shortcuts.

## A6. `right-click menu` (292×683)

Panel grammar identical (292 wide). 14 rows × 48 + 3 separators + 8 = 683.
Separator #222222 x4→288 at y244, y293, y486. 7 submenu chevrons 5×10 @
x260–265 (27px inset) #8F8F8F. Shortcuts right-aligned, right edge ≈x273
(19px inset) #6B6B6B ≈15px.

```
 1  Play           Space     ▶ (orange)
 2  Stop           S
 3  Previous Item  P
 4  Next Item      N
 5  Record         R         circle+dot (orange)
 —— sep y244 ——
 6  Hide UI        Ctrl + \  monitor
 —— sep y293 ——
 7  Playback       ›         sliders (orange)
 8  Video          ›
 9  Audio          ›         EQ bars (orange)
10  Subtitle       ›         subtitle card
 —— sep y486 ——
11  More Settings  ›         gear + orange dot
12  Playlist       ›         list
13  Open Media     ›         folder + orange fold
14  Quit VLC       Ctrl + Q  power/door icon
```

## A7. `option` — menu-item component (301×292)

```
Property 1=Default    label only, no bg · #8F8F8F · row area 261×48 (implied)
Property 1=hover      rect 261×48 rx6 #1F1F1F · label #8F8F8F
Property 1=selected   rect 261×48 rx6 #1F1F1F · label #8F8F8F
                      + check 14×11 · stroke 2 · #D37918 · round joins
                        @ x248–262 → 19px from row right edge, v-centered
```
**Notes**: no separate "pressed" state exists in the design. Selected ≡ hover
bg; selection is signaled by the orange check only. Check is the ONLY orange
element — label stays #8F8F8F even when selected.

## A8. `setting category` — Settings sidebar item (228×140)

```
selected=no   icon 12×19.5 @ x34 · stroke 1.5 #8F8F8F · label "Interface"
              #8F8F8F @ x65.2 (19px gap) · cap 10.8 → ≈15px · no bg
selected=yes  rect 188×40 rx2 #222222 · icon fill #D37918 (filled, not stroked)
              · label #EAEAEA
```
**Notes**: (1) Radius is **2**, NOT r6 — conflicts with W25 spec §2 "active
row: r6 raised bg". SVG wins: `rx="2"`. (2) This is the ONLY menu-family
component where the selected label brightens (#EAEAEA) and the icon turns
accent (#D37918). (3) Height 40 (not 48) — Settings sidebar density.

## A9. `Dropdown` (406×369) — standard dropdown

```
state=default (closed trigger)
├─ field: 364×48 · rx6 · fill #090909  ← reads as 2px border on panels
├─ inner:  360×44 · rx4 · fill #191919 · inset 2
├─ label "Option" @ x38.2 (15px from inner left) · #8F8F8F · ≈15px
└─ chevron-down 10×5 @ x352–362 (23px from field right) · #8F8F8F · sw1.5

state=open
├─ list: 364×232 · rx6 · fill #191919 · shadow 0/+4y σ6.9 black @35%
├─ 5 rows: 360×44 · rx4 · inset 2 · 2px gaps (pitch 46)
│    row bg: transparent · selected row #1F1F1F + check 14×11 sw2 #D37918
│    @ x350–364 (19px from row right edge)
├─ row label @ x38.2 · #8F8F8F · ≈15px
└─ list height = 2 + 5×44 + 4×2 gaps = 232
```
**Notes**: two-layer field (#090909 outer + #191919 inner) ≈ a 2px
`#090909` border — implement as `border: 2px solid var(--surface)` on a
`--surface.panel` field, or a single #090909 fill; pick ONE and keep Buttons
consistent (see A13 which uses flat #090909). NO leading icons in dropdown
rows (icon `g` groups are empty placeholders).

## A10. `Profiles` (651×633) — menu + open submenu

Parent = the exact `menu` component (A1) with row 8 "Profile (Movies)"
showing hover bg `x4 y341 367×48 rx6 #1F1F1F`.

```
submenu "Profile" (flyout)
├─ panel 265×249 · rx10 · #191919@.95 · blur 50 · shadow 0/+8 σ7.5 46%
│  (height = 8 + 48×5 + 1 sep = 249)
├─ position: x=386 (11px right of parent panel edge 375) · top y=337
│  (parent row-8 top 341 − 4)
├─ row 1 (selected): rect x390 y341 257×48 rx6 #1F1F1F · check 14×11 sw2
│  #D37918 @ x409–423 (icon slot, centered x416) · label #8F8F8F @ x439
├─ rows 2–4: plain labels @ x438.3–439.2 · #8F8F8F
├─ separator #222222 x390→647 @ y533 (after row 4)
└─ row 5 "New": plus icon 14×14 #8F8F8F @ x409–423 · label #8F8F8F
```
**Notes**: render shows real labels Movies / Shows / Music / Work / New
[VERIFY — SVG layer ids are "Option" placeholders; vision read these words].
The selected profile replaces its icon slot with the orange check. The "+ New"
footer row after a separator is a recurring menu pattern (also in Setting
Preset, sidebar "Create playlist").

## A11. `Setting Preset` (406×412) — compact dropdown, opens UPWARD

```
state=default (trigger, top)
├─ 364×42 · rx6 · fill #090909   (42 tall — compact vs Dropdown's 48)
├─ label "Option" @ x38.2 (17px inset) · #8F8F8F · ≈15px
└─ chevron-UP 10×5 @ x352–362 (23px inset) · #8F8F8F   (opens upward)

state=open
├─ list 364×242 · rx6 · #191919 · ABOVE the trigger
│  shadow: 0 offset · dilate 8 · blur σ7 · black @35% (border-glow look)
│  height = 2 + 6×40 flush rows = 242
├─ row 1: plus icon 14×14 #6B6B6B @ x42–56 · label #6B6B6B ("new preset")
├─ rows 2–5: 3-dot bullet (Ø3 dots @ x43/49/55, 6px pitch, #8F8F8F, sw3
│  round caps) · label #8F8F8F @ x71.8 (≈17px after bullet)
└─ trigger (bottom, re-used as selected row): 364×42 rx6 #090909 containing
   selected row 360×38 rx4 #1F1F1F (inset 2) + check 14×11 sw2 #D37918
   @ x350–364 (19px inset)
```
**Notes**: rows are 40px FLUSH (no gaps) — different row grammar from
Dropdown (44 + 2px gaps). Chevron direction signals open direction: ∧ here,
∨ in Dropdown/origin/target.

---

# PART B — CONTROL PARTS (15 frames)

## B1. `Controls` (1960×11139) — control bar + popover panels, 15 states

State sections (y offsets): default 33 · waveform 117 · music 216 ·
Variant10 542 · queue 1658 · subtitles 2354 · audio 3106 · video 3858 ·
video settings 4957 · audio settings 6056 · subtitle settings 7192 ·
playback 8333 · sub timeline 8646 · bookmarks 9916 · chapters 11013.

### Control bar (default state)
```
bar: 54px tall (explicit 1920×54 strip #191919@95% in sub-timeline state)
     contents vertically centered (y≈47)
├─ play pill  54×28 · rx6 · #D37918 @ x78 · glyph ▶ 8.5×12.5 #161616
├─ stop icon  8×10 #8F8F8F @ x151–160 (sw1.5)      [default variant has
│              no prev/next — music variant does]
├─ current time "00:02:10" #8F8F8F · x197–257 · cap 11.2 → ≈15px tabular
├─ seekbar    1238×28 · rx6 · #090909 · x274–1512
│   ├─ played fill 98×24 · rx4 · #371F06 · x276 (inset 2)
│   ├─ playhead 8×24 · rx4 · #D37918 · @ end of played (x366–374)
│   └─ (NO thumb circle — playhead is an 8px rounded bar)
├─ total time "00:25:40" #8F8F8F · x1526–1588
├─ volume icon speaker+3arcs ~18px #8F8F8F @ x1620–1638
├─ volume bar (see B7-volume): wedge 89×27.7 #090909 @ x1648–1737
│   gradient fill 33px wide = 50% level · stroke-mask 86×24.8
├─ spectrum icon (5 bars 2/6/8/12/18px tall) @ x1763–1771 #8F8F8F
├─ subtitles icon (card 20.5×14.5 rx~3 + 2 lines) @ x1802–1823
├─ settings gear 17.2×18 + dot @ x1846–1863
└─ list/menu icon (3 lines 12–14px + ticks) @ x1897–1913
    right-cluster icon pitch ≈ 40–50px, all #8F8F8F sw1.5
```

### Waveform variant
seekbar track same 1238×28 r6 #090909; content = #404040 bars 2px wide,
**5px pitch**, heights 2–14 (y-centered), full width; playhead 4×24 rx2
#D37918; no #371F06 fill.

### Music variant
`prev` x51–60 · `back-15` icon 18×6 + "15" label 7.4cap #8F8F8F @ x83–101 ·
play pill 54×28 #D37918 @ x116 · `fwd-15` @ x185–203 · `next` @ x227–236 ·
timecode x273 · seekbar **1204×28** r6 @ x350–1554 · total x1568 · volume
icon x1662 + bar x1690–1779 (same 89px wedge) · spectrum x1805 · gear x1846 ·
menu x1897.

### Popover panels (queue/bookmarks/chapters/subtitles/audio/video/settings)
```
panel: 400 wide · rx10 · #0E0E0E @95% · right-anchored (x1520–1920)
       heights: 946 (queue/bookmarks/chapters/settings) or 524 (tab panels)
       shadow 0/+4 σ6.9 35% (queue) — reuse menu shadow family
├─ header strip 400×50 · #090909 (rx2 corners)
├─ tab row (in header): e.g. Video · Audio · Subtitles @ x1588/1718/1839
│   cap 9.1 → ≈12–13px · active #8F8F8F · inactive #6B6B6B
│   (queue/bookmarks/chapters tabs same grammar @ x1586/1703/1838)
├─ content inset 20 (x1540, width 360):
│   ├─ rows 44px pitch · label #969696 ≈15px · right value #6B6B6B
│   │   (or #8F8F8F for current/selected) right-aligned to x≈1895
│   ├─ Button "Show Advanced Settings": 360×32 rx4 #090909 · icon 14×15
│   │   stroke 1.25 #8F8F8F · label #6B6B6B — full B13 grammar
│   ├─ dropdowns 220×48 rx6 #090909 + inner 216×44 rx4 #191919 (stacked,
│   │   68px pitch) — compact B9 grammar at 220 width
│   ├─ segmented control 360×44 rx6 #090909 · 3×117.33×40 rx4 · 2px gaps
│   │   active #371F06 + text #D37918 · inactive #191919 + #6B6B6B
│   ├─ mini slider 360×16 rx6 #090909 + played 12px-tall #371F06 inset 2
│   ├─ input rows 360×42/48 rx6 #090909
│   └─ color swatch field 138×34 rx5 #090909 · 2px #191919 stroke
├─ bookmarks rows: index digit #D37918 @ x1544 · name #969696 @ x1588 ·
│   time #6B6B6B right @ x1895 · 44px pitch
└─ queue rows: label #969696 + duration #6B6B6B right · 44px pitch
```

### sub timeline variant
Second full-width strip **1920×54 #191919@95%** directly under the bar.
Its seekbar: played 596×24 rx4 #371F06 · chapter segments white@14% rx1
(widths 12–83px) · subtitle cue rows below: "DON'T BLINK 1/2" #969696 +
frame-accurate timecodes "00:00:42:023" #6B6B6B (cap 10.5 → ≈15px tabular).

**Tokens needed**: `controlbar.height(54)`, `play.pill(54×28 r6 #D37918)`,
`seekbar.track(28h r6 #090909)`, `seekbar.fill(#371F06 r4 inset2)`,
`seekbar.playhead(8×24 r4 #D37918)`, `waveform.bar(2w/5pitch #404040)`,
`timecode(≈15px tabular #8F8F8F)`, `panel.popover(400 r10 #0E0E0E@.95)`,
`panel.header(50h #090909)`, `panel.row(44)`, `tab.active/inactive`.

**Notes**: (1) Playhead is a BAR, not a circle — spec §2's "circular thumb"
applies to Miniplayer only. (2) #371F06 (accent.tinted) is the played fill,
not the accent — orange appears only as the 8px leading edge. (3) Volume
wedge + gradient live INSIDE the 54px bar (28px tall). (4) Settings panels
(audio/subtitle settings) use section headers #969696 cap 12.4 → ≈17px +
label/value rows (label #6B6B6B left x1544, value #8F8F8F right x1775).

## B2. `Button` (400×140)

```
Property 1=Default   360×32 · rx4 · fill #090909
                     icon info-circle 14.4×15 stroke 1.25 #8F8F8F
                     label "Show Advanced Settings" #6B6B6B cap 10.7 → ≈15px
                     content horizontally CENTERED: icon x106.8 + 9.6 gap +
                     label → x294.9 (center 200.85 ≈ button center 200)
Property 1=Variant2  360×32 · rx4 · fill #191919 · label #8F8F8F · same icon
```
**Notes**: two button modes — recessed (#090909 + dim text) vs panel
(#191919 + brighter text). Icon stroke 1.25 (finer than menu icons' 1.5).
Centered content ⇒ this is the full-width footer button (Settings "Advanced
Options", panels' "Show Advanced Settings").

## B3. `Toggle` (120×194)

```
Value=on   track 68×32 · rx6 · #090909
           left segment 33×26 rx3 #371F06 @ x35 (3px inset)
           right knob  26×26 rx3 #AB6314 @ x71 (3px gap after segment)
           "ON" #D37918 cap 9.0 → ≈12px, centered in left segment (x42.7–59.1)
Value=off  track 68×32 rx6 #090909
           right segment 33×26 rx3 #191919 @ x64
           left knob 26×26 rx3 #404040 @ x35
           "OFF" #6B6B6B ≈12px in right segment (x68.7–90.5)
```
**Notes**: NOT a classic switch — a two-cell segmented pill with the text
living in the non-knob cell. ON = accent.tinted cell + accent text + accent-
strong knob; OFF = panel cell + tertiary text + border-gray knob. Knob 26×26
r3 (square-ish). Total 68×32 matches the "32×16 toggles w/ ON/OFF text" hint
in W25 spec §2 (actual: 68×32 — spec's 32×16 is wrong, SVG wins).

## B4. `Values` — segmented control (253×404)

```
container 213×44 · rx6 · #090909   (2px inset rows inside)
2-segment: 2 × 103.5×44… inner 103.5×40 rx4 @ x22 & x127.5 (2px gap)
3-segment: 3 × 68.33×40 rx4 @ x22 / x92.33 / x162.67 (2px gaps)
active segment:   #371F06 + label #D37918
inactive segment: #191919 + label #6B6B6B
labels ≈15px centered ("Relative"/"Pixels", "1"/"2"/"3")
```
**Notes**: same active-grammar as Toggle ON and Dropdown selected —
`#371F06` cell + `#D37918` text is THE selection pattern for controls
(while menus use #1F1F1F + gray text + orange check).

## B5. `Values-1` — radio rows w/ conditional input (360×216)

```
container 320×78 · rx6 · #090909
row 1: 316×36 rx4 (inset 2) — selected: #371F06 + label #D37918
row 2: 316×36 rx4 — unselected: #191919 + label #6B6B6B
       rows flush (2px gap): y22–58, y60–96
row 2 = "Always starts at:" + trailing input:
       input 182×32 · rx2 · #090909 @ x154 (2px from row right edge 336)
       value "200%" #8F8F8F cap 10.5 → ≈15px @ x167 (13px inset)
       DISABLED variant: whole input group opacity 0.4 (row unselected)
Variant2 flips: row1 "Remember last volume" unselected, row2 selected
```
**Notes**: r2 on the inner input (micro radius — matches W25 "r4 inputs"
only loosely; SVG says 2). Conditional enable via opacity 0.4, not color
swap.

## B6. `Current Time` (114×164)

```
state=Default  "00:02:10" #8F8F8F · cap 11.2 → ≈15px tabular · no bg
state=hover    chip 76×28 rx6 #090909 · text #8F8F8F centered
state=click    chip 76×28 rx6 #090909
               text split: edited fields #EAEAEA + rest #8F8F8F
               caret 1×15.5 #D37918 @ 6.5px from chip right edge, v-centered
```

## B7. `Duration` (114×164)

```
state=Default  "00:25:40" #8F8F8F ≈15px tabular
state=click    "-00:23:30" #8F8F8F (minus prefix = remaining-time mode)
```
**Notes**: Duration is a passive readout — no chip, no caret. Current Time is
the editable one. Timecode width ≈60–70px for `HH:MM:SS`.

## B8. `volume` (130×646) — 13 gradient states

```
states: 0% · 10–100% (10) · 150% · 200% — stacked, 48px pitch
track:  HORIZONTAL WEDGE 89 wide × ~28 tall (left edge ≈11 tall →
        right edge ≈21 tall, rounded corners r≈2.5) · #090909
        path: left cap y77.6–88.4 → right cap y72.3–93.7 (10% state)
fill:   rounded rect r1, ~30 tall, width ∝ level, hard right edge, no thumb
        gradient (fixed to FULL track width, not to fill):
          0%   #95D318 (green)
          57.9% #B2A918 (yellow)
          79.2% #D37918 (orange)
          100% #FF3737 (red)
measured fill widths: 0/7/14/20/26/33/41/48/53/58/63/75/88 px
        (50% = 33px; 200% = 88px = full width) — mapping is NOT linear
        0–200 [VERIFY mapping; 100% lands at 63px ≈ 71% of track]
no labels, no speaker icons, no thumb in this component
```
**Notes**: greens/reds (`#95D318 #B2A918 #FF3737`) exist ONLY here (and the
EQ band set) — they are NOT the UI accent; do not tokenize as accent. The
gradient must be anchored to the track, so a full (200%) bar shows the whole
green→red ramp and 100% ends in the orange zone. Vision confirms: no handle,
hard fill edge, tapering wedge silhouette.

## B9. `play` (94×116)

```
type=play   pill 54×28 rx6 #D37918 · glyph ▶ 8.5×12.5 fill #161616
            (glyph optically centered, center x≈48)
type=pause  pill 54×28 rx6 #6B6B6B (content.tertiary)
            bars 2×10 ×2 @ 6px apart (x43–45, x49–51) · fills #0E0E0E/#161616
            (two ink values are an export artifact — use one dark ink)
```
**Notes**: pause state grays the pill (#6B6B6B) — the accent is reserved for
"playing". Glyph ink #161616 = gray.750 (content-on-accent candidate).

## B10. `origin` — subtitle-source dropdown (240×525)

```
trigger 200×34 · rx6 · #151515 (surface.muted — NOT #090909!)
        label "From Computer" #969696 @ x33.2 (13px inset) · ≈15px
        chevron-down 10×5 #8F8F8F @ x195–205 (15px inset)
open list 209×148 · rx6 · #191919 · shadow 0/+4 σ6.9 35%
        3 rows 205×48 rx6 · inset 2 · FLUSH (48px pitch, no gaps)
        hover row #1B1B1B (≠ menu's #1F1F1F!) · others transparent
        row 1 selected: label #8F8F8F + check 14×11 sw2 #D37918
          @ x190–204 (19px from row right 223)
        rows 2–3: "From OpenSubtitles", "From Link …" · #969696
```
**Notes**: the compact 34px trigger + 48px flush menu rows + #1B1B1B hover =
the "view/sort options" family (matches Sorting/View Options frames). THIRD
hover color in the system: #1F1F1F (menus), #1B1B1B (these lists), #222222
(setting category).

## B11. `target` — subtitle-destination dropdown (240×520)

Identical geometry to `origin` (trigger 200×34 r6 #151515, list 209×148 r6
#191919, rows 205×48 r6 flush, hover #1B1B1B). Labels: trigger "To This
Folder" / "To This File"; rows "To This Folder" / "To This File" /
"To This Playlist". Same check grammar on the selected row.

## B12. `sidebar` (4492×1028) — 10 sidebar states, 447px pitch

Per state: icon rail **50×986 #191919** + 20px gap + playlist panel
**377×986 #0E0E0E@95%**.

```
icon rail: Icon Wrappers 30×32 (hover white@6%) · collapse handle 4×6 rx1 #8F8F8F
playlist panel:
├─ breadcrumb header y≈41–62: crumbs #8F8F8F cap ~12 → ≈15–16px · "›"
│  separators #404040 · "…" overflow #8F8F8F (e.g. "The Wand › S01",
│  "Videos › Shows", "Songs › My Songs")
├─ meta row y≈92–103: "<N> items" #6B6B6B cap 10.7 → ≈15px
├─ create row y≈157–170: "+ Add" / "Create playlist" / "+ Add Network Stream"
│  #6B6B6B + plus icon
├─ item rows 42px pitch (y199, 241, 283, …):
│   media rows: thumbnail 32×32 rx2 (#303030 placeholder) + label @ ≈10.7px
│   gap · #969696 (first row #8F8F8F [VERIFY: playing/hover], lower rows
│   #6B6B6B [VERIFY: dimmed]) · cap 10.1–10.7 → ≈14–15px
│   thumbnail progress: 3px-tall #D37918 bar at thumb bottom-left, width
│   ∝ progress (4–7px observed)
│   folder/playlist rows: small icon + label, no thumbnail
└─ section headers (Home state): "Continue Watching", "Listen Again",
    "Frequently Visited Folders/Playlists", "Frequently Listened Artists"
    #969696 · cap ~12.4 → ≈17px · y-on-pitch 193 (=42×?) sections spaced
    ~193px (≈4.6 rows)
```
States L→R: 1 `The Wand › S01` episodes (15 items) · 2 `Home` (110 items,
all sections) · 3 `Videos` (3 items; Add + Shows/Films/Anime) · 4
`Videos › Shows` (Add + The Wand) · 5 `Shows › The Wand › …` (S01/S02) · 6
`Music` (Songs/Albums/Artists/Genres) · 7 `Music › Songs` (My Songs) · 8
`Songs › My Songs` (track list) · 9 `Network` (Add Network Stream +
Podcasts/Radio) · 10 `Playlists` (Create playlist + Cool Playlist).

**Notes**: panel bg is #0E0E0E@95% (surface.deep), NOT #191919 — the sidebar
is a recessed well next to the #191919 rail. Thumbnails 32×32 r2. 42px row
pitch ≠ menu 48. `#969696` needs a token (e.g. `content.row`).

## B13. `Icon` (548×812) — icon system

```
206 variants · ~90 glyphs · 20×20 boxes on a 44px grid
axes (Figma variant props): name · direction(default/left/right/up/down) ·
  state(default/active/selected) · value · EQ · two-tone(no/yes)
stroke: 1.5 standard (20px box) · 2 for small glyphs (chevrons, 10px content)
        · 3 rare · EXCLUDE stray stroke-width 1.28571 copies (artboard-scaled)
colors: default #8F8F8F · active #D37918 · two-tone secondary #AB6314
        (active two-tone = #D37918 + #AB6314)
glyphs: settings, playback settings, preferences, playlist, lists, hotkeys,
  subtitles, audio, playback, play, pause, next, stop, previous, refresh,
  recent, fullscreen, new folder, album, repeat, shuffle, view, sort, artist,
  profile, info, interface, subtitle, file, swap, window, speed, queue,
  chapters, bookmarks, save, disc, capture-device, quit, ui, genre, arrow,
  close, folder, streaming, chevron, thumbnail, home, sidebar, video, songs,
  videos, tv, link, check, paint, record, favorite, plugins, privacy, EQ,
  forward/backward-10/15-sec, replay, volume, move, minimize, maximize,
  expand-sidebar, collapse-sidebar, music, divider, upload, open, more, vlc,
  add, remove, search, zoom, accessibility, wrench, grid, list, snapshot,
  bookmark, spectometer, spectrum, scope, waveform, PiP, HDR, edit
sheet extras (not icons): 4×6 rx1 collapse handle · 2×12 rx0.5 divider ticks
  · 12×3 rx1 #AB6314 mini-progress · 20×18 rx4 window glyph · 24×24 rx2
  #303030 thumbnail placeholders
```
**Notes**: build icons as stroke glyphs (1.5px, round caps/joins) in a 20×20
viewbox, with an `accent` stroke layer for two-tone. "active" = full accent
recolor; "selected" variants exist for playback. EQ family has 18 variants.

## B14. `Icon Wrapper` (72×122) — icon button

```
Property 1=Default  bare icon (e.g. 17.2×18) · stroke 1.5 #8F8F8F · no bg
Property 1=hover    rect 30×32 · rx4 · white @ 6% opacity
                    icon centered (≈6.4px h-inset / 7px v-inset)
```
**Notes**: THE icon-button hover recipe used across the rail/sidebar/control
bar: `background: rgba(255,255,255,0.06)` on r4 — no border, no scale. Hit
target 30×32 (keep ≥44px a11y hit area via padding in implementation).

---

# PART C — CONSOLIDATED SHARED GRAMMAR

## C1. Menus

| Element | Value |
|---|---|
| Panel | r10 · #191919 @95% · backdrop-blur 50 · shadow 0/+8y σ7.5 black 46% |
| Panel height | `8 + 48×rows + 1×separators` (exact for every frame) |
| Row | 48px · inset 4 from panel · r6 · hover/selected #1F1F1F |
| Row icon | 20×20 · stroke 1.5 · two-tone #8F8F8F + #AB6314 · 16px from row left |
| Label | x = icon right + 13px · #8F8F8F · ≈15px (cap 10.4–10.8) |
| Shortcut | right-aligned · 19px from panel edge / 15px from row edge · #6B6B6B · same ≈15px |
| Submenu chevron | "›" 5×10 · 27px from panel edge · #8F8F8F · sw1.5 |
| Selected check | 14×11 · sw2 · #D37918 · 19px from row right edge · replaces icon slot in submenus |
| Separator | 1px #222222 · full row width (4px inset) · round caps · sits ON row boundary (+1px height) |
| "+ New" footer | after separator · plus icon 14×14 #8F8F8F (or #6B6B6B in Setting Preset) |
| Submenu position | +11px right of parent panel edge · top ≈ parent row top − 4 |

Three hover/selected flavors — do NOT merge: **#1F1F1F** (menus + standard
dropdowns) · **#1B1B1B** (origin/target compact lists) · **#222222 + icon
#D37918 + label #EAEAEA** (setting category, r2, 40px rows).

Dropdown field grammar: outer r6 + inner r4 inset 2 ("2px border"); standard
364×48 with 44px rows + 2px gaps; compact 364×42 (Setting Preset) or 200×34
(origin/target) with 40/48px flush rows; chevron 10×5, 23px inset (std) or
15px (compact), direction = open direction.

## C2. Control parts

**Selection grammar (controls, NOT menus)**: active cell `#371F06` +
text `#D37918`; inactive cell `#191919` + text `#6B6B6B`; container `#090909`
r6 with cells r4 inset 2 and 2px gaps. Appears in: Toggle, Values,
Values-1, panel segmented controls.

**Control sizes**: buttons/pills 28–32 tall (play pill 54×28 r6; Button
360×32 r4; Toggle 68×32 r6); dropdown fields 34/42/48 tall; menu rows 48;
panel rows 44; sidebar rows 42; settings category rows 40.

**Radii**: menu panel 10 · menu/dropdown rows 6 · dropdown inner/segments 4 ·
inputs 2–4 · icon-button hover 4 · popover panels 10 · setting category 2.

**Icon system**: 20×20, stroke 1.5, round caps/joins, #8F8F8F base, accent
layer #D37918 (active) / #AB6314 (two-tone secondary); icon-button hover =
white@6% r4, no border. Fine-detail strokes 1.25 (Button info icon), small
glyphs 2.

**Type scale (cap-height measured)**: timecodes ≈15px tabular (#8F8F8F;
edited fields #EAEAEA) · labels/menu text ≈15px · toggle text ≈12px ·
panel tabs ≈12–13px (cap 9.1) · section headers ≈17px (cap 12.4) ·
meta/count rows ≈15px (cap 10.7). All sentence case; tabular-nums on
timecodes. [VERIFY weights — likely 400/500 mix per W25 spec]

**Seekbar grammar**: container r6 h28 #090909 (never hairline-bordered);
played #371F06 r4 inset 2; playhead #D37918 8×24 r4 (4×24 r2 in waveform);
waveform = #404040 bars 2w/5pitch h2–14; chapter/subtitle segments
white@14% r1; NO circular thumb in the main bar.

**Volume grammar**: 89px wedge track #090909 (11→21px tall L→R); fill width
∝ level with fixed full-width gradient #95D318→#B2A918(57.9%)→#D37918
(79.2%)→#FF3737(100%); no thumb/labels; 50% = 33px.

**Popover panel grammar**: 400 wide r10 #0E0E0E@95%, right-anchored; header
50px #090909 w/ tab row; content inset 20 (360 wide); rows 44px; label
#969696 / value #6B6B6B–#8F8F8F right-aligned; footer Button 360×32 r4
#090909 "Show Advanced Settings".

## C3. Cross-cutting implementation pitfalls

1. **Label color**: menus use #8F8F8F (not #e8e8e8) — W25 spec §2 conflict.
2. **Shortcut size**: same size as labels (≈15px), only color differs.
3. **Menu transparency**: #191919@.95 + blur(50px) — backdrop-filter needed
   for pixel parity, but at 95% opacity a solid fill is visually identical.
4. **Height formula** (C1) — compute panel height; never hardcode 585/683/….
5. **Three hover colors** (C1) — three different list families.
6. **Check vs chevron slot**: selected rows put the check at 19px right
   inset; submenus put it in the ICON slot (x = panel + 30).
7. **Playhead is a bar** (8×24 r4), not a dot.
8. **Pause pill is gray #6B6B6B** — accent only while playing.
9. **Volume gradient is absolute** (anchored to track), green→red; greens/
   reds are volume/EQ-only colors, not theme accents.
10. **`#969696` missing from tokens** (sidebar rows, origin/target labels);
    needs a semantic slot (e.g. `content.row`) from the token lane.
11. **Setting category r2** (not r6) — Settings sidebar exception.
12. **Dropdown field double-layer** = 2px #090909 border on #191919 — keep
    consistent with flat #090909 Buttons; recommend tokens
    `field.bg`/`field.border` so both map cleanly.
13. Icon `Frame` groups inside dropdown triggers are EMPTY placeholders —
    don't render an icon slot there.
14. Sidebar panel is #0E0E0E (deep) next to a #191919 rail — inverted vs
    the player's #191919-panel-on-#090909; don't reuse one token blindly.

## C4. Open questions for the Lead / token lane

- Add `#969696` (row labels) and `#1B1B1B` (compact-list hover) to
  `themes/tokens.json`? (currently absent)
- Confirm intended font sizes: ≈15px menu/timecode, ≈17px section headers,
  ≈12px toggle/tabs — derived from cap-heights, weights unverifiable.
- Volume level→width mapping is non-linear (100% ≈ 71% of track) — confirm
  intended curve [VERIFY].
- Should `Button` Variant2 (#191919) be the "secondary" and default
  (#090909) the "footer/recessed" token pair?
