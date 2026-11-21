# AUDIT: ui/src/screens/playlist/options/optionsTestSupport.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 107 / 5 (FakeOptionsTransport [subscribe/emit/send], OPTION_ENTRIES, OPTION_META, MountOptions, mountOptions, OptionsProviders) |
| File verdict | REAL (test scaffolding — used by 5 suites, dispatch-spy assertions real; the fake's always-success shape leaves refusal paths untested) |
| Reachable from shipped entry point? | NO — test-only: every importer is a *.test.tsx (ThumbnailGrid.test, PlaylistOptionsProvider.test, MediaList.test, OptionsMenus.test, + OPTION_ENTRIES in OptionsMenus.test); no production import chain reaches it, so it is tree-shaken from the vite build (baseline #22 tsc+vite OK). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Grep `optionsTestSupport\|mountOptions\|OptionsProviders\|FakeOptionsTransport\|OPTION_META\|OPTION_ENTRIES` over ui/src → only the 4 test files + this file (zero production consumers); Read PlaylistProvider.tsx:128-153 (snapshot effect: `completed.kind === "state-snapshot"` guard — the fake's unit completion falls through safely); Read transport.ts type surface (CommandTransport: send/subscribe) via the import list; sed quote verification. Baseline: ui-vitest.log.txt 136 files / 1705 passed (the 4 consuming suites all green). |

## 1. One-paragraph verdict
Legitimate, tightly-scoped test scaffolding: a minimal CommandTransport double (spy array + listener set + always-completed send), three episode-named fixture entries with watched/half-watched metadata matching the media.svg grammar, and two mount helpers (full RTL render + a renderHook wrapper) that compose the Client→Playlist→PlaylistOptions provider stack. The suites using it assert real behavior through the spy (MediaList.test:59/:94/:107 and ThumbnailGrid.test:47 assert the exact commands dispatched), so this is a test double at the designed seam, not a mock of the subject under test. The one limitation: send() answers EVERY command — including `system.get-state-snapshot` and every playlist verb the real host refuses — with `{ completed: { kind: "unit" } }`, so the entire options-layer test surface never sees an error, a typed refusal, or a real snapshot payload; the provider's snapshot guard tolerates it (kind mismatch falls through to nulls), but no options test pins what the UI does when the host says no (the gap my PlaylistProvider audit's F-001 showed matters against the real host).

## 2. What actually works (verified)
— verified by the four consuming suites (PlaylistOptionsProvider.test.tsx 15-171, MediaList.test.tsx 20-152, OptionsMenus.test.tsx 31-149, ThumbnailGrid.test.tsx 14-55) all passing in baseline ui-vitest (136 files, 1705 passed), with dispatch assertions reading `transport.sent`.
— verified by reading PlaylistProvider.tsx:130-148: the fake's unit completion for `system.get-state-snapshot` cannot crash or mis-seed the provider — the `completed.kind === "state-snapshot"` guard skips it, leaving honest nulls.
— verified by Grep: no production file imports anything from this module → no fixture leakage into shipped code.

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/playlist/options/optionsTestSupport.tsx:30-37
Code:
```ts
  async send(request: CommandRequest): Promise<CommandResponse> {
    this.sent.push(request);
    return {
      gen: 1,
      requestId: request.requestId,
      result: { completed: { kind: "unit" } },
    };
  }
```
What is wrong: The double answers every command with a unit success — no error path, no `unsupported-for-host` (what the real host returns for every playlist command, per the SERVED table at apps/cli/src/dispatch.rs:38-54), no `feature-not-exposed`, no state-snapshot payload. The options layer's degradation behavior (what the sort/view/watched UI does when the engine refuses) is therefore untested by every suite built on this transport.
Why it matters: Test-honesty gap, not a product bug: the UI's real-host failure modes (the subject of the wave's S2 findings) are invisible to this entire test family, so regressions in the refusal handling of the options layer would pass CI.
What correct looks like: A configurable `respond` hook on the fake (default unit, overridable per cmd) so one test per suite can feed the typed refusal and pin the honest degradation.
Confidence: HIGH — Verification: read + consuming suites read; real-host refusal behavior cited from dispatch.rs SERVED (read in the CropTool audit earlier this session).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FakeOptionsTransport.subscribe` | 18-23 | REAL | — | Listener set + unsubscribe closure; correct. |
| `FakeOptionsTransport.emit` | 18-28 | REAL | — | Test seam for engine events; Set deletion during iteration is safe in JS. |
| `FakeOptionsTransport.send` | 30-37 | REAL | S4 | Spy + always-unit response (F-001). |
| `OPTION_ENTRIES` / `OPTION_META` | 40-51 | REAL | — | Fixtures; comment ("id 3 watched (full bar), id 2 half-watched") matches the data (3: {watched:true}; 2: {watched:true, watchedProgress:0.5}). |
| `MountOptions` / `mountOptions` | 53-78 | REAL | — | Composes Client+Playlist+PlaylistOptions; returns the spy — the suites' dispatch assertions depend on it. |
| `OptionsProviders` | 81-106 | REAL | — | renderHook wrapper; note it hardcodes OPTION_ENTRIES (no entries prop) — adequate for its users. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 4 consuming suites | YES (behavioral) | They render real components through this stack and assert DOM + dispatched commands via the spy. |
| Gap: no suite feeds an error/refusal response through this fake (F-001). | | |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "trimmed to what the options layer reads: state snapshot + playlist command spy" | optionsTestSupport.tsx:2-3 | PARTLY — snapshot is never ANSWERED, only dispatched | send() returns unit for get-state-snapshot; provider guard skips it (F-001). |
| "media.svg-shaped metadata: id 3 watched (full bar), id 2 half-watched" | optionsTestSupport.tsx:46 | REAL | Data matches (read). |

## 7. Dead / duplicated / unreachable code
None dead; all exports consumed by the 4 test files. Unreachable from production by design (tree-shaken).

## 8. Dependencies & flags
ClientProvider/transport types (CommandTransport, CommandRequest/Response, Event), PlaylistProvider (+PlaylistEntry), PlaylistOptionsProvider, sorting types. No engine/transport implementation imports, no feature flags. Co-located with the code it tests (repo convention: *.test.tsx siblings).

## 9. Open questions for the next agent
- If the options layer grows real-host degradation handling (per my PlaylistProvider F-001), this transport needs the refusal hook (F-001's fix) — owner is whoever wires `unsupported-for-host` into the provider keying.
