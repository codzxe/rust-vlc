# AUDIT: ui/src/screens/player/tools/dynamics/DynamicsCard.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-11-21 |
| Lines / functions | 144 / 3 (DynamicsRow memo adapter, DynamicsCard, setRowValue) |
| File verdict | SHALLOW |
| Reachable from shipped entry point? | WITHIN WEB BUILD: YES (ToolsDialog.tsx:37,127-129 mounts Compressor/Spatializer/StereoWidener tools, each rendering this card). ENGINE: NO — audio-engine effects are Gain/Mute/Balance/Eq3Band only (effects.rs:162,185,244; eq.rs:235); no compressor/spatializer/stereo-widener stage, no audio.set-* command for them (rg over generated/commands.ts + dispatch.rs: zero). SHIPPED DESKTOP BINARY: NO (baseline #2; W14-A01) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg AudioEffect impls in crates/audio-engine/src/effects*.rs (Gain/Mute/Balance/Eq3Band only); rg set-compressor/set-spatializer/stereo-widen in commands+dispatch (zero); rg DynamicsCard consumers; ToolsDialog mount check; harness.test.tsx survey |

## 1. One-paragraph verdict
The twin of VideoCard: a clean, memoized, frame-faithful collapsible card
whose every slider writes local state and nothing else. The honesty chain is
complete and accurate (staged note, data-staged-tool, data-pending-command,
engine inventory confirmed empty of all three effects). The j10 P3-5
memoization fix is real and correct. But the card is the shell for THREE
fully-interactive audio tools (compressor, spatializer, stereo widener)
that cannot affect audio in any build — and unlike the video family, the
engine has no partial core to grow from (no dynamics stage exists at all).

## 2. What actually works (verified)
— verified by baseline ui-vitest.log.txt (1705 passed): harness tests render
all three tools; frame-contract pins (thumb-flush travel, chevron native
scale, degree ticks, light-theme token remap) execute; CompressorTool/
SpatializerTool/StereoWidenerTool test files exercise the rows.
— verified by read: the memo adapter (43-61) takes the stable module-level
row def + card-level stable sink (useCallback 81-83) — the documented
re-render bailout is sound; `values[row.id] ?? row.initial` fallback is
safe for late-added row ids.
— verified by rg: the header's engine claim is TRUE (only Gain/Mute/Balance/
Eq3Band implement AudioEffect) — the staged markers do not over- or
under-claim.

## 3. Findings
### F-001 [S3] Shell for three interactive audio tools that can touch no audio
Where: ui/src/screens/player/tools/dynamics/DynamicsCard.tsx:16-21
Code:
```
// HONEST STAGING (the effects-area pattern, settings/areas/effects.tsx): no
// compressor/spatializer/stereo-widener engine stage or catalog command
// exists. Controls stay fully interactive with LOCAL state, visibly marked
// staged — a role="note" line + data-pending-command naming the
// forward-declared command they await — never a dead control that looks
// engine-bound.
```
What is wrong: Same pattern as my ColorsTool F-001: the user opens the
player Tools dialog, drags compressor thresholds and spatializer widths —
all local state, zero audio effect, disclaimed by a staged note. The claim
in the comment is accurate; the shipped UX is still a full-fidelity mock of
three VLC feature panels.
Why it matters: Three tool panels' worth of interaction surface with no
engine lane; the engine's effect chain (EffectChain) has no insert point
for them yet, so this is further from real than the EQ (whose Eq3Band core
exists).
What correct looks like: Engine stage first (compressor is a well-bounded
biquad-family effect), then the card's pendingCommand becomes a dispatch;
or disabled-with-reason staging like the extensions area's gate.
Confidence: HIGH — Verification: read + rg (engine inventory + command
catalog).
(NOTE: this finding is filed once here for the card; the three tool files
are other agents' assignments and inherit the same verdict.)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DynamicsRow (memo) | 43-61 | REAL | — | correct stable-props memoization (j10 P3-5); closure passes row.id correctly |
| DynamicsCard | 64-144 | SHALLOW (as feature) | S3 (F-001) | card mechanics real (aria-expanded/controls, hidden body kept in DOM so aria-controls resolves — header claim verified at 121-127) |
| setRowValue | 81-83 | REAL | — | useCallback, functional update, stable identity for the memo |

## 5. Test honesty (harness.test.tsx + the three tool tests)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| renders all three tools (25-33) | YES (crash-level) | smoke only |
| HTML snapshot export (34-57) | NO (in CI-less runs) | env-gated dev harness (W26C2_HARNESS_OUT) — honest dev-only label |
| frame-contract pins (59-96) | YES | CSS-rule assertions on dynamics.css (thumb flush, chevron scale, ticks, tokens) |
| CompressorTool/SpatializerTool/StereoWidenerTool tests | YES | per-tool row + staged-marker assertions (sibling files, other agents) |
Mutation check: remove the memo → no test fails (perf-only fix, untestable
at this level — acceptable). Remove the staged note → tool tests fail (they
assert role="note" copy, per the sibling files' describe names).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "no compressor/spatializer/stereo-widener engine stage or catalog command exists" | header 16-18 | REAL | effects.rs inventory + catalog rg (zero) |
| "expanded body stays in the DOM (hidden) so aria-controls always resolves" | header 12-14 | REAL | code 121-127 |
| afeat rows for compressor/spatializer (audio-feature family) | docs/feature-matrix.toml (audio features) | consistent with staged (not_started) | engine inventory confirms |

## 7. Dead / duplicated / unreachable code
None in-file. VideoCard (my file 15) reuses this card's CSS classes by
import — composition, not duplication (verified in VideoCard.tsx:39).

## 8. Dependencies & flags
icon, copy (useDynamicsMsg), ToolSliderRow, types, dynamics.css. No client
import (leaf panel — correct). No feature flags.

## 9. Open questions for the next agent
- The three tool files' pendingCommand strings (audio.set-compressor etc.)
  — do they all agree on naming like the video family's split
  (video.set-colors vs video.set-adjust)? Check for the same magic-string
  drift I flagged in ColorsTool F-002.
