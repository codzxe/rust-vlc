// Message catalog + useMsg() for the Phase-4 scaffold.
//
// Doc 07 §6.5 / ui README: user-visible text must flow through useMsg() +
// MessageKey — the `lumen/i18n-no-literals` lint rule (eslint.config.js)
// bans raw JSX literals. The real generated locale packs arrive with the i18n
// pipeline (doc 07); this hand-written table is the scaffold stand-in and
// every entry honestly labels its screen as a placeholder.

export type MessageKey =
  | "screen.player"
  | "screen.player.music"
  | "screen.miniplayer"
  | "screen.playlist"
  | "screen.settings"
  | "screen.onboarding"
  | "screen.keyboard-shortcuts"
  | "screen.open-network-stream"
  | "screen.not-found"
  | "screen.placeholder.body"
  | "screen.dev.tokens"
  | "tokens.mode"
  | "tokens.accent"
  // Settings framework chrome (card 4.E1).
  | "settings.nav.title"
  | "settings.search.label"
  | "settings.search.no-results"
  | "settings.apply"
  | "settings.revert"
  | "settings.unsaved-changes"
  | "settings.restart-required"
  | "settings.restart-applied"
  | "settings.advanced-badge"
  | "settings.reset-field"
  | "settings.reset-screen"
  | "settings.confirm-leave.title"
  | "settings.confirm-leave.body"
  | "settings.confirm-leave.discard"
  | "settings.confirm-leave.stay"
  // Area + section labels (generated nav, card 4.E1; ids from schemas/settings.toml).
  | "settings.area.global"
  | "settings.area.interface"
  | "settings.area.privacy"
  | "settings.area.playback"
  | "settings.area.performance"
  | "settings.section.global.language"
  | "settings.section.interface.appearance"
  | "settings.section.interface.window"
  | "settings.section.privacy.data"
  | "settings.section.playback.tracks"
  | "settings.section.playback.playback"
  | "settings.section.performance.resources"
  // Field labels — one per schemas/settings.toml entry (drift-checked by
  // ui/src/screens/settings/generated.test.ts against SETTINGS_MSGIDS).
  | "settings.global.language"
  | "settings.interface.theme"
  | "settings.interface.start-maximized"
  | "settings.interface.remember-window-geometry"
  | "settings.interface.window-width"
  | "settings.interface.window-height"
  | "settings.interface.window-min-width"
  | "settings.interface.window-min-height"
  | "settings.privacy.telemetry"
  | "settings.privacy.crash-reports"
  | "settings.privacy.usage-stats"
  | "settings.privacy.update-checks"
  | "settings.playback.preferred-audio-languages"
  | "settings.playback.preferred-subtitle-languages"
  | "settings.playback.subtitle-mode"
  | "settings.playback.hw-decode-policy"
  | "settings.playback.start-paused"
  | "settings.playback.default-rate"
  | "settings.performance.cache-demux-mib"
  | "settings.performance.cache-network-mib"
  | "settings.performance.max-open-sessions"
  // Enum option labels (values are schema technical ids; copy lives here).
  | "settings.value.global-language.en-us"
  | "settings.value.interface-theme.dark"
  | "settings.value.interface-theme.light"
  | "settings.value.privacy-crash-reports.off"
  | "settings.value.privacy-crash-reports.ask"
  | "settings.value.privacy-crash-reports.send"
  | "settings.value.playback-subtitle-mode.auto"
  | "settings.value.playback-subtitle-mode.forced-only"
  | "settings.value.playback-subtitle-mode.on"
  | "settings.value.playback-subtitle-mode.off"
  | "settings.value.playback-hw-decode-policy.software-only"
  | "settings.value.playback-hw-decode-policy.prefer-hardware"
  | "settings.value.playback-hw-decode-policy.hardware-only";

const messages: Readonly<Record<MessageKey, string>> = {
  "screen.player": "Player (placeholder)",
  "screen.player.music": "Music player (placeholder)",
  "screen.miniplayer": "Miniplayer (placeholder)",
  "screen.playlist": "Playlist (placeholder)",
  "screen.settings": "Settings",
  "screen.onboarding": "Onboarding (placeholder)",
  "screen.keyboard-shortcuts": "Keyboard shortcuts (placeholder)",
  "screen.open-network-stream": "Open network stream (placeholder)",
  "screen.not-found": "Screen not found",
  "screen.placeholder.body":
    "Route skeleton only — the Figma-exact composition lands in later Phase-4 cards (doc 08 §1.6).",
  "screen.dev.tokens": "Token sandbox (dev fixture — not a product screen)",
  "tokens.mode": "Mode",
  "tokens.accent": "Accent",
  "settings.nav.title": "Settings",
  "settings.search.label": "Search settings",
  "settings.search.no-results": "No settings match your search",
  "settings.apply": "Apply",
  "settings.revert": "Revert",
  "settings.unsaved-changes": "Unsaved changes",
  "settings.restart-required": "Restart required",
  "settings.restart-applied": "Applied — restart Lumen for the changes to take effect",
  "settings.advanced-badge": "Advanced",
  "settings.reset-field": "Reset to default",
  "settings.reset-screen": "Reset all to defaults",
  "settings.confirm-leave.title": "Discard unsaved changes?",
  "settings.confirm-leave.body": "Changes that were not applied will be lost.",
  "settings.confirm-leave.discard": "Discard changes",
  "settings.confirm-leave.stay": "Stay",
  "settings.area.global": "Global",
  "settings.area.interface": "Interface",
  "settings.area.privacy": "Privacy",
  "settings.area.playback": "Playback",
  "settings.area.performance": "Performance",
  "settings.section.global.language": "Language",
  "settings.section.interface.appearance": "Appearance",
  "settings.section.interface.window": "Window",
  "settings.section.privacy.data": "Data & privacy",
  "settings.section.playback.tracks": "Tracks",
  "settings.section.playback.playback": "Playback",
  "settings.section.performance.resources": "Resources",
  "settings.global.language": "Language",
  "settings.interface.theme": "Theme",
  "settings.interface.start-maximized": "Start maximized",
  "settings.interface.remember-window-geometry": "Remember window geometry",
  "settings.interface.window-width": "Window width",
  "settings.interface.window-height": "Window height",
  "settings.interface.window-min-width": "Minimum window width",
  "settings.interface.window-min-height": "Minimum window height",
  "settings.privacy.telemetry": "Share anonymous usage data",
  "settings.privacy.crash-reports": "Crash reports",
  "settings.privacy.usage-stats": "Usage statistics",
  "settings.privacy.update-checks": "Check for updates",
  "settings.playback.preferred-audio-languages": "Preferred audio languages",
  "settings.playback.preferred-subtitle-languages": "Preferred subtitle languages",
  "settings.playback.subtitle-mode": "Subtitle mode",
  "settings.playback.hw-decode-policy": "Hardware decode policy",
  "settings.playback.start-paused": "Start paused",
  "settings.playback.default-rate": "Default playback rate",
  "settings.performance.cache-demux-mib": "Demuxer cache (MiB)",
  "settings.performance.cache-network-mib": "Network cache (MiB)",
  "settings.performance.max-open-sessions": "Maximum open sessions",
  "settings.value.global-language.en-us": "English (US)",
  "settings.value.interface-theme.dark": "Dark",
  "settings.value.interface-theme.light": "Light",
  "settings.value.privacy-crash-reports.off": "Off",
  "settings.value.privacy-crash-reports.ask": "Ask",
  "settings.value.privacy-crash-reports.send": "Send",
  "settings.value.playback-subtitle-mode.auto": "Automatic",
  "settings.value.playback-subtitle-mode.forced-only": "Forced only",
  "settings.value.playback-subtitle-mode.on": "On",
  "settings.value.playback-subtitle-mode.off": "Off",
  "settings.value.playback-hw-decode-policy.software-only": "Software only",
  "settings.value.playback-hw-decode-policy.prefer-hardware": "Prefer hardware",
  "settings.value.playback-hw-decode-policy.hardware-only": "Hardware only",
};

export type Msg = (key: MessageKey) => string;

// Scaffold stand-in for the generated i18n hook. Hook-shaped on purpose so
// call sites do not change when the real locale-pack implementation lands.
export function useMsg(): Msg {
  return (key) => messages[key];
}
