# DATA AUDIT: design/figma-renders/Toggle.png (credit: W19-A06, recovered by W19-S2)
| Field | Value |
|---|---|
| Auditor | W19-S2 (recovery) |
| Date | 2026-11-24 |
| Type | `file`: PNG image data, 900 x 1455, 8-bit/color RGBA, non-interlaced (md5 a3f24508...) |
| Dimensions | 900x1455 = 7.5x render of design/figma/Toggle.svg (120x194) |
| Cited as evidence? | YES — ControlBar.tsx:525 ("Shuffle as a Toggle.png switch"), SettingsPanel.tsx:352, ControlBar.test.tsx:1005, w26.a03.md:4, w26.i03.md:41 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
The form-family switch render, and the full five-color state anatomy of Toggle.svg
(W19-R7's audit) is present by census, symmetric where it should be:
- #090909 x68,939 px — the 68x32 rx6 tracks (both variants);
- ON: #371F06 x44,194 (33x26 segment) + #AB6314 x37,184 (26x26 knob);
- OFF: #191919 x43,797 (segment) + #404040 x37,190 (knob);
- on/off knob areas 37,184 vs 37,190 px — symmetric to 0.016%, i.e. same-size knobs
  across the two state groups, as the SVG anatomy claims.
- 7.5x uniform render scale on both axes (900/120 = 1455/194).
- The consumers citing it ("Toggle.png switch", "Toggle.png grammar") reference this
  anatomy; Switch.test.tsx:92's assertions were verified in the R7 SVG audit.
No defects. Checked: scale arithmetic, all 5 state colors by census, knob-area symmetry,
5 consumer citations.
