// w26.i13 lane keys (Settings areas A fixer — h08 ratings): the ⓘ hint copy
// for the global/interface/privacy rows whose frames carry the info affordance
// (tools-settings.md §C1–C3; circle extraction: 6 on global default + 4 on the
// global-1 advanced rows, 11 on interface, 9 on privacy — the frames pin the
// r7.5 #404040 circle after the label, not the string; the copy below is the
// honest one-line description of what each row does).
//
// Registered into ui/src/i18n/messages.ts by the Lead after the wave merge
// (contract §4.5); until then the settings areas resolve these via the
// per-file lane seams (areas/index.tsx · areas/interface.tsx ·
// areas/privacy.tsx — and SettingField.tsx for the hint prop), so the copy
// renders identically before and after registration.

export const keys = {
  // --- Global — Startup (Settings - global.svg ⓘ cy 327 / 369) --------------
  "settings.hint.global.show-dialogs": "Show a dialog to confirm actions and report errors.",
  "settings.hint.global.update-checks": "Look for app updates automatically at startup.",
  // --- Global — Diagnostics (ⓘ cy 470 / 512 / 556 / 602) ---------------------
  "settings.hint.global.developer-mode": "Unlock developer tools and extra diagnostics.",
  "settings.hint.global.save-log": "Write a log file while the app runs.",
  "settings.hint.global.log-file": "The path of the log file.",
  "settings.hint.global.log-details": "How much detail the log records.",
  // --- Global — advanced rows (Settings - global-1.svg ⓘ cy 724 / 768 / 814 / 858)
  "settings.hint.global.show-minor-warnings": "Also record minor warnings that are usually skipped.",
  "settings.hint.global.extra-cli-options": "Extra options passed to the playback engine at launch. Applied after a restart.",
  "settings.hint.global.log-format": "The file format of the saved log.",
  "settings.hint.global.collect-playback-statistics": "Count plays per item for the library statistics.",

  // --- Interface (Settings - interface.svg ⓘ — 11 of 13 rows; the frame
  //     carries NO ⓘ on "Remember window size and position" (cy 367) and none
  //     after the "Hide after" label (x305, y728) — reproduced honestly) ------
  "settings.hint.interface.use-custom-skin": "Load the window chrome from a skin file.",
  "settings.hint.interface.skin-file": "The skin file used while a custom skin is on.",
  "settings.hint.interface.play-video-in-main-window": "Play videos here instead of in a separate window.",
  "settings.hint.interface.resize-to-fit-video": "Resize the window to match the video size.",
  "settings.hint.interface.keep-above": "Keep this window in front of other windows.",
  "settings.hint.interface.window-decorations": "Show the system window frame around the window.",
  "settings.hint.interface.filename-in-title-bar": "Show the playing file name in the window title.",
  "settings.hint.interface.open-fullscreen-on": "The screen that fullscreen playback opens on.",
  "settings.hint.interface.start-in-compact-mode": "Start in a compact window with only the player.",
  "settings.hint.interface.auto-hide-controls": "When the playback controls fade out.",
  "settings.hint.interface.show-controls-in-fullscreen": "Keep the controls visible during fullscreen playback.",

  // --- Privacy (Settings - privacy.svg ⓘ — every row incl. the Clear action) -
  "settings.hint.privacy.save-recent-items": "Keep a list of what you played recently.",
  "settings.hint.privacy.remember-playback-position": "Resume each item where you stopped.",
  "settings.hint.privacy.never-remember-matching": "Items whose names contain this text are never remembered.",
  "settings.hint.privacy.clear-history": "Forget every remembered item and playback position.",
  "settings.hint.privacy.fetch-metadata-online": "Download titles and cover art from the internet.",
  "settings.hint.privacy.submit-lastfm": "Scrobble what you play to your last.fm profile.",
  "settings.hint.privacy.lastfm-username": "The last.fm account plays are submitted to.",
  "settings.hint.privacy.lastfm-password": "The password for the last.fm account.",
  "settings.hint.privacy.lastfm-submission-url": "Scrobble server address — change it only for a custom server.",
} as const;
