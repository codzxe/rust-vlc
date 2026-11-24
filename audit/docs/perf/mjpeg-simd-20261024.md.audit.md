# CLAIM AUDIT: docs/perf/mjpeg-simd-20261024.md
| Field | Value |
|---|---|
| Auditor | W19-A02 |
| Claims extracted | 8 concrete claims |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 5 REAL / 0 / 0 / 0 / 0 / 1 FALSE number (1.18×) / 2 (re-run; clippy/zero_alloc gates) |

## 1. Blunt verdict
An honest negative-results record: "the SSE4.1 pass-1 IDCT is a NO-WIN" — and its
§4.1/§4.2 numbers reproduce from the committed raw file to the millisecond. Two defects:
the pre-SIMD avi-av ratio "1.18×" matches nothing in its own cited raw data (which says
1.34×), and "8-file medians" for the 0.30× avi-video claim hides that lumen's median is
over 5 files — the other 3 FAILED to decode (lumen `ok: false` in the raw JSON).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "SIMD vs scalar … 0063: full SIMD med 55.8 ms, scalar 54.7, speedup 1.021× (slower)" | §4.1 | REAL (EXECUTED — recomputed) | raw/mjpeg/stage-after-20261024.txt: `STAGEBENCH … med_full_ms=55.8 med_scalar_full_ms=54.7 simd_speedup=1.021 … load_before=0.38 load_after=0.38` — byte-exact match incl. the load rows |
| 2 | "0127 … 4 of 7 passes … 12 053 ms vs 11 996 ms → 0.995×" | §4.1 | REAL (EXECUTED — recomputed) | raw file 0127 passes 0-3 full [12054.6,12051.3,12157.8,12045.0] scalar [11984.3,12008.3,12009.9,11956.4]; medians 12052.95/11996.3 → ratio 0.995 — matches |
| 3 | stage shares: huffman 27.8%, idct 23.2%, store 3.1%, assemble 45.7% | §4.2 | REAL | raw: `pct_huffman=27.8 pct_idct=23.2 pct_store=3.1 pct_assemble=45.7` exact |
| 4 | `dispatch_is_bit_exact_vs_scalar` — 256-block seeded equality | §3 | REAL | crates/codecs/src/video/mjpeg/idct_simd.rs:333 `fn dispatch_is_bit_exact_vs_scalar()` exists |
| 5 | SSE4.1 dispatch via `is_x86_feature_detected!`, scalar fallback, stage_bench ablation | §1-2 | REAL | idct_simd.rs:4 + :75; crates/codecs/examples/stage_bench.rs exists |
| 6 | "pre-SIMD … 1.18× on avi-av" | §4.3 | FALSE (vs cited raw) | docs/perf/raw/mjpeg/speed-before-avi-av.md table: `ratio vs ffmpeg 1.34x` (ratio-of-medians 0.2443/0.1829 = 1.335, python3). No derivation in the raw files yields 1.18 (median-of-per-file-ratios = 1.19; fps ratio 1.22). The doc says "cited unchanged" — it is not |
| 7 | "0.30× ffmpeg on avi-video (8-file medians, best-of-1)" | §4.3 | REAL number / FALSE framing | speed-before-avi-video.md: lumen `n=5` (not 8!) med wall 1507.8 ms vs ffmpeg 447.7 ms → 0.297 ≈ 0.30× ✓; but the raw JSON shows 3 of 8 avi-video files have lumen `ok: false` (0011/0019/0027 failed) — "8-file medians" is wrong; it is a 5-survivor median |
| 8 | raw artifacts + subset_replay.py committed | §5 | REAL | ls raw/mjpeg/: stage-after-20261024.txt, speed-before-avi-av.{json,md}, speed-before-avi-video.{json,md}, subset_replay.py — all present |

## 3. Findings
F-001 [S3] — the 1.18× pre-SIMD avi-av ratio contradicts its own raw data.
Where: doc §4.3 vs raw/mjpeg/speed-before-avi-av.md (table row "1.34x").
What is wrong: a number labeled "cited unchanged" that no raw-file derivation produces.
Why it matters: understates a favorable result in one place while the aggregate doc says
1.34× — the program's own "honest numbers" rule fails at the citation level. Confidence: HIGH.
F-002 [S3] — "8-file medians" hides 3 decode failures.
Where: doc §4.3 vs raw JSON (lumen ok:false on media/0011, 0019, 0027).
What is wrong: lumen's 0.30× is a median over the 5 files lumen could decode; the 3
failures are silently excluded from the sentence (the raw .md does show n=5). Confidence: HIGH.
F-003 [S4] — gates cited but not re-run (cargo-build-budget; toolchain path found late —
no re-run attempted here): `cargo test -p codecs --test zero_alloc_decode` PASS and clippy
CLEAN remain UNVERIFIED by me; baseline clippy CLEAN (audit/runs/baseline/cargo-clippy.txt)
covers the workspace claim at the orchestrator's earlier date.

## 4. Contradictions
| Doc says | Raw says | True |
|---|---|---|
| 1.18× on avi-av | 1.34× (ratio-of-medians) | raw |
| 8-file medians (avi-video) | lumen n=5, 3 files ok:false | raw |

## 5. Fabrication / inflation indicators
The core verdict (SIMD no-win, "these are the numbers, unfiltered") is TRUE and raw-backed.
The §4.3 pre-SIMD citations are the soft spot: one unreconcilable number and one
survivorship-framing — both flattering-to-nobody, but neither honest citation practice.
