# AUDIT: ui/src/screens/player/tools/video/EssentialTool.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 148 / 2 (EssentialTool + ESSENTIAL_ROWS data) |
| File verdict | SHALLOW (real local interactivity, zero backend wiring — visibly staged, honestly justified) |
| Reachable from shipped entry point? | YES — ToolsDialog.tsx (imports the four video tools incl. EssentialTool) -> player screen -> main.tsx |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg -n "brightness\|contrast\|...essential" ui/src/generated/commands.ts` (empty); same for crates/commands/src/catalog/command.rs (empty); EssentialTool.test.tsx case scan |

## 1. One-paragraph verdict
Eight picture-adjustment sliders (hue/brightness/contrast/saturation/gamma/
sharpen-sigma/banding-radius/grain-variance) with genuinely working local
interaction — and genuinely nowhere to send it: values live in useState, no
dispatch, reset on unmount. Unlike CropTool (whose "no command exists" claim
was false), this file's claim is TRUE: neither catalog contains any adjust/
essential command (`rg` over commands.ts and command.rs -> empty), and
`pendingCommand="video.set-essential"` is a forward-declared name that exists
nowhere. The staging is visible (VideoCard markers). The initial values are
deliberately calibrated so the PAINTED thumbs match the Figma frame — the
numbers are design props, not engine truth.

## 2. What actually works (verified)
— Catalog cross-check: no brightness/contrast/saturation/hue/gamma/sharpen/
grain/banding/essential command in the UI catalog or the Rust catalog (rg
empty on both) — the header claim is honest. — Tests (EssentialTool.test.tsx):
8 cases incl. tick counts, staged display ranges + valuetext, keyboard commit
updating valuetext — behavioral, would fail on regression. Not run (no
node_modules; baseline vitest 1705 passed cited).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/tools/video/EssentialTool.tsx:125-127, 141-143
Code:
```
  const [values, setValues] = useState<VideoSliderRowState>(() =>
    Object.fromEntries(ESSENTIAL_ROWS.map((row) => [row.id, row.initial])),
  );
```
What is wrong: eight fully-interactive sliders whose values are discarded —
local state only, no persistence, no engine effect, reset on dialog close.
The user can adjust the picture "Hue −37°, Brightness 1.42" and nothing
happens to any pixel. Staged markers + the missing catalog command make this
honest-but-dead rather than fraud — the same class as CropTool but with a
true justification.
Why it matters: the Tools dialog's VIDEO tab is 100% staged surface area
(CropTool + EssentialTool + HdrTool + ColorsTool all follow this pattern);
users cannot adjust anything.
Confidence: HIGH — Verification: read + rg (both catalogs).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ESSENTIAL_ROWS (data) | 40-121 | REAL | — | frame-calibrated initial values (documented as paint-matching, not engine data) |
| EssentialTool | 124-148 | SHALLOW | S3 (F-001) | local state only; staged markers via VideoCard |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| "sliders carry the staged display ranges and valuetext" | YES | asserts min/max/step/valuetext per row (L117) |
| "keyboard commit updates the valuetext" | YES | interaction assertion (L147) |
| tick/frame cases | YES | count + paint-domain assertions |
Tests pin the STAGED behavior honestly (the name says "staged display ranges") —
nothing pretends engine wiring.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no adjust/sharpen/gradfun/grain engine stage or catalog command exists" | EssentialTool.tsx:26-27 | REAL | rg empty over commands.ts + command.rs |
| "Engine ranges replace these on wiring (types.ts note)" | EssentialTool.tsx:22 | PAPER-ONLY | no wiring exists; types.ts note audited below (my assignment) |

## 7. Dead / duplicated / unreachable code
None beyond the staged-by-design nature.

## 8. Dependencies & flags
VideoCard, VideoSliderRow, ./types. No transport import — backendless. No
feature gates.

## 9. Open questions for the next agent
- HdrTool.tsx and ColorsTool.tsx (same folder): verify their "no command
  exists" claims against the catalog the same way — CropTool's was false;
  EssentialTool's is true. Each must be checked individually.
- Is any video-adjust command PLANNED in the engine (renderer-wgpu domain)?
  The catalog gap is real today.
