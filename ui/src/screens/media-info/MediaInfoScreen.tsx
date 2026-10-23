// Card 4.D2 — /media-info route body: the MediaInfoPanel under a page
// heading (doc 08 §1.6 screen inventory). Thin screen per §4.1: content is
// the shared panel (also mounted in the player shell's media-info region).

import { useMsg } from "../../i18n/messages";
import { MediaInfoPanel } from "./MediaInfoPanel";
import { MediaInfoProvider } from "./MediaInfoProvider";

export function MediaInfoScreen() {
  const m = useMsg();
  return (
    <main className="lumen-mi-screen" data-screen="media-info">
      <h1 className="lumen-mi-screen__title">{m("screen.media-info")}</h1>
      <MediaInfoProvider>
        <MediaInfoPanel className="lumen-mi-screen__panel" />
      </MediaInfoProvider>
    </main>
  );
}
