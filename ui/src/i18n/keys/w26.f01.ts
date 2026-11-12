// w26.f01 lane keys (settings seed extension) — every msgid introduced by the
// schemas/settings.toml w26.f01 seed: frame-verbatim section titles + row
// labels + generator-derived enum option labels for the content-gap rows of
// e05 F1/F2/F8 (global: Appearance/Startup/Diagnostics + advanced), e06 F4
// (privacy: History/Network), e06 F5 (playback: Startup/Order/Seeking +
// Tracks additions), e06 F3 (video: Output/Picture/Deinterlacing/Framing),
// e07 F2/F3/F4 (audio: Output additions/Volume/Loudness), e08 F3
// (performance: Decoding/Smooth Playback/Developer).
//
// Registered into ui/src/i18n/messages.ts by the Lead after the wave merge
// (contract §4.5) — until then generated.test.ts proves coverage via this
// file and the screens render the labels once the catalog carries the keys.
// All copy is frame-verbatim (tools-settings.md §C1/C3/C4/C5/C6/C11); one
// Lead decision is flagged inline ("Give VLC real-time priority").
export const keys = {
  // --- Section titles (new [[setting]] sections, w26.f01 seed) --------------
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
  // settings.section.audio.output is already registered ("Output").
  "settings.section.audio.volume": "Volume",
  "settings.section.audio.loudness": "Loudness",
  "settings.section.performance.decoding": "Decoding",
  "settings.section.performance.smooth-playback": "Smooth Playback",
  "settings.section.performance.developer": "Developer",

  // --- Row labels — global (e05 F1/F2/F8, frame C1 + global-1) --------------
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

  // --- Row labels — privacy (e06 F4, frame C3) -------------------------------
  "settings.privacy.save-recent-items": "Save recently played items",
  "settings.privacy.remember-playback-position": "Remember the playback position",
  "settings.privacy.never-remember-matching": "Never remember items matching",
  "settings.privacy.fetch-metadata-online": "Fetch titles and cover art from the internet",
  "settings.privacy.submit-lastfm": "Submit played tracks to last.fm",
  "settings.privacy.lastfm-username": "Last.fm username",
  "settings.privacy.lastfm-password": "Last.fm password",
  "settings.privacy.lastfm-submission-url": "Last.fm submission URL",

  // --- Row labels — playback (e06 F5, frame C4) -------------------------------
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

  // --- Row labels — video (e06 F3, frame C5) ----------------------------------
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

  // --- Row labels — audio (e07 F2/F3/F4, frame C6) ----------------------------
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

  // --- Row labels — performance (e08 F3, frame C11) ---------------------------
  "settings.performance.hw-decode": "Hardware-accelerated decoding",
  "settings.performance.video-post-processing": "Video post-processing",
  "settings.performance.h264-loop-filter": "Skip the H.264 loop filter",
  "settings.performance.decoding-threads": "Decoding threads",
  "settings.performance.drop-frames": "Drop frames when the system can't keep up",
  // LEAD DECISION (w26.f01): frame-verbatim C11 copy carries the VLC name —
  // retitle to "Give Lumen real-time priority" at registration if product
  // voice prefers it (frames are the design authority; flagged, not silently
  // changed).
  "settings.performance.realtime-priority": "Give VLC real-time priority",
  "settings.performance.process-priority": "Process priority",
  "settings.performance.encoder": "Encoder",
  "settings.performance.decoder": "Decoder",
  "settings.performance.clock-synchronisation": "Clock synchronisation",
  "settings.performance.clock-jitter": "Clock jitter",
  "settings.performance.clock-reference-averaging": "Clock reference averaging",

  // --- Enum option labels (generator-derived
  //     settings.value.<key>.<value> msgids) ---------------------------------
  // global.accent-color — the 9 frame swatches, ids = themes/tokens.json.
  "settings.value.global-accent-color.orange": "Orange",
  "settings.value.global-accent-color.red": "Red",
  "settings.value.global-accent-color.yellow": "Yellow",
  "settings.value.global-accent-color.green": "Green",
  "settings.value.global-accent-color.cyan": "Cyan",
  "settings.value.global-accent-color.blue": "Blue",
  "settings.value.global-accent-color.violet": "Violet",
  "settings.value.global-accent-color.pink": "Pink",
  "settings.value.global-accent-color.gray": "Gray",
  // global.log-details / global.log-format
  "settings.value.global-log-details.errors-only": "Errors only",
  "settings.value.global-log-details.errors-and-warnings": "Errors and warnings",
  "settings.value.global-log-details.all-messages": "All messages",
  "settings.value.global-log-format.plain-text": "Plain text",
  "settings.value.global-log-format.html": "HTML",
  // playback enums
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
  // video enums
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
  "settings.value.video-aspect-ratio.1:1": "1:1",
  "settings.value.video-aspect-ratio.4:3": "4:3",
  "settings.value.video-aspect-ratio.16:9": "16:9",
  "settings.value.video-aspect-ratio.16:10": "16:10",
  "settings.value.video-aspect-ratio.2.35:1": "2.35:1",
  "settings.value.video-aspect-ratio.2.39:1": "2.39:1",
  "settings.value.video-crop.none": "None",
  "settings.value.video-crop.1:1": "1:1",
  "settings.value.video-crop.4:3": "4:3",
  "settings.value.video-crop.16:9": "16:9",
  "settings.value.video-crop.16:10": "16:10",
  // audio enums
  "settings.value.audio-speaker-setup.automatic": "Automatic",
  "settings.value.audio-speaker-setup.mono": "Mono",
  "settings.value.audio-speaker-setup.stereo": "Stereo",
  "settings.value.audio-speaker-setup.quadraphonic": "Quadraphonic",
  "settings.value.audio-speaker-setup.5.1": "5.1",
  "settings.value.audio-speaker-setup.7.1": "7.1",
  "settings.value.audio-replay-gain.off": "Off",
  "settings.value.audio-replay-gain.track": "Track",
  "settings.value.audio-replay-gain.album": "Album",
  // performance enums
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
} as const;
