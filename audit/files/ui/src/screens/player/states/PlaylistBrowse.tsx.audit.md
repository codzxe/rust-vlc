# AUDIT: ui/src/screens/player/states/PlaylistBrowse.tsx
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 239 / 2 components + 1 helper |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle: PlayerScreen.tsx:126 imports, :1067 mounts `<PlaylistBrowse>` when a docked playlists-browser row opens (browseId state, :738). /player route → PlayerScreen. No shipped binary (W14 E0425). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | sed comparison of the two folderOf bodies; rg states.css hook census (browse + preview classes); StateRouter.test.tsx browse-suite read; PlayerScreen media-surface comment read (:25-40); rg folderOf duplicates. |

## 1. One-paragraph verdict
The player's playlist-browse face: a natural-sorted names-only grid plus the Player-3 floating now-playing card. The honesty discipline is the best I have audited in this wave: the grid cards are deliberately INERT presentation (divs, not fake play buttons — "names-only local data has no play seam" disclosed inline), the frame's watched-progress bars are explicitly NOT rendered ("a fabricated bar would be theater"), the PiP toggle is disabled-with-reason while no session is up, and the now-playing card's video area is a labeled placeholder (`data-placeholder="engine-wiring-4.H1"`) with the engine's real position on the Seekbar and engine-truth-first title — the Wave-0 "GUI displays no video" pin shows up here as an honestly-labeled 4.H1 handoff, consistent with PlayerScreen.tsx:25-40 (the UI's only real media element is a `<video>` for browser-addressable URL sources; engine-decoded frames render nowhere). One S4: the folderOf helper is a byte-identical private duplicate of MiniPlayer.tsx:105's — disclosed ("restated locally (it is not exported from variants/)") but exactly the duplication the same wave's k06 cleanup eliminated for laneMsg.

## 2. What actually works (verified)
- Mount chain: PlayerScreen.tsx:1067 (browseId from the docked playlists browser :732-744) — reachable; exit/PiP-toggle props wired to PlayerScreen state.
- Grid honesty: cards are `<div>`s with title tooltips (:137) — no dead buttons; empty state copy (:126); naturalSort reuse from the shared list module (:88).
- Key regression fix verified in test: `key={`${index}:${name}`}` (:133) — StateRouter.test.tsx:423-454 renders duplicate filenames and asserts 3 distinct cells with ZERO React same-key console warnings (the honest probe). Ran in baseline vitest (row 23).
- NowPlayingCard wiring: `usePlayerControls()` → Seekbar value=state.position, duration, buffered, pendingSeek, disabled=mapped.seek.disabled with aria-label carrying the disable reason (:203-214), `onCommit={actions.seek}` — the same real seam the main seekbar uses; title = engine mediaTitle ?? playlist entry title ?? honest empty (:173, :218); folder chip from real file data (:174, :222-227); hasVideo picks the glyph from real track data (:175).
- The `<video>`-under-the-grid claim (:29-31) matches PlayerScreen.tsx:25-40 ("real MEDIA SURFACE is a <video> element mounted while a session is up … renders real bytes where the source addresses the browser's media stack") — the preview card's placeholder is honest about the decode half.
- CSS hooks: all lumen-state-browse__* and lumen-state-preview__* classes defined in states/states.css (:488-591, 16 preview matches).
- i18n: every string via useLaneMsg(i02Keys)/useMsg — no literals.

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/player/states/PlaylistBrowse.tsx:53-60 (duplicate of variants/MiniPlayer.tsx:103-109)
Code:
```
/** Containing folder of a file source ("/a/b/clip.mp4" → "b"); null when the
 * source is not a file or has no parent segment (honest hide). MiniPlayer's
 * helper, restated locally (it is not exported from variants/). */
function folderOf(entry: PlaylistEntry): string | null {
```
What is wrong: Byte-identical private implementation of MiniPlayer.tsx:105's folderOf (verified by sed comparison — same body, same doc comment). The comment discloses the duplication but the wave's own precedent fixed this pattern: w26.k06 (J06-9) moved the lane resolver to the shared ../laneMsg helper and deleted this file's local copy — folderOf got the same treatment nowhere.
Why it matters: Two copies of a path-parsing helper can drift silently (a trailing-slash or UNC-path fix in one leaves the other wrong); the file's own header documents the k06 dedup discipline it here violates.
What correct looks like: Export folderOf from a shared player module (../controls or ../laneMsg's sibling) and import in both — the exact k06/J06-9 move.
Confidence: HIGH — Verification: read both bodies; sed line-by-line identical.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| BrowsePlaylistData (type) | 44-48 | REAL | — | Store-data shape (names only — honesty contract at :13-18) |
| folderOf | 56-60 | REAL (duplicated) | S4 | F-001 |
| PlaylistBrowseProps (type) | 62-73 | REAL | — | PlayerScreen-injected truth (hasMedia host-injected) |
| PlaylistBrowse | 76-155 | REAL | — | Natural-sorted rows (memoized); exit + PiP toggle (disabled-with-reason, aria-pressed :117-118); empty state; inert grid cards; NowPlayingCard mounted only while hasMedia && previewOpen (:150-152) |
| NowPlayingCard | 161-239 | REAL | — | Engine/playlist truth only; Seekbar = real seam; placeholder labeled `data-placeholder="engine-wiring-4.H1"` + sr-note; data-empty honesty on title and seek (:200-202, :217) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "duplicate filenames render as DISTINCT cells" (StateRouter.test.tsx:423-454) | YES | Renders ["clip.mp4","clip.mp4","other.mp4"], asserts 3 listitems AND zero console "same key" warnings — real regression probe |
| Coverage gap (not a finding, a fact): the PiP disabled-with-reason state, NowPlayingCard seek wiring, folderOf, and exit are untested at this file's level — the same seams are covered by MiniPlayer/PlayerScreen suites (cross-ref A10's MiniPlayer audit) | — | — |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the cards render as honest presentation … the frame's 4px #D37918 watched-progress bars render NOWHERE — no engine seam exists for resume positions" | :13-18 | REAL | Cards are inert divs (:137); no progress bar in the render tree; honesty rule inline |
| "PlayerScreen keeps the <video> media surface mounted UNDER the opaque grid" | :29-31 | REAL | PlayerScreen.tsx:25-40 (the UI-half-only media surface, 4.H1 handoff for the decode half) |
| "the art box is the honest 4.H1 placeholder" | :24, :184-191 | REAL | data-placeholder="engine-wiring-4.H1" + sr-note m("player.canvas.placeholder") + glyph — the Wave-0 no-video pin, honestly labeled |
| "the ✕ collapses the card (the header's PiP toggle re-opens it)" | :27-29 | REAL | onHide → onPreviewToggle(false) (:151); header toggle re-opens (:112-122) |
| "engine truth first, playlist truth second … null renders the honest quiet empty — never a fake" | :169-173 | REAL | `state.mediaTitle ?? current?.title ?? null` + data-empty |

## 7. Dead / duplicated / unreachable code
- F-001: folderOf duplicate (MiniPlayer.tsx:105 ↔ here :56).
- Nothing else: all hooks consumed (states.css census), all props wired (PlayerScreen :1067).

## 8. Dependencies & flags
components barrel (Icon/IconButton/Seekbar), list/naturalSort, i18n (w26.i02 lane keys), playlist module (usePlaylist/PlaylistEntry), player context (usePlayerControls), ../controls (mapTransportControls, seekDisableReason), ../laneMsg. No flags. The 4.H1 placeholder contract ties this file to the wave's headline reality — no engine-frame rendering exists anywhere (Wave-0 pin; engine's video path is null-device/compositor-only in the shipped CLI features).

## 9. Open questions for the next agent
- MiniPlayer.tsx (W15-A10) owns the other folderOf copy — F-001 should be fixed once, on their side or a shared module; cross-count once.
- PlayerScreen.tsx (unassigned to me) owns browseId + the <video> surface — my :29-31 verification cites its comments; its auditor should confirm the <video> element actually stays mounted under the browse grid (I read the comment, not the mount code).
