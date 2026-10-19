# 08 — UI Implementation & Figma Fidelity Plan

**Status:** planning (Wave B) · **Owner:** sub-agent 1.B8 · **Audience:** Lead, UI implementation agents
**Traceability:** MASTER-PROMPT §2 (UI/React+TS, truthful UI), §3.5 (command layer only), §5 (declarative UI, tokens, i18n), §7 (primary source), §10 (web), §13 (testing/CI), §16 (Phase 2/4 gates)
**Companions:** docs/01-architecture.md (crate registry incl. `crates/theme-tokens`), docs/05-command-layer.md (§1 truthful-UI rules C1–C8, §2 command catalog, §3 event catalog, §8 keymaps/menus as data), docs/07-config-driven-system.md (**absent at writing time** — parallel Wave B agent; §3/§8 here are written to reconcile with it once it lands), design/README.md (inventory), design/figma/ (pinned exports).

**Marker conventions.** `[VERIFY-DESIGN]` = value extracted from the SVG exports (per-layer flattened values, not the Figma variables file; includes scale artifacts — see §1.1). `[PROPOSAL]` = invention beyond the brief awaiting Lead decision. `[VERIFY]` = claim requiring external confirmation (e.g., from the Figma file owner). Every numeric value in §1 carries `[VERIFY-DESIGN]` even where the marker is elided for table readability.

---

## 1. Design package intake & analysis

### 1.1 Methodology

The 102 SVG exports (~77 MB; 7 files 3–8.5 MB) are text files and were mined programmatically with `rg` rather than read whole: attribute-frequency extraction (`fill="#…"`, `stroke="#…"`, `stop-color="#…"`, `rx="…"`, `stroke-width="…"`, `fill-opacity="…"`, `backdrop-filter:blur(…)`, `feGaussianBlur stdDeviation`), layer-name extraction (`<g id="…">`, which preserves component/state names such as `Property 1=hover`, `state=open`, `Value=on`, `type=pause`), and text recovery. Three structural facts shape everything below:

1. **All text is outlined.** No `<text>`, `font-family`, `font-size`, `letter-spacing` or `font-weight` attributes exist anywhere in the package (verified by zero-count searches across all 102 files). Every string is a `<path>` whose `id` preserves the layer name — usually the original string (e.g. `id="00:02:10"`, `id="Welcome to VLC!"`). **Consequence:** the type family, weights and exact sizes are *not recoverable from the exports*; only geometry is. Cap-heights below were measured from path anchor coordinates (Python bbox script over `M`/`L` anchors). Font identity is `[VERIFY]` with the file owner (§2).
2. **Component exports carry annotation chrome.** Small component files wrap content in a `0.5`-inset rect stroked `#8A38F5` dashed `10 5` (`stroke-dasharray`) with content inset 20px. This purple frame is Figma's export annotation, **not design** — it must be excluded from token extraction and from screenshot baselines (crop or mask). The `#8A38F5` / `#F052B3` occurrences that are *not* this frame are real UI (accent swatches).
3. **Some artboards are not at 1× scale.** Onboarding frames measure e.g. `15.4px` control height, `rx="3.3"`, `4.4px` knob — exactly 1.1× the `14 / 3 / 4` values seen at 1× elsewhere; scaled icon copies show `stroke-width` `0.825 / 1.125 / 1.28571`. Token derivation must therefore **normalize against known anchors** (button height 28, seek knob 8×24, control bar 54) before recording a value, and record the artboard scale used. `[PROPOSAL]` the extractor computes scale per file from anchor geometry.

Figma's *background blur* is exported as a CSS `backdrop-filter:blur(25px)` hint plus `data-figma-bg-blur-radius="50"` (Figma blur 50 ⇒ 25px CSS); the 13.75px/27.5 pair appears on smaller panels. Drop shadows export as `feColorMatrix`+`feGaussianBlur`+`feOffset` chains (samples: offset dy 4.76, blur σ 4.76 and 5.9, black at 38% and 70% alpha). `[VERIFY-DESIGN]`

### 1.2 Color palette actually used (hex frequency, whole corpus)

**Dark theme (default) — neutral ramp** (fill counts across 102 files; strokes add ~2.4k more of `#8F8F8F`):

| Role (observed) | Hex | fill count | Notes |
|---|---|---|---|
| Deepest well (seek track, chips, input bg, toggle track, EQ preset chips) | `#090909` | 560 | near-black |
| Playlist/settings panel surface | `#0E0E0E` @ 0.95 | 97 | translucent over video |
| Search field | `#151515` | 36 | settings search 200×34 |
| Secondary surface (icons rail, menus, dropdown list, EQ panel) | `#191919` | 298 | |
| Topbar / control bar / window border / music bg | `#1B1B1B` | 64 (+ stroke 4, width 20) | |
| Hover / selected item fill | `#1F1F1F` | 13 | menu items, dropdown selection |
| Selected settings category | `#222222` | 73 | 188×40 rx2 |
| Misc deeper surfaces | `#262626`, `#161616`, `#171717`, `#2E2E2E`, `#373737`, `#202020`, `#0C0C0C`, `#1E1E1E` | 50/53/29/11/39/1/1/4 | long tail — candidate for consolidation (§3) |
| Thumbnail / placeholder base, EQ outer | `#303030` | 114 | also *light-theme text* |
| Off-knob, waveform bars, tertiary | `#404040` | 1011 | high count via sidebar/library art |
| Primary icon/label gray | `#8F8F8F` | 1716 (+3221 strokes) | the workhorse foreground |
| Secondary text | `#969696`, `#AFAFAF`, `#C6C6C6` | 224/36/86 | |
| Tertiary/disabled text, OFF labels | `#6B6B6B` | 501 | |
| Brightest text (Miniplayer titles) | `#EAEAEA` | 54 | |

**Accent family (orange)** — the product accent, dominant brand signal:

| Hex | Observed use | Count |
|---|---|---|
| `#D37918` | primary accent: play button, seek knob, ON toggle label, selected swatch, onboarding primary button, selection rings | 381 |
| `#E8851A` | light-theme knob / brighter accent | 2 |
| `#C2711A` | light-theme accent variant (icon tints) | 3 |
| `#AB6314` | darker accent: ON-knob, EQ thumbs, pressed/hover accent fills | 232 |
| `#371F06` | accent-tinted dark fill: seek progress, segmented selected, toggle ON track | 189 |
| `#D8BA9A` | light-theme progress tint | 1 |
| `#FF0000` | record indicator (2 uses) | 2 |

**Secondary accent set** (settings "Accent color" swatches, drawn at 70% opacity except the selected one; also `#1A1611` warm-dark stroke in Miniplayer): `#E15032` (red-orange), `#C49E23` (yellow), `#19A234` (green), `#18AAD3` (cyan), `#185DD3` (blue), `#8E42FF` (violet), `#F052B3` (pink), `#AFAFAF` (gray), plus default `#D37918`. **This is the runtime theming surface: the user-selectable accent (Onboarding-3…10 iterate the same screen with each swatch selected).**

**Audio-visualizer gradient** (50 stops each): `#FF3737 → #D37918 → #B2A918 → #95D318` (red→orange→gold→lime).

**Light theme** (from `Player - Light.svg`; the only light frame in the package):

| Dark value | Light value | Element |
|---|---|---|
| `#1B1B1B` topbar/control bar | `#D5D5D5` @ 0.75 | topbar & icon rail |
| `#191919` icon rail | `#D5D5D5` @ 0.75 | |
| `#0E0E0E` @ 0.95 playlist panel | `#E7E7E7` @ 0.8 | |
| `#090909` seek track | `#DEDEDE` | |
| `#371F06` progress fill | `#D8BA9A` | |
| `#D37918` knob | `#E8851A` | brighter for contrast on light |
| `#D37918` play button | `#D37918` | **accent constant across themes** |
| `#8F8F8F` icons/labels | `#2E2E2E` / `#303030` | |
| `#969696` secondary | `#303030` / `#ACACAC` | |

**Opacity as a first-class token:** surface translucency `0.95` (169 uses) and `0.8`; scrim/overlay `0.2` (85), `0.14`, `0.24`; element opacity `0.5` (76), `0.3` (25), `0.7` (16); swatch rest-state 0.7. Theme system must carry alpha separately from hue (§3).

**Interpretation.** The corpus uses a ~10-step warm-neutral gray ramp plus a single orange accent with 4 tonal variants and 9 selectable accent hues — consistent with a token system of `surface / surface-raised / surface-overlay / content-primary / content-secondary / content-tertiary / accent / accent-strong / accent-tinted` roles rather than raw hex. Long-tail near-duplicates (`#161616` vs `#171717` vs `#1E1E1E`) are export noise or one-offs; the consolidation rule is in §3.4.

### 1.3 Typography

Text is outlined (§1.1), so only geometry is measurable. Cap-heights (digits/caps, descenders excluded where noted):

| Measured cap-height (px) | Example string (file) | Derived font-size ≈ cap/0.72 `[VERIFY-DESIGN]` | Working token name `[PROPOSAL]` |
|---|---|---|---|
| 4.4 | `ON`/`OFF` toggle micro-labels (Toggle.svg) | ~6 | `font-micro` |
| 7.8 | `Search` placeholder (Settings - global), `Save` (network dialog) | ~11 | `font-caption` |
| 10.3 | `Videos` topbar, `Action`, menu items `Option`, `Accent color`, `Cancel` | ~14 | `font-body` |
| 11.1 | `00:02:10` time digits (tabular) | ~15–16 | `font-time` (mono/tabular figures) |
| 11.8 | `Next`, `Network URL` | ~16 | `font-label` |
| 13.0 | `VLC Media Player` topbar title, `Keyboard Shortcuts` / `Open Network Stream` dialog titles, `App` settings group | ~18 | `font-title` |
| ~13–15 | `Playlists` sidebar header, `Global Settings` settings title | ~18 | `font-title` (descender-inclusive 14.9/15.1) |
| 23.6 | `Welcome to VLC!` (Onboarding) | ~32 | `font-display` |

Time digits at `52.4px / 8 chars` (6.55px advance) with 11.1 cap-height indicate a **condensed or tabular-numeral face for time** distinct from the UI face — or tighter tracking `[VERIFY-DESIGN]`. Weights: at least two (regular + medium/bold appear in headers vs body — inferred from stroke thickness of outlines, not measurable precisely `[VERIFY-DESIGN]`). **The family itself is unknown**; the interim stack must bundle a metric-compatible open font and be re-derived when the Figma file is obtained (§2). Letterform proportions (cap ≈ 0.72 em) match common geometric sans stacks (Inter/Poppins/SF-like) `[VERIFY-DESIGN]`.

### 1.4 Spacing, radii, elevation

**Radius scale** (frequency across corpus): `1` (1027 — micro rounding on swatches/thumbs), `2` (316 — sharp panels, EQ, selected category), `3` (188 — small knobs/insets), `4` (354 — buttons inner, seek fill, thumb), `5` (163 — thumbnails), `6` (559 — default control radius: buttons, menus items, segments, toggle, seek), `4.5` (35 — **annotation frames, exclude**), `10` (51 — context menu, popover), `12` (180 — onboarding theme cards, settings rows), `14` (98 — settings window, miniplayer cards), `19/20` (10 — main window + 20px border stroke), `21` (9 — theme-card selection ring), `30` (9 — large cards). Working radius ramp: **2 / 4 / 6 / 10 / 12 / 14 / 19** (+1 for hairline rounding) `[PROPOSAL over VERIFY-DESIGN data]`.

**Key geometry** (all `[VERIFY-DESIGN]`, dark 1920×1080 frame unless noted):

| Element | Geometry |
|---|---|
| Window | 1920×1080, corner radius 19, 20px `#1B1B1B` border; frameless custom chrome (topbar is in-app) |
| Topbar | full width × 40, `#1B1B1B`; music mode identical |
| Bottom control bar | full width × 54, `#1B1B1B` |
| Left icon rail | 50 wide × 986, `#191919` |
| Playlist panel | 377 wide × 986, `#0E0E0E` @ 0.95 |
| Play button | 54×28 rx6 `#D37918`, glyph `#161616`; `type=play` / `type=pause` states |
| Seek bar | track 1238×28 rx6 `#090909`; progress fill inset 2 (98×24 rx4 `#371F06`); handle 8×24 rx4 `#D37918` (a slim vertical bar, not a circle) |
| Music-mode seek | waveform bars 2px wide `#404040`, knob 4×24 rx2 |
| Album art (music) | 737×737 rx20, full-bleed blurred art behind (`#1B1B1B` base) |
| Settings window | 1048×865 rx14 `#0E0E0E` @ 0.95, backdrop-blur 25px, drop shadow; header 50 `#090909`; floats over player |
| Settings sidebar | 216 wide; list 200×799 rx6 `#191919` @ 0.95; selected item 188×40 rx2 `#222222`; item pitch ~42 |
| Segmented control | container 320×36 rx6 `#090909`; segments 104×32 rx4; selected `#371F06`, unselected `#191919` |
| Context menu | 265×296 rx10 `#191919` @ 0.95 + 4px inset ring; items 48 high, hover 257×48 rx6 `#1F1F1F` |
| Dropdown | trigger 364×48 rx6 `#090909` (inner 360×44 rx4 `#191919`); open list 364×232 rx6 `#191919`, selected item `#1F1F1F`; `state=default/open`, `state=dropdown 1/2` |
| Button (generic) | 360×32 rx4; `Property 1=Default` `#090909`, `Property 1=hover` `#191919` |
| Toggle | 68×32 rx6 `#090909` track; ON: fill 33×26 `#371F06` + knob 26×26 rx3 `#AB6314` right; OFF: knob `#404040` left; micro labels `#D37918`/`#6B6B6B` |
| Slider (EQ, vertical) | track 16, fill 12, thumb 8×24 rx4 `#AB6314`; EQ panel 784×489 rx2 `#191919`; preset chips 60×32 rx6 `#090909` |
| Miniplayer | card 391×339 rx14 `#1B1B1B` + shadow (dy 4.76, blur 4.76, black 38%); thumbnail 373×209 rx5 + 2px `#090909` stroke; seek 375×16 rx6, knob 8×12 rx4; expanded variant 391×689 with 375×331 rx10 section; `Property 1=Default/…` variants |
| Onboarding | window 1090×759 rx14 `#0E0E0E`; theme cards 100×100 rx12 (light `#AFAFAF`, dark `#222222`, system gradient), selection ring 120×120 rx21 stroke `#D37918` 4px; primary button 100×32 rx6 `#D37918`; text input 469×34 rx5 `#090909` + 2px `#191919` stroke; step progress 15.4-high bars rx3.3 (1.1× artboard) |
| Icon system | line icons, stroke `#8F8F8F`, stroke-width 1.5 (3464 uses; 2 and 3 for emphasis); hit wrapper ~48 (Icon Wrapper.svg) |

**Spacing grid:** insets of 8 (panels), 20 (annotation padding — not design), item pitch 42–48, control heights 28/32/34/36/48, and a consistent 2px inner inset on filled tracks. The data is consistent with a **4px base grid** with 8px component padding `[PROPOSAL over VERIFY-DESIGN data]`.

### 1.5 Component inventory (from component-level exports)

Extracted states/names (these are the *only* interaction states present in the package — see §2 gap list):

| Export | Component | States / variants observed |
|---|---|---|
| `Button.svg` | Button | `Property 1=Default` `#090909`, `Property 1=hover` `#191919` (360×32 rx4) |
| `Toggle.svg` | Toggle switch | `Value=on`, `Value=off` |
| `Dropdown.svg` | Dropdown/select | closed trigger; `state=open`; `state=dropdown 1/2` list variants; selected item |
| `option.svg` | Menu option row | `Property 1=Default` (no fill), `Property 1=hover`, `Property 1=selected` (48h rows) |
| `Controls.svg` | Control-bar kit | `play` ×15, `volume` ×15 (mute/low/med/high × states), `range` ×5, `text_input` ×12, `option` ×6, `wrapper` ×7, `degrees` — the full control strip states |
| `play.svg` | Play/pause button | `type=play`, `type=pause` |
| `volume.svg` | Volume flyout | vertical slider, stroke-based, 129×645 |
| `Values.svg` / `Values-1.svg` | Value steppers | `option=1/2, number=2/3` variants |
| `Current Time.svg` / `Duration.svg` | Time readouts | `state=Default`, `state=click` |
| `Icon.svg` / `Icon Wrapper.svg` | Icon + 48px hit area | default |
| `Setting Preset.svg`, `Profiles.svg`, `setting category.svg`, `origin.svg`, `target.svg`, `sidebar.svg`, `media.svg`, `thumbnail.svg`, `audio value.svg` | Settings/library atoms | assorted |
| `context menu.svg`, `file/folder/playlist context menu.svg`, `right-click menu.svg`, `menu.svg` | Menus | items + accelerators (menu.svg: Open File(s) `Ctrl+O`, Open Folder `Ctrl+F`, Open Network Stream `Ctrl+N`, Open Disc `Ctrl+D`, Open Capture Device `Ctrl+C`, Convert `Ctrl+R`, Open Recent ▸, Keyboard Shortcuts, Customize Player, Settings `Ctrl+P`, About VLC `Shift+F1`) |
| `Components.svg`, `Designs.svg` | Overview sheets | `Components.svg` is a single outlined word-mark (no component data); `Designs.svg` similar cover sheet |

**Cross-reference finding (doc 05):** the design's menu accelerators partially conflict with the doc 05 §8.2 seed keymap (`Ctrl+Shift+O` for open URL vs design `Ctrl+N`; `Ctrl+F` folder; `Ctrl+C` capture conflicts with copy conventions). Resolution belongs to the keymap data file, not code — both sources feed `config/keymaps/default.toml`, design wins on visible chrome `[PROPOSAL]`, flagged for Lead.

### 1.6 Screen inventory → routes

All 102 files, grouped; route names are `[PROPOSAL]` (config-driven layouts will own actual composition, MP §5):

| Route (proposed) | Exports | Notes |
|---|---|---|
| `player` (video, dark) | `Player.svg`, `Player-1..6.svg` | 1–4: library sidebar variants (Home/Videos/Music/Network/Playlists/Recent, "The Wand" show); 5: Home screen ("Continue Watching", 110 items, search); 6: compact variant |
| `player` (light) | `Player - Light.svg` | §1.2 light mapping |
| `player.music` | `Player - Music.svg`, `-1` | album art 737², waveform seek |
| `player.hide-ui` | `Player - hide UI.svg` | video only, no chrome (unobtrusive-controls target state) |
| `player.customize` | `Player - customize.svg`, `-1..8` | control-bar editor: buttons (Playback Speed, ±10s/±15s, Stop, Repeat, Shuffle, Record, Snapshot, PiP, HDR, Zoom, Right Sidebar, Quit, Settings, Go Home) + display toggles (Thumbnail, File/Artist/Folder/Playlist Name, Properties, Replay, Chapters, Favorite, Bookmarks) — **direct input for the customize panel's data model** |
| `miniplayer` | `Miniplayer.svg` | 4 stacked variants (default, overlay scrim, expanded+playlist, …) |
| `settings/{area}` ×13 | `Settings - global(-1)`, `interface(-1)`, `privacy`, `playback(-1)`, `video(-1)`, `audio(-1)`, `subtitles(-1)`, `controls`, `library`, `network`, `performance`, `recording`, `extensions` (20 files) | design/README says "14 areas" — 13 distinct areas found on disk `[VERIFY-DESIGN]`; `-1` files are second states (scrolled/expanded), not light theme. Maps to the settings-schema UI placement (MP §5, doc 07) |
| `onboarding` ×11 | `Onboarding.svg` (welcome + media folder), `-1` (theme: Light/Dark/System), `-2` (shortcuts + Finish), `-3..-10` (theme+accent with each of 8 alternate accents selected) | flow: welcome → folder → theme+accent → shortcuts → finish |
| `keyboard-shortcuts` | `Keyboard Shortcuts.svg` | searchable table: Action / Hotkey / Global Hotkey; sections Basic/Playback/… |
| `open-network-stream` | `Open network stream.svg` | URL, "cascading" playlist title, Cancel/Save |
| `playlist` (new) | `new playlist.svg`, `playlist.svg` | details form + drag-and-drop file box ("Drag files into the box or click here…") |
| `playlist/subtitles` flow | `Add Subtitles to Playlist/Folder{,-1..3}.svg`, `subtitle one/multiple.svg`, `sub filename.svg` | `state=from computer/from opensub/to this file/to this folder` |
| context menus | `context menu.svg`, `file/folder/playlist context menu.svg`, `right-click menu.svg` | |
| audio tools | `Audio - Equalizer.svg`, `EQ.svg`, `EQ presets.svg`, `Audio - Compressor/Spatializer/Stereo Widener Settings.svg`, `Audio Visualizer.svg`, `audio value.svg` | EQ panel rx2 — deliberately sharper |
| video tools | `Video - Colors/Crop/Essential.svg`, `HDR Options.svg` | |
| library atoms | `sidebar.svg`, `media.svg`, `thumbnail.svg`, `Sorting Options.svg`, `View Options.svg`, `Profiles.svg`, `Setting Preset.svg`, `origin.svg`, `target.svg`, `menu.svg`, `option.svg`, `dropdown.svg`, `Controls.svg`, `Button.svg`, `Toggle.svg`, `Values{,-1}.svg`, `Current Time.svg`, `Duration.svg`, `Icon{, Wrapper}.svg`, `play.svg`, `volume.svg`, `setting category.svg`, `audio value.svg`, `Components.svg`, `Designs.svg` | component-level exports (§1.5) |

---

## 2. Figma source-of-truth strategy (MP §7 bullet 1)

**What the exports give us:** composition, exact geometry, effective colors (flattened per layer), radii, translucency, blur/shadow parameters, real UI copy (via outlined-path ids), the accent-set, and a partial state matrix (§1.5).

**What only the original Figma file (or a variables export) can give:**

1. **Variables/collections** — the actual token names, modes (dark/light/accent), and the *aliasing* the designer used (e.g., whether `#371F06` is "accent/tinted" or a manual value). MP §7 explicitly requires obtaining "the file link or exported frames plus variables".
2. **Typography** — family, weights, optical sizing, line-height, letter-spacing, and text styles (§1.1: exports carry none).
3. **Complete component state sets** — MP §7 enumerates hover, pressed, focus, disabled, fullscreen, loading, error, empty. The exports cover **hover, click/pressed, open, selected, on/off, play/pause, type/default variants** — i.e., roughly half. **Missing entirely: focus-visible, disabled, fullscreen chrome, loading, buffering, error, empty states.** These must be *designed*, not guessed: they are engine-truth states (§5) and a11y-critical (§4.5).
4. **Auto-layout / constraints** — responsive behavior, min/max sizes, and how panels reflow (the exports are fixed 1920×1080 or component-sized).
5. **Prototype wiring** — intended navigation and animation timing (the `-1..N` sequences imply transitions we cannot read).

`[VERIFY with file owner]` — action for Lead: request the Figma Community file link + variables export (or REST export with `variables` scope), and confirm **license terms before any commercial use** (risk R-07, design/README.md; Figma Community files are typically remix-allowed but the underlying font license and redistribution of derived assets need checking — relevant because we will *rebuild* the UI, and screenshot baselines will embed derived rasterizations).

**Interim strategy (now → variables arrive):**

- Derive tokens from the SVGs per §1 with the `[VERIFY-DESIGN]` markers, normalized for artboard scale; record extraction provenance in the token file (`$description` / `$extensions` metadata).
- **Re-derive when variables arrive**: the token file (§3) is regenerated, semantic names re-mapped to Figma variable names, and the screenshot baselines (§6) re-pinned. Budget one explicit "token re-derivation" task in Phase 1 — cheap now, expensive after components hardcode anything.
- Missing states are specified in this doc (§4, §5) as **design proposals `[PROPOSAL]`** derived from the system's own logic (e.g., disabled = `content-tertiary` at 0.5 opacity, focus = 2px accent ring at 2px offset matching the onboarding selection-ring language). These await either Lead approval or the Figma file's own state frames, whichever comes first.
- The 8-alternate accent set (§1.2) is treated as a runtime-selectable mode: tokens must be parameterized by accent hue, not just light/dark (§3.3).

---

## 3. Token pipeline (MP §7 bullet 2)

### 3.1 Format and flow

```
Figma variables (or SVG-derived interim, §1)
        │  (extraction, scale-normalized)
        ▼
themes/tokens.json  — W3C Design Tokens draft format [PROPOSAL]
        │  cargo xtask tokens  (tools/xtask, calls crates/theme-tokens)
        ├──▶ ui/src/tokens/generated.css     (CSS custom properties, per theme attribute)
        ├──▶ crates/theme-tokens/src/generated.rs  (Rust constants for engine-side needs)
        └──▶ themes/*.css / built-in theme manifests (user-selectable, product.toml default)
```

The **W3C Design Tokens JSON** format (draft) is the proposed interchange: it has groups, `$value`/`$type`, aliases (`{accent.primary}`), and per-mode values — matching our light/dark/accent needs and being an emerging standard with existing tooling (Style Dictionary can consume it). `[PROPOSAL]` — Lead approves the format; the fallback is a simpler bespoke schema with the same layering.

Per doc 01 §2.3, `crates/theme-tokens` owns parse/emit (`TokenSet`, `parse`, `emit_css`, `emit_rust`) with golden tests; `tools/xtask tokens` is the CLI entry (MP §4 lists xtask "tokens"). The UI imports only generated CSS + a generated TS type map; the engine side gets Rust constants (e.g., renderer clear colors, subtitle-safe margins) — **but no engine component may consume a UI-semantic token for non-theming purposes** (ARCH-3: engine owns no UI state).

### 3.2 Naming grammar — three layers

```
primitive (raw, extraction-accurate, never consumed by components)
  gray.900=#090909 … gray.40=#AFAFAF, orange.500=#D37918, orange.600=#AB6314,
  orange.700=#371F06, orange.300=#E8851A, radius.6=6px, space.4=4px, font.body=14px…
alias (theme-role, the Figma variables we expect to recover)
  surface, surface.raised, surface.overlay, content.1/2/3, accent.primary,
  accent.strong, accent.tinted, stroke.icon…
semantic (what components consume — the ONLY allowed reference)
  --color-surface-raised, --color-content-secondary, --color-accent-tinted,
  --radius-control, --space-inset, --font-size-body, --shadow-overlay, --blur-backdrop
```

Rules `[PROPOSAL]`:
- Components reference **semantic tokens only** (MP §7: "Components consume semantic tokens only"). PR lint (§8) rejects raw `#hex`, raw `rgb()`, raw `px` font-sizes/radii in TSX/CSS-in-source; `var(--…)` only.
- Semantic tokens resolve through aliases, never directly to primitives; primitives exist for re-derivation and diffing against future Figma variables.
- Token names describe *role*, never *value* (`--color-surface-raised`, not `--color-gray-800`) — so light/dark/accent switch by re-pointing aliases.

### 3.3 Theming mechanics (per the design package)

- **Modes:** `dark` (default), `light`, plus **9 accent selections** (`#D37918` default + 8 alternates §1.2). Implemented as `[data-theme="dark|light"]` + `[data-accent="orange|red|yellow|green|cyan|blue|violet|pink|gray"]` on the root element; CSS custom properties swap per attribute; no JS style recalculation beyond attribute set.
- Accent derivatives must be *computed* per accent hue unless the design supplies them: the observed family is primary `#D37918`, strong/darker `#AB6314`, tinted-on-dark `#371F06`, on-light `#E8851A`/`#C2711A`, light-progress `#D8BA9A`. For the 8 alternate accents we only have the primary swatch `[VERIFY-DESIGN]` — derive strong/tinted by HSL shifts and validate contrast ≥ 4.5:1 for text-on-accent `[PROPOSAL]`, re-pin when the Figma file provides the real ramp.
- The **accent is user-configurable** (settings "Accent color", onboarding step) → it belongs to the settings schema (doc 07) with token-backed rendering; `product.toml` carries the default accent (MP §5 brand fields).
- **Translucency + backdrop blur are theme facts:** surfaces over video use alpha 0.95/0.8 + `backdrop-filter: blur(25px)` (13.75px small panels). Webview support for backdrop-filter must be verified per platform (§7 risk cousin); token carries blur radius, CSS applies with `@supports` fallback to solid fill `[PROPOSAL]`.
- **Shadows** (miniplayer: dy 4.76 / blur 4.76 / 38% black; stronger variant 70%) become `--shadow-card`, `--shadow-overlay` `[VERIFY-DESIGN]`.

### 3.4 Validation & hygiene

- `cargo xtask tokens --check` in CI: regenerated output is byte-stable (no drift between committed CSS/Rust and tokens.json).
- **Lint rule** (ui/ ESLint/Stylelint): no raw hex/px in `components/**` and `screens/**`; only `var(--semantic-…)`. Allowlist for the generated files and SVG-asset fills.
- **Coverage check**: every semantic token is consumed ≥ once (dead-token detector) and every component-facing token exists in all modes (dark/light × default accent at minimum; alternate accents validated programmatically for contrast).
- **Consolidation policy**: near-duplicate primitives (`#161616/#171717/#1E1E1E`) map to the nearest canonical step *only if* visually indistinguishable at 1× (ΔE threshold, documented); otherwise they stay distinct `[PROPOSAL]`. This is recorded in the token file metadata so re-derivation from Figma variables can revisit each merge.

---

## 4. Component architecture (React + TypeScript)

### 4.1 Structure

```
ui/src/
  tokens/        generated.css + generated token types (§3) — committed build artifacts
  components/
    primitives/  Button, IconButton, Toggle, Dropdown, Slider (range + EQ vertical),
                 SegmentedControl, ContextMenu, MenuItem, Tooltip, TextInput, SearchField,
                 TimeDisplay, Icon (sprite/registry), Swatch, WaveformSeek, Visualizer
    composites/  ControlBar, SeekBar, VolumeControl (flyout), TrackMenu (audio/video/sub),
                 PlaylistPanel, PlaylistRow, SettingsScreen (+ SettingsSidebar, SettingsRow),
                 OnboardingFlow, CustomizePanel, MediaInfoPanel, KeyboardShortcutsScreen,
                 NetworkStreamDialog, NewPlaylistDialog, Miniplayer, AudioToolPanels (EQ,
                 Compressor, Spatializer, StereoWidener, Colors, Crop, Essential, HDR)
    overlays/    MenuBar (app menu), ContextMenuHost, DialogHost, ToastHost [PROPOSAL]
  screens/       PlayerScreen (video/music/hide-UI modes), HomeScreen, LibraryScreen,
                 SettingsScreen router, OnboardingScreen, MiniplayerScreen — thin: layout
                 from config (layouts/*.toml, doc 05 §8.3 / doc 07) + composites
  state/         UI-only stores; command-layer client wrappers (§4.3)
  bindings/      generated command-layer client (doc 05 §5.1 schema-first codegen)
  i18n/          generated locale packs (doc 07 / MP §5 localization)
```

Primitives map 1:1 to the exported atoms (§1.5); composites map to the screen regions in §1.6. The **customize panel** is fully data-driven: its button catalog (±10s/15s, speed, PiP, HDR, …) and display toggles become `layouts/customize.toml` entries referencing commands/UI-actions (doc 05 §8) — the design's own list is the seed `[PROPOSAL]`.

### 4.2 State management boundaries (MP §2, §3.5, doc 05 §1)

- **UI-owned:** hover/focus/pressed, open/closed panels, dialog stacks, drag affordances, layout mode (music/hide-UI/mini), uncommitted form state (settings edits before Save), selected playlist row (presentation), theme/accent applied to DOM.
- **Engine-owned, UI-rendered:** playback state, position/duration, volume/mute, track lists/selection, fullscreen, rate, buffering, errors, media info, playlist contents/order. The UI **subscribes to events and queries via the command layer; it never caches-past or predicts engine state** beyond the explicit pending-pattern of §5.
- **Never duplicated:** a single canonical snapshot store fed by `playback.state-changed`/`time-tick` events (doc 05 §3); UI components read via selectors. No second source of truth in component trees; derived values (formatted time) memoized from the snapshot.
- Bindings: `bindings/` is *generated* from the command schema (doc 05 §5.1) — hand-written IPC calls are lint-forbidden (ARCH-5 enforcement in ui/).

### 4.3 MP §7 required-UX checklist → components

| §7 requirement | Owner(s) | Notes |
|---|---|---|
| Playback canvas, unobtrusive controls | PlayerScreen + ControlBar; `player.hide-ui` state | auto-hide on idle in fullscreen `[PROPOSAL]` timing; fade per reduced-motion (§4.5) |
| Seek | SeekBar (28px track / 8×24 bar knob), WaveformSeek (music) | pending-seek rendering §5 |
| Volume | VolumeControl (icon + vertical flyout, states mute/low/med/high × hover from Controls.svg) | |
| Fullscreen | ControlBar button + `video.set-fullscreen`; window-state events | |
| Track menus | TrackMenu ×3 (dynamic from `tracks.list`, doc 05 §8.3) | |
| Playlist & queue | PlaylistPanel 377px, rows, `playlist.*` commands; drag-and-drop reorder | |
| Media information | MediaInfoPanel (`media.get-info`) | design has no dedicated frame `[VERIFY-DESIGN]` → proposal styling from settings language |
| Settings by area | SettingsScreen router ×13 areas; rows generated from settings schema (doc 07) | segmented controls, toggles, swatches, steppers (§1.5 atoms) |
| Light + dark themes | token modes (§3.3) | |
| Drag-and-drop | PlaylistPanel, NewPlaylistDialog drop-zone, player surface open-file | |
| High-DPI | SVG-asset icons + CSS logical px; tested at 1×/1.5×/2× (§6) | |
| Keyboard navigation | focus order per §4.5; keymap data (doc 05 §8) | |
| Focus states | visible focus ring token (§2 interim design) | |
| Accessible labels | §4.5 | |
| Loading / buffering / unsupported / error states | §5 surfaces | **not present in exports — designed here `[PROPOSAL]`** |

### 4.4 Interaction states (component × state matrix)

Every interactive primitive implements: `rest`, `hover`, `active/pressed`, `focus-visible`, `disabled`, and where relevant `selected`, `checked`, `open`, `dragging`, `pending` (§5). Exports supply rest/hover/pressed/selected/open/on-off (§1.5); focus/disabled/pending follow the §2 interim rules: focus = 2px `accent.primary` ring, 2px offset (matches onboarding ring language); disabled = content colors at 0.5 opacity + `pointer-events: none`; pending = 0.7 opacity + spinner affordance `[PROPOSAL]`.

### 4.5 Accessibility spec `[PROPOSAL]`

- Roles: buttons → `button` (icon-only require `aria-label` from i18n keys); SeekBar → `slider` with `aria-valuenow/min/max/text` (formatted time); toggles → `switch`; menus → `menu`/`menuitem` with `aria-checked`/`aria-disabled`; dialogs → `role="dialog"` + focus trap; playlist → `listbox`/`option`; settings rows → labelled groups.
- Focus order: DOM order = visual order (topbar → canvas → control bar → panels); roving tabindex in menus/playlist; `Esc` closes overlays (matches Onboarding "Back/Esc" and Keyboard-Shortcuts "Exit fullscreen / close dialog: esc").
- Reduced motion: `prefers-reduced-motion` disables auto-hide fades, waveform animation, visualizer motion; buffering spinners become static labeled text.
- Contrast: enforce ≥4.5:1 text, ≥3:1 UI components across dark/light × all 9 accents in CI (axe + custom token-contrast check §3.4).
- Live regions for engine-truth announcements (buffering start/end, errors) — `aria-live="polite"`; time updates *not* announced.

---

## 5. Truthful UI patterns (MP §2, doc 05 §1 C1–C8)

The UI distinguishes **accepted** (command dispatched, `request_id` outstanding) from **completed** (engine event confirms). Nothing renders as done before the engine says so; nothing renders as failed without a typed error. All engine-visible state comes from the doc 05 §3 event stream / query results.

### 5.1 PlayButton — states × engine states

| PlayButton visual | engine: idle | loading | playing | paused | buffering | ended | error |
|---|---|---|---|---|---|---|---|
| `disabled` (opacity 0.5) | no media open | — | — | — | — | — | — |
| `play` glyph, rest | ✓ | | | ✓ | | ✓ (restart affordance) | |
| `pause` glyph | | | ✓ | | | | |
| `pending` (0.7 + spinner) | | `session.open` accepted | `playback.play/pause` accepted, ≤250 ms | idem | — | — | — |
| `play` + error toast | | | | | | | open failed → typed error (§5.4) |

Pending resolves on `playback.state-changed` (doc 05 §3) or the command completion; a stale `request_id`/generation is dropped (doc 05 §4). Note `session.open` is *never* optimistic: the play glyph appears only when `loading → playing` transition is observed.

### 5.2 SeekBar — pending seek rendering `[PROPOSAL]`

1. User scrubs → local **preview position** (thumb follows pointer) while `--accent-tinted` fill stays at the *engine-reported* position.
2. On commit (`change`/pointer-up) → dispatch `playback.seek` with `request_id`; bar enters pending: thumb locked at target, fill still at engine position; new input cancels the outstanding request (client-side, per doc 05 §4 staleness rules).
3. `playback.position-changed`/`state-changed` with matching generation → fill animates to engine truth; mismatch → re-render from snapshot, no fabricated jump.
4. Failed seek → typed error surface (§5.4) + fill snaps back to engine position; never leaves the bar at the failed target.
While buffering, `aria-busy` is set; the knob is never hidden by a spinner — the canvas overlay carries that state instead.

### 5.3 Buffering / loading / empty overlays (engine-event-driven)

- **Loading (open/probe):** center overlay on the video canvas, `loading` label (i18n key), auto-dismissed by first `playing` (or replaced by error). Driven by doc 05's playback states — no timer-based "fake progress".
- **Buffering:** subtle canvas indicator + `state=buffering` event; control bar remains interactive (volume/track commands still valid); seek is disabled-with-reason while the engine reports non-seekable media (`media.get-info`), not by guesswork.
- **Empty:** player with no media = the exports' chrome with an empty canvas + "Open media" affordance (design: no empty-state frame exists `[VERIFY-DESIGN]` → composed from existing atoms `[PROPOSAL]`).
- **Unsupported:** typed `unsupported` error → distinct surface naming format info from the error payload (MP §8: "known format info"), with "no playback attempted" wording — never a black frame.

### 5.4 Error surfaces

Every typed error (doc 05 §2 taxonomy: unsupported / corrupt / inaccessible / network / decoder / device / resource-limit) renders: stage-named message from i18n, retry/alternate-action buttons only where the command layer offers a meaningful action, and a "copy details" affordance with the redacted error id (doc 05 §1 redaction rules). Errors are **events, not exceptions in the UI**: one `ErrorBoundary` per overlay region so a render defect cannot blank the player canvas (canvas is engine-owned regardless, §7).

### 5.5 Component-state tables owned by this doc

Deliverables for implementation: one markdown state table per primitive (PlayButton above, SeekBar, VolumeControl incl. mute-pending, TrackMenu incl. disabled-while-loading, PlaylistPanel incl. reorder-pending, Settings rows incl. restart-required badges from the settings schema, dialogs incl. Save-pending). These tables live in `ui/src/components/**/states.md` and are enforced by the interaction tests (§9.2) `[PROPOSAL]`.

---

## 6. Screenshot-diff acceptance testing (MP §7 bullet 3)

**Definition of "pixel-exact" (adopted verbatim from MP §7):** layout, spacing, color, radii, effects and typography match within documented tolerance; byte-identical output is **not** claimed (OS text rasterization differs).

### 6.1 Baselines

- Source: `design/figma/*.svg` → PNG at defined scales (1×, 1.5×, 2×) via a pure-Rust rasterizer (e.g. `resvg`) in `cargo xtask design-baseline` `[PROPOSAL]`. **Annotation chrome** (§1.1 purple dashed frames and their 20px insets) is cropped/masked; embedded raster placeholders (video/album art) are replaced with flat-token-colored fills so diffs don't compare media content `[PROPOSAL]`.
- Screens under test render the **same stubbed state**: engine snapshot fixtures (§6.4) chosen to match each export (e.g. `Player.svg` = playing @ 00:02:10/00:25:40, 2-item playlist "Cool Playlist").
- Baseline set: every screen in §1.6 (~40 unique compositions after variant dedup) × {dark, light} × {default accent} at 1×; accents beyond default sampled (onboarding + settings global ×9 accents, matching exports Onboarding-3..10).

### 6.2 Diff engine and tolerances

- Per-pixel diff (pixelmatch-class, antialiasing-aware) producing a count of changed pixels **plus** structural checks: DOM-measured bounding boxes of annotated regions vs SVG geometry (±1px layout tolerance), radii/color token sampling at probe points (ΔE ≤ 2 for colors, exact for radii), font metrics (cap-height of rendered text vs §1.3 table, ±0.5px) `[PROPOSAL]`.
- Text blocks: compared via geometry + token color, not exact raster, acknowledging platform font rendering; this is why bundled fonts (§6.4) matter — they make even text diffs stable *within* a CI OS.
- Overall pass thresholds: 0 differing pixels outside tolerance zones; ≤0.1% differing pixels inside AA/text zones `[PROPOSAL]`, tuned after the first calibration run and then frozen in the repo with the tolerance file.

### 6.3 Harness, CI, xtask

- `cargo xtask ui-shots` (local): starts the UI in headless harness mode (Vite dev server + Playwright Chromium), iterates the fixture matrix, writes `tests/visual/{actual,diff}`, exits non-zero on threshold breach; `--update` re-pins baselines (see policy).
- CI (MP §13 "Figma screenshot diffs" gate): one dedicated job per OS (windows/linux/macos) since font rasterization differs; **per-OS baselines** are accepted reality `[PROPOSAL]`, with Linux as the canonical one. Nightly job additionally runs the accent ×9 and DPI matrix to keep the fast lane green-quick.
- Tauri shell shots (real webview) are a *separate, smaller* suite — chrome-level only (window radius, topbar) — because webview rendering variance otherwise dominates; the component/screen matrix runs in Chromium for determinism. `[PROPOSAL]`

### 6.4 Flakiness controls

Deterministic fonts (bundle the real typeface in the harness once identified, §2; interim: one pinned open font) · animations disabled (`prefers-reduced-motion` forced) · engine fully stubbed via fixture snapshots replayed through the command-layer testkit (doc 05 §7) — **no real media in visual tests** · fixed clock (no wall time in time displays) · locale pinned to pseudo/en-US baseline · anti-aliasing variance handled by tolerance zones · webview GPU disabled where it introduces variance; CI runners pinned to stable images.

### 6.5 Baseline update policy

Baselines change only via `xtask ui-shots --update` in a commit that also references either (a) a token change (tokens.json diff), (b) a layout-config change, or (c) a design-package re-export (design/figma/ change). Reviewer checklist: diff images must be attached to the PR. Deleting or weakening a threshold requires Lead sign-off (AGENTS §3.5 analog — never weaken a test to pass it).

---

## 7. Video-in-webview prototype plan (Phase 2, MP §7 bullet 6)

**Problem:** the desktop UI is a webview (Tauri); decoded video frames come from the Rust engine's wgpu renderer. Two integration candidates; prototype both behind the command layer so the UI code is identical either way.

### 7.1 Candidates

- **Candidate A — native wgpu surface behind a transparent webview.** The Tauri window is made transparent (and layered); a native child surface (wgpu + platform windowing) renders video *under* the webview; the webview's player region is a transparent hole; UI chrome composites on top. Pros: zero copies into the webview, full GPU path, engine-owned presentation (MP §3.4 isolation intact). Cons: per-webview transparency/compositing quirks; z-order and input routing complexity; fullscreen and multi-monitor edge cases; macOS WKWebView layering is finicky; **WebKitGTK is the known pain point** (transparent windows historically unreliable, compositor-dependent).
- **Candidate B — frame streaming to a canvas.** Engine renders (or blits) frames into shared memory / texture; web side draws to `<canvas>` via WebGPU/WebGL (or `ImageData` fallback). Pros: identical code path as the **web build** (MP §10 — WebGPU/WebGL there anyway), works in every webview (no window-level transparency), simpler fullscreen (canvas is just DOM), miniplayer/PiP reuse. Cons: per-frame copy cost (mitigable with WebGPU external textures / `SharedArrayBuffer` on desktop where COOP/COEP are controllable), CPU/GPU overhead to measure, color-space care, potential compositing artifacts (tearing) if not vsync-locked.

### 7.2 Evaluation matrix (measure, don't assume — MP §11)

| Criterion | A (native surface) | B (canvas streaming) |
|---|---|---|
| Time-to-first-frame latency | expected low; measure | copy + upload overhead; measure |
| Steady-state CPU / GPU | expected lowest | measure at 1080p/4K, 24/60 fps |
| Compositing artifacts (tearing, flicker, z-order) | risk concentrated in webview transparency | risk concentrated in canvas present mode |
| Window resize / DPI changes | surface resize path | canvas buffer resize path |
| Fullscreen & multi-monitor | OS-level, per-platform work | DOM-level, uniform |
| Web-build reuse (MP §10) | none | direct |
| Miniplayer / PiP / overlay UI over video | transparency-dependent | trivial (video is a DOM element) |
| Webview support surface | 3 platforms × transparency bugs | 3 platforms × standard canvas |
| Implementation risk | high on Linux | medium, known patterns |

### 7.3 Test protocol (per webview: WebView2/Windows, WKWebView/macOS, WebKitGTK/Linux)

For each candidate × webview: (1) transparent-window smoke (A only: blank window, alpha gradient, video-visible test); (2) 60 s playback of generated test pattern (doc 03 fixtures) — record dropped frames, present latency (engine timestamp → compositor), CPU/GPU via OS profilers; (3) resize storm; (4) fullscreen enter/exit ×20; (5) dual-monitor drag; (6) 1×/2× DPI; (7) overlay UI over video (settings panel translucency + backdrop-filter, §3.3) checking artifacts; (8) long-run (1 h) leak watch. Results recorded per cell in a decision matrix checked into `docs/` with raw numbers `[VERIFY]` — no cell assumed.

### 7.4 Recommendation `[PROPOSAL]`

Prototype **B first** (canvas via WebGPU, WebGL fallback): it unblocks Phase 2's first vertical, shares the web-build path (MP §10), and carries the least platform risk; keep the engine's presentation behind a `VideoSurface` interface so **A** can slot in as the desktop fast path where its measurements win (expected: Windows WebView2, possibly macOS). **Fallback chain: A(per-OS, if measured better) → B(WebGPU) → B(WebGL) → B(readback + 2D canvas, diagnostic-quality only, honestly labeled).** WebKitGTK gets explicit early spikes on both candidates since it is the highest-risk webview (MP §7). Decision by ADR in Phase 2 with the measured matrix attached.

---

## 8. UI workspace layout

```
ui/
  package.json  tsconfig.json  vite.config.ts  (toolchain [PROPOSAL]: Vite + vitest +
  playwright)   eslint/stylelint config (incl. token lint §3.4)
  src/
    tokens/       generated.css, token types (§3)
    components/   primitives/ composites/ overlays/  (§4.1)
    screens/      route-level compositions (§4.1)
    state/        snapshot store, selectors, UI-only stores (§4.2)
    bindings/     GENERATED command-layer client (doc 05 §5.1) — do not hand-edit
    i18n/         generated locale packs (doc 07)
    app/          root providers, theme/accent applier, router
  test/
    unit/         vitest + @testing-library/react (jsdom)
    a11y/         vitest-axe + Playwright axe scans
    i18n/         pseudo-locale completeness/overflow tests
    visual/       Playwright screenshot specs + fixtures matrix
    fixtures/     engine snapshot fixtures (shared format with testkit, doc 05 §7)
  index.html      shell entry (desktop Tauri + web shared)
```

- **Shared with `apps/web`** (MP §2: "UI code is shared with desktop"): `apps/web` is a thin host that mounts the same `ui/` bundle and provides the wasm command binding; no screen/component may import Tauri APIs directly — only `bindings/` (which the hosts configure). `[PROPOSAL]` dependency direction enforced by lint (`no-restricted-imports`).
- Build outputs: `ui/dist` consumed by `apps/desktop` (Tauri) and `apps/web`; CI runs typecheck, lint, unit, a11y, visual (§9.5) on every PR; wasm/host integration builds belong to apps' pipelines.
- Tooling choices `[PROPOSAL]` (Lead approval per MP §9-adjacent JS-tooling policy): Vite (dev speed, shared config), vitest + Testing Library (unit/interaction), Playwright (visual + keyboard-nav, headless cross-OS), Stylelint + ESLint (token + import-boundary rules). All JS deps recorded with license + supply-chain notes for doc 11's audit.

---

## 9. UI test plan (MP §13 mapping)

| Layer | Tool | Scope | CI gate (MP §13) |
|---|---|---|---|
| 9.1 Unit (components) | vitest + RTL | every primitive: renders from tokens, state matrix §4.4 rows, a11y roles/labels present | every PR, merge-blocking |
| 9.2 Interaction | vitest (user-event) | §5 tables: pending-seek, play/pause pending, error surfaces from typed errors, dialog flows, settings edit/save, drag-and-drop reorder, focus order traversal, roving tabindex | every PR |
| 9.3 Screenshot diffs | Playwright + xtask (§6) | §1.6 screens × themes × DPI vs pinned baselines | every PR (1× dark/light), nightly (full matrix) |
| 9.4 A11y audits | vitest-axe + Playwright axe | zero critical violations; contrast checks across 9 accents × 2 themes (token math, §3.4); focus-visible presence on every interactive; reduced-motion variants | every PR |
| 9.5 i18n | vitest | pseudo-locale: no missing keys, no hardcoded strings in TSX (lint), placeholder args validated, truncation/overflow on longest-locale render of key screens | every PR |
| 9.6 Keyboard nav | Playwright | full player operation without pointer using the default keymap (doc 05 §8.2): open→play→seek→volume→tracks→fullscreen→menus | every PR |
| 9.7 Command-layer conformance | doc 05 §7 testkit (UI profile) | UI never bypasses the command layer (import lint + runtime fixture asserts); stale-generation scenarios replayed as UI snapshots | every PR |
| 9.8 Shell integration | Tauri + Playwright | window chrome shots, drag-drop from OS, media-key handling, title template from product.toml (rebrand propagation test, MP §5) | per-OS release lane |

Fixtures are engine *snapshot* files (doc 05 §7 format) — the same fixtures drive visual, interaction and conformance tests, so a UI state is defined exactly once `[PROPOSAL]`.

---

## 10. Open questions / decisions requested

1. **Figma file + variables**: obtain link/variables export and license clearance (R-07) — blocks token re-derivation and true font identity `[VERIFY]`.
2. **Font**: interim bundled typeface choice (and final, once known) — affects every screenshot baseline; decide before Phase 2 pin.
3. **W3C design-tokens JSON** as the tokens format (§3.1) — interface-level, needs Lead sign-off.
4. **Keymap reconciliation**: design accelerators (Ctrl+O/F/N/D/C/R, Shift+F1) vs doc 05 §8.2 seed — one data file must win (§1.5 finding).
5. **Settings areas count**: 13 areas found vs README's 14 (§1.6) — confirm against the Figma page list `[VERIFY-DESIGN]`.
6. **Missing-state designs** (focus/disabled/loading/error/empty/fullscreen chrome): approve the §2/§4.4/§5 interim language or commission state frames.
7. **Video-in-webview**: approve B-first prototype order + ADR template (§7.4).
8. **Per-OS baselines** acceptance (§6.3) and the exact diff thresholds after calibration.
9. **JS toolchain + deps** (Vite/vitest/RTL/Playwright) approval for the dependency audit (§8).

## Traceability summary

MP §2 (React+TS owns presentation only) → §4.2 · MP §3.5 (command layer only) → §4.2, §9.7 · MP §5 (config-driven, tokens, i18n) → §3, §4.1, §8, §9.5 · MP §7 (all six bullets) → §1–§7 respectively · MP §10 (web) → §7, §8 · MP §13 (CI gates) → §6.3, §9 · MP §16 (Phase 2 token proof + video prototype; Phase 4 Figma-exact UI) → §3, §6, §7 · doc 01 (theme-tokens crate) → §3.1 · doc 05 (C1–C8, catalogs, keymaps/menus-as-data) → §1.5, §4, §5 · doc 07 (settings schema, config) → §1.6, §4.1 (reconcile when published).

*All §1 values are export-derived (`[VERIFY-DESIGN]`); all invented mechanisms are `[PROPOSAL]`; unconfirmed external facts are `[VERIFY]`. No VLC source, assets or branding were consulted — "VLC Media Player" strings above are the community redesign's own placeholder copy and become product.toml-localized strings, not shipped trademarks (clean-room, AGENTS §3.1).*


