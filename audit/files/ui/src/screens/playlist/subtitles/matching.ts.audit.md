# AUDIT: ui/src/screens/playlist/subtitles/matching.ts
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 135 / 7 exported fns + 2 consts |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: AddSubtitlesDialog (mounted by PlaylistScreen.tsx / PlaylistPanel.tsx) imports pairSubtitles/matchesMode (AddSubtitlesDialog.tsx:45) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg consumer chain + subtitle.add-file host check (exit 0/1); baseline ui-test.txt cited |

## 1. One-paragraph verdict
Pure, well-scoped filename-matching logic for the Add-Subtitles review list: VLC-style normalization (lowercase, separator folding), conservative tag stripping, five strategies, deterministic greedy pairing in natural order with a used-set so one subtitle never pairs twice. No backend claims — the surrounding dialog passes host-provided names in and reports selections out through `onSave` (Save renders honest-disabled without it), because no shipped host can produce subtitle candidates or commit a selection (`subtitle.add-file` needs a host-filesystem path the web UI cannot produce; no host implements it — rg: 0 matches in dispatch.rs AND demo-transport.ts). One S4: the "conservative" tag word list contains common English words ("web", "internal", "proper") that can clip real titles.

## 2. What actually works (verified)
— All functions pure; `pairSubtitles` is deterministic (candidates natural-sorted, first-match, used-set).
— Consumer chain verified: PlaylistScreen/PlaylistPanel -> AddSubtitlesDialog.tsx:45 imports.
— matching.test.ts passes (baseline ui-test.txt).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/playlist/subtitles/matching.ts:66-72
Code:
```
const TAG_WORDS: readonly string[] = [
  "720p", "1080p", "2160p", "480p",
  "hdtv", "web", "webrip", "web-dl", "bluray", "brrip", "dvdrip", "hdrip",
```
What is wrong: "web", "internal", "proper" are ordinary English words; a title like "Charlotte's Web" normalizes to "charlotte's" after the cut (titleOfStem cuts at the first tag word), so "contains" compares against a clipped title and "extra"/"exact" can miss ("charlotte's web 2006.srt" vs media "Charlotte's Web" → exact fails because media title became "charlotte's"). The comment claims "Conservative — only well-known tag words are cut so real titles are never clipped", which is not strictly true for these words. Blast radius: a few missed pairs in a review list the user can still fix by hand. Confidence: HIGH — Verification: read (mutation reasoning on the exported pure fns).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MATCH_MODES/DEFAULT_MATCH_MODE | 26-31 | REAL | - | frame row order; default "extra" per frame |
| isSubtitleFile | 37-41 | REAL | - | extension set; dot<=0 guard |
| splitExtension | 44-48 | REAL | - | last-dot split |
| normalizeStem | 54-59 | REAL | - | separator folding |
| TAG_WORDS/titleOfStem | 66-79 | REAL | S4 | F-001 |
| matchesMode | 82-102 | REAL | - | exhaustive switch; empty-guard |
| pairSubtitles | 115-135 | REAL | - | deterministic greedy |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| matching.test.ts (baseline pass) | YES | strategies/normalization/pinning asserted; would NOT catch the F-001 word-collision (no such case seeded) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Conservative — only well-known tag words are cut so real titles are never clipped" | L63-64 | FALSE at the margin | F-001 |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
naturalSort from list primitives. No gates.

## 9. Open questions for the next agent
- AddSubtitlesDialog is mounted by PlaylistScreen/PlaylistPanel — with WHAT candidates in production? If candidates are always empty in real hosts, the matched-pairs product this module powers is demo/harness-only surface.
