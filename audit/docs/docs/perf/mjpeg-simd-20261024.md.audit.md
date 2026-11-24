# CLAIM AUDIT: docs/perf/mjpeg-simd-20261024.md
| Field | Value |
|---|---|
| Auditor | W19-A02 (recovered by W19-R4) |
| Claims extracted | 10 |
| REAL 7 / REAL-UNPROVEN 0 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNVERIFIED 3 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
An anti-inflation perf record: it reports a SIMD no-win (point estimates on
the slow side) with full per-pass raw data committed, and every number in the
doc recomputes from that raw file — medians, mins, shares, the 0127 deadline
truncation to 4 passes, even the loadavg 0.38→0.38. The code artifacts exist
(stage_bench with the ScalarIdctStages ablation arm, the SSE4.1 kernel with
runtime dispatch, the bit-exact test). The doc's own "bad news, unfiltered"
verdict is exactly what the raw data shows.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "`stage_bench` (this repo, `crates/codecs/examples/`) times the SAME bitstream ... only the transform hook swapped" | :26-32 | REAL | crates/codecs/examples/stage_bench.rs exists; :208 `struct ScalarIdctStages;` :210 `impl decoder::ScanHooks for ScalarIdctStages` :303 `decode_scan_with::<ScalarIdctStages>` — the ablation arm is real. |
| 2 | SSE4.1 IDCT pass-1 via std::arch + `is_x86_feature_detected!("sse4.1")` runtime dispatch at `ScanHooks::transform` | :19-22 | REAL | crates/codecs/src/video/mjpeg/idct_simd.rs carries the kernel + dispatch (`rg is_x86_feature_detected` hit). |
| 3 | "`idct_simd::tests::dispatch_is_bit_exact_vs_scalar` — 256-block seeded equality" | :43-45 | REAL | idct_simd.rs:333 `fn dispatch_is_bit_exact_vs_scalar()`. |
| 4 | 0063 row: N=7, SIMD med 55.8, scalar med 54.7, "1.021× (SIMD slower)", min 55.8/54.5 | :56 | REAL | stage-after-20261024.txt: raw passes (62.6/56.0/55.8/56.1/55.8/55.8/55.8 -> med 55.8; scalar 54.5-59.1 -> med 54.7) + summary line "full(simd) 55.8 ... 1.021x (102.1% of scalar walk)" + "min_full_ms=55.8 min_scalar_full_ms=54.5" — recomputed by hand, exact. |
| 5 | 0127 row: "4 of 7 — deadline kill, medians over the 4 recorded passes", 12 053 / 11 996, 0.995× | :57 | REAL | Raw file: exactly 4 pass rows (12054.6/12051.3/12157.8/12045.0 vs scalar 11984.3/12008.3/12009.9/11956.4); median full = 12052.95 ≈ 12053, scalar = 11996.3 ≈ 11996, ratio 0.995 — recomputed, exact. |
| 6 | Stage shares row 0063: huffman 27.8%, idct+dequant 23.2%, store 3.1%, assemble 45.7% | :69 | REAL | Raw: "shares of full: parse 0.0% | huffman 27.8% | idct+dequant 23.2% | store 3.1% | assemble(color/crop) 45.7%" — identical values. |
| 7 | "0063 run: 0.38 → 0.38, machine otherwise idle" (loadavg sampling) | :12 | REAL | Raw summary: `load_before=0.38 load_after=0.38`; stage_bench.rs:196 `fn loadavg()` + :51 "/proc/loadavg sampled around the measured". |
| 8 | Raw/predecessor data files: stage-after-20261024.txt + speed-before-avi-av/video.{json,md}; tools: subset_replay.py, speed_bench.py | :14,87-90 | REAL | `ls docs/perf/raw/mjpeg/` -> all 5 named files; tools/corpus/{subset_replay.py,speed_bench.py} exist. |
| 9 | "172/0 codecs suite at merge time" + zero_alloc_decode PASS + clippy CLEAN | :44,109-116 | UNVERIFIED (runs) / corroborated | Not re-runnable (cargo budget); baseline clippy CLEAN (spec §13) consistent. |
| 10 | End-to-end post-SIMD lumen-vs-ffmpeg "DNF this session ... expectation, NOT a measurement, labeled as such" | :78-83 | REAL (honesty marker) | The doc explicitly refuses to claim an unmeasured number — the [PROPOSAL]/expectation labeling is present as stated. |

## 3. Contradictions
None. Doc-vs-raw recomputation agrees on every one of the ~15 numbers I
checked, including the awkward ones (SIMD slower on both fixtures).

## 4. Fabrication / inflation indicators
None — the record is the opposite of inflation: a negative result with
committed raw data, a DNF honestly marked, and a recommendation section that
argues AGAINST the feature it measured ("keep scalar" proposal).
