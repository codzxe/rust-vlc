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
 */

export type IconName =
  // play.svg + Controls.svg (transport)
  | "play"
  | "pause"
  | "stop"
  // volume family: Icon.svg "close, value=none|low|medium" + "volume, value=high"
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

/** One glyph = children rendered inside the stroked `<svg>` shell.
 * Paths mined from design/figma/Icon.svg (see header for attribution). */
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
  "open-folder": <path d="M21 9.541V17C21 18.657 19.657 20 18 20H6C4.343 20 3 18.657 3 17V6.598C3 5.163 4.163 4 5.598 4H8.197C9.323 4 10.375 4.563 11 5.5C11.625 6.437 12.677 7 13.803 7H18.459C19.862 7 21 8.138 21 9.541Z" />,

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
  "customize-player": (
    <>
      <path d="M11.989 9.318L19.07 2.237C19.751 1.556 20.854 1.556 21.535 2.237C22.216 2.918 22.216 4.022 21.535 4.702L14.454 11.783C13.774 12.464 12.67 12.464 11.989 11.783C11.309 11.102 11.309 9.999 11.989 9.318Z" />
      <path d="M10.641 14.547C11.914 16.244 11.841 19.547 8.641 19.547C5.441 19.547 4.696 21.444 4.696 22.444C3.863 21.611 1.784 17.856 4.696 14.944C6.196 13.444 9.141 12.547 10.641 14.547Z" />
    </>
  ),

  // Icon.svg "name=info" (Ellipse 9_2 + Vector 98/99) — matches menu.svg "About VLC"
  "about": (
    <>
      <circle cx="12" cy="12" r="10" />
      <path d="M12 11V17" strokeWidth={2} />
      <path d="M12 7V7" strokeWidth={2} />
    </>
  ),
};

export const iconNames = Object.keys(glyphs) as IconName[];

export function getIconGlyph(name: IconName): ReactElement {
  return glyphs[name];
}
