// Card 4.D2 — /media-info route body: the MediaInfoPanel under a page
// heading (doc 08 §1.6 screen inventory). Thin screen per §4.1: content is
// the shared panel (also mounted in the player shell's media-info region).
//
// W26 Figma language (w26.b13): a top-anchored working-screen column
// (min(680px, 100%), Settings content-column grammar) on the near-black
// surface; the section cards carry the surface. The Solstice calm-backdrop
// veil is retired with the v2 aesthetic (W25 §0) — the screen paints
// tokens only and never touches documentElement while mounting.

import { useMsg } from "../../i18n/messages";
import { MediaInfoPanel } from "./MediaInfoPanel";
import { MediaInfoProvider } from "./MediaInfoProvider";
import "./media-info.css";

export function MediaInfoScreen() {
  const m = useMsg();

  return (
    <main className="lumen-mi-screen" data-screen="media-info">
      <h1 className="lumen-mi-screen__title">{m("screen.media-info")}</h1>
      <MediaInfoProvider>
        {/* showTitle=false — the route's h1 above is the one heading; the
         * panel title is the player side-region's header, not the page's. */}
        <MediaInfoPanel className="lumen-mi-screen__panel" showTitle={false} />
      </MediaInfoProvider>
    </main>
  );
}
