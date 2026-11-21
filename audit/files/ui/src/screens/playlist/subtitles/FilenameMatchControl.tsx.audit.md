# AUDIT: ui/src/screens/playlist/subtitles/FilenameMatchControl.tsx
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 51 / 1 component (+1 interface +1 const map) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle — AddSubtitlesDialog.tsx:306 renders it in the footer whenever pairs mode is on (the dialog opens from /playlist file-row menu / F2 / playlist context, all real mounts). W14 caveat: no shipped desktop binary (build-desktop-tauri.log.txt exit 101). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | grep MATCH_MODES/matching.ts (5 modes, default "extra" — matches the frame's checked row claim); grep mounts (AddSubtitlesDialog.tsx:306 production; harness.tsx:116,121); grep i18n keys (6 lane keys w26.b11.ts, 12 catalog entries messages.ts — all five LABEL_KEY ids covered). |

## 1. One-paragraph verdict
A correct 40-line wrapper: the shared Dropdown composed with the frame's five match strategies, controlled value/onChange, labels through the lane i18n seam, cast on change safe because options are exactly the MatchMode union. The catch is systemic (recorded in my subtitles.css audit): the strategy this control selects drives matching.ts's pairing of candidates to media names, and candidates can never exist in any production mount (no folder-scan/OpenSubtitles seam; all three dialog mounts pass empty-candidate data) — so the control is an interactive selector for a strategy that has nothing to match in every shipped flow. It IS reachable and the matching logic is real and tested; the pipeline upstream of it is the void.

## 2. What actually works (verified)
— Options/labels: MATCH_MODES (matching.ts:26) is exactly LABEL_KEY's domain; all five lane keys registered. Controlled wiring correct. Baseline: FilenameMatchControl.test.tsx + AddSubtitlesDialog suites pass (ui-vitest.log.txt); `tsc --noEmit` clean (ui-build.log.txt).

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/playlist/subtitles/FilenameMatchControl.tsx:44-46 (+ AddSubtitlesDialog.tsx:306)
Code:
```
      options={MATCH_MODES.map((mode) => ({ value: mode, labelKey: LABEL_KEY[mode] }))}
      value={value}
      onChange={(next) => onChange(next as MatchMode)}
```
What is wrong: the selected strategy feeds matching.ts's candidate↔media pairing, but no production mount can ever supply candidates (the three AddSubtitlesDialog mounts — PlaylistScreen.tsx:391,523, PlaylistPanel.tsx:293 — pass no candidate source; "No folder-scan / OpenSubtitles seam exists yet"). The control stays fully interactive while its output has nothing to act on — the user picks "Exact title match" and the pairs list shows the same unmatched media names as "Off".
Why it matters: a live-looking control with no live effect in any shipped build (the subtitles.css audit's F-001 from the control's side); blast radius is one dialog.
What correct looks like: disable the control (with the staged-row pattern) until candidates exist, or wire the candidate source.
Confidence: HIGH — mount chain traced end-to-end.
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `LABEL_KEY` (const) | 16-22 | REAL | — | 5 modes ↔ 5 lane keys, exhaustive |
| `FilenameMatchControlProps` (interface) | 24-31 | REAL | — | |
| `FilenameMatchControl` | 33-51 | REAL | S3 (F-001) | shared Dropdown composition; safe cast; className join |

## 5. Test honesty
FilenameMatchControl.test.tsx (other agent) pins options/labels/value wiring — would fail on label or option mutations; cannot catch F-001 (nothing asserts the selection changes any production output).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the frame's checked row is 'Title with extra characters' (matching.DEFAULT_MATCH_MODE)" | FilenameMatchControl.tsx:7-8 | REAL | matching.ts:31 `DEFAULT_MATCH_MODE: MatchMode = "extra"` |
| "Options verbatim: Off / Any title / Contains title / Title with extra characters / Exact title match" | FilenameMatchControl.tsx:6-7 | REAL-UNPROVEN | five lane keys exist; English strings not compared to the frame here |

## 7. Dead / duplicated / unreachable code
None in-file (the dead-effect issue is F-001's systemic pipeline gap).

## 8. Dependencies & flags
Dropdown (shared menu), matching.ts (MATCH_MODES/MatchMode), useSubtitlesMsg lane seam. All exist.

## 9. Open questions for the next agent
- matching.ts owner: is the pairing logic's no-candidate behavior (all media unmatched) asserted anywhere, or only the with-fixtures path?
- AddSubtitlesDialog owner: does pairs mode re-match live when the strategy changes (state flow matchMode → matched pairs)?
