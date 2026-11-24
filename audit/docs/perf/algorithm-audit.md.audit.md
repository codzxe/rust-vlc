# CLAIM AUDIT: docs/perf/algorithm-audit.md (card 5.X1a)
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 10 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 10 / 0 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
Every measured number in the doc reproduces byte-for-byte from the committed raw files.
I cross-checked all four end-to-end walls, all four stage tables (medians, scalar arms,
SIMD penalties, stage percentages, even the negative residual) and the DNF refusal rows
against `docs/perf/raw/{speed-x1a-20261024.json, mjpeg/stage-x1a-20261024.txt}` — exact
matches throughout. The doc also flags its own limitations (loadavg 2-core, 5 passes,
suspects-not-attribution). This is what a measurement doc should be.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "0073 uhd4k ... lumen wall 6.619 s ... 0.158x ffmpeg, 0.239x realtime" | §1 table | REAL | speed-x1a json per_file: wall_best 6.618631 (0073), ffmpeg rows match; ratios recompute |
| 2 | "0052 3.182 s / 0124 2.444 s / 0087 1.092 s" | §1 | REAL | json: 3.1822 / 2.4437 / 1.0924 — exact |
| 3 | "assemble 63.1% / idct 23.6% / huffman 6.7% / store 5.4%" (0073) | §2 | REAL | stage-x1a txt line 1: pct_assemble=63.1 pct_idct=23.6 pct_huffman=6.7 pct_store=5.4 |
| 4 | "residual ... −20.1 ms ... negative — recorded, not hidden" (0087) | §2 | REAL | txt: residual_ms=-20.05; doc flags it in-line |
| 5 | "SIMD penalty 1.279x / 1.599x / 1.176x / 1.191x — 4/4 lanes, same direction" | §4 | REAL | txt: simd_speedup=1.279, 1.599, 1.176, 1.191 (4 lanes) |
| 6 | "end-to-end 6619 ms vs codec walk 1381 ms → gap 5238 ms (79%)" (0073) | §3 | REAL | 6.6186 json + 1380.7 txt — arithmetic exact |
| 7 | "wav/mp3/flac decode-bench → typed refusal probe.unsupported / probe.no-demuxer (exit 65)" | §5 | REAL | json: 3+ refusal rows rc=65, ok:false, dnf:true |
| 8 | "AV1: DNF — no corpus lane ... attempt recorded, killed" | §5 | REAL (honest DNF) | no av1 rows in per_file; the doc refuses to fabricate a number |
| 9 | "loadavg 1.4–4.0 during the runs" (machine row) | header | REAL | txt: load_before=2.88 / load_after=2.40 etc. — consistent range |
| 10 | "raw stdout rows preserved in docs/perf/raw/mjpeg/stage-x1a-20261024.txt and .../speed-x1a-20261024.json" | L6-8 | REAL | both files exist and contain the cited data |

## 3-5. Contradictions / fabrication / matrix rows
None found — numbers check to the byte; limitations self-flagged. The doc explicitly
says "NO fixes landed here" (fix-list only) — no overclaim of improvement. Commands:
python3 json reads; sed/cat raw txt.
