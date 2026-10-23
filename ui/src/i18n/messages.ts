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
  // --- player screen shell (card 4.C1) ---
  | "player.canvas.label"
  | "player.canvas.placeholder"
  | "player.region.title-bar"
  | "player.region.controls-bar"
  | "player.region.library"
  | "player.region.playlist"
  | "player.region.media-info"
  | "player.panel.placeholder"
  | "player.controls.placeholder"
  | "player.menu.app"
  | "player.empty.title"
  | "player.empty.body"
  | "player.empty.open"
  // --- control bar (card 4.C2) ---
  | "player.controls.play"
  | "player.controls.pause"
  | "player.controls.stop"
  | "player.controls.restart"
  | "player.controls.buffering-hint"
  | "player.controls.seekbar"
  | "player.controls.volume"
  | "player.controls.mute"
  | "player.controls.unmute"
  | "player.controls.duration-unknown"
  | "player.controls.group-transport"
  | "player.controls.group-volume"
  | "player.controls.group-tracks"
  | "player.controls.reason.no-media"
  | "player.controls.reason.not-seekable"
  | "player.controls.reason.unknown"
  | "player.track.audio"
  | "player.track.video"
  | "player.track.subtitle"
  | "player.track.unknown"
  | "player.track.auto"
  | "player.track.disable"
  | "player.error.title"
  | "player.error.dismiss"
  // --- playback UX states (card 4.C3, doc 08 §5.3/§5.4) ---
  | "player.state.loading.label"
  | "player.state.buffering.label"
  | "player.state.unsupported.title"
  | "player.state.unsupported.body"
  | "player.state.unsupported.container-label"
  | "player.state.unsupported.codec-label"
  | "player.state.unsupported.no-format"
  | "player.state.error.title"
  | "player.state.error.stage-label"
  | "player.state.error.stage.unknown"
  | "player.state.error.unknown"
  | "player.state.error.retry"
  | "player.state.error.stage.session"
  | "player.state.error.stage.input"
  | "player.state.error.stage.probe"
  | "player.state.error.stage.demux"
  | "player.state.error.stage.decode"
  | "player.state.error.stage.audio-output"
  | "player.state.error.stage.video-output"
  | "player.state.error.stage.subtitle"
  | "player.state.error.stage.renderer"
  | "player.state.error.stage.playlist"
  | "player.state.error.stage.config"
  | "player.state.error.stage.plugin"
  | "player.state.error.stage.control"
  | "player.state.error.action.unsupported"
  | "player.state.error.action.corrupt"
  | "player.state.error.action.inaccessible"
  | "player.state.error.action.network"
  | "player.state.error.action.decoder"
  | "player.state.error.action.device"
  | "player.state.error.action.resource-limit"
  | "player.state.error.action.invalid-argument"
  | "player.state.error.action.out-of-range"
  | "player.state.error.action.not-found"
  | "player.state.error.action.not-ready"
  | "player.state.error.action.not-seekable"
  | "player.state.error.action.canceled"
  | "player.state.error.action.feature-not-exposed"
  | "player.state.error.action.unsupported-for-host"
  | "player.state.error.action.permission-denied"
  | "player.state.error.action.io"
  | "player.state.error.action.internal"
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
  "screen.player": "Player",
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
  // --- player screen shell (card 4.C1) ---
  "player.canvas.label": "Video canvas",
  "player.canvas.placeholder": "Placeholder: video output is not connected yet — engine wiring lands with card 4.H1.",
  "player.region.title-bar": "Player title bar",
  "player.region.controls-bar": "Playback controls",
  "player.region.library": "Library",
  "player.region.playlist": "Playlist",
  "player.region.media-info": "Media information",
  "player.panel.placeholder": "Placeholder panel container — content is implemented by a later card.",
  "player.controls.placeholder": "Placeholder region — transport controls land with card 4.C2.",
  "player.menu.app": "Application menu",
  "player.empty.title": "No media open",
  "player.empty.body": "Open media to start playback.",
  "player.empty.open": "Open media",
  "player.controls.play": "Play",
  "player.controls.pause": "Pause",
  "player.controls.stop": "Stop",
  "player.controls.restart": "Restart playback",
  "player.controls.buffering-hint": "Buffering…",
  "player.controls.seekbar": "Seek",
  "player.controls.volume": "Volume",
  "player.controls.mute": "Mute",
  "player.controls.unmute": "Unmute",
  "player.controls.duration-unknown": "--:--",
  "player.controls.group-transport": "Playback controls",
  "player.controls.group-volume": "Audio controls",
  "player.controls.group-tracks": "Track selection",
  "player.controls.reason.no-media": "no media open",
  "player.controls.reason.not-seekable": "media is not seekable",
  "player.controls.reason.unknown": "unavailable",
  "player.track.audio": "Audio track",
  "player.track.video": "Video track",
  "player.track.subtitle": "Subtitle track",
  "player.track.unknown": "Unknown",
  "player.track.auto": "Automatic",
  "player.track.disable": "Disabled",
  "player.error.title": "Playback error",
  "player.error.dismiss": "Dismiss",
  // --- playback UX states (card 4.C3, doc 08 §5.3/§5.4) ---
  "player.state.loading.label": "Opening media…",
  "player.state.buffering.label": "Buffering…",
  "player.state.unsupported.title": "Format not supported",
  "player.state.unsupported.body": "No playback was attempted.",
  "player.state.unsupported.container-label": "Container",
  "player.state.unsupported.codec-label": "Codec",
  "player.state.unsupported.no-format": "The engine reported no format details for this source.",
  "player.state.error.title": "Playback error",
  "player.state.error.stage-label": "Failed at stage",
  "player.state.error.stage.unknown": "unknown",
  "player.state.error.unknown": "Playback failed. The engine provided no error details.",
  "player.state.error.retry": "Try again",
  "player.state.error.stage.session": "session",
  "player.state.error.stage.input": "input",
  "player.state.error.stage.probe": "probe",
  "player.state.error.stage.demux": "demux",
  "player.state.error.stage.decode": "decode",
  "player.state.error.stage.audio-output": "audio output",
  "player.state.error.stage.video-output": "video output",
  "player.state.error.stage.subtitle": "subtitle",
  "player.state.error.stage.renderer": "renderer",
  "player.state.error.stage.playlist": "playlist",
  "player.state.error.stage.config": "config",
  "player.state.error.stage.plugin": "plugin",
  "player.state.error.stage.control": "control",
  "player.state.error.action.unsupported": "Open a different file, or install a decoder that supports this format.",
  "player.state.error.action.corrupt": "The file appears damaged — try re-downloading or repairing it.",
  "player.state.error.action.inaccessible": "Check that the source exists and Lumen has permission to read it.",
  "player.state.error.action.network": "Check the connection, then try again.",
  "player.state.error.action.decoder": "Try again, or switch the hardware-decode policy to software-only in Settings.",
  "player.state.error.action.device": "Check the audio/video output device, then try again.",
  "player.state.error.action.resource-limit": "Close other sessions or raise the resource limits in Settings, then try again.",
  "player.state.error.action.invalid-argument": "This action is not valid in the current playback state.",
  "player.state.error.action.out-of-range": "The requested value is outside the allowed range.",
  "player.state.error.action.not-found": "The requested item no longer exists.",
  "player.state.error.action.not-ready": "No media is open yet — open media first.",
  "player.state.error.action.not-seekable": "This media cannot be seeked (it may be a live stream).",
  "player.state.error.action.canceled": "The action was superseded by a newer one.",
  "player.state.error.action.feature-not-exposed": "This feature is disabled in the current build.",
  "player.state.error.action.unsupported-for-host": "This host cannot open that source type.",
  "player.state.error.action.permission-denied": "Lumen is not authorized to perform this action.",
  "player.state.error.action.io": "A read or write error occurred — check the file and storage, then try again.",
  "player.state.error.action.internal": "An internal error occurred — try again; if it persists, check the logs.",
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

/** Runtime guard for config-provided label keys (doc 07 §7.2 validator rule:
 * every `label_key` must resolve — the consumption side fails loudly on an
 * unknown key instead of rendering an empty label). */
export function isMessageKey(key: string): key is MessageKey {
  return Object.prototype.hasOwnProperty.call(messages, key);
}

// Scaffold stand-in for the generated i18n hook. Hook-shaped on purpose so
// call sites do not change when the real locale-pack implementation lands.
export function useMsg(): Msg {
  return (key) => messages[key];
}
