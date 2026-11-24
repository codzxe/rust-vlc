# AUDIT: ui/src/screens/settings/areas/effects.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Lines / functions | 244 / 2 describes, 9 its |
| File verdict | REAL (executed in the committed baseline: 11/11 green) |
| Reachable from shipped entry point? | NO (vitest unit suite; the component under test IS shipped in the UI) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## 1. One-paragraph verdict
An honest staged-UI test suite: it renders the real EffectsArea through a
fixture transport that RECORDS every command sent, then asserts (a) the frame's
11 faders/8 presets/6 ⓘ affordances exist with the right ARIA semantics,
(b) every control is disabled and carries data-pending-command naming the
awaited engine command (audio.set-eq / audio.set-spatializer / audio.set-stereo-
widener), and (c) after real interaction (preset pick, fader keydown, collapse/
reopen) `transport.sent` stays EMPTY — proving staged rows never send. The mock
is at the transport seam (legitimate), not a mock of the component under test.

## 2. What actually works (verified)
- Baseline (audit/runs/baseline/ui-test.txt:66): `✓ src/screens/settings/areas/
  effects.test.tsx (11 tests) 531ms` — green in the committed run (suite total:
  1705 passed | 1 skipped, line 178-179).
- The asserted markers exist in the component source: effects.tsx header
  documents the staged-command pattern; makeFixtureTransport
  (audio-fixtures.ts:12-21) records `sent` requests — the "never sends"
  assertion has teeth.

## 3. Findings
F-001 [S4] Where: effects.test.tsx:94 (comment) vs the assertion at :95-96.
Code: `// Club = [0, 0, 4.8, 3.6, 3.6, 3.6, 2.4, 0, 0, 0] — the panel chips follow.
 expect(screen.getAllByText("4.8 dB")).toHaveLength(1);
 expect(screen.getAllByText("3.6 dB")).toHaveLength(3);`
What is wrong: the count-based assertion is satisfied by ANY layout containing
one "4.8 dB" and three "3.6 dB" texts (e.g. a wrong 2.4-band rendering plus
coincidental duplicates elsewhere would pass); it does not pin WHICH faders hold
which values, unlike the exact array in the comment. Weak-but-real; the
surrounding tests (preset name, aria) carry the rest. Confidence: HIGH — read.
Also noted: "Engine stage ready" staged-note honesty is asserted in 3 separate
tests (over-coverage, no defect).

## 4. Per-function table (test blocks)
| Block | Lines | Verdict | Notes |
|---|---|---|---|
| renderArea helper | 23-31 | REAL | fixture transport + real component |
| describe "the frame's Effects section" (6 its) | 33-161 | REAL | ARIA structure, preset state-sharing, ⓘ operability incl. reveal/dismiss |
| describe "staged rows are inert WITH REASON" (5 its) | 163-244 | REAL | disabled + pending-command names; `transport.sent` empty proof |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 9 | YES | structural ARIA + state + the sent-log assertion; e.g. wiring a control live would fail "staged controls never send anything" |
## 6. Doc claims: header comments describe the staged design — matches component source (read cross-check).
## 7. Dead code: none.
## 8. Dependencies: @testing-library/react, vitest, ClientProvider, fixture transport.
## 9. Open questions: none.
