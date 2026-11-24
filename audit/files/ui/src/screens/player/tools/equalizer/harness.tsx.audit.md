# AUDIT: ui/src/screens/player/tools/equalizer/harness.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 43 / 2 (EqualizerCardStates, EqualizerPresetsHarness) |
| File verdict | REAL (dev-only vision harness) |
| Reachable from shipped entry point? | NO — test-only per the w26.b10 pattern (header; harness.test.tsx consumer) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read; pattern verified in options/harness.tsx audit (same lane pattern) |

## 1. One-paragraph verdict
Minimal dev-only composition harness for the EQ frames (collapsed + expanded card states, presets-open state driven via public API in the test). No stubs, no data fixtures — it composes the real (inert, staged) EqualizerTool. Zero findings.

## 2. What actually works (verified)
— verified by import pattern (harness.test.tsx only; same convention as options/harness.tsx).

## 3. Findings
None.

## 4. Per-function table
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| EqualizerCardStates | 23-30 | REAL (dev) | both frame states |
| EqualizerPresetsHarness | 37-43 | REAL (dev) | presets state via public API |

## 5. Test honesty
Visual-loop artifact (screenshot comparison), not behavioral.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "DEV-ONLY vision harness (never imported by app routes)" (L1) | header | REAL | test-only import chain |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Real EqualizerTool. No flags.

## 9. Open questions for the next agent
- None.
