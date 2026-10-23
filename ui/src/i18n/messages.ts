// Message catalog + useMsg() for the Phase-4 scaffold.
//
// Doc 07 §6.5 / ui README: user-visible text must flow through useMsg() +
// MessageKey — the `lumen/i18n-no-literals` lint rule (eslint.config.js)
// bans raw JSX literals. The real generated locale packs arrive with the i18n
// pipeline (doc 07); this hand-written table is the scaffold stand-in and
// every entry honestly labels its screen as a placeholder.
//
// Card 4.H3 adds the locale-selection seam: `setActiveLocale("pseudo")`
// switches useMsg() to a deterministic transform of this catalog (./pseudo.ts)
// for clipping/hardcode proofs. Integrity (coverage, duplicates, key grammar,
// placeholder grammar) is gated by `npm run l10n:check`
// (ui/scripts/l10n-check.mjs).

import { PSEUDO_LOCALE_TAG, toPseudo } from "./pseudo";

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
  // --- player variants (card 4.F2: miniplayer + music mode) ---
  | "player.variant.miniplayer.label"
  | "player.variant.miniplayer.controls"
  | "player.variant.miniplayer.expand"
  | "player.variant.miniplayer.close"
  | "player.art.label"
  | "player.art.placeholder"
  | "player.music.waveform-placeholder"
  // --- control bar (card 4.C2) ---
  | "player.controls.play"
  | "player.controls.pause"
  | "player.controls.stop"
  | "player.controls.restart"
  | "player.controls.buffering-hint"
  | "player.controls.seekbar"
  | "player.controls.volume"
  | "player.controls.mute"
  // --- playlist panel + queue (card 4.D1) ---
  | "playlist.panel.title"
  | "playlist.panel.count-suffix"
  | "playlist.panel.empty-title"
  | "playlist.panel.empty-body"
  | "playlist.panel.not-exposed"
  | "playlist.panel.load-handoff"
  // --- customize panels (card 4.F3) ---
  | "player.region.customize"
  | "customize.panel.crop-aspect"
  | "customize.panel.video-colors"
  | "customize.panel.essential"
  | "customize.panel.hdr"
  | "customize.field.aspect"
  | "customize.field.crop"
  | "customize.field.zoom"
  | "customize.field.brightness"
  | "customize.field.contrast"
  | "customize.field.saturation"
  | "customize.field.hue"
  | "customize.field.gamma"
  | "customize.field.speed"
  | "customize.field.tonemap-mode"
  | "customize.field.tonemap-strength"
  | "customize.option.default"
  | "customize.option.none"
  | "customize.ratio.1-1"
  | "customize.ratio.4-3"
  | "customize.ratio.16-9"
  | "customize.ratio.16-10"
  | "customize.ratio.2-35-1"
  | "customize.ratio.2-39-1"
  | "customize.repeat.off"
  | "customize.repeat.all"
  | "customize.repeat.one"
  | "customize.tonemap.clip"
  | "customize.tonemap.mobius"
  | "customize.tonemap.reinhard"
  | "customize.tonemap.hable"
  | "customize.action.seek-back-10"
  | "customize.action.seek-fwd-10"
  | "customize.action.seek-back-60"
  | "customize.action.seek-fwd-60"
  | "customize.action.repeat"
  | "customize.action.shuffle"
  | "customize.none-available"
  | "playlist.controls.prev"
  | "playlist.controls.next"
  | "playlist.controls.shuffle"
  | "playlist.controls.repeat.off"
  | "playlist.controls.repeat.one"
  | "playlist.controls.repeat.all"
  | "playlist.controls.clear"
  | "playlist.entry.current"
  | "playlist.entry.unknown-title"
  | "playlist.entry.unknown-duration"
  | "playlist.entry.play-now"
  | "playlist.entry.play-next"
  | "playlist.entry.play-later"
  | "playlist.entry.remove"
  // --- media info screen (card 4.D2) ---
  | "screen.media-info"
  | "media-info.panel.title"
  | "media-info.unknown"
  | "media-info.section.general"
  | "media-info.section.video"
  | "media-info.section.audio"
  | "media-info.section.subtitle"
  | "media-info.section.metadata"
  | "media-info.field.title"
  | "media-info.field.source"
  | "media-info.field.container"
  | "media-info.field.duration"
  | "media-info.field.size"
  | "media-info.field.bitrate"
  | "media-info.field.live"
  | "media-info.field.seekable"
  | "media-info.field.chapters"
  | "media-info.field.codec"
  | "media-info.field.profile"
  | "media-info.field.level"
  | "media-info.field.dimensions"
  | "media-info.field.frame-rate"
  | "media-info.field.bit-depth"
  | "media-info.field.hdr"
  | "media-info.field.channels"
  | "media-info.field.sample-rate"
  | "media-info.field.codec-params"
  | "media-info.field.language"
  | "media-info.field.name"
  | "media-info.field.origin"
  | "media-info.stream.video"
  | "media-info.stream.audio"
  | "media-info.stream.subtitle"
  | "media-info.origin.stream"
  | "media-info.origin.external"
  | "media-info.value.yes"
  | "media-info.value.no"
  | "media-info.duration.infinite"
  | "media-info.metadata.none"
  | "media-info.empty.title"
  | "media-info.empty.body"
  | "media-info.panel.not-exposed"
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
  | "settings.value.playback-hw-decode-policy.hardware-only"
  | "settings.area.hotkeys"
  | "settings.section.hotkeys.bindings"
  | "settings.hotkeys.column.action"
  | "settings.hotkeys.column.key"
  | "settings.hotkeys.column.command"
  | "settings.hotkeys.column.mode"
  | "settings.hotkeys.conflict"
  | "settings.hotkeys.edit-hint"
  | "settings.hotkeys.reset"
  | "settings.hotkeys.apply"
  | "settings.hotkeys.revert"
  | "settings.hotkeys.defaults-note"
  | "settings.locale.en-us"
  | "settings.appearance.theme-note"
  | "hotkeys.about"
  | "hotkeys.advanced-controls"
  | "hotkeys.aspect-ratio-cycle"
  | "hotkeys.audio-delay-down"
  | "hotkeys.audio-delay-up"
  | "hotkeys.audio-device-cycle"
  | "hotkeys.audio-track-cycle"
  | "hotkeys.autoscale-toggle"
  | "hotkeys.bookmarks"
  | "hotkeys.chapter-next"
  | "hotkeys.chapter-prev"
  | "hotkeys.clear-playlist"
  | "hotkeys.codec-info"
  | "hotkeys.convert"
  | "hotkeys.crop-cycle"
  | "hotkeys.crop-one-px-bottom"
  | "hotkeys.crop-one-px-left"
  | "hotkeys.crop-one-px-right"
  | "hotkeys.crop-one-px-top"
  | "hotkeys.deinterlace-mode-cycle"
  | "hotkeys.deinterlace-toggle"
  | "hotkeys.disc-menu"
  | "hotkeys.dock-playlist"
  | "hotkeys.effects-and-filters"
  | "hotkeys.exit-fullscreen"
  | "hotkeys.file-browser-toggle"
  | "hotkeys.frame-next"
  | "hotkeys.frame-prev"
  | "hotkeys.fullscreen"
  | "hotkeys.hide-ui"
  | "hotkeys.history-back"
  | "hotkeys.jump-to-time"
  | "hotkeys.load-playlist"
  | "hotkeys.media-info"
  | "hotkeys.messages"
  | "hotkeys.minimal-view"
  | "hotkeys.mute"
  | "hotkeys.nav-activate"
  | "hotkeys.nav-down"
  | "hotkeys.nav-left"
  | "hotkeys.nav-right"
  | "hotkeys.nav-up"
  | "hotkeys.next-item"
  | "hotkeys.next-track"
  | "hotkeys.open-capture-device"
  | "hotkeys.open-disc"
  | "hotkeys.open-file"
  | "hotkeys.open-folder"
  | "hotkeys.open-location-clipboard"
  | "hotkeys.open-network-stream"
  | "hotkeys.pause"
  | "hotkeys.play"
  | "hotkeys.play-bookmark-1"
  | "hotkeys.play-bookmark-10"
  | "hotkeys.play-bookmark-2"
  | "hotkeys.play-bookmark-3"
  | "hotkeys.play-bookmark-4"
  | "hotkeys.play-bookmark-5"
  | "hotkeys.play-bookmark-6"
  | "hotkeys.play-bookmark-7"
  | "hotkeys.play-bookmark-8"
  | "hotkeys.play-bookmark-9"
  | "hotkeys.play-pause"
  | "hotkeys.playlist-panel"
  | "hotkeys.preferences"
  | "hotkeys.prev-item"
  | "hotkeys.prev-track"
  | "hotkeys.program-next"
  | "hotkeys.program-prev"
  | "hotkeys.queue-toggle"
  | "hotkeys.quit"
  | "hotkeys.rate-faster"
  | "hotkeys.rate-faster-fine"
  | "hotkeys.rate-normal"
  | "hotkeys.rate-slower"
  | "hotkeys.rate-slower-fine"
  | "hotkeys.record"
  | "hotkeys.save-playlist"
  | "hotkeys.scale-down"
  | "hotkeys.scale-up"
  | "hotkeys.seek-backward"
  | "hotkeys.seek-backward-extrashort"
  | "hotkeys.seek-backward-long"
  | "hotkeys.seek-backward-medium"
  | "hotkeys.seek-forward"
  | "hotkeys.seek-forward-extrashort"
  | "hotkeys.seek-forward-long"
  | "hotkeys.seek-forward-medium"
  | "hotkeys.set-bookmark-1"
  | "hotkeys.set-bookmark-10"
  | "hotkeys.set-bookmark-2"
  | "hotkeys.set-bookmark-3"
  | "hotkeys.set-bookmark-4"
  | "hotkeys.set-bookmark-5"
  | "hotkeys.set-bookmark-6"
  | "hotkeys.set-bookmark-7"
  | "hotkeys.set-bookmark-8"
  | "hotkeys.set-bookmark-9"
  | "hotkeys.show-controls-fullscreen"
  | "hotkeys.show-position"
  | "hotkeys.shuffle"
  | "hotkeys.snapshot"
  | "hotkeys.stop"
  | "hotkeys.subsync-apply"
  | "hotkeys.subsync-mark-audio"
  | "hotkeys.subsync-mark-sub"
  | "hotkeys.subsync-reset"
  | "hotkeys.subtitle-delay-down"
  | "hotkeys.subtitle-delay-up"
  | "hotkeys.subtitle-scale-down"
  | "hotkeys.subtitle-scale-normal"
  | "hotkeys.subtitle-scale-up"
  | "hotkeys.subtitle-toggle"
  | "hotkeys.subtitle-track-cycle"
  | "hotkeys.subtitle-track-cycle-reverse"
  | "hotkeys.title-next"
  | "hotkeys.title-prev"
  | "hotkeys.uncrop-one-px-bottom"
  | "hotkeys.uncrop-one-px-left"
  | "hotkeys.uncrop-one-px-right"
  | "hotkeys.uncrop-one-px-top"
  | "hotkeys.unzoom"
  | "hotkeys.viewpoint-fov-in"
  | "hotkeys.viewpoint-fov-out"
  | "hotkeys.vlm"
  | "hotkeys.volume-down"
  | "hotkeys.volume-up"
  | "hotkeys.wallpaper"
  | "hotkeys.zoom-cycle"
  | "hotkeys.zoom-double"
  | "hotkeys.zoom-half"
  | "hotkeys.zoom-original"
  | "hotkeys.zoom-quarter"
  // Settings group 2 areas (card 4.E3) — audio/video/subtitles screens.
  | "settings.section.audio.output"
  | "settings.audio.output-device"
  | "settings.audio.device-list-loading"
  | "settings.audio.device-enumeration-unavailable"
  | "settings.audio.no-devices"
  | "settings.audio.device-default"
  | "settings.capability-unavailable"
  | "settings.area.empty"
  // Settings group 3 areas (card 4.E4) — codecs / library / network / about / advanced.
  | "settings.area.codecs"
  | "settings.section.codecs.decode"
  | "settings.section.codecs.cache"
  | "settings.codecs.hw-decode-unavailable"
  | "settings.area.library"
  | "settings.area.network"
  | "settings.area.about"
  | "settings.section.about.updates"
  | "settings.section.about.about"
  | "settings.about.product"
  | "settings.about.ui-package"
  | "settings.about.version"
  | "settings.area.advanced"
  | "settings.advanced.fold-show"
  | "settings.advanced.fold-hide"
  | "settings.advanced.intro"
  // --- onboarding wizard (card 4.F1; privacy copy is an honest
  // [PROPOSAL] placeholder — the design package has no privacy artboard,
  // wording follows doc 07 §2.6/§5.4) ---
  | "onboarding.welcome.title"
  | "onboarding.welcome.body"
  | "onboarding.privacy.title"
  | "onboarding.privacy.body"
  | "onboarding.privacy.note"
  | "onboarding.privacy.telemetry.label"
  | "onboarding.privacy.telemetry.description"
  | "onboarding.privacy.update-checks.label"
  | "onboarding.privacy.update-checks.description"
  | "onboarding.privacy.online-suggestions.label"
  | "onboarding.privacy.online-suggestions.description"
  | "onboarding.appearance.title"
  | "onboarding.appearance.body"
  | "onboarding.appearance.theme.light"
  | "onboarding.appearance.theme.dark"
  | "onboarding.appearance.theme.system"
  | "onboarding.appearance.accent"
  | "onboarding.accent.blue"
  | "onboarding.accent.cyan"
  | "onboarding.accent.gray"
  | "onboarding.accent.green"
  | "onboarding.accent.orange"
  | "onboarding.accent.pink"
  | "onboarding.accent.red"
  | "onboarding.accent.violet"
  | "onboarding.accent.yellow"
  | "onboarding.shortcuts.title"
  | "onboarding.shortcuts.body"
  | "onboarding.shortcuts.hint-play-pause"
  | "onboarding.shortcuts.hint-seek"
  | "onboarding.shortcuts.hint-fullscreen"
  | "onboarding.shortcuts.hint-more"
  | "onboarding.done.title"
  | "onboarding.done.body"
  | "onboarding.action.back"
  | "onboarding.action.next"
  | "onboarding.action.skip"
  | "onboarding.action.get-started"
  | "onboarding.progress.label"
  | "onboarding.progress.of";

// The en-US reference catalog (doc 07 §6.3) — exported for the pseudo
// generator, tests, and l10n tooling.
export const messages: Readonly<Record<MessageKey, string>> = {
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
  "player.variant.miniplayer.label": "Miniplayer",
  "player.variant.miniplayer.controls": "Miniplayer controls",
  "player.variant.miniplayer.expand": "Expand to full player",
  "player.variant.miniplayer.close": "Close miniplayer",
  "player.art.label": "Album art",
  "player.art.placeholder": "Album art placeholder — artwork renders once the engine reports it.",
  "player.music.waveform-placeholder":
    "Waveform seek appears when the engine reports audio levels; the standard seekbar is shown meanwhile.",
  "player.controls.play": "Play",
  "player.controls.pause": "Pause",
  "player.controls.stop": "Stop",
  "player.controls.restart": "Restart playback",
  "player.controls.buffering-hint": "Buffering…",
  "player.controls.seekbar": "Seek",
  "player.controls.volume": "Volume",
  "player.controls.mute": "Mute",
  // --- playlist panel + queue (card 4.D1) ---
  "playlist.panel.title": "Playlist",
  "playlist.panel.count-suffix": "items",
  "playlist.panel.empty-title": "Nothing in the queue",
  "playlist.panel.empty-body": "Files you open are listed here in order.",
  "playlist.panel.not-exposed": "Playlist commands are not exposed by this build — showing seeded data only.",
  "playlist.panel.load-handoff": "Opening playlist files (.m3u) is wired by the app shell (card 4.H1).",
  // --- media info screen (card 4.D2) ---
  "screen.media-info": "Media information",
  "media-info.panel.title": "Media information",
  "media-info.unknown": "Unknown",
  "media-info.section.general": "General",
  "media-info.section.video": "Video",
  "media-info.section.audio": "Audio",
  "media-info.section.subtitle": "Subtitles",
  "media-info.section.metadata": "Metadata",
  "media-info.field.title": "Title",
  "media-info.field.source": "Source",
  "media-info.field.container": "Container",
  "media-info.field.duration": "Duration",
  "media-info.field.size": "Size",
  "media-info.field.bitrate": "Bitrate",
  "media-info.field.live": "Live",
  "media-info.field.seekable": "Seekable",
  "media-info.field.chapters": "Chapters",
  "media-info.field.codec": "Codec",
  "media-info.field.profile": "Profile",
  "media-info.field.level": "Level",
  "media-info.field.dimensions": "Dimensions",
  "media-info.field.frame-rate": "Frame rate",
  "media-info.field.bit-depth": "Bit depth",
  "media-info.field.hdr": "HDR",
  "media-info.field.channels": "Channels",
  "media-info.field.sample-rate": "Sample rate",
  "media-info.field.codec-params": "Codec parameters",
  "media-info.field.language": "Language",
  "media-info.field.name": "Name",
  "media-info.field.origin": "Origin",
  "media-info.stream.video": "Video stream",
  "media-info.stream.audio": "Audio stream",
  "media-info.stream.subtitle": "Subtitle stream",
  "media-info.origin.stream": "Container stream",
  "media-info.origin.external": "External file",
  "media-info.value.yes": "Yes",
  "media-info.value.no": "No",
  "media-info.duration.infinite": "∞ (live)",
  "media-info.metadata.none": "No metadata tags reported.",
  "media-info.empty.title": "No media open",
  "media-info.empty.body": "Media information appears once a media file or stream is open.",
  "media-info.panel.not-exposed": "Media information is not available — the command layer did not expose it.",
  // --- customize panels (card 4.F3) ---
  "player.region.customize": "Customize",
  "customize.panel.crop-aspect": "Crop & aspect ratio",
  "customize.panel.video-colors": "Video colors",
  "customize.panel.essential": "Essential",
  "customize.panel.hdr": "HDR options",
  "customize.field.aspect": "Aspect ratio",
  "customize.field.crop": "Crop",
  "customize.field.zoom": "Zoom",
  "customize.field.brightness": "Brightness",
  "customize.field.contrast": "Contrast",
  "customize.field.saturation": "Saturation",
  "customize.field.hue": "Hue",
  "customize.field.gamma": "Gamma",
  "customize.field.speed": "Playback speed",
  "customize.field.tonemap-mode": "Tone mapping",
  "customize.field.tonemap-strength": "Tone-mapping strength",
  "customize.option.default": "Default",
  "customize.option.none": "None",
  "customize.ratio.1-1": "1:1",
  "customize.ratio.4-3": "4:3",
  "customize.ratio.16-9": "16:9",
  "customize.ratio.16-10": "16:10",
  "customize.ratio.2-35-1": "2.35:1",
  "customize.ratio.2-39-1": "2.39:1",
  "customize.repeat.off": "Off",
  "customize.repeat.all": "All",
  "customize.repeat.one": "One",
  "customize.tonemap.clip": "Clip",
  "customize.tonemap.mobius": "Mobius",
  "customize.tonemap.reinhard": "Reinhard",
  "customize.tonemap.hable": "Hable",
  "customize.action.seek-back-10": "-10 s",
  "customize.action.seek-fwd-10": "+10 s",
  "customize.action.seek-back-60": "-60 s",
  "customize.action.seek-fwd-60": "+60 s",
  "customize.action.repeat": "Repeat",
  "customize.action.shuffle": "Shuffle",
  "customize.none-available": "No customize controls are supported by the connected engine.",
  "playlist.controls.prev": "Previous item",
  "playlist.controls.next": "Next item",
  "playlist.controls.shuffle": "Shuffle",
  "playlist.controls.repeat.off": "Repeat: off",
  "playlist.controls.repeat.one": "Repeat: one",
  "playlist.controls.repeat.all": "Repeat: all",
  "playlist.controls.clear": "Clear playlist",
  "playlist.entry.current": "Now playing",
  "playlist.entry.unknown-title": "Unknown title",
  "playlist.entry.unknown-duration": "—",
  "playlist.entry.play-now": "Play now",
  "playlist.entry.play-next": "Play next",
  "playlist.entry.play-later": "Play later",
  "playlist.entry.remove": "Remove from queue",
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
  // Hotkeys editor + settings-area extensions (card 4.E2). Copy for the
  // hotkeys.* rows is derived from the keymaps/default.toml label-msgids —
  // replace with reference-catalog copy when the i18n pipeline lands (doc 07 §6).
  "settings.area.hotkeys": "Hotkeys",
  "settings.section.hotkeys.bindings": "Keyboard shortcuts",
  "settings.hotkeys.column.action": "Action",
  "settings.hotkeys.column.key": "Key",
  "settings.hotkeys.column.command": "Command",
  "settings.hotkeys.column.mode": "Mode",
  "settings.hotkeys.conflict": "Key combination already in use",
  "settings.hotkeys.edit-hint": "Type a key combination, e.g. Ctrl+K",
  "settings.hotkeys.reset": "Reset to default",
  "settings.hotkeys.apply": "Apply",
  "settings.hotkeys.revert": "Revert",
  "settings.hotkeys.defaults-note": "Bindings are seeded from the default keymap (keymaps/default.toml).",
  "settings.locale.en-us": "English (United States)",
  "settings.appearance.theme-note": "Theme changes apply instantly for preview.",
  "hotkeys.about": "About",
  "hotkeys.advanced-controls": "Advanced Controls",
  "hotkeys.aspect-ratio-cycle": "Aspect Ratio Cycle",
  "hotkeys.audio-delay-down": "Audio Delay Down",
  "hotkeys.audio-delay-up": "Audio Delay Up",
  "hotkeys.audio-device-cycle": "Audio Device Cycle",
  "hotkeys.audio-track-cycle": "Audio Track Cycle",
  "hotkeys.autoscale-toggle": "Autoscale Toggle",
  "hotkeys.bookmarks": "Bookmarks",
  "hotkeys.chapter-next": "Chapter Next",
  "hotkeys.chapter-prev": "Chapter Prev",
  "hotkeys.clear-playlist": "Clear Playlist",
  "hotkeys.codec-info": "Codec Info",
  "hotkeys.convert": "Convert",
  "hotkeys.crop-cycle": "Crop Cycle",
  "hotkeys.crop-one-px-bottom": "Crop One Px Bottom",
  "hotkeys.crop-one-px-left": "Crop One Px Left",
  "hotkeys.crop-one-px-right": "Crop One Px Right",
  "hotkeys.crop-one-px-top": "Crop One Px Top",
  "hotkeys.deinterlace-mode-cycle": "Deinterlace Mode Cycle",
  "hotkeys.deinterlace-toggle": "Deinterlace Toggle",
  "hotkeys.disc-menu": "Disc Menu",
  "hotkeys.dock-playlist": "Dock Playlist",
  "hotkeys.effects-and-filters": "Effects And Filters",
  "hotkeys.exit-fullscreen": "Exit Fullscreen",
  "hotkeys.file-browser-toggle": "File Browser Toggle",
  "hotkeys.frame-next": "Frame Next",
  "hotkeys.frame-prev": "Frame Prev",
  "hotkeys.fullscreen": "Fullscreen",
  "hotkeys.hide-ui": "Hide Ui",
  "hotkeys.history-back": "History Back",
  "hotkeys.jump-to-time": "Jump To Time",
  "hotkeys.load-playlist": "Load Playlist",
  "hotkeys.media-info": "Media Info",
  "hotkeys.messages": "Messages",
  "hotkeys.minimal-view": "Minimal View",
  "hotkeys.mute": "Mute",
  "hotkeys.nav-activate": "Nav Activate",
  "hotkeys.nav-down": "Nav Down",
  "hotkeys.nav-left": "Nav Left",
  "hotkeys.nav-right": "Nav Right",
  "hotkeys.nav-up": "Nav Up",
  "hotkeys.next-item": "Next Item",
  "hotkeys.next-track": "Next Track",
  "hotkeys.open-capture-device": "Open Capture Device",
  "hotkeys.open-disc": "Open Disc",
  "hotkeys.open-file": "Open File",
  "hotkeys.open-folder": "Open Folder",
  "hotkeys.open-location-clipboard": "Open Location Clipboard",
  "hotkeys.open-network-stream": "Open Network Stream",
  "hotkeys.pause": "Pause",
  "hotkeys.play": "Play",
  "hotkeys.play-bookmark-1": "Play Bookmark 1",
  "hotkeys.play-bookmark-10": "Play Bookmark 10",
  "hotkeys.play-bookmark-2": "Play Bookmark 2",
  "hotkeys.play-bookmark-3": "Play Bookmark 3",
  "hotkeys.play-bookmark-4": "Play Bookmark 4",
  "hotkeys.play-bookmark-5": "Play Bookmark 5",
  "hotkeys.play-bookmark-6": "Play Bookmark 6",
  "hotkeys.play-bookmark-7": "Play Bookmark 7",
  "hotkeys.play-bookmark-8": "Play Bookmark 8",
  "hotkeys.play-bookmark-9": "Play Bookmark 9",
  "hotkeys.play-pause": "Play Pause",
  "hotkeys.playlist-panel": "Playlist Panel",
  "hotkeys.preferences": "Preferences",
  "hotkeys.prev-item": "Prev Item",
  "hotkeys.prev-track": "Prev Track",
  "hotkeys.program-next": "Program Next",
  "hotkeys.program-prev": "Program Prev",
  "hotkeys.queue-toggle": "Queue Toggle",
  "hotkeys.quit": "Quit",
  "hotkeys.rate-faster": "Rate Faster",
  "hotkeys.rate-faster-fine": "Rate Faster Fine",
  "hotkeys.rate-normal": "Rate Normal",
  "hotkeys.rate-slower": "Rate Slower",
  "hotkeys.rate-slower-fine": "Rate Slower Fine",
  "hotkeys.record": "Record",
  "hotkeys.save-playlist": "Save Playlist",
  "hotkeys.scale-down": "Scale Down",
  "hotkeys.scale-up": "Scale Up",
  "hotkeys.seek-backward": "Seek Backward",
  "hotkeys.seek-backward-extrashort": "Seek Backward Extrashort",
  "hotkeys.seek-backward-long": "Seek Backward Long",
  "hotkeys.seek-backward-medium": "Seek Backward Medium",
  "hotkeys.seek-forward": "Seek Forward",
  "hotkeys.seek-forward-extrashort": "Seek Forward Extrashort",
  "hotkeys.seek-forward-long": "Seek Forward Long",
  "hotkeys.seek-forward-medium": "Seek Forward Medium",
  "hotkeys.set-bookmark-1": "Set Bookmark 1",
  "hotkeys.set-bookmark-10": "Set Bookmark 10",
  "hotkeys.set-bookmark-2": "Set Bookmark 2",
  "hotkeys.set-bookmark-3": "Set Bookmark 3",
  "hotkeys.set-bookmark-4": "Set Bookmark 4",
  "hotkeys.set-bookmark-5": "Set Bookmark 5",
  "hotkeys.set-bookmark-6": "Set Bookmark 6",
  "hotkeys.set-bookmark-7": "Set Bookmark 7",
  "hotkeys.set-bookmark-8": "Set Bookmark 8",
  "hotkeys.set-bookmark-9": "Set Bookmark 9",
  "hotkeys.show-controls-fullscreen": "Show Controls Fullscreen",
  "hotkeys.show-position": "Show Position",
  "hotkeys.shuffle": "Shuffle",
  "hotkeys.snapshot": "Snapshot",
  "hotkeys.stop": "Stop",
  "hotkeys.subsync-apply": "Subsync Apply",
  "hotkeys.subsync-mark-audio": "Subsync Mark Audio",
  "hotkeys.subsync-mark-sub": "Subsync Mark Sub",
  "hotkeys.subsync-reset": "Subsync Reset",
  "hotkeys.subtitle-delay-down": "Subtitle Delay Down",
  "hotkeys.subtitle-delay-up": "Subtitle Delay Up",
  "hotkeys.subtitle-scale-down": "Subtitle Scale Down",
  "hotkeys.subtitle-scale-normal": "Subtitle Scale Normal",
  "hotkeys.subtitle-scale-up": "Subtitle Scale Up",
  "hotkeys.subtitle-toggle": "Subtitle Toggle",
  "hotkeys.subtitle-track-cycle": "Subtitle Track Cycle",
  "hotkeys.subtitle-track-cycle-reverse": "Subtitle Track Cycle Reverse",
  "hotkeys.title-next": "Title Next",
  "hotkeys.title-prev": "Title Prev",
  "hotkeys.uncrop-one-px-bottom": "Uncrop One Px Bottom",
  "hotkeys.uncrop-one-px-left": "Uncrop One Px Left",
  "hotkeys.uncrop-one-px-right": "Uncrop One Px Right",
  "hotkeys.uncrop-one-px-top": "Uncrop One Px Top",
  "hotkeys.unzoom": "Unzoom",
  "hotkeys.viewpoint-fov-in": "Viewpoint Fov In",
  "hotkeys.viewpoint-fov-out": "Viewpoint Fov Out",
  "hotkeys.vlm": "Vlm",
  "hotkeys.volume-down": "Volume Down",
  "hotkeys.volume-up": "Volume Up",
  "hotkeys.wallpaper": "Wallpaper",
  "hotkeys.zoom-cycle": "Zoom Cycle",
  "hotkeys.zoom-double": "Zoom Double",
  "hotkeys.zoom-half": "Zoom Half",
  "hotkeys.zoom-original": "Zoom Original",
  "hotkeys.zoom-quarter": "Zoom Quarter",
  // Settings group 2 areas (card 4.E3).
  "settings.section.audio.output": "Output",
  "settings.audio.output-device": "Output device",
  "settings.audio.device-list-loading": "Checking for available audio devices…",
  "settings.audio.device-enumeration-unavailable": "Device enumeration is not available",
  "settings.audio.no-devices": "No audio devices found",
  "settings.audio.device-default": "default",
  "settings.capability-unavailable": "Not available in this build",
  "settings.area.empty": "No options in this area yet.",
  // Settings group 3 areas (card 4.E4).
  "settings.area.codecs": "Input & Codecs",
  "settings.section.codecs.decode": "Decoding",
  "settings.section.codecs.cache": "Cache",
  "settings.codecs.hw-decode-unavailable":
    "Hardware decode is accepted policy (ADR-001) but ships in a later phase — disabled while the feature is not exposed (F2).",
  "settings.area.library": "Playlist & Services",
  "settings.area.network": "Network",
  "settings.area.about": "Updates & About",
  "settings.section.about.updates": "Updates",
  "settings.section.about.about": "About",
  "settings.about.product": "Product",
  "settings.about.ui-package": "UI package",
  "settings.about.version": "Version",
  "settings.area.advanced": "Advanced",
  "settings.advanced.fold-show": "Show advanced settings",
  "settings.advanced.fold-hide": "Hide advanced settings",
  "settings.advanced.intro": "Advanced options from every screen, folded by default.",
  // Onboarding wizard (card 4.F1).
  "onboarding.welcome.title": "Welcome to Lumen",
  "onboarding.welcome.body":
    "A local-first media player. This short walkthrough sets your privacy choices and look — you can skip it and change everything later in Settings.",
  "onboarding.privacy.title": "Privacy first",
  "onboarding.privacy.body":
    "Everything below is off by default. Lumen shares nothing unless you turn a switch on.",
  "onboarding.privacy.note":
    "All choices are optional and stay off unless you enable them. You can change them any time in Settings → Privacy.",
  "onboarding.privacy.telemetry.label": "Send anonymous usage statistics",
  "onboarding.privacy.telemetry.description":
    "Off by default. If enabled, anonymous usage reports are sent to help improve Lumen. No media, file names, or identifiers are ever included.",
  "onboarding.privacy.update-checks.label": "Check for updates automatically",
  "onboarding.privacy.update-checks.description":
    "Off by default. If enabled, Lumen periodically asks the update server whether a new version is available.",
  "onboarding.privacy.online-suggestions.label": "Online suggestions",
  "onboarding.privacy.online-suggestions.description":
    "Off by default. If enabled, Lumen may fetch artwork and metadata suggestions from the internet for your library.",
  "onboarding.appearance.title": "Make it yours",
  "onboarding.appearance.body":
    "Pick a theme and an accent color — the preview updates instantly and nothing is locked in.",
  "onboarding.appearance.theme.light": "Light",
  "onboarding.appearance.theme.dark": "Dark",
  "onboarding.appearance.theme.system": "System",
  "onboarding.appearance.accent": "Accent color",
  "onboarding.accent.blue": "Blue",
  "onboarding.accent.cyan": "Cyan",
  "onboarding.accent.gray": "Gray",
  "onboarding.accent.green": "Green",
  "onboarding.accent.orange": "Orange",
  "onboarding.accent.pink": "Pink",
  "onboarding.accent.red": "Red",
  "onboarding.accent.violet": "Violet",
  "onboarding.accent.yellow": "Yellow",
  "onboarding.shortcuts.title": "Handy shortcuts",
  "onboarding.shortcuts.body":
    "Placeholder shortcut tour — the full map lives in the keyboard shortcuts screen.",
  "onboarding.shortcuts.hint-play-pause": "Space — play or pause",
  "onboarding.shortcuts.hint-seek": "Arrow keys — seek and adjust volume",
  "onboarding.shortcuts.hint-fullscreen": "F — toggle fullscreen",
  "onboarding.shortcuts.hint-more": "See all shortcuts under Help → Keyboard shortcuts",
  "onboarding.done.title": "You are set",
  "onboarding.done.body":
    "That is everything. Your privacy choices are exactly what you set — nothing was enabled for you.",
  "onboarding.action.back": "Back",
  "onboarding.action.next": "Next",
  "onboarding.action.skip": "Skip",
  "onboarding.action.get-started": "Get started",
  "onboarding.progress.label": "Onboarding step",
  "onboarding.progress.of": "of",
};

export type Msg = (key: MessageKey) => string;

// --- locale selection (card 4.H3) ------------------------------------------
// en-US is the reference catalog (doc 07 §6.3); `pseudo` is GENERATED from
// it at module load (never a second hand-written catalog — no drift). The
// real locale-pack pipeline (doc 07 §6, crates/localization) replaces this
// scaffold switch; the hook shape stays so call sites do not change.
export type LocaleTag = "en-US" | typeof PSEUDO_LOCALE_TAG;

const pseudoCatalog: Readonly<Record<MessageKey, string>> = Object.fromEntries(
  (Object.entries(messages) as [MessageKey, string][]).map(([k, v]) => [k, toPseudo(v)]),
) as Readonly<Record<MessageKey, string>>;

let active: LocaleTag = "en-US";

/** Select the active catalog ("en-US" | "pseudo"). Unknown tags fall back to
 * the reference catalog — selection can never crash rendering (doc 07 §6.3). */
export function setActiveLocale(tag: string): void {
  active = tag === PSEUDO_LOCALE_TAG ? PSEUDO_LOCALE_TAG : "en-US";
}

/** The currently selected catalog tag (test/bootstrap introspection). */
export function activeLocale(): LocaleTag {
  return active;
}

/** Runtime guard for config-provided label keys (doc 07 §7.2 validator rule:
 * every `label_key` must resolve — the consumption side fails loudly on an
 * unknown key instead of rendering an empty label). */
export function isMessageKey(key: string): key is MessageKey {
  return Object.prototype.hasOwnProperty.call(messages, key);
}

// Scaffold stand-in for the generated i18n hook. Hook-shaped on purpose so
// call sites do not change when the real locale-pack implementation lands.
export function useMsg(): Msg {
  const catalog: Readonly<Record<MessageKey, string>> =
    active === PSEUDO_LOCALE_TAG ? pseudoCatalog : messages;
  // Fallback to the reference catalog keeps the doc 07 §6.3 chain total —
  // a missing entry can never render `undefined`.
  return (key) => catalog[key] ?? messages[key];
}
