# AUDIT: ui/src/screens/player/tools/video/CropTool.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 214 / 8 (digits, CropFields/FRAME_CROP/CROP_PAIRS/axisOf, CropInput, CropPairRow, CropTool [setField + two CropPairRow wirings]) |
| File verdict | REAL (honest-staged UI; no backend will ever consume these values in the shipped product) |
| Reachable from shipped entry point? | YES in the web UI (PlayerScreen.tsx:1203 mounts `<ToolsDialogHost />`; ControlBar.tsx:702 `openToolsDialog(...)` from the player's tools control; ToolsDialog.tsx:135 `<CropTool />` on the video tab). NO in a desktop binary — none exists (W14-A01 E0425). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Grep `CropTool` in ui/src → ToolsDialog.tsx:135 (product mount) + harness.tsx:50-51 + CropTool.test.tsx; Read ui/src/screens/player/dialogs/ToolsDialog.tsx (mount + ToolsDialogHost seam); Grep `set-crop` in ui/src → generated/commands.ts:24 (the wire union DOES carry `video.set-crop { crop: CropSpec }`) and CropTool.tsx:173; Read apps/cli/src/dispatch.rs:38-54 (SERVED table: no VideoSetCrop) + :190-210 (owner_of: `video.*` → "video output control joins with the windowed renderer (2.D3)"); Read crates/video-engine/src/lib.rs:18-25 (the video filter chain incl. crop is "a later card; the pipeline seam is reserved"); Read crates/commands/src/catalog/command.rs:225-229 + spec.rs:371 + ids.rs:127 (the catalog command exists); Read ui/src/i18n/keys/w26.c4.ts (staged note copy); Grep `framing|crop` in ui/src/screens/settings → generated/schema.ts:1487-1496 (`video.crop` framing presets none/1:1/4:3/16:9/16:10 — the header's presets-live-elsewhere claim is true); Read tools/video/types.ts + VideoCard.tsx + copy.ts; sed -n quote verification. Baseline: ui-vitest.log.txt 136 files / 1705 passed (includes CropTool.test.tsx). |

## 1. One-paragraph verdict
As React code this is the cleanest kind of file in the wave: a 214-line pure-local-state card whose pair-coupling math (CROP_PAIRS + axisOf + setField twin-mirroring) is correct, whose accessibility is real (htmlFor/id wiring, aria-pressed link toggles, aria-hidden units), and whose 13 tests are behavioral and would fail on regression. The honest-staging design is genuinely implemented: the card renders a visible "Staged — controls activate with the video effects command card" note and carries data-pending-command="video.set-crop". The one factual defect is in the header: it claims "no crop engine stage or catalog command exists" — the engine-stage half is true (video-engine reserves the filter-chain seam, lib.rs:20-21) but the catalog-command half is FALSE: `video.set-crop` is a fully specified catalog command (CropSpec args, AppliedCrop result) present in the Rust catalog and the generated UI wire union; what does not exist is any host that serves it (SERVED excludes it; the typed refusal's owner names card 2.D3). The tool's values are consumed by nothing — in the shipped product this is an interactive control that can never affect video, by design and visibly disclosed.

## 2. What actually works (verified)
— verified by CropTool.test.tsx:10-95 (13 tests: collapsed/expanded shell, four labeled inputs with "0", two link toggles aria-pressed=true, linked-pair mirroring, unlink independence, non-digit sanitization, staged markers data-staged-tool/data-pending-command/role=note) + baseline ui-vitest (136 files, 1705 passed, 1 skipped).
— verified by reading setField (CropTool.tsx:158-167): the twin-mirror logic is correct for all four fields (CROP_PAIRS[axisOf(field)] gives the pair; twin = the other member; unlinked edits are field-local) — the tests at :53-83 pin exactly the three interesting cases.
— verified by Grep framing/crop in settings: schema.ts:1487-1496 carries the `video.crop` ratio presets under the framing section — the header's "the presets live in … Settings→Video framing rows, not here" deviation note is TRUE.

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/player/tools/video/CropTool.tsx:20-21 (header) vs :173 (the marker it contradicts)
Code:
```ts
// Honest staging: no crop engine stage or catalog command exists — local
// state + the visible staged markers (VideoCard), controls fully interactive.
```
(and, same file)
```ts
      pendingCommand="video.set-crop"
```
What is wrong: The claim "no … catalog command exists" is false. `video.set-crop` exists as a fully specified catalog command: crates/commands/src/catalog/command.rs:225-229 (`VideoSetCrop { crop: CropSpec }`), spec.rs:371-376 (id VideoSetCrop, args `crop: CropSpec`, result `AppliedCrop`), ids.rs:127, and the generated UI wire union ui/src/generated/commands.ts:24 (`{ args: { crop: CropSpec }; cmd: "video.set-crop" }`). What actually does not exist: (a) an engine crop stage — video-engine/src/lib.rs:20-21 says the video filter chain (color adjust, crop) is "a later card; the pipeline seam is reserved"; (b) any host that serves the command — SERVED (apps/cli/src/dispatch.rs:38-54) excludes it and the typed refusal's owner_of (dispatch.rs:195-196) answers "video output control joins with the windowed renderer (2.D3)". The same false sentence is copied into types.ts:50-53 ("No such command exists yet"). The comment misdescribes the architecture in the exact direction that matters for a staged tool: it says "we await a command that doesn't exist" when the truth is "the command exists end-to-end in the catalog and no host serves it" — a different gap with a different owner (host/renderer card 2.D3, not the command layer).
Why it matters: The honest-staging pattern's whole value is precision about what is missing; this header names the wrong missing piece. The next developer wiring the card will look for a command-layer card that is already written, and the file's own data-pending-command marker points at that existing command.
What correct looks like: "no crop engine stage exists and no shipped host serves `video.set-crop` (renderer card 2.D3) — local state + visible staged markers until then."
Confidence: HIGH — Verification: all five citation sites read (command.rs, spec.rs, ids.rs, dispatch.rs SERVED+owner_of, video-engine lib.rs, generated commands.ts).
### F-002 [S4]
Where: ui/src/screens/player/tools/video/CropTool.tsx:30, 71-79
Code:
```ts
const digits = (value: string): string => value.replace(/[^\d]/g, "");
```
What is wrong: Unbounded input: digits() admits arbitrarily long numeric strings (no maxLength attribute, no max clamp, no upper bound on pixels). A user can paste 40 digits; the field holds all 40 and, when linked, mirrors all 40 into the twin. Clearing a field yields "" (empty string), which the twin also mirrors.
Why it matters: Cosmetic today (nothing consumes the value — the tool is staged), but the moment the command card lands, `Number("99999999999999999999")` is 1e20 and a CropSpec built from raw input would be nonsense; a sane bound (maxLength or clamp to a plausible pixel range, e.g. 0–10000) belongs on the input now so the staged UI matches the eventual contract.
What correct looks like: maxLength={5} on the input (or clamp in digits()); treat "" as "0" at the eventual dispatch seam.
Confidence: HIGH — Verification: read; no consumer exists to trace (staged by design).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `digits` | 30 | REAL | S4 | Regex strip; unbounded (F-002). Pure. |
| `CropFields`/`FRAME_CROP`/`CROP_PAIRS`/`axisOf` | 33-55 | REAL | — | Module-level constants (w26.k11 fix); FRAME_CROP never mutated (all updates spread); CROP_PAIRS gives CropAxis a consumer (was dead export per types.ts:102-105). |
| `CropInput` | 58-87 | REAL | S4 | Controlled text input, htmlFor/id wiring correct, unit aria-hidden; no maxLength (F-002). |
| `CropPairRow` | 90-143 | REAL | — | Two inputs + link toggle (aria-pressed, aria-label from i18n); verbatim frame SVG; ids `${rowId}-first/-second` unique per row. |
| `CropTool` | 146-213 | REAL | S3 | Local-only state by design (staged, F-001 header lie); setField twin-mirror math correct and test-pinned; useId fallback when no id prop. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| :11 collapsed + aria-controls hidden | YES | DOM assertions on real render. |
| :21 defaultOpen expanded | YES | DOM assertion. |
| :30 four labeled inputs "0" + 4× "px" | YES | Role/name queries. |
| :38 two link toggles aria-pressed=true | YES | Frame-state pin. |
| :46 no `.video-sep` separators | YES | Negative DOM assertion. |
| :53 linked pair mirrors (Top→Bottom, Left untouched) | YES | fireEvent.change + value assertions — pins the core coupling. |
| :64 unlink makes pair independent | YES | Click + change + cross-field assertions. |
| :76 digit sanitization "1a2b"→"12" (+linked twin) | YES | Pins digits() + coupling in one. |
| :87 staged markers (data-staged-tool, data-pending-command, role=note text) | YES | Pins the honest-staging disclosure. |
| Gap: no test pins an upper bound or the empty-string state (F-002 unpinned). | | |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "no crop engine stage or catalog command exists" | CropTool.tsx:20-21 | HALF-FALSE (engine stage: true; catalog command: exists) | F-001: command.rs:225-229/spec.rs:371 vs video-engine lib.rs:20-21 and SERVED dispatch.rs:38-54. |
| "the frame carries NO aspect-ratio presets … the presets live in the customize panel / Settings→Video framing rows" | CropTool.tsx:15-18 | REAL | settings/generated/schema.ts:1487-1496 (`video.crop` enum none/1:1/4:3/16:9/16:10 in section "framing"). |
| staged note "Staged — controls activate with the video effects command card" | i18n/keys/w26.c4.ts:59-60, rendered via VideoCard role="note" | REAL | VideoCard.tsx:100-102 renders it; test :87-94 pins it. |
| "chain = pair locked (Top↔Bottom, Left↔Right — editing one mirrors the other)" | CropTool.tsx:11-13 | REAL | setField + tests :53-83. |

## 7. Dead / duplicated / unreachable code
No dead code in-file. Product-dead-by-design: the entire crop value pipeline (all four fields + link state) — no consumer, no dispatch, no persistence; disclosed via the staged note. Unreachable in the desktop product (no binary, W14-A01); reachable in the web UI and demo.

## 8. Dependencies & flags
VideoCard (shared shell, reuses dynamics.css grammar), useVideoMsg seam (copy.ts → i18n keys w26.c4 + w26.a18), types.ts (VideoToolProps, CropAxis). No transport, no engine imports, no feature flags. The named command `video.set-crop` is in the generated wire union but absent from both shipped hosts' SERVED tables (F-001).

## 9. Open questions for the next agent
- When card 2.D3 (windowed renderer) lands and serves `video.set-crop`, the CropSpec in the catalog is ratio-or-rectangle (`types.rs:420-433`) — the tool's four pixel fields map to the Rectangle variant only; nobody has reconciled the tool's input shape with the CropSpec enum (ratio presets live in settings, pixel crop here).
- Same false "no catalog command" sentence is copy-pasted at types.ts:50-53 and (per the effects-area pattern) likely in effects.tsx/dynamics files — one owner should fix all instances.
