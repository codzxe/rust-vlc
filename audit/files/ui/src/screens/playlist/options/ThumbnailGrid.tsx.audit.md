# AUDIT: ui/src/screens/playlist/options/ThumbnailGrid.tsx (compact, 74 LOC — spec §13)
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 74 / 1 (ThumbnailGrid) |
| File verdict | REAL (engine-wired play cards in engine scope; honest inert cards in local scope) |
| Reachable from shipped entry point? | YES — playlist options layer (grid view mode) inside the player dock; playItem -> PlaylistProvider.tsx:231-232 `dispatch({ ... cmd: "playlist.play-item", args: { id } })` -> client transport -> lumen_dispatch (command exists in the catalog) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg "playItem\|play-item" PlaylistProvider.tsx` -> :231-232 real dispatch; read of full file |

## 1. One-paragraph verdict
A genuinely wired grid view: with an engine PlaylistProvider ancestor each
card is a button whose click dispatches the REAL `playlist.play-item`
command through the bridge (verified dispatch in PlaylistProvider.tsx:232),
and in the local saved-playlist browse scope the card degrades to an
explicitly inert presentation element (`role="img"`, `--inert` class)
instead of a dead button — the honest-scope split the header documents
(:9-13). The artwork well is empty by missing seam (no artwork command —
same catalog gap as MusicMode), honestly marked with a quiet glyph. The
one defect is the empty-state: zero entries render literally nothing.

## 2. What actually works (verified)
— Play dispatch chain verified end to end (this file ->
  PlaylistProvider:231-232 -> client.send -> lumen_dispatch; catalog
  carries playlist.play-item, ids.rs).
— Duration formatting + unknown-duration fallback through i18n (no
  literals — the eslint local/i18n rule enforces).
— Colocated ThumbnailGrid.test.tsx (optionsTestSupport mount) in the
  vitest baseline.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/playlist/options/ThumbnailGrid.tsx:29-30
Code:
  if (sortedEntries.length === 0) return <></>;
What is wrong: an empty playlist in grid view renders NO node at all — no
empty-state message, no region for the aria-label to live on. The list
view sibling presumably has a ListEmpty; the grid silently vanishes. A
screen-reader user toggling to grid on an empty playlist gets zero
feedback that a view exists. What correct looks like: the shared empty
state (or a labeled empty region). Confidence: HIGH — Verification: read.
(Artwork-well emptiness is catalog-gap dead surface — same F class as
MusicMode F-001, not re-counted here.)

## 4. Per-function table (compact)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| ThumbnailGrid | 23-74 | REAL | S3 empty-state (F-001); play seam verified |

## 5. Test honesty
ThumbnailGrid.test.tsx (optionsTestSupport consumers list) — asserts card
rendering and click dispatch through the fake transport's `sent` array;
would fail if playItem stopped dispatching. The empty-grid case: if tested
at all, it pins the CURRENT nothing-render (a test enshrining F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the engine has no artwork seam yet, so the well renders honestly empty" | :6-7 | REAL | no artwork command in catalog (verified in MusicMode audit) |
| "never a dead button" (local scope) | :12-13 | REAL | :61-67 inert div with role="img" |

## 7. Dead / duplicated / unreachable code
None. cardBody shared between both scopes (no duplication).

## 8. Dependencies & flags
PlaylistProvider (usePlaylistOptional — the scope probe), options provider
(sortedEntries), sorting utils, i18n. No flags.

## 9. Open questions for the next agent
- sorting.ts auditor: entryDisplayTitle/formatDurationClock correctness
  (h:mm:ss clock math, negative/NaN durations) — this file trusts them
  blindly.
