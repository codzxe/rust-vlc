// GENERATED FILE — do not edit (card 4.E1, doc 07 §5.3 generator G1).
// Source of truth: schemas/settings.toml (card 1.B2) with default_ref
// targets and feature exposure resolved from product.toml.
// Regenerate: npm run settings:generate    Drift guard: npm run settings:check
/* eslint-disable */

export type SettingScalarValue = string | number | boolean;
export type SettingValue = SettingScalarValue | readonly string[];
export type SettingValueType =
  | "bool"
  | "int"
  | "float"
  | "string"
  | "enum"
  | "list<string>"
  | "duration-ms"
  | "path"
  | "url"
  | "percent";

export interface SettingEnumOption {
  readonly value: string;
  readonly msgid: string;
}

export interface SettingEntry {
  /** Dotted canonical id, e.g. "interface.theme" (doc 07 §5.2). */
  readonly key: string;
  readonly type: SettingValueType;
  /** Label msgid — rendered via useMsg, never literal copy. */
  readonly description: string;
  /** default_ref resolved through product.toml (D1). */
  readonly defaultValue: SettingValue;
  /** product.toml path this default references; "" when literal. */
  readonly defaultRef: string;
  readonly restartRequired: boolean;
  readonly advanced: boolean;
  /** "" = ungated; otherwise a product.toml [features] id. */
  readonly featureGate: string;
  /** Resolved from product.toml [features] (doc 05 §6.4 F1/F2). */
  readonly featureExposed: boolean;
  readonly enumeration: readonly SettingEnumOption[];
  readonly min?: number;
  readonly max?: number;
  readonly screen: string;
  readonly section: string;
  readonly order: number;
  readonly cliFlag: string;
  readonly since: string;
}

export interface SettingsSection {
  readonly id: string;
  /** Section heading msgid. */
  readonly title: string;
  readonly entries: readonly SettingEntry[];
}

export interface SettingsArea {
  readonly id: string;
  /** Nav label msgid. */
  readonly title: string;
  readonly sections: readonly SettingsSection[];
}

export const SETTING_GLOBAL_LANGUAGE: SettingEntry = {
  key: "global.language",
  type: "enum",
  description: "settings.global.language",
  defaultValue: "en-US",
  defaultRef: "defaults.locale",
  restartRequired: true,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [{ value: "en-US", msgid: "settings.value.global-language.en-us" }],
  min: undefined,
  max: undefined,
  screen: "global",
  section: "language",
  order: 10,
  cliFlag: "--global-language",
  since: "0.1.0",
};

export const SETTING_INTERFACE_THEME: SettingEntry = {
  key: "interface.theme",
  type: "enum",
  description: "settings.interface.theme",
  defaultValue: "dark",
  defaultRef: "defaults.theme",
  restartRequired: false,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [{ value: "dark", msgid: "settings.value.interface-theme.dark" }, { value: "light", msgid: "settings.value.interface-theme.light" }],
  min: undefined,
  max: undefined,
  screen: "interface",
  section: "appearance",
  order: 10,
  cliFlag: "--interface-theme",
  since: "0.1.0",
};

export const SETTING_INTERFACE_START_MAXIMIZED: SettingEntry = {
  key: "interface.start-maximized",
  type: "bool",
  description: "settings.interface.start-maximized",
  defaultValue: false,
  defaultRef: "window.start_maximized",
  restartRequired: true,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: undefined,
  max: undefined,
  screen: "interface",
  section: "window",
  order: 10,
  cliFlag: "--interface-start-maximized",
  since: "0.1.0",
};

export const SETTING_INTERFACE_REMEMBER_WINDOW_GEOMETRY: SettingEntry = {
  key: "interface.remember-window-geometry",
  type: "bool",
  description: "settings.interface.remember-window-geometry",
  defaultValue: true,
  defaultRef: "window.remember_geometry",
  restartRequired: true,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: undefined,
  max: undefined,
  screen: "interface",
  section: "window",
  order: 20,
  cliFlag: "--interface-remember-window-geometry",
  since: "0.1.0",
};

export const SETTING_INTERFACE_WINDOW_WIDTH: SettingEntry = {
  key: "interface.window-width",
  type: "int",
  description: "settings.interface.window-width",
  defaultValue: 1280,
  defaultRef: "window.width",
  restartRequired: true,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: 480,
  max: 3840,
  screen: "interface",
  section: "window",
  order: 30,
  cliFlag: "--interface-window-width",
  since: "0.1.0",
};

export const SETTING_INTERFACE_WINDOW_HEIGHT: SettingEntry = {
  key: "interface.window-height",
  type: "int",
  description: "settings.interface.window-height",
  defaultValue: 720,
  defaultRef: "window.height",
  restartRequired: true,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: 320,
  max: 2160,
  screen: "interface",
  section: "window",
  order: 40,
  cliFlag: "--interface-window-height",
  since: "0.1.0",
};

export const SETTING_INTERFACE_WINDOW_MIN_WIDTH: SettingEntry = {
  key: "interface.window-min-width",
  type: "int",
  description: "settings.interface.window-min-width",
  defaultValue: 480,
  defaultRef: "window.min_width",
  restartRequired: true,
  advanced: true,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: 480,
  max: 3840,
  screen: "interface",
  section: "window",
  order: 50,
  cliFlag: "--interface-window-min-width",
  since: "0.1.0",
};

export const SETTING_INTERFACE_WINDOW_MIN_HEIGHT: SettingEntry = {
  key: "interface.window-min-height",
  type: "int",
  description: "settings.interface.window-min-height",
  defaultValue: 320,
  defaultRef: "window.min_height",
  restartRequired: true,
  advanced: true,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: 320,
  max: 2160,
  screen: "interface",
  section: "window",
  order: 60,
  cliFlag: "--interface-window-min-height",
  since: "0.1.0",
};

export const SETTING_PRIVACY_TELEMETRY: SettingEntry = {
  key: "privacy.telemetry",
  type: "bool",
  description: "settings.privacy.telemetry",
  defaultValue: false,
  defaultRef: "telemetry.enabled",
  restartRequired: false,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: undefined,
  max: undefined,
  screen: "privacy",
  section: "data",
  order: 10,
  cliFlag: "--privacy-telemetry",
  since: "0.1.0",
};

export const SETTING_PRIVACY_CRASH_REPORTS: SettingEntry = {
  key: "privacy.crash-reports",
  type: "enum",
  description: "settings.privacy.crash-reports",
  defaultValue: "off",
  defaultRef: "telemetry.crash_reports",
  restartRequired: false,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [{ value: "off", msgid: "settings.value.privacy-crash-reports.off" }, { value: "ask", msgid: "settings.value.privacy-crash-reports.ask" }, { value: "send", msgid: "settings.value.privacy-crash-reports.send" }],
  min: undefined,
  max: undefined,
  screen: "privacy",
  section: "data",
  order: 20,
  cliFlag: "--privacy-crash-reports",
  since: "0.1.0",
};

export const SETTING_PRIVACY_USAGE_STATS: SettingEntry = {
  key: "privacy.usage-stats",
  type: "bool",
  description: "settings.privacy.usage-stats",
  defaultValue: false,
  defaultRef: "telemetry.usage_stats",
  restartRequired: false,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: undefined,
  max: undefined,
  screen: "privacy",
  section: "data",
  order: 30,
  cliFlag: "--privacy-usage-stats",
  since: "0.1.0",
};

export const SETTING_PRIVACY_UPDATE_CHECKS: SettingEntry = {
  key: "privacy.update-checks",
  type: "bool",
  description: "settings.privacy.update-checks",
  defaultValue: false,
  defaultRef: "telemetry.update_checks",
  restartRequired: false,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: undefined,
  max: undefined,
  screen: "privacy",
  section: "data",
  order: 40,
  cliFlag: "--privacy-update-checks",
  since: "0.1.0",
};

export const SETTING_PLAYBACK_PREFERRED_AUDIO_LANGUAGES: SettingEntry = {
  key: "playback.preferred-audio-languages",
  type: "list<string>",
  description: "settings.playback.preferred-audio-languages",
  defaultValue: ["en"],
  defaultRef: "playback.preferred_audio_languages",
  restartRequired: false,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: undefined,
  max: undefined,
  screen: "playback",
  section: "tracks",
  order: 10,
  cliFlag: "--playback-preferred-audio-languages",
  since: "0.1.0",
};

export const SETTING_PLAYBACK_PREFERRED_SUBTITLE_LANGUAGES: SettingEntry = {
  key: "playback.preferred-subtitle-languages",
  type: "list<string>",
  description: "settings.playback.preferred-subtitle-languages",
  defaultValue: ["en"],
  defaultRef: "playback.preferred_subtitle_languages",
  restartRequired: false,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: undefined,
  max: undefined,
  screen: "playback",
  section: "tracks",
  order: 20,
  cliFlag: "--playback-preferred-subtitle-languages",
  since: "0.1.0",
};

export const SETTING_PLAYBACK_SUBTITLE_MODE: SettingEntry = {
  key: "playback.subtitle-mode",
  type: "enum",
  description: "settings.playback.subtitle-mode",
  defaultValue: "auto",
  defaultRef: "playback.subtitle_mode",
  restartRequired: false,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [{ value: "auto", msgid: "settings.value.playback-subtitle-mode.auto" }, { value: "forced-only", msgid: "settings.value.playback-subtitle-mode.forced-only" }, { value: "on", msgid: "settings.value.playback-subtitle-mode.on" }, { value: "off", msgid: "settings.value.playback-subtitle-mode.off" }],
  min: undefined,
  max: undefined,
  screen: "playback",
  section: "tracks",
  order: 30,
  cliFlag: "--playback-subtitle-mode",
  since: "0.1.0",
};

export const SETTING_PLAYBACK_HW_DECODE_POLICY: SettingEntry = {
  key: "playback.hw-decode-policy",
  type: "enum",
  description: "settings.playback.hw-decode-policy",
  defaultValue: "software-only",
  defaultRef: "playback.hw_decode_policy",
  restartRequired: false,
  advanced: true,
  featureGate: "hw-decode",
  featureExposed: false,
  enumeration: [{ value: "software-only", msgid: "settings.value.playback-hw-decode-policy.software-only" }, { value: "prefer-hardware", msgid: "settings.value.playback-hw-decode-policy.prefer-hardware" }, { value: "hardware-only", msgid: "settings.value.playback-hw-decode-policy.hardware-only" }],
  min: undefined,
  max: undefined,
  screen: "playback",
  section: "tracks",
  order: 40,
  cliFlag: "--playback-hw-decode-policy",
  since: "0.1.0",
};

export const SETTING_PLAYBACK_START_PAUSED: SettingEntry = {
  key: "playback.start-paused",
  type: "bool",
  description: "settings.playback.start-paused",
  defaultValue: false,
  defaultRef: "playback.start_paused",
  restartRequired: false,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: undefined,
  max: undefined,
  screen: "playback",
  section: "playback",
  order: 10,
  cliFlag: "--playback-start-paused",
  since: "0.1.0",
};

export const SETTING_PLAYBACK_DEFAULT_RATE: SettingEntry = {
  key: "playback.default-rate",
  type: "float",
  description: "settings.playback.default-rate",
  defaultValue: 1,
  defaultRef: "playback.default_rate",
  restartRequired: false,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: 0.25,
  max: 4,
  screen: "playback",
  section: "playback",
  order: 20,
  cliFlag: "--playback-default-rate",
  since: "0.1.0",
};

export const SETTING_PERFORMANCE_CACHE_DEMUX_MIB: SettingEntry = {
  key: "performance.cache-demux-mib",
  type: "int",
  description: "settings.performance.cache-demux-mib",
  defaultValue: 32,
  defaultRef: "limits.cache_demux_mib",
  restartRequired: true,
  advanced: true,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: 4,
  max: 512,
  screen: "performance",
  section: "resources",
  order: 10,
  cliFlag: "--performance-cache-demux-mib",
  since: "0.1.0",
};

export const SETTING_PERFORMANCE_CACHE_NETWORK_MIB: SettingEntry = {
  key: "performance.cache-network-mib",
  type: "int",
  description: "settings.performance.cache-network-mib",
  defaultValue: 64,
  defaultRef: "limits.cache_network_mib",
  restartRequired: true,
  advanced: true,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: 4,
  max: 1024,
  screen: "performance",
  section: "resources",
  order: 20,
  cliFlag: "--performance-cache-network-mib",
  since: "0.1.0",
};

export const SETTING_PERFORMANCE_MAX_OPEN_SESSIONS: SettingEntry = {
  key: "performance.max-open-sessions",
  type: "int",
  description: "settings.performance.max-open-sessions",
  defaultValue: 4,
  defaultRef: "limits.max_open_sessions",
  restartRequired: false,
  advanced: true,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: 1,
  max: 16,
  screen: "performance",
  section: "resources",
  order: 30,
  cliFlag: "--performance-max-open-sessions",
  since: "0.1.0",
};

export const SETTING_ENGINE_LOG_LEVEL: SettingEntry = {
  key: "engine.log-level",
  type: "enum",
  description: "settings.engine.log-level",
  defaultValue: "info",
  defaultRef: "",
  restartRequired: false,
  advanced: true,
  featureGate: "",
  featureExposed: true,
  enumeration: [{ value: "error", msgid: "settings.value.engine-log-level.error" }, { value: "warn", msgid: "settings.value.engine-log-level.warn" }, { value: "info", msgid: "settings.value.engine-log-level.info" }, { value: "debug", msgid: "settings.value.engine-log-level.debug" }, { value: "trace", msgid: "settings.value.engine-log-level.trace" }],
  min: undefined,
  max: undefined,
  screen: "engine",
  section: "internal",
  order: 10,
  cliFlag: "--engine-log-level",
  since: "0.1.0",
};

export const SETTING_ENGINE_TICK_INTERVAL_MS: SettingEntry = {
  key: "engine.tick-interval-ms",
  type: "duration-ms",
  description: "settings.engine.tick-interval-ms",
  defaultValue: 250,
  defaultRef: "",
  restartRequired: true,
  advanced: true,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: 50,
  max: 2000,
  screen: "engine",
  section: "internal",
  order: 20,
  cliFlag: "--engine-tick-interval-ms",
  since: "0.1.0",
};

export const SETTING_ENGINE_MAX_EVENT_RATE: SettingEntry = {
  key: "engine.max-event-rate",
  type: "int",
  description: "settings.engine.max-event-rate",
  defaultValue: 100,
  defaultRef: "",
  restartRequired: true,
  advanced: true,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: 10,
  max: 1000,
  screen: "engine",
  section: "internal",
  order: 30,
  cliFlag: "--engine-max-event-rate",
  since: "0.1.0",
};

export const SETTING_ENGINE_PROBE_MAX_BYTES: SettingEntry = {
  key: "engine.probe-max-bytes",
  type: "int",
  description: "settings.engine.probe-max-bytes",
  defaultValue: 16777216,
  defaultRef: "limits.probe_max_bytes",
  restartRequired: false,
  advanced: true,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: 1048576,
  max: 67108864,
  screen: "engine",
  section: "internal",
  order: 40,
  cliFlag: "--engine-probe-max-bytes",
  since: "0.1.0",
};

export const SETTING_ENGINE_QUEUE_PACKETS_AUDIO: SettingEntry = {
  key: "engine.queue-packets-audio",
  type: "int",
  description: "settings.engine.queue-packets-audio",
  defaultValue: 1024,
  defaultRef: "limits.queue_packets_audio",
  restartRequired: true,
  advanced: true,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: 64,
  max: 8192,
  screen: "engine",
  section: "internal",
  order: 50,
  cliFlag: "--engine-queue-packets-audio",
  since: "0.1.0",
};

export const SETTING_ENGINE_QUEUE_PACKETS_VIDEO: SettingEntry = {
  key: "engine.queue-packets-video",
  type: "int",
  description: "settings.engine.queue-packets-video",
  defaultValue: 512,
  defaultRef: "limits.queue_packets_video",
  restartRequired: true,
  advanced: true,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: 64,
  max: 4096,
  screen: "engine",
  section: "internal",
  order: 60,
  cliFlag: "--engine-queue-packets-video",
  since: "0.1.0",
};

export const SETTING_EXTENSIONS_CRASH_BACKOFF_INITIAL_MS: SettingEntry = {
  key: "extensions.crash-backoff-initial-ms",
  type: "duration-ms",
  description: "settings.extensions.crash-backoff-initial-ms",
  defaultValue: 5000,
  defaultRef: "",
  restartRequired: false,
  advanced: true,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: 0,
  max: 600000,
  screen: "extensions",
  section: "crash-isolation",
  order: 10,
  cliFlag: "--extensions-crash-backoff-initial-ms",
  since: "0.1.0",
};

export const SETTING_EXTENSIONS_CRASH_BACKOFF_MULTIPLIER: SettingEntry = {
  key: "extensions.crash-backoff-multiplier",
  type: "int",
  description: "settings.extensions.crash-backoff-multiplier",
  defaultValue: 6,
  defaultRef: "",
  restartRequired: false,
  advanced: true,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: 1,
  max: 100,
  screen: "extensions",
  section: "crash-isolation",
  order: 20,
  cliFlag: "--extensions-crash-backoff-multiplier",
  since: "0.1.0",
};

export const SETTING_EXTENSIONS_CRASH_BACKOFF_MAX_MS: SettingEntry = {
  key: "extensions.crash-backoff-max-ms",
  type: "duration-ms",
  description: "settings.extensions.crash-backoff-max-ms",
  defaultValue: 300000,
  defaultRef: "",
  restartRequired: false,
  advanced: true,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: 5000,
  max: 3600000,
  screen: "extensions",
  section: "crash-isolation",
  order: 30,
  cliFlag: "--extensions-crash-backoff-max-ms",
  since: "0.1.0",
};

export const SETTING_EXTENSIONS_CRASH_RESTART_LIMIT: SettingEntry = {
  key: "extensions.crash-restart-limit",
  type: "int",
  description: "settings.extensions.crash-restart-limit",
  defaultValue: 3,
  defaultRef: "",
  restartRequired: false,
  advanced: true,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: 1,
  max: 10,
  screen: "extensions",
  section: "crash-isolation",
  order: 40,
  cliFlag: "--extensions-crash-restart-limit",
  since: "0.1.0",
};

export const SETTING_EXTENSIONS_CRASH_DISABLED_REENABLE_MS: SettingEntry = {
  key: "extensions.crash-disabled-reenable-ms",
  type: "duration-ms",
  description: "settings.extensions.crash-disabled-reenable-ms",
  defaultValue: 900000,
  defaultRef: "",
  restartRequired: false,
  advanced: true,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: 0,
  max: 86400000,
  screen: "extensions",
  section: "crash-isolation",
  order: 50,
  cliFlag: "--extensions-crash-disabled-reenable-ms",
  since: "0.1.0",
};

export const SETTING_CONTROLS_SKIP_VERY_SHORT: SettingEntry = {
  key: "controls.skip-very-short",
  type: "duration-ms",
  description: "settings.controls.skip-very-short",
  defaultValue: 3000,
  defaultRef: "",
  restartRequired: false,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: 1000,
  max: 60000,
  screen: "controls",
  section: "skip-amounts",
  order: 10,
  cliFlag: "--controls-skip-very-short",
  since: "0.1.0",
};

export const SETTING_CONTROLS_SKIP_SHORT: SettingEntry = {
  key: "controls.skip-short",
  type: "duration-ms",
  description: "settings.controls.skip-short",
  defaultValue: 10000,
  defaultRef: "",
  restartRequired: false,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: 1000,
  max: 120000,
  screen: "controls",
  section: "skip-amounts",
  order: 20,
  cliFlag: "--controls-skip-short",
  since: "0.1.0",
};

export const SETTING_CONTROLS_SKIP_MEDIUM: SettingEntry = {
  key: "controls.skip-medium",
  type: "duration-ms",
  description: "settings.controls.skip-medium",
  defaultValue: 60000,
  defaultRef: "",
  restartRequired: false,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: 5000,
  max: 600000,
  screen: "controls",
  section: "skip-amounts",
  order: 30,
  cliFlag: "--controls-skip-medium",
  since: "0.1.0",
};

export const SETTING_CONTROLS_SKIP_LONG: SettingEntry = {
  key: "controls.skip-long",
  type: "duration-ms",
  description: "settings.controls.skip-long",
  defaultValue: 300000,
  defaultRef: "",
  restartRequired: false,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: 10000,
  max: 1800000,
  screen: "controls",
  section: "skip-amounts",
  order: 40,
  cliFlag: "--controls-skip-long",
  since: "0.1.0",
};

export const SETTING_CONTROLS_MOUSE_SINGLE_CLICK: SettingEntry = {
  key: "controls.mouse-single-click",
  type: "enum",
  description: "settings.controls.mouse-single-click",
  defaultValue: "play-pause",
  defaultRef: "",
  restartRequired: false,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [{ value: "play-pause", msgid: "settings.value.controls-mouse-single-click.play-pause" }, { value: "toggle-fullscreen", msgid: "settings.value.controls-mouse-single-click.toggle-fullscreen" }, { value: "show-menu", msgid: "settings.value.controls-mouse-single-click.show-menu" }, { value: "none", msgid: "settings.value.controls-mouse-single-click.none" }],
  min: undefined,
  max: undefined,
  screen: "controls",
  section: "mouse",
  order: 10,
  cliFlag: "--controls-mouse-single-click",
  since: "0.1.0",
};

export const SETTING_CONTROLS_MOUSE_DOUBLE_CLICK: SettingEntry = {
  key: "controls.mouse-double-click",
  type: "enum",
  description: "settings.controls.mouse-double-click",
  defaultValue: "toggle-fullscreen",
  defaultRef: "",
  restartRequired: false,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [{ value: "toggle-fullscreen", msgid: "settings.value.controls-mouse-double-click.toggle-fullscreen" }, { value: "play-pause", msgid: "settings.value.controls-mouse-double-click.play-pause" }, { value: "show-menu", msgid: "settings.value.controls-mouse-double-click.show-menu" }, { value: "none", msgid: "settings.value.controls-mouse-double-click.none" }],
  min: undefined,
  max: undefined,
  screen: "controls",
  section: "mouse",
  order: 20,
  cliFlag: "--controls-mouse-double-click",
  since: "0.1.0",
};

export const SETTING_CONTROLS_MOUSE_SCROLL_WHEEL: SettingEntry = {
  key: "controls.mouse-scroll-wheel",
  type: "enum",
  description: "settings.controls.mouse-scroll-wheel",
  defaultValue: "volume",
  defaultRef: "",
  restartRequired: false,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [{ value: "volume", msgid: "settings.value.controls-mouse-scroll-wheel.volume" }, { value: "seek", msgid: "settings.value.controls-mouse-scroll-wheel.seek" }, { value: "none", msgid: "settings.value.controls-mouse-scroll-wheel.none" }],
  min: undefined,
  max: undefined,
  screen: "controls",
  section: "mouse",
  order: 30,
  cliFlag: "--controls-mouse-scroll-wheel",
  since: "0.1.0",
};

export const SETTING_CONTROLS_MOUSE_HORIZONTAL_SCROLL: SettingEntry = {
  key: "controls.mouse-horizontal-scroll",
  type: "enum",
  description: "settings.controls.mouse-horizontal-scroll",
  defaultValue: "seek",
  defaultRef: "",
  restartRequired: false,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [{ value: "seek", msgid: "settings.value.controls-mouse-horizontal-scroll.seek" }, { value: "volume", msgid: "settings.value.controls-mouse-horizontal-scroll.volume" }, { value: "none", msgid: "settings.value.controls-mouse-horizontal-scroll.none" }],
  min: undefined,
  max: undefined,
  screen: "controls",
  section: "mouse",
  order: 40,
  cliFlag: "--controls-mouse-horizontal-scroll",
  since: "0.1.0",
};

export const SETTING_CONTROLS_MOUSE_REVERSE_SCROLL_DIRECTION: SettingEntry = {
  key: "controls.mouse-reverse-scroll-direction",
  type: "bool",
  description: "settings.controls.mouse-reverse-scroll-direction",
  defaultValue: false,
  defaultRef: "",
  restartRequired: false,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  min: undefined,
  max: undefined,
  screen: "controls",
  section: "mouse",
  order: 50,
  cliFlag: "--controls-mouse-reverse-scroll-direction",
  since: "0.1.0",
};

export const SETTING_RECORDING_DIRECTORY: SettingEntry = {
  key: "recording.directory",
  type: "path",
  description: "settings.recording.directory",
  defaultValue: "~/Videos",
  defaultRef: "",
  restartRequired: false,
  advanced: false,
  featureGate: "recording",
  featureExposed: false,
  enumeration: [],
  min: undefined,
  max: undefined,
  screen: "recording",
  section: "recording",
  order: 10,
  cliFlag: "--recording-directory",
  since: "0.1.0",
};

export const SETTING_RECORDING_FORMAT: SettingEntry = {
  key: "recording.format",
  type: "enum",
  description: "settings.recording.format",
  defaultValue: "mp4-remux",
  defaultRef: "",
  restartRequired: false,
  advanced: false,
  featureGate: "recording",
  featureExposed: false,
  enumeration: [{ value: "mp4-remux", msgid: "settings.value.recording-format.mp4-remux" }, { value: "wav", msgid: "settings.value.recording-format.wav" }],
  min: undefined,
  max: undefined,
  screen: "recording",
  section: "recording",
  order: 20,
  cliFlag: "--recording-format",
  since: "0.1.0",
};

export const SETTING_EXTENSIONS_LOAD_ON_STARTUP: SettingEntry = {
  key: "extensions.load-on-startup",
  type: "bool",
  description: "settings.extensions.load-on-startup",
  defaultValue: true,
  defaultRef: "",
  restartRequired: true,
  advanced: false,
  featureGate: "wasm-extensions",
  featureExposed: false,
  enumeration: [],
  min: undefined,
  max: undefined,
  screen: "extensions",
  section: "add-ons",
  order: 10,
  cliFlag: "--extensions-load-on-startup",
  since: "0.1.0",
};

/** Every ui=true entry, in (screen, order) order. */
export const SETTINGS_ENTRIES: readonly SettingEntry[] = [
  SETTING_GLOBAL_LANGUAGE,
  SETTING_INTERFACE_THEME,
  SETTING_INTERFACE_START_MAXIMIZED,
  SETTING_INTERFACE_REMEMBER_WINDOW_GEOMETRY,
  SETTING_INTERFACE_WINDOW_WIDTH,
  SETTING_INTERFACE_WINDOW_HEIGHT,
  SETTING_INTERFACE_WINDOW_MIN_WIDTH,
  SETTING_INTERFACE_WINDOW_MIN_HEIGHT,
  SETTING_PRIVACY_TELEMETRY,
  SETTING_PRIVACY_CRASH_REPORTS,
  SETTING_PRIVACY_USAGE_STATS,
  SETTING_PRIVACY_UPDATE_CHECKS,
  SETTING_PLAYBACK_PREFERRED_AUDIO_LANGUAGES,
  SETTING_PLAYBACK_PREFERRED_SUBTITLE_LANGUAGES,
  SETTING_PLAYBACK_SUBTITLE_MODE,
  SETTING_PLAYBACK_HW_DECODE_POLICY,
  SETTING_PLAYBACK_START_PAUSED,
  SETTING_PLAYBACK_DEFAULT_RATE,
  SETTING_PERFORMANCE_CACHE_DEMUX_MIB,
  SETTING_PERFORMANCE_CACHE_NETWORK_MIB,
  SETTING_PERFORMANCE_MAX_OPEN_SESSIONS,
  SETTING_CONTROLS_SKIP_VERY_SHORT,
  SETTING_CONTROLS_SKIP_SHORT,
  SETTING_CONTROLS_SKIP_MEDIUM,
  SETTING_CONTROLS_SKIP_LONG,
  SETTING_CONTROLS_MOUSE_SINGLE_CLICK,
  SETTING_CONTROLS_MOUSE_DOUBLE_CLICK,
  SETTING_CONTROLS_MOUSE_SCROLL_WHEEL,
  SETTING_CONTROLS_MOUSE_HORIZONTAL_SCROLL,
  SETTING_CONTROLS_MOUSE_REVERSE_SCROLL_DIRECTION,
  SETTING_RECORDING_DIRECTORY,
  SETTING_RECORDING_FORMAT,
  SETTING_EXTENSIONS_LOAD_ON_STARTUP,
];

export const SETTINGS_AREAS: readonly SettingsArea[] = [
  {
    id: "global",
    title: "settings.area.global",
    sections: [
      {
        id: "language",
        title: "settings.section.global.language",
        entries: [SETTING_GLOBAL_LANGUAGE],
      },
    ],
  },
  {
    id: "interface",
    title: "settings.area.interface",
    sections: [
      {
        id: "appearance",
        title: "settings.section.interface.appearance",
        entries: [SETTING_INTERFACE_THEME],
      },
      {
        id: "window",
        title: "settings.section.interface.window",
        entries: [SETTING_INTERFACE_START_MAXIMIZED, SETTING_INTERFACE_REMEMBER_WINDOW_GEOMETRY, SETTING_INTERFACE_WINDOW_WIDTH, SETTING_INTERFACE_WINDOW_HEIGHT, SETTING_INTERFACE_WINDOW_MIN_WIDTH, SETTING_INTERFACE_WINDOW_MIN_HEIGHT],
      },
    ],
  },
  {
    id: "privacy",
    title: "settings.area.privacy",
    sections: [
      {
        id: "data",
        title: "settings.section.privacy.data",
        entries: [SETTING_PRIVACY_TELEMETRY, SETTING_PRIVACY_CRASH_REPORTS, SETTING_PRIVACY_USAGE_STATS, SETTING_PRIVACY_UPDATE_CHECKS],
      },
    ],
  },
  {
    id: "controls",
    title: "settings.area.controls",
    sections: [
      {
        id: "skip-amounts",
        title: "settings.section.controls.skip-amounts",
        entries: [SETTING_CONTROLS_SKIP_VERY_SHORT, SETTING_CONTROLS_SKIP_SHORT, SETTING_CONTROLS_SKIP_MEDIUM, SETTING_CONTROLS_SKIP_LONG],
      },
      {
        id: "mouse",
        title: "settings.section.controls.mouse",
        entries: [SETTING_CONTROLS_MOUSE_SINGLE_CLICK, SETTING_CONTROLS_MOUSE_DOUBLE_CLICK, SETTING_CONTROLS_MOUSE_SCROLL_WHEEL, SETTING_CONTROLS_MOUSE_HORIZONTAL_SCROLL, SETTING_CONTROLS_MOUSE_REVERSE_SCROLL_DIRECTION],
      },
    ],
  },
  {
    id: "playback",
    title: "settings.area.playback",
    sections: [
      {
        id: "tracks",
        title: "settings.section.playback.tracks",
        entries: [SETTING_PLAYBACK_PREFERRED_AUDIO_LANGUAGES, SETTING_PLAYBACK_PREFERRED_SUBTITLE_LANGUAGES, SETTING_PLAYBACK_SUBTITLE_MODE, SETTING_PLAYBACK_HW_DECODE_POLICY],
      },
      {
        id: "playback",
        title: "settings.section.playback.playback",
        entries: [SETTING_PLAYBACK_START_PAUSED, SETTING_PLAYBACK_DEFAULT_RATE],
      },
    ],
  },
  {
    id: "performance",
    title: "settings.area.performance",
    sections: [
      {
        id: "resources",
        title: "settings.section.performance.resources",
        entries: [SETTING_PERFORMANCE_CACHE_DEMUX_MIB, SETTING_PERFORMANCE_CACHE_NETWORK_MIB, SETTING_PERFORMANCE_MAX_OPEN_SESSIONS],
      },
    ],
  },
  {
    id: "recording",
    title: "settings.area.recording",
    sections: [
      {
        id: "recording",
        title: "settings.section.recording.recording",
        entries: [SETTING_RECORDING_DIRECTORY, SETTING_RECORDING_FORMAT],
      },
    ],
  },
  {
    id: "extensions",
    title: "settings.area.extensions",
    sections: [
      {
        id: "add-ons",
        title: "settings.section.extensions.add-ons",
        entries: [SETTING_EXTENSIONS_LOAD_ON_STARTUP],
      },
    ],
  },
];

/** Field-label msgids in schema order — the i18n coverage contract. */
export const SETTINGS_MSGIDS: readonly string[] = [
  "settings.global.language",
  "settings.interface.theme",
  "settings.interface.start-maximized",
  "settings.interface.remember-window-geometry",
  "settings.interface.window-width",
  "settings.interface.window-height",
  "settings.interface.window-min-width",
  "settings.interface.window-min-height",
  "settings.privacy.telemetry",
  "settings.privacy.crash-reports",
  "settings.privacy.usage-stats",
  "settings.privacy.update-checks",
  "settings.playback.preferred-audio-languages",
  "settings.playback.preferred-subtitle-languages",
  "settings.playback.subtitle-mode",
  "settings.playback.hw-decode-policy",
  "settings.playback.start-paused",
  "settings.playback.default-rate",
  "settings.performance.cache-demux-mib",
  "settings.performance.cache-network-mib",
  "settings.performance.max-open-sessions",
  "settings.engine.log-level",
  "settings.engine.tick-interval-ms",
  "settings.engine.max-event-rate",
  "settings.engine.probe-max-bytes",
  "settings.engine.queue-packets-audio",
  "settings.engine.queue-packets-video",
  "settings.extensions.crash-backoff-initial-ms",
  "settings.extensions.crash-backoff-multiplier",
  "settings.extensions.crash-backoff-max-ms",
  "settings.extensions.crash-restart-limit",
  "settings.extensions.crash-disabled-reenable-ms",
  "settings.controls.skip-very-short",
  "settings.controls.skip-short",
  "settings.controls.skip-medium",
  "settings.controls.skip-long",
  "settings.controls.mouse-single-click",
  "settings.controls.mouse-double-click",
  "settings.controls.mouse-scroll-wheel",
  "settings.controls.mouse-horizontal-scroll",
  "settings.controls.mouse-reverse-scroll-direction",
  "settings.recording.directory",
  "settings.recording.format",
  "settings.extensions.load-on-startup",
];
