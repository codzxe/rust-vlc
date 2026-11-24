# AUDIT: ui/src/screens/player/tools/video/HdrTool.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A05 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / tests | 119 / 6 (4 describes) |
| File verdict | REAL (baseline-verified green; honest about what jsdom cannot test) |
| Reachable from shipped entry point? | YES — HdrTool.tsx is the video-tools HDR section (tools dialog) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg baseline ui-test.txt:123 |

## 1. Verdict
Pins the HDR mode dropdown: closed trigger "Auto", six frame modes in order, selection by
click and keyboard (roving arrows + Enter commit), Escape restore-focus, list placement
degradation honestly limited to no-layout jsdom semantics, and the staged marker + pending
command (video.set-hdr). Baseline green: ui-test.txt:123 `✓ …(6 tests) 237ms`.

## 2. What actually works (verified)
- Baseline 6/6 (cite above).
- Mutation reasoning: mode list drift fails the array equality (:35); selection not
  committed fails :48-55; roving-keyboard regression fails :57-75; escape focus-restore
  fails :69-75; staged markers dropped fails :110-117.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/tools/video/HdrTool.test.tsx:83-86
Code:
 * state, cap as last resort — is pinned by resolveListPlacement unit
 * tests (useDropdownScrollportFlip.test.ts) + the lane's Playwright
 * probe (/tmp/cap-w26i08/).
What is wrong: the live-placement proof is cited to a /tmp directory that is not in the
repository — the evidence is ephemeral and unverifiable from any committed artifact. The
unit-test half of the citation IS in-tree (audited this run); the probe half is a dangling
reference.
What correct looks like: commit the probe output or drop the citation.
Confidence: HIGH — Verification: rg for 'cap-w26i08' in repo: no hits (only this comment).

## 4. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| closed trigger | YES | aria-expanded + text |
| six modes | YES | exact ordered array |
| click selection | YES | trigger text + closes |
| keyboard selection | YES | two arrows + Enter → HDR10 |
| jsdom no-layout degradation | YES | absence of markers/inline styles |
| staged markers | YES | data-staged-tool + data-pending-command + note |

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "jsdom has no layout … live behavior pinned by unit tests + Playwright probe" | :79-86 | HALF-REAL | unit tests real (this run); probe evidence uncommitted (F-001) |

## 6. Dead / duplicated / unreachable code
None.

## 7. Dependencies & flags
No mocks; real component; shared Dropdown primitive exercised through it.

## 8. Open questions for the next agent
None beyond F-001.
