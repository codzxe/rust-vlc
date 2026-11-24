# AUDIT: ui/src/screens/player/tools/video/ColorsTool.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 343 / 4 (ColorsHexRow, ColorsToggleRow, ColorsTool, module consts) |
| File verdict | SHALLOW (interactive shell, zero backend wiring — honestly labeled staged) |
| Reachable from shipped entry point? | YES — tools/video/index.ts -> ToolsDialog.tsx (player tools dialog) + settings/areas/effects.tsx (mounted in the settings Effects area) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg "video.set-colors" in schemas/ + crates/ (NO MATCHES — command does not exist); read VideoCard.tsx staged-note; read ColorsTool.test.tsx (244 LOC); rg staged-note copy in messages.ts |

## 1. One-paragraph verdict
Every control in this tool — 7 toggles, 3 sliders, 2 hex inputs, 1 gradient
dropdown — is LOCAL useState with zero engine dispatch: nothing leaves the
component. The module header admits it ("no color-threshold/negate/posterize/
gradient/cartoon/sepia engine stage or catalog command exists — local state"),
and the shared VideoCard renders a visible role="note" "Staged — controls
activate with the video effects command card" (messages.ts:2429) plus
data-staged-tool/data-pending-command markers. The awaited command
`video.set-colors` exists nowhere in schemas/ or crates/ (rg verified). For
the master question: this is a dead surface, disclosed. Interactivity quality
itself is high (a11y wiring, calibrated slider paint).

## 2. What actually works (verified)
- Interactive local state: toggle/slider/hex/dropdown handlers are real React
  state (lines 201-218) and the test suite drives them (244 LOC, asserts
  aria-expanded/valuenow/valuetext and hex swatch paint).
- Honest staging markers: VideoCard.tsx:68-69 `data-staged-tool` /
  `data-pending-command` + staged note rendered for every mount host;
  copy registered in messages.ts:2429.
- Slider step/initial calibration documented against the frame (comments
  51-63) and pinned by test (aria-valuenow 21.6).

## 3. Findings
F-001 [S3]
Where: ColorsTool.tsx:28-30 + 196-343
Code:
```
// Honest staging: no color-threshold/negate/posterize/gradient/cartoon/sepia
// engine stage or catalog command exists — local state + the visible staged
// markers (VideoCard), controls fully interactive.
```
What is wrong: 13 interactive controls with zero persistence and zero engine
effect — the user can toggle Negate, drag Sepia, type hex codes and nothing
happens to playback or settings. Disclosed in-file and on-screen, so not
fraud, but it is a dead surface shipped in the player's Tools dialog and the
settings Effects area. Why it matters: master-question counterexample —
UI elements NOT wired to any backend. What correct looks like: either the
video effects command card (the awaited seam) or non-render until wired.
Confidence: HIGH — Verification: read + rg (video.set-colors absent from
schemas/ and crates/).
F-002 [S4]
Where: ColorsTool.tsx:97-104
Code:
```
/** The Gradient mode sub-dropdown's options — VLC's gradient filter modes
 * (the frame exports only the closed trigger; option set [VERIFY engine
 * lane]). Mutably typed: the Dropdown primitive's options prop. */
```
What is wrong: an open [VERIFY engine lane] TODO shipped in a dialog the
user can open; "hemboss" is a misspelling of VLC's "emboss" carried into a
msgid (`player.tools.gradient.hemboss`). Cosmetic while staged; will surface
as a typo if the lane lands. Confidence: HIGH — Verification: read.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| percent | 49 | REAL | - | valuetext formatter |
| SATURATION/SIMILARITY/SEPIA row defs | 51-95 | REAL | - | calibrated staged initials, 0.05 lattice |
| GRADIENT_MODES | 97-104 | REAL | S4 | see F-002 |
| FRAME_TOGGLES / ALL_OFF | 106-134 | REAL | - | every toggle ships OFF per frame |
| ColorsHexRow | 137-178 | REAL | - | input + swatch (data-driven inline style, justified) + Toggle, labelled |
| ColorsToggleRow | 181-193 | REAL | - | |
| ColorsTool | 196-343 | SHALLOW | S3 | local state only; see F-001 |

## 5. Test honesty
ColorsTool.test.tsx (244 LOC): tests the interactive shell honestly —
expand/collapse, hex defaults, slider aria attributes, toggle flips, staged
markers. No test pretends engine effects happen. Would fail if the staged
markers or aria wiring regressed. Honest about what it tests (header line 5:
"local-state interactions, and the honest staged markers").

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no ... engine stage or catalog command exists" | ColorsTool.tsx:28-29 | REAL | rg video.set-colors / color effects in schemas+crates: no hits |
| staged note copy "Staged — controls activate with the video effects command card" | messages.ts:2429 | REAL | rendered by VideoCard.tsx staged <p role="note"> |
| frame-measured initials (20.73%/33.54%) | ColorsTool.tsx:55-57 | UNVERIFIED | needs the Figma frame; consistency between comment, code (21.6/34.55) and test (aria-valuenow 21.6) verified |

## 7. Dead / duplicated / unreachable code
None inside the file; the entire component is a staged (disclosed-inert)
surface by design.

## 8. Dependencies & flags
Toggle, Dropdown, VideoCard, VideoSliderRow, useDropdownScrollportFlip, i18n
copy. No feature gate hides it — the Tools dialog mounts it unconditionally
(TOOLSBHELL/ToolsDialog wiring verified by consumer rg).

## 9. Open questions for the next agent
- Does ToolsDialog.tsx or effects.tsx carry any runtime gate (e.g. data-staged
  note visibility in compact hosts)? The note renders inside the card body —
  collapsed cards hide it until expanded.
