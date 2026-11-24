# AUDIT: ui/src/screens/playlist/subtitles/FilenameMatchControl.tsx (compact, 51 LOC — spec §13)
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 51 / 1 (FilenameMatchControl) |
| File verdict | REAL (live local effect; ephemeral scope — nothing engine-side consumes the strategy) |
| Reachable from shipped entry point? | YES — AddSubtitlesDialog.tsx:306 renders it (`pairsMode && <FilenameMatchControl ...>`) inside the add-subtitles flow; harness.tsx is the dev-shell mount |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg match crates/subtitles/src/*.rs` -> NO engine match code; `rg subtitle schemas/generated/settings-docs.md` -> only subtitle-mode / preferred-languages / default-track keys (no match-mode key); consumer trace AddSubtitlesDialog.tsx:112,137,306 |

## 1. One-paragraph verdict
The control is honest and live: picking a strategy immediately re-pairs
the subtitle candidate list through the REAL client-side matcher
(AddSubtitlesDialog.tsx:137 `pairSubtitles(mediaNames, candidates,
matchMode)` — matching.ts is pure, tested code). But the strategy is
ephemeral useState (:112) — the engine has no filename-matching code
(rg over crates/subtitles -> nothing) and no match-mode config key
(settings-docs.md lists only subtitle-mode/preferred-languages/
default-track), so the choice is forgotten when the dialog closes and no
backend ever learns it. Not a dead button — a live button with a
session-limited, UI-local kingdom.

## 2. What actually works (verified)
— Effect chain verified: onChange -> setMatchMode -> useMemo re-pairs ->
MatchedPairsList renders. FilenameMatchControl.test.tsx + matching.test.ts
pin both halves (vitest baseline).
- Dropdown composition (shared primitive, same as VisualizerModeSelect).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/playlist/subtitles/FilenameMatchControl.tsx:44-46 (cross AddSubtitlesDialog.tsx:112)
Code:
      options={MATCH_MODES.map((mode) => ({ value: mode, labelKey: LABEL_KEY[mode] }))}
      value={value}
      onChange={(next) => onChange(next as MatchMode)}
What is wrong: the five-strategy control has no persistence and no engine
consumer — local state only. A user's matching preference resets to
"extra" (DEFAULT_MATCH_MODE, matching.ts:31) on every dialog open. Why it
matters: the settings-savvy UI elsewhere persists everything; this control
silently doesn't, and the engine-side matching gap (no crates/subtitles
match code) means the strategy cannot survive app restarts even in
principle today. What correct looks like: a subtitle.match-mode config key
(engine schema) or at minimum localStorage in the browser lane.
Confidence: HIGH — Verification: read + rg.

## 4. Per-function table (compact)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| FilenameMatchControl | 33-51 | REAL | S3 ephemeral scope (F-001) |

## 5. Test honesty
Colocated test pins rendering + change events; matching.test.ts pins the
five strategies' pairing behavior (would fail if a strategy's matcher
broke). No test asserts persistence — none exists (F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the frame's checked row is 'Title with extra characters' (matching.DEFAULT_MATCH_MODE)" | :7-8 | REAL | matching.ts:31, pinned by matching.test.ts:50 |

## 7. Dead / duplicated / unreachable code
None. LABEL_KEY covers all five MatchMode ids (exhaustive record).

## 8. Dependencies & flags
Dropdown primitive, matching constants, lane i18n. No flags, no transport.

## 9. Open questions for the next agent
- matching.ts auditor: verify the matchers handle path separators, unicode
  normalization, and multi-dot extensions — the control's live effect is
  only as real as those pure functions.
