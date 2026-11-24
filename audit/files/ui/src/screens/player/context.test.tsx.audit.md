# AUDIT: ui/src/screens/player/context.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / functions | 414 / 4 transport classes + 2 helpers + 11 tests |
| File verdict | REAL — failure-path tests of the player-state seam; all 11 ran green in baseline |
| Reachable from shipped entry point? | NO — vitest suite; ran in baseline ui-test.txt:48 `✓ src/screens/player/context.test.tsx (11 tests) 161ms` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg mirror-claim verification vs apps/cli; baseline cite |

## 1. One-paragraph verdict
This is what honest failure-path testing looks like: three scripted transports
(ERROR envelope, rejecting send, and a desktop-host mirror) pin the provider's
pendingToggle resolution on every path — engine ERROR, dead transport, happy
accepted+state-changed, a failing non-toggle command that must NOT resolve the
toggle, playback.error mid-flight, tracks.changed kind guards, and the desktop
cold-start "no session open" answers. Every assertion is exact state
(booleans, error code/stage/details). The claimed "mirrored from the Rust
sources (not a mock's guess)" for DesktopHostTransport checks out against the
real host code: dispatch.rs:170 and host.rs:860 produce the exact strings the
transport replays. The mocks mock the transport seam — the thing under test
(the provider state machine) is real.

## 2. What actually works (verified)
— Baseline: audit/runs/baseline/ui-test.txt:48 — `✓
src/screens/player/context.test.tsx (11 tests) 161ms`; overall `Test Files
136 passed` / `Tests 1705 passed | 1 skipped` (L178-179).
— Mirror claim verified: `rg "not served by this build's command-line host"
apps/cli/src/` → dispatch.rs:170; `rg "no session open" apps/cli/src/host.rs`
→ L860/1091/1288/1322/1340 (Stage::Session, ErrorCode::NotReady) — exactly
the shapes DesktopHostTransport returns (L106-114).
— Mutation reasoning: if pendingToggle stuck on error, the J06-2 test
(L166-180) fails on `expect(probe.latest().pendingToggle).toBe(false)` — this
is the exact stuck-spinner regression the header documents; if the kind guard
regressed, `expect(Object.keys(selections)).toEqual([])` (L260) fails; if the
desktop guard over-reached, L398-413's typed-error assertions fail.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/context.test.tsx:327-332
Code:
    await act(async () => {
      await Promise.resolve();
    });
    await act(async () => {
      await Promise.resolve();
    });
What is wrong: mount-probe resolution is driven by exactly two microtask
flushes — an implicit timing contract with the provider's internal promise
chain length; a future extra await hop in the provider makes these tests miss
the settled state without failing loudly deterministically (flaky-category
pattern, not current behavior — file passed in 161 ms).
Confidence: MEDIUM — Verification: read.
F-002 [S4]
Where: ui/src/screens/player/context.test.tsx:365-379
Code:
      if (request.cmd === "system.get-state-snapshot") {
        return {
          completed: {
            kind: "state-snapshot",
…
        } as unknown as CommandResponse["result"];
What is wrong: a double type-cast escape hatch (`as unknown as`) to build the
snapshot result — bypasses the protocol types for one test fixture instead of
declaring it; if the result shape drifts, this cast silences the compiler.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| ErrorTransport | 20-53 | REAL | scripted ERROR envelope; per-cmd override hook |
| UnreachableTransport | 56-63 | REAL | rejecting send — host-unreachable path |
| DesktopHostTransport | 81-121 | REAL | mirror verified vs Rust host (see §2) |
| StateProbe / mountProbe | 124-163 | REAL | real action triggers via fireEvent |
| 11 tests | 165-414 | REAL | all exact-state; baseline ok |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| toggle answers ERROR clears pending + typed error | YES | exact booleans + error fields |
| dead transport clears pending + not-ready | YES | exact stage/code |
| happy path accepted→state-changed resolves | YES | pending true→false + playbackState |
| failing non-toggle keeps toggle pending | YES | pendingToggle stays true, pendingSeek=120 |
| data track change → no stray key | YES | Object.keys == [] |
| secondary-audio → no-op | YES | Object.keys == [] |
| video kind still lands | YES | selections == {video: 11} |
| playback.error clears pending | YES | pending false + code "decoder" |
| desktop idle probes surface no error | YES | lastError null + state idle |
| demo not-found idle face stays exempt | YES | lastError null |
| guard doesn't over-reach (failed toggle surfaces) | YES | stage/code/message exact |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "mirrored from the Rust sources (not a mock's guess)" | file L66-79 | REAL | dispatch.rs:170 + host.rs:860 strings match |
| "violating doc 08 §5.1's ≤250 ms pending contract" | file L6-7 | REAL-UNPROVEN | doc 08 not re-read in this audit (UNVERIFIED: doc-08 budget line) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
@testing-library/react + vitest; imports the real ClientProvider and
PlayerControlsProvider. No snapshots-in-snapshot traps; no network.

## 9. Open questions for the next agent
- Does any test pin the 250 ms pending budget itself (timer-based), or only
  the resolution semantics? (None here — timing budget untested.)
