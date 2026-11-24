# AUDIT: ui/src/screens/player/tools/video/VideoCard.tsx
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 106 / 1 (VideoCard) |
| File verdict | REAL (shell for staged tools — one staged marker is FALSE) |
| Reachable from shipped entry point? | YES (EssentialTool.tsx:130-146, CropTool.tsx:173, ColorsTool.tsx:224 mount VideoCard inside the tools dialog Video tab) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 0 |
| Commands run | rg pendingCommand values; rg video.set-crop in generated protocol + engine evidence (docs/perf/raw/stats-probe.jsonl accepted-variant list); sed quote verification |

## 1. One-paragraph verdict
Clean, accessible tool-card shell (aria-expanded/aria-controls wired, body kept
in DOM via `hidden`, id composition via useId). It renders the "staged" note
and data-pending-command markers for the video tools. The hostile finding: the
CropTool stages itself as awaiting `video.set-crop` — but that command EXISTS
in the engine protocol and in the UI's own generated CommandRequest union
(commands.ts: `{ args: { crop: CropSpec }; cmd: "video.set-crop" }`), and the
engine's live unknown-variant error (docs/perf/raw/stats-probe.jsonl) lists it
as accepted. The crop UI keeps local state and a "not wired" confession while
its backend command is live and unused. ColorsTool (video.set-colors) and
EssentialTool (video.set-essential) are genuinely missing commands.

## 2. What actually works (verified)
— a11y contract: button 72-91 carries aria-expanded/aria-controls; body 92-103
  stays in DOM (`hidden={!open}`) so aria-controls always resolves — matches
  the header claim (23-25).
— Class composition reuses dynamics.css (imported line 39) — no duplication.

## 3. Findings
F-001 [S2]
Where: ui/src/screens/player/tools/video/VideoCard.tsx:68-69 + 100-102 (marker renderers); false claim source CropTool.tsx:20-21, 173
Code:
```
      data-staged-tool={tool}
      data-pending-command={pendingCommand}
```
and
```
        <p className="dynamics-tool__staged" role="note">
          {m("player.tools.video.staged")}
        </p>
```
What is wrong: The staged note tells users the tool awaits a command that (for
crop) already exists: `video.set-crop` is in the UI's generated CommandRequest
union AND in the engine's accepted-command list (stats-probe.jsonl stderr
variant enumeration, between video.set-aspect-ratio and video.set-zoom).
CropTool.tsx:20-21 claims "no crop engine stage or catalog command exists" —
that claim is FALSE. The crop card is fully interactive local state that could
be dispatching a live command today.
Why it matters: this is the inverse of a dead button — a LIVE backend seam
with a dead UI on top, mislabeled as awaiting the seam. The visible role=note
misinforms users that the feature is impossible when it is merely unwired.
What correct looks like: CropTool dispatches video.set-crop over the client
transport on commit; staged markers stay only for set-colors/set-essential.
Confidence: HIGH — Verification: read + rg generated protocol + engine
stats-probe evidence. (Engine-side implementation of set-crop semantics is the
engine auditors' row; the protocol's existence is certain.)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| VideoCard | 43-106 | REAL | F-001 | renders staged markers whose crop claim is false |

## 5. Test honesty
No tests in this file; video tool tests owned by other agents.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no video-effects engine stage or catalog command exists" | VideoCard.tsx:27-28 | FALSE for crop | video.set-crop in generated commands.ts + engine accepted list (stats-probe.jsonl) |
| "the expanded body stays in the DOM (`hidden`) so aria-controls always resolves" | 23-25 | REAL | line 97 `hidden={!open}` |

## 7. Dead / duplicated / unreachable code
None in this file. The dead surface is the unwired crop dispatch (CropTool
owner) rendered through this shell.

## 8. Dependencies & flags
dynamics.css, video.css, Icon, useVideoMsg (copy.ts), types.ts. No flags.

## 9. Open questions for the next agent
1. CropTool.tsx owner: wire or justify — the engine takes `crop: None | Ratio
   | Rect` (docs/05-command-layer.md row), and the tool already collects
   exactly a Rect (four pixel fields).
2. Engine auditors: does video.set-crop actually mutate the renderer, or is
   the engine handler itself a stub? (That would flip this finding's target.)
