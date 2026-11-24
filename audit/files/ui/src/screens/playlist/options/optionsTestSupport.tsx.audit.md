# AUDIT: ui/src/screens/playlist/options/optionsTestSupport.tsx
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 106 / 4 (FakeOptionsTransport class, mountOptions, OptionsProviders + fixtures) |
| File verdict | REAL (test-only scaffolding; zero production imports) |
| Reachable from shipped entry point? | NO — and correctly so: `rg -ln optionsTestSupport ui/src` -> only 4 .test.tsx consumers (ThumbnailGrid, PlaylistOptionsProvider, OptionsMenus, MediaList). Never bundled (vite tree excludes test files; no production import exists). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg consumers (above); read of full file |

## 1. One-paragraph verdict
Honest test scaffolding: a minimal fake CommandTransport whose send()
records every request (the spy the options-layer tests assert against) and
returns a canned unit-completed envelope, plus three fixture entries and a
provider-stack helper. The one design limitation worth recording: the
canned response is `{ completed: { kind: "unit" } }` for EVERY command
(:32-36), so any test that needs a real response shape (error envelopes,
listing payloads, applied values) cannot use this transport — response-
handling paths of the options layer are only testable via the emit() seam,
not send(). No production leakage, no fabricated "evidence" — the fixtures
are named as fixtures.

## 2. What actually works (verified)
— Consumer census: 4 test files, 0 production imports (rg). The 4 suites
run in the vitest baseline (1705 passed).
— The transport faithfully implements the CommandTransport interface
(subscribe/send), so provider code under test runs its real paths; only
the wire is faked — the correct seam to fake.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/playlist/options/optionsTestSupport.tsx:30-37
Code:
  async send(request: CommandRequest): Promise<CommandResponse> {
    this.sent.push(request);
    return {
      gen: 1,
      requestId: request.requestId,
      result: { completed: { kind: "unit" } },
    };
  }
What is wrong: every command "succeeds" with a unit payload — including
commands whose real completed shape differs (playlist.save, media.get-*
etc. would return structured payloads). Tests built on this transport can
never observe a failure envelope, so error-handling code in the options
layer is untested through this path. What correct looks like: a
`respond(request) => CommandResponse` injectable, defaulting to unit.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| FakeOptionsTransport.subscribe | 18-23 | REAL | listener set, honest unsubscribe |
| FakeOptionsTransport.emit | 26-28 | REAL | synchronous fan-out |
| FakeOptionsTransport.send | 30-37 | REAL (limited) | F-001 canned response |
| mountOptions | 62-78 | REAL | exposes transport spy + RTL view |
| OptionsProviders | 81-106 | REAL | transport NOT exposed — renderHook-only; callers cannot assert sends through this wrapper (minor asymmetry with mountOptions) |
| OPTION_ENTRIES / OPTION_META | 40-51 | REAL | fixtures, clearly named |

## 5. Test honesty
This file IS the test honesty enabler for 4 suites. The transport records
sent requests — assertions on `transport.sent` WOULD fail if the UI
stopped dispatching (real mutation coverage). The canned-success limitation
is F-001.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a minimal ClientProvider transport (the PlaylistPanel.test.tsx FakeTransport pattern, trimmed...)" | :1-3 | REAL | shape matches the described pattern |

## 7. Dead / duplicated / unreachable code
None. OptionsProviders and mountOptions overlap in purpose but serve
different call styles (renderHook vs RTL render) — both consumed by the
test suites.

## 8. Dependencies & flags
testing-library/react, ClientProvider, PlaylistProvider,
PlaylistOptionsProvider, sorting types. Test-only by import graph
(verified) — no flags needed.

## 9. Open questions for the next agent
- sorting.ts / PlaylistOptionsProvider auditors: does any production path
  swallow a non-unit response (the shape this transport can never produce)?
  If the provider destructures `completed.value` on unit responses, tests
  pass while real engines return listing payloads — a mock-blind-spot hunt.
