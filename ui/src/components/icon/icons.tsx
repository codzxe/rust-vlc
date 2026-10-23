import type { ReactElement } from "react";

/**
 * Card 4.B4 — icon system.
 * Design provenance: docs/08-ui-figma-fidelity.md §1.4 "Icon system: line
 * icons, stroke #8F8F8F, stroke-width 1.5 (3464 uses; 2 and 3 for emphasis);
 * hit wrapper ~48 (Icon Wrapper.svg)" and §1.5/§1.6 component + customize
 * catalogs, which are the ONLY sources for icon names below (never invent).
 *
 * Geometry: the exported Icon.svg sheet carries anonymous layer ids
 * ("Frame 1_17", "Ellipse 12_3" …), so exact per-glyph paths cannot yet be
 * attributed to names mechanically. The paths below are provisional standard
 * line-icon geometry on a 24×24 grid at stroke-width 1.5.
 * TODO [VERIFY-DESIGN]: mine real paths from design/figma/Icon.svg and
 * Controls.svg once per-glyph attribution exists (see NEXT-STEPS).
 */

export type IconName =
  // play.svg + Controls.svg (transport)
  | "play"
  | "pause"
  | "stop"
  // Controls.svg volume family: mute/low/med/high × states
  | "volume-mute"
  | "volume-low"
  | "volume-medium"
  | "volume-high"
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

/** One glyph = children rendered inside the stroked `<svg>` shell. */
const glyphs: Record<IconName, ReactElement> = {
  play: <path d="M8 5.5v13l11-6.5z" />,
  pause: (
    <>
      <path d="M9 5.5v13" />
      <path d="M15 5.5v13" />
    </>
  ),
  stop: <rect x="6.5" y="6.5" width="11" height="11" rx="1" />,
  "volume-mute": (
    <>
      <path d="M4 9.5h3l4.5-4v13l-4.5-4H4z" />
      <path d="M15.5 9.5l5 5m0-5l-5 5" />
    </>
  ),
  "volume-low": (
    <>
      <path d="M4 9.5h3l4.5-4v13l-4.5-4H4z" />
      <path d="M15 9.5a4 4 0 0 1 0 5" />
    </>
  ),
  "volume-medium": (
    <>
      <path d="M4 9.5h3l4.5-4v13l-4.5-4H4z" />
      <path d="M15 9a4 4 0 0 1 0 6" />
      <path d="M17.5 7a7.5 7.5 0 0 1 0 10" />
    </>
  ),
  "volume-high": (
    <>
      <path d="M4 9.5h3l4.5-4v13l-4.5-4H4z" />
      <path d="M15 9a4 4 0 0 1 0 6" />
      <path d="M17.5 7a7.5 7.5 0 0 1 0 10" />
      <path d="M20 5a11 11 0 0 1 0 14" />
    </>
  ),
  "skip-back-10": (
    <>
      <path d="M11 6H5.5v5.5" />
      <path d="M6 6.5a8 8 0 1 1-1.5 6" />
      <path d="M11 10.2l1.3-1v6.5" />
      <ellipse cx="15.1" cy="12.7" rx="1.9" ry="3.2" />
    </>
  ),
  "skip-forward-10": (
    <>
      <path d="M13 6h5.5v5.5" />
      <path d="M18 6.5a8 8 0 1 0 1.5 6" />
      <path d="M11 10.2l1.3-1v6.5" />
      <ellipse cx="15.1" cy="12.7" rx="1.9" ry="3.2" />
    </>
  ),
  "skip-back-15": (
    <>
      <path d="M11 6H5.5v5.5" />
      <path d="M6 6.5a8 8 0 1 1-1.5 6" />
      <path d="M11 10.2l1.3-1v6.5" />
      <path d="M14.3 9.7h3.3v3.1a3.3 3.3 0 1 1-3.3-3.1" />
    </>
  ),
  "skip-forward-15": (
    <>
      <path d="M13 6h5.5v5.5" />
      <path d="M18 6.5a8 8 0 1 0 1.5 6" />
      <path d="M11 10.2l1.3-1v6.5" />
      <path d="M14.3 9.7h3.3v3.1a3.3 3.3 0 1 1-3.3-3.1" />
    </>
  ),
  "playback-speed": (
    <>
      <path d="M12 6.5a5.5 5.5 0 1 0 5.5 5.5" />
      <path d="M17.5 12l3-3" />
      <path d="M9 12l3-2v4z" />
    </>
  ),
  repeat: (
    <>
      <path d="M7 5.5h10a3 3 0 0 1 3 3v2" />
      <path d="M17 18.5H7a3 3 0 0 1-3-3v-2" />
      <path d="M17 3l3 2.5-3 2.5" />
      <path d="M7 21l-3-2.5L7 16" />
    </>
  ),
  shuffle: (
    <>
      <path d="M4 6.5h4l8 11h4" />
      <path d="M4 17.5h4l2.5-3.5" />
      <path d="M13.5 10l2.5-3.5h4" />
      <path d="M18 4l2.5 2.5L18 9" />
      <path d="M18 15l2.5 2.5L18 20" />
    </>
  ),
  record: <circle cx="12" cy="12" r="5.5" />,
  snapshot: (
    <>
      <path d="M4 8.5A2 2 0 0 1 6 6.5h2l1.5-2h5l1.5 2h2a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2z" />
      <circle cx="12" cy="12.5" r="3.5" />
    </>
  ),
  pip: (
    <>
      <rect x="3.5" y="5.5" width="17" height="13" rx="2" />
      <rect x="12" y="11" width="7" height="6" rx="1" />
    </>
  ),
  hdr: (
    <>
      <rect x="3.5" y="6.5" width="17" height="11" rx="2" />
      <path d="M7 10v4" stroke-width="2" />
      <path d="M10.5 10v4" />
      <path d="M7 12h3.5" />
      <path d="M14 10v4h1.5a2 2 0 0 0 2-2 2 2 0 0 0-2-2z" />
    </>
  ),
  zoom: (
    <>
      <circle cx="11" cy="11" r="6" />
      <path d="M15.5 15.5L20 20" />
    </>
  ),
  "right-sidebar": (
    <>
      <rect x="3.5" y="4.5" width="17" height="15" rx="2" />
      <path d="M15.5 4.5v15" />
    </>
  ),
  quit: (
    <>
      <path d="M14 4.5H7a2 2 0 0 0-2 2v11a2 2 0 0 0 2 2h7" />
      <path d="M10.5 12H20" />
      <path d="M17 9l3 3-3 3" />
    </>
  ),
  settings: (
    <>
      <circle cx="12" cy="12" r="3" />
      <path d="M12 3.5v3M12 17.5v3M4.5 12h3M16.5 12h3M6.7 6.7l2.1 2.1M15.2 15.2l2.1 2.1M17.3 6.7l-2.1 2.1M8.8 15.2l-2.1 2.1" />
    </>
  ),
  "go-home": (
    <>
      <path d="M4.5 11L12 4.5l7.5 6.5" />
      <path d="M6.5 10v9.5h11V10" />
      <path d="M10 19.5v-5h4v5" />
    </>
  ),
  "open-file": (
    <>
      <path d="M13 4.5H7a2 2 0 0 0-2 2v11a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2v-8z" />
      <path d="M13 4.5V9.5h6" />
    </>
  ),
  "open-folder": <path d="M4 7a1.5 1.5 0 0 1 1.5-1.5H9l2 2h7.5A1.5 1.5 0 0 1 20 9v8a1.5 1.5 0 0 1-1.5 1.5h-13A1.5 1.5 0 0 1 4 17z" />,
  "network-stream": (
    <>
      <circle cx="12" cy="12" r="8" />
      <path d="M4 12h16" />
      <path d="M12 4c2.5 2.3 3.8 5 3.8 8s-1.3 5.7-3.8 8c-2.5-2.3-3.8-5-3.8-8s1.3-5.7 3.8-8z" />
    </>
  ),
  "open-disc": (
    <>
      <circle cx="12" cy="12" r="8" />
      <circle cx="12" cy="12" r="2.5" />
    </>
  ),
  "capture-device": (
    <>
      <rect x="3.5" y="7" width="12" height="10" rx="2" />
      <path d="M15.5 11l5-2.5v7l-5-2.5z" />
    </>
  ),
  convert: (
    <>
      <path d="M7 5.5v13" />
      <path d="M4 8l3-2.5L10 8" />
      <path d="M17 18.5v-13" />
      <path d="M14 16l3 2.5 3-2.5" />
    </>
  ),
  "keyboard-shortcuts": (
    <>
      <rect x="3.5" y="7.5" width="17" height="10" rx="2" />
      <path d="M6.5 10.5h.01M9.5 10.5h.01M12.5 10.5h.01M15.5 10.5h.01M18 10.5h-.01M6.5 13.5h.01M9.5 13.5h5M17.5 13.5h.01" />
    </>
  ),
  "customize-player": (
    <>
      <path d="M4.5 6.5h15v11h-15z" />
      <path d="M4.5 6.5l7.5 6.5 7.5-6.5" />
    </>
  ),
  about: (
    <>
      <circle cx="12" cy="12" r="8" />
      <path d="M12 11v5" />
      <path d="M12 8h.01" />
    </>
  ),
};

export const iconNames = Object.keys(glyphs) as IconName[];

export function getIconGlyph(name: IconName): ReactElement {
  return glyphs[name];
}
