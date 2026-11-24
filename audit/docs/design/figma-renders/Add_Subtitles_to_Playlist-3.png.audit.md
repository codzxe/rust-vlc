# DATA AUDIT: design/figma-renders/Add_Subtitles_to_Playlist-3.png (credit: W19-A07, recovered by W19-S2)
| Field | Value |
|---|---|
| Auditor | W19-S2 (recovery) |
| Date | 2026-11-24 |
| Type | `file`: PNG image data, 1088 x 840, 8-bit/color RGB (no alpha), non-interlaced (22440 B, md5 459ae247...) |
| Dimensions | 1088x840 = design/figma/Add Subtitles to Playlist/Folder-3.svg (1048x800) + 20px outer bleed on each side |
| Cited as evidence? | YES — w26.g19.md:17/:90 (state 4 = online pick; GAPS — P2-1 head, P3-1), CONTRACT.md:56 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
The 4th add-subtitles dialog state (online subtitle pick), render of Folder-3.svg:
- Source presence + scale: Folder-3.svg exists at 1048x800; the PNG's 1088x840 is
  that sheet plus a symmetric 20px bleed border (1048+40 = 1088, 800+40 = 840) —
  uniform on both axes, no distortion.
- State content census (PIL): #1A1A1A x694,066 (window fill), #090909 x100,953 (dark
  pane), **white x75,520** — the large white region distinctive of the ONLINE state
  (the subtitle-search page); Folder-3.svg itself carries 2 white fills, matching the
  census; #371F06 x2,920 = the selected-row fill. 360 unique colors — a flat-fill
  dialog, consistent with the 22KB compressed size.
- g19.md:90's verdict (GAPS — P2-1 header band 114px vs frame 96, P3-1) is the
  consumer's DOM-vs-frame measurement; nothing in this asset contradicts those
  numbers (the frame's own breadcrumb/head is what P2-1 measures).
No defects. Checked: source existence + scale arithmetic, white-region state
discriminator, selected-fill presence, 3 consumer citations.
