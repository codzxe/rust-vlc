# AUDIT: ui/src/screens/player/tools/video/EssentialTool.tsx
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 148 / 2 (EssentialTool + ESSENTIAL_ROWS table) |
| File verdict | REAL (code) — honest ghost: all 8 controls await `video.set-essential`, absent from the catalog; state is local-only |
| Reachable from shipped entry point? | NO as product (desktop-tauri cannot compile, W14 SUMMARY l.19). Dev/web: mounted by ToolsDialog.tsx (video tools) + the video harness; EssentialTool.test.tsx 10 behavioral cases; baseline rows 22-23 green. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg `set-essential` + `adjust|sharpen|gradfun|grain` in generated/commands.ts → ALL 0 hits (no such commands — the staging claim verified); rg `EssentialTool` consumers (ToolsDialog, harness, index); python arithmetic re-derivation of all 8 paint-percentage claims (below); read EssentialTool.test.tsx test list |

## 1. One-paragraph verdict
The most honest of the staged tools: the header says plainly "no adjust/sharpen/gradfun/grain engine stage or catalog command exists — local state + the visible staged markers, controls fully interactive", and every part of that verifies — the command is absent from the generated catalog, the card carries the pending-command marker, and the 8 sliders drive genuine local state with real keyboard/clamp behavior (tested). The initial-value calibration comments are arithmetically exact against the shared slider's flush-travel formula (all 8 recomputed: 48.8/48.8/48.8/33.65/10.27/1.87/42.63/20.15% vs the claimed 50/50/50/33.6/10.25/1.86/42.6/20.1). The single finding is the product-level one it shares with its siblings: a fully interactive 8-control video-adjustment card whose every adjustment is guaranteed to go nowhere — no command, no persistence, no consumer of the values.

## 2. What actually works (verified)
— verified by python arithmetic: paint% = 0.988·p − 0.6 (the L8 flush-travel center, W=328) applied to each row's initial — hue 50→48.8 ("≈50" per l.20-21's sanctioned −1.2pp), saturation 34.67→33.65 (claim 33.6, frame 33.5), gamma 11→10.27 (claim 10.25, frame 10.4), sharpen 2.5→1.87 (claim 1.86, frame 1.8), banding 43.75→42.63 (claim 42.6, frame 43.0), grain 21→20.15 (claim 20.1, frame 20.4). Every staged initial lands on its frame mark as documented.
— verified by rg → `video.set-essential` and every underlying engine concept (adjust/sharpen/gradfun/grain) have ZERO occurrences in generated/commands.ts: the pending marker names a real absence.
— verified by reading l.125-127 → state initialized from the row table; `values[row.id] ?? row.initial` fallback keeps rows total even if a row id drifts.
— verified by reading EssentialTool.test.tsx:24-163 → collapse/defaultOpen/frame order/tick counts/valuetext/keyboard commit/clamp-at-max/pending marker all behavioral; the clamp test (:154) exercises the range end.
— baseline rows 22-23: compiles, suite green.

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/player/tools/video/EssentialTool.tsx:124-147 (whole component; staging claim at l.26-28)
Code:
```
  return (
    <VideoCard
      tool="essential"
      nameKey="customize.panel.essential"
      pendingCommand="video.set-essential"
      {...props}
    >
```
What is wrong: The tool is an interactive dead end by design: 8 sliders (hue, brightness, contrast, saturation, gamma, sharpen sigma, banding radius, grain variance) whose values live in a local useState and are consumed by NOTHING — no catalog command exists (verified), no persistence seam is attached (unlike EqualizerTool, not even a dead onChange prop), and no other component imports the values. The card self-marks staged (`pendingCommand` + VideoCard's markers), so the honesty contract is kept — but the product ships a video-adjustment surface that can never adjust video, in any build, until an entire engine lane (adjust/sharpen/gradfun/grain stages + a `video.set-essential` command) is written.
Why it matters: This is the fourth member of the staged-tool family (equalizer/audio.set-eq per W8, dynamics, visualizer) — a complete, tested, frame-accurate UI layer floating over absent engine features. Individually honest, collectively the pattern the orchestrator's wave-15 brief names: "is any UI element wired to a real backend?" — for this family the answer is no.
What correct looks like: engine stages + command, or ship the card behind a feature flag until the lane exists.
Confidence: HIGH — Verification: catalog greps; consumer sweep; state-is-local read.
(Filed S3 consistent with the sibling staged-tool findings; the honesty devices keep it out of S2.)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ESSENTIAL_ROWS` (8 row defs) | 40-121 | REAL | — | every initial's paint% recomputed and matching its comment; formats (°, toFixed(2/1), px, %) consistent with steps; label keys reuse central customize.field.* (l.23-24 verified: hue/brightness/contrast/saturation/gamma keys match the customize lane's) |
| `EssentialTool` | 124-148 | REAL | S3 | F-001; state init from table; spread `{...props}` forwards VideoToolProps (defaultOpen etc. — ToolsDialog uses it) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| :24-41 collapse/defaultOpen | YES | behavioral |
| :43-63 eight rows in frame order | YES | order + labels asserted |
| :65-82 per-row tick counts | YES | structural counts |
| :84-115 tick paint domain (j11 P2-1 regression) | YES | pins the 2px-inset fill domain — would catch a travel-formula regression |
| :117-135 staged ranges + valuetext + label-only scoping | YES | aria-valuetext from `format` asserted |
| :147-161 keyboard commit + clamp-at-max | YES | the one true interaction test — values actually change and clamp |
| :163-168 pending-command marker | YES | the honesty marker is test-enforced |
Mutation check (reasoned): change gamma's initial to 5.0 → the valuetext/range test fails; drop the pendingCommand prop → the marker test fails. The suite is real.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "no adjust/sharpen/gradfun/grain engine stage or catalog command exists — local state + the visible staged markers, controls fully interactive" | l.26-28 | REAL | all greps 0-hit; local state verified; markers test-enforced |
| "a value v paints its thumb center at ≈ 0.988·v − 0.6 % of the track" | l.16-17 | REAL | arithmetic: (1−4/328)·p − 200/328 = 0.9878p − 0.61 |
| "thumb centers 50/50/50/33.5/10.4/1.8/43.0/20.4%" (frame measurements) | l.18-19 | REAL (as calibration targets) | all 8 initials paint within 0.4pp of them (recomputed) |
| "Engine ranges replace these on wiring (types.ts note)" | l.22 | REAL (promise) | types.ts carries the note (sibling file, W15-A10's queue) |
| "Hue/Brightness/… labels REUSE the central customize.field.* keys" | l.23-24 | REAL | key strings match the customize lane's field keys |

## 7. Dead / duplicated / unreachable code
No dead code in-file (props forwarded, rows all rendered). Unreachable-as-product: the entire tool (W14 + absent engine lane). Duplication: none — it composes VideoCard/VideoSliderRow rather than restating them.

## 8. Dependencies & flags
VideoCard (staging markers + expand grammar), VideoSliderRow (slider + ticks + valuetext), types (VideoToolProps/VideoSliderRowDef), no transport import (cannot dispatch — honest), no i18n import here (label keys resolve in the row). No feature flags. CSS: video.css (sibling audit).

## 9. Open questions for the next agent
- VideoCard.tsx owner: does the card render the pending-command marker text (the command NAME) to users, or only data attributes + the staged note? If the raw command id is user-visible, that is a copy defect.
- If the engine lane ever lands: the staged initials are display-calibrated (e.g. saturation 1.04 to paint 33.6%), NOT engine defaults — the wiring must map to real neutral values (1.0) or the first real apply will shift every control.
