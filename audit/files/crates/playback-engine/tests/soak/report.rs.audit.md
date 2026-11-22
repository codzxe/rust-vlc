# AUDIT: crates/playback-engine/tests/soak/report.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 314 / 1 production fn (`SoakReport::markdown`) + 1 struct |
| File verdict | REAL (measurement formatter) with one arithmetic bug in a reported figure |
| Reachable from shipped entry point? | N/A (test artifact renderer) — invoked by the soak driver (`cargo test -p playback-engine --test soak` per its own footer, 308-310); the emitted markdown is the committed evidence artifact. Execution UNVERIFIED (no cargo). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:0 |
| Commands run | Read; arithmetic verification of both slope conversions by hand; sed verification. No cargo (adaptation). |

## 1. One-paragraph verdict

The soak report's markdown renderer: a pure formatter over measured facts (samples, slopes, loop counts, per-phase triage, the full series). The honesty discipline is exemplary — every per-hour figure carries an EXTRAPOLATION label, missing data renders as UNDETERMINED/UNAVAILABLE instead of a zero, warmup samples are excluded from the fit but kept in the series, and the instrument self-check documents that the preallocated sample buffer is not itself a growth term. One real defect: the VmRSS slope's kB/s→MB/h conversion divides by 1024² instead of 1024, so every report understates the RSS leak rate by three orders of magnitude.

## 2. What actually works (verified)

- Every field of `SoakReport` (14-60) is driver-measured data; the renderer invents nothing (verified: `markdown()` only formats).
- Honest refusals: `live_slope: UNDETERMINED` on <2 samples or zero spread (131-137); `VmRSS: UNAVAILABLE` without /proc (149-154); warmup counted and excluded (70-71, 92-93).
- Verdict bands (115-123): negative / within the 5 MB/h target `[PROPOSAL]` (honestly tagged as target-not-asserted, 156-161) / below-ceiling "investigate" / above-ceiling "gross leak" — the CI assertion is the ceiling, the 5 MB/h is reported only.
- The triage table ranks loop ops by mean net growth with running-sum peaks, and the handoff note requires BOTH a positive per-cycle mean and a positive measured slope (242-253) — it cannot point fingers on noise.
- The live-bytes slope conversion `slope * 3600.0 / 1e6` (114) is correct decimal MB/h.

## 3. Findings

### F-001 [S3] VmRSS slope's MB/h conversion is off by 1024×
Where: crates/playback-engine/tests/soak/report.rs:139-147
Code:
```rust
        match self.rss_slope_kb_per_s {
            Some(slope) => {
                let mb_per_h = slope * 3600.0 / 1024.0 / 1024.0;
                let _ = writeln!(
                    out,
                    "- VmRSS slope: {slope:.3} kB/s = {mb_per_h:.3} MB/h (EXTRAPOLATION, same \
```
What is wrong: kB/s → MB/h requires one factor of 1024 (kB/h ÷ 1024 = MiB/h) or 1000 for decimal MB. The code divides by 1024 × 1024, which converts kB/h to GiB/h. A slope of 1024 kB/s (≈3.5 GiB/h of RSS growth — a gross leak) is reported as "3.516 MB/h" — comfortably inside the 5 MB/h target band of the neighboring live-bytes verdict.
Why it matters: This is the number a human reads to judge RSS behavior ("includes allocator arena/fragmentation effects the live-bytes line does not", 145-146). Every committed soak artifact understates the RSS leak rate by 1024×, and a real RSS leak that stays under the mislabeled figure reads as healthy. The raw kB/s value is printed alongside (partially self-correcting for a careful reader), but the derived headline number is wrong.
What correct looks like: `let mb_per_h = slope * 3600.0 / 1024.0;` (binary MiB) or `/ 1e6` scaled consistently with the live-bytes line's decimal convention — and pick one convention for both lines.
Confidence: HIGH — Verification: read + hand arithmetic (1024 kB/s → 3,686,400 kB/h → 3,600 MiB/h; code yields 3.5156).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SoakReport` (struct) | 14-60 | REAL | — | Measured facts only; every field documented. |
| `SoakReport::markdown` | 67-313 | REAL | S3 | Method/slope/counts/triage/series sections; F-001 conversion; division guards at 166-170/269-273; honest labels throughout. |

## 5. Test honesty

No tests in this file (it is the artifact generator for the soak test — scenario.rs is the driver, my file #24). The renderer's own honesty properties are the labeled-extrapolation discipline verified in §2.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every number in the report is measured; every per-hour figure derived from a shorter window is labeled an EXTRAPOLATION" (3-5) | module doc | SHALLOW | Measured: yes. Labeled: yes. Correctly converted: no — the VmRSS MB/h figure is 1024× off (F-001), so one labeled extrapolation is numerically wrong. |
| "VmRSS from `/proc/self/status` (kernel truth, arenas included)" (87) | method section | REAL-UNPROVEN | The reading code is the soak driver's (memory.rs — not my file); the claim is standard. |
| "the sampler is not a live-bytes growth term" (279-281) | instrument self-check | REAL | Preallocation documented with reserved/used counts. |
| "the 5 MB/h figure is the target [PROPOSAL], reported, not asserted" (159-160) | report text | REAL | The ceiling is the assertion; consistent with 115-123. |

## 7. Dead / duplicated / unreachable code

None. `ranked_by_growth()` is called twice (230, 240) — trivial formatting cost, not filed.

## 8. Dependencies & flags

std only (fmt::Write); parent modules `soak::memory`/`soak::scenario` (scenario.rs is my file #24; memory.rs is another agent's). Runs under `cargo test -p playback-engine --test soak` with SOAK_MINUTES env configuration.

## 9. Open questions for the next agent

- Whoever owns the committed soak artifacts (docs/? worklog?): recompute the VmRSS MB/h figures from the printed raw kB/s — every committed number is 1024× understated (F-001).
- W03 colleague with soak/memory.rs: does the slope fit itself (least squares) live there, and is it warmup-excluded as the renderer claims?
