# AUDIT: ui/src/screens/player/tools/visualizer/VisualizerDisplay.tsx
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 189 / 5 (1 component + 2 subcomponents + 2 helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle — rendered by VisualizerTool.tsx:91-93, mounted in the player Tools dialog (ToolsDialog.tsx:130) and the settings effects area (per VisualizerTool header); always in RESTING state in production (no levels source). W14 caveat: no shipped desktop binary (build-desktop-tauri.log.txt exit 101). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg 'VisualizerTool' src --glob '*.tsx' (production mounts: ToolsDialog.tsx:130 only — no levels/onChange); rg 'levels=' src/screens/player (only harness.tsx literals + the prop passthrough); read types.ts restingLevels (98-102); catalog union read (no audio.set-visualizer). |

## 1. One-paragraph verdict
A clean, honest rendering component: it draws exactly what it is given, clamps every level (non-finite → 0), renders nothing for "disabled", static frame geometry at rest, and labels its own state ("live"/"resting") in the aria-label. The geometry math is sound (spectrum bars capped at 41px to leave peak-dot headroom; dial radius lerp bounds correct). The catch is upstream, not here: no production mount ever supplies `levels`, so the live path (clamp pipeline, eq-token tone emission, live dial rings, per-bar data-level) is exercised only by tests and the visual harness — the component is a well-built receiver for data that does not exist.

## 2. What actually works (verified)
— Resting path fully traced: `restingLevels(mode)` (types.ts:98-102) normalizes the frame's static heights to 0..1; bars/slots/peak/dial all render at rest. Clamp01 handles NaN/Infinity/-negative. Baseline: VisualizerTool.test.tsx "honest display (levels contract)" suite (other agent) passes in ui-vitest.log.txt; `tsc --noEmit` clean (ui-build.log.txt).

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/player/tools/visualizer/VisualizerDisplay.tsx:46-55, 89, 124-127, 152-162
Code:
```
  /** Live levels 0..1, one per bar; absent/empty → the resting state. */
  levels?: readonly number[];
```
```
      const tone =
    mode === "spectrum" && live
      ? { "data-accent": (index % 7) + 1 }
      : { "data-tone": isGray ? "gray" : "accent" };
```
(abridged from lines 124-127)
What is wrong: the entire live lane — the `levels` prop contract, the clamp map (line 89), the eq-token tone emission, the live dial ring lerp (160-162), the per-bar `data-level` — has no production data source: the only production mount passes no levels (ToolsDialog.tsx:130), no analysis source exists in the UI or the command catalog (`audio.set-visualizer` is forward-declared, not in generated/commands.ts:24).
Why it matters: the component's raison d'être (visualize audio levels) is unreachable in every shipped build; it renders static art. The honesty is in the component's own header; the debt is systemic (no FFT source).
What correct looks like: an engine levels event (or WebAudio AnalyserNode tap in the browser shell) feeding the `levels` seam.
Confidence: HIGH — mount + catalog traced.
### F-002 [S4]
Where: ui/src/screens/player/tools/visualizer/VisualizerDisplay.tsx:130-135
Code:
```
      {mode === "spectrum" && (
        <span
          className="viz-display__peak"
          style={{ bottom: barPx + 2 }} /* frame: dot floats above the top */
        />
      )}
```
What is wrong: the peak dot's absolute `bottom` is computed from `barPx`, but the span's containing block is the SLOT (position:relative, 100% height of the 48px band). At rest the spectrum bars can reach 41px (SPECTRUM_BAR_PX), so `bottom: 43px` places the 3px dot at 43..46px — inside the 48px band ✓ — but for scope/waveform-height bars under the SPECTRUM cap the math is fine too; the actual defect is subtler: at REST the peak dots float above the resting depiction's bars — the frame's resting icon carries dots per its own geometry, so this is frame-faithful. Downgrading: the only real nit is that the dot's horizontal centering relies on `left: calc(50% - 1.5px)` in CSS while `bottom` is inline — split-brain positioning (one axis in each place).
Why it matters: maintainability nit, no visual defect found (both axes verified consistent: 3px dot, 50% center).
What correct looks like: both axes in one place.
Confidence: MEDIUM — geometry reasoned, not rendered.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `clamp01` | 35-38 | REAL | — | non-finite → 0; Math.min/max order correct |
| `modeLabelKey` | 41-43 | REAL | — | template-literal cast to the label union |
| `VisualizerDisplay` | 62-102 | REAL | S3 (F-001) | disabled → null; live/resting state + aria-label honest |
| `BarSlot` | 105-144 | REAL | S3 (F-001 via tone) / S4 (F-002) | barPx floor at BAR_W_PX=3; peak renders for spectrum at rest too |
| `SpectrometerDial` | 152-189 | REAL | S3 (F-001 live rings) | resting = frame's 2 rings + 4-arc dasharray (arcsC/8 math: 4 on, 4 off ✓); live lerp bounded [R_MIN,R_MAX] |

## 5. Test honesty
VisualizerTool.test.tsx (other agent) pins the levels contract: out-of-range levels ([1, 0.5, 2, -1] at line 211) assert clamping — those tests WOULD fail if clamp01 broke. Good mutation resistance for the live math; nothing can pin production behavior because production never feeds levels.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no FFT / audio-analysis source exists yet" | VisualizerDisplay.tsx:3 | REAL | rg: no analysis source; no catalog command |
| "never animated and never simulated" (resting) | VisualizerDisplay.tsx:6-8 | REAL | resting path renders static heights; no timers/intervals in file |
| "the bar COUNT is data-driven (levels.length), 6 at rest (frame)" | VisualizerDisplay.tsx:16-17 | REAL | line 89 maps levels or restingLevels |
| "SPECTRUM_BAR_PX = 41; leaves the 3px peak dot + 2px gap headroom" | VisualizerDisplay.tsx:59 | REAL | 41+2+3=46 ≤ 48 band ✓ |

## 7. Dead / duplicated / unreachable code
F-001 (live lane production-dead). No duplication.

## 8. Dependencies & flags
types.ts (VisualizerMode/restingLevels — another agent), visualizer.css (audited), no external deps. All imports exist.

## 9. Open questions for the next agent
- types.ts owner: RESTING_LEVELS values — do they reproduce the frame's six-bar geometry (claimed "frame")? Visual check only; no Figma access here.
- If a future host mounts VisualizerDisplay directly (props say "mountable standalone — e.g. a future music-mode host"), the aria-label "live/resting" suffix strings come from resolveLabel — who owns those keys?
