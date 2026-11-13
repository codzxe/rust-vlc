// w26.k13 (wave K fixer, lane 13/20 — Settings areas fixer, j13 review
// close-out). New message keys per CONTRACT §4.5 — the Lead registers them
// into ui/src/i18n/messages.ts post-wave; until then the settings field seam
// (SettingField.tsx laneCatalog) resolves them from this file.
//
// Content rules (j13 P1-1, the wave's last open P1 on this scope): the
// performance frame pins a r7.5 #404040 ⓘ circle after 10 of its 15 rows —
// the 4 Decoding rows, Process priority, and the 5 advanced Developer rows
// (circle extraction over `design/figma/Settings - performance.svg`:
// Ellipse 9..9_10 at cy 132/178/224/271/462/668/714/760/807/855). The
// frames pin the circle, never the string — the hint copy below is the
// honest one-line description of what each row does (the w26.i13/i14
// pattern), rendered through the shared SettingInfoDot grammar. The two
// Smooth-Playback toggles (Drop frames / real-time priority) carry NO circle
// in the frame, and the 3 advanced resources rows sit below the frame's
// exported fold — both absences are reproduced honestly (no ⓘ).
export const keys = {
  // --- Decoding (Settings - performance.svg ⓘ cy 132 / 178 / 224 / 271) ----
  "settings.hint.performance.hw-decode": "Use the graphics hardware to decode video when it can.",
  "settings.hint.performance.video-post-processing": "Apply extra filters to the picture after decoding.",
  "settings.hint.performance.h264-loop-filter": "When the H.264 loop filter runs while decoding.",
  "settings.hint.performance.decoding-threads": "How many threads decode video. 0 picks automatically.",
  // --- Smooth Playback (ⓘ cy 462 — Process priority only; the two toggles
  //     carry no circle in the frame) ----------------------------------------
  "settings.hint.performance.process-priority": "The priority the app asks the system for while playing.",
  // --- Developer — advanced rows behind the Advanced Options bar
  //     (ⓘ cy 668 / 714 / 760 / 807 / 855) -----------------------------------
  "settings.hint.performance.encoder": "The encoder used when recording or streaming out.",
  "settings.hint.performance.decoder": "The decoder preferred for playback.",
  "settings.hint.performance.clock-synchronisation": "Which stream's clock playback stays in step with.",
  "settings.hint.performance.clock-jitter": "How much clock drift is tolerated before playback re-syncs.",
  "settings.hint.performance.clock-reference-averaging": "How many clock samples are averaged to keep playback steady.",
} as const;
