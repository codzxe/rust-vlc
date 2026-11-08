// Card 4.D2 — /media-info route body: the MediaInfoPanel under a page
// heading (doc 08 §1.6 screen inventory). Thin screen per §4.1: content is
// the shared panel (also mounted in the player shell's media-info region).
//
// Solstice v2 (2026-11-09, brief §1): APP-tier screen — the backdrop veils
// to "calm" while mounted (cleaned up on unmount) and the section stack
// anchors to the shared max-width column (min(1200px, 92vw)) so the panels
// read as a structured work surface on one grid.

import { useEffect } from "react";
import { useMsg } from "../../i18n/messages";
import { MediaInfoPanel } from "./MediaInfoPanel";
import { MediaInfoProvider } from "./MediaInfoProvider";

export function MediaInfoScreen() {
  const m = useMsg();

  // APP tier: calm warm wash behind the anchored column.
  useEffect(() => {
    const root = document.documentElement;
    root.dataset.backdrop = "calm";
    return () => {
      delete root.dataset.backdrop;
    };
  }, []);

  return (
    <main className="lumen-mi-screen" data-screen="media-info">
      <h1 className="lumen-mi-screen__title">{m("screen.media-info")}</h1>
      <MediaInfoProvider>
        <MediaInfoPanel className="lumen-mi-screen__panel" />
      </MediaInfoProvider>
    </main>
  );
}
