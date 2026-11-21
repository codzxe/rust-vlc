# AUDIT: ui/src/screens/player/tools/visualizer/VisualizerTool.tsx
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 111 / 2 (VisualizerTool, commitMode) |
| File verdict | REAL (code) — honest ghost: awaits `audio.set-visualizer` (absent from catalog); its data and onChange seams have NO consumer |
| Reachable from shipped entry point? | NO as product (desktop-tauri cannot compile, W14 SUMMARY l.19). Dev/web: mounted by ToolsDialog.tsx:130 (bare — audio tab); baseline rows 22-23 green. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg `set-visualizer` in generated/commands.ts → 0 hits; rg `VisualizerTool` consumers → ONLY ToolsDialog.tsx:130 (bare mount, no props); rg `Visualizer` in effects.tsx / MiniPlayer.tsx / PlayerScreen.tsx / variants/ → 0 hits (the header's claimed mount points do not exist); read the component |

## 1. One-paragraph verdict
Same family, same verdict as its siblings: a clean, accessible (aria-expanded/controls, labelled group, role=note staging) tool card whose mode selector is fully interactive and connected to nothing — `audio.set-visualizer` does not exist in the catalog (verified), and the only consumer mounts it bare, so the `levels` data seam never receives audio data (the display can only ever render its static "resting frame depiction") and `onChange` never fires. The header's claim that the Settings effects area and "a16's music mode" mount this tool is FALSE at HEAD — nothing but the tools dialog renders it, which also means the visualizer is unreachable in the settings surface the comment advertises.

## 2. What actually works (verified)
— verified by reading l.55-108 → a11y is complete: button toggle with aria-expanded/aria-controls, labelled body group, staged note with role="note", data-staged-tool/data-pending-command markers.
— verified by reading l.89-96 → the display renders NOTHING while mode="disabled" (default) and only resting depictions otherwise — the "never simulated data" honesty claim is structural (no levels ⇒ no animation).
— verified by rg → `audio.set-visualizer` absent from generated/commands.ts: the staging marker names a real absence.
— baseline rows 22-23: compiles, suite green (VisualizerModeSelect/VisualizerDisplay suites owned by other agents).

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/player/tools/visualizer/VisualizerTool.tsx:18-21 (claim) and 35-53 (seams)
Code:
```
// Mountable as-is (tools dialog via ToolsDialogShell, the Settings → Audio
// effects area, a16's music mode): all state is local; `onChange` is the
// wire seam; `levels` is the host-owned data seam (readonly — push a new
// array per animation frame when the analysis source lands).
```
What is wrong: Every named seam is dead. The ONLY consumer is ToolsDialog.tsx:130 — `<VisualizerTool />` with no props: no `levels` (so the display can never leave its resting depiction — there is no analysis source anywhere in the tree to push levels), no `onChange` (mode changes evaporate), no `defaultMode`/`defaultOpen`. The claimed additional mount points do not exist: rg finds zero Visualizer references in effects.tsx, MiniPlayer.tsx, PlayerScreen.tsx, or variants/ (the "a16's music mode" surface).
Why it matters: The header advertises a wiring story (effects area, music mode, wire seam, data seam) that is four-fifths fiction at HEAD. The component is a self-contained demo inside one dialog — a visualizer that cannot visualize (no data source exists) and a mode selector that cannot affect playback (no command exists).
What correct looks like: either the advertised hosts mount it with seams, or the comment names ToolsDialog as the sole host and drops the seam framing until the engine lane lands.
Confidence: HIGH — Verification: consumer sweep across the whole ui/src; catalog grep.
(The staged markers themselves are honest — same S3 calibration as EqualizerTool/EssentialTool: interactive-but-inert by design, never faking engine binding.)

### F-002 [S4]
Where: ui/src/screens/player/tools/visualizer/VisualizerTool.tsx:105-107
Code:
```
        <p className="viz-tool__staged" role="note">
          {m("player.tools.visualizer.staged")}
        </p>
```
What is wrong: The staged note renders whenever the body is open — including mode="disabled", where the display area is empty and the user has touched nothing. The sibling EqualizerTool renders its note only when open too (same), but here the note sits under a mode dropdown defaulted to "disabled", so the FIRST thing an expanding user reads is a staging disclaimer for a feature they have not engaged.
Why it matters: Presentation-order nit; the note's content is correct, its placement is premature.
What correct looks like: note only when mode !== "disabled", or keep as-is deliberately (honesty-first). Judgement call — recorded as hygiene.
Confidence: HIGH on behavior; the severity is deliberately minimal.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PENDING_COMMAND` | 32-33 | REAL | — | "audio.set-visualizer" — absent from catalog (verified) |
| `VisualizerTool` (state/ids) | 35-48 | REAL | — | useId-based stable ids; idProp override supported (no consumer passes it) |
| `commitMode` | 50-53 | REAL | S3 | local set + dead onChange (F-001) |
| render (root+markers) | 55-63 | REAL | — | section + aria-label + staged markers |
| toggle button | 65-81 | REAL | — | aria-expanded/controls; shared chevron rotation grammar |
| body group | 82-108 | REAL | — | hidden when closed; labelled group |
| `VisualizerDisplay` mount | 91-96 | REAL | — | no levels prop passed by any host → resting depiction only (F-001) |
| `VisualizerModeSelect` mount | 98-104 | REAL | — | the frame's five-mode dropdown, fully interactive locally |
| staged note | 105-107 | REAL | S4 | F-002 placement |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| VisualizerTool has no dedicated test file in this directory (VisualizerModeSelect.test/VisualizerDisplay suites cover the children) | UNVERIFIED | the tool-level contract (markers, toggle, note) is exercised only via ToolsDialog.test.tsx (other agent's file) — rg shows ToolsDialog.test.tsx exists; internals not read by me. The dead seams (F-001) are precisely the kind of gap a tool-level suite would have caught (no test asserts onChange/levels flow). |
Mutation check (reasoned): remove the data-pending-command marker → if ToolsDialog.test or a pin asserts it, it fails (UNVERIFIED which); nothing can fail on the levels/onChange seams because no test exercises them.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "no visualizer engine stage or catalog command exists" | l.12-13 | REAL | rg catalog: 0 hits |
| "never a dead control that looks engine-bound" | l.15-16 | REAL | markers + note + tests of children; the control is interactive and honest |
| "Mountable as-is (tools dialog …, the Settings → Audio effects area, a16's music mode)" | l.18-19 | FALSE (2 of 3 mount points) | only ToolsDialog mounts it (rg sweep) |
| "`levels` is the host-owned data seam … when the analysis source lands" | l.20-21 | REAL (absence) | no analysis source exists; no host passes levels |

## 7. Dead / duplicated / unreachable code
Dead props in practice: levels/onChange/defaultMode/defaultOpen/id/className (sole consumer passes none). No duplicated logic (composes Display/ModeSelect). Unreachable in shipped product (W14).

## 8. Dependencies & flags
Icon, VisualizerDisplay, VisualizerModeSelect, useVisualizerMsg, types, visualizer.css. No transport import (cannot dispatch — honest). No feature flags.

## 9. Open questions for the next agent
- VisualizerDisplay.tsx owner: confirm the resting depictions are static SVG/CSS art (not canvas animations burning frames at 0 data) — performance angle if the display mounts a rAF loop unconditionally.
- a16's "music mode" (variants?): if that surface was planned to mount the visualizer, its absence is a dropped wiring — check the variants wave findings.
