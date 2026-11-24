# AUDIT: ui/src/screens/player/useChromeAutoHide.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 405 / 16 tests across 5 describes + ScriptedTransport + 2 helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test-infra for shipped hook (ui/src/screens/player/useChromeAutoHide.ts used by the shipped PlayerScreen); runs under vitest |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -n "useChromeAutoHide" audit/runs/baseline/ui-test.txt` → `47: ✓ src/screens/player/useChromeAutoHide.test.tsx (16 tests) 76ms`; full file read |

## 1. One-paragraph verdict
A rigorous state-machine test file: exact timer-boundary assertions (visible at delay−1, hidden at +1), reveal-on-activity with restart semantics, locked-then-re-arm transitions, engine event transitions through a scripted transport, DOM-truth overlay lock (menu/dialog presence in document), a render-count performance contract pinning zero host re-renders per pointer event, and referential stability of the returned object. Fake timers bracketed before/afterEach correctly. No mocks of the subject; ScriptedTransport mocks only the backend transport seam and its rejection path is documented as exercising the provider's try/catch.

## 2. What actually works (verified)
- Committed run: baseline ui-test.txt L47 `✓ ... useChromeAutoHide.test.tsx (16 tests) 76ms` (inside 1705 passed).
- Boundary discipline: L48-53 (delay−1 visible → +1 hidden), L61-67 (reveal restarts idle), L127-133 (show() restarts), L383-391 (mid-burst restart semantics).
- Performance contract: L348-375 — 25-event pointermove burst asserts `hostRenders === baseline` (the J06-1 regression pin), then re-verifies countdown restart via DOM textContent.
- Referential stability: L394-403 `expect(result.current).toBe(first)` after forced rerender.
- Not re-run here (no npm); baseline is the execution evidence.

## 3. Findings
None. Zero-findings checklist: tautology scan — every expect binds to hook state, DOM text, event payloads, or render counts produced by the real hook; no self-comparisons. Mock-of-subject scan — no vi.mock anywhere; ScriptedTransport (L142-156) implements the transport interface only, and its send() rejection is intentional + documented (L140-141). Skip scan — no it.skip/describe.skip; baseline shows 16 passed 0 failed for this file. Name-vs-body scan — each name states the exact transition asserted. Timer hygiene — vi.useFakeTimers/vi.useRealTimers bracketed (L22-28). Mutation reasoning: breaking the timer restart breaks L61-68; breaking the overlay lock breaks L261-311; breaking render-free activity breaks L348-375.

## 4. Per-function table (>100 LOC, test-file form)
| Test group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| legacy standalone machine | 30-136 | REAL | — | disabled/enabled gates, hide/reveal, keydown reveal, locked→re-arm, disable→visible, show() restart |
| player composition | 169-248 | REAL | — | windowed fade while playing, empty-state fade (retired no-media gate), panel lock, engine-idle re-arm |
| open-overlay lock | 252-327 | REAL | — | menu/dialog DOM truth: reveal+lock in one transition, re-arm on remove |
| render-cost contract | 331-404 | REAL | — | zero renders per pointer storm, restart semantics mid-burst, referential stability |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 16 | YES | exact boundary/timer/DOM/render-count assertions on the real hook; verified green baseline L47 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "commands reject (the provider's probes are try/caught — only the event stream matters here)" | comment L140-141 | REAL-UNPROVEN | PlayerControlsProvider impl not my file; consistent with tests passing |
| "the ref-held scheduler must keep the i02 restart semantics with ZERO renders per event" | comment L332-335 | REAL | asserted L355-360 (hostRenders === baseline over 25 events) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
@testing-library/react (renderHook/act/render), vitest fake timers, ClientProvider/PlayerControlsProvider seams, real hook under test.

## 9. Open questions for the next agent
- None.
