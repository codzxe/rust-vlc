# AUDIT: ui/src/screens/player/states/PlaylistBrowse.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 239 / 3 (folderOf, PlaylistBrowse, NowPlayingCard) |
| File verdict | SHALLOW (real transport controls; media grid is inert presentation) |
| Reachable from shipped entry point? | YES — PlayerScreen.tsx:126 imports, :1067 mounts `<PlaylistBrowse>` when a dock row opens a saved playlist; PlayerScreen is the /player home route (App.tsx:303) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | grep mounting chain; sed quote verify (L129-141); full read |

## 1. One-paragraph verdict
The saved-playlist browse face. Its now-playing card is genuinely wired (Seekbar -> actions.seek over the real player context; title/folder from engine+playlist truth; PiP toggle honestly disabled with reason while no session). But the media GRID — the face's main surface — is inert decoration: saved playlists persist file NAMES only (no sources, no play seam), so every card is a non-interactive div with a generic thumbnail icon; you can look at your playlist, not play it. The header comment honestly declares all of this (watched-progress bars "render NOWHERE — no engine seam ... a fabricated bar would be theater").

## 2. What actually works (verified)
— verified by read + grep chain (PlayerScreen.tsx:126,1067): seek/title/folder/chip wiring identical to MiniPlayer's real seams (usePlayerControls/usePlaylist); naturalSort ordering; key collision guard for duplicate names (L130-133).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/states/PlaylistBrowse.tsx:134-141
Code:
```
              {/* Honest presentation, never a dead button: names-only local
                  data has no play seam (playlistsStore; the ThumbnailGrid
                  inert-card precedent). */}
              <div className="lumen-state-browse__card" title={name}>
                <span className="lumen-state-browse__card-art" aria-hidden="true">
                  <Icon name="thumbnail" />
                </span>
              </div>
```
What is wrong: the entire media grid is unclickable — no button, no play action, no context menu; saved playlists cannot be played from their own browse surface. Root cause is upstream (playlistsStore persists names only — see my playlistsStore.ts audit), and the card honestly renders as presentation rather than faking a control. Why it matters: master-question census — the saved-playlist feature's core interaction (open a playlist, play an item) does not exist; the surface is a gallery of icon placeholders. What correct looks like: persist sources (file paths) at save time and wire click -> playlist.enqueue/play. Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| folderOf | 53-58 | REAL-UNPROVEN | — | local twin of MiniPlayer's helper (noted duplication, harmless) |
| PlaylistBrowse | 75-155 | SHALLOW | S3 | header controls real (exit, honest-disabled PiP); grid inert (F-001) |
| NowPlayingCard | 161-239 | REAL-UNPROVEN | — | real seek/state; art box honest placeholder (canvas <video> stays mounted underneath per header L25-27) |

## 5. Test honesty
No dedicated test file for PlaylistBrowse found in the states dir during scan (StateRouter.test.tsx may cover the face — UNVERIFIED depth). The placeholder attributes are pinned by variants/PlayerScreen tests generally.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "PlayerScreen keeps the <video> media surface mounted UNDER the opaque grid, so url sources keep their real bytes and audio flowing" | header L25-27 | REAL-UNPROVEN | consistent with PlayerScreen.tsx:1089-1096 mount logic; not executed |
| "the frame's ... watched-progress bars render NOWHERE — no engine seam exists for resume positions" | header L15-17 | REAL | no progress bar in grid code |

## 7. Dead / duplicated / unreachable code
folderOf duplicated from MiniPlayer (self-documented, S4-grade at most); no dead code.

## 8. Dependencies & flags
player context, playlist context, Seekbar, naturalSort, laneMsg. No feature flags.

## 9. Open questions for the next agent
- Should playlistsStore persist sources? (store lane decision; the NewPlaylistDialog drop zone reads real files — sources EXIST at creation time and are thrown away — see my playlistsStore audit.)
