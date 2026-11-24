# DATA AUDIT: design/figma/Icon Wrapper.svg (credit: W19-A07, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Type | `file`: SVG Scalable Vector Graphics image, ASCII, 22 lines, longest 1356 chars |
| Dimensions | width=72 height=122; hover chip 30x32 rx4 |
| Cited as evidence? | YES — 10+ hits across ui/src (appshell.css:225/250, variants.css:34/42, AppShell.tsx:667, Icon.test.tsx:400, dialogIcons.tsx:12) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## Verdict
Figma export of the rail/sidebar icon wrapper: two variants `Property 1=Default` and
`Property 1=hover`; the hover variant adds a `30x32 rx=4` white-0.06 icon plate; glyphs are
1.5-stroke paths ("Vector 75"/"Ellipse 10"). The rx4 claims consumers make are byte-exact:
variants.css:42 "frame: Icon Wrapper.svg rx4" and appshell.css:225 "frame: Icon Wrapper.svg
rx=4" both match SVG line 13 `<rect x="21" y="69" width="30" height="32" rx="4" fill="white"
fill-opacity="0.06"/>` (verified `sed -n '13p'`). The 1.5-stroke claim in dialogIcons.tsx:12
matches the four `width="1.5"` strokes in the file. ONE defect (S3): AppShell.tsx:667
comments "44px hit frame (Icon Wrapper.svg)" — the SVG contains NO 44px geometry (full
width/height inventory: 72/122, 71/121, 30/32, 1.5x4; `rg -o width/height` census above);
the code cites this file as the source of a dimension it does not carry. Icon.test.tsx:400
covers this file in tests (test honesty not re-run). Design asset; nothing executes.
