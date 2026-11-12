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
  | "screen.discovery"
  | "screen.convert"
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
  | "settings.loading"
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
  // Effects & Tools area (issue #33 / D-012 — audio DSP + video adjust).
  | "settings.area.effects"
  | "settings.section.effects.audio"
  | "settings.section.effects.video"
  | "settings.effects.volume"
  | "settings.effects.mute"
  | "settings.effects.balance"
  | "settings.effects.eq.low"
  | "settings.effects.eq.mid"
  | "settings.effects.eq.high"
  | "settings.effects.brightness"
  | "settings.effects.contrast"
  | "settings.effects.saturation"
  | "settings.effects.hue"
  | "settings.effects.pending-command"
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
  | "onboarding.progress.of"
  // --- Solstice app shell + navigation (2026-11-08 redesign) ---
  | "nav.title"
  | "nav.player"
  | "nav.playlist"
  | "nav.media-info"
  | "nav.shortcuts"
  | "nav.settings"
  | "nav.discovery"
  | "nav.open-menu"
  | "menu.file"
  | "menu.file.open-media"
  | "menu.file.open-network-stream"
  | "menu.file.convert"
  | "menu.file.exit"
  | "menu.playback"
  | "menu.playback.play-pause"
  | "menu.playback.stop"
  | "menu.playback.next"
  | "menu.playback.previous"
  | "menu.playback.shuffle"
  | "menu.playback.repeat"
  | "menu.view"
  | "menu.view.player"
  | "menu.view.music-mode"
  | "menu.view.miniplayer"
  | "menu.view.playlist"
  | "menu.view.media-info"
  | "menu.view.discovery"
  | "menu.view.shortcuts"
  | "menu.view.settings"
  | "menu.tools"
  | "menu.tools.customize"
  | "menu.tools.media-info"
  | "menu.tools.preferences"
  | "menu.help"
  | "menu.help.about"
  | "menu.help.keyboard-shortcuts"
  | "demo.badge"
  | "demo.notice.title"
  | "demo.notice.body"
  // --- open media dialog ---
  | "open.title"
  | "open.subtitle"
  | "open.recent"
  | "open.recent.empty"
  | "open.demo-clips"
  | "open.clip.bunny"
  | "open.clip.ambient"
  | "open.clip.live"
  | "open.clip.radio"
  | "open.action"
  | "open.cancel"
  | "open.close"
  | "open.path.label"
  | "open.path.placeholder"
  | "open.path.invalid"
  // --- open network stream screen ---
  | "stream.title"
  | "stream.subtitle"
  | "stream.url.label"
  | "stream.url.placeholder"
  | "stream.back"
  | "stream.recent"
  | "stream.recent.empty"
  | "stream.open"
  | "stream.invalid"
  // --- discovery screen (card 7.D1, issue #42) ---
  | "discovery.title"
  | "discovery.subtitle"
  | "discovery.volumes.title"
  | "discovery.volumes.count-suffix"
  | "discovery.volumes.empty"
  | "discovery.volumes.live"
  | "discovery.kind.fixed"
  | "discovery.kind.removable"
  | "discovery.kind.optical"
  | "discovery.kind.network"
  | "discovery.network.title"
  | "discovery.network.status"
  // --- convert screen (card 7.E3, issue #44) ---
  | "convert.title"
  | "convert.subtitle"
  | "convert.source.label"
  | "convert.source.placeholder"
  | "convert.source.required"
  | "convert.target.label"
  | "convert.target.wav"
  | "convert.target.wav.note"
  | "convert.target.mp4"
  | "convert.target.mp4.note"
  | "convert.start"
  | "convert.back"
  | "convert.queue.title"
  | "convert.queue.empty"
  | "convert.queue.full"
  | "convert.lane.unavailable"
  | "convert.job.status.queued"
  | "convert.job.status.running"
  | "convert.job.status.completed"
  | "convert.job.status.failed"
  | "convert.job.status.cancelled"
  | "convert.job.status.blocked"
  | "convert.job.cancel"
  | "convert.job.progress"
  | "convert.job.blocked.reason"
  // --- keyboard shortcuts screen ---
  | "shortcuts.title"
  | "shortcuts.subtitle"
  | "shortcuts.search.label"
  | "shortcuts.search.placeholder"
  | "shortcuts.search.empty"
  | "shortcuts.group.playback"
  | "shortcuts.group.navigation"
  | "shortcuts.group.volume"
  | "shortcuts.group.variants"
  | "shortcuts.edit.hint"
  // --- not found ---
  | "not-found.title"
  | "not-found.body"
  | "not-found.action"
  // --- onboarding extras (Solstice) ---
  | "onboarding.shortcuts.hint-volume"
  | "onboarding.shortcuts.hint-mute"
  | "onboarding.shortcuts.hint-playlist"
  | "onboarding.keys.enter"
  | "onboarding.keys.left"
  | "onboarding.keys.right"
  // --- player extras (Solstice) ---
  | "player.keys.strip.label"
  | "player.demo.playing"
  | "player.open.recent"
  | "app.name"
  | "convert.queue.count-suffix"
  | "convert.source.browse"
  | "convert.source.browse-gate"
  | "customize.box.label"
  | "customize.close"
  | "customize.instruction"
  | "customize.save"
  | "customize.tile.backward-10"
  | "customize.tile.backward-15"
  | "customize.tile.forward-10"
  | "customize.tile.forward-15"
  | "customize.title"
  | "discovery.volumes.empty-body"
  | "discovery.volumes.empty-title"
  | "not-found.body-quiet"
  | "not-found.heading"
  | "onboarding.appearance.heading"
  | "onboarding.library.body"
  | "onboarding.library.failed"
  | "onboarding.library.opened"
  | "onboarding.library.title"
  | "onboarding.playback.body"
  | "onboarding.playback.title"
  | "onboarding.shortcuts.frame"
  | "onboarding.shortcuts.heading"
  | "onboarding.shortcuts.item"
  | "onboarding.shortcuts.more"
  | "onboarding.shortcuts.seek"
  | "onboarding.shortcuts.volume"
  | "onboarding.subtitles.body"
  | "onboarding.subtitles.title"
  | "player.controls.cast"
  | "player.controls.exit-fullscreen"
  | "player.controls.fullscreen"
  | "player.controls.group-output"
  | "player.controls.next"
  | "player.mini.queue.label"
  | "player.mini.queue.toggle"
  | "player.mini.title-empty"
  | "player.tools.close"
  | "player.tools.collapse-settings"
  | "player.tools.expand-settings"
  | "playlist.library.count-suffix"
  | "playlist.library.create"
  | "playlist.library.empty-body"
  | "playlist.library.empty-title"
  | "playlist.library.items-suffix"
  | "playlist.library.list-label"
  | "playlist.library.menu"
  | "playlist.library.menu.play"
  | "playlist.library.menu.remove"
  | "playlist.new.browse"
  | "playlist.new.cancel"
  | "playlist.new.close"
  | "playlist.new.details"
  | "playlist.new.details.helper"
  | "playlist.new.drop"
  | "playlist.new.files"
  | "playlist.new.files.helper"
  | "playlist.new.name-placeholder"
  | "playlist.new.save"
  | "playlist.new.thumbnail"
  | "playlist.new.thumbnail.hint"
  | "playlist.new.title"
  | "playlist.options.entry-menu"
  | "playlist.options.grid-label"
  | "playlist.options.list-label"
  | "playlist.options.sort"
  | "playlist.options.sort.album-asc"
  | "playlist.options.sort.album-desc"
  | "playlist.options.sort.artist-asc"
  | "playlist.options.sort.artist-desc"
  | "playlist.options.sort.disc-number-asc"
  | "playlist.options.sort.disc-number-desc"
  | "playlist.options.sort.newest-first"
  | "playlist.options.sort.oldest-first"
  | "playlist.options.sort.title-asc"
  | "playlist.options.sort.title-desc"
  | "playlist.options.sort.track-number-asc"
  | "playlist.options.sort.track-number-desc"
  | "playlist.options.sort.unwatched-first"
  | "playlist.options.sort.uri-asc"
  | "playlist.options.sort.uri-desc"
  | "playlist.options.sort.watched-first"
  | "playlist.options.view"
  | "playlist.options.view.grid"
  | "playlist.options.view.list"
  | "playlist.subtitles.cancel"
  | "playlist.subtitles.close"
  | "playlist.subtitles.destination"
  | "playlist.subtitles.destination.file"
  | "playlist.subtitles.destination.folder"
  | "playlist.subtitles.destination.playlist"
  | "playlist.subtitles.empty-body"
  | "playlist.subtitles.empty-title"
  | "playlist.subtitles.folder-path"
  | "playlist.subtitles.language"
  | "playlist.subtitles.language.english"
  | "playlist.subtitles.list-label"
  | "playlist.subtitles.match"
  | "playlist.subtitles.match.any"
  | "playlist.subtitles.match.contains"
  | "playlist.subtitles.match.exact"
  | "playlist.subtitles.match.extra"
  | "playlist.subtitles.match.off"
  | "playlist.subtitles.pairs-label"
  | "playlist.subtitles.save"
  | "playlist.subtitles.search"
  | "playlist.subtitles.search.placeholder"
  | "playlist.subtitles.source"
  | "playlist.subtitles.source.computer"
  | "playlist.subtitles.source.online"
  | "playlist.subtitles.title"
  | "settings.advanced-options"
  | "settings.area.audio"
  | "settings.area.controls"
  | "settings.area.extensions"
  | "settings.area.recording"
  | "settings.area.subtitles"
  | "settings.area.video"
  | "settings.close"
  | "settings.controls.defaults"
  | "settings.controls.edit-shortcuts"
  | "settings.controls.mouse-double-click"
  | "settings.controls.mouse-horizontal-scroll"
  | "settings.controls.mouse-reverse-scroll-direction"
  | "settings.controls.mouse-scroll-wheel"
  | "settings.controls.mouse-single-click"
  | "settings.controls.reset-all-shortcuts"
  | "settings.controls.reset-shortcuts"
  | "settings.controls.skip-long"
  | "settings.controls.skip-medium"
  | "settings.controls.skip-short"
  | "settings.controls.skip-very-short"
  | "settings.extensions.add"
  | "settings.extensions.empty-body"
  | "settings.extensions.empty-title"
  | "settings.extensions.find-more"
  | "settings.extensions.load-on-startup"
  | "settings.extensions.remove"
  | "settings.extensions.search"
  | "settings.group.app"
  | "settings.group.media"
  | "settings.group.system"
  | "settings.recording.directory"
  | "settings.recording.format"
  | "settings.section.controls.keyboard-shortcuts"
  | "settings.section.controls.mouse"
  | "settings.section.controls.skip-amounts"
  | "settings.section.extensions.add-ons"
  | "settings.section.recording.recording"
  | "settings.value.controls-mouse-double-click.none"
  | "settings.value.controls-mouse-double-click.play-pause"
  | "settings.value.controls-mouse-double-click.show-menu"
  | "settings.value.controls-mouse-double-click.toggle-fullscreen"
  | "settings.value.controls-mouse-horizontal-scroll.none"
  | "settings.value.controls-mouse-horizontal-scroll.seek"
  | "settings.value.controls-mouse-horizontal-scroll.volume"
  | "settings.value.controls-mouse-scroll-wheel.none"
  | "settings.value.controls-mouse-scroll-wheel.seek"
  | "settings.value.controls-mouse-scroll-wheel.volume"
  | "settings.value.controls-mouse-single-click.none"
  | "settings.value.controls-mouse-single-click.play-pause"
  | "settings.value.controls-mouse-single-click.show-menu"
  | "settings.value.controls-mouse-single-click.toggle-fullscreen"
  | "settings.value.recording-format.mp4-remux"
  | "settings.value.recording-format.wav"
  | "settings.window-title.audio"
  | "settings.window-title.controls"
  | "settings.window-title.extensions"
  | "settings.window-title.global"
  | "settings.window-title.interface"
  | "settings.window-title.library"
  | "settings.window-title.network"
  | "settings.window-title.performance"
  | "settings.window-title.playback"
  | "settings.window-title.privacy"
  | "settings.window-title.recording"
  | "settings.window-title.subtitles"
  | "settings.window-title.video"
  | "shell.playlist.anime-to-watch"
  | "shell.playlist.cool-songs"
  | "shell.rail.add"
  | "shell.sidebar.back"
  | "shell.sidebar.count-suffix"
  | "shell.sidebar.create"
  | "shell.sidebar.create.hint"
  | "shell.sidebar.empty"
  | "shell.sidebar.list-label"
  | "shell.sidebar.more"
  | "shell.sidebar.more.hint"
  | "shell.sidebar.search"
  | "shell.sidebar.search.empty"
  | "shell.sidebar.search.placeholder"
  | "shell.sidebar.sort"
  | "shell.sidebar.title"
  | "shell.sidebar.view-grid"
  | "shell.sidebar.view-list"
  | "shell.title.toggle-sidebar"
  | "shell.window.close"
  | "shell.window.group"
  | "shell.window.maximize"
  | "shell.window.minimize"
  | "shortcuts.close"
  | "shortcuts.column.global"
  | "shortcuts.column.hotkey"
  | "shortcuts.filter.all"
  | "shortcuts.filter.label"
  | "shortcuts.save"
  | "stream.playlist.hint"
  | "stream.playlist.label"
  | "stream.playlist.optional"
  | "stream.playlist.placeholder"
  | "stream.url.example"
  | "stream.url.hint"
  | "discovery.menu.add-audio"
  | "discovery.menu.add-file"
  | "discovery.menu.add-folder"
  | "discovery.menu.add-subtitles"
  | "discovery.menu.info"
  | "discovery.menu.refresh"
  | "discovery.menu.title"
  | "player.canvas.menu.audio"
  | "player.canvas.menu.hide-queue"
  | "player.canvas.menu.hide-ui"
  | "player.canvas.menu.more-settings"
  | "player.canvas.menu.next"
  | "player.canvas.menu.open-media"
  | "player.canvas.menu.open-playlist"
  | "player.canvas.menu.previous"
  | "player.canvas.menu.quit"
  | "player.canvas.menu.show-queue"
  | "player.canvas.menu.subtitle"
  | "player.canvas.menu.title"
  | "player.canvas.menu.video"
  | "player.tools.colors.cartoon"
  | "player.tools.colors.color"
  | "player.tools.colors.extraction"
  | "player.tools.colors.gradient"
  | "player.tools.colors.negate"
  | "player.tools.colors.posterize"
  | "player.tools.colors.sepia"
  | "player.tools.colors.similarity"
  | "player.tools.colors.threshold"
  | "player.tools.compressor.attack"
  | "player.tools.compressor.knee"
  | "player.tools.compressor.makeup"
  | "player.tools.compressor.ratio"
  | "player.tools.compressor.release"
  | "player.tools.compressor.rms-peak"
  | "player.tools.compressor.threshold"
  | "player.tools.crop.bottom"
  | "player.tools.crop.left"
  | "player.tools.crop.link-horizontal"
  | "player.tools.crop.link-vertical"
  | "player.tools.crop.right"
  | "player.tools.crop.top"
  | "player.tools.crop.unit"
  | "player.tools.dialog.title"
  | "player.tools.dynamics.compressor"
  | "player.tools.dynamics.spatializer"
  | "player.tools.dynamics.staged"
  | "player.tools.dynamics.stereo-widener"
  | "player.tools.equalizer.staged"
  | "player.tools.essential.banding-radius"
  | "player.tools.essential.grain-variance"
  | "player.tools.essential.sharpen-sigma"
  | "player.tools.gradient.edge"
  | "player.tools.gradient.gradient"
  | "player.tools.gradient.hemboss"
  | "player.tools.gradient.mode"
  | "player.tools.hdr.auto"
  | "player.tools.hdr.dolby-vision"
  | "player.tools.hdr.hdr10"
  | "player.tools.hdr.hdr10-plus"
  | "player.tools.hdr.hlg"
  | "player.tools.hdr.mode"
  | "player.tools.hdr.off"
  | "player.tools.open"
  | "player.tools.spatializer.damp"
  | "player.tools.spatializer.dry"
  | "player.tools.spatializer.size"
  | "player.tools.spatializer.wet"
  | "player.tools.spatializer.width"
  | "player.tools.tab.audio"
  | "player.tools.tab.video"
  | "player.tools.video.staged"
  | "player.tools.visualizer.display.live"
  | "player.tools.visualizer.display.resting"
  | "player.tools.visualizer.mode"
  | "player.tools.visualizer.mode.disabled"
  | "player.tools.visualizer.mode.scope"
  | "player.tools.visualizer.mode.spectrometer"
  | "player.tools.visualizer.mode.spectrum"
  | "player.tools.visualizer.mode.waveform"
  | "player.tools.visualizer.staged"
  | "player.tools.visualizer.title"
  | "player.tools.widener.crossfeed"
  | "player.tools.widener.delay"
  | "player.tools.widener.dry-mix"
  | "player.tools.widener.feedback"
  | "playlist.queue.menu.add-audio"
  | "playlist.queue.menu.add-media"
  | "playlist.queue.menu.add-subtitles"
  | "playlist.queue.menu.favorite"
  | "playlist.queue.menu.play"
  | "playlist.queue.menu.save"
  | "playlist.queue.menu.title"
  | "screen.eq.band.12k"
  | "screen.eq.band.14k"
  | "screen.eq.band.16k"
  | "screen.eq.band.170"
  | "screen.eq.band.1k"
  | "screen.eq.band.310"
  | "screen.eq.band.3k"
  | "screen.eq.band.60"
  | "screen.eq.band.600"
  | "screen.eq.band.6k"
  | "screen.eq.preamp"
  | "screen.eq.preset"
  | "screen.eq.preset.classical"
  | "screen.eq.preset.club"
  | "screen.eq.preset.dance"
  | "screen.eq.preset.flat"
  | "screen.eq.preset.full-bass"
  | "screen.eq.preset.full-bass-treble"
  | "screen.eq.preset.full-treble"
  | "screen.eq.preset.none"
  | "screen.eq.title"
  | "screen.eq.unit-db"
  | "settings.effects.spatial-audio"
  | "settings.effects.spatial-audio-mode"
  | "settings.effects.spatial-mode.headphone-virtualization"
  | "settings.effects.stereo-widening"
  | "settings.effects.two-pass"
  | "settings.section.global.appearance"
  | "settings.section.global.startup"
  | "settings.section.global.diagnostics"
  | "settings.section.privacy.history"
  | "settings.section.privacy.network"
  | "settings.section.playback.startup"
  | "settings.section.playback.order"
  | "settings.section.playback.seeking"
  | "settings.section.video.output"
  | "settings.section.video.picture"
  | "settings.section.video.deinterlacing"
  | "settings.section.video.framing"
  | "settings.section.audio.volume"
  | "settings.section.audio.loudness"
  | "settings.section.performance.decoding"
  | "settings.section.performance.smooth-playback"
  | "settings.section.performance.developer"
  | "settings.global.accent-color"
  | "settings.global.show-dialogs"
  | "settings.global.developer-mode"
  | "settings.global.save-log"
  | "settings.global.log-file"
  | "settings.global.log-details"
  | "settings.global.show-minor-warnings"
  | "settings.global.extra-cli-options"
  | "settings.global.log-format"
  | "settings.global.collect-playback-statistics"
  | "settings.privacy.save-recent-items"
  | "settings.privacy.remember-playback-position"
  | "settings.privacy.never-remember-matching"
  | "settings.privacy.fetch-metadata-online"
  | "settings.privacy.submit-lastfm"
  | "settings.privacy.lastfm-username"
  | "settings.privacy.lastfm-password"
  | "settings.privacy.lastfm-submission-url"
  | "settings.playback.preferred-video-quality"
  | "settings.playback.menu-language"
  | "settings.playback.default-audio-track"
  | "settings.playback.default-subtitle-track"
  | "settings.playback.resume"
  | "settings.playback.pause-on-audio-focus"
  | "settings.playback.repeat-mode"
  | "settings.playback.shuffle"
  | "settings.playback.stop-after-each-item"
  | "settings.playback.finish-action"
  | "settings.playback.seek-precisely"
  | "settings.video.enable"
  | "settings.video.start-fullscreen"
  | "settings.video.disable-screensaver"
  | "settings.video.wallpaper"
  | "settings.video.hue"
  | "settings.video.brightness"
  | "settings.video.contrast"
  | "settings.video.saturation"
  | "settings.video.gamma"
  | "settings.video.deinterlace"
  | "settings.video.deinterlace-method"
  | "settings.video.aspect-ratio"
  | "settings.video.crop"
  | "settings.audio.enable"
  | "settings.audio.speaker-setup"
  | "settings.audio.maximum-volume"
  | "settings.audio.startup-volume"
  | "settings.audio.volume-step"
  | "settings.audio.normalize-loudness"
  | "settings.audio.target-level"
  | "settings.audio.night-mode"
  | "settings.audio.replay-gain"
  | "settings.audio.replay-gain-preamp"
  | "settings.performance.hw-decode"
  | "settings.performance.video-post-processing"
  | "settings.performance.h264-loop-filter"
  | "settings.performance.decoding-threads"
  | "settings.performance.drop-frames"
  | "settings.performance.realtime-priority"
  | "settings.performance.process-priority"
  | "settings.performance.encoder"
  | "settings.performance.decoder"
  | "settings.performance.clock-synchronisation"
  | "settings.performance.clock-jitter"
  | "settings.performance.clock-reference-averaging"
  | "settings.value.global-accent-color.orange"
  | "settings.value.global-accent-color.red"
  | "settings.value.global-accent-color.yellow"
  | "settings.value.global-accent-color.green"
  | "settings.value.global-accent-color.cyan"
  | "settings.value.global-accent-color.blue"
  | "settings.value.global-accent-color.violet"
  | "settings.value.global-accent-color.pink"
  | "settings.value.global-accent-color.gray"
  | "settings.value.global-log-details.errors-only"
  | "settings.value.global-log-details.errors-and-warnings"
  | "settings.value.global-log-details.all-messages"
  | "settings.value.global-log-format.plain-text"
  | "settings.value.global-log-format.html"
  | "settings.value.playback-preferred-video-quality.best-available"
  | "settings.value.playback-preferred-video-quality.high"
  | "settings.value.playback-preferred-video-quality.medium"
  | "settings.value.playback-preferred-video-quality.low"
  | "settings.value.playback-menu-language.en-us"
  | "settings.value.playback-resume.always"
  | "settings.value.playback-resume.ask"
  | "settings.value.playback-resume.never"
  | "settings.value.playback-repeat-mode.off"
  | "settings.value.playback-repeat-mode.one"
  | "settings.value.playback-repeat-mode.all"
  | "settings.value.playback-finish-action.stop"
  | "settings.value.playback-finish-action.pause"
  | "settings.value.playback-finish-action.quit"
  | "settings.value.video-deinterlace.automatic"
  | "settings.value.video-deinterlace.on"
  | "settings.value.video-deinterlace.off"
  | "settings.value.video-deinterlace-method.automatic"
  | "settings.value.video-deinterlace-method.blend"
  | "settings.value.video-deinterlace-method.bob"
  | "settings.value.video-deinterlace-method.linear"
  | "settings.value.video-deinterlace-method.yadif"
  | "settings.value.video-deinterlace-method.yadif2x"
  | "settings.value.video-aspect-ratio.automatic"
  | "settings.value.video-crop.none"
  | "settings.value.audio-speaker-setup.automatic"
  | "settings.value.audio-speaker-setup.mono"
  | "settings.value.audio-speaker-setup.stereo"
  | "settings.value.audio-speaker-setup.quadraphonic"
  | "settings.value.audio-speaker-setup.5.1"
  | "settings.value.audio-speaker-setup.7.1"
  | "settings.value.audio-replay-gain.off"
  | "settings.value.audio-replay-gain.track"
  | "settings.value.audio-replay-gain.album"
  | "settings.value.performance-hw-decode.automatic"
  | "settings.value.performance-hw-decode.on"
  | "settings.value.performance-hw-decode.off"
  | "settings.value.performance-video-post-processing.off"
  | "settings.value.performance-video-post-processing.automatic"
  | "settings.value.performance-video-post-processing.always"
  | "settings.value.performance-h264-loop-filter.never"
  | "settings.value.performance-h264-loop-filter.non-reference-frames"
  | "settings.value.performance-h264-loop-filter.always"
  | "settings.value.performance-encoder.automatic"
  | "settings.value.performance-encoder.software"
  | "settings.value.performance-encoder.hardware"
  | "settings.value.performance-decoder.automatic"
  | "settings.value.performance-decoder.software"
  | "settings.value.performance-decoder.hardware"
  | "settings.value.performance-clock-synchronisation.default"
  | "settings.value.performance-clock-synchronisation.audio"
  | "settings.value.performance-clock-synchronisation.video"
  | "settings.field.more-info"
  | "settings.search.placeholder"
  | "settings.value.interface-theme.auto"
  | "settings.section.subtitles.tracks-languages"
  | "settings.section.subtitles.loading"
  | "settings.section.subtitles.appearance"
  | "settings.subtitles.enable"
  | "settings.subtitles.preferred-language"
  | "settings.subtitles.text-encoding"
  | "settings.subtitles.auto-load"
  | "settings.subtitles.file-name-matching"
  | "settings.subtitles.font"
  | "settings.subtitles.size-values"
  | "settings.subtitles.font-size"
  | "settings.subtitles.bold"
  | "settings.subtitles.text-color"
  | "settings.subtitles.text-opacity"
  | "settings.subtitles.outline-thickness"
  | "settings.subtitles.outline-color"
  | "settings.subtitles.shadow"
  | "settings.subtitles.preferred-language.system"
  | "settings.subtitles.encoding.automatic"
  | "settings.subtitles.font.system-sans-serif"
  | "settings.subtitles.size-values.relative"
  | "settings.subtitles.size-values.pixels"
  | "settings.subtitles.font-size.automatic"
  | "settings.subtitles.outline-thickness.normal"
  | "settings.subtitles.text-color.value"
  | "settings.subtitles.outline-color.value"
  | "settings.subtitles.staged"
  | "screen.eq.open-panel"
  | "screen.eq.close-panel"
  | "playlist.browse.back"
  | "playlist.browse.empty-title"
  | "playlist.browse.empty-body"
  | "playlist.library.menu.open"
  | "playlist.subtitles.open"
  | "player.canvas.menu.customize-interface"
  | "player.appmenu.open-files"
  | "player.appmenu.open-recent"
  | "player.appmenu.profile"
  | "player.appmenu.keyboard-shortcuts"
  | "player.appmenu.customize"
  | "player.appmenu.about"
  | "settings.section.effects";

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
  "screen.discovery": "Discovery",
  "screen.convert": "Convert / Save (card 7.E3)",
  "screen.not-found": "Screen not found",
  "screen.placeholder.body":
    "Route skeleton only — the Figma-exact composition lands in later Phase-4 cards (doc 08 §1.6).",
  "screen.dev.tokens": "Token sandbox (dev fixture — not a product screen)",
  "tokens.mode": "Mode",
  "tokens.accent": "Accent",
  // --- player screen shell (card 4.C1) ---
  "player.canvas.label": "Video canvas",
  "player.canvas.placeholder": "Video appears here during playback — the render surface arrives with the desktop build.",
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
  "player.art.placeholder": "Album art appears once the engine reports it.",
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
  "playlist.panel.load-handoff": "Pick a file or drop it anywhere — playlists arrive with the desktop build.",
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
  "settings.loading": "Reading settings from the engine…",
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
  "settings.area.global": "General",
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
  "settings.area.library": "Library",
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
  // Effects & Tools area (issue #33 / D-012).
  "settings.area.effects": "Effects & Tools",
  "settings.section.effects.audio": "Audio effects",
  "settings.section.effects.video": "Video adjust",
  "settings.effects.volume": "Master volume",
  "settings.effects.mute": "Mute",
  "settings.effects.balance": "Balance",
  "settings.effects.eq.low": "Equalizer: bass (250 Hz)",
  "settings.effects.eq.mid": "Equalizer: mid (1 kHz)",
  "settings.effects.eq.high": "Equalizer: treble (4 kHz)",
  "settings.effects.brightness": "Brightness",
  "settings.effects.contrast": "Contrast",
  "settings.effects.saturation": "Saturation",
  "settings.effects.hue": "Hue",
  "settings.effects.pending-command": "Engine stage ready — control activates with the effects command card",
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
  "nav.title": "Primary navigation",
  "nav.player": "Player",
  "nav.playlist": "Playlist",
  "nav.media-info": "Media info",
  "nav.shortcuts": "Shortcuts",
  "nav.settings": "Settings",
  "nav.discovery": "Discovery",
  "nav.open-menu": "Open menu",
  "menu.file": "File",
  "menu.file.open-media": "Open media…",
  "menu.file.open-network-stream": "Open network stream…",
  "menu.file.convert": "Convert / Save…",
  "menu.file.exit": "Exit",
  "menu.playback": "Playback",
  "menu.playback.play-pause": "Play / pause",
  "menu.playback.stop": "Stop",
  "menu.playback.next": "Next",
  "menu.playback.previous": "Previous",
  "menu.playback.shuffle": "Shuffle",
  "menu.playback.repeat": "Repeat",
  "menu.view": "View",
  "menu.view.player": "Player",
  "menu.view.music-mode": "Music mode",
  "menu.view.miniplayer": "Miniplayer",
  "menu.view.playlist": "Playlist",
  "menu.view.media-info": "Media info",
  "menu.view.discovery": "Discovery",
  "menu.view.shortcuts": "Keyboard shortcuts",
  "menu.view.settings": "Settings",
  "menu.tools": "Tools",
  "menu.tools.customize": "Customize player…",
  "menu.tools.media-info": "Media information",
  "menu.tools.preferences": "Preferences",
  "menu.help": "Help",
  "menu.help.about": "About",
  "menu.help.keyboard-shortcuts": "Keyboard shortcuts",
  "demo.badge": "Demo",
  "demo.notice.title": "Demo mode",
  "demo.notice.body": "Playback is simulated in the browser shell. The desktop build drives the real engine.",
  "open.title": "Open media",
  "open.subtitle": "Pick a demo clip, a recent file, or paste a path.",
  "open.recent": "Recent",
  "open.recent.empty": "Nothing opened yet",
  "open.demo-clips": "Demo clips",
  "open.clip.bunny": "Big Buck Bunny (1080p, 10 min)",
  "open.clip.ambient": "Ambient loop (music, 3 min)",
  "open.clip.live": "Live edge (HLS stream)",
  "open.clip.radio": "Night radio (audio stream)",
  "open.action": "Open",
  "open.cancel": "Cancel",
  "open.close": "Close",
  "open.path.label": "File path or URL",
  "open.path.placeholder": "/media/demo/big-buck-bunny.mkv or https://…",
  "open.path.invalid": "Enter a file path or an http(s) URL",
  "stream.title": "Open network stream",
  "stream.subtitle": "Paste an HTTP, HLS or DASH address.",
  "stream.url.label": "Network URL",
  "stream.url.placeholder": "https://example.com/stream.m3u8",
  "stream.recent": "Recent streams",
  "stream.recent.empty": "No recent streams",
  "stream.back": "Back to player",
  "stream.open": "Open stream",
  "stream.invalid": "Enter a valid http:// or https:// URL",
  "discovery.title": "Discovery",
  "discovery.subtitle": "Local volumes and network services appear here as the engine reports them.",
  "discovery.volumes.title": "Local volumes",
  "discovery.volumes.count-suffix": "volumes",
  "discovery.volumes.empty": "No volumes reported yet — volume watching arrives with the platform integrations (issue #42).",
  "discovery.volumes.live": "Volume changes",
  "discovery.kind.fixed": "Fixed disk",
  "discovery.kind.removable": "Removable",
  "discovery.kind.optical": "Optical disc",
  "discovery.kind.network": "Network mount",
  "discovery.network.title": "Network services (UPnP)",
  "discovery.network.status": "The engine's SSDP/UPnP search is implemented but not reachable from this panel yet — the command-catalog wiring is a follow-up (issue #42).",
  "convert.title": "Convert / Save",
  "convert.subtitle": "Two honest conversions — nothing this engine cannot do.",
  "convert.source.label": "Source file",
  "convert.source.placeholder": "/media/demo/big-buck-bunny.mkv",
  "convert.source.required": "Enter a source file path",
  "convert.target.label": "Target",
  "convert.target.wav": "Record to WAV",
  "convert.target.wav.note": "Decodes the audio track and writes uncompressed PCM. Sample-format changes are lossless widenings only — never a silent quality loss.",
  "convert.target.mp4": "Remux to MP4",
  "convert.target.mp4.note": "Re-wraps H.264 video and AAC audio without re-encoding (stream copy). Any other codec is refused, not converted.",
  "convert.start": "Convert",
  "convert.back": "Back to player",
  "convert.queue.title": "Conversion queue",
  "convert.queue.empty": "No conversions queued",
  "convert.queue.full": "The conversion queue is full (8 jobs) — wait for one to finish.",
  "convert.lane.unavailable": "This build exposes no conversion commands: the engine’s conversion lane is not wired into its command catalog yet. Queued jobs report this honestly instead of pretending to run.",
  "convert.job.status.queued": "Queued",
  "convert.job.status.running": "Converting…",
  "convert.job.status.completed": "Completed",
  "convert.job.status.failed": "Failed",
  "convert.job.status.cancelled": "Cancelled",
  "convert.job.status.blocked": "Unavailable in this build",
  "convert.job.cancel": "Cancel",
  "convert.job.progress": "Conversion progress",
  "convert.job.blocked.reason": "Conversion is not exposed by this build (engine command catalog has no convert commands).",
  "shortcuts.title": "Keyboard shortcuts",
  "shortcuts.subtitle": "Every key the player understands. Edit overrides in Settings.",
  "shortcuts.search.label": "Search shortcuts",
  "shortcuts.search.placeholder": "Filter by action or key…",
  "shortcuts.search.empty": "No shortcuts match your search.",
  "shortcuts.group.playback": "Playback",
  "shortcuts.group.navigation": "Navigation",
  "shortcuts.group.volume": "Volume",
  "shortcuts.group.variants": "Player variants",
  "shortcuts.edit.hint": "Customize these in Settings \u2192 Hotkeys",
  "not-found.title": "Lost the thread",
  "not-found.body": "That page slipped off the reel. The player is still rolling.",
  "not-found.action": "Back to the player",
  "onboarding.shortcuts.hint-volume": "Volume wheel: turn it up, turn it down",
  "onboarding.shortcuts.hint-mute": "Mute in one tap",
  "onboarding.shortcuts.hint-playlist": "Jump to the playlist anytime",
  "onboarding.keys.enter": "Enter",
  "onboarding.keys.left": "Left",
  "onboarding.keys.right": "Right",
  "player.keys.strip.label": "Player keys",
  "player.demo.playing": "Simulated playback",
  "player.open.recent": "Recent",
  "app.name": "Lumen",
  "convert.queue.count-suffix": "jobs",
  "convert.source.browse": "Browse",
  "convert.source.browse-gate": "Native file browsing is not exposed in this build — paste a path instead.",
  "customize.box.label": "Toolbar items",
  "customize.close": "Close customize panel",
  "customize.instruction": "Drag items around and out of the box below to re-order them. Drag items into the box to remove them from the player.",
  "customize.save": "Save",
  "customize.tile.backward-10": "Backward 10s",
  "customize.tile.backward-15": "Backward 15s",
  "customize.tile.forward-10": "Forward 10s",
  "customize.tile.forward-15": "Forward 15s",
  "customize.title": "Customize Player",
  "discovery.volumes.empty-body": "Volume watching arrives with the platform integrations (issue #42).",
  "discovery.volumes.empty-title": "No volumes reported yet",
  "not-found.body-quiet": "The page you were looking for does not exist.",
  "not-found.heading": "Page not found",
  "onboarding.appearance.heading": "Select your preferred theme and color.",
  "onboarding.library.body": "Open a file, a playlist, or a network stream — paste a path or URL to queue it now, or leave it empty and open media from the player later.",
  "onboarding.library.failed": "That path could not be opened. Check it and try again.",
  "onboarding.library.opened": "Opened — it will be waiting when you finish.",
  "onboarding.library.title": "Media library",
  "onboarding.playback.body": "Quiet, honest defaults — every change saves the moment you make it and stays editable in Settings.",
  "onboarding.playback.title": "Playback",
  "onboarding.shortcuts.frame": "Previous / next frame",
  "onboarding.shortcuts.heading": "Learn the basic keyboard shortcuts.",
  "onboarding.shortcuts.item": "Previous / next item",
  "onboarding.shortcuts.more": "Find the full list in the menu.",
  "onboarding.shortcuts.seek": "Seek backward / forward",
  "onboarding.shortcuts.volume": "Increase / decrease volume",
  "onboarding.subtitles.body": "How subtitle tracks behave when a file carries them — saved the moment you change it, editable in Settings.",
  "onboarding.subtitles.title": "Subtitles",
  "player.controls.cast": "Cast",
  "player.controls.exit-fullscreen": "Exit fullscreen",
  "player.controls.fullscreen": "Enter fullscreen",
  "player.controls.group-output": "Player controls",
  "player.controls.next": "Next",
  "player.mini.queue.label": "Queue",
  "player.mini.queue.toggle": "Toggle queue",
  "player.mini.title-empty": "Nothing playing",
  "player.tools.close": "Close",
  "player.tools.collapse-settings": "Collapse Settings",
  "player.tools.expand-settings": "Expand Settings",
  "playlist.library.count-suffix": "playlists",
  "playlist.library.create": "New playlist",
  "playlist.library.empty-body": "Playlists you create are saved on this device.",
  "playlist.library.empty-title": "No playlists yet",
  "playlist.library.items-suffix": "items",
  "playlist.library.list-label": "Playlists",
  "playlist.library.menu": "Playlist actions",
  "playlist.library.menu.play": "Play",
  "playlist.library.menu.remove": "Remove",
  "playlist.new.browse": "Select files",
  "playlist.new.cancel": "Cancel",
  "playlist.new.close": "Close",
  "playlist.new.details": "Playlist details",
  "playlist.new.details.helper": "The playlist will be saved and displayed with this title and thumbnail.",
  "playlist.new.drop": "Drag files into the box or click here to select files.",
  "playlist.new.files": "Select files",
  "playlist.new.files.helper": "These files will be added to the playlist.",
  "playlist.new.name-placeholder": "e.g., Favorite Videos",
  "playlist.new.save": "Save",
  "playlist.new.thumbnail": "Playlist thumbnail",
  "playlist.new.thumbnail.hint": "Thumbnails are not exposed by this build yet.",
  "playlist.new.title": "New playlist",
  "playlist.options.entry-menu": "More actions",
  "playlist.options.grid-label": "Playlist thumbnails",
  "playlist.options.list-label": "Playlist media",
  "playlist.options.sort": "Sorting options",
  "playlist.options.sort.album-asc": "Album A-Z",
  "playlist.options.sort.album-desc": "Album Z-A",
  "playlist.options.sort.artist-asc": "Artist A-Z",
  "playlist.options.sort.artist-desc": "Artist Z-A",
  "playlist.options.sort.disc-number-asc": "Disc Number Ascending",
  "playlist.options.sort.disc-number-desc": "Disc Number Descending",
  "playlist.options.sort.newest-first": "Newest First",
  "playlist.options.sort.oldest-first": "Oldest First",
  "playlist.options.sort.title-asc": "Title A-Z",
  "playlist.options.sort.title-desc": "Title Z-A",
  "playlist.options.sort.track-number-asc": "Track Number Ascending",
  "playlist.options.sort.track-number-desc": "Track Number Descending",
  "playlist.options.sort.unwatched-first": "Unwatched First",
  "playlist.options.sort.uri-asc": "URI Ascending",
  "playlist.options.sort.uri-desc": "URI Descending",
  "playlist.options.sort.watched-first": "Watched First",
  "playlist.options.view": "View options",
  "playlist.options.view.grid": "Grid View",
  "playlist.options.view.list": "List View",
  "playlist.subtitles.cancel": "Cancel",
  "playlist.subtitles.close": "Close",
  "playlist.subtitles.destination": "Apply subtitles to",
  "playlist.subtitles.destination.file": "To This File",
  "playlist.subtitles.destination.folder": "To This Folder",
  "playlist.subtitles.destination.playlist": "To This Playlist",
  "playlist.subtitles.empty-body": "Subtitle files that match your videos appear here.",
  "playlist.subtitles.empty-title": "No subtitle files found",
  "playlist.subtitles.folder-path": "Subtitle folder path",
  "playlist.subtitles.language": "Subtitle language",
  "playlist.subtitles.language.english": "English",
  "playlist.subtitles.list-label": "Subtitle files",
  "playlist.subtitles.match": "Filename matching",
  "playlist.subtitles.match.any": "Any title",
  "playlist.subtitles.match.contains": "Contains title",
  "playlist.subtitles.match.exact": "Exact title match",
  "playlist.subtitles.match.extra": "Title with extra characters",
  "playlist.subtitles.match.off": "Off",
  "playlist.subtitles.pairs-label": "Matched subtitle pairs",
  "playlist.subtitles.save": "Save",
  "playlist.subtitles.search": "Search subtitles",
  "playlist.subtitles.search.placeholder": "The Wand S01E01",
  "playlist.subtitles.source": "Subtitle source",
  "playlist.subtitles.source.computer": "From Computer",
  "playlist.subtitles.source.online": "From OpenSubtitles",
  "playlist.subtitles.title": "Add Subtitles",
  "settings.advanced-options": "Advanced Options",
  "settings.area.audio": "Audio",
  "settings.area.controls": "Controls",
  "settings.area.extensions": "Extensions",
  "settings.area.recording": "Recording",
  "settings.area.subtitles": "Subtitles",
  "settings.area.video": "Video",
  "settings.close": "Close settings",
  "settings.controls.defaults": "Defaults",
  "settings.controls.edit-shortcuts": "Edit keyboard shortcuts",
  "settings.controls.mouse-double-click": "Double click on the video",
  "settings.controls.mouse-horizontal-scroll": "Horizontal scroll",
  "settings.controls.mouse-reverse-scroll-direction": "Reverse scroll direction",
  "settings.controls.mouse-scroll-wheel": "Scroll wheel",
  "settings.controls.mouse-single-click": "Single click on the video",
  "settings.controls.reset-all-shortcuts": "Reset all shortcuts",
  "settings.controls.reset-shortcuts": "Reset",
  "settings.controls.skip-long": "Long skip",
  "settings.controls.skip-medium": "Medium skip",
  "settings.controls.skip-short": "Short skip",
  "settings.controls.skip-very-short": "Very short skip",
  "settings.extensions.add": "Add",
  "settings.extensions.empty-body": "Add-ons appear here once the extensions feature ships in this build.",
  "settings.extensions.empty-title": "No add-ons installed",
  "settings.extensions.find-more": "Find more extensions",
  "settings.extensions.load-on-startup": "Load add-ons on startup",
  "settings.extensions.remove": "Remove",
  "settings.extensions.search": "Search",
  "settings.group.app": "App",
  "settings.group.media": "Media",
  "settings.group.system": "System",
  "settings.recording.directory": "Save recordings to",
  "settings.recording.format": "Recording format",
  "settings.section.controls.keyboard-shortcuts": "Keyboard shortcuts",
  "settings.section.controls.mouse": "Mouse",
  "settings.section.controls.skip-amounts": "Skip Amounts",
  "settings.section.extensions.add-ons": "Add-ons",
  "settings.section.recording.recording": "Recording",
  "settings.value.controls-mouse-double-click.none": "Do nothing",
  "settings.value.controls-mouse-double-click.play-pause": "Play / Pause",
  "settings.value.controls-mouse-double-click.show-menu": "Show menu",
  "settings.value.controls-mouse-double-click.toggle-fullscreen": "Toggle fullscreen",
  "settings.value.controls-mouse-horizontal-scroll.none": "Do nothing",
  "settings.value.controls-mouse-horizontal-scroll.seek": "Seek",
  "settings.value.controls-mouse-horizontal-scroll.volume": "Volume",
  "settings.value.controls-mouse-scroll-wheel.none": "Do nothing",
  "settings.value.controls-mouse-scroll-wheel.seek": "Seek",
  "settings.value.controls-mouse-scroll-wheel.volume": "Volume",
  "settings.value.controls-mouse-single-click.none": "Do nothing",
  "settings.value.controls-mouse-single-click.play-pause": "Play / Pause",
  "settings.value.controls-mouse-single-click.show-menu": "Show menu",
  "settings.value.controls-mouse-single-click.toggle-fullscreen": "Toggle fullscreen",
  "settings.value.recording-format.mp4-remux": "MP4 (stream copy)",
  "settings.value.recording-format.wav": "WAV",
  "settings.window-title.audio": "Audio Defaults",
  "settings.window-title.controls": "Mouse, Keyboard & Remote Controls",
  "settings.window-title.extensions": "Add-ons & Scripts",
  "settings.window-title.global": "Global Settings",
  "settings.window-title.interface": "Interface Settings",
  "settings.window-title.library": "Playlists, Folders & Files",
  "settings.window-title.network": "Network Connections & Security",
  "settings.window-title.performance": "Performance Settings",
  "settings.window-title.playback": "Playback Preferences",
  "settings.window-title.privacy": "Privacy Settings",
  "settings.window-title.recording": "Recording, Casting & Streaming Out",
  "settings.window-title.subtitles": "Subtitle Defaults",
  "settings.window-title.video": "Video Defaults",
  "shell.playlist.anime-to-watch": "Anime to Watch",
  "shell.playlist.cool-songs": "Cool Songs",
  "shell.rail.add": "Add media",
  "shell.sidebar.back": "Back",
  "shell.sidebar.count-suffix": "Items",
  "shell.sidebar.create": "Create playlist",
  "shell.sidebar.create.hint": "Creating playlists is not wired to the engine yet.",
  "shell.sidebar.empty": "No playlists yet.",
  "shell.sidebar.list-label": "Playlists",
  "shell.sidebar.more": "More playlist options",
  "shell.sidebar.more.hint": "Playlist actions are not exposed by this build yet.",
  "shell.sidebar.search": "Search playlists",
  "shell.sidebar.search.empty": "No playlists match your search.",
  "shell.sidebar.search.placeholder": "Search playlists…",
  "shell.sidebar.sort": "Sort playlists alphabetically",
  "shell.sidebar.title": "Playlists",
  "shell.sidebar.view-grid": "Grid view",
  "shell.sidebar.view-list": "List view",
  "shell.title.toggle-sidebar": "Toggle playlists sidebar",
  "shell.window.close": "Stop and return to the player",
  "shell.window.group": "Window controls",
  "shell.window.maximize": "Toggle fullscreen",
  "shell.window.minimize": "Minimize to miniplayer",
  "shortcuts.close": "Close shortcuts",
  "shortcuts.column.global": "Global Hotkey",
  "shortcuts.column.hotkey": "Hotkey",
  "shortcuts.filter.all": "All Fields",
  "shortcuts.filter.label": "Search scope",
  "shortcuts.save": "Save",
  "stream.playlist.hint": "The playlist will be saved and displayed with this title.",
  "stream.playlist.label": "Playlist Name",
  "stream.playlist.optional": "(optional)",
  "stream.playlist.placeholder": "e.g., Favorite TV Channels",
  "stream.url.example": "e.g., https://www.example.com/stream.avi",
  "stream.url.hint": "You can enter multiple links to load.",
  "discovery.menu.add-audio": "Add audio tracks",
  "discovery.menu.add-file": "Add file…",
  "discovery.menu.add-folder": "Add folder…",
  "discovery.menu.add-subtitles": "Add subtitle tracks",
  "discovery.menu.info": "Info",
  "discovery.menu.refresh": "Refresh",
  "discovery.menu.title": "Volume actions",
  "player.canvas.menu.audio": "Audio",
  "player.canvas.menu.hide-queue": "Hide queue panel",
  "player.canvas.menu.hide-ui": "Hide UI",
  "player.canvas.menu.more-settings": "More Settings",
  "player.canvas.menu.next": "Next Item",
  "player.canvas.menu.open-media": "Open Media",
  "player.canvas.menu.open-playlist": "Open playlist",
  "player.canvas.menu.previous": "Previous Item",
  "player.canvas.menu.quit": "Quit VLC",
  "player.canvas.menu.show-queue": "Show queue panel",
  "player.canvas.menu.subtitle": "Subtitle",
  "player.canvas.menu.title": "Player actions",
  "player.canvas.menu.video": "Video",
  "player.tools.colors.cartoon": "Cartoon",
  "player.tools.colors.color": "Color",
  "player.tools.colors.extraction": "Color Extraction",
  "player.tools.colors.gradient": "Gradient",
  "player.tools.colors.negate": "Negate Colors",
  "player.tools.colors.posterize": "Posterize",
  "player.tools.colors.sepia": "Sepia Intensity",
  "player.tools.colors.similarity": "Similarity",
  "player.tools.colors.threshold": "Color Threshold",
  "player.tools.compressor.attack": "Attack",
  "player.tools.compressor.knee": "Knee Radius",
  "player.tools.compressor.makeup": "Makeup Gain",
  "player.tools.compressor.ratio": "Ratio",
  "player.tools.compressor.release": "Release",
  "player.tools.compressor.rms-peak": "RMS / Peak",
  "player.tools.compressor.threshold": "Threshold",
  "player.tools.crop.bottom": "Bottom",
  "player.tools.crop.left": "Left",
  "player.tools.crop.link-horizontal": "Link left and right",
  "player.tools.crop.link-vertical": "Link top and bottom",
  "player.tools.crop.right": "Right",
  "player.tools.crop.top": "Top",
  "player.tools.crop.unit": "px",
  "player.tools.dialog.title": "Tools",
  "player.tools.dynamics.compressor": "Compressor",
  "player.tools.dynamics.spatializer": "Spatializer",
  "player.tools.dynamics.staged": "Staged — controls activate with the audio effects command card",
  "player.tools.dynamics.stereo-widener": "Stereo Widener",
  "player.tools.equalizer.staged": "Staged — controls activate with the audio effects command card",
  "player.tools.essential.banding-radius": "Banding Removal Radius",
  "player.tools.essential.grain-variance": "Film Grain Variance",
  "player.tools.essential.sharpen-sigma": "Sharpen Sigma",
  "player.tools.gradient.edge": "Edge",
  "player.tools.gradient.gradient": "Gradient",
  "player.tools.gradient.hemboss": "Hemboss",
  "player.tools.gradient.mode": "Gradient mode",
  "player.tools.hdr.auto": "Auto",
  "player.tools.hdr.dolby-vision": "Dolby Vision",
  "player.tools.hdr.hdr10": "HDR10",
  "player.tools.hdr.hdr10-plus": "HDR10+",
  "player.tools.hdr.hlg": "HLG",
  "player.tools.hdr.mode": "HDR mode",
  "player.tools.hdr.off": "Off",
  "player.tools.open": "Audio and video tools",
  "player.tools.spatializer.damp": "Damp",
  "player.tools.spatializer.dry": "Dry",
  "player.tools.spatializer.size": "Size",
  "player.tools.spatializer.wet": "Wet",
  "player.tools.spatializer.width": "Width",
  "player.tools.tab.audio": "Audio",
  "player.tools.tab.video": "Video",
  "player.tools.video.staged": "Staged — controls activate with the video effects command card",
  "player.tools.visualizer.display.live": "Live",
  "player.tools.visualizer.display.resting": "Waiting for audio data",
  "player.tools.visualizer.mode": "Visualizer",
  "player.tools.visualizer.mode.disabled": "Disabled",
  "player.tools.visualizer.mode.scope": "Scope",
  "player.tools.visualizer.mode.spectrometer": "Spectrometer",
  "player.tools.visualizer.mode.spectrum": "Spectrum",
  "player.tools.visualizer.mode.waveform": "Waveform",
  "player.tools.visualizer.staged": "Staged — the visualizer activates with the audio effects command card",
  "player.tools.visualizer.title": "Audio Visualizer",
  "player.tools.widener.crossfeed": "Crossfeed",
  "player.tools.widener.delay": "Delay Time",
  "player.tools.widener.dry-mix": "Dry Mix",
  "player.tools.widener.feedback": "Feedback Gain",
  "playlist.queue.menu.add-audio": "Add audio tracks",
  "playlist.queue.menu.add-media": "Add media…",
  "playlist.queue.menu.add-subtitles": "Add subtitle tracks",
  "playlist.queue.menu.favorite": "Favorite",
  "playlist.queue.menu.play": "Play",
  "playlist.queue.menu.save": "Save playlist",
  "playlist.queue.menu.title": "Queue item actions",
  "screen.eq.band.12k": "12 KHz",
  "screen.eq.band.14k": "14 KHz",
  "screen.eq.band.16k": "16 KHz",
  "screen.eq.band.170": "170 Hz",
  "screen.eq.band.1k": "1 KHz",
  "screen.eq.band.310": "310 Hz",
  "screen.eq.band.3k": "3 KHz",
  "screen.eq.band.60": "60 Hz",
  "screen.eq.band.600": "600 Hz",
  "screen.eq.band.6k": "6 KHz",
  "screen.eq.preamp": "Preamp",
  "screen.eq.preset": "Equalizer preset",
  "screen.eq.preset.classical": "Classical",
  "screen.eq.preset.club": "Club",
  "screen.eq.preset.dance": "Dance",
  "screen.eq.preset.flat": "Flat",
  "screen.eq.preset.full-bass": "Full bass",
  "screen.eq.preset.full-bass-treble": "Full bass & treble",
  "screen.eq.preset.full-treble": "Full treble",
  "screen.eq.preset.none": "None",
  "screen.eq.title": "Equalizer",
  "screen.eq.unit-db": "dB",
  "settings.effects.spatial-audio": "Spatial audio",
  "settings.effects.spatial-audio-mode": "Spatial audio mode",
  "settings.effects.spatial-mode.headphone-virtualization": "Headphone virtualization",
  "settings.effects.stereo-widening": "Stereo widening",
  "settings.effects.two-pass": "Two-pass",
  "settings.section.effects": "Effects",
  "settings.section.global.appearance": "Appearance",
  "settings.section.global.startup": "Startup",
  "settings.section.global.diagnostics": "Diagnostics",
  "settings.section.privacy.history": "History",
  "settings.section.privacy.network": "Network",
  "settings.section.playback.startup": "Startup",
  "settings.section.playback.order": "Order",
  "settings.section.playback.seeking": "Seeking",
  "settings.section.video.output": "Output",
  "settings.section.video.picture": "Picture",
  "settings.section.video.deinterlacing": "Deinterlacing",
  "settings.section.video.framing": "Framing",
  "settings.section.audio.volume": "Volume",
  "settings.section.audio.loudness": "Loudness",
  "settings.section.performance.decoding": "Decoding",
  "settings.section.performance.smooth-playback": "Smooth Playback",
  "settings.section.performance.developer": "Developer",
  "settings.global.accent-color": "Accent color",
  "settings.global.show-dialogs": "Show confirmation and error dialogs",
  "settings.global.developer-mode": "Developer mode",
  "settings.global.save-log": "Save a log",
  "settings.global.log-file": "Log file",
  "settings.global.log-details": "Log details",
  "settings.global.show-minor-warnings": "Show minor warnings",
  "settings.global.extra-cli-options": "Additional command line options",
  "settings.global.log-format": "Log format",
  "settings.global.collect-playback-statistics": "Collect playback statistics",
  "settings.privacy.save-recent-items": "Save recently played items",
  "settings.privacy.remember-playback-position": "Remember the playback position",
  "settings.privacy.never-remember-matching": "Never remember items matching",
  "settings.privacy.fetch-metadata-online": "Fetch titles and cover art from the internet",
  "settings.privacy.submit-lastfm": "Submit played tracks to last.fm",
  "settings.privacy.lastfm-username": "Last.fm username",
  "settings.privacy.lastfm-password": "Last.fm password",
  "settings.privacy.lastfm-submission-url": "Last.fm submission URL",
  "settings.playback.preferred-video-quality": "Preferred video quality",
  "settings.playback.menu-language": "Preferred menu language",
  "settings.playback.default-audio-track": "Default audio track",
  "settings.playback.default-subtitle-track": "Default subtitle track",
  "settings.playback.resume": "Continue where you left off",
  "settings.playback.pause-on-audio-focus": "Pause when another app uses audio",
  "settings.playback.repeat-mode": "Repeat",
  "settings.playback.shuffle": "Shuffle",
  "settings.playback.stop-after-each-item": "Stop after each item",
  "settings.playback.finish-action": "When playback finishes",
  "settings.playback.seek-precisely": "Seek precisely",
  "settings.video.enable": "Enable video",
  "settings.video.start-fullscreen": "Start videos in fullscreen",
  "settings.video.disable-screensaver": "Stop the screensaver during playback",
  "settings.video.wallpaper": "Play video as desktop wallpaper",
  "settings.video.hue": "Hue",
  "settings.video.brightness": "Brightness",
  "settings.video.contrast": "Contrast",
  "settings.video.saturation": "Saturation",
  "settings.video.gamma": "Gamma",
  "settings.video.deinterlace": "Deinterlacing",
  "settings.video.deinterlace-method": "Method",
  "settings.video.aspect-ratio": "Aspect ratio",
  "settings.video.crop": "Crop",
  "settings.audio.enable": "Enable audio",
  "settings.audio.speaker-setup": "Speaker setup",
  "settings.audio.maximum-volume": "Maximum volume",
  "settings.audio.startup-volume": "Startup volume",
  "settings.audio.volume-step": "Volume step",
  "settings.audio.normalize-loudness": "Normalize loudness",
  "settings.audio.target-level": "Target level",
  "settings.audio.night-mode": "Night mode",
  "settings.audio.replay-gain": "Replay gain",
  "settings.audio.replay-gain-preamp": "Replay gain preamp",
  "settings.performance.hw-decode": "Hardware-accelerated decoding",
  "settings.performance.video-post-processing": "Video post-processing",
  "settings.performance.h264-loop-filter": "Skip the H.264 loop filter",
  "settings.performance.decoding-threads": "Decoding threads",
  "settings.performance.drop-frames": "Drop frames when the system can't keep up",
  "settings.performance.realtime-priority": "Give VLC real-time priority",
  "settings.performance.process-priority": "Process priority",
  "settings.performance.encoder": "Encoder",
  "settings.performance.decoder": "Decoder",
  "settings.performance.clock-synchronisation": "Clock synchronisation",
  "settings.performance.clock-jitter": "Clock jitter",
  "settings.performance.clock-reference-averaging": "Clock reference averaging",
  "settings.value.global-accent-color.orange": "Orange",
  "settings.value.global-accent-color.red": "Red",
  "settings.value.global-accent-color.yellow": "Yellow",
  "settings.value.global-accent-color.green": "Green",
  "settings.value.global-accent-color.cyan": "Cyan",
  "settings.value.global-accent-color.blue": "Blue",
  "settings.value.global-accent-color.violet": "Violet",
  "settings.value.global-accent-color.pink": "Pink",
  "settings.value.global-accent-color.gray": "Gray",
  "settings.value.global-log-details.errors-only": "Errors only",
  "settings.value.global-log-details.errors-and-warnings": "Errors and warnings",
  "settings.value.global-log-details.all-messages": "All messages",
  "settings.value.global-log-format.plain-text": "Plain text",
  "settings.value.global-log-format.html": "HTML",
  "settings.value.playback-preferred-video-quality.best-available": "Best available",
  "settings.value.playback-preferred-video-quality.high": "High",
  "settings.value.playback-preferred-video-quality.medium": "Medium",
  "settings.value.playback-preferred-video-quality.low": "Low",
  "settings.value.playback-menu-language.en-us": "English",
  "settings.value.playback-resume.always": "Always",
  "settings.value.playback-resume.ask": "Ask",
  "settings.value.playback-resume.never": "Never",
  "settings.value.playback-repeat-mode.off": "Off",
  "settings.value.playback-repeat-mode.one": "One",
  "settings.value.playback-repeat-mode.all": "All",
  "settings.value.playback-finish-action.stop": "Stop",
  "settings.value.playback-finish-action.pause": "Pause",
  "settings.value.playback-finish-action.quit": "Quit",
  "settings.value.video-deinterlace.automatic": "Automatic",
  "settings.value.video-deinterlace.on": "On",
  "settings.value.video-deinterlace.off": "Off",
  "settings.value.video-deinterlace-method.automatic": "Automatic",
  "settings.value.video-deinterlace-method.blend": "Blend",
  "settings.value.video-deinterlace-method.bob": "Bob",
  "settings.value.video-deinterlace-method.linear": "Linear",
  "settings.value.video-deinterlace-method.yadif": "YADIF",
  "settings.value.video-deinterlace-method.yadif2x": "YADIF 2x",
  "settings.value.video-aspect-ratio.automatic": "Automatic",
  "settings.value.video-crop.none": "None",
  "settings.value.audio-speaker-setup.automatic": "Automatic",
  "settings.value.audio-speaker-setup.mono": "Mono",
  "settings.value.audio-speaker-setup.stereo": "Stereo",
  "settings.value.audio-speaker-setup.quadraphonic": "Quadraphonic",
  "settings.value.audio-speaker-setup.5.1": "5.1",
  "settings.value.audio-speaker-setup.7.1": "7.1",
  "settings.value.audio-replay-gain.off": "Off",
  "settings.value.audio-replay-gain.track": "Track",
  "settings.value.audio-replay-gain.album": "Album",
  "settings.value.performance-hw-decode.automatic": "Automatic",
  "settings.value.performance-hw-decode.on": "On",
  "settings.value.performance-hw-decode.off": "Off",
  "settings.value.performance-video-post-processing.off": "Off",
  "settings.value.performance-video-post-processing.automatic": "Automatic",
  "settings.value.performance-video-post-processing.always": "Always",
  "settings.value.performance-h264-loop-filter.never": "Never",
  "settings.value.performance-h264-loop-filter.non-reference-frames": "Non-reference frames",
  "settings.value.performance-h264-loop-filter.always": "Always",
  "settings.value.performance-encoder.automatic": "Automatic",
  "settings.value.performance-encoder.software": "Software",
  "settings.value.performance-encoder.hardware": "Hardware",
  "settings.value.performance-decoder.automatic": "Automatic",
  "settings.value.performance-decoder.software": "Software",
  "settings.value.performance-decoder.hardware": "Hardware",
  "settings.value.performance-clock-synchronisation.default": "Default",
  "settings.value.performance-clock-synchronisation.audio": "Audio clock",
  "settings.value.performance-clock-synchronisation.video": "Video clock",
  "settings.field.more-info": "More info",
  "settings.search.placeholder": "Search",
  "settings.value.interface-theme.auto": "System",
  "settings.section.subtitles.tracks-languages": "Tracks & Languages",
  "settings.section.subtitles.loading": "Loading Subtitles",
  "settings.section.subtitles.appearance": "Appearance",
  "settings.subtitles.enable": "Enable subtitles",
  "settings.subtitles.preferred-language": "Preferred language",
  "settings.subtitles.text-encoding": "Text encoding",
  "settings.subtitles.auto-load": "Load subtitle files automatically",
  "settings.subtitles.file-name-matching": "File name matching",
  "settings.subtitles.font": "Font",
  "settings.subtitles.size-values": "Size values",
  "settings.subtitles.font-size": "Font size",
  "settings.subtitles.bold": "Bold",
  "settings.subtitles.text-color": "Text color",
  "settings.subtitles.text-opacity": "Text opacity",
  "settings.subtitles.outline-thickness": "Outline thickness",
  "settings.subtitles.outline-color": "Outline color",
  "settings.subtitles.shadow": "Shadow",
  "settings.subtitles.preferred-language.system": "System language",
  "settings.subtitles.encoding.automatic": "Automatic",
  "settings.subtitles.font.system-sans-serif": "System, sans-serif",
  "settings.subtitles.size-values.relative": "Relative",
  "settings.subtitles.size-values.pixels": "Pixels",
  "settings.subtitles.font-size.automatic": "Automatic",
  "settings.subtitles.outline-thickness.normal": "Normal",
  "settings.subtitles.text-color.value": "#FFFFFF",
  "settings.subtitles.outline-color.value": "#000000",
  "settings.subtitles.staged": "Staged — controls activate when the subtitles settings seed lands",
  "screen.eq.open-panel": "Open Equalizer",
  "screen.eq.close-panel": "Close Equalizer",
  "playlist.browse.back": "All playlists",
  "playlist.browse.empty-title": "Nothing in this playlist yet",
  "playlist.browse.empty-body": "Add files while creating a playlist to fill it.",
  "playlist.library.menu.open": "Open",
  "playlist.subtitles.open": "Add subtitles",
  "player.canvas.menu.customize-interface": "Customize interface…",
  "player.appmenu.open-files": "Open File(s)",
  "player.appmenu.open-recent": "Open Recent",
  "player.appmenu.profile": "Profile",
  "player.appmenu.keyboard-shortcuts": "Keyboard Shortcuts",
  "player.appmenu.customize": "Customize Player",
  "player.appmenu.about": "About VLC",
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
