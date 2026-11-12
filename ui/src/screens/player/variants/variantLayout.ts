// Card 4.F2 — variant-scoped layout data (doc 07 §7: "which panels … exist
// comes from layout config"; doc 08 §1.6 route anatomy).
//
// Design matrix (doc 08; [PROPOSAL] where the exports are silent about a
// region's presence — flagged per region below). w26.f11 (e03 F5/F6/F8):
// the music playlist docks LEFT (Player - Music-1.svg: rail 50px at x=0, the
// 377px column at x=50, y=40..1026) through the SAME docked-sidebar seam as
// the full variant — so the shell's `lumen:toggle-sidebar` toggle closes it
// and the Player - Music.png no-column state becomes reachable; the music
// title-bar region is GONE (the app shell's banner is the one topbar — e03
// F8 "doubled top bar"):
//
// | Region            | full | miniplayer | music  | Source |
// |-------------------|------|------------|--------|--------|
// | title-bar (top)   |  ✗   |     ✗      |   ✗    | the app shell's banner is the one topbar (w26.f09; e03 F8) |
// | video-canvas      |  ✓   |   thumb ✓  |   ✗    | §1.6 `player.music` shows album art, not video |
// | album-art (center)|  ✗   |     ✗      |   ✓    | §1.4 "Album art (music) 737×737 rx20" |
// | controls-bar      |  ✓   |  mini ✓    |   ✓    | §1.5 Controls kit reused (card 4.C2) |
// | library (left)    |  ✓   |     ✗      |   ✗    | Player - Music.svg shows no library sidebar [PROPOSAL] |
// | playlist (left)   | open |     ✗      |  open  | Player - Music-1.svg docks the column LEFT of the stage; toggle-closable (e03 F5/F6) |
// | media-info        |  ✓   |     ✗      |   ✗    | not in the music/mini exports [PROPOSAL] |
//
// Each non-full variant ships as its own layout.v1 TOML source parsed by
// C1's parser (same format as config/layouts/*.toml will be). Regions use
// the variant-scoped `visible_when` condition tokens ("miniplayer",
// "music-mode") added to layout.ts — meaningful whenever these sources are
// merged into or reused by a fuller layout.

import {
  DEFAULT_LAYOUT,
  parseLayoutConfig,
  resolveLayout,
  type LayoutConfig,
} from "../layout";
import type { PlayerVariant } from "./variant";

/** Compact-card layout — Miniplayer.svg "Default" variant (§1.4 geometry). */
export const MINIPLAYER_LAYOUT_TOML = `# Card 4.F2 — miniplayer layout [PROPOSAL] (stands in for
# config/layouts/miniplayer.toml; same schema as the default layout).
schema = "layout.v1"

[[region]]
id = "mini-thumbnail"
anchor = "center"
label_key = "player.canvas.label"
order = 10
visible_when = "miniplayer"

[[region]]
id = "mini-controls"
anchor = "bottom"
label_key = "player.variant.miniplayer.controls"
order = 10
visible_when = "miniplayer"
`;

/** Music-mode layout — Player - Music.svg + Player - Music-1.svg (art +
 * transport + the LEFT-docked playlist column; w26.f11 e03 F5/F6/F8). */
export const MUSIC_LAYOUT_TOML = `# Card 4.F2 — music-mode layout [PROPOSAL] (stands in for
# config/layouts/music.toml; same schema as the default layout).
# w26.f11 (e03 F5/F6/F8): the playlist docks LEFT through the same
# docked-sidebar seam as the full variant (Player - Music-1.svg: the 377px
# column sits at x=50, left of the stage) — the shell's
# lumen:toggle-sidebar event closes it (the Player - Music.png no-column
# state is reachable); no title-bar region (the app shell's banner is the
# one topbar — e03 F8 "doubled top bar").
schema = "layout.v1"

[[region]]
id = "album-art"
anchor = "center"
label_key = "player.art.label"
order = 10
visible_when = "music-mode"

[[region]]
id = "controls-bar"
anchor = "bottom"
label_key = "player.region.controls-bar"
order = 10

[[region]]
id = "playlist"
anchor = "left"
label_key = "player.region.playlist"
order = 10
visible_when = "music-mode"
`;

/** Parsed variant layouts (module-load; a bad source is a programmer error). */
export const MINIPLAYER_LAYOUT: LayoutConfig = parseLayoutConfig(MINIPLAYER_LAYOUT_TOML);
export const MUSIC_LAYOUT: LayoutConfig = parseLayoutConfig(MUSIC_LAYOUT_TOML);

/** The per-variant default layouts ("full" = C1's default layout). */
export const PLAYER_VARIANT_LAYOUTS: Readonly<Record<PlayerVariant, LayoutConfig>> = {
  full: DEFAULT_LAYOUT,
  miniplayer: MINIPLAYER_LAYOUT,
  music: MUSIC_LAYOUT,
};

/**
 * Resolve the effective layout for a variant. A present non-empty custom
 * source always wins (config owns existence/order — doc 07 §7.1); only when
 * the host supplies no source does the variant pick its default layout.
 * A present-but-invalid source throws LayoutParseError (C1 semantics).
 */
export function resolveVariantLayout(
  source: string | null | undefined,
  variant: PlayerVariant,
): LayoutConfig {
  if (source === null || source === undefined || source.trim() === "") {
    return PLAYER_VARIANT_LAYOUTS[variant];
  }
  return resolveLayout(source);
}
