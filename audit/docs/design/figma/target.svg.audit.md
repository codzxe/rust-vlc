# DATA AUDIT: design/figma/target.svg (credit: W19-A08, recovered by W19-S2)
| Field | Value |
|---|---|
| Auditor | W19-S2 (recovery) |
| Date | 2026-11-24 |
| Type | `file`: SVG Scalable Vector Graphics image, ASCII, 103 lines, 76364 B (longest line 9135 chars) |
| Dimensions | 240x520 sheet |
| Cited as evidence? | YES — docs/08-ui-figma-fidelity.md:142/:169 (settings/library atoms), design/README.md:24 (menus list), w26.g15.md:5, w26.e12.md:240 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
The "target" state family: four state groups — `id="state=to this folder"`,
`state=to this file`, `state=dropdown 1`, `state=dropdown 2` — each opened by the same
compact trigger: `<rect x="20" y="20" width="200" height="34" rx="6" fill="#151515"/>`
(4 triggers total, verified: 4 rects width="200" height="34" with fill #151515). The
consumer claim checks out byte-exact:
- w26.e12.md:240 "`origin/target.svg` compact triggers are #151515" == the 4 trigger fills ✓
  (and its contrast point — Setting Preset.svg is #090909 — is that file's business).
- Outer frame `<rect x="0.5" y="0.5" width="239" height="519" rx="4.5" stroke="#8A38F5"
  stroke-dasharray="10 5"/>` is Figma export annotation chrome, excluded from token
  mining per tokens.json $description ("purple #8A38F5 dashed frames ... excluded") and
  doc 08 §1.1-1.2 — correctly NOT a token claim anywhere.
No defects. Checked: state ids, trigger geometry/fill counts, annotation-chrome status,
all 4 citing docs. Design asset; nothing here executes.
