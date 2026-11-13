// Settings area: Playback — "Playback Preferences" (Settings - playback.svg,
// analysis tools-settings.md §C4; w26.g11/gfix-B composition).
//
// w26.gfix-B COMPOSITION (frame truth, top → bottom; the schema seed carries
// every schema row — the generated section data stays the source of truth):
//   Startup (y83) — Continue where you left off → "Always" · Start playing
//     as soon as a file opens → ON (the inverted start-paused key) · Default
//     playback speed → slider (fill 139/320, 6 ticks) · Pause when another
//     app uses audio → ON.
//   Tracks (y339) — Preferred video quality → "Best available" · Preferred
//     menu language → "English" · Default audio track → "1" · Default
//     subtitle track → "1".
//   Order (y582) — Repeat → "Off" · Shuffle → ON · Stop after each item →
//     ON · When playback finishes → "Stop".
//   Seeking (y817) — Seek precisely → ON.
//
// w26.i14 (h09 defect #2): the frame hangs a ⓘ circle on 12 of the 13 rows —
// every row EXCEPT the Default-playback-speed slider (circle extraction: cy
// 132/176/285/388/434/480/526/631/675/717/761/864). The area hand-renders its
// sections (the PrivacyArea precedent) to pass per-row `hint` props through
// SettingField, with this lane's hint copy (ui/src/i18n/keys/w26.i14.ts);
// the screen stays schema-driven: sections/entries/order come from the seed.
//
// `Settings - playback-1` is a duplicate export of the same artboard (17
// rect y-deltas of +1/+2px; g11) — no second state exists.
//
// w26.k13 (j13 ruling 1): the section loop rides the shared areas layer
// (schemaSectionViews + the hiddenKeys composition option). w26.k12
// (j12 P2-2): no search filter — the shell swaps in cross-area results
// while a query is live, so an area-side filter could never execute; the
// hidden rows below stay REAL, LIVE settings that the cross-area search
// results and the onboarding wizard still reach.

import type { ReactElement } from "react";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as i14Keys } from "../../../i18n/keys/w26.i14";
import { SettingField } from "../SettingField";
import { schemaSectionViews } from "./shared";

type I14Key = keyof typeof i14Keys;

/** The ⓘ hint msgid cast through the registry seam (the laneMsg precedent). */
const hintMsg = (key: I14Key): MessageKey => key as string as MessageKey;

/** ⓘ hint msgids per schema row (w26.i14, h09 #2 — 12 of the frame's 13
 * rows; the Default playback speed slider carries no circle, honestly). */
const PLAYBACK_HINTS: Readonly<Record<string, MessageKey>> = {
  "playback.resume": hintMsg("settings.hint.playback.resume"),
  "playback.start-paused": hintMsg("settings.hint.playback.start-paused"),
  "playback.pause-on-audio-focus": hintMsg("settings.hint.playback.pause-on-audio-focus"),
  "playback.preferred-video-quality": hintMsg("settings.hint.playback.preferred-video-quality"),
  "playback.menu-language": hintMsg("settings.hint.playback.menu-language"),
  "playback.default-audio-track": hintMsg("settings.hint.playback.default-audio-track"),
  "playback.default-subtitle-track": hintMsg("settings.hint.playback.default-subtitle-track"),
  "playback.repeat-mode": hintMsg("settings.hint.playback.repeat-mode"),
  "playback.shuffle": hintMsg("settings.hint.playback.shuffle"),
  "playback.stop-after-each-item": hintMsg("settings.hint.playback.stop-after-each-item"),
  "playback.finish-action": hintMsg("settings.hint.playback.finish-action"),
  "playback.seek-precisely": hintMsg("settings.hint.playback.seek-precisely"),
};

/**
 * Playback rows the frame's Tracks section does not carry (g11-P2-2): the
 * three language/mode rows stay REAL, LIVE settings (the onboarding wizard
 * writes them; the shell's cross-area search still finds them) — the
 * playback screen's own composition just hides them from the frame's 4-row
 * Tracks section.
 */
const PLAYBACK_HIDDEN_KEYS: readonly string[] = [
  "playback.preferred-audio-languages",
  "playback.preferred-subtitle-languages",
  "playback.subtitle-mode",
];

/**
 * Playback settings area (route /settings/playback; Media group) — the
 * frame's 4 sections (Startup · Tracks · Order · Seeking) from the schema
 * seed minus the non-frame Tracks rows, hand-rendered with per-row ⓘ hints
 * (h09 #2). The seed's own "Track languages" section hides entirely (all
 * three of its rows are PLAYBACK_HIDDEN_KEYS).
 */
export function PlaybackArea(): ReactElement {
  const m = useMsg();
  return (
    <div className="settings-sections" data-settings-area="playback">
      {schemaSectionViews("playback", { hiddenKeys: PLAYBACK_HIDDEN_KEYS }).map(
        ({ section, visible }) => {
          if (visible.length === 0) return null;
          return (
            <section key={section.id} className="settings-section" data-section={section.id}>
              <h2 className="settings-section__title">{m(section.title as MessageKey)}</h2>
              {visible.map((entry) => (
                <SettingField key={entry.key} entry={entry} hint={PLAYBACK_HINTS[entry.key]} />
              ))}
            </section>
          );
        },
      )}
    </div>
  );
}
