# AUDIT: ui/src/screens/player/dialogs/GotoTimeDialog.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Lines / tests | 211 / 10 `it` blocks |
| File verdict | REAL — executed, 10/10 passing in baseline |
| Reachable from shipped entry point? | N/A — vitest suite (baseline ui-test.txt:91) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; baseline citation |

## 1. One-paragraph verdict
Real seam test for the seek funnel. Pins the wire unit (90 s →
`{ target: { kind: "absolute", time: 90_000_000 } }` µs; 1:02:05 →
3 725 000 000 µs), mount-time prefill via an on-demand host (the
PlayerScreen pattern) after a `timing.time-tick` event (90 s → "1:30"),
honest unknown duration ("--:--", no fabricated ceiling — a 9999-second
time still seeks, 182-195), duration-bounded refusal with the truth in the
error (167-180), unparseable input refused without a seek (156-165), and
focus/Escape/close-quiet semantics. Mock is the transport only.

## 2. What actually works (verified)
- Baseline: audit/runs/baseline/ui-test.txt:91 (verbatim):
```
 ✓ src/screens/player/dialogs/GotoTimeDialog.test.tsx (10 tests) 253ms
```
- Wire-unit pin (133-138):
```
    const seek = transport.sent.find((r) => r.cmd === "playback.seek");
    // w26.k17: the seam stays seconds; the wire target crosses at the
    // send edge — 90 s = 90 000 000 µs.
    expect(seek?.args).toEqual({
      target: { kind: "absolute", time: 90_000_000 },
    });
```
- Honest-unknown pin (96-97): `expect(screen.getByText(/0:00\s*\/\s*--:--/)).toBeInTheDocument();` — "never a fabricated ceiling".
- Negative seek-count assertions on every refusal path (163, 176, 209).

## 3. Findings
None. Zero-findings justification: exact payload equalities at µs
magnitude; honest-error paths asserted with zero-sent seek counts and
no-close assertions; the on-demand host (64-73) correctly avoids the
mount-order trap the authors document.

## 4. Per-function table
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| RecordingTransport | 22-46 | REAL | records sent; emits engine events |
| GotoOpenOnDemand host | 64-73 | REAL | seeds seam truth before mount-time prefill |
| focus test | 76-86 | REAL | input-first focus |
| prefill tests | 88-122 | REAL | 0:00 default; 1:30 from 90 s event; inputmode text |
| submit tests | 124-154 | REAL | µs absolute pins via form + button |
| refusal tests | 156-180 | REAL | banana → invalid; 700 > 600 s → beyondEnd("10:00") |
| unknown-duration test | 182-195 | REAL | 9999 s still seeks (9 999 000 000 µs) |
| Escape / quiet close | 197-210 | REAL | onClose("escape"); close without seek |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| All 10 | YES | exact µs payloads; zero-seek counts on refusals; prefill values |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Honest unknown duration renders '--:--', never a fabricated ceiling" | header :6-7 | REAL (tested) | tests 96-97, 182-195 |

## 7. Dead / duplicated code
None.

## 8. Dependencies & flags
@testing-library/react, vitest, PlayerControlsProvider. No gates.

## 9. Open questions
None.
