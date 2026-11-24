# AUDIT: ui/src/screens/playlist/PlaylistPanel.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | 451 / 26 it() tests, 4 describe blocks, 5 helpers |
| File verdict | REAL (26/26 pass in baseline; assertions bite) |
| Reachable from shipped entry point? | YES via test lane — baseline ui-test.txt line 38 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg counts (exit 0); sed quote verification (exit 0); full file read |

## 1. One-paragraph verdict
Honest, dense component test: a typed FakeTransport (`implements CommandTransport` —
compiler-checked, better than a cast) records every CommandRequest; tests assert exact
dispatch args (`[{ id: 1 }]`, `{ id: 3, toIndex: 0 }`, move-toIndex post-removal arithmetic),
engine-truth checkbox faces, virtualization bounds (10k entries -> <100 rows + spacer height
400000px), and two genuine pinned regressions (empty-text external drop no-op; not-exposed
state). 26 `it(` == baseline's 26 tests, exact match.

## 2. What actually works (verified)
- Baseline: audit/runs/baseline/ui-test.txt line 38 `✓ src/screens/playlist/PlaylistPanel.test.tsx (26 tests) 924ms`.
- Virtualization proof (verified sed -n '128,139p'): `setup(many);` … `expect(rows.length).toBeLessThan(100);`
  `expect(spacer.style.height).toBe("400000px"); // 10_000 × ROW_HEIGHTS.compact (40, w26.a09)` —
  would FAIL if the windowed list rendered all rows or dropped the spacer math.
- Regression pin (lines 246-252, read): external drop with empty/whitespace text payload
  asserts ZERO playlist.move dispatches — pins the real Number("")===0 bug fix (w26.k14 j14 #9).
- Exact-args assertions: play-item `{ id: 1 }`, move indices `[{ toIndex: 1 }, { toIndex: 2 }]`,
  enqueue `{ source: { kind: "url", url: "https://demo.lumen.dev/live/edge-hsl.m3u8" }, ... }`.

## 3. Findings
F-001 [S4]
Where: lines 165-166, 192 (verified rg)
Code: `expect(transport.sent.some((r) => r.cmd === "playlist.next")).toBe(true);`
What is wrong: membership form again — next/prev/clear dispatches are never count-checked;
double-dispatch regressions pass. (The entry-action and repeat tests DO use exact
filter().map() equality — the strongest form in the file.) Confidence: HIGH — read.

F-002 [S4]
Where: lines 105-108 (`rowOf` returns `rows.find(...)!`), 143 (`querySelector<HTMLElement>(".lumen-vlist__spacer")!`)
What is wrong: non-null `!` assertions convert a missing row into a thrown TypeError with an
opaque message instead of a clean expect failure. Cosmetic DX debt in 4 spots.
Confidence: HIGH — read.

## 4. Per-function table
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FakeTransport | 26-90 | REAL | — | implements CommandTransport; canned snapshot; exposed=false lane |
| ENTRIES / setup / rowOf | 92-118 | REAL | S4 | `!` in rowOf |
| describe render | 120-158 | REAL | — | unknowns, current marker, virtualization, empty, not-exposed |
| describe queue controls + entry actions | 160-253 | REAL | S4 | exact move arithmetic; drag reorder; empty-drop regression |
| describe queue-row context menu | 255-382 | REAL | — | 8 pinned rows, focus, no-seam disabled honesty, route nav |
| describe enqueue affordance | 386-451 | REAL | — | demo clips + Tauri-internals empty-seam case with cleanup |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| bounded window for 10k entries | YES | row count < 100 + spacer height exact |
| external drop empty payload no-op | YES | zero move dispatches asserted |
| entry actions dispatch play-item / move / remove | YES | exact args arrays, order included |
| repeat button cycles off → all → one | YES | modes array equality [{all},{one}] |
| next/prev buttons dispatch | YES for dispatch, NO for dup-dispatch | `some()` (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| 26 tests pass | baseline ui-test.txt:38 | REAL | vitest line |
| "The FakePlaylistTransport doubles as the dispatch spy" (header line 4) | file itself | REAL | `sent` recorded + asserted |

## 7. Dead / duplicated code
FakeTransport duplicates ControlBar.test.tsx's FakeTransport shape (different canned
snapshot). Shared-fixture candidate — duplicated in ≥3 test files (see transport.test.ts
audit). Not dead.

## 8. Dependencies & flags
react-router MemoryRouter for the /media-info route test — heavier than needed but real
routing. No timers; waitFor used properly. No snapshot tests.

## 9. Open questions
- The demo-clip enqueue fixtures (demo.lumen.dev URLs) come from useEnqueueSources — is that
  seam demo-gated in production builds? (W-next: enqueueSources.test.tsx assignee.)
