// FX6-A05 lane keys (wave FX6, FIX-PLAN §3 F1.2 — the webview half of the
// frame lane): the frame-counter readout's labels. Genuinely new keys —
// no "player.frames.*" family exists in the central catalog (grep'd first;
// reuse rule, CONTRACT §4.5). The Lead registers these into
// ui/src/i18n/messages.ts after the wave merge; until then the readout
// resolves them seam-first (central catalog → this file) via the
// useFrameMsg resolver in MediaSurface.tsx — the w26.a10 / w28.window
// pre-registration pattern — so the copy renders identically before and
// after registration.
export const keys = {
  // The readout's three counters. The VALUES are live numbers rendered
  // beside these labels (never copy): presented + skipped are THIS
  // webview's own counts at the draw call; the last frame id is the
  // engine-assigned id riding the LUF1 record header. The engine's
  // produced/shipped/dropped-at-mailbox counters do not ride the
  // transport in this wave (the FX6-A03 seam) — nothing here fabricates
  // them as if they were webview observables.
  "player.frames.presented": "Frames presented",
  "player.frames.last-id": "Last frame id",
  "player.frames.skipped": "Skipped records",
} as const;
