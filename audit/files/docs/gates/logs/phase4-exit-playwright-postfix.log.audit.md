# AUDIT: docs/gates/logs/phase4-exit-playwright-postfix.log
| Field | Value |
|---|---|
| Auditor | W17-A05 |
| Date | 2026-11-24 |
| Lines / functions | 50 / n.a. (Playwright run: 38 tests, 1 worker) |
| File verdict | REAL (data audit: genuine Playwright transcript, internally consistent, matches citing doc exactly) |
| Reachable from shipped entry point? | n.a. (log artifact; the suite it records is ui e2e — a11y/smoke/theme specs, still present in the tree) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | grep citation (phase4-exit.md:57, :210); ✓-count vs summary (38/38); ls of the three spec files; grep test-body strings in current specs |

## 1. One-paragraph verdict
A genuine Playwright list-reporter transcript: 38 tests, all ✓, "38 passed (40.2s)", with real environment noise (Vite configLoader warning, NO_COLOR/FORCE_COLOR warning) that hand-written logs never contain. Its citing doc's arithmetic is exact: smoke 1/1 + a11y 16/16 + theme 21/21 = 38, matching the log's test numbering (#17 smoke, #1-16 a11y, #18-38 theme), and the "(40.2 s)" figure matches the log's summary. The recorded suite still exists in the tree; the spec line anchors have drifted since capture (file evolved), which is normal historical-log behavior.

## 2. What actually works (verified)
— verified by counting: log lists ✓ 1..38 with per-test durations; summary "  38 passed (40.2s)" — count and duration internally consistent.
— verified by citation cross-check: docs/gates/phase4-exit.md:57 "Re-run: **38 passed (40.2 s), exit 0** — smoke 1/1, a11y 16/16, theme 21/21" and :210 "smoke+a11y+theme post-fix: 38/38" — every number reproduces from the log.
— verified by ls: ui/e2e/a11y.spec.ts, smoke.spec.ts, theme.spec.ts all exist in the current tree; test-body strings ("axe: zero wcag", "resolves the accent cascade") present (loop-generated test names, one grep hit each as expected for looped specs).

## 3. Findings
F-001 [S4]
Where: docs/gates/logs/phase4-exit-playwright-postfix.log (whole file; no header)
Code:
```
[WebServer] (!) Your Vite config uses features that are unsupported by `configLoader: 'native'`, ...
Running 38 tests using 1 worker
```
What is wrong: no capture header (worktree/date/commit/exit marker) — same convention gap as the other phase-4-era logs; provenance only inferable from content. The exit code "exit 0" exists only in the citing doc, not in the log. Confidence: HIGH — Verification: read.
(No other findings: 38/38 pass with honest breakdown; the citing doc's "smoke fix (this session)" note describes a real spec edit — the onboarding-state seeding visible today in ui/e2e/smoke.spec.ts's seeding approach.)

## 4. Per-function table
Not applicable (data/log). Test groups verified: a11y 16 (11 axe wcag2a/2aa + 4 keyboard + 1 focus-ring token), smoke 1, theme 21 (default/auto/switch + 16 accent-matrix).

## 5. Test honesty
The log IS test output. The a11y tests assert zero violations (would fail on real a11y regressions); theme matrix asserts cascade resolution per accent×mode. Would these fail if broken? Yes (axe violations / cascade mismatch assert). No tautologies visible in names; the current spec files carry the same generators.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "38 passed (40.2 s), exit 0 — smoke 1/1, a11y 16/16, theme 21/21" | phase4-exit.md:57 | REAL | log summary + numbering (#1-16 a11y, #17 smoke, #18-38 theme) |
| "smoke+a11y+theme post-fix: 38/38" | phase4-exit.md:210 | REAL | log summary line |

## 7. Dead / duplicated / unreachable code
n.a.

## 8. Dependencies & flags
Playwright (1 worker — sandbox-honest), Vite dev server as [WebServer]; NO_COLOR/FORCE_COLOR conflict warning recorded — plausible env noise, not sanitized.

## 9. Open questions for the next agent
- Was this run's browser-load equivalent re-verified in the current era? Baseline rc-smoke says browser-load PASS (ui build + load) but a fresh playwright e2e run against the current tree is not in the baseline — the 38/38 claim is phase-4-era only. UNVERIFIED for current tree.
