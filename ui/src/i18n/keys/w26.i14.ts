// w26.i14 (wave I fixer, lane 14/20 — Settings playback/video/audio/subtitles
// ratings close-out). New message keys per CONTRACT §4.5 — the Lead registers
// them into ui/src/i18n/messages.ts post-wave; until then the settings field
// seam (SettingField.tsx laneCatalog) resolves them from this file.
//
// Content rules:
//  • h09 defect #1 (ship-blocker): the video Aspect-ratio / Crop dropdowns
//    reference 10 unregistered enum msgids (generated/schema.ts
//    SETTING_VIDEO_ASPECT_RATIO / SETTING_VIDEO_CROP enumerations) — the labels
//    are the frame's own ratio spellings (Settings - video.svg "Framing"
//    dropdown values), so the copy is frame-verbatim, not invented. Picking
//    one no longer blanks the trigger.
//  • h09 #2 / h10 #1 (ⓘ affordances): the playback/video/audio frames pin a
//    r7.5 #404040 ⓘ circle after most row labels (circle extraction:
//    playback.svg 12 — every row except the Default-playback-speed slider;
//    video.svg 8 — Output×4 + Deinterlacing×2 + Framing×2, none on the
//    Picture sliders or the Reset action; audio.svg 10 — Output×3, Startup
//    volume, Volume step + the 5 Loudness rows, none on the Maximum-volume
//    slider). The frames pin the circle, never the string — the hint copy
//    below is the honest one-line description of what each row does (the
//    w26.i13 pattern), rendered through the shared SettingInfoDot grammar.
export const keys = {
  // --- Video "Framing" dropdown values (frame-verbatim ratio spellings) -----
  "settings.value.video-aspect-ratio.1:1": "1:1",
  "settings.value.video-aspect-ratio.4:3": "4:3",
  "settings.value.video-aspect-ratio.16:9": "16:9",
  "settings.value.video-aspect-ratio.16:10": "16:10",
  "settings.value.video-aspect-ratio.2.35:1": "2.35:1",
  "settings.value.video-aspect-ratio.2.39:1": "2.39:1",
  "settings.value.video-crop.1:1": "1:1",
  "settings.value.video-crop.4:3": "4:3",
  "settings.value.video-crop.16:9": "16:9",
  "settings.value.video-crop.16:10": "16:10",

  // --- Playback (Settings - playback.svg ⓘ — 12 rows; the Default playback
  //     speed slider row carries no circle, reproduced honestly) -------------
  "settings.hint.playback.resume": "Reopen the last playlist and item when the app starts.",
  "settings.hint.playback.start-paused": "Start playback automatically instead of opening the file paused.",
  "settings.hint.playback.pause-on-audio-focus": "Pause playback while another app plays sound.",
  "settings.hint.playback.preferred-video-quality": "The quality chosen when a stream offers several.",
  "settings.hint.playback.menu-language": "The preferred language for disc and stream menus.",
  "settings.hint.playback.default-audio-track": "The audio track selected by default.",
  "settings.hint.playback.default-subtitle-track": "The subtitle track selected by default.",
  "settings.hint.playback.repeat-mode": "How the playlist repeats — off, one item, or the whole list.",
  "settings.hint.playback.shuffle": "Play the playlist in random order.",
  "settings.hint.playback.stop-after-each-item": "Stop after each item instead of continuing.",
  "settings.hint.playback.finish-action": "What happens when playback reaches the end of the list.",
  "settings.hint.playback.seek-precisely": "Jump to the exact time, which can be slower on some files.",

  // --- Video (Settings - video.svg ⓘ — Output×4 + Deinterlacing×2 +
  //     Framing×2; the Picture sliders and the Reset action carry none) ------
  "settings.hint.video.enable": "Show the video track during playback.",
  "settings.hint.video.start-fullscreen": "Open files in fullscreen mode.",
  "settings.hint.video.disable-screensaver": "Keep the screen awake during playback.",
  "settings.hint.video.wallpaper": "Show the video behind the desktop icons.",
  "settings.hint.video.deinterlace": "When interlaced video is deinterlaced.",
  "settings.hint.video.deinterlace-method": "The filter used to clean up interlaced video.",
  "settings.hint.video.aspect-ratio": "Force the picture into a fixed aspect ratio.",
  "settings.hint.video.crop": "Trim the picture to a fixed shape.",

  // --- Audio (Settings - audio.svg ⓘ — Output×3, Startup volume, Volume
  //     step + the 5 Loudness rows; the Maximum-volume slider carries none) --
  "settings.hint.audio.enable": "Play the audio track during playback.",
  "settings.hint.audio.output-device": "The device audio plays through.",
  "settings.hint.audio.speaker-setup": "The speaker layout used for output.",
  "settings.hint.audio.startup-volume": "The volume playback starts at.",
  "settings.hint.audio.volume-step": "How much the volume changes per step.",
  "settings.hint.audio.normalize-loudness": "Even out loudness differences between items.",
  "settings.hint.audio.target-level": "The loudness level normalization aims for.",
  "settings.hint.audio.night-mode": "Narrow the dynamic range for quiet listening.",
  "settings.hint.audio.replay-gain": "How replay gain tags adjust playback volume.",
  "settings.hint.audio.replay-gain-preamp": "Extra gain applied on top of replay gain.",
} as const;
