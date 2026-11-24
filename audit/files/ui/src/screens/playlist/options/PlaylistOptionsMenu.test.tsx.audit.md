# AUDIT: ui/src/screens/playlist/options/PlaylistOptionsMenu.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 296 / 19 tests across 5 describes + 3 helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test-infra for shipped component (ui/src/screens/playlist/options/PlaylistOptionsMenu.tsx); runs under vitest |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -n "PlaylistOptionsMenu.test" audit/runs/baseline/ui-test.txt` → `68: ✓ src/screens/playlist/options/PlaylistOptionsMenu.test.tsx (19 tests) 341ms`; full file read |

## 1. One-paragraph verdict
A complete APG menu-model test file: radio semantics with single aria-checked, click-select-dismiss-focus-restore, disabled inertness, outside-pointer dismiss, arrows cycling with disabled-skip + wrap, Home/End, Enter/Space activation, Escape restore, Tab trap, trigger toggle, align/placement classes, and a measurement suite that stubs trigger rects and window heights to pin the flip/cap arithmetic with EXACT expected values ("288px", "above" vs "below"), including live-token measurement and resize re-measure. Geometry stubs are documented jsdom workarounds with restoreAllMocks hygiene.

## 2. What actually works (verified)
- Committed run: baseline ui-test.txt L68 `✓ ... (19 tests) 341ms` (inside 1705 passed).
- Math verified by reading: L221-241 — 16 rows × 48 + 8 = 776 natural; viewport 600, trigger y300 → below 248 / above 288, cap 288px asserted exactly; L210-219 flip case arithmetic stated and asserted; L264-274 resize re-measure flips class under an open menu.
- Mock hygiene: `afterEach(() => { vi.restoreAllMocks(); window.innerHeight = 768; })` (L205-208).

## 3. Findings
None. Zero-findings checklist: tautology scan — every expect binds to roles/attributes/classes/style/focus/onSelect payloads of the real component; no self-comparisons. Mock-of-subject scan — no vi.mock of the component; spies only stub getBoundingClientRect/getComputedStyle (jsdom limitation, documented L189-190 and L244-247) and are restored. Skip scan — no it.skip; baseline shows 19 passed. Name-vs-body scan — each name matches the asserted behavior (verified per test). Mutation reasoning: breaking disabled-skip fails L112-119; breaking the cap arithmetic fails L241 exact "288px"; breaking resize re-measure fails L272-273; breaking focus restore fails L82/L145.

## 4. Per-function table (>100 LOC, test-file form)
| Test group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| rendering | 43-72 | REAL | — | trigger semantics, popup rows/icons, single checked |
| selection | 74-103 | REAL | — | select+dismiss+restore, disabled inert, outside-dismiss |
| keyboard model (APG) | 105-164 | REAL | — | arrows/Home/End/Enter/Escape/Tab trap/toggle |
| anchor classes | 166-186 | REAL | — | end/above class flip |
| open-direction measurement | 188-295 | REAL | — | flip/cap math with exact px, live tokens, resize, zero-height guard |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 19 | YES | exact role/attribute/style/focus/ordering assertions; verified green baseline L68 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the shared measure() only reads the TRIGGER's rect" | comment L189-190 | REAL-UNPROVEN | measure() impl not my file; consistent with stubbed-rect tests passing |
| "jsdom's getComputedStyle returns '' for stylesheet custom properties" | comment L245-246 | REAL-UNPROVEN | standard jsdom behavior; consistent with the live-token stub test passing |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
@testing-library/react, vitest; real PlaylistOptionsMenu; resolveLabel injected as identity (documented simplification — bypasses i18n, subject untouched).

## 9. Open questions for the next agent
- None.
