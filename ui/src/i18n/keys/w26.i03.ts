// w26.i03 (wave I fixer, lane 03/20 — ControlBar + control parts, h04
// close-out). New message keys per CONTRACT §4.5 — the Lead registers them
// into ui/src/i18n/messages.ts post-wave; until then the ControlBar panel
// seam (the i03 lane resolver) resolves them from this file.
//
// Content rule: every label is frame-verbatim from the Controls.svg variant
// panels (the settings-popover family this lane rebuilt) — section headers,
// the queue tab labels, and the Defaults language rows. Reused catalog keys
// cover the rest: menu.playback / player.canvas.menu.video|audio|subtitle
// (tab labels), menu.playback.shuffle|repeat (Playback rows),
// player.track.auto|disable, player.controls.reason.no-media|.unknown,
// player.controls.advanced-settings (Button.png footer), playlist.entry.
// unknown-title.
export const keys = {
  // w26.k18 (j18 F4): player.controls.queue-items pruned from this lane file — verified dead
  // (unreferenced by any source; grandfathered in ui/i18n-dead-keys.json)
  // so a future Lead prune of the catalog cannot resurrect them via re-merge.
  /** Controls.svg audio/subtitles panels — section header over the track rows. */
  "player.controls.section.embedded-tracks": "Embedded Tracks",
  /** Controls.svg panel tab — plural spelling (Subtitles_12); the canvas
   * context-menu row keeps the singular (right-click menu.svg "Subtitle"). */
  "player.controls.tab.subtitles": "Subtitles",
  /** Controls.svg audio panel — section header (no engine seam: honest row). */
  "player.controls.section.output-device": "Output Device",
  /** Controls.svg subtitles panel — section header (no external-track seam). */
  "player.controls.section.external-tracks": "External Tracks",
  /** Controls.svg audio-settings panel — label/value section header. */
  "player.controls.section.defaults": "Defaults",
  /** Controls.svg audio-settings Defaults row — right value = track language. */
  "player.controls.row.audio-language": "Audio language",
  /** Controls.svg subtitle-settings Defaults row. */
  "player.controls.row.subtitle-language": "Subtitle language",
  /** Controls.svg queue panel — tab 2. */
  "player.controls.tab.bookmarks": "Bookmarks",
  /** Controls.svg queue panel — tab 3. */
  "player.controls.tab.chapters": "Chapters",
  /** Playback tab — the Repeat segmented control's short segment labels. */
  "player.controls.repeat.off": "Off",
  "player.controls.repeat.all": "All",
  "player.controls.repeat.one": "One",
} as const;
