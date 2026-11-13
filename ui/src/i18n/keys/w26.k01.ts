// w26.k01 (wave K fixers, lane 01 of 20 — button/ + form/ review close-out,
// issue #64 / review w26.j01). New message keys per CONTRACT §4.5 — the Lead
// registers them into ui/src/i18n/messages.ts post-wave; until then the
// shared switch core's seam-first resolver (button/useSwitchLabels.ts)
// resolves them from this file, so the copy renders identically before and
// after (same pattern as w26.b15 / w26.f05).
//
// j01-4: the Toggle.svg ON/OFF micro labels were hardcoded English literals
// (allowlisted as "decorative artwork" — but aria-hidden hides them from AT,
// not from sighted non-English users). They are user-visible copy: they now
// flow through useMsg() + these keys. Frame-verbatim en-US casing ("ON"/
// "OFF", Toggle.svg paths); a real locale pack can spell them out
// ("Ein"/"Aus") — the 33×26 segment has room for ~6 glyphs at caption size.
export const keys = {
  "switch.on": "ON",
  "switch.off": "OFF",
} as const;
