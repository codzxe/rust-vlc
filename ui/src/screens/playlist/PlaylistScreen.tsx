// Card 4.D1 — /playlist route body (doc 08 §1.6 screen inventory). The same
// PlaylistPanel the player shell embeds in its `playlist` region, promoted to
// a page with a route heading.

import { useMsg } from "../../i18n/messages";
import { PlaylistPanel } from "./PlaylistPanel";
import { PlaylistProvider } from "./PlaylistProvider";

export function PlaylistScreen() {
  const m = useMsg();
  return (
    <main className="lumen-playlist-page">
      <h1 className="lumen-playlist-page__title">{m("screen.playlist")}</h1>
      {/* Seed entries stay empty here — real M3U data arrives through the
          app shell's `playlist.load` wiring (handoff: card 4.H1). */}
      <PlaylistProvider>
        <PlaylistPanel className="lumen-playlist-page__panel" />
      </PlaylistProvider>
    </main>
  );
}
