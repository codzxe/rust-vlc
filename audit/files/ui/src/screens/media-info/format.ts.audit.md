# AUDIT: ui/src/screens/media-info/format.ts
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 57 / 4 (formatDurationUs, formatDuration, codecLabel, sourceLocation) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — consumed by media-info/MediaInfoPanel.tsx, open-media/OpenMediaDialog.tsx, settings/SettingField.tsx, playlist/subtitles (rg list) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `node -e` execution of formatDurationUs (5 cases, output below); `rg -n "usToSeconds" wire-time.ts`; consumer scan |

## 1. One-paragraph verdict
Pure, total, honest formatters. The µs→h:mm:ss path executes correctly
(3661000000µs -> "1:01:01"; negative clamps to "0:00"; truncation not
rounding); `unknown` durations return null (panel renders "Unknown", never a
fabricated value); `infinite` is passed through as engine truth; codecLabel
returns null when the descriptor carries nothing. Zero findings.

## 2. What actually works (verified)
— `node -e` (faithful transcription): `formatDurationUs(3661000000)` ->
"1:01:01"; `(65500)` -> "0:00" (65.5 ms — correct µs semantics);
`(0)`/`(-5)` -> "0:00" (clamp). — usToSeconds confirmed at
client/wire-time.ts:31 (µs/1e6 — the w26.k17 unit-split fix this file cites).
— Consumers verified (4 production files).

## 3. Findings
None. Checked and how: unit correctness (executed); fabricated values (none —
null-honest contract held in all branches); dead code (none — all 4 exports
consumed); overflow (µs counts are far below Number.MAX_SAFE_INTEGER).

## 4. Per-function table
N/A (<100 LOC — compact audit). All four functions REAL: formatDurationUs
(L13-21, executed), formatDuration (L28-37, total switch), codecLabel (L44-51,
null-honest), sourceLocation (L55-57, total).

## 5. Test honesty
MediaInfoScreen.test.tsx and MediaInfoPanel tests consume these formatters
(other agents' scope); the formatter logic itself is execution-verified here.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "absent input returns `null` (rendered as the i18n 'Unknown' ... never a fabricated value)" | format.ts:1-3 | REAL | L34-35, L48 branches |
| "Truncates, never rounds" | format.ts:9 | REAL | Math.floor (L14), executed |
| "`Duration.time` is a µs Timestamp on the 1.D4 wire" | format.ts:10-11 | REAL | wire-time.ts:31 usToSeconds |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
client/transport types, client/wire-time (usToSeconds). No feature gates;
pure functions.

## 9. Open questions for the next agent
- MediaInfoPanel.tsx: confirm the "Unknown"/null rendering actually reaches
  the DOM for every formatter null path (panel audit's job).
