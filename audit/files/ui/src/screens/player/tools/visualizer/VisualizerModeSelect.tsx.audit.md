# AUDIT: ui/src/screens/player/tools/visualizer/VisualizerModeSelect.tsx (compact, 84 LOC — spec §13)
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 84 / 1 (VisualizerModeSelect) |
| File verdict | REAL (thin composition over the shared Dropdown; the FEATURE it selects is data-dead by design) |
| Reachable from shipped entry point? | YES — the visualizer tool panel inside the player's tools dialog (VisualizerTool.tsx:93 renders the display; the select is the tool's mode row) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg "levels" VisualizerTool.tsx types.ts` -> levels is a host-owned seam, no FFT source; `rg synthetic\|animated\|Math.random` over visualizer/ -> honest markers only |

## 1. One-paragraph verdict
The control is real: five frame modes, shared Dropdown combobox model
(arrows/typeahead/Escape per the w26.k10 primitive), commit seam to the
parent tool, i18n through the lane resolver. The feature behind it is
honestly dead: no FFT/levels source exists (types.ts:14-15 "no FFT/levels
source exists yet, so absent levels render the frame's RESTING depiction"),
and the catalog has no audio-sample/spectrum command — so the dropdown's
entire effect is choosing which STATIC glyph decorates the panel. The
honesty markers are everywhere (test :227 "an empty levels array stays
RESTING (honest: no data, no fake bars)"), which keeps this out of fraud
territory, but the user-facing truth is: a mode selector for a visualizer
that can never visualize.

## 2. What actually works (verified)
— Dropdown composition verified (imports, options mapping, onChange cast).
— VisualizerTool.test.tsx (colocated, vitest baseline) pins selection
behavior and the resting-display honesty.
— Mode icons are decorative SVGs (ModeIcons.tsx:16) — no fabricated data.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/tools/visualizer/VisualizerModeSelect.tsx:60-69 (cross types.ts:14-15)
Code:
  const options: DropdownOption<VisualizerLabelKey>[] = VISUALIZER_MODES.map(
    (mode) => ({
      value: mode.id,
      labelKey: mode.labelKey,
      iconNode: mode.icon === undefined ? undefined : <mode.icon />,
What is wrong: a fully interactive five-option combobox whose entire
effect is cosmetic — no levels source exists (engine has no audio-data
command; audio output itself is the null backend per ADR-005), so every
mode renders the same RESTING static depiction shape. Dead surface behind
a real control; the honesty markers ("never simulated") make it
non-fraudulent but it is still a shipped non-feature. What correct looks
like: hide the mode row until a levels seam exists, or ship it labeled as
a preview. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table (compact)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| VisualizerModeSelect | 52-84 | REAL | S3: cosmetic-only effect (F-001) |

## 5. Test honesty
VisualizerTool.test.tsx:227 pins the no-data honesty (empty levels ->
resting). No test can fail on "visualizer has no data source" because that
is the shipped truth, not a regression.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the primitive carries the full combobox interaction model" | :6-10 | REAL-UNPROVEN | Dropdown.tsx is another agent's file; claimed behaviors (typeahead, flip) not re-verified here |
| "never animated, never simulated — no idle shimmer" | types.ts:16 | REAL | rg finds no animation/random source in the lane |

## 7. Dead / duplicated / unreachable code
The old 230-line lane-scoped combobox is claimed GONE (:2-4) — verified:
no duplicate Dropdown implementation in this directory (ls: only the
composition + icons + display).

## 8. Dependencies & flags
components/menu Dropdown, lane types (VISUALIZER_MODES). No flags.

## 9. Open questions for the next agent
- Dropdown.tsx auditor: verify the claimed interaction model (roving
  tabindex with the same no-match hole as SegmentedControl? typeahead?
  focus restoration) — the whole visualizer row leans on it.
