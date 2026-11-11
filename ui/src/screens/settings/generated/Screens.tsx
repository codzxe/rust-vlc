// GENERATED FILE — do not edit (card 4.E1, doc 07 §5.3 generator G1).
// One screen component per settings area, statically bound to the
// schema entries — adding/removing/reordering schema entries changes
// this file, and `npm run settings:check` fails until it is regenerated.
/* eslint-disable */

import type { ReactElement } from "react";
import type { MessageKey } from "../../../i18n/messages";
import { SettingsSections } from "../SettingsSections";
import type { GeneratedSection } from "../SettingsSections";
import type { SettingEntry } from "./schema";
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
import { SETTING_CONTROLS_SKIP_VERY_SHORT } from "./schema";
import { SETTING_CONTROLS_SKIP_SHORT } from "./schema";
import { SETTING_CONTROLS_SKIP_MEDIUM } from "./schema";
import { SETTING_CONTROLS_SKIP_LONG } from "./schema";
import { SETTING_CONTROLS_MOUSE_SINGLE_CLICK } from "./schema";
import { SETTING_CONTROLS_MOUSE_DOUBLE_CLICK } from "./schema";
import { SETTING_CONTROLS_MOUSE_SCROLL_WHEEL } from "./schema";
import { SETTING_CONTROLS_MOUSE_HORIZONTAL_SCROLL } from "./schema";
import { SETTING_CONTROLS_MOUSE_REVERSE_SCROLL_DIRECTION } from "./schema";
import { SETTING_PLAYBACK_PREFERRED_AUDIO_LANGUAGES } from "./schema";
import { SETTING_PLAYBACK_PREFERRED_SUBTITLE_LANGUAGES } from "./schema";
import { SETTING_PLAYBACK_SUBTITLE_MODE } from "./schema";
import { SETTING_PLAYBACK_HW_DECODE_POLICY } from "./schema";
import { SETTING_PLAYBACK_START_PAUSED } from "./schema";
import { SETTING_PLAYBACK_DEFAULT_RATE } from "./schema";
import { SETTING_PERFORMANCE_CACHE_DEMUX_MIB } from "./schema";
import { SETTING_PERFORMANCE_CACHE_NETWORK_MIB } from "./schema";
import { SETTING_PERFORMANCE_MAX_OPEN_SESSIONS } from "./schema";
import { SETTING_RECORDING_DIRECTORY } from "./schema";
import { SETTING_RECORDING_FORMAT } from "./schema";
import { SETTING_EXTENSIONS_LOAD_ON_STARTUP } from "./schema";

const msgid = (id: string): MessageKey => id as MessageKey;

/** Section props for <SettingsSections> (see msgid above). */
const section = (id: string, title: string, entries: readonly SettingEntry[]): GeneratedSection => ({
  id,
  title: msgid(title),
  entries,
});

/** Screen "global" — generated from schemas/settings.toml. */
function GlobalScreen(): ReactElement {
  return (
    <SettingsSections
      area="global"
      sections={[
        section("language", "settings.section.global.language", [
          SETTING_GLOBAL_LANGUAGE,
        ]),
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
        section("appearance", "settings.section.interface.appearance", [
          SETTING_INTERFACE_THEME,
        ]),
        section("window", "settings.section.interface.window", [
          SETTING_INTERFACE_START_MAXIMIZED, SETTING_INTERFACE_REMEMBER_WINDOW_GEOMETRY, SETTING_INTERFACE_WINDOW_WIDTH, SETTING_INTERFACE_WINDOW_HEIGHT, SETTING_INTERFACE_WINDOW_MIN_WIDTH, SETTING_INTERFACE_WINDOW_MIN_HEIGHT,
        ]),
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
        section("data", "settings.section.privacy.data", [
          SETTING_PRIVACY_TELEMETRY, SETTING_PRIVACY_CRASH_REPORTS, SETTING_PRIVACY_USAGE_STATS, SETTING_PRIVACY_UPDATE_CHECKS,
        ]),
      ]}
    />
  );
}

/** Screen "controls" — generated from schemas/settings.toml. */
function ControlsScreen(): ReactElement {
  return (
    <SettingsSections
      area="controls"
      sections={[
        section("skip-amounts", "settings.section.controls.skip-amounts", [
          SETTING_CONTROLS_SKIP_VERY_SHORT, SETTING_CONTROLS_SKIP_SHORT, SETTING_CONTROLS_SKIP_MEDIUM, SETTING_CONTROLS_SKIP_LONG,
        ]),
        section("mouse", "settings.section.controls.mouse", [
          SETTING_CONTROLS_MOUSE_SINGLE_CLICK, SETTING_CONTROLS_MOUSE_DOUBLE_CLICK, SETTING_CONTROLS_MOUSE_SCROLL_WHEEL, SETTING_CONTROLS_MOUSE_HORIZONTAL_SCROLL, SETTING_CONTROLS_MOUSE_REVERSE_SCROLL_DIRECTION,
        ]),
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
        section("tracks", "settings.section.playback.tracks", [
          SETTING_PLAYBACK_PREFERRED_AUDIO_LANGUAGES, SETTING_PLAYBACK_PREFERRED_SUBTITLE_LANGUAGES, SETTING_PLAYBACK_SUBTITLE_MODE, SETTING_PLAYBACK_HW_DECODE_POLICY,
        ]),
        section("playback", "settings.section.playback.playback", [
          SETTING_PLAYBACK_START_PAUSED, SETTING_PLAYBACK_DEFAULT_RATE,
        ]),
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
        section("resources", "settings.section.performance.resources", [
          SETTING_PERFORMANCE_CACHE_DEMUX_MIB, SETTING_PERFORMANCE_CACHE_NETWORK_MIB, SETTING_PERFORMANCE_MAX_OPEN_SESSIONS,
        ]),
      ]}
    />
  );
}

/** Screen "recording" — generated from schemas/settings.toml. */
function RecordingScreen(): ReactElement {
  return (
    <SettingsSections
      area="recording"
      sections={[
        section("recording", "settings.section.recording.recording", [
          SETTING_RECORDING_DIRECTORY, SETTING_RECORDING_FORMAT,
        ]),
      ]}
    />
  );
}

/** Screen "extensions" — generated from schemas/settings.toml. */
function ExtensionsScreen(): ReactElement {
  return (
    <SettingsSections
      area="extensions"
      sections={[
        section("add-ons", "settings.section.extensions.add-ons", [
          SETTING_EXTENSIONS_LOAD_ON_STARTUP,
        ]),
      ]}
    />
  );
}

export const GENERATED_AREA_IDS = ["global", "interface", "privacy", "controls", "playback", "performance", "recording", "extensions"] as const;

export type SettingsAreaId = (typeof GENERATED_AREA_IDS)[number];

export const GENERATED_AREA_TITLES: Record<SettingsAreaId, MessageKey> = {
  global: msgid("settings.area.global"),
  interface: msgid("settings.area.interface"),
  privacy: msgid("settings.area.privacy"),
  controls: msgid("settings.area.controls"),
  playback: msgid("settings.area.playback"),
  performance: msgid("settings.area.performance"),
  recording: msgid("settings.area.recording"),
  extensions: msgid("settings.area.extensions"),
};

export const GENERATED_SCREENS: Record<SettingsAreaId, () => ReactElement> = {
  global: GlobalScreen,
  interface: InterfaceScreen,
  privacy: PrivacyScreen,
  controls: ControlsScreen,
  playback: PlaybackScreen,
  performance: PerformanceScreen,
  recording: RecordingScreen,
  extensions: ExtensionsScreen,
};
