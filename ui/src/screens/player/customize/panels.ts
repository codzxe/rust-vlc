// w26.gfixA (issue #62) — the customize tile catalog, rebuilt to the
// frame-measured inventory of `design/figma/Player - customize.svg` (the
// 31-tile box, rows 1–4 × 9 columns — verified byte-identical across all
// 9 customize frames by w26.g08). Pure DATA:
//
//   * engine-bound tiles name the command they dispatch (capability-gated
//     in capabilities.ts — hidden until the engine lists the command, per
//     doc 05 §3 "no dead controls");
//   * UI-owned toolbar items (command: null) always render — they are
//     toolbar layout primitives with no engine surface (w26.g07-P1-2's
//     catalog split);
//   * row order + column order are the frame's exact reading order.
//
// Tile glyph note: the frames draw one shared "text block" glyph for the
// five Name/Properties tiles; the icon registry carries no such glyph, so
// those tiles use their semantic registry icons (artist/folder/…). A
// dedicated glyph is an icon-lane handoff (recorded in the lane log).
//
// The select/slider adjustment rows of the rail era are GONE from this
// surface — no customize frame contains a section, slider or dropdown
// (w26.g07-P2-1, w26.g08-P2-4, w26.g09-P2-4); that grammar lives in the
// tools dialog (dialogs/ToolsDialog.tsx) which already hosts the Crop,
// Colors, Essential and HDR tools.

import type { IconName } from "../../../components/icon/icons";
import type {
  CommandArgs,
  CustomizeControl,
  CustomizeOption,
  CustomizeTile,
  ToolbarLayout,
} from "./types";

/** Repeat cycle (engine modes, frame row 1 "Repeat"). */
const REPEAT_OPTIONS: readonly CustomizeOption[] = [
  { id: "off", args: { mode: "off" } },
  { id: "all", args: { mode: "all" } },
  { id: "one", args: { mode: "one" } },
];

/** Playback-rate cycle (frame row 1 "Playback Speed"). */
const RATE_OPTIONS: readonly CustomizeOption[] = [
  { id: "0.25", args: { rate: 0.25 } },
  { id: "0.5", args: { rate: 0.5 } },
  { id: "0.75", args: { rate: 0.75 } },
  { id: "1", args: { rate: 1 } },
  { id: "1.25", args: { rate: 1.25 } },
  { id: "1.5", args: { rate: 1.5 } },
  { id: "2", args: { rate: 2 } },
];

/** Zoom cycle (frame row 2 "Zoom"). */
const ZOOM_OPTIONS: readonly CustomizeOption[] = [
  { id: "0.5", args: { factor: 0.5 } },
  { id: "1", args: { factor: 1 } },
  { id: "1.5", args: { factor: 1.5 } },
  { id: "2", args: { factor: 2 } },
];

/** A UI-owned toolbar item (no engine command, never dispatches). */
function uiTile(
  id: string,
  labelKey: CustomizeTile["labelKey"],
  icon?: IconName,
): CustomizeTile {
  return { id, labelKey, command: null, control: { kind: "ui" }, icon };
}

function seekTile(
  id: string,
  labelKey: CustomizeTile["labelKey"],
  deltaMs: number,
  icon: IconName,
): CustomizeTile {
  return {
    id,
    labelKey,
    command: "playback.seek",
    icon,
    control: { kind: "action", args: { target: { kind: "relative", deltaMs } } },
  };
}

/**
 * THE CATALOG — the frame's 31 tiles in exact frame reading order
 * (row 1: Spacer…Repeat · row 2: Shuffle…VLC Settings · row 3: Go
 * Home…Chapters · row 4: Favorite…Playlists).
 */
export const CUSTOMIZE_TILES: readonly CustomizeTile[] = [
  // ---- row 1 (frame y=318) ------------------------------------------------
  // Spacer: the frame's well is EMPTY (no glyph) — a toolbar spacer.
  uiTile("spacer", "customize.tile.spacer"),
  uiTile("divider", "customize.tile.divider", "divider"),
  {
    id: "playback-speed",
    labelKey: "customize.tile.playback-speed",
    command: "playback.set-rate",
    icon: "playback-speed",
    control: { kind: "cycle", options: RATE_OPTIONS, defaultOption: "1" },
  },
  seekTile("seek-fwd-10", "customize.tile.forward-10", 10_000, "skip-forward-10"),
  seekTile("seek-back-10", "customize.tile.backward-10", -10_000, "skip-back-10"),
  seekTile("seek-fwd-15", "customize.tile.forward-15", 15_000, "skip-forward-15"),
  seekTile("seek-back-15", "customize.tile.backward-15", -15_000, "skip-back-15"),
  {
    id: "stop",
    labelKey: "player.controls.stop",
    command: "playback.stop",
    icon: "stop",
    control: { kind: "action", args: {} },
  },
  {
    id: "repeat",
    labelKey: "customize.action.repeat",
    command: "playlist.set-repeat",
    icon: "repeat",
    control: { kind: "cycle", options: REPEAT_OPTIONS, defaultOption: "off" },
  },
  // ---- row 2 (frame y=438) ------------------------------------------------
  {
    id: "shuffle",
    labelKey: "customize.action.shuffle",
    command: "playlist.set-shuffle",
    icon: "shuffle",
    control: {
      kind: "toggle-action",
      argsFor: (engineValue) => ({ enabled: !(engineValue ?? false) }),
    },
  },
  {
    // No record command in the v1 catalog (keymaps/default.toml note:
    // feature-gated) — forward-declared id, hidden until an engine lists it.
    id: "record",
    labelKey: "customize.tile.record",
    command: "video.record",
    icon: "record",
    control: { kind: "action", args: {} },
  },
  {
    id: "snapshot",
    labelKey: "customize.tile.snapshot",
    command: "video.take-screenshot",
    icon: "snapshot",
    control: { kind: "action", args: {} },
  },
  {
    // Forward-declared (no PiP command in the 1.D4 wire union yet).
    id: "pip",
    labelKey: "customize.tile.pip",
    command: "video.toggle-pip",
    icon: "pip",
    control: { kind: "action", args: {} },
  },
  {
    // Forward-declared (HDR tone mapping; the tools dialog owns its panel).
    id: "hdr",
    labelKey: "customize.tile.hdr",
    command: "video.set-tone-mapping",
    icon: "hdr",
    control: { kind: "action", args: {} },
  },
  {
    id: "zoom",
    labelKey: "customize.tile.zoom",
    command: "video.set-zoom",
    icon: "zoom",
    control: { kind: "cycle", options: ZOOM_OPTIONS, defaultOption: "1" },
  },
  uiTile("right-sidebar", "customize.tile.right-sidebar", "right-sidebar"),
  uiTile("quit", "customize.tile.quit", "quit"),
  uiTile("vlc-settings", "customize.tile.vlc-settings", "settings"),
  // ---- row 3 (frame y=558) ------------------------------------------------
  uiTile("go-home", "customize.tile.go-home", "go-home"),
  uiTile("thumbnail", "customize.tile.thumbnail", "thumbnail"),
  uiTile("file-name", "customize.tile.file-name", "open-file"),
  uiTile("artist-name", "customize.tile.artist-name", "artist"),
  uiTile("folder-name", "customize.tile.folder-name", "folder"),
  uiTile("playlist-name", "customize.tile.playlist-name", "edit"),
  uiTile("properties", "customize.tile.properties", "about"),
  uiTile("replay", "customize.tile.replay", "recent"),
  uiTile("chapters", "customize.tile.chapters", "chapters"),
  // ---- row 4 (frame y=674) ------------------------------------------------
  uiTile("favorite", "customize.tile.favorite", "favorite"),
  uiTile("bookmark", "customize.tile.bookmark", "bookmark"),
  uiTile("bookmarks", "customize.tile.bookmarks", "lists"),
  uiTile("playlists", "customize.tile.playlists", "playlist"),
];

/** Catalog ids in frame order (the default box order). */
export const CUSTOMIZE_TILE_IDS: readonly string[] = CUSTOMIZE_TILES.map(
  (tile) => tile.id,
);

const TILE_BY_ID: ReadonlyMap<string, CustomizeTile> = new Map(
  CUSTOMIZE_TILES.map((tile) => [tile.id, tile]),
);

/** Catalog lookup by tile id (null = unknown id, never guessed). */
export function tileById(id: string): CustomizeTile | null {
  return TILE_BY_ID.get(id) ?? null;
}

/**
 * The default toolbar arrangement (frame -1 baseline): the catalog in
 * frame order + an EMPTY band — frame -1's clean control band is the
 * fixed transport cluster + the long empty slot + the right cluster (no
 * placed toolbar items; drops from the box add them, frames -6/-7).
 */
export const DEFAULT_TOOLBAR_LAYOUT: ToolbarLayout = {
  order: CUSTOMIZE_TILE_IDS,
  band: [],
};

/** The args payload a cycle control dispatches for its confirmed option. */
export function cycleArgs(
  control: Extract<CustomizeControl, { kind: "cycle" }>,
  optionId: string | null,
): CommandArgs {
  const options = control.options;
  const index =
    optionId === null
      ? options.findIndex((option) => option.id === control.defaultOption)
      : options.findIndex((option) => option.id === optionId);
  const current = index >= 0 ? index : 0;
  const next = (current + 1) % options.length;
  return options[next]!.args;
}
