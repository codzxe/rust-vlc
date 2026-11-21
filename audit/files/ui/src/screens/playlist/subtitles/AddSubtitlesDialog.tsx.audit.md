# AUDIT: ui/src/screens/playlist/subtitles/AddSubtitlesDialog.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 325 / 3 functions + 2 type/module blocks (AddSubtitlesDialog, save, reopen-reset effect; AddSubtitlesResult/Props interfaces, DESTINATIONS/SOURCES) |
| File verdict | REAL (component logic) mounted into a DEAD END — every production invocation passes no `candidates` and no `onSave`, so the dialog can never show a pair or commit |
| Reachable from shipped entry point? | YES as web-UI source: three production mounts — PlaylistPanel.tsx:293 (queue context menu), PlaylistScreen.tsx:391 (playlist-scoped, `initialDestination="playlist"`), PlaylistScreen.tsx:523 (file-scoped, `initialDestination="file"`); all reachable from App routes. Real shell: no GUI binary (W14); `subtitle.add-file` (the only engine command in this area) needs a host path the web UI cannot produce. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `sed -n '385,400p;515,535p' PlaylistScreen.tsx` (both mounts: NO candidates, NO onSave); PlaylistPanel.tsx:293-297 read (same); `grep -n "empty" MatchedPairsList.tsx SubtitlePickList.tsx` (both lists carry honest empty states :34-36/:147-149); `sed -n 'pairSubtitles' matching.ts` (empty candidates → [] pairs, proven by filter at entry); test census: 22+5+4+17+14 = 62 tests across the subtitles lane; baseline #22/#23 |

## 1. One-paragraph verdict
A frame-exact, well-engineered dialog — reopen-state reset, gated online/destination composition, payload hygiene that keeps a stale multi-pick out of the pairs commit, natural-order commit, 62 passing tests across its lane — whose every production mount hands it an empty candidate set and no save hook. `pairSubtitles(mediaNames, [], mode)` returns zero pairs by construction, so from the queue-row menu, the playlist toolbar, and the file context menu alike, the user gets: title, three pills, an honest "empty" body, and a Save button that is permanently disabled (`onSave === undefined \|\| pairs.length === 0`, line 315). The header is truthful about why (no folder-scan/OpenSubtitles engine service; `subtitle.add-file` needs a host path the web UI cannot produce) — but the honesty lives in comments the user never reads, while the queue-row menu entry that opens this dialog renders ENABLED next to honestly-disabled siblings.

## 2. What actually works (verified)
— verified by matching.ts:115-133 read → empty `candidates` yields `[]` pairs (filter at entry, find over empty); both list components render explicit empty states (MatchedPairsList.tsx:34-36, SubtitlePickList.tsx:147-149) — no blank-body dead UI.
— verified by baseline #23 → AddSubtitlesDialog.test.tsx (22 it-blocks) + FilenameMatchControl (5) + MatchedPairsList (4) + SubtitlePickList (17) + matching.test.ts (14) all pass; tests exercise onSave with real payloads (17 onSave references), gated compositions, reopen reset.
— verified by read of the w26.k15 fixes → :109-111 (invalid online+non-file mount coerced), :121-127 (closed→open resets selected/search — no stale multi-pick leak), :173 (pairs mode commits `[]` selected — no stale candidate payload), :211-213 (leaving "file" destination drops online source).

## 3. Findings

### F-001 [S3]
Where: ui/src/screens/playlist/subtitles/AddSubtitlesDialog.tsx:309-315, vs the three mounts (PlaylistPanel.tsx:293-297, PlaylistScreen.tsx:391-396, :523-528)
Code:
```
              onSave === undefined || (pairsMode ? pairs.length === 0 : selected.length === 0)
```
What is wrong: no production mount passes `candidates` or `onSave` (verified at all three sites) — so the dialog's Save is structurally always disabled and its body always the empty state. The feature is complete and tested, but its only shipped invocations dead-end. The user-facing chain: queue-row context menu → "Add subtitle tracks" (ENABLED, PlaylistPanel.tsx:437-443) → modal that can never list or commit anything. The same menu disables favorite/save-playlist/add-audio as "no seam" — the subtitles row is treated as seamed because the DIALOG exists, but the dialog itself has no seam (no candidates source, no save hook).
Why it matters: this is the wave's "dead buttons" exhibit: an enabled entry that opens an inert modal — three clicks deep of honesty visible only to a reader of source comments; the honest-disabled grammar the file's own family uses was not applied at the entry-point level.
What correct looks like: either disable the menu entries until a host wires candidates/onSave (matching the favorite/save rows), or mount with a real `onSave` that at minimum reports the selection to the playlist store (the local-seam precedent PlaylistScreen already uses for saved playlists).
Confidence: HIGH — Verification: read of all three mounts + matching.ts empty-input proof + Save-disable expression.

### F-002 [S4]
Where: ui/src/screens/playlist/subtitles/AddSubtitlesDialog.tsx:262-271
Code:
```
                <Dropdown
                  className="lumen-plsubs__pill"
                  options={[{ value: "english", labelKey: "playlist.subtitles.language.english" as const }]}
                  value="english"
                  onChange={() => {
                    /* single language pinned by Frame-3 */
                  }}
```
What is wrong: a single-option Dropdown with a no-op onChange — a control that renders as an interactive dropdown (chevron, hover states, aria-expanded) but can never change anything. Frame fidelity (Folder-3's language pill) is honored; affordance honesty is not: the shared Dropdown announces itself as a menu.
Why it matters: a decorative widget that promises choice; keyboard/AT users open it to find one immutable option — a small trust leak in the same dialog that prides itself on never faking.
What correct looks like: a non-interactive pill (text + flag glyph) until a second language exists, or `disabled` with the staged-inert grammar settings.css uses for exactly this situation.
Confidence: HIGH — Verification: read; Dropdown component renders full menu semantics (menu.css audited this wave).

### F-003 [S4]
Where: ui/src/screens/playlist/subtitles/AddSubtitlesDialog.tsx:98, 101 vs PlaylistPanel.tsx:293-297
Code:
```
  initialDestination = "folder",
  ...
  folderPath = [],
```
What is wrong: the default destination is "folder", and the queue mount (PlaylistPanel) passes neither `folderPath` nor `initialDestination` — so the dialog opens scoped "To This Folder" with an EMPTY breadcrumb (folder icon + zero path text, line 243 renders null) while the actual scope is the playlist queue. The two PlaylistScreen mounts pass the correct initialDestination ("playlist"/"file"); the queue mount is the odd one out.
Why it matters: scope mislabeled at the first paint of a modal whose whole purpose is scope (To This X); the empty crumb pill is itself a dangling affordance.
What correct looks like: PlaylistPanel mounts with `initialDestination="playlist"`; default should arguably be "playlist" (the safest scope) rather than "folder".
Confidence: HIGH — Verification: read of defaults + all three mounts.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `AddSubtitlesResult` / `AddSubtitlesDialogProps` / `DESTINATIONS` / `SOURCES` | 56-90 | REAL | — | prop contract documented per-item; `onSave` optional (the honesty seam) |
| `AddSubtitlesDialog` | 92-325 | REAL (logic) / DEAD-END (mounts) | S3/S4 | hooks unconditional before null return (:149) — correct; reopen reset :121-127 correct; pairs memo gated; F-001/F-002/F-003 |
| `save` | 162-177 | REAL | — | payload hygiene (pairsMode → selected []), naturalSort commit, onClose after commit |
| reopen-reset effect | 120-127 | REAL | — | wasOpen ref pattern — correct edge handling (open→open no reset) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| AddSubtitlesDialog.test.tsx (22, baseline-passed) | YES | real-props tests: onSave payloads asserted, gating, reopen reset, Escape behavior (the two g19 regression tests :151-160) |
| matching.test.ts (14) | YES | pure matcher, exact pairs |
| SubtitlePickList (17) / MatchedPairsList (4) / FilenameMatchControl (5) | YES | selection semantics, empty states, pinned modes |
| ALL of the above | NO for production reality | every test passes `candidates`/`onSave` — the two props no production mount provides (F-001); the suite proves the component works exactly as designed for callers that do not exist |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the engine catalog has no folder-scan / OpenSubtitles service (doc 05 §2)" | AddSubtitlesDialog.tsx:23-24 | REAL | generated/commands.ts grep: no such commands |
| "`subtitle.add-file` DOES exist (doc 05 §280) but needs a host-filesystem `path`" | AddSubtitlesDialog.tsx:24-27 | REAL | commands.ts:24 `cmd: "subtitle.add-file"` args `{ encoding?, path, select? }` — path is a string, no blob variant |
| "without `onSave` Save renders honest-disabled (w26.k15, j15 P2-3)" | AddSubtitlesDialog.tsx:30-31 | REAL | :315 verified — and F-001 notes every production mount hits this branch |
| "no shipped host can produce candidates or commit a selection yet" | AddSubtitlesDialog.tsx:27-29 | REAL | the three mounts pass no candidates/onSave |

## 7. Dead / duplicated / unreachable code
No in-file dead code. The feature-as-mounted is the dead code: SubtitlePickList/online-search/MatchedPairsList/matching.ts are exercised only by tests + harness.tsx (dev-only, my file 13) because no mount supplies candidates.

## 8. Dependencies & flags
Dialog/Dropdown/Button/IconButton/Icon/Card primitives; naturalSort from list; 5 sibling modules in subtitles/ (each with tests). No flags.

## 9. Open questions for the next agent
- F-001 generalizes: PlaylistScreen's OWN toolbar "Add-Subtitles" action (test :289 asserts it opens the dialog) — same dead end. The playlist wave owner should decide entry-point-disable vs local-seam onSave; my PlaylistPanel audit (F-001/F-003) and this file tell the same story from both ends.
- `subtitle.add-file`'s `[PROPOSAL]` blob variant (header :25-26) — if doc 05 promises a web-capable variant, matrix rows citing it are FALSE-at-HEAD; docs wave should reconcile.
