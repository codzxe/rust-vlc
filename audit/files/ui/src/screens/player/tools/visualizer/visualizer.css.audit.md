# AUDIT: ui/src/screens/player/tools/visualizer/visualizer.css
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 319 / 27 selector blocks |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle — the visualizer card mounts in the player Tools dialog (ToolsDialog.tsx:130 `<VisualizerTool />`, importing this sheet via VisualizerTool.tsx:31). W14 caveat: no shipped desktop binary (build-desktop-tauri.log.txt exit 101). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg -o 'viz-*' over the visualizer dir (all 27 blocks' classes consumed: viz-tool family in VisualizerTool.tsx, viz-display family in VisualizerDisplay.tsx, viz-select in VisualizerModeSelect.tsx, viz-tone-* in ModeIcons/VisualizerDisplay); token check (--color-eq-1..7, --color-accent-strong, --font-size-section all exist); rg 'VisualizerTool' src — the ONLY production mount is ToolsDialog.tsx:130 with NO levels/onChange props. |

## 1. One-paragraph verdict
A competent, fully-consumed tool-card stylesheet whose LIVE-state lane is dead in every production mount: the sole production consumer (ToolsDialog.tsx:130) passes no `levels`, no analysis source exists anywhere in the UI or the command catalog (`audio.set-visualizer` is a forward-declared name, not a catalog command), so the display is permanently `data-state="resting"` — the seven eq-token cycling rules (189-195) style bars that can never render (the `data-accent` attribute is emitted only when live, VisualizerDisplay.tsx:124-127). The peak-dot rule (199-206) IS reachable — the resting spectrum depiction renders peak spans too (VisualizerDisplay.tsx:130-135) — corrected after initial overreach. The card/selector/resting-depiction styling is real, frame-measured, and correctly theme-mapped (the light block documents its Δ-ramp reasoning). The honest-staging design (staged note rule at 118-122) means this is documented debt, not deception.

## 2. What actually works (verified)
— class↔component cross-check (all 27 blocks consumed); token existence verified; reduced-motion guard covers exactly the three transitioning selectors (.viz-tool:35, .viz-tool__toggle:65-67, .viz-tool__chevron:96). Baseline: visualizer suites pass incl. harness visual tests (ui-vitest.log.txt 1705 total); `tsc --noEmit` clean (ui-build.log.txt).

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/player/tools/visualizer/visualizer.css:189-195
Code:
```
.viz-display__bar[data-accent="1"] { background: var(--color-eq-1); }
.viz-display__bar[data-accent="2"] { background: var(--color-eq-2); }
```
What is wrong: the live-Spectrum eq-token cycling is production-unreachable. `data-accent` is emitted only when `live` is true (VisualizerDisplay.tsx:124-127 — `mode === "spectrum" && live ? { "data-accent": … } : { "data-tone": … }`), which requires a non-empty `levels` prop — and the only production mount (ToolsDialog.tsx:130) passes none; no FFT/analysis source exists (VisualizerDisplay.tsx:3-4 "no FFT / audio-analysis source exists yet"); the awaited `audio.set-visualizer` (VisualizerTool.tsx:34) is not in the generated command catalog (generated/commands.ts:24 union has no visualizer command). At rest, spectrum bars carry `data-tone="gray"` (all-gray — VisualizerDisplay.tsx:122-123), so lines 189-195 never match in production.
Why it matters: the "Audio Visualizer" never visualizes audio in any shipped build — mode selection is cosmetic (switches which static resting depiction shows, local state only, no onChange consumer in production); the eq-token accent work (7 tokens, harness states, tests) exercises nothing real.
What correct looks like: land the analysis source (engine levels event or WebAudio tap) + the catalog command, or ship the tool as a static "coming" depiction and drop the live lane.
Confidence: HIGH — mount site, catalog union, and component logic all read; tone emission traced line-by-line.

## 4. Per-function table (selector blocks)
| Selector block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.viz-tool` / `--open` | 26-42 | REAL | — | morphing surface; Δ2 token mapping documented |
| `.viz-tool__toggle` (+open height/hover/focus-visible) | 47-84 | REAL | — | house hover grammar documented (a19 B14) |
| `.viz-tool__chevron` (+expanded rotate) | 92-101 | REAL | — | −7.5px calibration documented (w26.i07) |
| `.viz-tool__body` / `[hidden]` | 105-114 | REAL | — | |
| `.viz-tool__staged` | 118-122 | REAL | — | the honest-unbound marker |
| `.viz-display` | 129-139 | REAL | — | 64px height is house choice [VERIFY] — flagged in-file |
| `.viz-display__bars` / `__slot` (+data-align center) | 143-168 | REAL | — | |
| `.viz-display__bar` (+tone gray/accent) | 172-185 | REAL | — | renders at rest (restingLevels) too |
| `[data-accent="1".."7"]` | 189-195 | UNREACHABLE-in-production | S3 (F-001) | live-only tone emission |
| `.viz-display__peak` | 199-206 | REAL | — | renders in the resting spectrum depiction too (VisualizerDisplay.tsx:130-135) |
| `.viz-display__dial` | 209-211 | REAL | — | spectrometer resting state IS reachable |
| `.viz-tone-gray` / `.viz-tone-accent` | 215-221 | REAL | — | used by ModeIcons + dial |
| `.viz-select` + dropdown trigger/value/option composition | 230-270 | REAL | — | shared-Dropdown composition, frame-measured |
| `@media reduced-motion` | 276-282 | REAL | — | covers exactly the 3 transitioning selectors |
| light-theme block | 299-319 | REAL | — | Δ-ramp reasoning documented; open-card restated (specificity noted) |

## 5. Test honesty
VisualizerTool.test.tsx / harness.test.tsx belong to other agents; they render the live lane with literal levels (harness.tsx) — the only thing exercising F-001's dead styles.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "live Spectrum bars cycle the eq token group" | visualizer.css:16-17, 187-188 | SHALLOW | the css is real; "live" never happens in production (F-001) |
| "Semantic tokens only … raw px below are frame-measured structural dimensions" | visualizer.css:19-22 | REAL | no raw hex in declarations (frame hexes live in comments as claimed) |
| "the well height carries no frame measure [VERIFY]" | visualizer.css:22 | REAL | honest open question, in-file |

## 7. Dead / duplicated / unreachable code
F-001 (live lane). No duplication found.

## 8. Dependencies & flags
Shared Dropdown grammar (`.lumen-dropdown__trigger/__value/__option`); tokens --color-eq-1..7 etc. all exist. Depends on VisualizerDisplay/ModeIcons for class consumers.

## 9. Open questions for the next agent
- VisualizerTool.tsx owner: the staged marker names `audio.set-visualizer` — if ANY doc/matrix row claims a working visualizer, this is an S0 doc lie (check matrix rows for visualizer; none seen in my slice).
- Does the equalizer sibling (audio.set-eq, W8's nonexistent backend) share the staged-marker pattern? Cross-check for a systemic count of forward-declared commands.
