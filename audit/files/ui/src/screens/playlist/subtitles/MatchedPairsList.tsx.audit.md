# AUDIT: ui/src/screens/playlist/subtitles/MatchedPairsList.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 69 / 1 component (MatchedPairsList) + props interface |
| File verdict | REAL (correct, tested — but its rows are production-dead: every mount starves it to the empty state) |
| Reachable from shipped entry point? | YES-EMPTY-ONLY: sole production consumer AddSubtitlesDialog.tsx:280 (pairs mode) computes `pairs` from `mediaNames`/`candidates`, and all 3 production mounts pass neither (my AddSubtitlesDialog audit, file 8) → production renders only the empty state; rows render in tests/harness only. No GUI binary exists at all (W14). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n "MatchedPairsList" src/` (consumer = AddSubtitlesDialog + index.ts re-export + tests); `rg -n "lumen-plsubs-pair" src/` (all 6 classes styled in subtitles.css:329-391); read matching.ts:104-135 (pairSubtitles), natural.ts:90-92 (no dedupe), w26.b11.ts (all 3 msg keys exist), SubtitleGlyphs.tsx (size = width, aspect kept); baseline #23 (vitest 1705 pass incl. MatchedPairsList.test.tsx) |

## 1. One-paragraph verdict
A small, clean presentational list: every class it emits has a live rule (subtitles.css:329-391, incl. the frame-exact 8.15px dot-pitch leader and the w26.gfix-E 40px-pitch fix), the i18n seam resolves all three keys (w26.b11.ts:44-47), and 4 tests pin anatomy/order/empty/label-override. But it inherits its parent's death: production mounts pass no `mediaNames`/`candidates`, so `pairSubtitles` always returns `[]` and only the empty state ever ships. Two real defects inside the lane: the "honest empty state" copy lies when subtitle files exist but none matched (match mode "off" guarantees it), and `key={pair.mediaName}` collides/silently drops rows for duplicate filenames — realistic for a playlist spanning folders.

## 2. What actually works (verified)
— verified by read + cross-ref → all 6 `lumen-plsubs-pair*` selectors styled in subtitles.css (flex row, 40px pitch, fixed 472px subtitle column, radial-gradient dotted leader at frame 8.15px pitch) — no orphan classes.
— verified by read w26.b11.ts:44-47 → `pairs-label`, `empty-title`, `empty-body` all present in the lane catalog; useSubtitlesMsg seam resolves them (fallback order m(key) ?? laneCatalog).
— verified by MatchedPairsList.test.tsx:17-45 (baseline #23, 1705 pass) → list role+name, 2 listitems with both names, anatomy counts (2 icons/1 leader/2 names), empty state, label override.
— verified by read SubtitleGlyphs.tsx:24-78 → glyph `size` sets width, height scales by frame aspect (18×20 / 18×16) — no distortion.

## 3. Findings

### F-001 [S3]
Where: ui/src/screens/playlist/subtitles/MatchedPairsList.tsx:32-39
Code:
```
  if (pairs.length === 0) {
    return (
      <div className="lumen-plsubs-empty" id={id}>
        <p className="lumen-plsubs-empty__title">{t("playlist.subtitles.empty-title")}</p>
```
What is wrong: the empty state fires whenever `pairs.length === 0`, but the copy ("No subtitle files found", w26.b11.ts:46) claims files are absent. In the dialog's pairs mode (AddSubtitlesDialog.tsx:279-280 the MatchedPairsList is the ONLY body content) with match mode "off", `pairSubtitles` returns `[]` for ANY candidate set (matching.ts:130 `if (candidate === undefined) continue;` and matchesMode "off" never matches — matching.test.ts:125 pins `mode "off"` → `[]`), so a folder full of subtitle files is reported as containing none.
Why it matters: the test at :36-40 names this the "honest empty state"; it is honest only for the zero-candidates case and actively false for the unmatched case. A user switching the footer match dropdown to "Off" sees "No subtitle files found" while the folder has files.
What correct looks like: distinguish "no candidates" from "none matched" (second string, e.g. "No matches under this strategy"), or scope the copy to both cases ("No matching subtitle files").
Confidence: HIGH — Verification: read of matching.ts + matching.test.ts:125 + AddSubtitlesDialog.tsx:277-289.

### F-002 [S3]
Where: ui/src/screens/playlist/subtitles/MatchedPairsList.tsx:48
Code:
```
      {pairs.map((pair) => (
        <li key={pair.mediaName} className="lumen-plsubs-pair" role="listitem">
```
What is wrong: React key is the bare media filename. `pairSubtitles` does not dedupe media names (matching.ts:126 iterates `naturalSort(mediaNames)`, and natural.ts:90-92 is a plain stable sort — no dedupe), so a playlist spanning two folders with the same filename (realistic: "S01E01.mkv" in two season folders) yields either two rows sharing one key (React reconciliation bug) or, when only one subtitle candidate matches, one row while the second media file silently vanishes from the review list the user is about to approve.
Why it matters: this list's whole job is pre-save review; dropped/duplicated rows mean the user approves a plan that does not match what the host will do.
What correct looks like: key on the pair index or a composite (folder-qualified path once the host supplies one); dedupe in `pairSubtitles`.
Confidence: HIGH — Verification: read matching.ts:115-135 + natural.ts:90-92.

### F-003 [S4]
Where: ui/src/screens/playlist/subtitles/MatchedPairsList.tsx:50
Code:
```
            <MediaFileGlyph size={19} />
```
What is wrong: the file's own frame header pins "media glyph 18×16" / "subtitle glyph 18×20" (:9, :14) but both glyphs render at width 19 (:50, :61). Every other frame delta in this lane is logged in comments (the CSS logs #6B6B6B→tertiary, leader #373737→hairline); this +1px width delta is unlogged and unexplained.
Why it matters: the lane's stated contract is pixel-perfect porting with logged deltas (contract §4.4); an unlogged delta breaks the audit trail convention the sibling files maintain.
What correct looks like: `size={18}` or a comment logging the 19px delta like the others.
Confidence: HIGH — Verification: read SubtitleGlyphs.tsx (size = svg width) + this file's header comment.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MatchedPairsListProps` | 22-28 | REAL | — | pairs/label/id all consumed; documented as pairSubtitles output |
| `MatchedPairsList` | 30-69 | REAL | S3 | empty-state branch F-001; key F-002; glyph size F-003; `role="list"/"listitem"` on ul/li is the correct VoiceOver restoration for `list-style:none` rows (subtitles.css:335); icons aria-hidden; leader + side column match CSS contract |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| renders a labelled list of pair items in the given order (:17-25) | YES | role+name query, count, both names asserted |
| frame anatomy 2 icons/1 leader/2 names (:27-34) | YES | class-count DOM assertions |
| honest empty state (:36-40) | PARTLY | pins the strings but the "honest" name is wrong in the unmatched case (F-001) — test cannot catch copy-scope lies |
| accessible-name override (:42-45) | YES | getByRole with name "Pairs" |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "15 rows pairing every media file with the subtitle file the current strategy selected" | MatchedPairsList.tsx:1-4 (header) | SHALLOW | rows render only when fed; production feeds nothing (AddSubtitlesDialog mounts pass no candidates); "every media file" is false under duplicate names (F-002) and unmatched files are omitted entirely (no unpaired indicator) |

## 7. Dead / duplicated / unreachable code
The non-empty branch (rows) is production-unreachable (parent starves it). Nothing duplicated.

## 8. Dependencies & flags
SubtitleGlyphs (aspect-correct ports), useSubtitlesMsg/w26.b11 lane keys, matching.pairSubtitles. No feature flags.

## 9. Open questions for the next agent
- If the host ever wires real candidates, decide whether unpaired media rows (media with NO matching subtitle) should render in the review list — currently they are silently absent, so the "review" shows only successes.
