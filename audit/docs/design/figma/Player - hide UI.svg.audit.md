# DATA AUDIT: design/figma/Player - hide UI.svg (credit: W19-A07, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Type / size | `file`: SVG Scalable Vector Graphics image, ASCII, 21 lines, 117 443 bytes (longest line 64 384 chars — embedded base64 raster) |
| Dimensions | width=1920 height=1080 (fullscreen frame) |
| Cited as evidence? | YES — keymaps/default.toml:216, PlayerScreen.tsx:62/355/754/769, useChromeAutoHide.ts:10, states.css:16/59, tests |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## Verdict
The "hide UI" target-state design frame: a 1920x1080 black canvas with the video content
baked in as a base64 `<image data-name="video.png">` pattern fill, plus one blurred bezel
rect (`stroke="#1B1B1B" stroke-width="20"`). Content census: only 5 rect-level elements
(rg '<path|<text|<circle|<rect' -> 5), zero interactive/UI glyphs — the file is genuinely
zero-chrome, which is exactly what the code claims it is: PlayerScreen.tsx:769 '...the
player's own parts. "Player - hide UI.svg" is a zero-chrome' state; useChromeAutoHide.ts:10
calls it the reference frame for chrome retirement. keymaps/default.toml:216 honestly
labels the Ctrl+\ binding a "design-invented key ... (Player - hide UI.svg target state)".
S4 only: the file name contains spaces AND hyphens ("Player - hide UI.svg"), forcing quoted
path handling in every consumer. Verified: `sed -n '3,4p'` (output above byte-verbatim);
`rg -c` element census; 10+ repo citations found. Design asset; nothing here executes.
