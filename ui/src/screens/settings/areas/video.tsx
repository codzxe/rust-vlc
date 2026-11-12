// Settings area: Video — "Video Defaults" (Settings - video.svg, analysis
// tools-settings.md §C5; w26.g12-P0-1 composition fix).
//
// w26.gfix-B COMPOSITION (frame truth, top → bottom; the schema seed carries
// every row — the generated section data stays the source of truth):
//   Output (y83) — Enable video ON · Start videos in fullscreen OFF · Stop
//     the screensaver during playback ON · Play video as desktop wallpaper
//     OFF (frame-verified toggles, 68×32 @x956).
//   Picture (y310) — Hue · Brightness · Contrast · Saturation · Gamma inline
//     sliders (320×16 rx6 tracks, 8×24 #AB6314 thumbs) + Reset picture →
//     Reset 100×32 rx6 #191919 @x924 (y551) — the ACTION row below, spliced
//     into the section by this module (no [[setting]] key can carry an
//     action; g12-P2-7).
//   Deinterlacing (y621) — Deinterlacing "Automatic" · Method "Automatic".
//   Framing (y772) — Aspect ratio "Automatic" · Crop "None".
// The codecs fold (codecs→video merge) renders BELOW this area — the
// registry (areas/index.tsx) composes <VideoArea/> + <CodecsArea/>.
//
// `Settings - video-1` is a byte-duplicate export (md5 equal) — no second
// state exists.

import type { ReactElement } from "react";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as fixerKeys } from "../../../i18n/keys/w26.gfixB";
import { SettingsSections } from "../SettingsSections";
import { schemaSectionsOf } from "../SettingsSections";
import { withRows } from "../SettingsSections";
import { SETTING_VIDEO_BRIGHTNESS, SETTING_VIDEO_CONTRAST, SETTING_VIDEO_GAMMA, SETTING_VIDEO_HUE, SETTING_VIDEO_SATURATION } from "../generated/schema";
import { useSettings } from "../SettingsProvider";

type FixerKey = keyof typeof fixerKeys;
/** Pre-registration seam (see w26.gfixB.ts). */
const laneCatalog: Readonly<Record<string, string>> = fixerKeys;

/** useAreaMsg — useMsg + this lane's pre-registration keys. */
function useAreaMsg() {
  const m = useMsg();
  return (key: MessageKey | FixerKey): string => m(key as MessageKey) ?? laneCatalog[key];
}

/** The five Picture keys the Reset action restores (schema order). */
const PICTURE_KEYS: readonly string[] = [
  SETTING_VIDEO_HUE.key,
  SETTING_VIDEO_BRIGHTNESS.key,
  SETTING_VIDEO_CONTRAST.key,
  SETTING_VIDEO_SATURATION.key,
  SETTING_VIDEO_GAMMA.key,
];

/**
 * The "Reset picture" action row (frame C5 y551: label left, Reset
 * 100×32 rx6 #191919 right-anchored @x924). A real behavior: resets the five
 * picture keys to their schema defaults through the settings store — never
 * a dead control.
 */
export function ResetPictureRow(): ReactElement {
  const m = useAreaMsg();
  const { resetField } = useSettings();
  return (
    <div className="settings-field" data-kind="action" data-video-action="reset-picture">
      <div className="settings-field__row">
        <div className="settings-field__lead">
          <span className="settings-field__label">{m("settings.video.reset-picture")}</span>
        </div>
        <div className="settings-field__control">
          <button
            type="button"
            className="settings-action"
            onClick={() => {
              for (const key of PICTURE_KEYS) resetField(key);
            }}
          >
            {m("settings.video.reset")}
          </button>
        </div>
      </div>
    </div>
  );
}

/**
 * Video settings area (route /settings/video; Media group) — the frame's 4
 * sections from the schema seed + the Reset picture action row.
 */
export function VideoArea(): ReactElement {
  return (
    <SettingsSections
      area="video"
      sections={withRows(schemaSectionsOf("video"), "picture", [
        {
          row: {
            rowId: "reset-picture",
            keywords: ["Reset picture", "Reset"],
            node: <ResetPictureRow />,
          },
        },
      ])}
    />
  );
}
