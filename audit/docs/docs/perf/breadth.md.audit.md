# CLAIM AUDIT: docs/perf/breadth.md (217 lines, Phase-5 breadth perf snapshot, card 5.R1)
| Field | Value |
|---|---|
| Auditor | W19-A09 — RECOVERED by W19-R2 |
| Claims extracted | 12 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 10 / 0 / 0 / 0 / 0 / 1 (S4 mislabel) / 1 |
| Internal contradictions found | 0 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |

## 1. Blunt verdict
The doc's subtitle should be "perf-vs-FFMPEG" and it SAYS so in its own oracle row
("VLC 3.0.23 is absent from this sandbox … ffmpeg is the speed oracle everywhere") —
no bait-and-switch. Every number I could recompute from the committed raw CSVs/
transcripts reproduces exactly: seek medians 102.10/100.70/404.35/705.30 ms digit-exact,
probe 2.48–2.49 ms digit-exact, FLAC 46 421 260 samples/s byte-exact, mkv demux
4.77 ms / 53 494 pkt/s / 773.2 MiB/s verbatim, ffmpeg walls 0.430/0.410 from
ffmpeg-lanes.csv medians. One S4 mislabel: §9 says mjpeg-decode-bench.csv has "25
rows"; the file has 20 (4 lanes × 5 runs, headerless).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "ffmpeg 7.1.5-0+deb13u1 — VLC 3.0.23 is absent … the harness refuses without it" | :17 | REAL | run.py:950-952 `except OracleUnavailable … return 2` (verified in my PERF-VS-VLC ledger); oracle-run json records ffmpeg 7.1.5 as the regeneration tool. |
| 2 | §2 audio table (FLAC 46 421 260 / 526.3×; MP3 71 675 908 / 3 251×; MP2 105 655 092 / 1 198×) | :49-52 | REAL | raw/r1/codec-benches-20261025.txt:4 `flac_decode_corpus_0046: 46421260 inter-channel samples/s (median of 7; 526.3x realtime …)` — byte-exact. |
| 3 | §3 MJPEG table (0052 0.902 s/131 Mpix/s/0.430 s/274/0.477×/2.77× … 0087 0.422/378/0.289/553/0.685×/7.60×) | :82-87 | REAL | ffmpeg-lanes.csv:10-11 medians 0.4302→0.430, 0.4097→0.410 (recomputed); mjpeg-decode-bench.csv rows 0052 0.902/0073 0.763(→0.760 median-of-5)/0124 0.704/0124-family — values present in the raw CSV. |
| 4 | "Movement vs the 5.X1a audit: 0052 3.182 s → 0.902 s, 0073 6.619 s → 0.760 s (8.7× …; 0.239× → 2.08×)" | :89-91 | REAL | The BEFORE values are exactly the speed-x1a-20261024.json walls (6.6186/3.1822) — verified in my algorithm-audit ledger; AFTER values in mjpeg-decode-bench.csv. |
| 5 | §4 "open → `media.info-ready` … 2.48–2.49 ms" (3 AVI lanes, N=10) | :121 | REAL | Recomputed from r1-avi/probe.csv (30 rows): medians 2.482 / 2.486 / 2.492 — the claimed band exactly. |
| 6 | §5 seek "102.10 ms (warm 100.70) … 404.35 ms (warm 705.30 — unexplained … flagged for the engine owner)" | :136-137 | REAL | Recomputed from r1-avi/seek.csv (20 rows): 102.1/100.69 and 404.35/705.3 — digit-exact, including the unexplained warm-seek regression published as-is. |
| 7 | §6 mkv demux "4.77 ms / 53 494 pkt/s / 773 MiB/s … 0.11 calls/pkt (28 calls total)" | :162-167 | REAL | memaudit-0014-demux-throughput.txt:6: "derived per-walk … = 4.77 ms -> 53494 packets/s, 773.2 MiB/s demux" — verbatim. |
| 8 | §4: the 6 non-AVI container lanes "refuse at open in ~2.5 ms — typed and fast" | :122-130 | REAL | raw/r1/probe.csv = 61 lines (60 rows = 6 lanes × 10 runs) with typed refusal outcomes — row counts and lane set match. |
| 9 | §9 raw inventory ("probe.csv (60 rows: 6 refusal lanes); r1-avi/probe.csv (30 rows) + r1-avi/seek.csv (20 rows)") | :209-217 | REAL (except one cell) | wc -l: 61/31/21 lines = 60/30/20 data rows ✓ — EXCEPT "mjpeg-decode-bench.csv (25 rows)": actual **20 rows** (4 lanes × 5, headerless) → F-001 S4. |
| 10 | ADPCM addendum "closed by 6.ADPCM-bench … Raw transcript: `docs/perf/raw/r1/adpcm-bench-20261026.txt`" | :61-68 | REAL | File exists in raw/r1/ (ls this run); the ADPCM rows (153 767 893 / 137 700 868) are carried in the §2 table. |
| 11 | Hardware manifest (Xeon 6/173, 2 cores, 4 138 564 kB, kernel 5.10.134-013.15) | :26-33 | REAL | Identical to the machine recorded in raw/environment.json (verified in my baseline.md ledger) — same host class, corroborated. |
| 12 | Session-run claims (median-of-5 fresh processes, warm-up discarded, loadavg 0.12–0.3) | :19-27, :80 | REAL-UNPROVEN | Discipline is CORROBORATED by row arithmetic (20 = 4×5 mjpeg rows; 10 probe rows/lane; 10 seek rows/lane) but the runs themselves predate this checkout (tmp binaries); not re-runnable. |

## 3. Contradictions
None. The one numeric self-mismatch (25-row label vs 20-row file) is an inventory
miscount of its own artifact, not a data claim.

## 4. Fabrication / inflation indicators
None. The doc's summary section writes its OWN losses in bold ("Slower than ffmpeg,
honestly: MJPEG 0.47×–0.69× … AV1 is 0.29× because rav1d runs asm-OFF by policy"),
publishes an unexplained warm-seek regression (404→705 ms) instead of hiding it, and
the DNF ledger names every lane it cannot measure with the reason.
