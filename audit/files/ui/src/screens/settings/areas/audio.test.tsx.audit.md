# AUDIT: ui/src/screens/settings/areas/audio.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A03 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / tests | 230 / 11 (3 describes) |
| File verdict | REAL (baseline-verified green; the "config-never-invents" rule is genuinely tested) |
| Reachable from shipped entry point? | YES — audio.tsx (282 LOC) is the /settings/audio area screen; test composes ClientProvider+SettingsProvider like production |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg baseline ui-test.txt:62; wc source files |

## 1. Verdict
Behavioral suite for the audio settings area with the honest-capability contract under test:
device list renders ONLY from engine enumeration; absent capability renders the trigger
visible+disabled+with-reason (never a hidden stub); failed set-device reverts the selection;
empty enumeration yields no invented devices. Baseline green: ui-test.txt:62
`✓ …areas/audio.test.tsx (11 tests) 445ms`.

## 2. What actually works (verified)
- Baseline 11/11 pass (cite above).
- Mutation reasoning: enumeration-ignoring UI fails (:64-77 — asserts audio.list-devices was
  sent AND options render from the fixture list); optimistic set-device UI fails the revert
  test (:103-115); capability lie fails the note/trigger-disabled asserts (:119-178);
  frame-order regression fails the label-array equality (:195-199); schema-key unbinding
  fails the per-entry loop (:185-191).
- Mocks are at the transport seam only (makeFixtureTransport + MockTransport); the component
  tree is real, wire command names are asserted verbatim (audio.set-device args.id = the
  engine-reported "wasapi:hdmi", :96-97).
- The "reason renders twice by design" comment (:122-124) forces the role-based query — the
  test refuses bare-text matching that would pass on the duplicated string alone.

## 3. Findings
None. What was checked and how: all 11 tests read line-by-line for tautologies (none — every
assert is DOM state or dispatched-command evidence); fixture helper audio-fixtures.ts (27
LOC) inspected (a scripted transport recorder, not a component mock); source file sizes;
baseline run line; error-path coverage (5 distinct failure modes); inertness proof
(:148-154 asserts exactly one command sent and zero set-device).

## 4. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| enumeration query+render | YES | sent-commands + option roles |
| default pre-selection | YES | trigger text equality |
| set-device dispatch | YES | exact id in wire payload |
| failed set reverts | YES | trigger reverts to default text |
| feature-not-exposed grammar | YES | note role + disabled + data-staged + title |
| inert control | YES | zero set-device sent |
| pending → final reason | YES | two-phase note text |
| empty enumeration | YES | "No audio devices found" + disabled |
| non-feature error | YES | generic reason + disabled |
| schema rows in frame order | YES | per-entry binding + ordered labels |
| no dead control for absent key | YES | absence assertions |

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "absent capabilities render DISABLED WITH REASON … never a 4×4 stub" | :4-6 | REAL | 5 disabled-with-reason tests |
| "config-never-invents" device list | :6-7 | REAL | empty-enumeration test :166-171 |

## 6. Dead / duplicated / unreachable code
None.

## 7. Dependencies & flags
No mocks of the component; fixtures via audio-fixtures.ts; i18n via schema-generated entries.

## 8. Open questions for the next agent
None blocking.
