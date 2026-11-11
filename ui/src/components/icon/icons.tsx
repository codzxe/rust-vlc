import type { ReactElement } from "react";

/**
 * Card 4.B4 — icon system (issue #35: real glyph paths mined from the pinned
 * Figma exports; replaces the provisional hand-drawn geometry).
 *
 * Design provenance: docs/08-ui-figma-fidelity.md §1.4 "Icon system: line
 * icons, stroke #8F8F8F, stroke-width 1.5 (3464 uses; 2 and 3 for emphasis);
 * hit wrapper ~48 (Icon Wrapper.svg)" and §1.5/§1.6 component + customize
 * catalogs, which are the ONLY sources for icon names below (never invent).
 *
 * MINING METHOD (all glyph paths below are verbatim design geometry):
 * design/figma/Icon.svg is the pinned icon sheet; every glyph group id carries
 * a property string "name=…, direction=…, state=…, value=…, EQ=…, two-tone=…".
 * Each registry entry takes its default-state group (state=default,
 * two-tone=no) — the per-glyph comment above each entry records the source
 * group and its layer ids. Where the sheet name differs from the registry name
 * the comment documents the cross-reference evidence:
 *   - menu accelerators (open-file … about) are cross-checked against the
 *     actual glyphs in design/figma/menu.svg (settings = Vector 75 + Ellipse 10
 *     in both files; capture-device/convert/customize-player/about match by
 *     identical vector ids and geometry);
 *   - play/pause are cross-checked against play.svg (type=play/type=pause) and
 *     the Controls.svg play button;
 *   - the volume family lives in the sheet under a misnamed "close" component
 *     whose value= axis (none/low/medium) plus speaker geometry identifies the
 *     volume levels; name=volume value=high completes it.
 *
 * W26.a07 EXTENSION (frame-mined, same method): the redesign's control bar
 * (Controls.svg/Player.svg), menus (menu.svg, *context menu.svg, Profiles.svg),
 * miniplayer and View Options frames were mined for the remaining redesign
 * glyphs. Cross-references recorded per entry below, e.g.:
 *   - next/previous = sheet name=next/name=previous (state=default), identical
 *     to the Controls.svg Frame 8/Frame 10 transport buttons and Player.svg's
 *     bottom bar (filled triangle + bar);
 *   - menu (hamburger) = the Controls.svg "Icon Wrapper_4"/Player.svg cluster
 *     (3× short nub + 12u line at 5u pitch);
 *   - profile = sheet name=profile, identical to menu.svg "Profile (Movies)"
 *     (Ellipse 7 head + Vector 72 shoulders) and the Miniplayer meta row;
 *   - subtitles = sheet name=subtitles, identical to the Controls.svg cc button
 *     (Rectangle 1 + Vector 8/9 dashed rows);
 *   - audio = sheet name=audio (5 level bars), the Controls.svg level-pill icon
 *     and the Settings audio sidebar glyph;
 *   - equalizer = sheet name=preferences (3 sliders + offset nubs);
 *   - thumbnail = View Options.svg Rectangle 22-25 (2×2 grid of 7.25×6.5
 *     r1.25 tiles);
 *   - hide-ui = right-click menu.svg "Hide UI" row (Rectangle 2 + Vector 115);
 *   - vlc = sheet name=vlc (traffic cone), the title-bar logo simplified.
 *
 * GEOMETRY CONVENTIONS:
 *   - 24×24 viewBox, no rescaling: paths are translated from sheet coordinates
 *     so each glyph's bounding-box centre lands on (12, 12) — the same
 *     centring the sheet itself uses on its 44px-pitch cells (verified: settings
 *     centred on (32,32), volume-high on (297,164)≈cell (296,164)).
 *   - stroke #8F8F8F / stroke-width 1.5 are supplied by the <Icon> shell
 *     (currentColor); elements the design FILLS carry fill="currentColor"
 *     (transport glyphs, record dot, digits…), fill-only shapes also pin
 *     stroke="none" so the shell stroke cannot outline them.
 *   - per-element stroke-width overrides from the sheet are kept verbatim
 *     (e.g. info stem/dot at 2 = --stroke-icon-strong; speed needle 1.28571).
 *   - KNOWN DEVIATION: the sheet exports file/home/sidebar bodies as
 *     Figma "inside" strokes (mask + doubled stroke-width). The same path data
 *     is used here with the shell's centred 1.5 stroke, so those outlines
 *     extend ~0.75u outward instead of inward; shape geometry is identical.
 *     The doubled sw=3 mask twin is dropped (w26.a07 applies the same rule to
 *     the new left-sidebar/collapse-sidebar/expand-sidebar/tv entries).
 */

export type IconName =
  // play.svg + Controls.svg (transport)
  | "play"
  | "pause"
  | "stop"
  // Icon.svg name=next / name=previous — Controls.svg Frame 8/Frame 10 + Player.svg bottom bar
  | "next"
  | "previous"
  // volume family: Icon.svg "close, value=none|low|medium" + "volume, value=high"
  | "volume-mute"
  | "volume-low"
  | "volume-medium"
  | "volume-high"
  // window/title-bar controls (Player.svg Vector 22_2/22_3, sheet name=minimize/maximize/fullscreen)
  | "fullscreen"
  | "minimize"
  | "maximize"
  // chrome: hamburger (Controls.svg Icon Wrapper_4), ✕ (sheet name=close), ⋯ (sheet name=more)
  | "menu"
  | "close"
  | "more"
  // sidebar toggles (sheet name=sidebar direction=left|right, collapse-sidebar, expand-sidebar)
  | "left-sidebar"
  | "collapse-sidebar"
  | "expand-sidebar"
  // navigation & data utilities (sheet name=search/sort/list/lists/grid/add/check/chevron)
  | "search"
  | "sort"
  | "list"
  | "lists"
  | "grid"
  | "thumbnail" // View Options.svg Rectangle 22-25 (thumbnail view)
  | "add"
  | "check"
  | "chevron-up"
  | "chevron-down"
  | "chevron-left"
  | "chevron-right"
  // content models (sheet name=folder/playlist/queue/person/profile/artist/favorite)
  | "folder"
  | "playlist"
  | "queue"
  | "person"
  | "profile"
  | "artist"
  | "favorite"
  // media tools (sheet name=preferences→equalizer, audio, subtitles)
  | "equalizer"
  | "audio"
  | "subtitles"
  // aliases for the redesign vocabulary mapped onto identical sheet geometry
  | "info" // = sheet name=info, same glyph as "about"
  | "wand" // = sheet name=paint, same glyph as "customize-player"
  // menu/context-menu utilities (sheet name=recent/refresh/save/edit/remove/music/chapters/bookmark)
  | "recent"
  | "refresh"
  | "save"
  | "edit"
  | "remove"
  | "music"
  | "chapters"
  | "bookmark"
  // settings areas (sheet name=interface/privacy/playback/plugins/tv/wrench/accessibility)
  | "interface"
  | "privacy"
  | "playback"
  | "plugins"
  | "tv"
  | "wrench"
  | "accessibility"
  // misc (sheet name=divider/vlc; right-click menu.svg "Hide UI")
  | "divider"
  | "vlc"
  | "hide-ui"
  // customize-panel button catalog (doc 08 §1.6 Player - customize)
  | "skip-back-10"
  | "skip-forward-10"
  | "skip-back-15"
  | "skip-forward-15"
  | "playback-speed"
  | "repeat"
  | "shuffle"
  | "record"
  | "snapshot"
  | "pip"
  | "hdr"
  | "zoom"
  | "right-sidebar"
  | "quit"
  | "settings"
  | "go-home"
  // menu.svg accelerators (doc 08 §1.5)
  | "open-file"
  | "open-folder"
  | "network-stream"
  | "open-disc"
  | "capture-device"
  | "convert"
  | "keyboard-shortcuts"
  | "customize-player"
  | "about";

/** One glyph = children rendered inside the stroked `<svg>` shell.
 * Paths mined from design/figma/Icon.svg (see header for attribution).
 * Shared consts serve registry aliases (same design glyph, two vocabulary
 * names) without duplicating path data. */

// sheet name=folder (Vector 23_2) — menu.svg "Open Folder", Player.svg rail/breadcrumb
const folderGlyph = (
  <path d="M21 9.541V17C21 18.657 19.657 20 18 20H6C4.343 20 3 18.657 3 17V6.598C3 5.163 4.163 4 5.598 4H8.197C9.323 4 10.375 4.563 11 5.5C11.625 6.437 12.677 7 13.803 7H18.459C19.862 7 21 8.138 21 9.541Z" />
);

// sheet name=info (Ellipse 9_2 + Vector 98/99) — menu.svg "About VLC"
const infoGlyph = (
  <>
    <circle cx="12" cy="12" r="10" />
    <path d="M12 11V17" strokeWidth={2} />
    <path d="M12 7V7" strokeWidth={2} />
  </>
);

// sheet name=paint (Rectangle 256 + Vector 100) — menu.svg "Customize Player" (the "wand")
const wandGlyph = (
  <>
    <path d="M11.989 9.318L19.07 2.237C19.751 1.556 20.854 1.556 21.535 2.237C22.216 2.918 22.216 4.022 21.535 4.702L14.454 11.783C13.774 12.464 12.67 12.464 11.989 11.783C11.309 11.102 11.309 9.999 11.989 9.318Z" />
    <path d="M10.641 14.547C11.914 16.244 11.841 19.547 8.641 19.547C5.441 19.547 4.696 21.444 4.696 22.444C3.863 21.611 1.784 17.856 4.696 14.944C6.196 13.444 9.141 12.547 10.641 14.547Z" />
  </>
);

// sheet name=profile (head + shoulders) — menu.svg "Profile (Movies)", Miniplayer meta row
const personGlyph = (
  <>
    <path d="M15.25 7.125C15.25 8.92 13.795 10.375 12 10.375C10.205 10.375 8.75 8.92 8.75 7.125C8.75 5.33 10.205 3.875 12 3.875C13.795 3.875 15.25 5.33 15.25 7.125Z" />
    <path d="M5 20.125C5.311 18.125 7.147 14.125 12 14.125C16.853 14.125 18.844 18.125 19 20.125" />
  </>
);

// sheet name=preferences sliders — the equalizer glyph (3 lines + offset nubs)
const equalizerGlyph = (
  <>
    <path d="M5 4V20" />
    <path d="M7 8.0098L3 7.9997" />
    <path d="M12 4V20" />
    <path d="M14 16.0098L10 15.9997" />
    <path d="M19 4V20" />
    <path d="M21 12.0098L17 11.9997" />
  </>
);

const glyphs: Record<IconName, ReactElement> = {
  // Icon.svg "name=play" (Vector 16_4) — identical geometry to play.svg type=play and the Controls.svg play button
  "play": <path d="M7.75 16.955V7.045C7.75 6.213 8.707 5.744 9.364 6.255L15.735 11.211C16.25 11.611 16.25 12.389 15.735 12.789L9.364 17.745C8.707 18.256 7.75 17.787 7.75 16.955Z" fill="currentColor" />,

  // Icon.svg "name=pause" (Vector 17_4/16_9) — matches play.svg type=pause
  "pause": (
    <>
      <path d="M8 17V7H10V17H8Z" fill="currentColor" />
      <path d="M14 17V7H16V17H14Z" fill="currentColor" />
    </>
  ),

  // Icon.svg "name=stop" (Vector 16_12)
  "stop": <path d="M8 17V7H16V17H8Z" fill="currentColor" />,

  // Icon.svg "name=next" (Frame 17, state=default) — filled triangle + bar; identical to
  // Controls.svg Frame 10 (Vector 18_2/Vector 16_3) and the Player.svg bottom bar transport
  "next": (
    <>
      <path d="M7.5 17V7L13.5 12L7.5 17Z" fill="currentColor" />
      <path d="M14.5 17V7H16.5V17H14.5Z" fill="currentColor" />
    </>
  ),

  // Icon.svg "name=previous" (Frame 18, state=default) — bar + filled triangle; identical to
  // Controls.svg Frame 8 (Vector 16/Vector 18) and the Player.svg bottom bar transport
  "previous": (
    <>
      <path d="M7.5 17V7H9.5V17H7.5Z" fill="currentColor" />
      <path d="M16.5 17V7L10.5 12L16.5 17Z" fill="currentColor" />
    </>
  ),

  // Icon.svg "name=close, value=none" — sheet group is misnamed "close"; the value=none axis + speaker geometry identify the muted volume state (the Controls.svg volume buttons carry the same speaker family)
  "volume-mute": (
    <>
      <path d="M4.547 8.294L11.547 4.738C12.212 4.4 13 4.883 13 5.63V12V18.37C13 19.117 12.212 19.6 11.547 19.262L4.547 15.706C4.211 15.536 4 15.191 4 14.815V12V9.185C4 8.809 4.211 8.464 4.547 8.294Z" />
      <path d="M16 10L18 12M18 12L20 14M18 12L20 10M18 12L16 14" />
    </>
  ),

  // Icon.svg "name=close, value=low" — see volume-mute note; one wave
  "volume-low": (
    <>
      <path d="M6.547 8.294L13.547 4.738C14.212 4.4 15 4.883 15 5.63V12V18.37C15 19.117 14.212 19.6 13.547 19.262L6.547 15.706C6.211 15.536 6 15.191 6 14.815V12V9.185C6 8.809 6.211 8.464 6.547 8.294Z" />
      <path d="M18 11L18 13" />
    </>
  ),

  // Icon.svg "name=close, value=medium" — see volume-mute note; two waves
  "volume-medium": (
    <>
      <path d="M5.047 8.294L12.047 4.738C12.712 4.4 13.5 4.883 13.5 5.63V12V18.37C13.5 19.117 12.712 19.6 12.047 19.262L5.047 15.706C4.711 15.536 4.5 15.191 4.5 14.815V12V9.185C4.5 8.809 4.711 8.464 5.047 8.294Z" />
      <path d="M16.5 11L16.5 13" />
      <path d="M19.5 10L19.5 14" />
    </>
  ),

  // Icon.svg "name=volume, value=high" (Vector 31-34) — same three-bar wave glyph as the Controls.svg volume buttons
  "volume-high": (
    <>
      <path d="M3.547 8.294L10.547 4.738C11.212 4.4 12 4.883 12 5.63V12V18.37C12 19.117 11.212 19.6 10.547 19.262L3.547 15.706C3.211 15.536 3 15.191 3 14.815V12V9.185C3 8.809 3.211 8.464 3.547 8.294Z" />
      <path d="M15 11L15 13" />
      <path d="M18 10L18 14" />
      <path d="M21 8L21 16" />
    </>
  ),

  // Icon.svg "name=fullscreen" (Frame_37, Vector 22_4) — the four corner marks; identical
  // to Player.svg title bar Vector 22_3
  "fullscreen": <path d="M9.6 6H6V9.6M14.4 6H18V9.6M18 14.4V18H14.4M9.6 18H6V14.4" />,

  // Icon.svg "name=minimize" (Frame_86) — 12u baseline
  "minimize": <path d="M6 12H18" />,

  // Icon.svg "name=maximize" (Frame_87) — 12×12 square; identical to Player.svg title-bar Vector 22_2
  "maximize": <path d="M6 6H18V18H6V6Z" />,

  // Hamburger — Controls.svg "Icon Wrapper_4"/Icon_8 (Vector/Vector_2…Vector_6) and
  // Player.svg bottom bar (Vector_4-6 + Vector_6/8): three rows of 1u nub + 12u line at 5u pitch
  "menu": (
    <>
      <path d="M4 7H5" />
      <path d="M8 7H20" />
      <path d="M4 12H5" />
      <path d="M8 12H20" />
      <path d="M4 17H5" />
      <path d="M8 17H20" />
    </>
  ),

  // Icon.svg "name=close, value=default" (Frame 34) — the ✕; identical to the
  // Player.svg title-bar close (Vector 20/21)
  "close": (
    <>
      <path d="M7 7L17 17" />
      <path d="M17 7L7 17" />
    </>
  ),

  // Icon.svg "name=more" (Frame 49, Vector 36/37/38) — three 3px round-cap dots;
  // identical to the Player.svg title-bar ⋯ (Vector 36/37/38) and media.svg row menus
  "more": (
    <>
      <path d="M6 12L6 12" strokeWidth={3} />
      <path d="M12 12L12 12" strokeWidth={3} />
      <path d="M18 12L18 12" strokeWidth={3} />
    </>
  ),

  // Icon.svg "name=sidebar, direction=left" (Frame_46: mask shape + Rectangle 4_2 bar)
  // — the Player.svg title-bar sidebar toggle (Rectangle 2 + Rectangle 4)
  "left-sidebar": (
    <>
      <path d="M3 6C3 4.343 4.343 3 6 3H18C19.657 3 21 4.343 21 6V18C21 19.657 19.657 21 18 21H6C4.343 21 3 19.657 3 18V6Z" />
      <rect x="6" y="6" width="2" height="12" rx="0.5" fill="currentColor" stroke="none" />
    </>
  ),

  // Icon.svg "name=collapse-sidebar" (Frame_51) — panel + left bar + arrow pointing into the rail
  "collapse-sidebar": (
    <>
      <path d="M3 6C3 4.343 4.343 3 6 3H18C19.657 3 21 4.343 21 6V18C21 19.657 19.657 21 18 21H6C4.343 21 3 19.657 3 18V6Z" />
      <rect x="6" y="6" width="2" height="12" rx="0.5" fill="currentColor" stroke="none" />
      <path d="M13.143 14L11 12L13.143 10M11 12H17" />
    </>
  ),

  // Icon.svg "name=expand-sidebar" (Frame_50) — panel + left bar + arrow pointing away
  "expand-sidebar": (
    <>
      <path d="M3 6C3 4.343 4.343 3 6 3H18C19.657 3 21 4.343 21 6V18C21 19.657 19.657 21 18 21H6C4.343 21 3 19.657 3 18V6Z" />
      <rect x="6" y="6" width="2" height="12" rx="0.5" fill="currentColor" stroke="none" />
      <path d="M14.857 14L17 12L14.857 10M17 12H11" />
    </>
  ),

  // Icon.svg "name=search" (Frame 1_19, Ellipse 5 + Vector 39/40) — magnifier;
  // identical to the Settings top-bar search (Ellipse 5 in Settings - global.svg)
  "search": <path d="M15.345 15.345C16.495 14.195 17.206 12.607 17.206 10.853C17.206 7.344 14.362 4.5 10.853 4.5C7.344 4.5 4.5 7.344 4.5 10.853C4.5 14.362 7.344 17.206 10.853 17.206C12.607 17.206 14.195 16.495 15.345 15.345ZM15.345 15.345L19.5 19.5" />,

  // Icon.svg "name=sort" (Frame 1_17) — up/down arrows; identical to the subtitle
  // dialog sort arrows (Vector 42/43 in subtitle multiple.svg)
  "sort": (
    <>
      <path d="M6.5 6V19M10 15.5L6.5 19L3 15.5" />
      <path d="M17.5 18V5M14 8.5L17.5 5L21 8.5" />
    </>
  ),

  // Icon.svg "name=list" (Frame 1_33) — two chips + two lines (view: list)
  "list": (
    <>
      <path d="M4.125 4.75H6.125C6.815 4.75 7.375 5.31 7.375 6V8C7.375 8.69 6.815 9.25 6.125 9.25H4.125C3.435 9.25 2.875 8.69 2.875 8V6C2.875 5.31 3.435 4.75 4.125 4.75Z" />
      <path d="M4.125 14.75H6.125C6.815 14.75 7.375 15.31 7.375 16V18C7.375 18.69 6.815 19.25 6.125 19.25H4.125C3.435 19.25 2.875 18.69 2.875 18V16C2.875 15.31 3.435 14.75 4.125 14.75Z" />
      <path d="M10.125 7H21.125" strokeWidth={1.28571} />
      <path d="M10.125 17H21.125" strokeWidth={1.28571} />
    </>
  ),

  // Icon.svg "name=lists" (Frame 1_34) — three compact nub+line rows (sidebar meta)
  "lists": (
    <>
      <path d="M4 7H5" />
      <path d="M8 7H20" />
      <path d="M4 12H5" />
      <path d="M8 12H20" />
      <path d="M4 17H5" />
      <path d="M8 17H20" />
    </>
  ),

  // Icon.svg "name=grid" (Frame 1_32) — 2×2 rounded squares (view: grid)
  "grid": (
    <>
      <path d="M3 3.75H9C9.69 3.75 10.25 4.31 10.25 5V9C10.25 9.69 9.69 10.25 9 10.25H3C2.31 10.25 1.75 9.69 1.75 9V5C1.75 4.31 2.31 3.75 3 3.75Z" />
      <path d="M15 3.75H21C21.69 3.75 22.25 4.31 22.25 5V9C22.25 9.69 21.69 10.25 21 10.25H15C14.31 10.25 13.75 9.69 13.75 9V5C13.75 4.31 14.31 3.75 15 3.75Z" />
      <path d="M3 13.75H9C9.69 13.75 10.25 14.31 10.25 15V19C10.25 19.69 9.69 20.25 9 20.25H3C2.31 20.25 1.75 19.69 1.75 19V15C1.75 14.31 2.31 13.75 3 13.75Z" />
      <path d="M15 13.75H21C21.69 13.75 22.25 14.31 22.25 15V19C22.25 19.69 21.69 20.25 21 20.25H15C14.31 20.25 13.75 19.69 13.75 19V15C13.75 14.31 14.31 13.75 15 13.75Z" />
    </>
  ),

  // View Options.svg Rectangle 22-25 — 2×2 grid of 7.25×6.5 r1.25 tiles (view: thumbnails)
  "thumbnail": (
    <>
      <rect x="2.375" y="3.75" width="7.25" height="6.5" rx="1.25" />
      <rect x="14.375" y="3.75" width="7.25" height="6.5" rx="1.25" />
      <rect x="2.375" y="13.75" width="7.25" height="6.5" rx="1.25" />
      <rect x="14.375" y="13.75" width="7.25" height="6.5" rx="1.25" />
    </>
  ),

  // Icon.svg "name=add" (Frame 1_7) — plus; identical to folder context menu Vector 39/40
  // ("New Folder") and new playlist.svg Vector 39/40
  "add": (
    <>
      <path d="M12 5V19" />
      <path d="M19 12H5" />
    </>
  ),

  // Icon.svg "name=check, state=selected" (Frame 1_15) — identical to the option.svg
  // checkmark (Vector 44) used in checked menu rows
  "check": <path d="M5 12L9.667 17.5L19 6.5" />,

  // Icon.svg "name=chevron, direction=up|down|left|right" (Frame_88/73/90/89) —
  // identical to the 5×10 menu submark chevrons (Vector 27 family)
  "chevron-up": <path d="M7 14.5L12 9.5L17 14.5" />,
  "chevron-down": <path d="M7 9.5L12 14.5L17 9.5" />,
  "chevron-left": <path d="M14.5 7L9.5 12L14.5 17" />,
  "chevron-right": <path d="M9.5 7L14.5 12L9.5 17" />,

  // Icon.svg "name=folder" (Vector 23_2) — alias of "open-folder" (same glyph)
  "folder": folderGlyph,

  // Icon.svg "name=playlist" (Frame 1_38) — list + play triangle; identical to the
  // sidebar playlist rows (Vector/Vector_2/Vector_3 + Vector 1) and right-click "Playlist"
  "playlist": (
    <>
      <path d="M2.9095 6H20.9095" />
      <path d="M2.9095 11H11.5575" />
      <path d="M2.9095 16H11.5575" />
      <path d="M17.0905 18V10L21.0905 14L17.0905 18Z" fill="currentColor" />
    </>
  ),

  // Icon.svg "name=queue" (Frame 1_40) — pill + two lines (the queue toggle)
  "queue": (
    <>
      <path d="M6 4.875H18C19.243 4.875 20.25 5.882 20.25 7.125C20.25 8.368 19.243 9.375 18 9.375H6C4.757 9.375 3.75 8.368 3.75 7.125C3.75 5.882 4.757 4.875 6 4.875Z" />
      <path d="M5 14.125H19" />
      <path d="M5 19.125H19" />
    </>
  ),

  // Icon.svg "name=profile" (Frame 1_22) — alias "person": head + shoulders
  "person": personGlyph,
  "profile": personGlyph,

  // Icon.svg "name=artist" (Frame 1_21) — person + eighth note (attribution)
  "artist": (
    <>
      <path d="M11.75 7.125C11.75 8.92 10.2949 10.375 8.5 10.375C6.7051 10.375 5.25 8.92 5.25 7.125C5.25 5.33 6.7051 3.875 8.5 3.875C10.2949 3.875 11.75 5.33 11.75 7.125Z" />
      <path d="M1.5 20.125C1.8111 18.125 3.6467 14.125 8.5 14.125C13.3533 14.125 15.3444 18.125 15.5 20.125" />
      <path d="M17.3478 13.125C19.973 13.125 19.9728 10.5 19.9728 10.5C19.9728 10.5 16.5992 8.935 15.6436 10.846C15.1603 11.813 15.9389 13.125 17.3478 13.125Z" fill="currentColor" stroke="none" />
      <path d="M22.5 6.125H19.9728V10.5M19.9728 10.5C19.9728 10.5 19.973 13.125 17.3478 13.125C15.9389 13.125 15.1603 11.813 15.6436 10.846C16.5992 8.935 19.9728 10.5 19.9728 10.5Z" />
    </>
  ),

  // Icon.svg "name=favorite" (Frame 1_73) — heart; identical to the file/playlist
  // context-menu "Union" favorite row
  "favorite": <path d="M12.0005 6.312L12.9435 5.369C14.7655 3.546 17.7205 3.546 19.5425 5.369C21.6255 7.452 21.6255 10.829 19.5425 12.911L12.0005 20.454L4.4575 12.911C2.3745 10.829 2.3745 7.452 4.4575 5.369C6.2805 3.546 9.2345 3.546 11.0575 5.369L12.0005 6.312Z" />,

  // Icon.svg "name=preferences" (Frame_96) — 3 sliders with offset nubs: THE equalizer glyph
  "equalizer": equalizerGlyph,

  // Icon.svg "name=audio" (Frame_12) — five level bars; the Controls.svg level pill
  // (Vector 10-14) and right-click "Audio" row use the same geometry
  "audio": (
    <>
      <path d="M4 11V13" />
      <path d="M8 8V16" />
      <path d="M12 3V21" />
      <path d="M16 6V18" />
      <path d="M20 9V15" />
    </>
  ),

  // Icon.svg "name=subtitles" (Frame_9) — cc card + two dashed rows; identical to the
  // Controls.svg cc button (Rectangle 1 + Vector 8/9) and right-click "Subtitle" row
  "subtitles": (
    <>
      <path d="M5 4.75H19C20.795 4.75 22.25 6.2051 22.25 8V16C22.25 17.7949 20.795 19.25 19 19.25H5C3.205 19.25 1.75 17.7949 1.75 16V8C1.75 6.2051 3.205 4.75 5 4.75Z" />
      <path d="M5 13H13M15.5 13H19" />
      <path d="M5 16H8M14 16H19M11.5 16H10.5" />
    </>
  ),

  // Icon.svg "name=info" (Frame 1_26) — alias of "about" (same glyph)
  "info": infoGlyph,

  // Icon.svg "name=backward-10-sec" (Vector 19_3 + outlined "10")
  "skip-back-10": (
    <>
      <path d="M21 10.44C20.571 8.44 18.429 4.44 13.286 4.44C8.143 4.44 6.429 8.44 5.571 10.44M10.714 8.94L5.571 10.44L3 6.94" />
      <path d="M11.459 12.167V19.44H10.358V13.268H10.315L8.575 14.404V13.353L10.39 12.167H11.459ZM15.958 19.561C15.397 19.558 14.918 19.41 14.52 19.117C14.122 18.823 13.818 18.396 13.607 17.835C13.397 17.274 13.291 16.598 13.291 15.807C13.291 15.019 13.397 14.345 13.607 13.787C13.82 13.228 14.126 12.802 14.523 12.508C14.924 12.215 15.402 12.068 15.958 12.068C16.514 12.068 16.991 12.216 17.389 12.512C17.787 12.805 18.091 13.231 18.302 13.79C18.515 14.346 18.621 15.019 18.621 15.807C18.621 16.6 18.516 17.277 18.305 17.838C18.095 18.397 17.79 18.824 17.393 19.12C16.995 19.414 16.517 19.561 15.958 19.561ZM15.958 18.613C16.451 18.613 16.835 18.372 17.112 17.892C17.392 17.411 17.531 16.716 17.531 15.807C17.531 15.203 17.467 14.693 17.339 14.277C17.214 13.858 17.033 13.54 16.796 13.325C16.562 13.107 16.282 12.998 15.958 12.998C15.468 12.998 15.083 13.24 14.804 13.723C14.525 14.206 14.384 14.9 14.381 15.807C14.381 16.413 14.444 16.926 14.57 17.345C14.697 17.761 14.879 18.078 15.113 18.293C15.347 18.506 15.629 18.613 15.958 18.613Z" fill="currentColor" stroke="none" />
    </>
  ),

  // Icon.svg "name=forward-10-sec" (Vector 19 + outlined "10")
  "skip-forward-10": (
    <>
      <path d="M3 10.44C3.429 8.44 5.571 4.44 10.714 4.44C15.857 4.44 17.571 8.44 18.429 10.44M13.286 8.94L18.429 10.44L21 6.94" />
      <path d="M8.459 12.167V19.44H7.358V13.268H7.315L5.575 14.404V13.353L7.39 12.167H8.459ZM12.958 19.561C12.397 19.558 11.918 19.41 11.52 19.117C11.122 18.823 10.818 18.396 10.607 17.835C10.397 17.274 10.291 16.598 10.291 15.807C10.291 15.019 10.397 14.345 10.607 13.787C10.82 13.228 11.126 12.802 11.523 12.508C11.924 12.215 12.402 12.068 12.958 12.068C13.514 12.068 13.991 12.216 14.389 12.512C14.787 12.805 15.091 13.231 15.302 13.79C15.515 14.346 15.621 15.019 15.621 15.807C15.621 16.6 15.516 17.277 15.305 17.838C15.095 18.397 14.79 18.824 14.393 19.12C13.995 19.414 13.517 19.561 12.958 19.561ZM12.958 18.613C13.451 18.613 13.835 18.372 14.112 17.892C14.392 17.411 14.531 16.716 14.531 15.807C14.531 15.203 14.467 14.693 14.339 14.277C14.214 13.858 14.033 13.54 13.796 13.325C13.562 13.107 13.282 12.998 12.958 12.998C12.468 12.998 12.083 13.24 11.804 13.723C11.525 14.206 11.384 14.9 11.381 15.807C11.381 16.413 11.444 16.926 11.57 17.345C11.697 17.761 11.879 18.078 12.113 18.293C12.347 18.506 12.629 18.613 12.958 18.613Z" fill="currentColor" stroke="none" />
    </>
  ),

  // Icon.svg "name=backward-15-sec" (Vector 19_4 + outlined "15")
  "skip-back-15": (
    <>
      <path d="M21 10.451C20.571 8.451 18.429 4.451 13.286 4.451C8.143 4.451 6.429 8.451 5.571 10.451M10.714 8.951L5.571 10.451L3 6.951" />
      <path d="M11.459 12.178V19.451H10.358V13.279H10.315L8.575 14.415V13.364L10.39 12.178H11.459ZM15.827 19.549C15.382 19.549 14.981 19.465 14.626 19.294C14.274 19.121 13.992 18.885 13.781 18.584C13.571 18.284 13.458 17.941 13.444 17.555H14.509C14.535 17.867 14.674 18.124 14.925 18.325C15.176 18.526 15.476 18.626 15.827 18.626C16.106 18.626 16.353 18.562 16.569 18.435C16.787 18.305 16.957 18.126 17.08 17.899C17.206 17.672 17.269 17.412 17.269 17.12C17.269 16.826 17.205 16.561 17.077 16.329C16.949 16.097 16.773 15.915 16.548 15.782C16.325 15.65 16.069 15.583 15.781 15.579C15.56 15.579 15.339 15.618 15.117 15.694C14.894 15.769 14.714 15.869 14.577 15.992L13.572 15.843L13.98 12.178H17.979V13.119H14.893L14.662 15.154H14.705C14.847 15.017 15.035 14.901 15.269 14.81C15.506 14.716 15.759 14.671 16.029 14.671C16.472 14.671 16.866 14.776 17.212 14.987C17.56 15.198 17.833 15.486 18.032 15.85C18.233 16.212 18.333 16.629 18.33 17.1C18.333 17.571 18.226 17.99 18.011 18.361C17.798 18.73 17.502 19.02 17.123 19.233C16.746 19.445 16.314 19.549 15.827 19.549Z" fill="currentColor" stroke="none" />
    </>
  ),

  // Icon.svg "name=forward-15-sec" (Vector 19_2 + outlined "15")
  "skip-forward-15": (
    <>
      <path d="M3 10.451C3.429 8.451 5.571 4.451 10.714 4.451C15.857 4.451 17.571 8.451 18.429 10.451M13.286 8.951L18.429 10.451L21 6.951" />
      <path d="M8.459 12.178V19.451H7.358V13.279H7.315L5.575 14.415V13.364L7.39 12.178H8.459ZM12.827 19.55C12.382 19.55 11.982 19.465 11.626 19.294C11.274 19.122 10.992 18.885 10.781 18.585C10.571 18.284 10.458 17.941 10.444 17.555H11.509C11.535 17.867 11.674 18.124 11.925 18.325C12.176 18.526 12.476 18.627 12.827 18.627C13.106 18.627 13.353 18.562 13.569 18.434C13.787 18.305 13.957 18.126 14.08 17.898C14.206 17.672 14.268 17.413 14.268 17.121C14.268 16.826 14.205 16.562 14.077 16.33C13.949 16.098 13.772 15.915 13.548 15.782C13.325 15.65 13.069 15.583 12.781 15.58C12.56 15.58 12.339 15.618 12.116 15.694C11.894 15.77 11.714 15.869 11.577 15.992L10.572 15.843L10.98 12.178H14.979V13.119H11.893L11.662 15.154H11.705C11.847 15.017 12.035 14.901 12.269 14.809C12.506 14.717 12.759 14.671 13.029 14.671C13.472 14.671 13.866 14.776 14.212 14.987C14.56 15.198 14.833 15.486 15.032 15.85C15.233 16.212 15.333 16.629 15.33 17.1C15.333 17.571 15.226 17.991 15.011 18.361C14.798 18.73 14.502 19.02 14.123 19.234C13.746 19.445 13.314 19.55 12.827 19.55Z" fill="currentColor" stroke="none" />
    </>
  ),

  // Icon.svg "name=speed" (Ellipse 13 + Vector 82; needle keeps its source stroke-width 1.28571)
  "playback-speed": (
    <>
      <path d="M21 12C21 16.971 16.971 21 12 21C7.029 21 3 16.971 3 12C3 7.029 7.029 3 12 3" />
      <path d="M11.162 9.419L18 6L14.581 12.838C13.803 14.393 11.73 14.73 10.5 13.5C9.27 12.27 9.607 10.197 11.162 9.419Z" fill="currentColor" strokeWidth={1.28571} />
    </>
  ),

  // Icon.svg "name=repeat, direction=right" (Vector 27_13/28_6)
  "repeat": (
    <>
      <path d="M16 10L19 7L16 4M19 7H8C6.343 7 5 8.343 5 10" />
      <path d="M8 14L5 17L8 20M5 17L16 17C17.657 17 19 15.657 19 14" />
    </>
  ),

  // Icon.svg "name=shuffle, direction=right" (Vector 27_15/28_8)
  "shuffle": (
    <>
      <path d="M18 10.25L20.5 7.75L18 5.25M20.5 7.75H15.866C13.308 7.75 11.086 9.51 10.5 12C9.914 14.49 7.692 16.25 5.134 16.25H3.5" />
      <path d="M18 13.75L20.5 16.25L18 18.75M20.5 16.25L15.866 16.25C13.308 16.25 11.086 14.49 10.5 12C9.914 9.51 7.692 7.75 5.134 7.75L3.5 7.75" />
    </>
  ),

  // Icon.svg "name=record" (Ellipse 14 ring + filled Ellipse 15)
  "record": (
    <>
      <path d="M20.25 12C20.25 16.556 16.556 20.25 12 20.25C7.444 20.25 3.75 16.556 3.75 12C3.75 7.444 7.444 3.75 12 3.75C16.556 3.75 20.25 7.444 20.25 12Z" />
      <path d="M15.25 12C15.25 13.795 13.795 15.25 12 15.25C10.205 15.25 8.75 13.795 8.75 12C8.75 10.205 10.205 8.75 12 8.75C13.795 8.75 15.25 10.205 15.25 12Z" fill="currentColor" />
    </>
  ),

  // Icon.svg "name=snapshot" (Ellipse 15_4 + Vector 22_6 focus frame)
  "snapshot": (
    <>
      <path d="M15.25 12C15.25 13.795 13.795 15.25 12 15.25C10.205 15.25 8.75 13.795 8.75 12C8.75 10.205 10.205 8.75 12 8.75C13.795 8.75 15.25 10.205 15.25 12Z" />
      <path d="M8.8 4H4V8.8M15.2 4H20V8.8M20 15.2V20H15.2M8.8 20H4V15.2" />
    </>
  ),

  // Icon.svg "name=PiP" (Vector 22_11 + filled Vector 23_14)
  "pip": (
    <>
      <path d="M6 6H18V18H6V6Z" />
      <path d="M18 13H13V18H18V13Z" fill="currentColor" />
    </>
  ),

  // Icon.svg "name=HDR" (Rectangle 255 + outlined "HDR" letters)
  "hdr": (
    <>
      <rect x="2" y="3" width="20" height="18" rx="4" />
      <path d="M4.472 15V9.57H5.62V11.81H7.951V9.57H9.096V15H7.951V12.757H5.62V15H4.472ZM11.968 15H10.043V9.57H11.984C12.53 9.57 13 9.678 13.394 9.896C13.789 10.111 14.092 10.422 14.304 10.827C14.518 11.231 14.625 11.716 14.625 12.28C14.625 12.845 14.518 13.331 14.304 13.738C14.092 14.144 13.787 14.456 13.389 14.674C12.993 14.891 12.519 15 11.968 15ZM11.191 14.016H11.92C12.259 14.016 12.545 13.956 12.777 13.836C13.01 13.714 13.185 13.526 13.302 13.271C13.42 13.015 13.479 12.684 13.479 12.28C13.479 11.878 13.42 11.55 13.302 11.296C13.185 11.041 13.011 10.854 12.779 10.734C12.548 10.614 12.262 10.553 11.923 10.553H11.191V14.016ZM15.475 15V9.57H17.618C18.028 9.57 18.378 9.643 18.668 9.79C18.959 9.935 19.181 10.141 19.333 10.408C19.487 10.673 19.564 10.985 19.564 11.344C19.564 11.704 19.486 12.014 19.33 12.274C19.175 12.532 18.949 12.73 18.654 12.868C18.361 13.006 18.006 13.075 17.588 13.075H16.154V12.152H17.403C17.622 12.152 17.804 12.122 17.949 12.062C18.094 12.002 18.202 11.912 18.272 11.792C18.345 11.671 18.381 11.522 18.381 11.344C18.381 11.163 18.345 11.011 18.272 10.887C18.202 10.764 18.093 10.67 17.946 10.606C17.801 10.541 17.618 10.508 17.397 10.508H16.623V15H15.475ZM18.408 12.529L19.757 15H18.49L17.169 12.529H18.408Z" fill="currentColor" stroke="none" />
    </>
  ),

  // Icon.svg "name=zoom" (Ellipse 5_2 + Vector 39_4/40_9)
  "zoom": (
    <>
      <path d="M15.791 15.791C17.094 14.488 17.9 12.688 17.9 10.7C17.9 6.724 14.676 3.5 10.7 3.5C6.724 3.5 3.5 6.724 3.5 10.7C3.5 14.676 6.724 17.9 10.7 17.9C12.688 17.9 14.488 17.094 15.791 15.791ZM15.791 15.791L20.5 20.5" />
      <path d="M10.5 7.5V13.5" />
      <path d="M13.5 10.5L7.5 10.5" />
    </>
  ),

  // Icon.svg "name=sidebar, direction=right" (mask shape + Rectangle 4_2 bar)
  "right-sidebar": (
    <>
      <path d="M3 6C3 4.343 4.343 3 6 3H18C19.657 3 21 4.343 21 6V18C21 19.657 19.657 21 18 21H6C4.343 21 3 19.657 3 18V6Z" />
      <rect x="16" y="6" width="2" height="12" rx="0.5" fill="currentColor" stroke="none" />
    </>
  ),

  // Icon.svg "name=quit" (Rectangle 2_16 + Vector 27_20)
  "quit": (
    <>
      <path d="M12 3H6C4.343 3 3 4.343 3 6V18C3 19.657 4.343 21 6 21H12" />
      <path d="M16 17L21 12L16 7M21 12H9" />
    </>
  ),

  // Icon.svg "name=settings" (Vector 75 + Ellipse 10) — same geometry as the menu.svg Settings row
  "settings": (
    <>
      <path d="M12.504 21H11.597C10.707 21 9.985 20.278 9.985 19.388C9.985 18.816 9.683 18.288 9.191 17.998L8.251 17.445C7.728 17.137 7.078 17.138 6.555 17.447C5.761 17.917 4.736 17.653 4.268 16.857L3.873 16.185C3.421 15.417 3.686 14.427 4.462 13.988C4.97 13.7 5.28 13.157 5.27 12.572L5.252 11.491C5.242 10.918 4.93 10.392 4.431 10.11C3.645 9.664 3.376 8.66 3.834 7.881L4.245 7.183C4.7 6.409 5.693 6.143 6.474 6.586C6.986 6.876 7.614 6.871 8.121 6.572L9.089 6.002C9.582 5.712 9.884 5.183 9.884 4.612C9.884 3.722 10.605 3 11.496 3H12.425C13.303 3 14.015 3.712 14.015 4.59C14.015 5.172 14.333 5.707 14.844 5.986L15.833 6.524C16.34 6.8 16.953 6.795 17.455 6.511C18.248 6.062 19.254 6.331 19.717 7.117L20.127 7.815C20.579 8.583 20.314 9.573 19.538 10.012C19.03 10.3 18.719 10.843 18.73 11.427L18.748 12.51C18.758 13.083 19.07 13.608 19.569 13.89C20.355 14.336 20.624 15.34 20.166 16.119L19.755 16.817C19.3 17.591 18.307 17.857 17.526 17.414C17.014 17.124 16.386 17.129 15.879 17.428L14.91 17.998C14.418 18.288 14.116 18.816 14.116 19.388C14.116 20.278 13.394 21 12.504 21Z" />
      <path d="M14.25 12C14.25 13.243 13.243 14.25 12 14.25C10.757 14.25 9.75 13.243 9.75 12C9.75 10.757 10.757 9.75 12 9.75C13.243 9.75 14.25 10.757 14.25 12Z" />
    </>
  ),

  // Icon.svg "name=home" (mask shape + Rectangle 3 door)
  "go-home": (
    <>
      <path d="M3 10.387C3 9.383 3.501 8.447 4.336 7.89L10.336 3.89C11.344 3.219 12.656 3.219 13.664 3.89L19.664 7.89C20.499 8.447 21 9.383 21 10.387V17.781C21 19.438 19.657 20.781 18 20.781H6C4.343 20.781 3 19.438 3 17.781V10.387Z" />
      <rect x="10" y="11.781" width="4" height="6" rx="1" fill="currentColor" stroke="none" />
    </>
  ),

  // Icon.svg "name=file" (mask shape) — same page geometry as the menu.svg "Open File(s)" row
  "open-file": <path d="M3 8C3 4.686 5.686 2 9 2H17C19.209 2 21 3.791 21 6V18C21 20.209 19.209 22 17 22H7C4.791 22 3 20.209 3 18V8Z" />,

  // Icon.svg "name=folder" (Vector 23_2) — matches menu.svg "Open Folder"
  "open-folder": folderGlyph,

  // Icon.svg "name=streaming" (Ellipse 1-3) — matches menu.svg "Open Network Stream"
  "network-stream": (
    <>
      <circle cx="12" cy="12" r="2" fill="currentColor" stroke="none" />
      <path d="M7.757 7.757C6.672 8.843 6 10.343 6 12C6 13.657 6.672 15.157 7.757 16.243" />
      <path d="M16.243 16.243C17.328 15.157 18 13.657 18 12C18 10.343 17.328 8.843 16.243 7.757" />
      <path d="M19.071 4.929C20.881 6.738 22 9.238 22 12C22 14.761 20.881 17.261 19.071 19.071M4.929 19.071C3.119 17.261 2 14.761 2 12C2 9.238 3.119 6.738 4.929 4.929" />
    </>
  ),

  // Icon.svg "name=disc" (Ellipse 19_2/20_2 + Vector 105_2/106_2) — matches menu.svg "Open Disc"
  "open-disc": (
    <>
      <path d="M21.25 12C21.25 17.109 17.109 21.25 12 21.25C6.891 21.25 2.75 17.109 2.75 12C2.75 6.891 6.891 2.75 12 2.75C17.109 2.75 21.25 6.891 21.25 12Z" />
      <path d="M6 10L8 10C8 10 8 9.333 8.667 8.667C9.333 8 10 8 10 8L10 6C10 6 8.667 6 7.333 7.333C6 8.667 6 10 6 10Z" fill="currentColor" />
      <path d="M18 14L16 14C16 14 16 14.667 15.333 15.333C14.667 16 14 16 14 16L14 18C14 18 15.333 18 16.667 16.667C18 15.333 18 14 18 14Z" fill="currentColor" />
      <circle cx="12" cy="12" r="2" />
    </>
  ),

  // Icon.svg "name=capture-device" (Vector 107 + Vector 16_18) — same vectors as menu.svg "Open Capture Device"
  "capture-device": (
    <>
      <path d="M2.5 2H4.5C5.052 2 5.5 2.448 5.5 3V4M5.5 4V18M5.5 4H19.5C20.605 4 21.5 4.895 21.5 6V16C21.5 17.105 20.605 18 19.5 18H18.5M5.5 18V22M5.5 18H9.5M9.5 18V20C9.5 20.552 9.948 21 10.5 21H12.5M9.5 18H12.5M12.5 21V18M12.5 21H15.5M12.5 18H15.5M15.5 21V18M15.5 21H17.5C18.052 21 18.5 20.552 18.5 20V18M15.5 18H18.5" />
      <path d="M12.5 8V14L16.5 11L12.5 8Z" />
    </>
  ),

  // Icon.svg "name=swap" (Vector 42_5/43_3) — same arrows as menu.svg "Convert"
  "convert": (
    <>
      <path d="M6 17.5L19 17.5M15.5 14L19 17.5L15.5 21" />
      <path d="M18 6.5H5M8.5 10L5 6.5L8.5 3" />
    </>
  ),

  // Icon.svg "name=hotkeys" (Vector 9-17 + Rectangle 1) — matches menu.svg "Keyboard Shortcuts"
  "keyboard-shortcuts": (
    <>
      <path d="M9 16H15" />
      <path d="M6 9H6" />
      <path d="M6 12H6" />
      <path d="M10 9H10" />
      <path d="M10 12H10" />
      <path d="M14 9H14" />
      <path d="M14 12H14" />
      <path d="M18 9H18" />
      <path d="M18 12H18" />
      <path d="M5 4.75H19C20.795 4.75 22.25 6.205 22.25 8V16C22.25 17.795 20.795 19.25 19 19.25H5C3.205 19.25 1.75 17.795 1.75 16V8C1.75 6.205 3.205 4.75 5 4.75Z" />
    </>
  ),

  // Icon.svg "name=paint" (Rectangle 256 + Vector 100) — same vectors as menu.svg "Customize Player"
  "customize-player": wandGlyph,

  // Icon.svg "name=info" (Ellipse 9_2 + Vector 98/99) — matches menu.svg "About VLC"
  "about": infoGlyph,

  // ===== w26.a07 additions (menus, settings areas, misc) =====

  // Icon.svg "name=paint" (Rectangle 256 + Vector 100) — alias of "customize-player": the wand
  "wand": wandGlyph,

  // Icon.svg "name=recent" (Frame 1_28) — clock with rewind arrow; identical to
  // menu.svg "Open Recent" (Ellipse 12 + Vector 101)
  "recent": (
    <>
      <path d="M13.3575 5C17.1445 5 20.2145 8.134 20.2145 12C20.2145 15.866 17.1445 19 13.3575 19C9.5705 19 6.5005 15.866 6.5005 12V10.444M3.7855 13.159L6.5005 10.444L9.2145 13.159" />
      <path d="M13.2145 8V11.143C13.2145 11.616 13.5985 12 14.0715 12H15.7145" />
    </>
  ),

  // Icon.svg "name=refresh" (Frame_88/89 region) — twin refresh arrows
  "refresh": (
    <>
      <path d="M20 10C19.5714 8 17.4286 4 12.2857 4C7.1429 4 5.4286 8 4.5714 10M9.7143 8.5L4.5714 10L2 6.5" />
      <path d="M4 14C4.4286 16 6.5714 20 11.7143 20C16.8571 20 18.5714 16 19.4286 14M14.2857 15.5L19.4286 14L22 17.5" />
    </>
  ),

  // Icon.svg "name=save" (Frame 1_76, two-tone=yes) — floppy: outline + two fills
  "save": (
    <>
      <path d="M3 17V7C3 4.791 4.791 3 7 3H8H13H13.343C14.404 3 15.421 3.421 16.172 4.172L18 6L19.828 7.828C20.579 8.579 21 9.596 21 10.657V17C21 19.209 19.209 21 17 21H16H8H7C4.791 21 3 19.209 3 17Z" />
      <path d="M9 12.778C8.448 12.778 8 13.225 8 13.778V18C8 18.552 8.448 19 9 19H15C15.552 19 16 18.552 16 18V13.778C16 13.225 15.552 12.778 15 12.778H9Z" fill="currentColor" stroke="none" />
      <path d="M13 6C13 5.448 12.552 5 12 5H9C8.448 5 8 5.448 8 6V7.889C8 8.441 8.448 8.889 9 8.889H12C12.552 8.889 13 8.441 13 7.889V6Z" fill="currentColor" stroke="none" />
    </>
  ),

  // Icon.svg "name=edit" (Frame 1_75) — pencil with baseline
  "edit": (
    <>
      <path d="M7.0365 20.0445H19.0365" />
      <path d="M15.0115 10.2255L9.2845 15.8555L5.6405 17.0215C5.2965 17.1315 4.9635 16.8265 5.0505 16.4815L6.0225 12.6485L11.7495 7.0185L14.5045 4.3105C14.8645 3.9555 15.4485 3.9555 15.8095 4.3105L17.7665 6.2345C18.1265 6.5885 18.1265 7.1625 17.7665 7.5175L15.0115 10.2255Z" />
    </>
  ),

  // Icon.svg "name=remove" (Frame 1_18) — the minus bar
  "remove": <path d="M19 12H5" />,

  // Icon.svg "name=music" (Frame 1_20) — eighth note (music mode)
  "music": <path d="M18.291 6H13.959V13.5M13.959 13.5C13.959 13.5 13.959 18 9.459 18C7.043 18 5.709 15.75 6.537 14.093C8.175 10.817 13.959 13.5 13.959 13.5Z" />,

  // Icon.svg "name=chapters" (Frame 1_44) — play chip + three rows
  "chapters": (
    <>
      <path d="M3 15V9L7 12L3 15Z" fill="currentColor" />
      <path d="M11 6H21" />
      <path d="M11 12H21" />
      <path d="M11 18H21" />
    </>
  ),

  // Icon.svg "name=bookmark" (Frame 1_46) — bookmark with plus
  "bookmark": (
    <>
      <path d="M5.955 4.063L5.0916 19.605C5.0531 20.298 5.8116 20.747 6.4008 20.38L11.5466 17.172C11.8241 16.999 12.1759 16.999 12.4534 17.172L17.5992 20.38C18.1884 20.747 18.9469 20.298 18.9084 19.605L18.045 4.063C18.0197 3.608 17.6441 3.253 17.1892 3.253H6.8108C6.3559 3.253 5.9803 3.608 5.955 4.063Z" />
      <path d="M12 7.253V13.253" />
      <path d="M15 10.253H9" />
    </>
  ),

  // Icon.svg "name=interface" (Frame 1_23) — cursor arrow (Settings → Interface)
  "interface": <path d="M6 2.274V17.803L11 16.774L13.033 20.646C13.44 21.42 14.39 21.726 15.172 21.335C15.957 20.943 16.281 19.992 15.899 19.201L14 15.274L18 12.156L6 2.274Z" />,

  // Icon.svg "name=privacy" (Frame 1_23 region) — shield (Settings → Privacy)
  "privacy": <path d="M20 8.3425V9.0165C20 13.1505 18.174 17.0715 15.01 19.7325L14.574 20.0985C13.086 21.3505 10.914 21.3505 9.426 20.0985L8.99 19.7325C5.826 17.0715 4 13.1505 4 9.0165V8.3425C4 7.0915 4.776 5.9725 5.947 5.5335L6.874 5.1855C7.623 4.9055 8.342 4.5495 9.02 4.1255L10.41 3.2575C11.383 2.6495 12.617 2.6495 13.59 3.2575L14.98 4.1255C15.658 4.5495 16.377 4.9055 17.126 5.1855L18.053 5.5335C19.224 5.9725 20 7.0915 20 8.3425Z" />,

  // Icon.svg "name=playback" (Frame 1_13) — outlined play (Settings → Playback;
  // right-click "Playback" row uses the same triangle)
  "playback": <path d="M8 6V18L16 12L8 6Z" />,

  // Icon.svg "name=plugins" (Frame 1_53) — puzzle piece (Settings → Extensions)
  "plugins": <path d="M6.6 5H4C3.448 5 3 5.448 3 6V8.5V9.25C3 9.802 3.448 10.25 4 10.25H4.85C5.816 10.25 6.6 11.034 6.6 12C6.6 12.966 5.816 13.75 4.85 13.75H4C3.448 13.75 3 14.198 3 14.75V15.5V18C3 18.552 3.448 19 4 19H6.6H7.4C7.952 19 8.4 18.552 8.4 18V17.3C8.4 16.306 9.206 15.5 10.2 15.5C11.194 15.5 12 16.306 12 17.3V18C12 18.552 12.448 19 13 19H13.8H16.4C16.952 19 17.4 18.552 17.4 18V15.5V14.75C17.4 14.198 17.848 13.75 18.4 13.75H19.25C20.216 13.75 21 12.966 21 12C21 11.034 20.216 10.25 19.25 10.25H18.4C17.848 10.25 17.4 9.802 17.4 9.25V8.5V6C17.4 5.448 16.952 5 16.4 5H13.8H13C12.448 5 12 5.448 12 6V6.391C12 7.385 11.194 8.191 10.2 8.191C9.206 8.191 8.4 7.385 8.4 6.391V6C8.4 5.448 7.952 5 7.4 5H6.6Z" />,

  // Icon.svg "name=tv" (Frame_67) — tv + play + antenna (Settings → Video area)
  "tv": (
    <>
      <path d="M3 10.5C3 8.843 4.343 7.5 6 7.5H18C19.657 7.5 21 8.843 21 10.5V18.5C21 20.157 19.657 21.5 18 21.5H6C4.343 21.5 3 20.157 3 18.5V10.5Z" />
      <path d="M9 3.75L12 7.5L15 2.5" />
      <path d="M11 16.5V12.5L14 14.5L11 16.5Z" fill="currentColor" stroke="none" />
    </>
  ),

  // Icon.svg "name=wrench" (Frame_98) — wrench (Settings → General); needle keeps
  // its source stroke-width 1.28571
  "wrench": <path d="M16.0875 3.2715H14.2265C13.9995 3.2715 13.7815 3.3615 13.6205 3.5225L11.5505 5.5915C11.3905 5.7525 11.2995 5.9705 11.2995 6.1975V9.7735C11.2995 10.0005 11.2095 10.2185 11.0485 10.3795L3.9405 17.4875C3.2715 18.1575 3.2715 19.2425 3.9405 19.9125L4.0875 20.0595C4.7575 20.7285 5.8425 20.7285 6.5125 20.0595L13.6205 12.9515C13.7815 12.7905 13.9995 12.7005 14.2265 12.7005H17.8025C18.0295 12.7005 18.2475 12.6095 18.4085 12.4495L20.4775 10.3795C20.6385 10.2185 20.7285 10.0005 20.7285 9.7735V7.9125C20.7285 7.1485 19.8055 6.7665 19.2655 7.3065L17.5505 9.0205C17.3905 9.1815 17.1725 9.2715 16.9445 9.2715H15.5855C15.1125 9.2715 14.7285 8.8875 14.7285 8.4145V7.0555C14.7285 6.8275 14.8185 6.6095 14.9795 6.4495L16.6935 4.7345C17.2335 4.1945 16.8515 3.2715 16.0875 3.2715Z" strokeWidth={1.28571} />,

  // Icon.svg "name=accessibility" (Frame 1_25) — person in circle
  "accessibility": (
    <>
      <circle cx="12" cy="12" r="11" />
      <circle cx="12" cy="6" r="2" fill="currentColor" stroke="none" />
      <path d="M12 13L9 19M12 13L15 19M12 13V11.81V10.619M12 10.619L13 10.295L17 9M12 10.619L11 10.295L7 9" />
    </>
  ),

  // Icon.svg "name=divider" (Frame_72) — vertical rule (customize panel separators)
  "divider": <path d="M12 3V21" />,

  // Icon.svg "name=vlc" (Frame 4) — the traffic-cone mark, all filled
  "vlc": (
    <>
      <path d="M16.8185 14.5665L17.2075 15.7335C16.0415 16.5885 14.2535 17.3665 12.0775 17.3665C9.8235 17.3665 8.1135 16.5885 6.9485 15.7335L7.3365 14.5665H5.5495C5.1605 14.5665 4.9275 14.7995 4.7715 15.1115L3.0625 21.6445C2.9065 22.1115 3.2955 22.6555 3.8395 22.6555H20.2385C20.7825 22.6555 21.0935 22.1885 21.0155 21.6445L19.2285 15.1115C19.1505 14.7995 18.8395 14.5665 18.4505 14.5665H16.8185Z" fill="currentColor" stroke="none" />
      <path d="M7.8805 12.3105C8.8135 13.0105 10.2125 13.6325 12.0005 13.6325C13.7875 13.6325 15.1865 12.9325 16.1195 12.3105L15.0315 8.5775C14.3315 9.0445 13.3215 9.3555 12.0005 9.3555C10.6785 9.3555 9.6685 8.9665 8.9685 8.5775L7.8805 12.3105Z" fill="currentColor" stroke="none" />
      <path d="M9.9795 4.9995L10.6785 2.5885C11.0675 1.3445 12.8545 1.3445 13.2435 2.5885L13.9425 4.9995C13.3985 5.1555 12.7775 5.3115 11.9995 5.3115C11.2225 5.3115 10.6005 5.1555 9.9795 4.9995Z" fill="currentColor" stroke="none" />
    </>
  ),

  // right-click menu.svg "Hide UI" row — rounded square + diagonal (Rectangle 2 +
  // Vector 115; the accent slash inherits currentColor like every other stroke)
  "hide-ui": (
    <>
      <path d="M6 21H18C19.657 21 21 19.657 21 18V6C21 4.343 19.657 3 18 3H6C4.343 3 3 4.343 3 6V18C3 19.657 4.343 21 6 21Z" />
      <path d="M8 8L16 16" />
    </>
  ),
};

export const iconNames = Object.keys(glyphs) as IconName[];

export function getIconGlyph(name: IconName): ReactElement {
  return glyphs[name];
}
