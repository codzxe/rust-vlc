# Figma fidelity spec — the Wave-25 redesign contract

Status: ACTIVE (Lead, 2026-11-11) · Source of truth: `design/figma/*.svg`
(102 frames, Figma Community "VLC Redesign (Community)", R-07 cleared) with
rendered references in `design/figma-renders/*.png` (cairosvg, width 900).

## 0. What changes and what stays

The 2026-11-09 Solstice v2 brief fixed the FLOW (single-click open, honest
dropdowns, scrims, natural sort, safe padding) — that work STAYS. The v2
AESTHETIC (sunset stage, taped notes, washi tape, keycaps, Nunito, warm
neutrals, yellow default accent) is REPLACED by the Figma package's
language: a compact, near-black, professional desktop player with a single
sparing orange accent. The user verdict on Solstice v2's look is final.

Every screen: same routes, same behavior contracts, same tests — new skin
composed to the Figma frames.

## 1. The extracted design language (measured, W25)

Colors measured across all 102 frames (fill frequency; `scripts/extract_design_tokens.py`,
summary in the wave-25 worklog). Semantic mapping for `themes/tokens.json`:

### Dark mode (default)
| Token (semantic)      | Value (dark)   | Evidence |
|-----------------------|----------------|----------|
| `surface`             | `#090909`      | 560 fills — main window bg (Player, Settings, menus) |
| `surface.deep`        | `#0e0e0e`      | 97 fills — recessed wells (seekbar track, inputs) |
| `surface.panel`       | `#191919`      | 298 fills — sidebar/cards on dark |
| `surface.chrome`      | `#161616`      | 53 fills — title bar / control strip |
| `surface.raised`      | `#222222`      | 73 fills — hover/elevated rows |
| `surface.hover`       | `#262626`      | 50 fills |
| `surface.selected`    | `#1f1f1f`      | 13 fills |
| `surface.muted`       | `#151515`      | 36 fills |
| `stroke.border`       | `#404040`      | 1011 fills + Settings strokes — the workhorse border |
| `content.primary`     | `#c6c6c6`      | 86 fills; menu labels measure `#e8e8e8` |
| `content.bright`      | `#eaeaea`      | 54 fills — headings, active labels |
| `content.secondary`   | `#8f8f8f`      | 1716 fills — meta text, inactive labels, outlined glyphs |
| `content.tertiary`    | `#6b6b6b`      | 501 fills — shortcuts, disabled |
| `accent`              | `#d37918`      | 381 fills — THE VLC orange (default accent becomes orange) |
| `accent.strong`       | `#ab6314`      | 232 fills — pressed/darker |
| `accent.bright`       | `#e8851a`      | hover/lighter |
| `accent.tinted`       | `#371f06`      | 189 fills — accent glow on dark (EQ fill start, selected tints) |
| `content.on-accent`   | `#090909`      | dark ink on orange pills (play button glyph) |
| `record`              | `#e15032`      | red-orange (recording state) |
| EQ band set           | `#e15032 #c49e23 #19a234 #18aad3 #185dd3 #8e42ff #f052b3` (+ oranges) | 18 uses each — equalizer band/level accents |

### Light mode (`Player - Light.svg`)
| Token (semantic)    | Value (light)  |
|---------------------|----------------|
| `surface`           | `#f0f0f0` (chrome `#f5f5f5`) |
| `surface.panel`     | `#efefef`      |
| `content.primary`   | `#2b2b2b`      |
| `content.secondary` | `#6e6e6e`      |
| `stroke.border`     | `#d9d9d9`      |
| icons               | `#9a9a9a`      |
| accent              | `#d37918` (slightly more saturated; seekbar may gradient orange→yellow) |
| video canvas        | STAYS DARK `#2b2b2b–#333` in BOTH modes — the "picture frame" rule |

### Radii (measured rx frequency)
`1` chips · `2–3` micro · `4` inputs/swatches · `6` buttons/dropdowns/menu items (559 uses of 6!) · `10–12` menus/popovers · `12–14` cards/panels (180×12, 98×14) · `19–21` sheets · `30` hero pills. Panels are SHARP-edged (square chrome), rounding lives on controls — this is NOT a bubbly UI.

### Type scale (frames are text-outlined; identities remain [VERIFY] — ship a geometric-sans stack)
10–13px body dominates (compact desktop utility); 13px sidebar/menu medium;
14–15px section headings semibold; timecodes tabular/mono-feel. Sentence
case everywhere, NO all-caps except tiny group labels (Settings sidebar
groups). Font token: `Inter, -apple-system, "Segoe UI", Roboto, sans-serif`
with `font-variant-numeric: tabular-nums` for timecodes. Replace Nunito
throughout (tokens.json `font` groups).

### Accent swatches (Settings → Appearance)
9 swatches in the design: orange (selected), red, yellow, green, teal, blue, purple, magenta, gray — matches the existing 9-accent token system; reorder so ORANGE is default, and align swatch hexes to the design set.

## 2. Per-family layout specs (VLM-measured from the frames)

### Player (dark, `Player.svg` + variants; route `/player`)
- Title bar ~24–32px: orange app dot · sidebar-toggle · wordmark+breadcrumb left · centered app title · window controls right.
- Icon rail ~48–56px wide (design ~24–28 at 1920 but icon targets ≥44px hit areas): vertical nav icons + `+` pinned bottom.
- Sidebar ~200–220px: header (back arrow, "Playlists" 13px medium, `…` + search right), meta row ("2 Items" + sort/list/search icons), `+ Create playlist` row, playlist rows (icon + label). Empty space is fine — this is a quiet rail.
- Canvas: fills the rest; artwork centered; **empty state = the onboarding-style hero, not a blank void**.
- Bottom control bar ~40–48px, three groups: left = stop · **orange play pill (white/dark glyph)** · next; center-left = timecode (tabular) · slim seekbar (3–4px radius ends, orange fill, dark `#0e0e0e` track) · remaining track; right = total time · volume icon · small green level pill · screen/cast · menu · gear.
- Variants: fullscreen, menus-open, hide-UI (`Player - hide UI.svg` — chrome gone, cursor-idle fade), music mode (ambient stage — the ONE screen that may be theatrical), miniplayer (`Miniplayer.svg` — floating card r16–20: video r12 + 1px border, 4–6px progress w/ orange fill + circular thumb, title row, meta row w/ folder/person icons + queue toggle, collapsible Queue panel above with drag-handle rows + durations), customize panel (`Player - customize*.svg`, 9 states).

### Settings (`Settings - *.svg`, 13 areas; route `/settings/:area`)
- Top bar ~36–40px: rounded search pill (dark bg, magnifier, gray placeholder) left · area title · close ✕ right.
- Sidebar ~135–200px, groups **App / Media / System**: General, Interface, Privacy, Controls · Playback, Video, Audio, Subtitles, Library · Network, Performance, Recording, Extensions. Active row: r6 raised bg + orange icon + white label.
- Content: single ~640px column, section headers 15px semibold (Appearance / Startup / Diagnostics…), rows = label left (+ optional ⓘ) · control right (32×16 toggles w/ ON/OFF text, r6 dropdowns, r6 segmented Theme control w/ orange active segment, 9 accent swatches r4 w/ ring on selected, r6 inputs + Browse r6 button).
- Footer: full-width r8 "Advanced Options ›" bar.

### Audio/video tools (`Audio -*.svg`, `Video -*.svg`, `EQ*.svg`; player dialogs)
EQ: collapsible card (r8, "Expand/Collapse Settings" + chevron), Preamp + 10 band rows — label left, value chip (r6 dark `#141414`, white value) right, full-width pill slider below (14px track r7, amber gradient fill `#371f06→#a5621a`, 8×20 r4 amber handle). Compressor/Spatializer/Widener/Visualizer: same card+row grammar. Video Colors/Crop/Essential/HDR: same.

### Playlist (`playlist.svg`, `new playlist.svg`; route `/playlist`)
Black `#000` panel r14, rows ~66px r10 with icon + title + `•••` on hover, hover `#1c1c1c`, text `#9a9a9a`→`#e6e6e6`.

### Menus (`menu.svg`, `*context menu.svg`, `option.svg`, `dropdown.svg`)
400px-wide panels r10–12, bg `#1c1c1c–#202020`, rows 58–60px: 22–24px icon left · label `#e8e8e8` 15–16px medium · right-aligned shortcut `#9e9e9e` 13–14px OR submenu chevron · thin separators · selective orange icon accents (profile, wand).

### Dialogs/flows (`Open network stream.svg`, `Keyboard Shortcuts.svg`, `Onboarding*.svg`)
Onboarding: centered hero (orange traffic-cone/logo art), `#161616` frame r-xl on `#0a0a0a`, inputs `#2a2a2a` r6 border `#3a3a3a` focus-orange. 11 onboarding frames = step flow.

## 3. The comparison loop (how "similar" is judged)

Per screen, per lane:
1. `cd ui && npm run dev` (or reuse) then screenshot with
   `node /home/z/my-project/scripts/dev-shots.mjs <routes>` (writes `/tmp/shots`).
2. Compare against the matching `design/figma-renders/<frame>.png` by running
   the vision CLI with BOTH images:
   `z-ai vision -p "Compare image 1 (current UI) to image 2 (target design). List concrete visual gaps: layout regions, colors, radii, typography, spacing. End with VERDICT: MATCH or GAPS." -i shot.png -i design/figma-renders/Player.png`
3. Fix gaps; repeat until VERDICT: MATCH (or only [VERIFY]-font-identity deltas remain — those are documented, not blocking).
4. Record iterations in the lane worklog entry.

## 4. Non-negotiables (unchanged from the house rules)

- SEMANTIC TOKENS ONLY in components (`var(--…)`, eslint `semantic-tokens-only`); every VALUE lands in `themes/tokens.json` and regenerates via `cargo xtask tokens` (the committed `ui/src/tokens/generated/*` must be regenerated and committed together).
- Config-driven: product name/colors via `product.toml` + themes + locales; ZERO new hardcoded literals (English copy included — i18n catalogs).
- Solstice remnants (sunset/note/tape/keycap token groups, `data-backdrop` vivid tiers, Nunito) are REMOVED with their consumers in the owning lane; `solstice.css` is replaced by a new base stylesheet this wave.
- Existing tests keep passing (update snapshots/visual baselines ONLY for intentional redesign changes; DOM snapshots regenerate with `-u` and get committed — they are now tracked).
- Strict Rust untouched; UI lane is TS/CSS/React only.
- Visual baselines: lanes do NOT re-seed CI baselines; the Lead re-seeds from the CI artifact after the wave merge (runner-rendered truth).
