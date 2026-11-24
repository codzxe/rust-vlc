# DATA AUDIT: design/figma/Toggle.svg (credit: W19-A06, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Type | `file`: SVG Scalable Vector Graphics image, ASCII, 17 lines, longest 1364 chars |
| Dimensions | 120x194 sheet; tracks 68x32 rx6 |
| Cited as evidence? | YES — Switch.tsx:24/35, Switch.test.tsx:23/66/92, button/states.md:70/82, form.css:4, states.md:3 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
The form-family switch design: two variants `Value=on` / `Value=off`, each a 68x32 rx6
track on `#090909` with 33x26 rx3 segment + 26x26 rx3 knob. Every geometry claim consumers
make reconciles byte-exactly:
- Switch.tsx:35 "Geometry is an honest port of Toggle.svg ... 68×32 rx6 track" == SVG line 5
  `<rect x="32" y="20" width="68" height="32" rx="6" fill="#090909"/>` ✓
- Switch.test.tsx:92 "68x32 track, 33x26/26x26 segments" == lines 5-7 (33x26 `#371F06` +
  26x26 `#AB6314` knob) ✓ — and the test body really asserts those dimensions against
  the generated CSS (width: 68px/33px/26px), so the test would fail if the port drifted.
- button/states.md:70 "68x32 rx6 ... (#090909 per Toggle.svg" == fill ✓.
- On-knob `#AB6314` / off states `#191919` + `#404040` match the two variant groups
  line-for-line (`sed -n '5,7p;11,13p'` output above, byte-verbatim).
No defects found. Checked: variant ids, track/segment/knob geometry, fills, consumer
claims in 6 files, test honesty by inspection. Design asset; nothing here executes.
