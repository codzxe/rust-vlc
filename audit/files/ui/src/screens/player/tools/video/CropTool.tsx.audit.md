# AUDIT: ui/src/screens/player/tools/video/CropTool.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 213 / 6 (digits, axisOf, CropInput, CropPairRow, CropTool/setField, module consts) |
| File verdict | SHALLOW (real local interactivity, zero backend wiring — visibly staged) |
| Reachable from shipped entry point? | YES — mounted at player/dialogs/ToolsDialog.tsx:135 (`<CropTool />`, imported L39) -> player screen -> main.tsx |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n "set-crop" ui/src/generated/commands.ts crates/commands/src/catalog/command.rs` (present in BOTH); `rg -n "pendingCommand\|staged" VideoCard.tsx`; `rg -n "CropTool" ToolsDialog.tsx`; CropTool.test.tsx case list |

## 1. One-paragraph verdict
The crop card's LOCAL machinery is real and tested: digit sanitization, pair
linking/unlinking, mirrored edits, frame-accurate anatomy. But nothing leaves
the component: crop values live in useState, are never dispatched, and reset on
unmount. The card is visibly marked staged (VideoCard renders
`data-staged-tool` + a role="note" staged line, VideoCard.tsx:68/100-101).
The justification comment is false: `video.set-crop` DOES exist in the command
catalog — both the generated UI contract (commands.ts:24) and the Rust catalog
(command.rs:225-226 "renderer parameter (class S)") — the tool simply never
calls it.

## 2. What actually works (verified)
— Local behaviors per code read: setField mirror logic (L158-167) correct for
both axes; digits() sanitize (L30); aria-pressed link toggles (L120). —
CropTool.test.tsx: 8 behavioral cases (mirror across linked pair L53, unlink
independence L64, digit sanitize L76) — would fail if the logic broke. Not run
here (no node_modules; baseline vitest 1705 passed cited). — Reachable:
ToolsDialog.tsx:39,135.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/tools/video/CropTool.tsx:20-21
Code:
```
// Honest staging: no crop engine stage or catalog command exists — local
// state + the visible staged markers (VideoCard), controls fully interactive.
```
What is wrong: "no ... catalog command exists" is false. `video.set-crop` is
in the generated command catalog (commands.ts:24: `{ args: { crop: CropSpec };
cmd: "video.set-crop" }`) and the Rust catalog (command.rs:225-226). The tool
accepts crop pixels and never sends them anywhere; the comment blames a
catalog gap that does not exist.
Why it matters: users type crop values that LOOK applied (fully interactive
inputs, staged note notwithstanding) while the engine command sits unused in
the contract; the staging rationale is fabricated.
What correct looks like: dispatch `video.set-crop` on commit (the
PlaylistProvider pattern), or correct the comment to the real reason (engine
stage not implemented / feature-not-exposed).
Confidence: HIGH — Verification: read + rg (both catalogs).
Engine-side implementation of video.set-crop: UNVERIFIED (commands domain).

F-002 [S3]
Where: ui/src/screens/player/tools/video/CropTool.tsx:151
Code:
```
  const [crop, setCrop] = useState<CropFields>(FRAME_CROP);
```
What is wrong: crop is component-local and unclamped — no maximum (a 7-digit
pixel count is accepted), no validation against video dimensions (impossible
without a stage, granted), and values silently vanish when the dialog closes.
Every session starts at 0/0/0/0 regardless of any engine crop state.
Why it matters: the tool cannot represent or affect real state; combined with
F-001 this is a dead-effect control with production chrome.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: ui/src/screens/player/tools/video/CropTool.tsx:173
Code:
```
      pendingCommand="video.set-crop"
```
What is wrong: the prop name promises a pending command; VideoCard only uses
it as a data attribute (`data-pending-command`, VideoCard.tsx:69) for the
staged marker. No dispatch path exists anywhere under tools/video.
Confidence: HIGH — Verification: read + rg (VideoCard usage).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| digits | 30 | REAL | — | strips non-digits |
| CROP_PAIRS / axisOf | 48-55 | REAL | — | pair map consumed by setField (w26.k11 note: types.ts CropAxis was dead before this) |
| CropInput | 58-87 | REAL | — | label+input wired via htmlFor/id; unit aria-hidden |
| CropPairRow | 90-143 | REAL | — | link toggle aria-pressed; verbatim frame svg path |
| CropTool | 146-213 | SHALLOW | S3 (F-001/F-002) | real local state, no dispatch |
| — setField | 158-167 | REAL | — | mirror logic correct |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| "a linked pair mirrors edits across both fields" | YES | asserts both field values after edit (L53) |
| "unlinking a pair makes its fields independent" | YES | L64 |
| "sanitizes non-digit input" | YES | L76 |
| 5 structure/frame cases | YES (markup) | class/attribute assertions |
Tests are honest about LOCAL behavior; none can catch the missing dispatch
(the file has none) — the dead surface is untested-by-design.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no crop engine stage or catalog command exists" | CropTool.tsx:20 | FALSE (catalog half) | commands.ts:24 + command.rs:225-226 |
| "the visible staged markers (VideoCard)" | CropTool.tsx:21 | REAL | VideoCard.tsx:68,100-101 (role="note") |

## 7. Dead / duplicated / unreachable code
None beyond the staged-by-design nature. FRAME_CROP is the initial-state
constant (used).

## 8. Dependencies & flags
VideoCard, ./copy (useVideoMsg), ./types (CropAxis, VideoToolProps). No
transport import — backendless. No feature gates.

## 9. Open questions for the next agent
- Same as extensions.tsx F-001: does the engine implement `video.set-crop`
  (catalog says class S renderer parameter)? If yes, this tool ignored a
  usable command. Check the renderer-wgpu / commands handler tables.
- EssentialTool.tsx and HdrTool/ColorsTool (same folder, other agents) —
  verify whether ANY tool under tools/video dispatches a real command, or the
  entire tools dialog is staged.
