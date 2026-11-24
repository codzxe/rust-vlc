# CLAIM AUDIT: docs/perf/raw/sweep-v4-smoke-20261024.log
| Field | Value |
|---|---|
| Auditor | W19-A04 |
| Claims extracted | 5 |
| REAL 4 / REAL-UNPROVEN 1 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNVERIFIED 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A raw replay log of the v4 corpus against the 2026-10-24 build (first vertical:
AVI+MJPEG+PCM). Its file accounting reconciles EXACTLY with the corpus manifest I
audited earlier: 126 rows, and the per-prefix counts (media 88 / dirty 14 / garbage 22 /
hls 2) match the manifest's `rel` prefixes to the file. The log's headline honesty: only
21 files "ok", 103 honestly "unsupported" (probe.no-demuxer with the sniffed bytes
quoted), zero panics, zero timeouts. This is the anti-inflation artifact — it publishes
the 83% refusal rate of its own engine.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | replays the v4 corpus (126 files) | :1-126 | REAL | `rg -c 'SWEEPLINE'` -> 126; summary "total files: 126" (:128); python3 over the manifest: rel prefixes {media: 88, dirty: 14, hls: 2, garbage: 22} == the log's path-prefix histogram (88/14/22/2) — exact match with corpus-v4-smoke-manifest-20261023.json |
| 2 | "ok 21 · unsupported 103 · dirty-reject 1 · typed-error 1 · panic 0 · timeout 0 · unknown 0" | :127-136 | REAL | summary block verbatim; awk verdict histogram consistent (103 unsupported / 22 incl. header noise / 1 / 1) |
| 3 | unsupported rows are typed probe refusals quoting the sniffed bytes | :2-6 etc. | REAL | e.g. row 2: "unsupported probe … probe.no-demuxer … [probe] unsupported (sniffed 47 40 11 10 00 42 f0 25 00 01 c1 00 00 ff 01 ff)" — raw byte evidence per refusal |
| 4 | only AVI-family files decode (first vertical) | per-class matrix | REAL | per-class matrix: ok rows concentrate in avi-av (4/4), avi-video (11/17 — 6 huffyuv refusals, consistent with the memory-audit's finding); audio-only 0/14 "unsupported" — WAV/OGG not yet wired on 2026-10-24 (the container registry landed later per open-path-wiring) |
| 5 | timing column (0.03-0.05 s/file) | col 16 | REAL-UNPROVEN | wall figures from that machine/date; not re-runnable (no corpus in repo) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| WAV sniffed "52 49 46 46 … 57 41 56 45 66 6d 74 20" ("RIFF…WAVEfmt ") → probe.no-demuxer | later tree wires wav (containers/wav.rs) | both true in their eras — this log predates the 6.OPW wiring; it records the honest state of 2026-10-24 |

## 4. Fabrication / inflation indicators
- None. The opposite: a 21/126 pass rate published verbatim with per-class refusal
  detail. This log is what "honest evidence" looks like in this repo.

## 5. Matrix rows
N/A (raw evidence log; feeds the corpus/perf lanes).

## 6. Open questions for the next agent
- Re-run the sweep against today's build (12 containers wired) — expect unsupported to
  collapse from 103 to ~garbage+dirty-only. No re-run recorded in docs/perf/raw since.
