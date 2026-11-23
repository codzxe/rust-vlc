# AUDIT: crates/playback-engine/tests/soak/memory.rs
| Field | Value |
|---|---|
| Auditor | W03-A07 |
| Date | 2026-11-23 |
| Lines / functions | 356 / 16 (11 instrument fns/types + 5 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test-instrument module of the soak test binaries (tests/soak/main.rs; shared into tests/soak_scenarios/main.rs via `#[path = "../soak/memory.rs"]`, main.rs:51-52 of that binary). It measures the REAL engine under soak (the counting allocator and /proc are real system facts), but ships in no binary. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Direct run `./target/debug/deps/soak-8f11c7047e2d0b88 memory::tests` -> 5 passed / 0 failed (exit 0); baseline audit/runs/baseline/cargo-test-workspace.txt:4492-4497 (same 5 ok inside the soak binary's 12-test run); sed/rg verifications; hand-check of the least-squares cancellation margins (see §2). |

## 1. One-paragraph verdict
Honest measurement instruments, competently built: VmRSS read from /proc/self/status with every miss mapped to `None` ("an absent instrument is reported, never faked" — and it is), a least-squares slope that refuses undetermined windows (<2 points, zero spread) instead of inventing a line, a fixed-size per-phase leak-attribution accumulator whose triage cannot itself be a growth term, and a wall-clock cadence sampler that re-arms from the schedule (no drift) and pre-reserves its series so sampling does not pollute the counting allocator it feeds. The extrapolation-to-MB/h is labeled as an extrapolation at every consumer. The math is sound at real soak magnitudes (I hand-checked the catastrophic-cancellation margins for a 4 h / 57,600-sample window: relative error ~1e-15, harmless). Two hygiene nits: a phase that never ran reports mean 0.0 — numerically indistinguishable from a measured zero — and the slope's zero-spread guard compares an absolute denominator to `f64::EPSILON`, a dimensionally meaningless threshold that happens to work because the x-axis is seconds.

## 2. What actually works (verified)
— `./target/debug/deps/soak-8f11c7047e2d0b88 memory::tests` -> `test result: ok. 5 passed; 0 failed` (exit 0). Verified by read: (1) `read_vm_rss_kb` (59-64) returns None on any of read/line/parse miss — no zero-fabrication; (2) the slope refusal cases are pinned by test 3 (empty, single point, zero spread); (3) noisy-trend recovery within 5% pinned by test 2; (4) `PhaseAccumulator::record` (158-163) uses saturating arithmetic and tracks a running-sum high-water so transient megabytes are visible as peaks, not growth — pinned by test 4 including the negative-delta (close frees) case; (5) `CadenceSampler::sample`'s re-arm loop (244-246) is bounded by stall/interval and keeps timestamps honest; (6) cancellation margin for n*Σt² − (Σt)² at 4 h × 250 ms cadence ≈ 5.7e16 with terms ≈ 2.3e17 — f64 relative error 1e-16 gives ~23 absolute error, i.e. ~4e-16 relative: the naive formula is adequate; (7) `vm_rss_reads_on_proc_platforms` asserts kb > 0 on Linux (this host) and only prints on absence — on this platform it is a real assert (passed).

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/soak/memory.rs:167-174
Code:
```
    pub fn mean_per_cycle(&self, phase: Phase) -> f64 {
        let index = phase as usize;
        let cycles = self.cycles[index];
        if cycles == 0 {
            return 0.0;
        }
        self.deltas[index] as f64 / cycles as f64
    }
```
What is wrong: a phase that never ran (0 cycles) reports mean 0.0 — the same number as a phase that ran and measured exactly zero net growth. The triage report can then present "track-switch: 0.0 B/cycle" for a phase the scenario never exercised, implying a clean measurement where there was none. Blast radius: report wording only (the ranked ordering puts unrun phases mid-pack). What correct looks like: return `Option<f64>`/NaN for 0 cycles and let the report say "not exercised". Confidence: HIGH — Verification: read.
Confidence: HIGH — Verification: read

F-002 [S4]
Where: crates/playback-engine/tests/soak/memory.rs:86-89
Code:
```
    let denominator = n * sum_tt - sum_t * sum_t;
    if denominator <= f64::EPSILON {
        return None;
    }
```
What is wrong: the guard compares an absolute, dimensionful quantity (n·Σt² − (Σt)², units: seconds²) against `f64::EPSILON` (2.2e-16, a dimensionless relative-precision constant). Whether a window is "zero spread" should be judged in the x-axis's units; with x in seconds the threshold effectively means "spread below ~15 ns refuses", which is accidentally reasonable, but any reuse with, e.g., cycle-count or byte axes silently changes the refusal boundary by orders of magnitude. What correct looks like: a relative test (`denominator <= spread_tolerance * n * sum_tt`) or a documented absolute floor in seconds. Confidence: HIGH — Verification: read + hand derivation (n=2, |t1−t2| = ε ⇒ denominator = ε²; refusal iff ε² ≤ 2.2e-16 ⇒ ε ≤ 1.5e-8 s).
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Sample (struct) | 38-51 | REAL | — | rss Option — absent ≠ zero |
| read_vm_rss_kb | 59-64 | REAL | — | bounded single-read; every miss → None |
| least_squares_slope | 70-91 | REAL | S4 | naive formula adequate (hand-checked); EPSILON guard dimensionally sloppy (F-002) |
| Phase (enum) | 97-113 | REAL | — | the 5 loop ops |
| Phase::ALL / label | 117-135 | REAL | — | stable labels |
| PhaseAccumulator (struct) | 145-153 | REAL | — | fixed-size: deltas/cycles/peaks |
| PhaseAccumulator::record | 158-163 | REAL | — | saturating; high-water after add |
| PhaseAccumulator::mean_per_cycle | 167-174 | REAL | S4 | 0.0 for never-run phase (F-001) |
| PhaseAccumulator::peak | 178-180 | REAL | — | — |
| PhaseAccumulator::ranked_by_growth | 185-193 | REAL | — | NaN-safe ordering |
| CadenceSampler (struct) | 201-207 | REAL | — | preallocated series |
| CadenceSampler::new | 213-222 | REAL | — | interval ≥ 1 ms; capacity + 16 slack |
| CadenceSampler::elapsed_s | 226-228 | REAL | — | — |
| CadenceSampler::ticked | 234-236 | REAL | — | inline, no second thread |
| CadenceSampler::sample | 239-254 | REAL | — | schedule-based re-arm (no drift) |
| CadenceSampler::samples / reserved_capacity | 258-267 | REAL | — | capacity proof for the report |
| tests: slope_math_is_exact_on_clean_lines | 277-294 | REAL | — | flat/rising/falling exact; ran ok |
| tests: slope_math_recovers_noisy_trends | 298-314 | REAL | — | ±50 square wave vs 1000 B/s; ran ok |
| tests: slope_refuses_undetermined_windows | 317-322 | REAL | — | 3 refusal shapes; ran ok |
| tests: phase_accumulator_ranks_and_means | 326-341 | REAL | — | means, ranking, frees-last, peak; ran ok |
| tests: vm_rss_reads_on_proc_platforms | 344-355 | REAL | — | real assert on Linux (passed here) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| slope_math_is_exact_on_clean_lines | YES | exact slopes asserted within 1e-6 |
| slope_math_recovers_noisy_trends | YES | 5% tolerance on a known 1000 B/s trend |
| slope_refuses_undetermined_windows | YES | pins the None refusals |
| phase_accumulator_ranks_and_means | YES | exact means/ordering/peak incl. negative deltas |
| vm_rss_reads_on_proc_platforms | YES on Linux (vacuous elsewhere, disclosed) | kb > 0 assert; None path only prints — documented platform limitation |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "an absent instrument is reported, never faked" | memory.rs:16-17 | REAL | read_vm_rss_kb:60-63 all-miss → None; vm_rss test |
| "a monotonic climb here IS a leak by definition" (live_bytes) | memory.rs:19-21 | REAL (definition) | live_bytes is requested-minus-freed from the counting allocator — monotone climb = retention; instrument real (testkit alloc_count) |
| "Extrapolating a short window ... is an EXTRAPOLATION and every consumer ... labels it so" | memory.rs:27-30 | REAL (this file) | slope returns B/s; MB/h conversion + labeling live in the report layer (soak/report.rs — spot-checked naming in baseline output) |
| "No unsafe, std-only, no panics on missing data" | memory.rs:32 | REAL | rg: no unsafe in file; Options throughout; the only panics are in tests via .must() |

## 7. Dead / duplicated / unreachable code
- None. Every instrument fn is consumed by the soak runners (tests/soak/main.rs and the shared soak_scenarios binary; baseline runs both).
- No #[ignore], no todo!/unimplemented! (rg verified).

## 8. Dependencies & flags
std only (fs, time::Instant) + testkit::Must in tests. Linux-leaning (/proc) with an honest non-Linux degradation. Compiled into two test binaries via module include; no feature gates.

## 9. Open questions for the next agent
- soak/report.rs + runner.rs: confirm the MB/h extrapolation really is labeled at every consumer as claimed (report layer is another agent's file unless assigned to me — it is not).
- The PhaseAccumulator's attribution reads live_bytes at phase boundaries — a leak that only manifests mid-phase (e.g., during seek ticks) is attributed to whatever phase bracketed it; acceptable, but the report should say "attributed at boundaries" (wording check for the report agent).
