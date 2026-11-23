# AUDIT: crates/codecs/examples/stage_bench.rs
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-23 |
| Lines / functions | 483 / 8 (main, run_file, split_frames, loadavg, timed_pass, median, minimum, ScalarIdctStages) |
| File verdict | REAL (instrument, executed) |
| Reachable from shipped entry point? | NO by design (cargo example — dev instrument, not shipped); committed release-run evidence exists in docs/perf/raw/mjpeg/ |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | executed prebuilt `./target/debug/examples/stage_bench-62f5db167ab6014a /tmp/bench_420_odd.mjpeg --passes 5` -> exit 0, validation gate passed, STAGEBENCH line printed; python3 extraction of 9 AVI MJPEG chunks (11874 bytes); read of committed evidence docs/perf/raw/mjpeg/stage-{x1a,after}-20261024.txt |

## 1. One-paragraph verdict
A real, self-validating stage-attribution benchmark. It #[path]-includes the live MJPEG decoder sources (so it measures shipped code, not a copy), refuses to report if the staged-full walk is not byte-identical to the production walk on every frame of the input (the "seam invalid" gate), interleaves ablation arms per pass, reports medians + minima + loadavg, and prints one machine-parseable STAGEBENCH line. I executed it end-to-end (exit 0, gate passed, output format exactly matches the committed release-run evidence files). Defects are hygiene-level: timed arms swallow decode errors; the frame splitter is a duplicated heuristic that can merge EOI-less chunks (my run: 9 AVI chunks -> 5 spans); on tiny/debug inputs the printed "shares" can exceed 100% with a negative residual and the tool prints them anyway.

## 2. What actually works (verified)
— verified by executing the prebuilt example on an mjpeg stream I extracted from tests/fixtures/mjpeg_h3/mjpeg_420_odd.avi: exit 0, per-pass rows, `STAGEBENCH file=/tmp/bench_420_odd.mjpeg frames=5 passes=5 I420 86x62 ... load_before=1.21 load_after=1.21`; the staged-vs-production byte-equality gate passed (no "seam invalid" error).
— verified committed release evidence: docs/perf/raw/mjpeg/stage-x1a-20261024.txt and stage-after-20261024.txt contain STAGEBENCH lines whose field names exactly match this file's format string (L420-427), with loadavg and per-pass rows — the numbers cited in perf docs come from this tool.

## 3. Findings
F-001 [S4]
Where: crates/codecs/examples/stage_bench.rs:296-331
Code: (verbatim, L296-300)
```
        full_v.push(timed_pass(|| {
            for frame in &spans {
                let _ = ctx.decode(frame);
            }
        }));
```
What is wrong: every timed arm swallows decode errors (`let _ =`). Only the full-vs-staged-FullStages equality is validated (L244-270); if a stub arm (NoIdct/NoWrite/NoIdctNoWrite) errors on a frame, that frame's cost silently vanishes from that arm and the attribution skews with no diagnostic. Confidence: HIGH — Verification: read (my execution had no errors, so output unaffected).
F-002 [S4]
Where: crates/codecs/examples/stage_bench.rs:112-194
Code: (verbatim, L112-113)
```
/// One frame's byte range in the raw stream (the huffman_bench
/// splitter, verbatim — same walk shape for the same fixtures).
```
What is wrong: duplicated splitter (self-admitted "verbatim" from huffman_bench); heuristic: AVI-MJPEG chunks without EOI make the following SOI a "stray SOI" (L140) and frames merge — my extraction produced 9 chunks but split_frames found 5 spans, so the `frames=` count and per-frame attribution silently merge frames. For a bench this is tolerable; it is not a decoder. Confidence: HIGH — Verification: ran (9 vs 5) + read.
F-003 [S4]
Where: crates/codecs/examples/stage_bench.rs:405-413
Code: (verbatim)
```
    eprintln!(
        "shares of full: parse {:.1}% | huffman {:.1}% | idct+dequant {:.1}% | store {:.1}% | \
         assemble(color/crop) {:.1}% | residual {residual:.1} ms",
```
What is wrong: no closure sanity guard — on tiny/debug inputs the deltas are noise and the printed "shares" sum > 100% with a large negative residual (my run: parse 0.2 + huffman 11.6 + idct 89.3 + store 45.6 + assemble 58.9 = 205.6%, residual -10.9 ms) and the tool emits them as-is. The honest-numbers mandate would be stronger with a warning when |residual| exceeds a tolerance of `scan`. Committed evidence is release-profile with sane shares, so no recorded number is tainted. Confidence: HIGH — Verification: ran (205.6% printed).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module includes (#[path]) | 61-105 | REAL | - | live sources; #[allow(dead_code)] scoped and honestly justified (L65-72) |
| split_frames | 119-194 | REAL | S4 | bounds-safe walks (p+3 guards); heuristic merges EOI-less frames (F-002) |
| loadavg | 196-200 | REAL | - | /proc/loadavg first field |
| ScalarIdctStages | 208-215 | REAL | - | SIMD ablation arm via ScanHooks |
| timed_pass | 219-223 | REAL | - | |
| run_file | 225-441 | REAL | S4 | validation gate real (executed); error swallowing F-001; shares F-003 |
| main | 443-483 | REAL | - | arg parsing, --passes>=3 enforced, exit 64/70 codes |

## 5. Test honesty
No tests (example). The built-in validation gate substitutes: it would fail the run (exit 70) if the seam diverged — verified by code read; not triggered in my run.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the example always measures the exact shipped code" | L44-45 | REAL | #[path] includes of ../src/video/mjpeg/* (L73-105) |
| "staged-full walk must byte-equal the production walk on every frame" | L241-243 | REAL | L244-270 gate; executed without error |
| "one STAGEBENCH summary line per file on stdout (medians + shares)" | L50-51 | REAL | my run printed it |
| stage-x1a evidence header "binary: release example from wip/p5-x1a @ 774ac5b" | docs/perf/raw/mjpeg/stage-x1a-20261024.txt:2 | REAL-UNPROVEN | output format matches this tool; that specific binary/commit not re-runnable here (UNVERIFIED, no git) |

## 7. Dead / duplicated / unreachable code
split_frames duplicated from huffman_bench (F-002). ScalarIdctStages used only when idct_simd exists — it is compiled unconditionally; fine.

## 8. Dependencies & flags
Example target of codecs crate; std only. No feature gates observed. Not in the release lane (dev instrument).

## 9. Open questions for the next agent
- huffman_bench/color_bench (same #[path] pattern) are not in my assignment — same F-001/F-002 shape likely applies there.
- The "phase3-exit gate §2 row 4a" claim (8 dead_code errors fixed by scoped allows) — verify in that gate doc if assigned.
