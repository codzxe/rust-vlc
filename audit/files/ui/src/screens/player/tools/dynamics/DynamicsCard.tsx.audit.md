# AUDIT: ui/src/screens/player/tools/dynamics/DynamicsCard.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 (CORRECTED same day — F-001 retracted as display artifact) |
| Lines / functions | 140 / 2 (DynamicsCard, DynamicsRow memo adapter) |
| File verdict | SHALLOW (honestly-staged: interactive local state, no engine target — by design and admitted) |
| Reachable from shipped entry point? | YES — CompressorTool/StereoWidenerTool → tools dialog + effects settings area (effects.tsx) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg staged-copy existence (messages.ts:924-949); consumer census; fixed-string count `.dynamics-tool__body[hidden]` → 1 (collapse rule INTACT) |

## 1. One-paragraph verdict
The honest twin of the equalizer's dead seam: the header flatly states "no compressor/spatializer/stereo-widener engine stage or catalog command exists", the body renders a `role="note"` staged marker (i18n key `player.tools.dynamics.staged`, exists in messages.ts:924) and a `data-pending-command` attribute naming the awaited command. Sliders are fully interactive against local state. The collapse mechanism (`hidden={!open}` + `.dynamics-tool__body[hidden]` at dynamics.css:102) is intact — verified by fixed-string count.

## 2. What actually works (verified)
— verified by read: memo adapter with stable row defs (real perf care: only the changed row re-renders); aria-expanded/controls wiring; staged marker keys exist in the catalog.
— verified by fixed-string count: `.dynamics-tool__body[hidden]` present (1) — "Collapse Settings" works.
— verified by rg: consumed by CompressorTool, StereoWidenerTool, VideoCard (shared shell).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/tools/dynamics/DynamicsCard.tsx:23-26 (header claim)
Code:
```
// HONEST STAGING (the effects-area pattern, settings/areas/effects.tsx): no
// compressor/spatializer/stereo-widener engine stage or catalog command
// exists. Controls stay fully interactive with LOCAL state, visibly marked
// staged — a role="note" line + data-pending-command naming the
```
What is wrong: "visibly marked staged" overstates — the marker is a note line + data attributes; the sliders look and drag exactly like live ones (no disabled state, no visual differentiation of the track). A screen-reader user gets the note; a sighted user dragging a compressor threshold gets no signal the value goes nowhere until they read the footnote. Honest by documentation, not by interaction design. Confidence: HIGH — Verification: read.

F-002 [S3] — RETRACTED (display artifact)
An earlier version claimed the `.dynamics-tool__body[hidden]` rule was textually corrupted and the collapse broken. That was a display artifact of the Bash tool's output sanitizer (strips `[h`/`[m` sequences from displayed text). `rg -c --fixed-strings '.dynamics-tool__body[hidden]'` → 1: INTACT. No finding stands.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DynamicsRow (memo) | 38-58 | REAL | - | stable-props memoization |
| DynamicsCard | 61-140 | SHALLOW (staged) | S3 | local state only; staged note; collapse intact |

## 5. Test honesty
Lane tests (not in my list) assert interaction + markers; UNVERIFIED content.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no compressor/spatializer/stereo-widener engine stage or catalog command exists" | header L23-24 | REAL | rg: no such commands in the 65-cmd union |
| "visibly marked staged" | header L25 | SHALLOW | note line + data attrs only (F-001) |
| "never a dead control that looks engine-bound" | header L26 | FALSE-ish | the control DOES look engine-bound (full-interactive slider); only the note differentiates |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Icon, ToolSliderRow, dynamics.css. No env vars.

## 9. Open questions for the next agent
- Does any engine crate implement a dynamics/compressor audio filter? (If the engine has one but no command, the UI staging has a landing pad; if no, the tools dialog ships three make-believe audio tools.)
