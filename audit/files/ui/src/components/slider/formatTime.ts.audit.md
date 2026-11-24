# AUDIT: ui/src/components/slider/formatTime.ts
| Field | Value |
|---|---|
| Auditor | W15-R2 (credited: A07) |
| Date | 2026-11-23 |
| Lines / functions | 15 / 1 (`formatSeekTime`) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — main.tsx -> App.tsx -> PlayerScreen.tsx:110 (GotoTimeDialog) and :107 (ControlBar); GotoTimeDialog.tsx:38,60,79,125-126 calls formatSeekTime |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | node -e (14-case execution of a byte-equivalent copy, output below); rg consumers (exit 0); sed Seekbar.test.tsx:154-165 |

## 1. One-paragraph verdict
Correct pure formatter. I executed a byte-equivalent copy against 14 inputs:
0->"0:00", 61->"1:01", 600->"10:00", 3600->"1:00:00", 3661->"1:01:01",
7325->"2:02:05", -3/-100.7->"0:00" (negative clamp works), 12.9->"0:12"
(truncation), 1e9->"277777:46:40". The doc comment's claims (`M:SS`/`H:MM:SS`,
negative-safe) are all true. Consumers are real: GotoTimeDialog.tsx:60,79,125,
ControlBar.tsx:128 reference, Seekbar.tsx, barrel re-export (slider/index.ts).
Test coverage exists and is honest (Seekbar.test.tsx:154-160 pins 5 forms
including the negative clamp).

## 2. What actually works (verified)
— verified by `node -e '<copy>'` -> all 14 outputs correct (list above).
— verified by `rg -ln formatSeekTime` -> 7 files: Seekbar.tsx, barrel, 3
  screens/dialogs consumers, test file.

## 3. Findings
None. Checked: padStart logic (minute padding only when h>0 — matches M:SS
spec), negative clamp (Math.max(0,...)), floor-vs-round (floor, correct for a
progress display), hour overflow (no cap — a 277777-hour string is ugly but the
input is impossible from real media durations), NaN input (produces "NaN:NaN"
— standard JS coercion, no doc claim violated, callers pass engine numbers).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `formatSeekTime` | 8-15 | REAL | — | Executed; all forms correct |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| Seekbar.test.tsx:154-160 | YES | Pins exact strings for 0/65/600/3725/-3; removing padStart or the clamp fails it |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Default time formatter: `M:SS` / `H:MM:SS`, negative-safe" | formatTime.ts:7 | REAL | node execution: 65->"1:05", 3725->"1:02:05", -3->"0:00" |
| "the barrel and the control bar keep importing the formatter" | formatTime.ts:4-5 | REAL | slider/index.ts + ControlBar.tsx:128 / GotoTimeDialog.tsx:38 import it |

## 7. Dead / duplicated / unreachable code
None. No duplicated time formatter elsewhere? UNVERIFIED — did not sweep all
of ui/src for another M:SS formatter; BookmarksDialog.tsx appeared in the rg
consumer list (it imports it, so no duplication there).

## 8. Dependencies & flags
None. Pure function, zero imports.

## 9. Open questions for the next agent
None.
