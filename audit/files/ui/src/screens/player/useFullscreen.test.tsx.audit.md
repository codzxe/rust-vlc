# AUDIT: ui/src/screens/player/useFullscreen.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | 118 / 5 it() tests, 1 describe, 4 helpers |
| File verdict | REAL (5/5 pass in baseline; never-optimistic state sync pinned) |
| Reachable from shipped entry point? | YES via test lane — baseline ui-test.txt line 143 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | full file read; baseline cite |

## 1. One-paragraph verdict
Honest hook test with the exact right property pinned: fullscreen state syncs ONLY from
`fullscreenchange` events, never from request/exit return values. The strongest test asserts
`requestFullscreen` was called AND `data-fullscreen` stays "false" until the event lands;
`exit()` keeps "true" until the event clears it. Typed `FullscreenUnsupportedError` rejections
asserted on a bare host. Mock discipline is correct (configurable defineProperty, restored in
beforeEach/afterEach).

## 2. What actually works (verified)
- Baseline: ui-test.txt line 143 `✓ src/screens/player/useFullscreen.test.tsx (5 tests) 50ms`.
- Read-verified anti-optimism: `fireEvent.click(screen.getByTestId("request"));`
  `expect(target.requestFullscreen).toHaveBeenCalledTimes(1);`
  `expect(statusOf("data-fullscreen")).toBe("false");` (state waits for the event).
- Typed refusal: `await expect(result.current.request()).rejects.toBeInstanceOf(FullscreenUnsupportedError);`.

## 3. Findings
F-001 [S4] line 45 `return el!.getAttribute(attr)!;` — `!` assertions again (cosmetic).
Everything else checked clean: no timers faked (none needed), no snapshot tests, mocks
scoped and restored.

## 4. Per-function table
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| Host | 13-25 | REAL | full hook surface exposed via test ids |
| setFullscreenElement / statusOf | 27-46 | REAL | event-driven state driver |
| beforeEach/afterEach | 35-53 | REAL | mock install/restore symmetry |
| 5 tests | 56-118 | REAL | unsupported + event-sync + toggle + typed errors |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| requests fullscreen; state syncs only from event | YES | call-count 1 + state "false" until event |
| exit() ... syncs only on the event | YES | state "true" after exit call |
| rejects with FullscreenUnsupportedError | YES | rejects-to-instance assertions |

## 6. Doc claims
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "asserted against the EVENTS, never the request return values" (header line 4) | file itself | REAL | the two event-sync tests |
| 5 tests pass | baseline ui-test.txt:143 | REAL | vitest line |

## 7-9. Dead code / deps / open questions
None / vitest+RTL only / none.
