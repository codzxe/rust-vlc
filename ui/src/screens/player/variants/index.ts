// Card 4.F2 owns ui/src/screens/player/variants/** (Miniplayer + Music
// mode; variant model, variant-scoped layout data, switching keymap seed,
// persistence). Consumes C1's shell/layout/keymap + C2's controls seam.

export {
  PLAYER_VARIANTS,
  VARIANT_STORAGE_KEY,
  clearPersistedVariant,
  isPlayerVariant,
  loadPersistedVariant,
  parsePlayerVariant,
  persistVariant,
  type PlayerVariant,
} from "./variant";
export {
  MINIPLAYER_LAYOUT,
  MINIPLAYER_LAYOUT_TOML,
  MUSIC_LAYOUT,
  MUSIC_LAYOUT_TOML,
  PLAYER_VARIANT_LAYOUTS,
  resolveVariantLayout,
} from "./variantLayout";
export {
  PLAYER_SHELL_KEYMAP,
  VARIANT_KEYMAP,
  VARIANT_KEYMAP_TOML,
  mergeKeymaps,
} from "./variantKeymap";
export { MiniPlayer } from "./MiniPlayer";
export { MusicMode } from "./MusicMode";
