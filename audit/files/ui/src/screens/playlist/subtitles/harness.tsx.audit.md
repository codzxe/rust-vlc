# AUDIT: ui/src/screens/playlist/subtitles/harness.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 152 / 5 harness components + 4 fixture exports (FolderPairsHarness, FolderMultiHarness, FolderOnlineHarness, SubFilenameHarness, SubtitleRowsHarness; HARNESS_EPISODES/FOLDER_CANDIDATES/MULTI_CANDIDATES/ONLINE_CANDIDATES/PATH) |
| File verdict | REAL (dev-only vision harness, used by later waves) — header's evidence citations stale/false; the vision comparison loop it exists for never completed |
| Reachable from shipped entry point? | NO — by explicit design ("never imported by app routes", :1); sole consumer is harness.test.tsx:18 (verified by repo grep). Env-gated HTML export (`W26B11_HARNESS_OUT`) for manual screenshotting. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `grep -rln "FolderPairsHarness\|…"` (sole consumer: harness.test.tsx); `ls design/figma-renders/ \| grep -i subtitle/filename` (Add_Subtitles_to_Playlist.png + -1/-2/-3 + subtitle_one/multiple + sub_filename.png ALL exist — contradicting the header); `cat docs/waves/w26/logs/w26.b11.md` (18 lines: "Vision proof owed (wave-e)"; NO loop record); `grep -n -B2 -A6 "W26B11_HARNESS_OUT" docs/waves/w26/findings/w26.e09.md docs/waves/w26/logs/w26.f06.md` (e09: harness exports run, but all 8 `z-ai vision` attempts HTTP 429; f06: one manual re-export + measurement); `sed -n '1,60p' harness.test.tsx` (2 it-blocks: no-crash + env-gated export); baseline #23 |

## 1. One-paragraph verdict
A dev-only vision harness that does its one job (mounting five frame states of the subtitle flow, including a documented scope-class trick so the filename closeup renders the dialog-scoped control rather than the shared Dropdown chrome) and was genuinely used by later waves — e09 exported its HTML for the comparison loop and f06 re-exported it to measure a fixed control. Two problems: the header's evidence story is false at HEAD ("no PNG renders exist" — design/figma-renders/ contains all seven reference PNGs; "cairosvg exports live in the lane log record" — the lane log w26.b11.md contains no such record and instead says "Vision proof owed"), and the loop the file exists to serve never completed: every automated vision comparison died on HTTP 429, so the subtitle frames' fidelity rests on one manual measurement in f06 plus human eyeballs. Also: the harness wires the dialog the way the recipe documents — `onSave` + `candidates` — a configuration no production mount uses, so its screenshots validate a dialog users cannot reach.

## 2. What actually works (verified)
— verified by `grep -rln` over src/ → harness.tsx imported ONLY by harness.test.tsx (the never-in-app-routes claim holds).
— verified by docs/waves/w26/logs/w26.f06.md:78-82 → the harness HTML was actually exported and measured post-fix (trigger 362×48 r6 vs frame rect x22 y22 362×48) — the tool works end-to-end when driven.
— verified by harness.test.tsx (baseline 1705-pass) → 5 states render without crashing; the env-gated export inlines 9 CSS files (hand-maintained list — same rot risk as the other harnesses).
— verified by read → SubFilenameHarness's `lumen-plsubs` scope class (:115,:120) matches the documented dialog-scoping requirement (subtitles.css targets `.lumen-plsubs .lumen-plsubs-match`).

## 3. Findings

### F-001 [S3]
Where: ui/src/screens/playlist/subtitles/harness.tsx:3-6 vs the tree
Code:
```
// (contract §5) can screenshot them against design/figma-renders/
// {subtitle_one,subtitle_multiple,sub_filename}.png and the "Add Subtitles
// to Playlist/Folder*" states (rendered from the SVGs — no PNG renders
// exist; cairosvg exports live in the lane log record).
```
What is wrong: both evidence claims are false at HEAD. (a) `design/figma-renders/` DOES contain Add_Subtitles_to_Playlist.png, -1.png, -2.png, -3.png, subtitle_one.png, subtitle_multiple.png, sub_filename.png — "no PNG renders exist" is stale. (b) The lane log it cites (docs/waves/w26/logs/w26.b11.md, 18 lines) contains NO cairosvg exports and NO loop record — it records the opposite: "Vision proof owed (wave-e)". The real usage trail lives in w26.e09.md:27-33, where all 8 automated vision attempts failed HTTP 429, and w26.f06.md:78-82, one manual measurement.
Why it matters: the harness's reason to exist is the contract-§5 comparison loop; the committed record shows that loop never completed — the subtitle frames' frame-fidelity claim ("Figma dialog grammar" per the b11 log's Shipped list) has no committed visual proof, and a reader following the header's pointers finds a dead end where a record was promised.
What correct looks like: fix the header to cite e09/f06 (and the vision-429 failure), or actually run the comparison (REQUEST-ORCH-RUN below) and commit the verdicts.
Confidence: HIGH — Verification: ls of figma-renders, full read of w26.b11.md, grep-verified e09/f06 excerpts.
REQUEST-ORCH-RUN: re-run the contract-§5 subtitle-frame comparison (export via `cd ui && W26B11_HARNESS_OUT=/tmp/w26b11 npm run test -- subtitles/harness.test` then screenshot vs design/figma-renders/*.png on a host with a browser) — would close the "vision proof owed" debt or expose drift.

### F-002 [S4]
Where: ui/src/screens/playlist/subtitles/harness.tsx:50-53
Code:
```
 * w26.k15: the mounts pass `onSave` (the documented recipe shape) so the
 * harness exercises the fully-wired dialog — Save gates on the commit seam
```
What is wrong: correct for a harness, but the screenshots it produces (and the e09/f06 verification trail) validate the ONLY fully-wired configuration of this dialog in the repository — no production mount passes `onSave`/`candidates` (my AddSubtitlesDialog audit F-001). The vision loop therefore proves frame fidelity of a dialog state users cannot reach, while the state they do reach (empty body, disabled Save) has never been screenshotted.
Why it matters: verification effort aimed at the wrong variant; the honest-empty state is the shipped one and is unvalidated.
What correct looks like: one more harness state `FolderEmptyHarness` (no candidates, no onSave) so the shipped dead-end form is also visually pinned.
Confidence: HIGH — Verification: cross-ref production mounts (PlaylistPanel.tsx:293, PlaylistScreen.tsx:391/:523).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `HARNESS_EPISODES` / `HARNESS_FOLDER_CANDIDATES` | 22-30 | REAL | — | 15 generated episode/srt names — deterministic |
| `HARNESS_MULTI_CANDIDATES` / `HARNESS_ONLINE_CANDIDATES` / `HARNESS_PATH` | 33-48 | REAL | — | frame fixtures |
| `FolderPairsHarness` | 54-65 | REAL | S4 | fully-wired mount (F-002) |
| `FolderMultiHarness` | 69-81 | REAL | — | file destination, 6 candidates |
| `FolderOnlineHarness` | 84-96 | REAL | — | online source composition |
| `SubFilenameHarness` | 109-126 | REAL | — | scope-class closeup ×2 (closed/open) — documented two-layer trick |
| `SubtitleRowsHarness` | 130-152 | REAL | — | row fragments + a wired SubtitlePickList |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| harness.test.tsx "renders all frame states without crashing" | Only for crash-level breakage | `innerHTML.length > 0` is a smoke assertion |
| harness.test.tsx env-gated export | NO in normal runs | skipped without W26B11_HARNESS_OUT; the CSS_FILES list is hand-maintained (9 files — new sheets silently missing) |
| (the comparison loop itself) | NO — never completed | e09's 8 vision attempts all HTTP 429; no committed verdicts (F-001) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "no PNG renders exist; cairosvg exports live in the lane log record" | harness.tsx:5-6 | FALSE | figma-renders has all 7 PNGs; b11 log has no record (F-001) |
| "see the lane log for the loop record" | harness.tsx:8-9 | FALSE | w26.b11.md says "Vision proof owed (wave-e)" |
| "never imported by app routes" | harness.tsx:1 | REAL | sole consumer harness.test.tsx:18 |
| b11 log "Vision proof owed (wave-e)" / e09 "all HTTP 429" | docs/waves/w26/logs/w26.b11.md, findings/w26.e09.md:32-33 | REAL | the vision debt is honestly recorded — in the logs, not the harness header |

## 7. Dead / duplicated / unreachable code
None in-file (fixtures all used by the harness states). The file itself is intentionally unreachable from production.

## 8. Dependencies & flags
AddSubtitlesDialog + 3 subcomponents + matching.ts + harness.css; eslint-disable `local/i18n-no-literals` with per-site justification (frame captions) — correct dev-only hygiene.

## 9. Open questions for the next agent
- If REQUEST-ORCH-RUN (F-001) executes, the seven figma-renders PNGs are the references — note Add_Subtitles_to_Playlist-1/-2/-3 vs the harness's four states map 1:1 (pairs/multi/online + Folder-1?), confirm which state matches which PNG before diffing.
- The hand-maintained CSS_FILES list pattern appears in ≥6 harness test files (equalizer/video/visualizer/dialogs/playlist-options/subtitles) — a shared constant would end the rot; cheap win for the tools lane.
