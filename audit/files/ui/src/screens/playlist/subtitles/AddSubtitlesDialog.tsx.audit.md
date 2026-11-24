# AUDIT: ui/src/screens/playlist/subtitles/AddSubtitlesDialog.tsx
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 326 / 1 component (+save/handlers) |
| File verdict | SHALLOW (real, honest UI shell — but zero backend wiring by design: no client import, no candidate source, no Save commit in any production mount) |
| Reachable from shipped entry point? | YES, as a dead-end: PlaylistScreen.tsx:391-397 (playlist-scoped) and :523-529 (file-scoped) mount it from the playlist toolbar/row-menu actions; both mounts pass NO `candidates` and NO `onSave`. Only the visual harness (subtitles/harness.tsx:56) exercises a wired flow. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 0 |
| Commands run | rg AddSubtitlesDialog mounts; rg "subtitle" src/generated/commands.ts (subtitle.add-file EXISTS, needs host path); sed PlaylistScreen.tsx:380-400,500-535; baseline ui-test.txt (AddSubtitlesDialog/PlaylistScreen suites PASS) |

## 1. One-paragraph verdict
The dialog is internally honest and correctly built: state resets on reopen, mode-gated source/destination composition, real pairing logic (matching.ts, 14 tests baseline-PASS), and a Save that is disabled unless `onSave` is supplied AND the mode has a payload — the header comment states plainly that no host can produce candidates or commit a selection yet. The master-question verdict is therefore blunt: the "Add subtitles" toolbar button in the shipped playlist screen opens a dialog that can NEVER list a candidate (candidates defaults `[]`, no production mount supplies any) and can NEVER save (`onSave` never supplied). It never touches the client or the engine — `subtitle.add-file` exists on the wire (generated/commands.ts:24) but requires a host-filesystem path this web UI cannot produce, and this file never sends it. A prominent user-facing flow that is a guaranteed dead end.

## 2. What actually works (verified)
— verified by rg: `subtitle.add-file` present in the generated schema (`{ args: { encoding?…; path: string; select?: boolean }; cmd: "subtitle.add-file" }`) — the header's claim about the wire is accurate.
— verified by sed: PlaylistScreen.tsx:391-397 and :523-529 mount without `candidates`/`onSave`; the comment there admits "No folder-scan / OpenSubtitles seam exists yet … the candidate list renders the honest empty state — never fabricated."
— verified by baseline: matching.test.ts (14 tests) and the PlaylistScreen Add-subtitles suite (opens dialog, asserts honest empty state "No subtitle files found") PASS.

## 3. Findings
F-001 [S2]
Where: ui/src/screens/playlist/subtitles/AddSubtitlesDialog.tsx:72-87 (props) with PlaylistScreen.tsx:391-397
Code:
```
  /** Candidate subtitle files (folder scan or search results — host seam). */
  candidates?: readonly string[];
  /** Save commit hook (see HONESTY above). */
  onSave?: (result: AddSubtitlesResult) => void;
```
What is wrong: Both optional props default empty/undefined and NO production caller supplies either (rg: only subtitles/harness.tsx:56 passes candidates/onSave; PlaylistScreen passes open/onClose/mediaNames/initialDestination only). Consequently every real user path renders the empty state and a permanently disabled Save (L307-316 `disabled={ onSave === undefined || (pairsMode ? pairs.length === 0 : selected.length === 0) }`). The flow is unreachable-by-construction from the backend side: no folder scan, no OpenSubtitles service, no file picker producing a host path, and `subtitle.add-file` is never invoked from this file. Why it matters: the playlist toolbar and every file row's context menu advertise "Add subtitles" — a core media-player capability — and the destination is a dialog that can commit nothing, in the dev shell AND the desktop shell alike. The honesty labeling (disabled Save, empty state, comments) keeps this out of S0-fraud territory, but the shipped feature is a dead surface. What correct looks like: a desktop-lane host seam — Tauri file dialog → `subtitle.add-file` path — or hiding the action until that seam exists. Confidence: HIGH — Verification: ran rg on all mounts + read both mount sites.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| AddSubtitlesDialog | 92-325 | SHALLOW | S2 | real UI, dead production flow (F-001) |
| ↳ source coercion init | 109-111 | REAL | — | online+non-file coerced to computer (documented k15 P3-9) |
| ↳ reopen-reset effect | 120-127 | REAL | — | clears stale selection/search on close→open |
| ↳ pairs memo | 136-139 | REAL | — | pairSubtitles (real matching, tested) |
| ↳ visibleCandidates memo | 142-147 | REAL | — | honest contains-filter for online mode |
| ↳ save | 162-177 | REAL | — | natural-order commit; pairs mode never leaks stale `selected` |
| ↳ render: header pills/Dropdowns | 184-275 | REAL | — | gated online option; breadcrumb/search per frames |
| ↳ body (MatchedPairsList / SubtitlePickList) | 278-289 | REAL | — | |
| ↳ footer (Cancel / FilenameMatchControl / Save) | 302-321 | REAL | — | honest-disabled Save logic correct |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| AddSubtitlesDialog.test.tsx (baseline PASS) | YES | pins reopen-reset, mode gating, disabled-Save semantics |
| PlaylistScreen.test.tsx:289-303 | YES | pins the honest empty state — and simultaneously PROVES the production dead end (asserts "No subtitle files found" with no candidates source) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the engine catalog has no folder-scan / OpenSubtitles service" | AddSubtitlesDialog.tsx:23-24 | TRUE | generated CommandRequest union (commands.ts:24) has no scan/search commands |
| "`subtitle.add-file` DOES exist … but needs a host-filesystem `path`" | AddSubtitlesDialog.tsx:24-26 | TRUE | commands.ts:24 `path: string` required |
| "without `onSave` Save renders honest-disabled" | AddSubtitlesDialog.tsx:30-31 | TRUE | L307-316; production mounts pass no onSave |

## 7. Dead / duplicated / unreachable code
The whole flow is production-dead despite being production-reachable (F-001). The language Dropdown (L262-271) is a single pinned "english" option with a no-op onChange — honestly commented ("single language pinned by Folder-3"), a stub within a stub.

## 8. Dependencies & flags
Button/IconButton/Icon/Dropdown/Dialog/naturalSort/pairSubtitles. NO client/transport import — the component is engine-blind by design. No flags.

## 9. Open questions for the next agent
- Does the desktop lane plan (doc 05 §280) actually wire a Tauri file dialog to subtitle.add-file anywhere in crates/desktop? If not, the dead end extends across the entire product, not just this file.
- Matrix check: if any feature-matrix row claims subtitle-file-adding works on desktop, it is FALSE at the UI layer — this dialog cannot trigger it.
