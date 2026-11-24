# CLAIM AUDIT: docs/corpus/corpus-v4-smoke-manifest-20261023.json
| Field | Value |
|---|---|
| Auditor | W19-A04 |
| Claims extracted | 12 |
| REAL 2 / REAL-UNPROVEN 3 / SHALLOW 0 / STUB 0 / PAPER-ONLY 2 / FALSE 1 / UNVERIFIED 4 |
| Internal contradictions found | 2 |

## 1. Blunt verdict
This is a machine-generated data record (9527 lines, 126 file entries), not prose. Its
`bytes_total` and `per_class` blocks contradict its own `files` array: `count` says 126
files but `bytes_total`/`per_class` only account for the 91 clean-phase files; the 35
dirty/garbage files (8,075,943 bytes) are silently excluded from both. The generator
(`tools/corpus/gen_corpus.py`) is the root cause: `dirty_pass` and `garbage_pass` append
to `self.files` without updating `self.bytes_total`/`self.per_class`. The manifest is
otherwise self-consistent and internally honest (probe negatives recorded, rejected draws
recorded), but the corpus files themselves are NOT in the repo, so every per-file
sha256/bytes claim is unverifiable here.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | `"generator": "tools/corpus/gen_corpus.py"` | :2 | REAL | `ls tools/corpus/gen_corpus.py` -> exists, 1338 lines |
| 2 | `"count": 126` | :7 | REAL | python3: `len(files)==126` == count |
| 3 | `"bytes_total": 69512763` | :8 | FALSE | python3: `sum(f['bytes'])==77588706`; per_class sum==69512763 (clean files only) |
| 4 | `"schema_note": "v4 = v3 keys + per-file 'post' notes + v4 classes/probes"` | :4 | REAL | 2 files carry `post` (dirty/0029-riff-size-lie.avi); tool_version 4 matches gen_corpus.py:1239 `"tool_version": 4` |
| 5 | `"v3 manifests replay unchanged"` (schema_note tail) | :4 | UNVERIFIED | would require regenerating a v3 corpus with ffmpeg; no corpus committed, no ffmpeg run in sandbox |
| 6 | `capability_probes` — 29 probes, 27 true | :12-42 | UNVERIFIED | environment record of the generating machine's ffmpeg; nothing in-repo can prove or disprove; negatives honestly recorded: `"rotate_meta": false` (:27), `zeroframe:mp4: false` |
| 7 | `rejected_draws` — 2 rejects recorded | :43+ | REAL-UNPROVEN | python3: 2 entries; idx 84 `unsupported-draw`, idx -1 ffmpeg "Nothing was written into output file" — plausible record, cannot replay |
| 8 | All 91 clean files `"ffprobe_ok": true` | :44-9380 | REAL-UNPROVEN | python3: 91/91 true, 0 false, 35 dirty/garbage entries lack the key entirely |
| 9 | garbage `empty-00.bin` `"bytes": 0, "sha256": "e3b0c442…"` | :9096-9103 | REAL | python3 hashlib: sha256(b"") == e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 — matches |
| 10 | per-file `rel` paths (e.g. `media/0000-avi-video.avi`) | :48+ | UNVERIFIED | Glob `**/0000-avi-video.avi`, `**/garbage/empty-00.bin` -> no hits; corpus not committed to repo |
| 11 | `per_class` blocks describe the manifest's classes | :9383+ | FALSE-as-total | covers only 38 clean classes; omits `garbage`(22)/`dirty`(12)/`dirty-lie-ext`(1) = 35 files; actual per-class counts of present classes all match exactly |
| 12 | `wall_seconds: 41.5, jobs: 2, bytes_planned_est: 26865458` | :9-11 | PAPER-ONLY | runtime metadata; est is 2.6x below bytes_total — an estimate, no claim violated |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| `bytes_total: 69512763` (:8) | `files` array sums to 77588706 | files array is true; python3 sum over all 126 entries. Generator bug: gen_corpus.py:1026 adds bytes only in clean phase; dirty_pass (:1076-1082, :1151-1156) and garbage_pass (:1228-1233) append to `self.files` without updating `bytes_total`/`per_class` |
| `count: 126` (:7) vs `per_class` | per_class counts sum to 91 | count is true (len(files)==126); per_class is incomplete by 35 files |

## 4. Fabrication / inflation indicators
- `bytes_total` UNDER-counts (understates, not inflates) — sloppy accounting, not puffery.
- Probe negatives (`rotate_meta: false`, `zeroframe:mp4: false`) are recorded instead of
  hidden — opposite of fabrication; good.
- rejected_draws admit an ffmpeg failure verbatim — honest.
- No fabricated hashes observed; the one hash checkable from first principles (empty
  file) is exact.

## 5. Matrix rows
N/A — data manifest, no feature-matrix rows.

## 6. Reachability (file serves as evidence for what?)
The manifest is cited by corpus tooling: `tools/corpus/run_harness.py` /
`docs/corpus/report-*.md` / `docs/perf/raw/sweep-v4-smoke-20261024.log` (sweep replays
the v4 corpus). It is a test-fixture inventory, not shipped code.

## 7. Open questions for the next agent
- Sweep log `docs/perf/raw/sweep-v4-smoke-20261024.log` claims to replay this corpus —
  cross-check counts (it should see 126 files / or only clean 91?).
- `bytes_planned_est` 26.9 MB vs actual 69.5 MB: check `draw_plan()` estimate logic.
- Should `per_class` include dirty/garbage? If intentional, rename to `per_class_clean`
  and `bytes_clean` to stop lying.
