# CLAIM AUDIT: docs/perf/corpus-v4-sweep-20261024.md
| Field | Value |
|---|---|
| Auditor | W19-A02 |
| Claims extracted | 10 concrete claims |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 7 REAL (recounted from committed raw logs) / 0 / 0 / 0 / 0 / 2 FALSE sub-claims (per-class table total; ≤0.63 s) / 1 (lumen-sweep binary re-run) |

## 1. Blunt verdict
The load-bearing claims are true and I re-derived them from the committed raw logs: both
headline tallies (126-file smoke, 538-file big), the 75/20/8 unsupported breakdown, the
hevc-in-avi silent-skip finding, the odd-chunk-avi budget firing, the 25.5 s slowest
file. The two defects are cosmetic arithmetic: the per-class table says "90 files" while
its own rows sum to 94 (manifest media = 89), and "all ≤ 0.63 s" is contradicted by
media/0078 at 0.9 s in the doc's own raw log.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (command + output) |
|---|---|---|---|---|
| 1 | smoke tally: ok 21, unsupported 103, dirty-reject 1, typed-error 1, panic 0, timeout 0 (126) | L18-27 | REAL (EXECUTED) | awk -F'\t' over raw/sweep-v4-smoke-20261024.log: `103 unsupported / 21 ok / 1 typed-error / 1 dirty-reject`, total SWEEPLINE lines = 126 |
| 2 | big tally: ok 82 (15.2%), unsupported 446 (82.9%), dirty-reject 9, typed-error 1, panic 0, timeout 0 (538) | L115-124 | REAL (EXECUTED) | awk over raw/sweep-corpus-big-20261024.log: `446/82/9/1`, total = 538 |
| 3 | "Unsupported reason breakdown (103): probe.no-demuxer 75, sweep.container.no_demuxer 20, demux.avi.no-supported-streams 8" | L96-98 | REAL (EXECUTED) | awk on field 7: `75 / 20 / 8` — exact match |
| 4 | "hevc-in-avi … outcome `ok` with v_pkts/v_dec = 0/0, a_pkts/a_dec = 7/7, 1 demux recovery" | L83-86 | REAL (EXECUTED) | log line: `SWEEPLINE hevc-in-avi media/0074-… ok … 0/0 7/7 0/0 1 1 a avi` — exact match; "an ok that is not really ok" finding confirmed in data |
| 5 | odd-chunk-avi typed-error `sweep.concealment.budget` "budget 1000, attempted 1000", v_pkts=1000, v_concealed=1000 | L40-42 | REAL (EXECUTED) | log line: `typed-error decode resource-limit sweep.concealment.budget 1000/0 … 1000/0` + detail `(budget 1000, attempted 1000)` — exact |
| 6 | dirty 14 / garbage 22 / hls 2 (damage census) | L44-53 | REAL (EXECUTED) | raw log class counts: 14 mutation lines (truncate/swap/lie/splice/flip/dup/bit-drift/riff-size-lie), 22 `.bin`, 2 master.m3u8 |
| 7 | "slowest file media/0339-avi-video.avi at 25.5 s" (big sweep) | L138 | REAL (EXECUTED) | awk max on field 15: `25.48 /tmp/corpus-big/media/0339-avi-video.avi` |
| 8 | "per-file wall times are all ≤ 0.63 s" (smoke) | L16-17 | FALSE | awk max field 15: `media/0078-avi-video.avi ok 0.9` — one file at 0.9 s exceeds 0.63 s (0002 at 0.63 exactly is the second-slowest) |
| 9 | "Per-class pass/fail matrix (media/ classes, 90 files)" | L64 | FALSE (arithmetic) | the table's own rows sum to 94; corpus-v4 manifest media files = 89 (python3 classification: media 89 incl. riff-size-lie, dirty 13, garbage 22, hls 2 = 126 ✓). Neither 94 nor 89 equals the stated 90; three rows over-count (mp4-family 8 vs 7, flv-family 9 vs 7, chapter-family 6 vs 3) |
| 10 | raw logs "committed, force-added" | L4, L99 | REAL | ls: sweep-v4-smoke-20261024.log (26,929 B), sweep-corpus-big-20261024.log (118,569 B) both present in docs/perf/raw/ |

## 3. Findings
F-001 [S3] — per-class table does not reconcile.
Where: doc L64 + table rows vs manifest classification.
What is wrong: claimed "90 files", rows sum 94, manifest media = 89; three row counts
overstate (chapter-family 6 vs 3 actual). Why it matters: the matrix is the coverage
baseline the doc itself calls "the baseline the Phase-5 container work must move" — a
wrong class census misdirects that work. Confidence: HIGH (python3 recount).
F-002 [S4] — "all ≤ 0.63 s" contradicted by its own log (0.9 s on 0078).
Confidence: HIGH (awk over the committed log).

## 4. Contradictions
| Doc says | Raw log/manifest says | True |
|---|---|---|
| "media/ classes, 90 files" | 89 media files in manifest; table rows sum 94 | manifest |
| "all ≤ 0.63 s" | 0078 = 0.9 s | raw log |

## 5. Fabrication / inflation indicators
None in the tallies — every number I could recompute from the committed logs matched
exactly, including the unflattering ones (82.9% unsupported is reported as honest
coverage state; hevc-in-avi silent skip self-flagged). The sweep binary itself was not
re-run (media absent, binary not rebuilt) — tally verification is log-derived, not
re-measured: UNVERIFIED at the execution layer, REAL at the record layer.
