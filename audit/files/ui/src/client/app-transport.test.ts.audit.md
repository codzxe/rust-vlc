# AUDIT: ui/src/client/app-transport.test.ts
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | 131 / 6 it() tests, 2 describe blocks, 2 helpers |
| File verdict | REAL (6/6 pass in baseline; the selection + honesty rules pinned) |
| Reachable from shipped entry point? | YES via test lane — baseline ui-test.txt line 126 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full file read; baseline cite |

## 1. One-paragraph verdict
Honest host-probe test: pins the browser-fallback vs desktop-shell selection on the presence
of `__TAURI_INTERNALS__`, the one-log-line honesty rule on BOTH paths, the REAL
TauriTransport dispatch shape (`lumen_dispatch` with `{ request }` payload — verified through
a recording fake internals), the no-re-probe invariant (selection captures the backend; a
send adds exactly one invoke), and the isDesktopShell/probe agreement. Global state is saved
and restored in afterEach — no cross-test leakage.

## 2. What actually works (verified)
- Baseline: ui-test.txt line 126 `✓ src/client/app-transport.test.ts (6 tests) 7ms`.
- Read-verified: `expect(log).toHaveBeenCalledTimes(1);` on both paths (the honesty rule —
  exactly one visible line stating what is live);
  `const dispatch = invokes.find((call) => call.command === "lumen_dispatch");`
  `expect(dispatch?.payload).toEqual({ request });` (real wire shape);
  no-re-probe: `expect(invokes).toHaveLength(0);` then send -> `toHaveLength(1)`.

## 3. Findings
Zero findings beyond notes below. Checked and clean: globals restore (savedInternals
round-trip, lines 46-54); DemoTransport-vs-TauriTransport instanceof discrimination; label
content matching (origin-specific copy).

## 4. Per-function table
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| installInternals | 36-49 | REAL | records every invoke; canned OPEN_RESPONSE |
| describe selectAppTransport | 56-113 | REAL | 5 tests incl. wire shape + no-re-probe |
| describe isDesktopShell | 115-131 | REAL | both-ways probe agreement |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| inside the shell: REAL TauriTransport over lumen_dispatch | YES | invoke name + payload equality |
| the choice is announced exactly once | YES | call-count 1 + label arg |
| the desktop probe sees the same backend (no drift) | YES | invoke count 0 -> 1 |
| outside the shell: browser fallback announced once | YES | origin + instanceof + log-once |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the real webview cells remain ADR-004/INFRA-BLOCKED" (header line 7) | file comment | REAL-UNPROVEN | desktop-tauri build fails at gdk-sys per baseline (AGENT-SPEC §13); no real webview run exists |
| 6 tests pass | baseline ui-test.txt:126 | REAL | vitest line |

## 7. Dead / duplicated code
None.

## 8. Dependencies & flags
Imports TauriTransport + DemoTransport (the real transports under selection) — the SUT is
the selection logic, transports exercised through the injected seam. No router/DOM beyond jsdom globals.

## 9. Open questions
- Does TauriTransport's response handling have its own test (tauri-transport.test.ts, 10
  tests per baseline line 85)? Yes — separate file, not in my assignment.
