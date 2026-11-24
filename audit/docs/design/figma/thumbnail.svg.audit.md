# DATA AUDIT: design/figma/thumbnail.svg (credit: W19-A08, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Type / size | `file`: SVG Scalable Vector Graphics image, ASCII, 16 lines, 460 147 bytes (embedded base64 raster) |
| Dimensions | 272x189 rx14 card; artwork image 2242x1250 |
| Cited as evidence? | YES — options.css:135/156, ThumbnailGrid.tsx:1, sorting.test.ts:146, playlist.css:566, PlaylistOptionsToolbar.tsx:42 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
The playlist grid-view thumbnail card: 272x189 rx14 plate on `#191919`@0.95, full-bleed
screenshot artwork baked in as base64 (`data-name="Screenshot 2026-09-13 at
5.01.39…PM.png"`), duration label rendered as outline path with id `00:02:10`. All
consumer claims reconcile byte-exactly:
- options.css:156 "width: 272px; /* frame: thumbnail.svg 272×189 */" == SVG `width="272"
  height="189" rx="14"` (line 1 + line 3) ✓
- ThumbnailGrid.tsx:1 "272×189 r14 cards" ✓
- sorting.test.ts:146 "formatDurationClock renders zero-padded h:mm:ss (thumbnail.svg
  00:02:10)" — the test body asserts `formatDurationClock(130) -> "00:02:10"` (130 s =
  2:10 ✓), tying the duration grammar to the frame label ✓ (test would fail on drift)
- playlist.css:566 honestly notes the artwork's measured dominant tone `≈ #2d2b29` and
  "the frame carries no stroke" — the SVG has no stroke on the plate ✓.
No defects. Checked: card geometry, corner radius, fills, embedded raster identity,
duration label, five consumer citations, one consumer test. Design asset; nothing here
executes. Quotes byte-verbatim via `sed -n '3p;7p'` (output above).
