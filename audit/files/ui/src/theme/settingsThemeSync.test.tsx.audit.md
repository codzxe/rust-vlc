# AUDIT: ui/src/theme/settingsThemeSync.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A01) |
| Date | 2026-11-24 |
| Lines / functions | 174 / 13 tests + Harness/Sync/mount fixtures |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — vitest baseline ui-test.txt:100 (13 tests, 73ms); seam is the live app's theme bootstrap (ThemeProvider consumes it) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | it-count 13 == baseline; rg-free (subject read directly); sed quote verify |

## 1. One-paragraph verdict
Real wiring tests for the settings→theme sync: bootstrap reads through the REAL persistence
key (dark/light/auto accepted; unknown + corrupt stores → null, "never guesses"), the schema
default landing on <html> as data-theme, live-preview flips for both theme and accent, revert
semantics, rogue-value drop (both fields), and the auto/SYSTEM resolution against a stubbed OS
preference. The Harness writes through the real SettingsProvider (memory persistence) — no
mock of the seam under test. 13/13 pass in the baseline. One hygiene nit and one honest
coverage gap (runtime OS-preference change listener).

## 2. What actually works (verified)
— verified by citation audit/runs/baseline/ui-test.txt:100 `✓ src/theme/settingsThemeSync.test.tsx
(13 tests) 73ms` (1705-passed run); 13 it() blocks on disk match.

## 3. Findings
F-001 [S4]
Where: ui/src/theme/settingsThemeSync.test.tsx:158-172
Code:
    const query = {
      matches: true,
      addEventListener: () => {},
      removeEventListener: () => {},
    };
    vi.stubGlobal("matchMedia", vi.fn().mockReturnValue(query));
What is wrong: the auto-mode test stubs matchMedia with no-op listeners — only the INITIAL
resolution is proven; the runtime OS-preference-change path (ThemeProvider reacting to a
`change` event while mounted) is never driven. Coverage gap, not a broken assertion.
Why it matters: dark/light auto-follow during a session is claimed by the e19-F5 feature but
only half-proven here. What correct looks like: a second stub whose listener is captured and
fired with matches flipped, asserting data-theme follows.
Confidence: HIGH — Verification: read.
No S0-S3: assertions bind to documentElement.dataset state driven through real providers.

## 4. Per-function table
| Test (line) | Verdict | Severity | Notes |
|---|---|---|---|
| reads dark/light (59) | REAL | — | clean store → null first |
| accepts SYSTEM (67) | REAL | — | e19-F5 |
| unknown/corrupt → null (72) | REAL | — | hostile inputs |
| reads generated accents (95) | REAL | — | |
| accent rogue/corrupt → null (103) | REAL | — | |
| default dark on mount (112) | REAL | — | |
| draft flips data-theme (117) | REAL | — | live preview |
| revert flips back (123) | REAL | — | |
| accent stays orange untouched (131) | REAL | — | |
| accent draft flips (138) | REAL | — | w26.f08 r2 |
| rogue accent dropped (144) | REAL | — | never throws |
| non-theme value ignored (152) | REAL | — | |
| SYSTEM/auto forwarded (158) | REAL | S4 | F-001: listener path untested |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 13 | YES | every assertion checks <html> dataset or the reader's return against real localStorage seeds; e.g. if the sync stopped writing data-theme, 117/120 fails |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "attribute-only theming (data-theme on <html>)" | header 13-14 | REAL | tests 112-156 |
| "same contract covers data-accent (global.accent-color)" | header 15 | REAL | tests 131-149 |

## 7. Dead / duplicated / unreachable code
Hygiene nit: the third describe (111-173) has no afterEach resetting dataset.theme/accent (the
first two do); only the last test manually cleans theme. Intra-file leakage is masked because
every mount re-applies; harmless but inconsistent (would bite if a future test asserted absence
of attributes).

## 8. Dependencies & flags
In-repo: ThemeProvider, SettingsProvider, memory persistence, generated schema keys. Stubbing
limited to matchMedia (documented stand-in for Playwright's emulateMedia).

## 9. Open questions for the next agent
Does any Playwright lane drive the OS colorScheme CHANGE while mounted (the other half of
e19-F5)? If not, auto-follow-at-runtime is unit- and integration-unproven.
