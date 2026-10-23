// Settings area: Video — card 4.E3, doc 07 §5.4 screen "video".
//
// CONFIG-NEVER-INVENTS AUDIT for this screen (the honest result):
//   • deinterlace (doc 07 §5.4 seed: video.deinterlace, enum auto/on/off,
//     gated): NO entry exists in schemas/settings.toml → rendered as
//     NOTHING, never a disabled dead control that looks alive. Handoff note
//     in ui/NEXT-STEPS.md for the schema owner.
//   • display selection: neither a schema entry NOR a display-enumeration
//     command exists in the command catalog (unlike audio.list-devices) —
//     there is no honest data source, so nothing is rendered and no
//     transport call is made. Inventing monitor options would violate MP §5.
//   • default-aspect-ratio / default-crop / frame-drop-policy (§5.4 seed):
//     same — no schema entries yet → nothing + handoff note.
// When the schema entries land, E1's generated SettingsSections renders them
// (feature gates included); this module only carries hand-authored surface.

import type { ReactElement } from "react";
import { useMsg } from "../../../i18n/messages";

/** Video settings area — currently an honest empty state, zero controls. */
export function VideoArea(): ReactElement {
  const m = useMsg();
  return (
    <div className="settings-sections" data-settings-area="video" data-testid="video-area">
      <p className="settings-screen__empty" data-testid="settings-area-empty" role="note">
        {m("settings.area.empty")}
      </p>
      {/* deinterlace / display selection / aspect / crop / frame-drop:
          absent from the schema → deliberately nothing (never dead controls). */}
    </div>
  );
}
