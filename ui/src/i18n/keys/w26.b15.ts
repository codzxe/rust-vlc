// w26.b15 lane keys (open-network-stream dialog) — the Open network
// stream.svg copy that has no existing catalog entry. Registered into
// ui/src/i18n/messages.ts by the Lead after the wave merge (contract §4.5);
// until then the screen resolves these via its local seam-first resolver
// (see network-stream/useNetworkStreamMsg.ts), so the copy renders
// identically before and after registration. All other dialog copy reuses
// the existing `stream.*` catalog keys plus `open.cancel` / `open.close`
// (grepped messages.ts first).
export const keys = {
  // URL section — the frame's helper line under the "Network URL" label.
  "stream.url.hint": "You can enter multiple links to load.",
  // Frame placeholder text (differs from the legacy stream.url.placeholder).
  "stream.url.example": "e.g., https://www.example.com/stream.avi",
  // Playlist Name (optional) section — frame text, two-tone label.
  "stream.playlist.label": "Playlist Name",
  "stream.playlist.optional": "(optional)",
  "stream.playlist.hint": "The playlist will be saved and displayed with this title.",
  "stream.playlist.placeholder": "e.g., Favorite TV Channels",
} as const;
