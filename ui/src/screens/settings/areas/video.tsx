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
// registry (areas/index.tsx) composes <VideoArea/> + <CodecsFold/> (w26.i14:
// an ACTUAL in-flow disclosure, the global.svg Frame 265 grammar — the flat
// append scrolled 270px past the frame's end while the registry called it a
// “fold”; h09 defect #5).
//
// w26.i14 (h09 defect #2): the frame hangs a ⓘ circle on 8 rows — the four
// Output toggles, the two Deinterlacing dropdowns and the two Framing
// dropdowns; the five Picture sliders and the Reset action carry none
// (circle extraction: cy 130/172/214/256/670/716/821/867). This module
// hand-renders its sections (the PrivacyArea precedent) to pass per-row
// `hint` props through SettingField, with this lane's hint copy
// (ui/src/i18n/keys/w26.i14.ts). The screen stays schema-driven.
//
// `Settings - video-1` is a byte-duplicate export (md5 equal) — no second
// state exists.

import { useState } from "react";
import type { ReactElement } from "react";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as fixerKeys } from "../../../i18n/keys/w26.gfixB";
import { keys as i14Keys } from "../../../i18n/keys/w26.i14";
import { Icon } from "../../../components/icon";
import type { SettingEntry } from "../generated/schema";
import { schemaSectionsOf } from "../SettingsSections";
import { SettingField } from "../SettingField";
import { useSettingsSearch } from "../search";
import { SETTING_VIDEO_BRIGHTNESS, SETTING_VIDEO_CONTRAST, SETTING_VIDEO_GAMMA, SETTING_VIDEO_HUE, SETTING_VIDEO_SATURATION } from "../generated/schema";
import { useSettings } from "../SettingsProvider";
import { CodecsArea } from "./codecs";

type FixerKey = keyof typeof fixerKeys;
type I14Key = keyof typeof i14Keys;
/** Pre-registration seam (see w26.gfixB.ts). */
const laneCatalog: Readonly<Record<string, string>> = fixerKeys;

/** useAreaMsg — useMsg + this lane's pre-registration keys. */
function useAreaMsg() {
  const m = useMsg();
  return (key: MessageKey | FixerKey | I14Key): string => m(key as MessageKey) ?? laneCatalog[key];
}

/** The ⓘ hint msgid cast through the registry seam (the laneMsg precedent). */
const hintMsg = (key: I14Key): MessageKey => key as string as MessageKey;

/** ⓘ hint msgids per schema row (w26.i14, h09 #2 — the frame's 8 circles;
 * the Picture sliders and the Reset action carry none, reproduced honestly). */
const VIDEO_HINTS: Readonly<Record<string, MessageKey>> = {
  "video.enable": hintMsg("settings.hint.video.enable"),
  "video.start-fullscreen": hintMsg("settings.hint.video.start-fullscreen"),
  "video.disable-screensaver": hintMsg("settings.hint.video.disable-screensaver"),
  "video.wallpaper": hintMsg("settings.hint.video.wallpaper"),
  "video.deinterlace": hintMsg("settings.hint.video.deinterlace"),
  "video.deinterlace-method": hintMsg("settings.hint.video.deinterlace-method"),
  "video.aspect-ratio": hintMsg("settings.hint.video.aspect-ratio"),
  "video.crop": hintMsg("settings.hint.video.crop"),
};

/** The hand row's search terms (SettingsSections' SettingRowNode contract). */
const RESET_PICTURE_KEYWORDS = ["Reset picture", "Reset"] as const;

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
 * 100×32 rx6 #191919 right-anchored @x924 — the frame carries NO ⓘ circle
 * here). A real behavior: resets the five picture keys to their schema
 * defaults through the settings store — never a dead control.
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
 * sections from the schema seed + the Reset picture action row, hand-rendered
 * with per-row ⓘ hints (h09 #2).
 */
export function VideoArea(): ReactElement {
  const m = useAreaMsg();
  const { query, matches } = useSettingsSearch();
  const needle = query.trim().toLowerCase();
  const resetRowMatches =
    needle === "" || RESET_PICTURE_KEYWORDS.some((keyword) => keyword.toLowerCase().includes(needle));

  return (
    <div className="settings-sections" data-settings-area="video">
      {schemaSectionsOf("video").map((section) => {
        // Pure-schema sections (the Reset picture hand row is rendered by
        // this module, not spliced into the section data): narrow the row
        // union so the schema-entry fields type-check below.
        const entries = section.entries
          .filter((row): row is SettingEntry => "key" in row)
          .filter((entry) => entry.featureExposed && matches(entry));
        // The Reset picture action row (frame C5 y551) rides the Picture
        // section's own visibility while a query is live (its keywords are
        // its filter) and closes the section in frame order.
        const showResetRow = section.id === "picture" && resetRowMatches;
        if (entries.length === 0 && !showResetRow) return null;
        return (
          <section key={section.id} className="settings-section" data-section={section.id}>
            <h2 className="settings-section__title">{m(section.title as MessageKey)}</h2>
            {entries.map((entry) => (
              <SettingField key={entry.key} entry={entry} hint={VIDEO_HINTS[entry.key]} />
            ))}
            {showResetRow ? (
              <div data-setting-row="reset-picture">
                <ResetPictureRow />
              </div>
            ) : null}
          </section>
        );
      })}
    </div>
  );
}

/**
 * The codecs→video merge as the IN-FLOW disclosure the registry's “fold”
 * naming promised (w26.i14, h09 defect #5): the global.svg Frame 265 grammar
 * (settings-advanced bar — 784×38 r2, label + ›/∨ chevron), collapsed by
 * default so the video column ends at the frame's last row instead of
 * scrolling 270px past it. The rows below the bar keep their own section
 * headers (Decoding / Cache) once opened.
 */
export function CodecsFold(): ReactElement {
  const m = useMsg();
  const [open, setOpen] = useState(false);
  return (
    <div className="settings-advanced" data-settings-fold="codecs">
      <button
        type="button"
        className="settings-advanced__bar"
        aria-expanded={open}
        aria-controls="settings-codecs-rows"
        onClick={() => setOpen((value) => !value)}
      >
        <span className="settings-advanced__label">{m("settings.advanced-options")}</span>
        <Icon
          name={open ? "chevron-down" : "chevron-right"}
          size={14}
          className="settings-advanced__chevron"
        />
      </button>
      {open ? (
        <div className="settings-advanced__rows" id="settings-codecs-rows">
          <CodecsArea />
        </div>
      ) : null}
    </div>
  );
}
