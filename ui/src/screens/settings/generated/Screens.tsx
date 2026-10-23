// GENERATED FILE — do not edit (card 4.E1, doc 07 §5.3 generator G1).
// One screen component per settings area, statically bound to the
// schema entries — adding/removing/reordering schema entries changes
// this file, and `npm run settings:check` fails until it is regenerated.
/* eslint-disable */

import type { ReactElement } from "react";
import type { MessageKey } from "../../../i18n/messages";
import { SettingsSections } from "../SettingsSections";
import { SETTING_GLOBAL_LANGUAGE } from "./schema";
import { SETTING_INTERFACE_THEME } from "./schema";
import { SETTING_INTERFACE_START_MAXIMIZED } from "./schema";
import { SETTING_INTERFACE_REMEMBER_WINDOW_GEOMETRY } from "./schema";
import { SETTING_INTERFACE_WINDOW_WIDTH } from "./schema";
import { SETTING_INTERFACE_WINDOW_HEIGHT } from "./schema";
import { SETTING_INTERFACE_WINDOW_MIN_WIDTH } from "./schema";
import { SETTING_INTERFACE_WINDOW_MIN_HEIGHT } from "./schema";
import { SETTING_PRIVACY_TELEMETRY } from "./schema";
import { SETTING_PRIVACY_CRASH_REPORTS } from "./schema";
import { SETTING_PRIVACY_USAGE_STATS } from "./schema";
import { SETTING_PRIVACY_UPDATE_CHECKS } from "./schema";
import { SETTING_PLAYBACK_PREFERRED_AUDIO_LANGUAGES } from "./schema";
import { SETTING_PLAYBACK_PREFERRED_SUBTITLE_LANGUAGES } from "./schema";
import { SETTING_PLAYBACK_SUBTITLE_MODE } from "./schema";
import { SETTING_PLAYBACK_HW_DECODE_POLICY } from "./schema";
import { SETTING_PLAYBACK_START_PAUSED } from "./schema";
import { SETTING_PLAYBACK_DEFAULT_RATE } from "./schema";
import { SETTING_PERFORMANCE_CACHE_DEMUX_MIB } from "./schema";
import { SETTING_PERFORMANCE_CACHE_NETWORK_MIB } from "./schema";
import { SETTING_PERFORMANCE_MAX_OPEN_SESSIONS } from "./schema";

/** Screen "global" — generated from schemas/settings.toml. */
function GlobalScreen(): ReactElement {
  return (
    <SettingsSections
      area="global"
      sections={[
        { id: "language", title: "settings.section.global.language", entries: [
          SETTING_GLOBAL_LANGUAGE,
        ] },
      ]}
    />
  );
}

/** Screen "interface" — generated from schemas/settings.toml. */
function InterfaceScreen(): ReactElement {
  return (
    <SettingsSections
      area="interface"
      sections={[
        { id: "appearance", title: "settings.section.interface.appearance", entries: [
          SETTING_INTERFACE_THEME,
        ] },
        { id: "window", title: "settings.section.interface.window", entries: [
          SETTING_INTERFACE_START_MAXIMIZED, SETTING_INTERFACE_REMEMBER_WINDOW_GEOMETRY, SETTING_INTERFACE_WINDOW_WIDTH, SETTING_INTERFACE_WINDOW_HEIGHT, SETTING_INTERFACE_WINDOW_MIN_WIDTH, SETTING_INTERFACE_WINDOW_MIN_HEIGHT,
        ] },
      ]}
    />
  );
}

/** Screen "privacy" — generated from schemas/settings.toml. */
function PrivacyScreen(): ReactElement {
  return (
    <SettingsSections
      area="privacy"
      sections={[
        { id: "data", title: "settings.section.privacy.data", entries: [
          SETTING_PRIVACY_TELEMETRY, SETTING_PRIVACY_CRASH_REPORTS, SETTING_PRIVACY_USAGE_STATS, SETTING_PRIVACY_UPDATE_CHECKS,
        ] },
      ]}
    />
  );
}

/** Screen "playback" — generated from schemas/settings.toml. */
function PlaybackScreen(): ReactElement {
  return (
    <SettingsSections
      area="playback"
      sections={[
        { id: "tracks", title: "settings.section.playback.tracks", entries: [
          SETTING_PLAYBACK_PREFERRED_AUDIO_LANGUAGES, SETTING_PLAYBACK_PREFERRED_SUBTITLE_LANGUAGES, SETTING_PLAYBACK_SUBTITLE_MODE, SETTING_PLAYBACK_HW_DECODE_POLICY,
        ] },
        { id: "playback", title: "settings.section.playback.playback", entries: [
          SETTING_PLAYBACK_START_PAUSED, SETTING_PLAYBACK_DEFAULT_RATE,
        ] },
      ]}
    />
  );
}

/** Screen "performance" — generated from schemas/settings.toml. */
function PerformanceScreen(): ReactElement {
  return (
    <SettingsSections
      area="performance"
      sections={[
        { id: "resources", title: "settings.section.performance.resources", entries: [
          SETTING_PERFORMANCE_CACHE_DEMUX_MIB, SETTING_PERFORMANCE_CACHE_NETWORK_MIB, SETTING_PERFORMANCE_MAX_OPEN_SESSIONS,
        ] },
      ]}
    />
  );
}

export const GENERATED_AREA_IDS = ["global", "interface", "privacy", "playback", "performance"] as const;

export type SettingsAreaId = (typeof GENERATED_AREA_IDS)[number];

export const GENERATED_AREA_TITLES: Record<SettingsAreaId, MessageKey> = {
  global: "settings.area.global",
  interface: "settings.area.interface",
  privacy: "settings.area.privacy",
  playback: "settings.area.playback",
  performance: "settings.area.performance",
};

export const GENERATED_SCREENS: Record<SettingsAreaId, () => ReactElement> = {
  global: GlobalScreen,
  interface: InterfaceScreen,
  privacy: PrivacyScreen,
  playback: PlaybackScreen,
  performance: PerformanceScreen,
};
