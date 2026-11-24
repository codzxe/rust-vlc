# CLAIM AUDIT: docs/corpus/compat-matrix-20261110.raw-rows.jsonl
| Field | Value |
|---|---|
| Auditor | W19-A07 (recovered by W19-R3) |
| Claims extracted | 6 (data-integrity claims of the machine record) |
| REAL 5 / REAL-by-design 1 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A clean machine-generated evidence file: 166 rows, one per fixture, all distinct, internally
consistent (statuses sum, no packet<frame anomalies, every "decoded" row has decoded
tracks), with HONEST failure rows carried verbatim (4 refused_probe with sniff hex, 24
refused_demux, 8 demux_fatal, 36 no_decoder tracks). 100/166 referenced fixtures exist in
this tree; the other 66 are tests/compat/.regen-cache/ entries — deliberately gitignored
(".regen-cache/" in tests/compat/.gitignore: "the committed truth is the manifest ...
the bytes regenerate on demand and never enter git") with the regen manifest (67 entries,
ffmpeg argvs + sha256 pins) committed. Cited as the immutable evidence by
compat-matrix-20261110.json:2984 and the w24-d/p review chain.

## 2. Claim ledger (data-integrity claims)
| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | 166 rows, one JSON object per line | REAL | wc -l = 166; python parse = 166 rows, 0 malformed lines |
| 2 | Every row self-describes outcome | REAL | outcome statuses: decoded 97 + decoded_partial 33 + refused_demux 24 + demux_fatal 8 + refused_probe 4 = 166 EXACT |
| 3 | Track-level honesty (typed refusals, no silent skips) | REAL | 178 decoded + 36 no_decoder tracks; no_decoder messages are typed ("[decode] unsupported (cli.codec_unavailable)"); refused_probe rows embed sniff hex + probe hints |
| 4 | Per-row metrics internally consistent | REAL | 0 tracks with packets<frames; 0 decoded rows lacking a decoded track; 166 distinct `rel` paths |
| 5 | Referenced fixtures present | REAL-by-design | 100/166 exist verbatim (tests/fixtures/generated/...); 66 = .regen-cache/ (gitignored-by-design; regen-corpus-manifest.json 67 entries with argvs+sha256 pins committed; 66 exercised in this run) |
| 6 | Cited as immutable evidence by the machine record | REAL | compat-matrix-20261110.json:2984 "the raw rows (compat-matrix-20261110.raw-rows.jsonl) are the immutable evidence and were not touched" |

## 3. Contradictions
None. (Note: absolute `path` fields carry the vanished worktree prefix
/home/z/my-project/wt/w24d/... — provenance only; `rel` is the durable key.)

## 4. Fabrication / inflation indicators
None — failures are recorded with full detail, not filtered. The 1-entry difference
(67 manifest entries vs 66 exercised) is visible rather than papered over.

## 5. Reachability
N/A (evidence data). The compat harness that generates it is tests/compat/ (wired per
its compat-matrix.json config; cache_dir config.rs:365).
Commands: wc -l, python3 json parse + collections.Counter analysis, os.path.exists
sweep, cat tests/compat/.gitignore, regen-corpus-manifest.json inspection.
