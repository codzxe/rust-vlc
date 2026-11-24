# AUDIT: ui/src/screens/network-stream/NetworkStreamScreen.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A10 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / tests | 183 / 11 (2 describes) |
| File verdict | REAL (baseline-verified green; wire-payload + persistence + a11y contracts) |
| Reachable from shipped entry point? | YES — NetworkStreamScreen.tsx is the /open-network-stream route screen (visual baseline corroborates) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg baseline ui-test.txt:88 |

## 1. Verdict
Behavior suite over the network-stream dialog: pure URL validator, frame anatomy, initial
focus on the field, four dismissal paths (Escape/scrim/Cancel asserted with ZERO commands
sent), honest invalid-URL error with role=alert, the submit wire shape (session.open with
{kind:"url"} source), recents persistence + refill, named-open writing a local playlist.
Baseline green: ui-test.txt:88 `✓ …(11 tests) 315ms`.

## 2. What actually works (verified)
- Baseline 11/11 (cite above).
- Mutation reasoning: dismissal leaking a session.open fails via `transport.sent.length`
  == 0 assertions (:98-110, 160-165); wrong wire shape fails the exact args equality
  (:130-132); error not announced fails role="alert" (:119); recents not persisted fails the
  localStorage JSON equality (:134-136); playlist write regression fails (:149-156).
- SpyTransport (:21-30) is a command-recording fake at the transport seam — the component
  tree, router, and stores are real; beforeEach/afterEach reset both stores (:57-65).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/network-stream/NetworkStreamScreen.test.tsx:26-29
Code:
  async send(request: CommandRequest): Promise<CommandResponse> {
    this.sent.push(request);
    return { gen: 1, requestId: request.requestId, result: { completed: { kind: "unit" } } };
What is wrong: SpyTransport answers EVERY command with a generic "unit" completion — the test
never exercises the screen's behavior on a FAILED session.open (no error-result path exists
here, unlike audio.test.tsx's revert test). Submit-failure UX is untested in this file.
What correct looks like: one test with an errorResult-shaped transport asserting the screen
stays + shows the honest failure.
Confidence: HIGH — Verification: read (no failure-response fixture in file).

## 4. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| isValidNetworkUrl | YES | 5 literal true/false cases |
| frame anatomy | YES | roles/labels/buttons presence |
| initial focus | YES | activeElement identity |
| Escape/scrim/Cancel zero-send | YES | sent.length === 0 |
| invalid URL honest error | YES | error text + role=alert + no session.open |
| submit wire shape | YES | args deep equality |
| named open → playlist | YES | saved playlist JSON checked |
| recents render + refill | YES | field value set on click |
| empty recents | YES | empty-state text |

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "http/https only, honest error, nothing sent" | :6 | REAL | validator + zero-send tests |
| helper copy "playlist will be saved…" made real | :9-11 | REAL | localStorage playlist assertion |

## 6. Dead / duplicated / unreachable code
None.

## 7. Dependencies & flags
No component mocks; real MemoryRouter routes; stores reset via resetPlaylistsForTests.

## 8. Open questions for the next agent
Cover the failed session.open path (F-001).
