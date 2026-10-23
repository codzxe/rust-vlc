// Card 4.F2 — variant-scoped layout data (doc 07 §7: "which panels … exist
// comes from layout config"; doc 08 §1.6 route anatomy).
//
// Design matrix (doc 08; [PROPOSAL] where the exports are silent about a
// region's presence — flagged per region below):
//
// | Region            | full | miniplayer | music  | Source |
// |-------------------|------|------------|--------|--------|
// | title-bar (top)   |  ✓   |     ✗      |   ✓    | §1.4 "music mode identical" topbar; Miniplayer.svg has no topbar |
// | video-canvas      |  ✓   |   thumb ✓  |   ✗    | §1.6 `player.music` shows album art, not video |
// | album-art (center)|  ✗   |     ✗      |   ✓    | §1.4 "Album art (music) 737×737 rx20" |
// | controls-bar      |  ✓   |  mini ✓    |   ✓    | §1.5 Controls kit reused (card 4.C2) |
// | library (left)    |  ✓   |     ✗      |   ✗    | Player - Music.svg shows no library sidebar [PROPOSAL] |
// | playlist (right)  | open |     ✗      |   ✓    | music = "playlist emphasis" (task card); Miniplayer.svg default has none [PROPOSAL] |
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

/** Music-mode layout — Player - Music.svg (art + transport + playlist). */
export const MUSIC_LAYOUT_TOML = `# Card 4.F2 — music-mode layout [PROPOSAL] (stands in for
# config/layouts/music.toml; same schema as the default layout).
schema = "layout.v1"

[[region]]
id = "album-art"
anchor = "center"
label_key = "player.art.label"
order = 10
visible_when = "music-mode"

[[region]]
id = "title-bar"
anchor = "top"
label_key = "player.region.title-bar"
order = 10

[[region]]
id = "controls-bar"
anchor = "bottom"
label_key = "player.region.controls-bar"
order = 10

[[region]]
id = "playlist"
anchor = "right"
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
