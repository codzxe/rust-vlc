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
import { SETTING_GLOBAL_ACCENT_COLOR } from "./schema";
import { SETTING_GLOBAL_SHOW_DIALOGS } from "./schema";
import { SETTING_GLOBAL_DEVELOPER_MODE } from "./schema";
import { SETTING_GLOBAL_SAVE_LOG } from "./schema";
import { SETTING_GLOBAL_LOG_FILE } from "./schema";
import { SETTING_GLOBAL_LOG_DETAILS } from "./schema";
import { SETTING_GLOBAL_SHOW_MINOR_WARNINGS } from "./schema";
import { SETTING_GLOBAL_EXTRA_CLI_OPTIONS } from "./schema";
import { SETTING_GLOBAL_LOG_FORMAT } from "./schema";
import { SETTING_GLOBAL_COLLECT_PLAYBACK_STATISTICS } from "./schema";
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
import { SETTING_PRIVACY_SAVE_RECENT_ITEMS } from "./schema";
import { SETTING_PRIVACY_REMEMBER_PLAYBACK_POSITION } from "./schema";
import { SETTING_PRIVACY_NEVER_REMEMBER_MATCHING } from "./schema";
import { SETTING_PRIVACY_FETCH_METADATA_ONLINE } from "./schema";
import { SETTING_PRIVACY_SUBMIT_LASTFM } from "./schema";
import { SETTING_PRIVACY_LASTFM_USERNAME } from "./schema";
import { SETTING_PRIVACY_LASTFM_PASSWORD } from "./schema";
import { SETTING_PRIVACY_LASTFM_SUBMISSION_URL } from "./schema";
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
import { SETTING_PLAYBACK_PREFERRED_VIDEO_QUALITY } from "./schema";
import { SETTING_PLAYBACK_MENU_LANGUAGE } from "./schema";
import { SETTING_PLAYBACK_DEFAULT_AUDIO_TRACK } from "./schema";
import { SETTING_PLAYBACK_DEFAULT_SUBTITLE_TRACK } from "./schema";
import { SETTING_PLAYBACK_START_PAUSED } from "./schema";
import { SETTING_PLAYBACK_DEFAULT_RATE } from "./schema";
import { SETTING_PLAYBACK_RESUME } from "./schema";
import { SETTING_PLAYBACK_PAUSE_ON_AUDIO_FOCUS } from "./schema";
import { SETTING_PLAYBACK_REPEAT_MODE } from "./schema";
import { SETTING_PLAYBACK_SHUFFLE } from "./schema";
import { SETTING_PLAYBACK_STOP_AFTER_EACH_ITEM } from "./schema";
import { SETTING_PLAYBACK_FINISH_ACTION } from "./schema";
import { SETTING_PLAYBACK_SEEK_PRECISELY } from "./schema";
import { SETTING_VIDEO_ENABLE } from "./schema";
import { SETTING_VIDEO_START_FULLSCREEN } from "./schema";
import { SETTING_VIDEO_DISABLE_SCREENSAVER } from "./schema";
import { SETTING_VIDEO_WALLPAPER } from "./schema";
import { SETTING_VIDEO_HUE } from "./schema";
import { SETTING_VIDEO_BRIGHTNESS } from "./schema";
import { SETTING_VIDEO_CONTRAST } from "./schema";
import { SETTING_VIDEO_SATURATION } from "./schema";
import { SETTING_VIDEO_GAMMA } from "./schema";
import { SETTING_VIDEO_DEINTERLACE } from "./schema";
import { SETTING_VIDEO_DEINTERLACE_METHOD } from "./schema";
import { SETTING_VIDEO_ASPECT_RATIO } from "./schema";
import { SETTING_VIDEO_CROP } from "./schema";
import { SETTING_AUDIO_ENABLE } from "./schema";
import { SETTING_AUDIO_SPEAKER_SETUP } from "./schema";
import { SETTING_AUDIO_MAXIMUM_VOLUME } from "./schema";
import { SETTING_AUDIO_STARTUP_VOLUME } from "./schema";
import { SETTING_AUDIO_VOLUME_STEP } from "./schema";
import { SETTING_AUDIO_NORMALIZE_LOUDNESS } from "./schema";
import { SETTING_AUDIO_TARGET_LEVEL } from "./schema";
import { SETTING_AUDIO_NIGHT_MODE } from "./schema";
import { SETTING_AUDIO_REPLAY_GAIN } from "./schema";
import { SETTING_AUDIO_REPLAY_GAIN_PREAMP } from "./schema";
import { SETTING_PERFORMANCE_CACHE_DEMUX_MIB } from "./schema";
import { SETTING_PERFORMANCE_CACHE_NETWORK_MIB } from "./schema";
import { SETTING_PERFORMANCE_MAX_OPEN_SESSIONS } from "./schema";
import { SETTING_PERFORMANCE_HW_DECODE } from "./schema";
import { SETTING_PERFORMANCE_VIDEO_POST_PROCESSING } from "./schema";
import { SETTING_PERFORMANCE_H264_LOOP_FILTER } from "./schema";
import { SETTING_PERFORMANCE_DECODING_THREADS } from "./schema";
import { SETTING_PERFORMANCE_DROP_FRAMES } from "./schema";
import { SETTING_PERFORMANCE_REALTIME_PRIORITY } from "./schema";
import { SETTING_PERFORMANCE_PROCESS_PRIORITY } from "./schema";
import { SETTING_PERFORMANCE_ENCODER } from "./schema";
import { SETTING_PERFORMANCE_DECODER } from "./schema";
import { SETTING_PERFORMANCE_CLOCK_SYNCHRONISATION } from "./schema";
import { SETTING_PERFORMANCE_CLOCK_JITTER } from "./schema";
import { SETTING_PERFORMANCE_CLOCK_REFERENCE_AVERAGING } from "./schema";
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
        section("appearance", "settings.section.global.appearance", [
          SETTING_GLOBAL_ACCENT_COLOR,
        ]),
        section("startup", "settings.section.global.startup", [
          SETTING_GLOBAL_SHOW_DIALOGS,
        ]),
        section("diagnostics", "settings.section.global.diagnostics", [
          SETTING_GLOBAL_DEVELOPER_MODE, SETTING_GLOBAL_SAVE_LOG, SETTING_GLOBAL_LOG_FILE, SETTING_GLOBAL_LOG_DETAILS, SETTING_GLOBAL_SHOW_MINOR_WARNINGS, SETTING_GLOBAL_EXTRA_CLI_OPTIONS, SETTING_GLOBAL_LOG_FORMAT, SETTING_GLOBAL_COLLECT_PLAYBACK_STATISTICS,
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
        section("history", "settings.section.privacy.history", [
          SETTING_PRIVACY_SAVE_RECENT_ITEMS, SETTING_PRIVACY_REMEMBER_PLAYBACK_POSITION, SETTING_PRIVACY_NEVER_REMEMBER_MATCHING,
        ]),
        section("network", "settings.section.privacy.network", [
          SETTING_PRIVACY_FETCH_METADATA_ONLINE, SETTING_PRIVACY_SUBMIT_LASTFM, SETTING_PRIVACY_LASTFM_USERNAME, SETTING_PRIVACY_LASTFM_PASSWORD, SETTING_PRIVACY_LASTFM_SUBMISSION_URL,
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
          SETTING_PLAYBACK_PREFERRED_AUDIO_LANGUAGES, SETTING_PLAYBACK_PREFERRED_SUBTITLE_LANGUAGES, SETTING_PLAYBACK_SUBTITLE_MODE, SETTING_PLAYBACK_HW_DECODE_POLICY, SETTING_PLAYBACK_PREFERRED_VIDEO_QUALITY, SETTING_PLAYBACK_MENU_LANGUAGE, SETTING_PLAYBACK_DEFAULT_AUDIO_TRACK, SETTING_PLAYBACK_DEFAULT_SUBTITLE_TRACK,
        ]),
        section("playback", "settings.section.playback.playback", [
          SETTING_PLAYBACK_START_PAUSED, SETTING_PLAYBACK_DEFAULT_RATE,
        ]),
        section("startup", "settings.section.playback.startup", [
          SETTING_PLAYBACK_RESUME, SETTING_PLAYBACK_PAUSE_ON_AUDIO_FOCUS,
        ]),
        section("order", "settings.section.playback.order", [
          SETTING_PLAYBACK_REPEAT_MODE, SETTING_PLAYBACK_SHUFFLE, SETTING_PLAYBACK_STOP_AFTER_EACH_ITEM, SETTING_PLAYBACK_FINISH_ACTION,
        ]),
        section("seeking", "settings.section.playback.seeking", [
          SETTING_PLAYBACK_SEEK_PRECISELY,
        ]),
      ]}
    />
  );
}

/** Screen "video" — generated from schemas/settings.toml. */
function VideoScreen(): ReactElement {
  return (
    <SettingsSections
      area="video"
      sections={[
        section("output", "settings.section.video.output", [
          SETTING_VIDEO_ENABLE, SETTING_VIDEO_START_FULLSCREEN, SETTING_VIDEO_DISABLE_SCREENSAVER, SETTING_VIDEO_WALLPAPER,
        ]),
        section("picture", "settings.section.video.picture", [
          SETTING_VIDEO_HUE, SETTING_VIDEO_BRIGHTNESS, SETTING_VIDEO_CONTRAST, SETTING_VIDEO_SATURATION, SETTING_VIDEO_GAMMA,
        ]),
        section("deinterlacing", "settings.section.video.deinterlacing", [
          SETTING_VIDEO_DEINTERLACE, SETTING_VIDEO_DEINTERLACE_METHOD,
        ]),
        section("framing", "settings.section.video.framing", [
          SETTING_VIDEO_ASPECT_RATIO, SETTING_VIDEO_CROP,
        ]),
      ]}
    />
  );
}

/** Screen "audio" — generated from schemas/settings.toml. */
function AudioScreen(): ReactElement {
  return (
    <SettingsSections
      area="audio"
      sections={[
        section("output", "settings.section.audio.output", [
          SETTING_AUDIO_ENABLE, SETTING_AUDIO_SPEAKER_SETUP,
        ]),
        section("volume", "settings.section.audio.volume", [
          SETTING_AUDIO_MAXIMUM_VOLUME, SETTING_AUDIO_STARTUP_VOLUME, SETTING_AUDIO_VOLUME_STEP,
        ]),
        section("loudness", "settings.section.audio.loudness", [
          SETTING_AUDIO_NORMALIZE_LOUDNESS, SETTING_AUDIO_TARGET_LEVEL, SETTING_AUDIO_NIGHT_MODE, SETTING_AUDIO_REPLAY_GAIN, SETTING_AUDIO_REPLAY_GAIN_PREAMP,
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
        section("decoding", "settings.section.performance.decoding", [
          SETTING_PERFORMANCE_HW_DECODE, SETTING_PERFORMANCE_VIDEO_POST_PROCESSING, SETTING_PERFORMANCE_H264_LOOP_FILTER, SETTING_PERFORMANCE_DECODING_THREADS,
        ]),
        section("smooth-playback", "settings.section.performance.smooth-playback", [
          SETTING_PERFORMANCE_DROP_FRAMES, SETTING_PERFORMANCE_REALTIME_PRIORITY, SETTING_PERFORMANCE_PROCESS_PRIORITY,
        ]),
        section("developer", "settings.section.performance.developer", [
          SETTING_PERFORMANCE_ENCODER, SETTING_PERFORMANCE_DECODER, SETTING_PERFORMANCE_CLOCK_SYNCHRONISATION, SETTING_PERFORMANCE_CLOCK_JITTER, SETTING_PERFORMANCE_CLOCK_REFERENCE_AVERAGING,
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

export const GENERATED_AREA_IDS = ["global", "interface", "privacy", "controls", "playback", "video", "audio", "performance", "recording", "extensions"] as const;

export type SettingsAreaId = (typeof GENERATED_AREA_IDS)[number];

export const GENERATED_AREA_TITLES: Record<SettingsAreaId, MessageKey> = {
  global: msgid("settings.area.global"),
  interface: msgid("settings.area.interface"),
  privacy: msgid("settings.area.privacy"),
  controls: msgid("settings.area.controls"),
  playback: msgid("settings.area.playback"),
  video: msgid("settings.area.video"),
  audio: msgid("settings.area.audio"),
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
  video: VideoScreen,
  audio: AudioScreen,
  performance: PerformanceScreen,
  recording: RecordingScreen,
  extensions: ExtensionsScreen,
};
