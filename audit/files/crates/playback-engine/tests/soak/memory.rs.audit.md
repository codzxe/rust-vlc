# AUDIT: crates/playback-engine/tests/soak/memory.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 356 / 8 production items + 5 unit tests |
| File verdict | REAL — the soak's measurement instruments (VmRSS reader, least-squares slope, per-phase leak attribution, wall-cadence sampler), std-only, honest-None discipline, with exact unit tests for the math |
| Reachable from shipped entry point? | N/A (soak instrument module, consumed by tests/soak/{dispatch,scenario}.rs — other agents' files; the 4 h soak itself is CI-nightly, soak-nightly.yml:140) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read/Grep (not executed — no cargo) |

## 1. One-paragraph verdict

Correct, careful instrumentation. `read_vm_rss_kb` parses
`/proc/self/status` with every miss an honest `None` (:59-64).
`least_squares_slope` refuses undetermined windows (< 2 points, zero
spread) with `None` (:70-91) and its unit tests pin exact slopes on
clean lines, trend recovery under ±50 noise, and the three refusals
(:277-322). `PhaseAccumulator` is the substitute for the
per-category allocator the workspace does not have — growth is
attributed to the loop phase (open/play/seek/switch/close) at whose
boundaries it happened, with a running-sum peak so transient
allocations are visible as peaks, not growth (:145-194). The
`CadenceSampler` pre-reserves its series so the instrument is not a
term in its own measurement (:196-268) and re-arms from the schedule
so a slow loop cannot drift the cadence (:241-246). Inline sampling,
no second thread — the counting allocator stays unpolluted (:230-232).

## 2. What actually works (verified)

— The slope math tests are exact-value (flat < 1e-9, rising/falling
< 1e-6, noisy within 50 of 1000) — they would fail on a formula bug.
— `PhaseAccumulator` tests pin means, ranking, and the running peak
(:327-341).
— `vm_rss_reads_on_proc_platforms` does not fake success on non-proc
platforms — it prints a visible note instead (:343-355).
— The instrument-not-a-growth-term discipline (reserved capacity,
fixed-size accumulator) is asserted/documented at :143-144, :262-267.

## 3. Findings

### F-001 [S4] Least squares without centering: numerically fragile for long windows
Where: memory.rs:70-91

Code:
```
    let denominator = n * sum_tt - sum_t * sum_t;
    if denominator <= f64::EPSILON {
        return None;
    }
    Some((n * sum_ty - sum_t * sum_y) / denominator)
```
What is wrong: the textbook uncentered normal equation. With t in
seconds over a multi-hour soak (t up to ~14 400, n ~ 14 400),
`sum_tt` ~ n·t² ~ 3e12 and `n·sum_tt − sum_t²` involves subtracting
two ~3e12-magnitude f64s — catastrophic cancellation territory; the
guard only catches a non-positive result, not a precision-degraded
one. In practice the true spread keeps the denominator far from zero
for a full soak, so results will be fine; a short window with large
t offsets (e.g., a soak resumed with t continuing from 14 400) could
lose significant digits silently.
What correct looks like: center first (`t − t̄`, `y − ȳ`), then slope
= Σ(t′y′)/Σ(t′²) — the numerically stable form; the same tests keep
passing.
Confidence: MEDIUM — numerical analysis; not executed.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Sample` | 34-51 | REAL | — | carries warmup flag for honest reporting |
| `read_vm_rss_kb` | 58-64 | REAL | — | honest None on any miss |
| `least_squares_slope` | 69-91 | REAL | S4 | uncentered normal equations (F-001) |
| `Phase` enum + label | 97-136 | REAL | — | the category axis |
| `PhaseAccumulator::{record,mean_per_cycle,peak,ranked_by_growth}` | 145-194 | REAL | — | fixed-size; peaks tracked |
| `CadenceSampler::{new,elapsed_s,ticked,sample,samples,reserved_capacity}` | 201-268 | REAL | — | schedule re-arm; reserved series; inline (no thread) |
| in-file tests (5) | 277-355 | REAL | — | exact math pins; honest platform note |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `slope_math_is_exact_on_clean_lines` | YES | exact slopes |
| `slope_math_recovers_noisy_trends` | YES | ±50 noise vs 1000 B/s truth |
| `slope_refuses_undetermined_windows` | YES | the three refusals |
| `phase_accumulator_ranks_and_means` | YES | means/ranking/peak pinned |
| `vm_rss_reads_on_proc_platforms` | PARTIAL | a parse regression fails only on /proc platforms; the None arm prints, does not fail |

Not executed by me (no cargo); W00 ran the suite green.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every #[global_allocator] in the tree is testkit's CountingGlobalAlloc" | :8-9 | REAL | consistent with the three alloc binaries audited |
| "an absent instrument is reported, never faked" | :19-20 | REAL | Option plumbing throughout |
| "Extrapolating a short window … is an EXTRAPOLATION and every consumer … labels it so" | :30-32 | UNVERIFIED | consumers are soak/dispatch.rs (other agent) |
| "no per-category allocator" | :8-10 | REAL | PhaseAccumulator is the stated substitute |

## 7. Dead / duplicated / unreachable code

None.

## 8. Dependencies & flags

std-only (fs, time::Instant) + testkit (Must) in tests. No feature
gates. Linux-leaning (`/proc`) with honest degradation.

## 9. Open questions for the next agent

- Wave 16 / soak owners (tests/soak/dispatch.rs, scenario.rs): does
  the consumer label the MB/h extrapolation as promised, and does the
  nightly soak assert a slope bound (or only report)?
