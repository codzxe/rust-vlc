// Settings area: Subtitles/OSD — card 4.E3, doc 07 §5.4 screen "subtitles".
//
// CONFIG-NEVER-INVENTS AUDIT for this screen (the honest result):
//   • subtitle language preference: the schema places it on the PLAYBACK
//     screen (playback.preferred-subtitle-languages, §5.4 seed) — E1's
//     generated PlaybackScreen already renders it. Duplicating it here would
//     define the option twice (doc 07 §5.2 D-rules); nothing is rendered.
//   • subtitle style options (font-size, color, outline-strength, position,
//     sync-step-ms, fallback-encoding per §5.4 seed): NO entries exist in
//     schemas/settings.toml → rendered as NOTHING, never dead controls.
//     Handoff note in ui/NEXT-STEPS.md for the schema owner.
//   • OSD behavior options: doc 07 §5.4 assigns controls.osd-duration-ms to
//     the CONTROLS screen, and no subtitles-screen OSD entry exists →
//     nothing here (the controls-area card owns that surface).

import type { ReactElement } from "react";
import { useMsg } from "../../../i18n/messages";

/** Subtitles/OSD settings area — currently an honest empty state, zero controls. */
export function SubtitlesArea(): ReactElement {
  const m = useMsg();
  return (
    <div className="settings-sections" data-settings-area="subtitles" data-testid="subtitles-area">
      <p className="settings-screen__empty" data-testid="settings-area-empty" role="note">
        {m("settings.area.empty")}
      </p>
      {/* language preference lives on the playback screen; style + OSD options
          have no schema entries yet → deliberately nothing (never dead controls). */}
    </div>
  );
}
