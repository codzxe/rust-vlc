# AUDIT: ui/src/screens/player/customize/CustomizePanels.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Lines / tests | 868 / 39 `it` blocks in 8 describes |
| File verdict | REAL — executed, all 39 passing in baseline |
| Reachable from shipped entry point? | N/A — vitest suite; wired into `ui` npm test lane (baseline ui-test.txt:30) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | full read (868 lines); baseline citation |

## 1. One-paragraph verdict
This is what a UI component test should be. The mocks sit at the correct
seam (a `StubTransport implements CommandTransport` that records every
sent request and answers `system.list-commands` with a scriptable
capability listing); assertions are on dispatch payloads, ARIA state, DOM
structure, focus ownership, and persisted localStorage bytes — not on
snapshots of the component itself. Drag-and-drop is exercised through a
functional `dataTransfer` stand-in (real setData/getData Map), keyboard
paths (Alt+ArrowRight, Delete) alongside pointer paths. The vitest
baseline executed this exact file: 39 tests, 2141ms, all passing
(audit/runs/baseline/ui-test.txt:30).

## 2. What actually works (verified)
- Baseline run: audit/runs/baseline/ui-test.txt:30 (verbatim):
```
 ✓ src/screens/player/customize/CustomizePanels.test.tsx (39 tests) 2141ms
```
- Dispatch-payload pinning (test at lines 217-231):
```
    const seeks = stub.sent.filter((request) => request.cmd === "playback.seek");
    expect(
      seeks.map((request) => (request.args as { target: { deltaMs: number } }).target.deltaMs),
    ).toEqual([-10_000, 10_000, -15_000, 15_000]);
```
- Unit correctness pinned at the wire edge (lines 773-775): the engine-reported 130 s
  position must cross as `130_000_000` µs in `media.add-bookmark` args — a
  seconds-vs-µs regression would fail this test.
- Persistence seam round-trip: Save → `localStorage` bytes → `parseToolbarLayout`
  equality (441, 453-458); junk/healing parser tests (463-474).
- Focus semantics regression (h07 defect 2): Save keeps focus inside the
  dialog, Escape fires the `lumen:close-panel` seam (637-662); the dead-BODY
  focus keeper is tested by actually blurring (664-676).
- Engine-event subscription pinning (284-296, 831-846): rate 1.5 event →
  next click dispatches 2 — the non-optimistic cycle contract.

## 3. Findings
F-001 [S4] — UI copy pins third-party branding
Where: ui/src/screens/player/customize/CustomizePanels.test.tsx:146,236 (and the 31-label array 140-152)
Code:
```
      "Shuffle", "Record", "Snapshot", "Picture in picture", "HDR", "Zoom",
      "Right sidebar", "Quit VLC", "VLC settings",
```
What is wrong: the test hard-pins the labels "Quit VLC" and "VLC settings"
— VLC trademarked names in the shipped product's customize inventory. The
test is honest about what the component renders; the observation is that
these two labels make any rebrand (tools/xtask rebrand lane, `check_identity`
gate) a red test unless the rebrand allowlist covers them.
Confidence: HIGH — Verification: read (component renders it; test pins it).

## 4. Per-function table (harness-level; test file)
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| StubTransport class | 60-96 | REAL | records `sent`; scriptable completions; honest `feature-not-exposed` default |
| dataTransfer stand-in | 115-124 | REAL | Map-backed setData/getData (jsdom has none) |
| frame catalog describe | 130-177 | REAL | exact 31-label array equality + length + anatomy |
| capability policy describe | 179-214 | REAL | catalog-not-report contract (31 render on any listing) |
| dispatch-spy describe | 216-297 | REAL | payload equality on seek/stop/shuffle/repeat/rate |
| box+band describe | 299-460 | REAL | DnD reorder/place/remove, Save/Cancel/persistence |
| persistence helpers describe | 462-494 | REAL | parser junk/heal, equality, broadcast event |
| band variants describe | 496-634 | REAL | per-frame structural pins, incl. document-position order (560-566) |
| focus/Escape describe | 636-677 | REAL | real blur + seam fire |
| real-bar describe | 679-867 | REAL | SnapTransport snapshot seam; µs unit pin; engine-truth cycles |

## 5. Test honesty
| Test group | Would it fail if code were broken? | Reason |
|---|---|---|
| All 39 | YES | exact payload arrays, role queries, attribute assertions, byte-level localStorage checks; the one intentionally weak assertion (class-against-itself) was itself replaced per the comment at 556-559 ("mild theater — this is the real structural pin") |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "w26.i05 ... the box is a catalog, not a capability report" | file header 1-11 | REAL (tested) | tests 131-213 |
| "39 tests" | — | REAL | baseline ui-test.txt:30 |

## 7. Dead / duplicated code
None.

## 8. Dependencies & flags
@testing-library/react, vitest, ClientProvider/MockTransport, generated
commands types. No feature gates.

## 9. Open questions for the next agent
- Does `check_identity` / lint-literals allow "Quit VLC"/"VLC settings" (see tools/xtask/data/rebrand-allowlist.toml)?
- The SnapTransport double-cast `as unknown as CommandResponse["result"]` (704) is the documented snapshot-union workaround — check whether generated types can express it without the cast.
