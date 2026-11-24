# AUDIT: ui/src/screens/playlist/enqueueSources.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A01) |
| Date | 2026-11-24 |
| Lines / functions | 197 / 8 tests + 3 fixtures (StubTransport, withDesktopShell, clientWrapper) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — vitest baseline ui-test.txt:83 (8 tests, 39ms); seam consumed by PlaylistPanel.tsx + MiniPlayer.tsx (guard test proves both) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg + sed over demo-transport.ts (recentOpenings/rememberOpening truth), it-count 8 == baseline |

## 1. One-paragraph verdict
Real, unusually architectural suite: source listing (four catalog clips, exact labels/paths),
recents-after-clips ordering, location dedup, desktop-shell clip hiding (honest empty seam),
menu shape (empty/footer/separator/icons), playLater dispatch with title-vs-null, plus a
cross-consumer guard that greps the two consumers' SOURCE FILES to keep the retired inline
copy dead. All 8 pass in the baseline. One real defect threads through it: the recents test's
NAME says "most recent first" while its body asserts FIRST-opened order — and the underlying
demo-transport getter's doc makes the same false claim (its reverse() over an unshift-list
yields oldest-first). The suite documents the contradiction in a comment and then enshrines
the wrong order as expected behavior.

## 2. What actually works (verified)
— verified by citation audit/runs/baseline/ui-test.txt:83 `✓ src/screens/playlist/enqueueSources.test.tsx
(8 tests) 39ms` (1705-passed run); 8 it() blocks on disk match.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/playlist/enqueueSources.test.tsx:64,86-90 (vs ui/src/client/demo-transport.ts:353-356)
Code (test, verbatim):
  it("demo transport: remembered openings surface after the clips, most recent first", async () => {
...
      // The seam mirrors the transport's recentOpenings order verbatim
      // (demo-transport hands back [first-opened, …, latest] — its getter
      // comment claims "most recent first"; that mismatch is the demo
      // lane's, noted in the lane log).
Code (subject, verbatim — demo-transport.ts:353-356):
  /** The open-media dialog's "Recent" section (most recent first). */
  get recentOpenings(): readonly DemoRecentEntry[] {
    return [...this.recent].reverse();
  }
What is wrong: rememberOpening (demo-transport.ts:980-985) UNSHIFTS new entries, so `recent` is
[latest…oldest]; `.reverse()` then yields [oldest…latest] — first-opened-first. The getter's doc
says "most recent first": the doc is false and the demo dialog's Recent section renders
oldest-first. The test name repeats the false claim while the body pins the buggy order
(recent-0 = the FIRST-opened example.com). Why it matters: the ordering contract is
self-contradictory tree-wide (name says A, code does ¬A, test blesses ¬A); any consumer that
trusts the doc ordering displays recents backwards. What correct looks like: drop `.reverse()`
(or append instead of unshift), fix the doc, flip the test name/body to most-recent-first.
Confidence: HIGH — Verification: read (rg+sed on both files; ordering traced through
rememberOpening + getter).

## 4. Per-function table
| Test (line) | Verdict | Severity | Notes |
|---|---|---|---|
| browser shell: four demo clips (52) | REAL | — | exact ids/labels/paths |
| demo recents after clips (64) | REAL | S3 | F-001 name/body/ordering contradiction |
| dedup by location (107) | REAL | — | clip+recent never doubles |
| desktop shell hides clips (123) | REAL | — | empty seam |
| empty-state menu (136) | REAL | — | single disabled row |
| menu shape + footer (142) | REAL | — | ids/icons/separator pinned |
| playLater dispatch (155) | REAL | — | title AND null both asserted; footer never enqueues |
| cross-consumer guard (181) | REAL | — | reads consumers' source via node:fs; blocks re-inlining |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| 7 of 8 | YES | assertions bind to hook output, menu shapes, dispatch args |
| recents test (64) | YES for membership/dedup, NO for ordering semantics | body pins the CURRENT (buggy) order — a fix to most-recent-first would FAIL this test until its expectations flip (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "most recent first" (recents) | test name :64 + demo-transport.ts:353 | FALSE | body + code produce first-opened-first (F-001) |
| "MiniPlayer builds through this module — a re-inlined copy fails the guard" | header 7-9 | REAL | guard test 181-195 greps both consumers |

## 7. Dead / duplicated / unreachable code
None in this file; its whole purpose is keeping duplication dead (guard test).

## 8. Dependencies & flags
In-repo (ClientProvider, DemoTransport, menu types, seam). node:fs import inside the guard test
is dev-only (vitest), not shipped.

## 9. Open questions for the next agent
The demo lane log referenced at :88-90 ("noted in the lane log") — confirm the demo-transport
ordering defect is tracked there and whether the open-media dialog's Recent section was ever
re-ordered; if shipped users see the demo shell, this is user-visible.
