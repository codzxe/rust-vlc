# AUDIT: ui/src/client/transport.test.ts
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | 36 / 4 it() tests, 1 describe |
| File verdict | REAL (4/4 pass in baseline; the MockTransport seam's own contract) |
| Reachable from shipped entry point? | YES via test lane — baseline ui-test.txt line 175 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | full file read; baseline cite |

## 1. One-paragraph verdict
Honest small contract test of MockTransport itself: `system.list-commands` answers a typed
EMPTY command listing (a real completed wire shape — the mock's honest "this build serves
nothing" answer), unsimulated commands answer the typed `feature-not-exposed` error with
stage control, `sent` records issued requests exactly, and unsubscribe is idempotent with
no side effects. These four behaviors are exactly what dozens of downstream tests rely on
(the mock is the fallback transport in the app-transport probe — see app-transport.test).

## 2. What actually works (verified)
- Baseline: ui-test.txt line 175 `✓ src/client/transport.test.ts (4 tests) 5ms`.
- Read-verified: `expect(response.result).toEqual({ completed: { kind: "command-listings", commands: [] } });`
  (exact wire shape) and `expect(response.result).toMatchObject({ error: { code: "feature-not-exposed", stage: "control" } });`.

## 3. Findings
F-001 [S4] No test covers the gen/requestId monotonicity of MockTransport's responses
(the FakeTransport in ControlBar.test mocks that behavior itself, so the REAL MockTransport
gen arithmetic is unpinned here). Cosmetic: gen is asserted implicitly in app-transport
tests via send() but not for consecutive sends. Confidence: HIGH — read.

## 4. Per-function table
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| describe MockTransport | 5-36 | REAL | 4 tests: listing shape, typed refusal, sent-recording, subscribe idempotence |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| answers system.list-commands | YES | exact completed-shape equality |
| typed feature-not-exposed | YES | toMatchObject on code+stage |
| records issued requests | YES | exact cmd array |
| subscribe/unsubscribe without side effects | YES | empty observed array |

## 6-9. Doc claims / dead code / deps / open questions
Baseline count 4 == 4. No dead code. Vitest only. None.
