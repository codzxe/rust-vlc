# AUDIT: ui/src/screens/player/controls/state-mapping.ts
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 179 / 3 (mapTransportControls, seekDisableReason, controlCommandOf) |
| File verdict | REAL (pure, exhaustive, consumed by the live control bar) |
| Reachable from shipped entry point? | YES — ControlBar.tsx imports it (rg: ControlBar.tsx, MiniPlayer.tsx, PlayerScreen.tsx, controls/index.ts); ControlBar renders inside PlayerScreen, the app's default route surface |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `rg "PlaybackState" ui/src/generated/commands.ts` -> :56 exactly 8 members (idle/opening/buffering/playing/paused/stopped/ended/error) = 8/8 switch cases covered; `rg toggle-play-pause keymaps/default.toml` -> :83; test count rg -> 8 `it(` in state-mapping.test.ts |

## 1. One-paragraph verdict
A genuinely correct pure mapper: the play-button switch covers every
PlaybackState member (verified against the generated union, 8/8 — no
default branch, so a future 9th member would fall to the inert initial
value rather than crash), stop/seek rules are conservative (never
optimistic, seek disabled only on REPORTED non-seekable), and
seekDisableReason gives typed disabled-with-reason strings. This is the
honest layer of the player UI: the states it maps FROM
(playback.state-changed events) are engine truth, and the mapping never
invents a glyph. Two stale-comment defects only.

## 2. What actually works (verified)
— Exhaustiveness verified against generated/commands.ts:56. 8 tests in
state-mapping.test.ts pin the §5.1 table rows (vitest baseline 1705
passed). Mutation reasoning: flip a glyph or a disabled flag -> the
parametrized row test fails.
— Consumers confirmed by rg (ControlBar/MiniPlayer/PlayerScreen).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/controls/state-mapping.ts:155-158
Code:
 * The seed keymap has no play/pause binding yet — this stays
 * data-driven so the loaded keymap lights it up without code changes
 * [PROPOSAL — recorded in ui/NEXT-STEPS.md].
What is wrong: the comment is stale — keymaps/default.toml:83 already
binds `toggle-play-pause = { key = "Space", ... command =
"playback.toggle-play-pause", ...}` (and :84 `S` -> stop, :61 MediaPlayPause).
The seam is LIVE, not a proposal waiting for data. Risk: a reader concludes
Space does nothing in the shipped keymap. Confidence: HIGH — Verification:
read + rg.
F-002 [S4]
Where: ui/src/screens/player/controls/state-mapping.ts:139-151
Code:
  if (input.seekable === false) return "player.controls.reason.not-seekable";
  if (
    input.playbackState === "idle" ||
    input.durationSeconds === null ||
    input.durationSeconds <= 0
  ) {
    return "player.controls.reason.no-media";
  }
  return "player.controls.reason.unknown";
What is wrong: `stopped` and `opening` (with a known duration) both fall
through to the generic "unknown" reason although the seek disable cause is
precisely the state — the a11y hint degrades to vagueness in two reachable
states. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mapTransportControls | 67-131 | REAL | — | 8/8 states covered (verified); no default -> inert fallback for future members |
| seekDisableReason | 139-151 | REAL | S4 | F-002 |
| controlCommandOf | 166-178 | REAL | S4 | seam live (F-001 comment stale) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| 8 cases in state-mapping.test.ts | YES | parametrized table asserts glyph/disabled/pending per state |
Not executed by me individually; covered by the vitest baseline run.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The seed keymap has no play/pause binding yet" | :156-157 | FALSE (stale) | keymaps/default.toml:83,84,61 |
| "[PROPOSAL] doc omits the column — stopped: play glyph + restart" | :16-17 | REAL (as implemented proposal) | :96-99 matches the comment |

## 7. Dead / duplicated / unreachable code
None. All three exports consumed (ControlBar/MiniPlayer/PlayerScreen).

## 8. Dependencies & flags
client/transport (PlaybackState re-export from generated/commands). No
flags. The mapping is pure — no transport calls here; the wiring question
belongs to ControlBar (dispatch) and context.tsx (event source).

## 9. Open questions for the next agent
- ControlBar auditor: confirm the dispatched `playback.*` commands actually
  reach lumen_dispatch in the desktop shell and that pendingToggle is
  cleared on BOTH response and error (the mapping trusts its caller).
