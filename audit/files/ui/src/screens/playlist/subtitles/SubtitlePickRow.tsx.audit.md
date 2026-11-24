# AUDIT: ui/src/screens/playlist/subtitles/SubtitlePickRow.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 80 / 1 component |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: SubtitlePickList.tsx:25 mounts it inside AddSubtitlesDialog (PlaylistScreen/PlaylistPanel mounts) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg class/consumer checks (exit 0); baseline ui-test.txt (SubtitlePickList tests pass) |

## 1. One-paragraph verdict
A pure presentational option row: file glyph, name, selected check / multiple-cycle glyph, with all interactive semantics (role, roving tabindex, handlers) spread in from SubtitlePickList via `optionProps` — correct separation, no invented behavior. All 4 class tokens styled in subtitles.css (11 rule hits). No findings.

## 2. What actually works (verified)
— Consumer + stylesheet wiring verified (rg).
— Decorative glyphs aria-hidden; ref-as-prop for roving focus.

## 3. Findings
None. Checked: no handlers stolen from the listbox, no literals, class/variant composition total.

## 4. Per-function table
N/A (<100 LOC): single component reviewed in full.

## 5. Test honesty
SubtitlePickList tests (baseline pass) exercise the row through the listbox (selection, keyboard).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The row is PRESENTATIONAL: selection semantics ... belong to SubtitlePickList" | L17-18 | TRUE | only `{...optionProps}` spread; no onClick of its own |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Icon + lane glyphs. No gates.

## 9. Open questions for the next agent
- None (data honesty of the surrounding dialog is covered in the matching.ts audit).
