# CLAIM AUDIT: docs/corpus/vlc-diff-report-20261023.lumen.json
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted | 4 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIF | 3 / 0 / 0 / 0 / 0 / 0 / 1-unverifiable |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The raw Lumen-only digest sidecar (500 entries, 61 ok with md5/pcm_bytes,
439 typed errors). Every field agrees with the merged report's rows[].lumen
objects — 500/500 identical (md5, status, error) — so it is a faithful
dump of one real run, not a second invented one. The digests themselves are
not re-executable in this sandbox (corpus absent).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | schema "bench-vs-vlc/lumen-pcm-digests/1" | lumen.json:2 | REAL | parses; matches merged report's `lumen_schema` field (json:12) |
| 2 | 500 results | whole file | REAL | python3 len(results)=500; rel set equals report rel set exactly |
| 3 | ok rows carry md5, pcm_bytes, frames, codec… | e.g. lumen.json:21 | REAL | all 61 ok rows have the full field set (python3 check); 0 dups |
| 4 | digests are real Lumen decode outputs | all ok rows | UNVERIFIED | corpus-big media absent + no built binary → cannot re-run; rc-smoke baseline (headless-decode AVI/MJPEG/PCM PASS) corroborates the pipeline, not the values |

## 3. Contradictions
None internal. One external: entry order is shuffled relative to rel sort
(0022 appears after 0021, 0020 after 0023) — consistent with parallel
two-job generation (manifest "jobs": 2), not a defect.

## 4. Fabrication / inflation indicators
None: 439 error rows retained verbatim with stage-tagged errors
("[probe] … (probe.no-demuxer)" / "[demux] no audio track"), the exact
honest-failure shape the merged report claims.

Findings: none beyond the shared F-002/F-003 of the merged report
(docs/docs/corpus/vlc-diff-report-20261023.json.audit.md) — dead
ffprobe_payload_bytes field is on the merged side only.
Commands: python3 cross-check (exit 0), sed spot checks.
