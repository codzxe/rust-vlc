# AUDIT: ui/src/components/list/range.ts
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 47 / 1 function (+2 interfaces) |
| File verdict | REAL — correct windowing math with one degenerate-input hazard |
| Reachable from shipped entry point? | YES (VirtualList consumes it — the playlist queue, settings lists, every virtualized list) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | read full file; hand-verified math for count=0, count=1, stale-scrollTop, negative scrollTop (below) |

## 1. One-paragraph verdict
Correct fixed-height virtualization math: negative scrollTop clamped (L35), stale scroll-past-end clamped to count-1 (the documented shrink race), overscan both sides, exact spacer height. I hand-checked the four edge cases (count=0 -> empty range with endIndex -1; count=1; scrollTop beyond end; negative scrollTop) — all produce sane windows. The only hazard: `itemHeight` of 0 (or negative) divides to Infinity/NaN with no guard; callers pass ROW_HEIGHTS constants today (playlist uses ROW_HEIGHTS.compact), so it is a latent contract, not a live bug.

## 2. What actually works (verified)
— verified by hand-execution of the formula for: (count=0, h=32, scrollTop=0, viewport=640, overscan=3) -> start 0/end -1/total 0 (correct empty); (count=10, scrollTop=9999) -> firstVisible=9, window clamped to tail (correct); (scrollTop=-50) -> clamped to 0 (correct); (count=1) -> single row (correct).

## 3. Findings
F-001 [S4]
Where: ui/src/components/list/range.ts:35-36
Code:
```
  const firstVisible = Math.min(Math.floor(Math.max(0, scrollTop) / itemHeight), Math.max(0, count - 1));
  const visibleCount = Math.ceil(Math.max(0, viewportHeight) / itemHeight);
```
What is wrong: itemHeight <= 0 is unguarded — division yields Infinity (visibleCount) or NaN (firstVisible via 0/0), producing a broken range. No current caller can pass it (ROW_HEIGHTS constants), so latent. Confidence: HIGH — Verification: read + hand math.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| VirtualRange (iface) | 14-20 | REAL | — | |
| computeRange | 34-40 | REAL | S4 | F-001 guard |
| VirtualScrollTarget (iface) | 42-46 | REAL | — | used by VirtualList scrollToIndex |

## 5. Test honesty
Perf test asserted by the header ("10k-row list renders ~25 row nodes") lives in the list lane (not my file). Would fail if windowing broke. UNVERIFIED beyond baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a stale scroll offset ... must never detach the window from the data" (L30-32) | doc | REAL | hand-verified clamp (count=10, scrollTop=9999) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
None (pure math). No feature flags.

## 9. Open questions for the next agent
- VirtualList owner: assert itemHeight > 0 at the call site or guard here (one line).
