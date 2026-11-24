# AUDIT: ui/src/client/tauri-transport.test.ts
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A03) |
| Date | 2026-11-24 |
| Lines / functions | 264 / 10 tests + FakeBackend fixture |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — vitest baseline ui-test.txt:85 (10 tests, 11ms). Subject is the desktop app's real transport: app-transport.ts:67 `new TauriTransport(backend, ...)` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg (subject consumers), it-count 10 == baseline |

## 1. One-paragraph verdict
A tight, hostile transport suite: single-command routing (every send → `lumen_dispatch` with the
Rust parameter name), verbatim response passthrough, typed CommandError rejection → error
envelope with echoed requestId, unexpected IPC failure rethrown (never inventing a response),
event forwarding, double unsubscribe-release accounting (before AND after listen resolution),
listen-failure seam, and out-of-shell detection throwing the typed unavailable error. The
injected-internals test pins the real binding contract (plugin:event|listen + transformCallback +
unlisten by event id). The file honestly marks its own limit: the actual webview frame shape
stays [VERIFY at the admission build] — and the desktop-tauri build is environment-blocked
(baseline §13), so end-to-end IPC is UNVERIFIED here by design, not by test fraud.

## 2. What actually works (verified)
— verified by citation audit/runs/baseline/ui-test.txt:85 `✓ src/client/tauri-transport.test.ts
(10 tests) 11ms` (1705-passed run); 10 it() blocks on disk match.

## 3. Findings
Zero numbered findings. Checked, with how:
- Tautologies: none — every assertion binds to recorded invokes, response objects, error
  envelopes, subscriber arrays, or spy call counts (e.g. lines 89-93, 120-127, 135-137).
- Mock-of-self: none — FakeBackend mocks the Tauri IPC boundary (the thing the transport wraps),
  not the transport. The internals test (212-263) uses a hand-rolled internals object matching
  the documented binding surface.
- Swallowed errors: explicitly tested AGAINST (line 130-138 asserts rethrow).
- Cleanup: globalThis.__TAURI_INTERNALS__ saved/restored in try/finally (230-261, 193-203) — no
  cross-test pollution.
- Fixture limitation (not a finding): FakeBackend keeps a single listenHandler, so concurrent
  multi-listener fan-out is untested; the transport binds one channel listener, so the gap is
  theoretical.

## 4. Per-function table
| Test (line) | Verdict | Severity | Notes |
|---|---|---|---|
| routes through lumen_dispatch (82) | REAL | — | payload `{ request: OPEN_REQUEST }` |
| response verbatim (96) | REAL | — | resolves.toEqual |
| CommandError → envelope (104) | REAL | — | requestId echoed 42 |
| unexpected IPC failure rethrown (130) | REAL | — | /lumen_dispatch IPC failed/ |
| event forwarding (140) | REAL | — | payload→Event |
| unsubscribe stops listener (151) | REAL | — | unlisten spy |
| unsubscribe-before-resolve releases once (163) | REAL | — | exactly-once accounting |
| listen failure seam (176) | REAL | — | onListenFailure called with error |
| out-of-shell: null + typed throw (192) | REAL | — | TauriTransportUnavailableError |
| internals binding contract (212) | REAL | — | handler id 5, unlisten eventId 31 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 10 | YES | e.g. drop error normalization → 104 fails with raw rejection; leak a fabricated response on IPC failure → 130 fails; wrong unlisten payload → 255 fails |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "frame shape and 'Any' default target stay [VERIFY at the admission build]" | comment 206-211 | honest UNVERIFIED marker | no webview in sandbox; desktop build env-blocked (baseline §13 gdk-sys) |
| single-command rule (nothing else addressed) | comment 91-92 | REAL | test 82-94 |

## 7. Dead / duplicated / unreachable code
None. OPEN_REQUEST/RESPONSE/STATE_EVENT fixtures all consumed.

## 8. Dependencies & flags
In-repo: tauri-transport + generated command types. No tauri npm package needed — the seam is a
hand interface (TauriBackend), which is exactly why this is testable without a webview.

## 9. Open questions for the next agent
Confirm at the admission build that the real webview's event frames match the pinned shape
(comment 206-211's own TODO). Until then, the listen/unlisten contract here is the only guard.
