# AUDIT: ui/src/screens/player/tools/equalizer/equalizer.css
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 461 / 3 skins (eq-tool 37-287, eq-panel 291-452, motion 456-461) |
| File verdict | REAL (skins a fully UNWIRED tool) |
| Reachable from shipped entry point? | YES (ToolsDialog.tsx:122 `<EqualizerTool />` -> EqualizerTool.tsx imports equalizer.css; all 10 class families have TSX consumers) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 0 |
| Commands run | rg class usage (all consumed); rg PENDING_COMMAND / onChange wiring; sed quote verification |

## 1. One-paragraph verdict
Meticulous frame-ported stylesheet — every px carries an EQ.svg/EQ-presets.svg
citation, all selectors consumed, one documented hex exception (0 dB tick
#1a1611, line 418, with stylelint-disable + token-lane handoff note). The
hostile finding is not the CSS but what it dresses: the ENTIRE equalizer UI
(11 vertical faders, preamp, 8 presets with hand-painted CSS curve glyphs) is
not wired to any audio backend. EqualizerTool.tsx:48 declares
`PENDING_COMMAND = "audio.set-eq"` — a forward-declared command that DOES NOT
EXIST ("no `audio.set-eq` command exists yet", line 47) — and the production
mount ToolsDialog.tsx:122 renders `<EqualizerTool />` with NO onChange, so
fader commits and preset picks go nowhere. The tool self-labels with
data-staged-tool="equalizer" and a visible staged note; the CSS class
`.eq-tool__staged` exists precisely to style that confession.

## 2. What actually works (verified)
— rg class check: eq-tool, eq-tool__staged, eq-tool__presets, eq-panel and all
its sub-elements have non-test TSX consumers (EqualizerTool.tsx, GraphicEQ.tsx).
— All 7 data-preset glyph selectors match real preset ids in presets.ts
(classical/club/dance/full-bass/full-bass-treble/full-treble verified lines
33-60; "none"/"flat" intentionally rule-less, documented lines 154-155).
— Preset curve glyphs painted via ::before + custom properties — no DOM
pollution of the shared Dropdown (documented g15-P3-2).

## 3. Findings
F-001 [S2]
Where: ui/src/screens/player/tools/equalizer/equalizer.css:44-50 (and 25-28 claim) — skin for EqualizerTool.tsx:46-53
Code:
```
/* Staged note — the honest-unbound marker (the dynamics/visualizer/video
 * in-body pattern): quiet caption line, never styled like an engine status. */
.eq-tool__staged {
```
What is wrong: This sheet implements the complete visual equalizer — faders,
0 dB ticks, value chips, preset selector with per-preset response-curve glyphs
— for a tool whose every control is dead: `audio.set-eq` does not exist
(EqualizerTool.tsx:47-48) and the shipped mount passes no onChange
(ToolsDialog.tsx:122). A user can drag 11 faders and pick presets; zero audio
processing results. The honesty is a quiet caption (`.eq-tool__staged`,
role="note") that most users will not read as "this does nothing".
Why it matters: largest dead interactive surface in the tools dialog; the
frame-perfect CSS amplifies the impression of a working feature.
What correct looks like: gate the tool behind the command's existence, or
render faders inert (disabled visuals) until the seam lands.
Confidence: HIGH — Verification: read EqualizerTool.tsx + ToolsDialog.tsx + rg.

F-002 [S3]
Where: ui/src/screens/player/tools/equalizer/equalizer.css:417-418
Code:
```
  /* stylelint-disable-next-line color-no-hex -- frame: EQ.svg Vector 60 stroke #1A1611 — the 0 dB tick ink; no semantic token exists (token-lane handoff, w26.f05 lane log) */
  background: #1a1611;
```
What is wrong: The header (lines 27-28) claims "Colors are var(--…) everywhere
except the 0 dB tick" — accurate for THIS sheet, but this is the third
distinct handoff-excuse pattern in the player skins (states.css pins 18 hex
values "awaiting a stays-dark token"; this one awaits a tick-ink token). The
token lane never lands; the exceptions accumulate. Debt, not a defect here.
Confidence: HIGH — Verification: read.

## 4. Per-function (selector block) table
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| .eq-tool + __staged + __presets (dropdown scope) | 37-141 | REAL | F-001 | staged-note styling for the unbound confession |
| preset curve glyph ::before rules (7 presets) | 143-287 | REAL | - | all data-preset ids verified against presets.ts |
| .eq-panel + __toggle/__chevron | 291-346 | REAL | - | morph bar/panel |
| .eq-panel__body/__rail/__col/__fader | 348-401 | REAL | - | max-width Lead fix documented (369-374) |
| 0 dB tick on fill ::after | 403-420 | REAL | F-002 | single documented hex exception |
| __label/__chip + reduced-motion | 422-461 | REAL | - | WCAG Lead ruling documented (422-427) |

## 5. Test honesty
EqualizerTool.test.tsx / GraphicEQ.test.tsx / harness.test.tsx owned by other
agents; harness.test.tsx:86 pins states.css-adjacent strings — presence tests.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no `audio.set-eq` command exists yet" | EqualizerTool.tsx:47 | REAL (the command is absent) | rg audio.set-eq in ui/src -> only the forward declaration |
| "Colors are var(--…) everywhere except the 0 dB tick" | equalizer.css:27-28 | REAL | hex scan: single #1a1611 in rules (line 418) |

## 7. Dead / duplicated / unreachable code
No dead selectors. `.tools-dialog__staged-tool .eq-tool__staged { display: none }`
(57-59) is a TRANSITIONAL suppression rule with an explicit deletion plan —
tracked debt, both branches verified live (ToolsDialog renders the wrapper).

## 8. Dependencies & flags
components/slider (lumen-slider--vertical, EQSlider), components/menu
(lumen-dropdown__*), form lane ValueChip (composed). Tokens from tokens.css.

## 9. Open questions for the next agent
Verify in the engine (crates/) that no equalizer/eq filter exists at all — the
UI's "staged" claim implies the engine lacks it; an engine-side audit row
should cross-reference `audio.set-eq`.
