# AUDIT: ui/src/screens/player/tools/video/VideoCard.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-11-21 |
| Lines / functions | 106 / 1 (VideoCard) |
| File verdict | SHALLOW |
| Reachable from shipped entry point? | WITHIN WEB BUILD: YES (ToolsDialog.tsx:39 mounts Colors/Crop/Essential/Hdr tools, each rendering this card). ENGINE: NO host lane — no video-effects stage; `video.set-crop` exists in the CATALOG (crates/commands/src/catalog/command.rs:225-226) but is served by NO host (apps/cli rg: zero VideoSetCrop; demo-transport: falls to notExposed). SHIPPED DESKTOP BINARY: NO (baseline #2; W14-A01) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | rg pendingCommand values across video tools (set-essential/set-crop/set-colors + HDR); rg "video.set-crop" in catalog (command.rs:225-226, ids.rs:127, types.rs:420) vs apps/cli (zero) vs demo-transport switch (absent → notExposed); read VideoCard body |

## 1. One-paragraph verdict
The shared collapsible shell for all four video tools — mechanically real
(aria wiring, hidden-body-in-DOM so aria-controls resolves, composition over
the dynamics stylesheet instead of duplication) — wrapping four staged
panels that touch no video. Two findings: the shell's blanket honesty claim
("no … catalog command exists") is FALSE for crop — `video.set-crop` is a
defined catalog command that no host serves, meaning the CropTool stages
behind a pendingCommand that already exists; and the shell itself is the
delivery vehicle for the interactive-mock pattern (my ColorsTool F-001,
same family).

## 2. What actually works (verified)
— verified by baseline ui-vitest.log.txt (1705 passed): the four tool test
files (ColorsTool/CropTool/EssentialTool/HdrTool .test.tsx) exercise the
card shell — collapse/expand, staged markers, aria wiring.
— verified by read: composition claim holds — dynamics.css imported once
here (line 39) for every mounting host; the card reuses dynamics-tool__
classes; chevron states and hidden body (92-98) match the documented grammar.
— verified by rg: staged markers (data-staged-tool 68, data-pending-command
69, role="note" 100-102) rendered for every tool mount.

## 3. Findings
### F-001 [S3] The shared shell delivers four fully-interactive video panels that affect no video
Where: ui/src/screens/player/tools/video/VideoCard.tsx:27-30
Code:
```
// HONEST STAGING (the effects-area pattern, settings/areas/effects.tsx): no
// video-effects engine stage or catalog command exists. Controls stay fully
// interactive with LOCAL state, visibly marked staged — a role="note" line +
// data-pending-command naming the forward-declared command they await.
```
What is wrong: The shell hosts Colors/Crop/Essential/HDR — every control in
all four is local state with a staged note (verified for Colors in my
ColorsTool audit; the others follow the same pattern per their test files).
No dispatch, no engine stage, no video effect in any build.
Why it matters: Same as ColorsTool F-001 — a complete VLC Ctrl+E tools
dialog experience with zero effect, disclaimed by small print.
What correct looks like: Engine stages + served commands; until then the
inert-with-reason grammar.
Confidence: HIGH — Verification: read + rg (no client import in the video
tools family; catalog/host survey).
### F-002 [S3] Header claim "no … catalog command exists" is FALSE — `video.set-crop` is a catalog command no host serves
Where: ui/src/screens/player/tools/video/VideoCard.tsx:28-29 (vs catalog command.rs:225-226)
Code:
```
// HONEST STAGING (the effects-area pattern, settings/areas/effects.tsx): no
// video-effects engine stage or catalog command exists. Controls stay fully
// interactive with LOCAL state, visibly marked staged — a role="note" line +
```
(catalog side, crates/commands/src/catalog/command.rs:225-226:)
```
    /// `video.set-crop` — renderer parameter (class S).
    #[serde(rename = "video.set-crop")]
```
What is wrong: The blanket claim is wrong for crop: `video.set-crop` IS in
the command catalog (also ids.rs:127, types.rs:420 CropSpec). No host serves
it (apps/cli: zero handlers; demo-transport: falls through to
feature-not-exposed) — so the STAGING is materially honest, but the stated
REASON is false, and CropTool's `pendingCommand="video.set-crop"`
(CropTool.tsx:173) names an existing command, not a forward-declared one.
The catalog/UI drift means: the moment a host starts serving set-crop, the
CropTool remains staged with no failing check anywhere.
Why it matters: The honesty ledger is the product's core discipline in this
repo; a false premise in the shared shell's contract misleads every future
maintainer of all four tools. It also hides a real wiring opportunity
(crop could dispatch today and honestly surface the not-exposed/unsupported
answer).
What correct looks like: The header should say "no SERVED command exists
(catalog has video.set-crop — unserved)"; better, CropTool dispatches the
real command and surfaces the typed refusal like the rest of the app does.
Confidence: HIGH — Verification: rg catalog + host + demo transport.
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| VideoCard | 43-106 | SHALLOW (as feature shell) | S3 (F-001/F-002) | mechanics real: generatedId fallback, aria-expanded/controls, hidden body in DOM, children slot |

## 5. Test honesty (via the four tool test files)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| card shell collapse/expand + staged markers (each tool's tests, e.g. ColorsTool.test 18-49, 236-240) | YES | shared shell exercised through every tool's suite |
Mutation check: delete the staged note render (100-102) → all four tools'
staged-marker tests fail. Solid pinning of the shell through its children.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "no video-effects engine stage or catalog command exists" | header 27-29 | FALSE (half) | F-002: video.set-crop is cataloged (command.rs:225); no host serves it |
| "the expanded body stays in the DOM (hidden) so aria-controls always resolves" (dynamics grammar, inherited) | dynamics header 12-14 | REAL | code 92-98 |
| vfeat-colors-adjust matrix note "UI sliders render inert-with-reason until the command lands" | feature-matrix.toml:1852 | SHALLOW | the Essential panel stages interactive-with-note instead (two honesty grammars in one dialog) |

## 7. Dead / duplicated / unreachable code
None in-file. The four hosted tools' local state is the dead surface
(counted in F-001).

## 8. Dependencies & flags
icon, copy (useVideoMsg), types, dynamics.css + video.css imports —
composition documented and verified. No client import.

## 9. Open questions for the next agent
- CropTool (other agent): given F-002, should crop dispatch the cataloged
  command now (surfacing the honest typed refusal) instead of staging?
- Who owns serving video.set-crop — is the catalog entry ahead of a planned
  renderer-parameter lane (command.rs:225 says "renderer parameter (class S)")?
