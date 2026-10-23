// Card 4.F2 — player variant model (doc 08 §1.6 routes `miniplayer`,
// `player.music`; §4.x "UI-owned state" explicitly lists "layout mode
// (music/hide-UI/mini)").
//
// Three layouts exist:
//   * "full"      — C1's shell anatomy (top bar / video canvas / control bar /
//                   library + playlist panels),
//   * "miniplayer"— the compact always-on-top-style card (doc 08 §1.4
//                   Miniplayer geometry: card 391×339 rx14, thumbnail
//                   373×209 rx5, seek 375×16 rx6 + 8×12 knob),
//   * "music"     — canvas de-emphasized: large album-art placeholder +
//                   transport + prominent playlist (doc 08 §1.4 "Album art
//                   (music) 737×737 rx20", "Music-mode seek", §1.6
//                   `player.music`).
//
// The variant is UI-owned state (doc 08), persisted client-side via the same
// localStorage pattern as the settings persistence seam
// (screens/settings/persistence.ts). [PROPOSAL]: keymaps/default.toml
// currently defines NO variant-toggle binding (verified — grep for
// miniplayer/music finds nothing) and the command catalog has no
// layout-switch command; the switching seam is therefore keymap-data-driven
// via variants/variantKeymap.ts and recorded in ui/NEXT-STEPS.md for the
// keymap loader card to reconcile.

/** The player's three layouts (doc 08 §1.6; "full" is C1's default). */
export type PlayerVariant = "full" | "miniplayer" | "music";

/** All variants in design-matrix order. */
export const PLAYER_VARIANTS: readonly PlayerVariant[] = ["full", "miniplayer", "music"];

/** Type guard + parser — unknown values are rejected, never guessed. */
export function isPlayerVariant(value: unknown): value is PlayerVariant {
  return typeof value === "string" && (PLAYER_VARIANTS as readonly string[]).includes(value);
}

/** Parse a persisted/injected variant; null = absent or unknown (honest). */
export function parsePlayerVariant(raw: string | null | undefined): PlayerVariant | null {
  return isPlayerVariant(raw) ? raw : null;
}

/** localStorage key (mirrors the settings persistence seam's naming). */
export const VARIANT_STORAGE_KEY = "lumen.player.variant";

/** Load the persisted variant; null when absent, unknown, or storage unavailable. */
export function loadPersistedVariant(): PlayerVariant | null {
  try {
    return parsePlayerVariant(globalThis.localStorage?.getItem(VARIANT_STORAGE_KEY));
  } catch {
    // Storage unavailable (sandboxed host) — session-only variant, honest.
    return null;
  }
}

/** Persist the variant; failures degrade to session-only (never throw). */
export function persistVariant(variant: PlayerVariant): void {
  try {
    globalThis.localStorage?.setItem(VARIANT_STORAGE_KEY, variant);
  } catch {
    // Unavailable host — same honest degradation as loadPersistedVariant.
  }
}

/** Clear persisted variant (tests / reset-to-default paths). */
export function clearPersistedVariant(): void {
  try {
    globalThis.localStorage?.removeItem(VARIANT_STORAGE_KEY);
  } catch {
    // Unavailable host — nothing persisted, nothing to clear.
  }
}
