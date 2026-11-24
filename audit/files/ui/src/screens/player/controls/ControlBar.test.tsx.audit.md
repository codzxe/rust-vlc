# AUDIT: ui/src/screens/player/controls/ControlBar.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | 1231 / 60 it() tests, 12 describe blocks, 10 helpers |
| File verdict | REAL (60/60 pass in baseline vitest run; assertions bite) |
| Reachable from shipped entry point? | YES via test lane: `npx vitest` — baseline ui-test.txt line 29 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | rg counts (exit 0); sed quote verification (exit 0); baseline cite ui-test.txt |

## 1. One-paragraph verdict
This is a genuinely honest, dense integration test: it mounts the REAL provider chain
(ClientProvider → PlaylistProvider → PlayerControlsProvider → ControlBar, and the full
PlayerScreen for canvas-menu tests) against a hand-rolled FakeTransport that records every
CommandRequest, then asserts actual dispatch args, µs wire units, aria state, engine-truth
(non-optimistic) values, and honest no-media/unknown renderings. Every command assertion
checks a real cmd string with real args; the volume test even pins that the engine's 0.4
completion overrides the optimistic 0.51. 60 `it(` blocks == baseline's 60 tests (counts
match exactly). Weaknesses are limited to assertion form (`some` instead of count) and a
type-cast escape hatch.

## 2. What actually works (verified)
- Baseline: audit/runs/baseline/ui-test.txt line 29 `✓ src/screens/player/controls/ControlBar.test.tsx (60 tests) 1791ms` — 60/60 pass (my count of `^\s+it\(` = 60, exact match).
- FakeTransport is a recording seam (verified sed -n '43,52p'):
  `class FakeTransport {` / `readonly sent: CommandRequest[] = [];` / `async send(request: CommandRequest): Promise<CommandResponse> {` / `this.sent.push(request);`
- Anti-optimism regression (verified sed -n '417,430p'): engine completes `volume: 0.4`
  after ArrowUp (0.51 predicted) and the test asserts `aria-valuenow` is "0.4" — would FAIL
  if the UI were optimistic.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/controls/ControlBar.test.tsx:188 (and ~20 sibling sites, rg verified)
Code: `expect(fake.sent.some((r) => r.cmd === "playback.toggle-play-pause")).toBe(true);`
What is wrong: membership assertion — no test asserts HOW MANY commands were dispatched.
A double-dispatch or duplicate-fan-out regression (e.g. provider re-mount re-firing the
command) passes every test in this file. Why it matters: command storms are a real class of
UI bug the file claims to pin ("every control invokes a real command"). What correct looks
like: `expect(fake.sent.filter(...)).toHaveLength(1)` at least for the primary dispatches.
Confidence: HIGH — Verification: read (rg all `sent.some` sites).

F-002 [S4]
Where: ui/src/screens/player/controls/ControlBar.test.tsx:118
Code: `} as unknown as CommandResponse["result"]);`
What is wrong: the snapshot helper force-casts partial objects through
`as unknown as`, bypassing the wire type. The comment at 107-110 claims unit corrections
are pinned by types; this cast means a snapshot missing/mistyping a wire field still compiles
and only fails at runtime lookup. Why it matters: type-level wire drift (the exact thing
w26.k17 fixed) is unguarded in the test's own fixtures. What correct looks like: build the
snapshot from the exported factory/type so the compiler rejects drift.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: ui/src/screens/player/controls/ControlBar.test.tsx:1203-1230 (CSS regression guards)
Code: `const controlsCss = readFileSync(`${dirname}/controls.css`, "utf8");` …
`return controlsCss.match(new RegExp(`\\n${escaped}[^{]*\\{([^}]*)\\}`))?.[1] ?? "";`
What is wrong: the CSS guard parses controls.css with a hand-rolled regex — `[^{]*` lets a
prefixed selector shadow the row, and `?? ""` makes a missing rule return "" (the
containment assertions then fail, which is the intent, but a REFACTORED selector name
yields a misleading "consolidated pin" failure rather than a clear missing-rule error).
Fragile-by-design pins, not test fraud. What correct looks like: a real CSS parser or an
assertion on the specific consolidated selector presence + legacy absence (which it does
for the play-pill pin — that one is fine).
Confidence: MEDIUM — Verification: read.

## 4. Per-function table (helpers + describe coverage)
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FakeTransport | 43-65 | REAL | — | records sent; canned respond; event emit |
| snapshotResult | 84-118 | REAL | S4 | type-cast escape (F-002) |
| track() | 67-82 | REAL | — | TrackInfo factory |
| renderBar / renderBarWithQueue | 121-159 | REAL | — | real provider chain |
| openAppMenu / openCanvasTrackMenu / flush / click | 162-179 | REAL | — | interaction helpers |
| describe transport cluster | 181-258 | REAL | S4 | 8 it; `some` form (F-001) |
| describe seekbar cluster | 261-382 | REAL | — | pending-seek truth incl. seek-failed typed error |
| describe volume/mute | 384-437 | REAL | — | anti-optimism pin (strongest test in file) |
| describe track menus | 440-600 | REAL | — | honest unknowns, aria-checked engine truth |
| describe error surfacing | 603-664 | REAL | — | desktop cold-start no-chip pin; playback.error fatal |
| describe app menu | 666-804 | REAL | — | pinned 12-row order; accelerators |
| describe seek cluster | 807-864 | REAL | — | remaining-face toggle, go-to-time |
| describe right cluster | 867-1024 | REAL | — | panels, queue radiogroup |
| describe k07 P1/P2-2/P2-4/P3-8 | 1028-1200 | REAL | — | roving tabindex + i18n + profile persistence |
| describe CSS pins | 1203-1230 | REAL | S4 | regex fragility (F-003) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| volume slider reflects the engine completion | YES | asserts 0.4 beats optimistic 0.51 |
| keyboard commit dispatches playback.seek | YES | asserts exact µs args `{target:{kind:"absolute",time:100_000_000}}` |
| timing.seek-failed clears pending AND surfaces | YES | asserts data-error-code=not-seekable on live DOM |
| previous is inert while the queue is empty | YES | asserts disabled + zero dispatch |
| play/pause toggle (line 182-189) | YES for dispatch, NO for double-dispatch | `some()` membership only (F-001) |
| CSS pins (1212-1230) | YES | string containment on the real CSS text |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "60 tests, all pass" | baseline ui-test.txt:29 | REAL | vitest log line |
| "spy assertions via a fake transport" (file header line 2) | file itself | REAL | FakeTransport records `sent` |

## 7. Dead / duplicated / unreachable code
None. All helpers used; MockTransport imported AND exercised (lines 604, 868, 919, 970...).

## 8. Dependencies & flags
No new deps: @testing-library/react, vitest, node:fs (CSS pins). No timers faked; async
flushed via act(). No snapshot tests (no inline toMatchInlineSnapshot) — good.

## 9. Open questions for the next agent
- Does any e2e Playwright run cover the same dispatch paths for real (see ui/e2e audits)?
