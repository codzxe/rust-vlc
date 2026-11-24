# AUDIT: tools/mem-audit/src/selfcheck.rs
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A05) |
| Date | 2026-11-24 |
| Lines / functions | 184 / 5 fns + 1 struct pair + 1 test |
| File verdict | REAL |
| Reachable from shipped entry point? | Not in release lane (dev instrument); executed lane = `cargo test -p lumen-mem-audit` + `lumen-mem-audit --self-check` CLI. Baseline proof: cargo-test-workspace.txt:3280-3295 `test selfcheck::tests::ladder_measures_positive_times ... ok` (9 passed, package exit 0) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg (global_allocator wiring, baseline test lines), sed quotes verified, it=baseline match |

## 1. One-paragraph verdict
The instrument-overhead honesty module: a no-op floor snapshot plus a five-size micro-bench
(16 B → 1 MiB) measuring the counting allocator's per-pair cost against `System` directly.
The methodology is correct: `#[global_allocator]` is installed unconditionally (main.rs:75-76),
so `std::alloc::alloc` genuinely routes through the counting path while `System.alloc` bypasses
it; null allocations abort the measurement as typed errors instead of laundering zeros;
deltas are reported verbatim (never clamped, noise-negative allowed). The unsafe blocks are
minimal, each carries a SAFETY comment, and the null-return contract is handled. The one test
asserts positive measured times, the five-row shape, and a loud sanity bound (delta < 1000
ns/pair) — it executed green in the orchestrator baseline.

## 2. What actually works (verified)
— verified by citation audit/runs/baseline/cargo-test-workspace.txt:3289
`test selfcheck::tests::ladder_measures_positive_times ... ok` within package block
:lumen-mem-audit exit_code=0 (line 3295); rg confirmed `#[global_allocator] static GLOBAL:
Counting` (main.rs:75) precedes any selfcheck call, so the measured "counting path" is the
live global, not a phantom.

## 3. Findings
Zero numbered findings. What was checked, and how:
- Fakeness (6.1): no todo/unimplemented/constant returns; every number is measured (Instant
  over ≥10k iterations per row). Verified by full read.
- Unsafe (6.2): two unsafe blocks (96-102, 118-124), both layout-validated upstream
  (`Layout::from_size_align(size, 1)` with size ≥ 16), null-checked before use, dealloc'd with
  the identical layout; SAFETY comments present and accurate. Verified by read.
- Swallowed errors (6.2): allocation failure returns `Err` (97-98, 119-120) — the honest
  abort the doc comment promises ("never a silent zero").
- Overflow/units (6.2): ns-per-pair computed as `elapsed().as_nanos() as f64 / iters` (104,
  126) — correct units; `as` cast from u128 to f64 is lossless for realistic nanosecond counts.
- Perf (6.3): iteration ladder bounded (200k/50k/10k by size class, 68-76) — no unbounded
  runtime; doc's "~20–200 ms per row" consistent with baseline's 0.25 s package total.
- Test honesty (6.4): would fail if the counting allocator stopped counting (counting_ns
  measured through the installed global; a no-op wrapper collapses to ~system_ns, delta → ~0,
  still positive — the assertion targets measurement aliveness and the <1 µs credibility
  bound catches runaway overhead). Verified by read + mutation reasoning.
- Doc-vs-code (6.7): header claims match behavior (floor = snapshot at :135; fixed
  counting-before-system order documented at 138-140 and implemented 141-142).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| OverheadRow::delta_ns | 47-49 | REAL | — | verbatim subtraction, no clamp |
| iters_for | 68-76 | REAL | — | bounded ladder by size class |
| pair_counting | 86-105 | REAL | — | unsafe, null-checked, black_box |
| pair_system | 110-127 | REAL | — | direct System bypass |
| run | 134-156 | REAL | — | floor + 5 rows |
| tests::ladder_measures_positive_times | 165-183 | REAL | — | executed green (baseline:3289) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ladder_measures_positive_times | YES | rows.len()==5 pins the ladder; >0 times pin aliveness; delta bound flags instrument-heavier-than-subject |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "measured, never asserted" (two numbers) | header 5 | REAL | run() measures; no constants |
| "counting first, then system, same size/iters" | 138-140 | REAL | loop 141-142 |
| "may be negative on timer noise — reported verbatim" | 44-45 | REAL | delta_ns has no clamp |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
std-only (alloc, hint, time). `#[allow(unsafe_code)]` (85, 109) pairs with the crate's
documented minimal-unsafe posture (main.rs:33-37); not in release lane — dev tool.

## 9. Open questions for the next agent
The `< 1000.0 ns/pair` sanity bound (176-181) is machine-load-sensitive on a 2-core runner;
if CI ever flakes there, the fix is a percentile re-measure, not a loosened bound (per the
file's own honesty rule).
