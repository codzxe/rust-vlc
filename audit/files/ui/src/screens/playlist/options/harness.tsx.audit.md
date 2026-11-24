# AUDIT: ui/src/screens/playlist/options/harness.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 135 / 5 (HARNESS_ENTRIES, HARNESS_META, harnessTransport, 3 harness components) |
| File verdict | REAL (dev-only tool; honestly stubbed transport) |
| Reachable from shipped entry point? | NO — imported only by harness.test.tsx (rg: 2 files, the harness + its test); header: "DEV-ONLY vision harness (never imported by app routes)" |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -ln "options/harness|OptionsMenusHarness"` (test-only consumers); `rg -n "initialEntries" PlaylistProvider.tsx` (L86, L90 — fixture seam) |

## 1. One-paragraph verdict
Legitimate dev-only composition harness for the Figma vision loop: four fixture entries + watched-meta pinned per frame, a transport stub that answers a canned state-snapshot (explicitly commented "the harness never talks to an engine"), wrapped in the real providers so the screenshot loop sees production rendering. Not on any app route. Zero findings — fixtures are correctly scoped to the harness/test layer, and the i18n-literal rule is disabled inline per caption with the frame-name justification.

## 2. What actually works (verified)
— verified by `rg`: harness imported only by its test; `initialEntries` is a documented test/fixture seam of PlaylistProvider (L86-90), not a production data path.

## 3. Findings
None. Checked: stub never fabricates engine behavior beyond a minimal snapshot; HARNESS_* fixtures used only in the harness + test; eslint-disable comments scoped per-line with reasons.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| HARNESS_ENTRIES / HARNESS_META | 26-40 | REAL (fixtures) | — | frame-mirroring, harness-scoped |
| harnessTransport | 43-50 | STUB (by design, disclosed) | — | "the harness never talks to an engine" |
| OptionsMenusHarness | 53-73 | REAL (dev tool) | — | state 1+2 |
| OptionsViewsHarness | 76-109 | REAL (dev tool) | — | state 3+4 |
| OptionsPopupCloseups | 114-135 | REAL (dev tool) | — | permanently-open popups via public API |

## 5. Test honesty
harness.test.tsx renders these into static HTML for screenshots (per header) — the harness's purpose is visual comparison, not behavioral assertion. No product claim made.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "DEV-ONLY vision harness (never imported by app routes)" (L1) | header | REAL | rg consumer sweep: test-only |

## 7. Dead / duplicated / unreachable code
None (dev tool by definition).

## 8. Dependencies & flags
Real providers + components under test; stub transport. No flags.

## 9. Open questions for the next agent
- None.
