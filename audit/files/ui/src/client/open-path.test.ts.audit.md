# AUDIT: ui/src/client/open-path.test.ts
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Lines / tests | 98 / 9 `it` blocks in 4 describes |
| File verdict | REAL — executed, 9/9 passing in baseline |
| Reachable from shipped entry point? | N/A — vitest suite (baseline ui-test.txt:151) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; baseline citation |

## 1. One-paragraph verdict
Pure-function test of the single UI-side open funnel — no mocks needed at
all (the unit under test is a pure classifier/request-builder). Covers the
admit set (m3u/m3u8/pls/xspf/cue, case-insensitive, Windows path), the
refuse set (media, doubled suffixes, extensionless, dotfile `.m3u` —
"never a guess"), the exact wire requests (`playlist.load {path, append:
false}` vs `session.open` with source verbatim, request id echoed), and
post-open routing. Every assertion is exact object equality.

## 2. What actually works (verified)
- Baseline: audit/runs/baseline/ui-test.txt:151 (verbatim):
```
 ✓ src/client/open-path.test.ts (9 tests) 5ms
```
- Refuse-set honesty (28-37, verbatim):
```
  it("refuses media, doubled suffixes, and extensionless paths (never a guess)", async () => {
    for (const path of [
      "/tmp/movie.avi",
      "/tmp/queue.m3u.bak",
      "/tmp/queue.",
      "/tmp/noext",
      "/tmp/.m3u",
```
- Exact wire shape (62-67):
```
    expect(openPathRequest(file("/media/queue.xspf"), 12)).toEqual({
      requestId: 12,
      cmd: "playlist.load",
      args: { path: "/media/queue.xspf", append: false },
    });
```

## 3. Findings
None. Zero-findings justification: pure function, exact equality asserts
on both branches, boundary cases (case-insensitivity `M3U8`/`M3u`, Windows
backslash path, dotfile) covered; executed green in baseline.

## 4. Per-function table
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| isPlaylistPath tests | 14-39 | REAL | admit set (6) + refuse set (5) |
| classifyOpenSource tests | 41-59 | REAL | URL/media/playlist kinds |
| openPathRequest tests | 61-90 | REAL | exact request objects, id echo |
| openPathRoute tests | 92-98 | REAL | /playlist vs /player |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| All 9 | YES | exact toEqual on every output |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the ONE UI-side file-open funnel ... parse lives engine-side" | comment :5-9 | REAL | open-path.ts consumes crates/playlists via the wire, not local parsing |

## 7. Dead / duplicated code
None.

## 8. Dependencies & flags
vitest only. No gates.

## 9. Open questions
None.
