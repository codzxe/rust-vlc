// Settings area: Playlist & services discovery — card 4.E4, doc 07 §5.4
// screen "library".
//
// CONFIG-NEVER-INVENT AUDIT (the honest result):
//   • Service discovery toggles (discover-enable, upnp/mdl/podcast service
//     discovery…): NO entries exist in schemas/settings.toml → rendered as
//     NOTHING, never a disabled dead control that looks alive. Handoff note
//     in ui/NEXT-STEPS.md for the schema owner.
//   • product.toml [features] `playlists = false` (Phase 4) and no streaming
//     features exposed — playlist behavior options have neither schema keys
//     nor exposed features, so nothing is rendered here either.
// When the schema entries land, E1's generated SettingsSections renders them
// (feature gates included); this module carries no option list of its own.

import type { ReactElement } from "react";
import { useMsg } from "../../../i18n/messages";

/** Playlist & services discovery area — honest empty state, zero controls. */
export function PlaylistServicesArea(): ReactElement {
  const m = useMsg();
  return (
    <div className="settings-sections" data-settings-area="library" data-testid="playlist-services-area">
      <p className="settings-screen__empty" data-testid="settings-area-empty" role="note">
        {m("settings.area.empty")}
      </p>
      {/* service discovery / playlist behavior: absent from the schema and the
          feature exposure list → deliberately nothing (never dead controls). */}
    </div>
  );
}
