# AUDIT: ui/src/screens/player/tools/dynamics/CompressorTool.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 93 / 2 (COMPRESSOR_ROWS def with 7 row entries + 7 format closures, CompressorTool) |
| File verdict | SHALLOW (real, tested, honestly-staged UI shell; the thing it is a tool FOR — a compressor — does not exist in the engine, and its catalog command exists nowhere in Rust) |
| Reachable from shipped entry point? | YES in the UI tree: ControlBar.tsx:702/713 `openToolsDialog("audio")` → ToolsDialog.tsx:37,127 `<CompressorTool />` — the player chrome mounts it. W14 ceiling applies (no GUI binary compiles at HEAD). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | `rg "set-compressor"` in ui/src → 4 hits (this file, types.ts:79, 2 tests); same pattern in crates/ and apps/ → **ZERO hits** — no handler, no dispatcher entry, no engine command; `rg -i compressor crates/audio-engine` → only effects.rs:3 comment ("EQ/compressor cards plug into" a rack that does not exist); read of DynamicsCard.tsx (local state only, `data-staged-tool`/`data-pending-command` markers, role="note" staging note — no transport call anywhere in the card); read of CompressorTool.test.tsx (pins 7 frame rows/defaults, slider interaction, staged markers); `rg "player.tools.compressor.rms-peak" ui/src/i18n/messages.ts` → :2392 "RMS / Peak"; `rg "FM5" "design/figma/Audio - Compressor Settings.svg"` → frame text-path literally reads "FM5 / Peak" (typo-correction claim TRUE); read of types.ts:23-53 (staged display-range disclosure). |

## 1. One-paragraph verdict
A well-built facade over nothing: seven frame-accurate slider rows (labels, defaults, formats all pinned by genuine tests, the "FM5 / Peak" frame typo really is corrected to "RMS / Peak" in the catalog) rendered by a shared card that keeps every ounce of state local and stamps itself `data-staged-tool` with a `data-pending-command="audio.set-compressor"` marker. That marker is the whole story: the command it names does not exist anywhere in the Rust workspace, the audio engine has no compressor stage (one wistful comment in effects.rs aside), and nothing this tool renders can affect a single audio sample. The staging is disclosed at three layers (types.ts:26-30, DynamicsCard header, the note the user sees), which keeps it out of fraud territory — but the user-facing fact stands: the player's Tools → Audio → Compressor offers seven live sliders that are wired to nothing.

## 2. What actually works (verified)
— verified by `rg` in ui/src/i18n/messages.ts:2387-2392: all 7 label keys resolve ("RMS / Peak", "Attack", …) — and the l10n gate (baseline #24) proves catalog/usage consistency for the family.
— verified by reading the frame SVG: "Audio - Compressor Settings.svg" contains the literal text-path "FM5 / Peak" — the header's typo claim (:9-10) is TRUE and the correction is real in messages.ts.
— verified by reading CompressorTool.test.tsx:9-115: FRAME_ROWS pins the frame's label+value column ("0.2", "25.0 ms", "100.0 ms", "-11.0 dB", "4.0 : 1", "5.0 dB", "7.0 dB"), slider interaction updates value text + aria-valuetext, End clamps to range max, staged markers asserted — these tests FAIL if rows/defaults/markers drift. Honest tests of what exists.
— verified reachability chain: ControlBar.tsx:702 → openToolsDialog("audio") → ToolsDialog.tsx:127 mounts CompressorTool.

## 3. Findings
### F-001 [S2]
Where: ui/src/screens/player/tools/dynamics/CompressorTool.tsx:83-92 (+ types.ts:77-82, crates/audio-engine/src/effects.rs:3)
Code:
```
export function CompressorTool(props: DynamicsToolProps): ReactElement {
  return (
    <DynamicsCard
      tool="compressor"
      nameKey="player.tools.dynamics.compressor"
      pendingCommand="audio.set-compressor"
```
What is wrong: The tool's only reason to exist — applying compression — is unimplemented. `audio.set-compressor` has ZERO occurrences in crates/ and apps/ (verified by rg; only this file, types.ts:79's comment, and two tests mention it). DynamicsCard holds values in `useState` and never touches a transport. The engine has no compressor DSP: the sole mention in crates/audio-engine is a comment saying EQ/compressor cards would "plug into" an effects rack (:3) — the rack does not exist. Same disease as audio.set-eq (W8 pin), one wave later and one layer deeper: here even the command name is still fictional.
Why it matters: A user opening Tools → Audio → Compressor in the (hypothetically shipped) player gets seven responsive sliders, keyboard a11y, value formatting — a complete instrument experience — that changes no audio. The staging note and data markers are honest, but they are developer-facing annotations, not a disabled control: the sliders are fully live.
What correct looks like: either land the compressor stage + catalog command (then the markers become a real send), or ship the card in a clearly non-interactive "coming soon" state instead of live sliders with hidden no-op semantics.
Confidence: HIGH — Verification: rg across crates+apps (zero hits); DynamicsCard read end-to-end (no transport import); ToolsDialog reachability read.

### F-002 [S4]
Where: ui/src/screens/player/tools/dynamics/CompressorTool.tsx:6-8
Code:
```
// Defaults are the frame's value column (VLC compressor semantics: RMS/peak
// 0.2, attack 25 ms, release 100 ms, threshold −11 dB, ratio 4:1, knee
// 5 dB, makeup 7 dB). Ranges are staged display ranges calibrated to the
```
What is wrong: The "VLC compressor semantics" attribution is UNVERIFIABLE from this repo: no VLC source, spec table, or citation exists in-tree, and network access is prohibited. The frame-values half of the claim is verified (test pins + frame SVG); the VLC half is an appeal to an authority not in evidence. Additionally the ranges (e.g., attack max 280 ms, rms-peak max 1.25) are explicitly stage-prop numbers "calibrated to the frame's measured thumb positions" (types.ts:26-30), so nothing here encodes real DSP bounds either.
Why it matters: When someone finally implements the command, these comments are the natural place to copy semantics from — an unverified attribution presented alongside verified frame data invites cargo-culting the numbers as engine truth.
What correct looks like: cite the VLC module/version whose defaults these match, or drop the parenthetical.
Confidence: HIGH that it is unverified; UNVERIFIED whether the attribution is true.
Severity: S4 (comment hygiene; no code depends on the claim — the values themselves are frame-pinned by tests).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| COMPRESSOR_ROWS (7 defs) | 16-80 | REAL (as display data) | S2 | frame-accurate labels/defaults/formats (test-pinned); ranges are staged fictions per types.ts:26-30; no consumer beyond the card renderer |
| format closures ×7 | 24,33,42,51,60,69,78 | REAL | — | toFixed(1) + unit strings match the test's FRAME_ROWS exactly |
| CompressorTool | 83-93 | SHALLOW | S2 | F-001: wrapper over DynamicsCard; pendingCommand names a nonexistent command; reachable from ControlBar:702 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "ships collapsed… with the expand toggle" | YES | asserts aria-expanded=false, hidden body, no leaked rows |
| "expands on toggle click and collapses back" | YES | keyDown ArrowRight → "25.5 ms" text + aria-valuetext (real state change on the correct row) |
| "clamps at the range maximum" | YES | End → "10.0 : 1" pins the ratio range |
| "marks the card staged with the pending command marker" | YES | asserts data-staged-tool=compressor, data-pending-command=audio.set-compressor, role=note /staged/i — the tests pin the honesty apparatus itself |
| gap | — | no test asserts any engine effect, because there is none to assert; the suite never pretends otherwise (title says "honest staged markers") |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "No such command exists yet — the marker keeps the controls honestly staged" | types.ts:79-80 | REAL | rg: zero Rust hits for set-compressor |
| "the design's 'FM5 / Peak' typo, corrected to 'RMS / Peak'" | :9-10 | REAL | frame SVG text-path "FM5 / Peak"; messages.ts:2392 "RMS / Peak" |
| "Defaults are the frame's value column" | :6 | REAL | CompressorTool.test.tsx FRAME_ROWS + frame SVG |
| "(VLC compressor semantics: …)" | :6-8 | UNVERIFIED | no in-repo source; no network — see F-002 |
| "Ranges are staged display ranges calibrated to the frame's measured thumb positions" | :8-9 + types.ts:26-30 | REAL (disclosed fiction) | ranges are display-only; no DSP bounds encoded |
| ToolsDialog header "all state is local to the tools" | ToolsDialog.tsx:14 | REAL | DynamicsCard useState only |

## 7. Dead / duplicated / unreachable code
None in-file. The latent deadness is outward: the command string it forwards is awaited by nothing (F-001).

## 8. Dependencies & flags
DynamicsCard + types only; no transport/engine imports in the entire dynamics tool chain (DynamicsCard read). No feature flags.

## 9. Open questions for the next agent
- SpatializerTool and StereoWidenerTool share this card and the same staged pattern — expect the same S2 (their pending commands presumably also have zero Rust hits; I checked only the compressor string).
- When the DSP command layer lands, who owns converting display ranges (thumb-calibrated) into engine ranges? types.ts:30 says "Engine ranges replace these" — the replacement point is nowhere marked in code.
