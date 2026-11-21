# AUDIT: ui/src/screens/player/tools/visualizer/harness.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 91 / 3 (HarnessCell, VisualizerSelectorStates, VisualizerToolStates) |
| File verdict | REAL (dev-only harness WITH a genuine committed verification record behind it — the c3 lane log's comparison loop ran two iterations, found and fixed real gaps) |
| Reachable from shipped entry point? | NO — dev-only by design ("never imported by app routes", :1); consumers: harness.test.tsx only. The tool it mounts (VisualizerTool) IS production-reachable via ToolsDialog.tsx. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `sed -n '60,95p' docs/waves/w26/logs/w26.c3.md` (comparison loop record EXISTS — 2 iterations, real fixes, probe numbers); `ls design/figma-renders/Audio_Visualizer.png` (reference exists); scale-math check (406 × 2.2167 = 899.98 ≈ 900 ✓); `grep -rn "levels" VisualizerTool.tsx ToolsDialog.tsx` (tool honest about no-FFT; ToolsDialog mounts WITHOUT levels); `grep -rn "color-eq" visualizer.css` (bars use var(--color-eq-1..7) — single-source tokens, answering my tokens.rs open question); baseline #23 |

## 1. One-paragraph verdict
The best of the three harnesses in my assignment: unlike the subtitles harness (whose vision loop was owed and never ran — my file-13 audit), this one's lane log contains a real two-iteration comparison record — a genuine gap found and fixed (trigger text-align, glyph boxes nudged −7px/−5px, caption removed, stage gutter pinned to 22px), deterministic computed-style probes, and a VLM match list — against a figma render that actually exists (Audio_Visualizer.png). The harness code itself is minimal and follows the c2 pattern (captions as props, frame-exact cell without caption so the shot composes 1:1). The one honest-shadow finding is inherited from the tool it mounts: the harness's "spectrum with live levels" state is fixture-only — production's ToolsDialog mounts VisualizerTool with no levels, and the tool's own header admits there is no FFT source, so the shipped visualizer is permanently a resting depiction; the live animation validated here is a state no engine can produce.

## 2. What actually works (verified)
— verified by docs/waves/w26/logs/w26.c3.md:68-95 → the loop record exists with concrete artifacts: "Iter 1 — GAPS — one REAL gap found (trigger text CENTERED … frame's label is flush-left) → fixed (text-align: start, probe now x39 start)"; "Iter 2 … Treated as grammar-MATCH on deterministic evidence". This is what a vision-verification trail should look like.
— verified by `ls design/figma-renders/` → Audio_Visualizer.png present (the comparison reference is committed, unlike the b11 lane's initial claim).
— verified by read → the frame cell omits its caption so the shot composes against the render 1:1 (:47-49); the open-state wrapper documents the a19 anchoring deviation with the lane-log pointer (:39-41).
— verified by `grep -rn "color-eq" visualizer.css` → spectrum bars consume `var(--color-eq-1..7)` from generated tokens.css — one source (tokens.json) feeds both the CSS the UI uses and the (dead) Rust constants; no hex duplication on the UI side. Closes the drift question from my generated/tokens.rs audit.

## 3. Findings

### F-001 [S4]
Where: ui/src/screens/player/tools/visualizer/harness.tsx:81-85 vs ToolsDialog.tsx (no levels) and VisualizerTool.tsx:8-9
Code:
```
        <VisualizerTool
          defaultOpen
          defaultMode="spectrum"
          levels={[0.2, 0.55, 0.35, 0.9, 0.5, 0.72, 0.4, 0.65, 0.3, 0.84, 0.45, 0.6]}
        />
```
What is wrong: the harness validates a live-levels spectrum state that no production mount can reach: ToolsDialog.tsx mounts VisualizerTool with no `levels` prop, and the tool's own header states "honest: no FFT source — absent levels render the mode's RESTING frame depiction". The engine has no audio-levels event to feed the seam, so the animated bars are harness/test-only decoration of a state the shipped visualizer never enters.
Why it matters: the vision loop proves fidelity of the livelier state while the state users actually get (resting depiction) gets one collapsed/expanded cell — verification emphasis inverted from shipped reality; mild because the resting state IS also mounted here (:79-80, :86).
What correct looks like: keep the live state (it pins the levels-rendering code), but the lane record should state plainly that live levels have no engine source at HEAD — which c3's log does via the tool's honesty note, so this is a documentation-grade gap only.
Confidence: HIGH — Verification: ToolsDialog grep (no levels), VisualizerTool header read, engine events census (my PlaylistPanel/DiscoveryScreen audits: no audio-level event in the catalog).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `HarnessCell` | 22-34 | REAL | — | caption-optional cell (c2 pattern); frame-exact cell omits it |
| `VisualizerSelectorStates` | 43-67 | REAL | — | two stacked selector states at frame offsets; second opened by the test before serializing |
| `VisualizerToolStates` | 75-90 | REAL | S4 | 4 tool states incl. fixture-levels spectrum (F-001) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| harness.test.tsx render-no-crash + env-gated export | crash-level only | same pattern as the other harnesses; the real assertions live in the c3 lane record (probes + 2 iteration verdicts) |
| c3 lane log loop record | YES as evidence | iteration 1 FOUND a real gap — the loop demonstrably detects drift, not a rubber stamp |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "see the lane log for the loop record" | harness.tsx:9 | REAL | w26.c3.md:68-95 contains the 2-iteration record |
| "screenshot them against design/figma-renders/Audio_Visualizer.png" | harness.tsx:4-5 | REAL | the PNG exists and the loop used it |
| "406px canvas × 2.2167 = the render's 900px width" | harness.tsx:8-9 | REAL | arithmetic checks (899.98) |

## 7. Dead / duplicated / unreachable code
None in-file. The live-levels tool state is production-unreachable (F-001, by honest design).

## 8. Dependencies & flags
VisualizerModeSelect, VisualizerTool, useVisualizerMsg, harness.css; eslint-disable not needed here (captions are prop pass-throughs per the c2 pattern — cleaner than the subtitles harness's inline-literal disables).

## 9. Open questions for the next agent
- If the engine ever gains an audio-levels event, the harness's fixture array (:84) is the shape to match — the tools lane should record the expected levels semantics (0..1 floats, band count 12?) when wiring it.
- The c3 log's "font identity … [VERIFY]" non-blocking class — has anyone closed it? Third harness lane (c1/c2/c3) all defer font weights; typography verification is a wave-wide hole.
