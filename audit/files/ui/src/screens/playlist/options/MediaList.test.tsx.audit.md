# AUDIT: ui/src/screens/playlist/options/MediaList.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Lines / tests | 192 / 17 `it` blocks in 5 describes |
| File verdict | REAL — executed, 17/17 passing in baseline |
| Reachable from shipped entry point? | N/A — vitest suite (baseline ui-test.txt:81) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; baseline citation |

## 1. One-paragraph verdict
Real component test with strong honesty pins: without a metadata seam every
row renders honestly unwatched (50-54); without `onEntryMenu` no dead •••
button renders (101-104); without a play seam local-scope rows are inert
presentation with NO play target, NO menu, NO current marking (180-191).
Interaction pins are exact (`playlist.play-item` args `{ id: 2 }`, zero
sent on kebab clicks), and the keyboard-liveness structural test (68-90)
asserts per-row native focusable buttons with DOM-order proof via
`compareDocumentPosition`.

## 2. What actually works (verified)
- Baseline: audit/runs/baseline/ui-test.txt:81 (verbatim):
```
 ✓ src/screens/playlist/options/MediaList.test.tsx (17 tests) 247ms
```
- Dispatch pin (62-65):
```
    fireEvent.click(screen.getByRole("button", { name: "Play The Wand_S01E02.mp4" }));
    const plays = transport.sent.filter((r) => r.cmd === "playlist.play-item");
    expect(plays).toHaveLength(1);
    expect(plays[0] && "args" in plays[0] ? plays[0].args : null).toEqual({ id: 2 });
```
- Honesty pins (50-53, 101-103, 187-190): negative queries — no watched
  class/bar without meta, no dead kebab without the handler, no play target
  without the engine seam.
- Engine event consumption (109-111): `playlist.changed` current-changed →
  aria-current + class.

## 3. Findings
None. Zero-findings justification: exact dispatch payloads, negative
presence checks for every un-wired affordance, structural liveness
assertions; mock is the transport/options seam only.

## 4. Per-function table
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| rowOf helper | 13-16 | REAL | |
| frame anatomy describe | 18-55 | REAL | row count, titles, watched classes, bar widths 50%/100% |
| interaction describe | 57-122 | REAL | play dispatch, keyboard liveness, kebab stops play-through, current item |
| entryMenu seam describe | 124-162 | REAL | ContextMenu right-click + kebab route, per-entry menus, no-resolver honesty |
| local scope describe | 164-192 | REAL | inert rows without engine provider |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| All 17 | YES | exact args `{id:2}`, zero-sent counts, negative DOM queries, DOM-order comparison |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "empty honesty" | header :4 | REAL | tests 118-121 (panel owns empty state) |

## 7. Dead / duplicated code
None.

## 8. Dependencies & flags
@testing-library/react, vitest, optionsTestSupport shared harness. No gates.

## 9. Open questions
None.
