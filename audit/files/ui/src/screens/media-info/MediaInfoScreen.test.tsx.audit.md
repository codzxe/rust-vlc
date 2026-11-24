# AUDIT: ui/src/screens/media-info/MediaInfoScreen.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / tests | 150 / 4 |
| File verdict | REAL — route body + empty state + a µs-wire duration regression pinned on the real DemoTransport |
| Reachable from shipped entry point? | NO — vitest; baseline ui-test.txt:115 `✓ …MediaInfoScreen.test.tsx (4 tests) 193ms` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg baseline |

## 1. One-paragraph verdict
Four honest tests: the fixture-driven populated body (heading + sections),
the honest empty state (not-found error → empty card on the r14 Card grammar),
the empty-state CTA firing the REAL shell-wide open-media event (captured via
window listener, not a mock), and the j17 P1-1 regression — the real
DemoTransport (not a mock) opens the bunny clip through session.open under
fake timers, advances the opening choreography 400 ms, mounts the screen
late, and asserts the rendered duration is 10:35 from the 635 000 000 µs
wire value (the old seconds/ms confusion rendered "0:00"). The regression
test documents its own failing-condition in-file (L114-115).

## 2. What actually works (verified)
— Baseline ui-test.txt:115: 4 tests green in 193 ms.
— Mutation reasoning: duration-unit regression → L142-144 fails (renders
"Duration0:00"); event seam regression → L105 fails (seen == []); empty-state
grammar regression → L73-76 fails.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/media-info/MediaInfoScreen.test.tsx:142-144
Code:
      expect(container.querySelector('[data-field="duration"]')?.textContent).toBe(
        "Duration10:35",
      );
What is wrong: asserts glued `textContent` ("Duration" + value with no
separator) — pins markup shape, not the value alone; a wrapper span breaks
it without behavior change. Same pattern as AddSubtitlesDialog F-001.
Confidence: HIGH — Verification: read.
F-002 [S4]
Where: :54-65 vs :80-91 — the identical inline `nothingOpen` transport object is defined twice verbatim (duplication).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| FixtureTransport | 20-32 | REAL | serves the fullMediaInfo fixture |
| populated body test | 43-51 | REAL | heading + sections + data-screen hook |
| empty-state tests | 53-106 | REAL | honest empty card; real event seam captured |
| DemoTransport regression | 109-150 | REAL | µs wire contract; fake timers; dispose in finally |

## 5. Test honesty
All 4 fail on their named regression (§2). The DemoTransport test uses the
REAL transport class — the unit contract (µs) is exercised end-to-end into
the formatter.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "This test FAILS on the old code at the duration assertion" | file L114-115 | REAL-UNPROVEN | mutation reasoning read; old code not present to run (UNVERIFIED: cannot run vitest — node_modules absent) |

## 7. Dead / duplicated / unreachable code
Duplicated inline transport (F-002); otherwise none.

## 8. Dependencies & flags
RTL + vitest fake timers; real ClientProvider/MediaInfoScreen/DemoTransport;
fixtures module; transport.dispose() in finally (hygiene).

## 9. Open questions for the next agent
- None.
