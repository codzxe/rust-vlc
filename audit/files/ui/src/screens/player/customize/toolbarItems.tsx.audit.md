# AUDIT: ui/src/screens/player/customize/toolbarItems.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 332 / 5 (useToolbarMsg, CustomizeToolbarItems, ToolbarNavigationSeam, RoutedToolbarNavigation + dispatch closure) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — ControlBar.tsx:207 `<CustomizeToolbarItems />` (the wiring the header says the Lead landed) -> player screen |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | rg requestId contract (docs/05-command-layer.md:425-428, docs/11:521); rg "video.record"/"toggle-pip"/"set-tone-mapping" in generated/commands.ts union (ABSENT); rg ControlBar wiring; read CustomizePanels.test.tsx real-bar cases |

## 1. One-paragraph verdict
The real-bar half of toolbar customization — this is live wiring: engine-bound
tiles dispatch genuine 1.D4 commands (verified against the generated wire
union), shuffle/repeat read the playlist provider's engine-confirmed state,
bookmark sends `media.add-bookmark` with the µs crossing, navigation tiles
route. Master-question answer: YES for this surface, with two real defects:
hardcoded `requestId: 1/2` violates the documented monotonic-per-connection
request-id contract, and the record/PiP/tone-mapping tiles dispatch commands
that DO NOT EXIST in the wire union — their errors are swallowed, leaving
click-dead buttons with zero user feedback.

## 2. What actually works (verified)
- ControlBar.tsx:207 renders `<CustomizeToolbarItems />` — the claimed wiring is real (rg).
- Bookmark tile: `media.add-bookmark` with `secondsToUs(Math.max(0, Math.floor(state.position)))` (lines 172-176) — real command, present in the generated CommandRequest union, µs-correct.
- shuffle/repeat: prefer `playlist.setShuffle/setRepeat` from the playlist provider (224-250) — engine-confirmed, never optimistic; repeat cycles off->all->one.
- Real-bar behavior is test-pinned: CustomizePanels.test.tsx:729 "renders the persisted band in the real bar", :747 live re-reads across Save, :779 playlists fires lumen:toggle-sidebar, :831 real bar follows engine rate events.

## 3. Findings
F-001 [S3]
Where: toolbarItems.tsx:113 and 175
Code:
```
    const request = { cmd: tile.command, args, requestId: 1 } as unknown as CommandRequest;
```
and
```
                    void wire.send({
                      cmd: "media.add-bookmark",
                      args: { time: secondsToUs(Math.max(0, Math.floor(state.position))) },
                      requestId: 2,
                    });
```
What is wrong: constant request ids. doc 05 §4.1: "`request_id` — monotonic
per host connection ... the control server ... enforces monotonicity per
connection as an anti-replay measure" (docs/05-command-layer.md:425-428;
docs/11-dependency-security-compliance.md:521 "per-connection request_id
monotonicity (anti-replay)"). Every tile click re-sends id 1 — on the
control-server host these would be rejected as replay; on the desktop
lumen_dispatch host ids are echoed-not-interpreted so it silently works.
The sibling seam (player/context.tsx:356-359) uses an incrementing counter —
this file diverges from the repo's own pattern.
What correct looks like: a per-provider counter like context.tsx.
Confidence: HIGH — Verification: read + rg docs.
F-002 [S3]
Where: toolbarItems.tsx:107-126 (+ panels.ts:135/150/158)
Code:
```
      if (tile.command === null) return;
      // Forward-declared commands (record/PiP/HDR) are not yet in the 1.D4
      // wire union — the cast bridge is CustomizePanels' pattern (recorded
      // in ui/NEXT-STEPS.md); errors are honest no-change.
      const request = { cmd: tile.command, args, requestId: 1 } as unknown as CommandRequest;
```
What is wrong: `video.record`, `video.toggle-pip`, `video.set-tone-mapping`
(panels.ts:135,150,158) are ABSENT from the generated CommandRequest union
(rg over src/generated/commands.ts — no such cmd strings). Clicking those
tiles sends a command no host serves; the response error is dropped on the
floor (dispatch reads only the "completed" branch, line 116-118) — no error
chip, no disabled state, no tooltip. Dead buttons with zero user-visible
failure mode, undisclosed (unlike the favorite/quit family which render
disabled-with-reason at 290-299).
Confidence: HIGH — Verification: rg generated union + read.
F-003 [S4]
Where: toolbarItems.tsx:113
Code:
```
      const request = { cmd: tile.command, args, requestId: 1 } as unknown as CommandRequest;
```
What is wrong: the `as unknown as CommandRequest` double-cast disables
compile-time checking for ALL engine-bound tiles, not just the
forward-declared three — a typo'd command string in panels.ts would ship
silently (runtime typed error only). Documented in ui/NEXT-STEPS.md.
Confidence: HIGH — Verification: read.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| useToolbarMsg | 63-74 | REAL | - | seam-first label resolver, 3 lane-key fallbacks |
| TOGGLE_SIDEBAR_EVENT / REPEAT_ORDER | 77-80 | REAL | - | frozen contract strings |
| CustomizeToolbarItems | 88-305 | REAL | S3/F-001,F-002 | hooks before early return (rules-of-hooks noted at 98-99) |
| — dispatch | 107-126 | REAL | S3 | swallows errors by design ("honest no-change") — but see F-002 |
| — event subscription | 128-133 | REAL | - | engine-confirmed cycle truth |
| — tile render branches | 153-299 | REAL | - | bookmark/sidebar/navigation/shuffle/repeat/action/cycle/disabled-with-reason |
| ToolbarNavigationSeam | 313-323 | REAL | - | router-presence split |
| RoutedToolbarNavigation | 325-332 | REAL | - | |

## 5. Test honesty
CustomizePanels.test.tsx real-bar cases (729/747/779/831): persisted band
render, live re-read across Save, sidebar seam, engine rate events. Mutation
reasoning: breaking the band render fails :729; breaking the µs bookmark
crossing is NOT covered (no bookmark-click test found in the real-bar
describe — UNVERIFIED whether it exists elsewhere). No test covers
record/PiP/tone-mapping dead clicks.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "engine-bound tiles dispatch their command" | toolbarItems.tsx:16-17 | REAL (partial) | true for union commands; FALSE for video.record/toggle-pip/set-tone-mapping (F-002) |
| "bookmark drops a media.add-bookmark at the engine-reported position" | toolbarItems.tsx:20-22 | REAL | code 172-176, command in union |
| "UI-owned tiles with NO surface ... render disabled-with-reason ... never theater" | toolbarItems.tsx:24-27 | REAL | lines 290-299 render disabled + "— unavailable" label suffix |
| "request_id ... monotonic per host connection" (contract this file violates) | docs/05-command-layer.md:425 | FALSE-in-practice for this file | hardcoded 1/2 (F-001) |

## 7. Dead / duplicated / unreachable code
None structurally; functionally dead: record/PiP/tone-mapping tile buttons
(F-002) — interactive-looking, no effect, no disclosure.

## 8. Dependencies & flags
react-router, client transport, i18n lane keys, cycleState/toolbarLayout/
panels siblings. No feature gates.

## 9. Open questions for the next agent
- Does the customize modal (CustomizePanels box) mark record/PiP/tone-mapping
  as staged/unavailable at selection time? If the palette offers them as
  normal tiles, F-002 worsens (user builds a dead button into their bar).
