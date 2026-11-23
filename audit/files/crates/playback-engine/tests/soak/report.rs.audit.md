# AUDIT: crates/playback-engine/tests/soak/report.rs
| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-23 |
| Lines / functions | 314 / 1 (`SoakReport::markdown`, plus the 22-field struct) |
| File verdict | REAL-UNPROVEN — the renderer executes in every soak run (printed + optional file), but NOTHING asserts its output; unit conversions verified by read only |
| Reachable from shipped entry point? | Test lane: tests/soak/main.rs:53 `mod report;`, :324-353 builds `SoakReport` and prints/writes `markdown()`. Runs in the baseline soak (`soak_loops_stay_memory_bounded ... ok`, 120.04s). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | None possible post disk-full (see log). Evidence: baseline cargo-test-workspace.txt:4482-4497 (soak 12 passed incl. the report-printing run) + read-verification of every formula. |

## 1. One-paragraph verdict
A pure markdown renderer over measured fields — no measurement happens here (all 22 fields are injected by main.rs from instruments; verified by reading main.rs:324-346). The formulas are correct by read (B/s × 3600 / 1e6 = MB/h at line 114; kB/s × 3600 / 1024² = MB/h at 141; mean cycle ms at 166-170), the EXTRAPOLATION labeling promise is kept (lines 126-128, 144-146), and the 5 MB/h target is honestly labeled "[PROPOSAL] ... reported, not asserted" (158-160) — matching the phase-3 gate doc's admission that the 4 h baseline run never happened. The defect: zero test coverage — unlike the sibling soak_scenarios crate's own report.rs (which has three unit tests in baseline), this renderer's output is never asserted anywhere; a unit-conversion regression would mislead every human reader while all machine assertions stay green.

## 2. What actually works (verified)
— Nothing executed directly (disk full). Indirect: the baseline soak run (`soak_loops_stay_memory_bounded ... ok`, 120.04s, cargo-test-workspace.txt:4493-4497) constructs and renders this report via main.rs:347-353 without panicking. Formula verification: READ (both conversions recomputed by hand).

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/soak/report.rs:67-313 (whole renderer)
Code:
```
    pub fn markdown(&self) -> String {
        let mut out = String::with_capacity(16 * 1024);
        let total = self.samples.len();
        let measured: Vec<&Sample> = self.samples.iter().filter(|s| !s.warmup).collect();
```
What is wrong: no test asserts any part of the rendered markdown — no golden file, no content check, not even a "contains the slope line" smoke test in this crate. The machine-checked soak assertions (slope fit, ceiling, liveness) all live in main.rs; this renderer is display-only, so a regression in its unit math (e.g. swapping the 1e6 for 1e3 at line 114) or verdict thresholds (115-123) would silently corrupt every emitted report while the suite stays green. The sibling crate got this right: tests/soak_scenarios has `report::tests::{civil_dates_match_known_days, utc_timestamp_has_the_documented_shape, worst_slope_headline_prints_the_largest_finite_slope}` (all ok in baseline). What correct looks like: the same three-or-four shape tests here (slope line formatting, verdict thresholds, series row count). Confidence: HIGH — Verification: read (rg `#[test]` in file: zero) + baseline citation.
F-002 [S4]
Where: crates/playback-engine/tests/soak/report.rs:230, 240
Code:
```
        for (rank, phase) in self.phases.ranked_by_growth().into_iter().enumerate() {
```
What is wrong: `ranked_by_growth()` (a sort) computed twice for the same immutable accumulator (again at line 240 for `worst`). Cold path, trivial cost — hygiene only. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SoakReport (struct, 22 fields) | 14-60 | REAL | — | all fields instrument-sourced (main.rs:324-346); docs claim "nothing here is invented" — verified by reading every field's producer |
| SoakReport::markdown | 67-313 | REAL-UNPROVEN | S4 | formulas correct by read; output never asserted (F-001); duplicated ranking (F-002) |

## 5. Test honesty
No tests in this file (F-001). The renderer's only execution is inside `soak_loops_stay_memory_bounded` (baseline ok), which asserts slopes/counts — not the rendered text.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every number in the report is measured; every per-hour figure derived from a shorter window is labeled an EXTRAPOLATION" | report.rs:1-5 | REAL | fields injected from instruments (main.rs:324-346, read); EXTRAPOLATION labels at 126-128 and 144-146 (read) |
| "the 5 MB/h design target is [PROPOSAL] — reported, never asserted" | docs/gates/phase3-exit.md:34 | REAL | report.rs:117-120, 158-160 wording matches; assertion in main.rs uses only the CI ceiling (main.rs:32-36 per the doc; read-confirmed the constant names) |
| "the 4 h nightly run has not been executed in this sandbox; no 4 h report exists on disk" | docs/gates/phase3-exit.md:34 | REAL-UNPROVEN (accepted) | no `SOAK_MINUTES=240` artifact found under docs/ (rg for SOAK artifacts found only the 20261109 nightly note + scenarios reports); honest PENDING marker |
| "the sampler is not a live-bytes growth term" | report.rs:279-281 | REAL-UNPROVEN | self-check line prints reserved vs used slots; the preallocation claim depends on memory.rs's sampler (not my file; baseline `memory::tests` ok) |

## 7. Dead / duplicated / unreachable code
None dead. `report.rs` of tests/soak_scenarios is a separate, test-covered renderer for the other soak system — near-duplicate purpose, different crate (noted for the duplication ledger; cross-crate, acceptable).

## 8. Dependencies & flags
std only (`fmt::Write`), super::memory (PhaseAccumulator, Sample), super::scenario::CycleCounts. `#[must_use = "the artifact"]` on markdown. No unsafe, no features. `let _ = writeln!(...)` on String writes — infallible, fine.

## 9. Open questions for the next agent
- soak/main.rs (not my file): confirm the CI ceiling constant and that `report_path()` (SOAK_REPORT) is the only file-write side effect.
- If a committed in-crate soak report artifact exists anywhere (docs/perf/), diff its numbers against a fresh `SOAK_MINUTES` run when disk allows — currently blocked (disk full).
