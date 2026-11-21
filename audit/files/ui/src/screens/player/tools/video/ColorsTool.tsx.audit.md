# AUDIT: ui/src/screens/player/tools/video/ColorsTool.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-11-21 |
| Lines / functions | 343 / 5 (percent, ColorsHexRow, ColorsToggleRow, ColorsTool, flip closure) + 4 config tables |
| File verdict | SHALLOW |
| Reachable from shipped entry point? | WITHIN WEB BUILD: YES (ToolsDialog.tsx:39,134 mounts `<ColorsTool/>` in the player's tools dialog; referenced by settings/areas/effects.tsx:42). ENGINE: NO — no video-effects stage or catalog command exists (`video.set-colors` absent from generated/commands.ts, rg-verified). SHIPPED DESKTOP BINARY: NO (tauri host unbuildable — baseline #2; W14-A01) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg "set-colors" generated/commands.ts (ABSENT); sed i18n/keys/w26.c4.ts:55-62 (staged note copy); rg "ColorsTool" mounts; sed feature-matrix.toml:1840-1877 (vfeat-colors-adjust partial / vfeat-color-fx not_started all platforms); ColorsTool.test.tsx survey incl. staged-marker test 236-240; read VideoCard.tsx (staged markers) |

## 1. One-paragraph verdict
A frame-faithful, fully-local control panel: two hex+swatch rows, three
calibrated sliders, five toggles, one sub-dropdown — every control works as
UI state and NOTHING else. No engine stage, no catalog command, no dispatch:
the header admits it (lines 28-30), the card renders a visible "Staged —
controls activate with the video effects command card" note
(messages.ts:2429), and `data-staged-tool`/`data-pending-command` markers are
test-pinned. The feature matrix honestly lists vfeat-color-fx as
not_started on all platforms (feature-matrix.toml:1871). So this is not
fraud — it is an interactive mock shipped inside the real player tools
dialog, one small-print note away from looking like a working VLC colors
panel. Tests are honest about testing LOCAL state only.

## 2. What actually works (verified)
— verified by baseline ui-vitest.log.txt (1705 passed): ColorsTool.test.tsx
executes card shell, frame rows (hex inputs, staged ranges, separators),
local-state updates, and the staged-marker assertions (236-240).
— verified by read: state is genuinely local (useState only; no client
import; no context); swatch fill guarded by isHexColor (frameData) so an
invalid hex paints nothing.
— verified by rg: the honesty chain is consistent — header comment,
VideoCard note (role="note"), i18n copy, data attributes, matrix row all
agree that the feature is staged/not-started.

## 3. Findings
### F-001 [S3] A fully interactive video-colors panel with zero effect on playback — disclaimed by one small-print note
Where: ui/src/screens/player/tools/video/ColorsTool.tsx:28-30
Code:
```
// Honest staging: no color-threshold/negate/posterize/gradient/cartoon/sepia
// engine stage or catalog command exists — local state + the visible staged
// markers (VideoCard), controls fully interactive.
```
What is wrong: The user can flip Negate, drag Saturation to 100%, pick a
gradient mode — and the picture never changes. Every control is decorative
UI state. The mitigations are real (visible staged note, data markers,
honest matrix row) but a "Staged" footnote does not change that the player
ships a colors tool that cannot color anything.
Why it matters: The panel occupies the player's Tools dialog (Ctrl+E family)
with the visual grammar of a working feature; a user who never reads the
note concludes the feature is broken. VLC's equivalent modules (extract,
colorthres, invert, posterize, gradient, sepia) actually process frames.
What correct looks like: Either the engine stage + `video.set-*` command
family lands (matrix phase-4 target) and this file dispatches real args, or
the tool ships disabled-with-reason like the seekbar's honest-disable
grammar, instead of fully-interactive-no-effect.
Confidence: HIGH — Verification: read + rg (no command, no stage, no
dispatch call anywhere in the file).
### F-002 [S4] `data-pending-command="video.set-colors"` names a command that exists nowhere — not even as a forward declaration
Where: ui/src/screens/player/tools/video/ColorsTool.tsx:224
Code:
```
      pendingCommand="video.set-colors"
```
What is wrong: `video.set-colors` is absent from generated/commands.ts and
from the feature matrix (which names `video.set-adjust` for the Essential
panel, feature-matrix.toml:1852). Nothing validates the string; two
forward-declared names already disagree across surfaces.
Why it matters: When the command family lands, the string here is the
contract — an unvalidated magic string can silently misspell the eventual
catalog name with no failing check.
What correct looks like: A single shared constant (or the generated command
union) as the source for pendingCommand, with a lint that the value is
either a real command or a registered forward-declaration.
Confidence: HIGH — Verification: rg over generated/commands.ts and
feature-matrix.toml.
### F-003 [S4] Gradient mode option set is unverified against any engine lane
Where: ui/src/screens/player/tools/video/ColorsTool.tsx:97-104
Code:
```
/** The Gradient mode sub-dropdown's options — VLC's gradient filter modes
 * (the frame exports only the closed trigger; option set [VERIFY engine
 * lane]). Mutably typed: the Dropdown primitive's options prop. */
const GRADIENT_MODES: DropdownOption<VideoLabelKey>[] = [
```
What is wrong: The three modes (gradient/edge/hemboss) are transcribed from
VLC's gradient filter with an explicit [VERIFY engine lane] tag that no lane
has closed; also "hemboss" is VLC's actual spelling of "emboss" — kept
deliberately? Not stated.
Why it matters: Unverified vocabulary riding a staged panel — if the engine
lane lands with different mode names, this list drifts silently.
What correct looks like: Close the [VERIFY] tag or derive the list from the
future command schema.
Confidence: HIGH — Verification: read (tag present in-code).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| percent | 49 | REAL | — | aria-valuetext formatter, rounds |
| SATURATION/SIMILARITY/SEPIA rows | 64-93 | REAL | — | calibrated initials + 0.05 lattice step (j11 P2-2) — the paint math is documented and test-pinned (test 67-96) |
| GRADIENT_MODES | 100-104 | REAL | S4 (F-003) | unverified option set |
| ColorsToggleId / FRAME_TOGGLES / ALL_OFF | 107-134 | REAL | — | every frame toggle ships OFF — frame-true |
| ColorsHexRow | 137-178 | REAL | — | label htmlFor input; swatch guarded by isHexColor; toggle carries aria-label |
| ColorsToggleRow | 181-193 | REAL | — | label+toggle row |
| ColorsTool | 196-343 | SHALLOW | S3 (F-001) | all-local state; no dispatch anywhere |
| flip | 216-218 | REAL | — | local toggle state only |
| useDropdownScrollportFlip (imported hook) | 214 | REAL | — | clip-aware list placement; own test file exists |

## 5. Test honesty (ColorsTool.test.tsx)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| card shell collapse/expand/defaultOpen (18-49) | YES | asserts aria-expanded + hidden body |
| frame rows render (52-128) | YES | asserts hex values, valuetext, ticks, OFF switches, 5 separators |
| hex typing updates field + swatch (131+) | YES | local-state roundtrip incl. invalid-hex swatch guard |
| staged markers (236-240) | YES | asserts data-staged-tool + data-pending-command |
Mutation check: delete the staged note render (VideoCard) → test 236-240
fails. Delete a slider row → frame-row tests fail. Honest suite — it tests
exactly what the component is (a local-state panel), never pretending an
engine path exists.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| vfeat-color-fx "Color effects" — status not_started, evidence "" | feature-matrix.toml:1866-1876 | REAL (honest) | no stage/command exists (rg); panel is staged |
| vfeat-colors-adjust "UI sliders render inert-with-reason until the command lands" | feature-matrix.toml:1852 | SHALLOW vs THIS panel | the Adjust sliders live in EssentialTool (other file); this Colors panel renders interactive-with-note instead of inert-with-reason — two different honesty grammars for the same staged family |
| "controls fully interactive" + staged markers | file header 28-30 | REAL | verified + test-pinned |

## 7. Dead / duplicated / unreachable code
- All state is local by design (the whole component is the "dead" surface —
  covered by F-001, not double-counted).
- `id` prop accepted but ToolsDialog mounts without one (generatedId path
  used) — API seam for tests only. Not dead (harness.tsx uses it).

## 8. Dependencies & flags
components (Toggle, Dropdown), VideoCard/VideoSliderRow/copy/frameData/types,
dynamics.css import via VideoCard. No feature flags; no client import at all.

## 9. Open questions for the next agent
- EssentialTool (not my file) allegedly renders the Adjust sliders
  "inert-with-reason" per the matrix note — verify it actually disables
  rather than staging, and whether the two grammars should unify.
- If the video.set-* command family lands, does `video.set-colors` or
  `video.set-adjust` own the Colors panel's args? (F-002's name conflict.)
