# AUDIT: ui/src/screens/settings/areas/recording.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A10 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / tests | 135 / 6 (2 describes) |
| File verdict | REAL (baseline-verified green; feature-gate + staged-seam contracts asserted structurally) |
| Reachable from shipped entry point? | YES — recording.tsx is the /settings/recording area screen |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg baseline ui-test.txt:111 |

## 1. Verdict
The recording-area contract: routing + aria-current, the generated-screen embedding made
structural (embedded root without testid inside the area wrapper), schema truth (exactly two
gated rows, both featureGate "recording"), the F2 gate hiding both rows while 10 staged rows
render instead of an empty note, staged rows frame-verbatim with tooltips and zero live
bindings, and the 4/10 ⓘ census operable. Baseline green: ui-test.txt:111 `✓ …(6 tests) 316ms`.

## 2. What actually works (verified)
- Baseline 6/6 (cite above).
- Mutation reasoning: gate leak fails :74-76 (data-setting selectors must find nothing);
  wrong staged count fails :80; live binding sneaking in fails :111; census drift fails
  :119-126 (exact set of 4 row ids); ⓘ not operable fails :127-133 (expand + collapse);
  embedding structural break fails :55-59.
- The schema-truth test pins generated data (keys array equality + per-entry featureGate).

## 3. Findings
None. Zero-findings list: all 6 tests read line-by-line (no tautologies; every assert is DOM
or schema data); mocks only at transport seam (MockTransport); SettingsProvider real; staged
grammar asserted via markers + tooltip + dimmed set; no invented options asserted implicitly
by the zero-[data-setting] rule; frame-verbatim values pinned (~/Videos, Automatic/Medium/None).

## 4. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| routed + aria-current | YES | area root + nav state |
| embeds generated screen | YES | structural containment query |
| schema truth | YES | exact key list + featureGate each |
| gate hidden + inert | YES | nulls for gated keys, 10 staged |
| staged frame rows | YES | labels + disabled + tooltips |
| ⓘ census | YES | exact 4-row set + operable |

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "rows hidden + inert while `recording` is off" | :3-4 | REAL | gated-rows test |
| "honest-surface rule for schema-less rows" | :5 | REAL | staged-inert asserts |

## 6. Dead / duplicated / unreachable code
None.

## 7. Dependencies & flags
No component mocks; generated schema/screens imported and pinned.

## 8. Open questions for the next agent
None blocking.
