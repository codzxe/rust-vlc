# AUDIT: ui/src/components/menu/Dropdown.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / tests | 284 / 18 |
| File verdict | REAL — dense a11y + keyboard + regression suite; one fragile shared-mock pattern |
| Reachable from shipped entry point? | NO — vitest; baseline ui-test.txt:60 `✓ src/components/menu/Dropdown.test.tsx (18 tests) 348ms` (vitest 5.0.5, `npm run test` log header L1-6) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | attempted local run `node_modules/.bin/vitest run …` → node_modules ABSENT in this sandbox (npm ci forbidden by spec) → UNVERIFIED locally, baseline cited |

## 1. One-paragraph verdict
An honest, deep component test: ARIA wiring (aria-haspopup/expanded/controls/
describedby value exposure), roving tabindex with disabled-skip, Home/End,
Enter/Escape/Tab semantics including the g19 host-dialog scoping regression
(stopPropagation verified by a host onKeyDown spy), typeahead, disabled-click
refusal, outside-pointerpress dismiss, density classes, decorative icons, and
a preferredDirection trio with getBoundingClientRect stubbed per case (the
jsdom-all-zero-rects limitation is called out in-file at L237-239). The one
structural defect: `onChange` is a MODULE-LEVEL shared vi.fn() (L20) never
reset in-file, so assertions like `expect(onChange).not.toHaveBeenCalled()`
(L184) depend on vitest's hidden mock lifecycle to stay meaningful — an
earlier test in the same file (L122) demonstrably fires it.

## 2. What actually works (verified)
— Baseline: ui-test.txt:60 — 18 tests, 348 ms, green on vitest 5.0.5 (cmd
`npm run test` → `vitest run`, log L1-6).
— Mutation reasoning: the g19 regression test (L137-157) fails if
stopPropagation regresses (hostKeyDown spy); the disabled-skip arrow test
(L92-102) fails if roving focus lands on a disabled option; the
aria-describedby pair (L44-62) fails if the SR value exposure regresses.
All 18 assert exact DOM/behavior.
— The rect-stub trio (L240-283) is explicit about jsdom limits and asserts
data-direction in all three branches (preference wins / yields / default).

## 3. Findings
F-001 [S4]
Where: ui/src/components/menu/Dropdown.test.tsx:20 and 184
Code:
const onChange = vi.fn();
…
    fireEvent.click(screen.getByRole("option", { name: "Français" }));
    expect(onChange).not.toHaveBeenCalled();
What is wrong: the mock is shared module-level across all 18 tests and never
cleared in this file, while an earlier test (L122 `expect(onChange).
toHaveBeenCalledWith("none")`) fires it; `not.toHaveBeenCalled()` at L184 is
therefore only meaningful if the runner clears mock state between tests.
The baseline (vitest 5.0.5) shows the file green, so the lifecycle holds
there — but the assertion's validity is runner-implementation-dependent
(order + clear semantics), not test-intrinsic. Contrast: this repo's own
AddSubtitlesDialog.test.tsx creates `const onClose = vi.fn()` per test
(L61) — the robust pattern. Why it matters: an upgrade/config change flips
this red spuriously, or masks a spurious earlier call. What correct looks
like: `onChange.mockClear()` in a beforeEach, or per-test vi.fn().
Confidence: HIGH on the pattern, MEDIUM on the consequence — Verification:
read + baseline run cited; local repro UNVERIFIED (node_modules absent,
npm ci forbidden — reason: sandbox budget).

F-002 [S4]
Where: :243-247 `vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({...} as DOMRect)` — cast-to-DOMRect escape hatch (hygiene; the stub omits nothing behaviorally relevant here).
Confidence: HIGH — Verification: read.

## 4. Per-function table (describe/it groups)
| Group | Lines | Verdict | Notes |
|---|---|---|---|
| trigger/ARIA basics | 34-72 | REAL | describedby value exposure pinned incl. rerender |
| listbox open/selected | 74-90 | REAL | aria-selected, aria-disabled, focus-on-open |
| keyboard nav | 92-135 | REAL | arrows skip disabled; Home/End; Enter/Escape full contracts |
| g19 host-scoping regression | 137-157 | REAL | stopPropagation + Tab restore |
| typeahead | 159-176 | REAL | prefix jump |
| click select/disabled | 178-188 | REAL | see F-001 (shared mock) |
| outside press | 190-197 | REAL | pointerDown on body dismisses |
| density classes | 199-216 | REAL | comfortable/compact |
| icons | 218-235 | REAL | decorative slot, aria-hidden |
| preferredDirection | 240-283 | REAL | 3-branch with explicit rect stubs |

## 5. Test honesty
17 of 18 would fail on their named regression outright; 1 (disabled-click,
L178-188) is only as honest as the shared-mock lifecycle allows (F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "jsdom rects are all-zero, so the trigger's box is stubbed per case" | file L237-239 | REAL | rectSpy + explicit viewport math in comments |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
RTL + vitest; setup.ts afterEach(cleanup) (globals: false); no network;
no config-level mock clearing (vite.config.ts test block L36-43 verified —
no clearMocks/restoreMocks keys).

## 9. Open questions for the next agent
- Confirm on vitest 5.0.5 whether module-level vi.fn() is auto-cleared per
  test (would upgrade F-001 to a latent-red hazard if not — it is green in
  the baseline, so empirically it is).
