// Card 4.D1 — /playlist route body (doc 08 §1.6 screen inventory). The same
// PlaylistPanel the player shell embeds in its `playlist` region, promoted to
// a page with a route heading.
//
// Solstice v2 (2026-11-09, brief §1): this is an APP-tier WORK SURFACE, not a
// floating note — while mounted the screen veils the sunset to "calm"
// (html[data-backdrop="calm"], removed on unmount), the page anchors to the
// shared max-width column (min(1200px, 92vw)) with the heading and the queue
// panel on ONE left edge, and the panel fills the remaining viewport height
// so the queue list owns the vertical space (the page itself never clips).

import { useEffect } from "react";
import { useMsg } from "../../i18n/messages";
import { PlaylistPanel } from "./PlaylistPanel";
import { PlaylistProvider } from "./PlaylistProvider";
import "./playlist.css";

export function PlaylistScreen() {
  const m = useMsg();

  // APP tier: the sunset becomes a quiet warm wash behind the anchored
  // column. Dropped for exactly as long as this screen is mounted.
  useEffect(() => {
    const root = document.documentElement;
    root.dataset.backdrop = "calm";
    return () => {
      delete root.dataset.backdrop;
    };
  }, []);

  return (
    <main className="lumen-playlist-page" data-screen="playlist">
      {/* Page heading row — same left edge as the panel below (the column
          owns the alignment; no extra offsets). The panel hides its own h2
          here (headingless) so the page never stacks two "Playlist"
          headings; the count chip + queue toolbar stay on the panel. */}
      <h1 className="lumen-playlist-page__title">{m("nav.playlist")}</h1>
      {/* Seed entries stay empty here — real M3U data arrives through the
          app shell's `playlist.load` wiring (handoff: card 4.H1). */}
      <PlaylistProvider>
        <PlaylistPanel className="lumen-playlist-page__panel" headingless />
      </PlaylistProvider>
    </main>
  );
}
