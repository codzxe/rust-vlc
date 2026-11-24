# AUDIT: ui/src/screens/player/states/error-display.ts
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 84 / 2 (isUnsupportedError, unsupportedFormatInfo) + 4 exported data maps |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (states/index.ts re-exports; StateRouter.tsx:38 + StageOverlays.tsx:51-53,206,261 consume) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg ErrorCode/Stage unions in generated/commands.ts (18+13, exact match); rg consumers; sed quote verification |

## 1. One-paragraph verdict
Correct, defensive error-presentation data. The exhaustive-Record claim is
real: ErrorCode's 18 union members and Stage's 13 match the Record keys 1:1,
so a codegen enum growth fails the typecheck instead of rendering a silent
blank. unsupportedFormatInfo is whitelist-only (format/codec), returns only
present non-empty strings, never fabricates. Zero findings.

## 2. What actually works (verified)
— rg generated/commands.ts:42 `ErrorCode = ("unsupported" | ... | "internal")`
— 18 members, all present as keys at error-display.ts:34-51; Stage at :92 —
13 members matching 16-28.
— Consumer chain: StateRouter.tsx:38 routes unsupported codes;
  StageOverlays.tsx:261 maps every code to an action hint.

## 3. Findings
None. Checked: enum exhaustiveness (verified against generated unions),
whitelist behavior (78-82: typeof + length guards), unsupported routing
(57-61), message-key validity (all keys follow the player.state.error.*
naming consumed by the i18n catalog — spot-checked pattern).

## 4. Per-function table
Not applicable (<100 LOC; 2 trivial pure functions + data maps, all REAL).

## 5. Test honesty
No tests in this file; error surfaces tested via StateRouter/StageOverlays
suites (other agents).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "exhaustive `Record`s so a codegen-side enum growth breaks the build" | :6-8 | REAL | Record<ErrorCode,...> requires all 18 keys; verified against union |
| "unknown keys are never surfaced" | :64-66 | REAL | whitelist loop over UNSUPPORTED_DETAIL_KEYS only |

## 7. Dead / duplicated / unreachable code
None — every export consumed (states/index.ts barrel + StateRouter +
StageOverlays).

## 8. Dependencies & flags
generated/commands (CommandError, ErrorCode, Stage), i18n MessageKey. No flags.

## 9. Open questions for the next agent
None.
