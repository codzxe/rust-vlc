# CLAIM AUDIT: docs/perf/raw/summary.json
| Field | Value |
|---|---|
| Auditor | W19-A02 |
| Claims extracted | 30 measurement entries (raw artifact) |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 0 / 0 / 0 / 0 / 1 (raw data record) / 0 / 1 (re-run) |

## 1. Blunt verdict
Raw aggregate for the 3.F1 perf baseline card. Cross-checked against
docs/perf/baseline.md's memory table — every median/max matches exactly, and the
unflattering entries (mp4 decode-bench = exit-65 refusal; mp4 session = refusal path)
are present rather than filtered out.

## 2. Claim ledger
| # | Claim (entry) | Verdict | Evidence |
|---|---|---|---|
| 1 | peak RSS medians: help 3224, pcm 3532, 2pcm 3380, mp4-refusal 3000, sessions 12356/11796 KiB (n=5) | REAL (matches consumer) | python3 over summary.json == baseline.md L151-158 table, cell-for-cell (e.g. `decode-bench:mp4_mjpeg_pcm.mp4 [exit65-refusal]` → median 3000.0 / max 3124.0) |
| 2 | decode_ms n=12: pcm bench_decode_s median 7.0, 2pcm 3.0; per_frame_ms_derived 0.117/0.15 | REAL (internal) | consistent: 2pcm per_frame 0.15 ms × 20 frames = 3.0 s bench; pcm 0.117 × 60 = 7.02 s ≈ median 7.0 ✓ (python3 arithmetic) |
| 3 | startup_ms: --version/warm 0.537 ms, ready(engine-info)/warm 0.546 ms (n=25) | REAL (internal) | entries present with plausible shape (cold-fadvise 6.706 > warm 0.537) |
| 4 | seek_ms n=20: seek1/seek2 response ≈ 5.34-5.35 ms, open_response ≈ 1.03 ms | REAL (internal) | seek1_event≈seek1_response≈seek2 pairs — self-consistent |
| — | (whole artifact re-measured) | UNVERIFIED | benchmarks not re-runnable in this sandbox; nothing contradicts, nothing re-executed |

## 3. Contradictions
None internal. Historical note: the mp4 refusal entries (exit 65) predate the MP4 demux
landing; the newer baseline mp4-smoke (2026-11-23) plays the file — different builds,
both honestly labeled. summary.json's own [exit65-refusal] label keeps the record honest.

## 4. Fabrication / inflation indicators
None — a refusal path carrying the same n=5 and full stats as the success paths is the
opposite of cherry-picking.
