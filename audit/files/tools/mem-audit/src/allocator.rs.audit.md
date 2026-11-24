# AUDIT: tools/mem-audit/src/allocator.rs
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | 430 / 10 functions + GlobalAlloc impl + 3 tests |
| File verdict | REAL (counting allocator; 3/3 tests pass in baseline) |
| Reachable from shipped entry point? | NO — dev tool `lumen-mem-audit` (installed as #[global_allocator] in tools/mem-audit/src/main.rs:76 `static GLOBAL: Counting = counting_global();`). Test tool, correctly not shipped. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | full file read; baseline cite (cargo-test-workspace.txt:3283-3285); rg consumers (exit 0) |

## 1. One-paragraph verdict
Genuine counting global allocator with honest requested-size semantics, explicit realloc
counting, documented-and-clamped multithread transients, and windowed phase attribution.
The unsafe surface is exactly the sanctioned GlobalAlloc forwarding with per-block SAFETY
comments and success-path-only counting. Its 3 unit tests drive PRIVATE sets through the
same `GlobalAlloc` impl (exact deltas: 4096 -> +4096 bytes / +1 call) and pass in the
committed baseline. One minor counting hole: same-size reallocs are invisible to every counter.

## 2. What actually works (verified)
- Baseline: audit/runs/baseline/cargo-test-workspace.txt:3283-3285 —
  `test allocator::tests::counted_allocation_moves_counters_by_exact_size ... ok` /
  `test allocator::tests::adjacent_windows_compose ... ok` /
  `test allocator::tests::peak_tracks_concurrent_live_and_reseeds ... ok`
  (package lumen-mem-audit, 9 passed, exit_code=0, lines 3293-3295).
- Consumers (rg, exit 0): walk.rs:716/731 `Window::open()` sequential phases; main.rs:76
  global install; selfcheck.rs:135 floor snapshot.
- Unsafe discipline (verified read): every block forwards pointer+layout verbatim to
  `System` and counts only on non-null success (lines 207-248).

## 3. Findings
F-001 [S4]
Where: tools/mem-audit/src/allocator.rs:232-248
Code (verified):
```
            let old = layout.size();
            if new_size > old {
                self.counters.count_growth(new_size - old);
            } else if new_size < old {
                self.counters.count_dealloc(old - new_size);
            }
```
What is wrong: `new_size == old` reallocs move NO counter — not alloc_calls, not
dealloc_calls, not bytes. The module doc (lines 19-21) claims "realloc is implemented
explicitly ... so the call counts reflect real calls, not emulated halves" — for the
equal-size case the call counts reflect NOTHING even though a real call occurred.
Why it matters: an audited path dominated by same-size reallocs would show zero allocator
activity. Blast radius: tiny (same-size reallocs are rare); tool-only.
What correct looks like: count the equal-size case as one alloc_call with 0 delta bytes,
or document the invisibility.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CounterSet::{new,snapshot,peak_live} | 95-125 | REAL | — | const zero set; Relaxed loads (documented) |
| count_alloc / count_dealloc | 130-147 | REAL | — | i64 clamp on live; fetch_max peak |
| Snapshot::live_bytes | 163-170 | REAL | — | saturating_sub |
| snapshot() / counting_global() | 174-194 | REAL | — | global seams |
| GlobalAlloc::{alloc,alloc_zeroed,dealloc,realloc} | 203-250 | REAL | S4 | F-001 equal-size hole |
| count_growth | 255-262 | REAL | — | delta form, peak fold |
| Phase::growth | 290-296 | REAL | — | peak − baseline |
| Window::{open,open_on,close} | 305-337 | REAL | — | re-seed peak; clobber documented (no-overlap rule) |
| tests (3) | 373-429 | REAL | — | exact deltas, leak-per-set documented |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| counted_allocation_moves_counters_by_exact_size | YES | exact byte/call deltas (4096, 1) both directions |
| peak_tracks_concurrent_live_and_reseeds | YES | peak = 1024+2048 exactly; re-seed to floor |
| adjacent_windows_compose | YES | per-window call counts 1/0, 1/2 — attribution exact |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "call counts reflect real calls" (realloc) | allocator.rs:19-21 | SHALLOW | true for growth/shrink, false for equal-size (F-001) |
| "clamped at zero ... documented rather than hidden" | allocator.rs:28-32 | REAL | line 139 `.max(0) as u64` |
| unsafe blocks "exactly 'call the same method on the wrapped System'" | allocator.rs:63-72 | REAL | read: 4 forwarding blocks, no pointer arithmetic |

## 7. Dead / duplicated code
None. `Window::open` vs `open_on` both used (global vs test sets).

## 8. Dependencies & flags
std-only. `#[allow(unsafe_code)]` annotations are scoped per-impl/function (crate is
unsafe_code=deny otherwise — check Cargo.toml lane; the allow is explicit, not blanket).

## 9. Open questions
- Does walk.rs hold any allocation across window boundaries that would smear attribution?
  (W-next: audit tools/mem-audit/src/walk.rs.)
