# CLAIM AUDIT: docs/corpus/corpus-big-manifest-20261022.json (485-file corpus ledger, 38,988 LOC)
| Field | Value |
|---|---|
| Auditor | W19-A01 |
| Claims extracted | 18 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 0 / 1 (file-level: REAL-UNPROVEN — per-file hashes unverifiable, media not committed by design) / 0 / 0 / 0 / 1 (claim #2: count/bytes_total pairing) / 0 |
| Internal contradictions found | 2 (count 485 vs per_class total 401; README "29 probe results" vs 17 in manifest) |

## 1. Blunt verdict
This is a data ledger, not prose: 485 per-file records (bytes/sha256/family/params/ffmpeg argv)
plus run bookkeeping. Every claim that is checkable WITHOUT the media bytes checks out:
count=485=len(files), per-class bytes match the file census to the byte, the seed and rejected-
draw count match the generation report exactly, and the generator cited by the manifest exists.
Two real defects: (1) `count: 485` (all files, incl. 84 dirty/garbage) is published next to
`bytes_total: 1354831752` (clean 401 files only) — sum(files.bytes)=1654423109; the 84 dirty/
garbage files' 299591357 bytes are silently excluded, and NOTHING inside the manifest says so
(the day-report says "(1292 MiB clean)", the README says "files written / bytes harvested" —
a consumer computing size-per-file or "485 files totaling 1.35 GB" gets a 6th of the truth);
(2) two "dirty" (corrupted) files are sha256-IDENTICAL to their clean source — the corruption
ops were no-ops on those HLS playlists. The per-file sha256/bytes/ffprobe claims are UNVERIFIED
here because the corpus media is deliberately not committed (.gitignore: "/corpus-big/ ...
regenerable via recorded seeds; too big for git") and regeneration needs ffmpeg (out of budget).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "count": 485 | :7 | VERIFIED | python3: len(files)=485; report-20261022-2002-skip-suites.md:4 "files: **485** (1292 MiB clean)" |
| 2 | "bytes_total": 1354831752 | :8 | MISLEADING (clean-only, undisclosed) | python3: sum(files.bytes)=1654423109; dirty/garbage bytes=299591357; 1354831752+299591357=1654423109; per_class bytes sum == bytes_total exactly; per_class omits classes dirty(55)/dirty-lie-ext(7)/garbage(22); no qualifier anywhere in the manifest |
| 3 | "bytes_planned_est": 221551911 | :9 | TRUE-but-6.1x-off | actual 1354831752; README:160 "plan-time estimate actually gated against `--budget-mb`" — the 1500 MiB preset budget gated an estimate that understated 6x, so the gate never engaged |
| 4 | "seed": 58007138736976908607880252061 | :5 | VERIFIED (recorded) | report:3 "corpus seed: **58007138736976908607880252061** (non-deterministic run...)" |
| 5 | "generated_at": "2026-10-22T19:52:17+0000" | :6 | CONSISTENT | final report written 20:02 same day (filename report-20261022-2002) |
| 6 | "generator": "tools/corpus/gen_corpus.py" | :2 | TRUE | file exists (ls tools/corpus/) |
| 7 | "tool_version": 3 | :3 | TRUE (drift documented) | gen_corpus.py:1239 now writes `"tool_version": 4` + :1240-1241 schema_note "v4 = v3 keys + per-file 'post' notes + v4 classes/probes; v3 manifests replay unchanged" |
| 8 | rejected_draws: 19 records | :23- | VERIFIED | python3 len=19; report:4 "rejected draws: 19" |
| 9 | capability_probes: 17 entries, e.g. "x264_422": true ... "rotate_meta": false | :12-29 | WELL-FORMED; count drifts from docs | manifest has 17 probes; tools/corpus/README.md:160 documents "the 29 probe results" (README describes the v4 tool, this manifest is v3) |
| 10 | files[]: every record has bytes, sha256, class, family, params, cmd | whole | VERIFIED structurally | python3: 0 missing sha256, 0 malformed (len!=64); all 485 have class/family/params; ffprobe_ok=true for the 401 clean, key absent for the 84 dirty/garbage (gen_corpus.py:127-128 "excluded from the clean-file ffprobe gate") |
| 11 | per_file ffprobe claims ("ffprobe_ok": true) | per record | UNVERIFIED | media not committed (.gitignore:105 "/corpus-big/"); regen needs ffmpeg — out of sandbox budget |
| 12 | per-file bytes + sha256 | per record | UNVERIFIED (media absent by design) | find for 0000-flv-video.flv -> no hit; .gitignore:104-105 "Wave-1 corpora (regenerable via recorded seeds; too big for git)" |
| 13 | per_class {class:{count,bytes}} aggregates | tail | VERIFIED vs census | python3: per-class count and bytes match files[] exactly for all 21 clean classes; total 401 |
| 14 | report composition string matches manifest census | report:5 | VERIFIED | "audio-only=56, avi-av=43, ... webm-video=9" == python Counter of files[] classes; sums to 485 |
| 15 | "path": basename-only key (6 records share path "master.m3u8") | files[] | DATA SHAPE (not unique) | 480 unique paths / 485; the 6 hls-av entries live at distinct rel (hls/0174/... hls/0404/...); consumers must key on rel |
| 16 | dirty-class files are corrupted | dirty/* | 2 FALSE of 55 (no-op corruptions) | python3: dirty/bit-drift-region-033-master.m3u8 and dirty/dup-range-043-master.m3u8 have sha256 EQUAL to their clean source (op bit-drift-region / dup-range changed nothing); 7 dirty-lie-ext are byte-identical BY DESIGN (renamed copies) |
| 17 | garbage class: 22 archetypes (empty/one-byte/magic files) | garbage/* | COHERENT | python3 Counter: 22 distinct families (empty, one-byte, riff-magic, ftyp-magic, ebml-magic, random-bytes, all-zero, ...), incl. bytes=0 garbage/empty-00.bin |
| 18 | preset "standard" = 420 files, 1500 MiB budget | README:50-58 | RECONCILED, under-documented | 420 draws − 19 rejected = 401 clean; +84 dirty/garbage = 485; actual clean 1292 MiB < 1500 MiB cap (but see claim #3: the gate never saw the real size) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| count: 485 AND bytes_total: 1354831752 (adjacent fields) | sum(files.bytes) = 1654423109; per_class total = 401 | The fields measure different populations: count counts all files, bytes_total/per_class count only the clean 401. True state = report:4 "(1292 MiB clean)"; the manifest itself never states the clean/dirty split of its own aggregates |
| README:160 "capability_probes | the 29 probe results" | manifest carries 17 probes | Both true for their era: README documents the v4 generator; this manifest is tool_version 3. Drift, not a lie — but any reader of THIS manifest gets 17, not 29 |

## 4. Fabrication / inflation indicators
None found in the checkable layer: every number that can be recomputed from the manifest's own
records recomputes exactly (count, per-class counts/bytes, sum checks, seed/report cross-refs).
The media layer (sha256/bytes/ffprobe_ok) cannot be tested here — the corpus is intentionally
uncommitted, and the manifest+seed are the designated reproducibility anchor. The two no-op
"dirty" files (claim #16) are the closest thing to a false record: a corruption claim with a
sha256 proving the file was never altered.

## 5. Commands run (verification trail)
python3 json.load + census (count/bytes/per_class/ffprobe/sha checks; outputs inline above);
sed -n '6,12p' + '24,27p' manifest (byte-verbatim quotes); sed -n '1,6p' report-20261022-2002;
sed -n '95,107p' .gitignore; sed -n '50,72p' + '156,166p' tools/corpus/README.md; sed -n
'1018,1036p' + '1236,1250p' tools/corpus/gen_corpus.py; ls tools/corpus/; find for corpus
media (absent); python3 on corpus-v4-smoke-manifest-20261023.json (same count/bytes asymmetry:
126 vs 91 — pattern is the schema design, not a one-off).

## 6. Open questions for the next agent
- Regenerate the corpus from seed 58007138736976908607880252061 with `--replay` on a machine
  with ffmpeg and diff the 485 sha256s — this is the only way to upgrade UNVERIFIED to REAL.
- Fix or flag the 2 no-op dirty files (bit-drift-region-033 / dup-range-043): the dirty class
  promises corruption the bytes contradict.
- Decide whether bytes_total/count should carry an explicit clean/dirty split; per_class
  already exposes it implicitly (401 vs 485).
