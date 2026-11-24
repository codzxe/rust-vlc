# AUDIT: ui/src/client/open-path.ts
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 75 / 4 (isPlaylistPath, classifyOpenSource, openPathRequest, openPathRoute) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — the ONE funnel: OpenMediaDialog, OnboardingScreen, StageOverlays, ControlBar all build opens through it (rg) -> real commands on the shared transport |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg apps/desktop/src/files.rs is_playlist_path (mirror verified, same 5 suffixes); rg consumers |

## 1. One-paragraph verdict
The file-open classification rule: playlist suffixes -> `playlist.load
{append:false}`, everything else -> `session.open`, URLs always media.
Master-question relevant: every UI-originated open funnels through here into
real catalog commands. The claimed Rust mirror is verified — files.rs:53-58
carries the same five suffixes with the same case-insensitive compare and the
same Path::extension dotfile semantics the TS side re-implements
(`dot <= nameStart`). No findings.

## 2. What actually works (verified)
- Mirror check: files.rs:54 `["m3u","m3u8","pls","xspf","cue"]` == PLAYLIST_FILE_EXTENSIONS; both lowercase before compare; both treat dotfiles as extension-less (TS: lines 33-34; Rust: Path::extension None for ".m3u").
- Four production consumers build their opens through openPathRequest (rg) — the "ONE funnel" claim holds on the UI side.
- requestId is a PARAMETER here (unlike toolbarItems' hardcoded ids) — the correct pattern; callers can keep monotonicity.

## 3. Findings
None. Checked: suffix-parity with the engine (verified), dotfile edge
(handled), trailing-dot edge (`dot === length-1` -> false, line 30),
backslash paths (handled, line 33), URL short-circuit (correct — a URL
ending in .m3u8 is still media-opened, which matches "URLs are always
media" and is the documented intent).

## 4. Per-function table (<100 LOC)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| PLAYLIST_FILE_EXTENSIONS | 19-25 | REAL | mirror-verified |
| isPlaylistPath | 28-37 | REAL | Path::extension semantics |
| classifyOpenSource | 45-50 | REAL | |
| openPathRequest | 58-70 | REAL | typed CommandRequest, no casts |
| openPathRoute | 73-75 | REAL | |

## 5. Test honesty
open-path.test.ts exists (consumer scan); the dotfile/suffix edges are the
kind of cases it should pin — spot-verified file presence, full case list
UNVERIFIED (belongs to its own audit if assigned).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The Rust shell mirrors this exact rule (apps/desktop files.rs is_playlist_path + dispatch.rs open_path_request)" | open-path.ts:9-11 | REAL | files.rs:53-58 read; dispatch.rs open_path_request seen in frame-channel audit's rg (apps/desktop/src/dispatch.rs:253) |
| "the UI never parses [playlists]" | open-path.ts:4-6 | REAL | no playlist parser exists in ui/src (classify only by suffix) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Generated transport types only. No gates.

## 9. Open questions for the next agent
- None.
