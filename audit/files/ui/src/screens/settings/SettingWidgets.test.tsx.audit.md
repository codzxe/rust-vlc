# AUDIT: ui/src/screens/settings/SettingWidgets.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A09 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / tests | 93 / 3 (1 describe) |
| File verdict | REAL (baseline-verified green; roving-tabindex + wrap-around keyboard grammar fully asserted) |
| Reachable from shipped entry point? | YES — StartupVolumeField renders the audio area's startup-volume rows (composed in audio area) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg baseline ui-test.txt:142 |

## 1. Verdict
Small but dense: the Values-1 radiogroup contract — two radios with roving tabindex (only
the checked one tabbable), all four arrows with wrap-around in BOTH directions, mode commits
through the real settings store (ModeProbe reads useSettings), pointer path, and nested
input inertness/liveness. Baseline green: ui-test.txt:142 `✓ …(3 tests) 147ms`.

## 2. What actually works (verified)
- Baseline 3/3 (cite above).
- Mutation reasoning: roving tabindex regression fails :43-46/:64-65; wrap-around
  one-directional fails :68-79 (the exact past-bug shape the header names); store commit
  missing fails :58-79 (latestMode sequence); nested input staying disabled fails :90.

## 3. Findings
None. Zero-findings list: 3 tests read line-by-line (no tautologies — asserts are ARIA state
and store values); the module-level `latestMode` probe is a render-observable read, not a
component mock; SettingsProvider is real; only the single widget family under test is
exercised (file scope is honestly narrow per header "the only shell-side gap the review carried").

## 4. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| roving tabindex | YES | tabindex 0/-1 pairs + aria-checked |
| arrows + wrap | YES | 5 key events, exact mode sequence |
| click selection | YES | checked + nested input enabled |

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "k01's shared roving keyboard grammar (four arrows, wrap-around)" | :2-5 | REAL | both-direction wrap asserted |

## 6. Dead / duplicated / unreachable code
None.

## 7. Dependencies & flags
No mocks of the component; generated schema constants; real provider.

## 8. Open questions for the next agent
None blocking.
