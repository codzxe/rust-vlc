# DATA AUDIT: docs/corpus/corpus-big-manifest-20261022.json
| Field | Value |
|---|---|
| Auditor | W19-A01 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Size / records | 38,988 lines / 485 file records + 19 rejected_draws + 17 capability_probes |
| File verdict | REAL-UNPROVEN (structure/arithmetic verified; media payloads absent) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | python3 json parse + census (exit 0); rg -n records; sed -n spot-quotes |

## 1. What it is / provenance
Machine manifest of the "corpus-big" generated test corpus: generator `tools/corpus/gen_corpus.py`
tool_version 3, seed 58007138736976908607880252061, generated_at 2026-10-22T19:52:17+0000,
jobs 2, wall 1010.4s. Verbatim header (sed -n '3,11p' verified byte-identical):
```
 "tool_version": 3,
 "seed": 58007138736976908607880252061,
 "generated_at": "2026-10-22T19:52:17+0000",
 "count": 485,
 "bytes_total": 1354831752,
 "bytes_planned_est": 221551911,
```
Prior audit by dead agent W19-A01 exists at audit/docs/corpus/corpus-big-manifest-20261022.json.audit.md;
my independent re-run confirms its numbers exactly (see below).

## 2. Data verification (executed, python3)
- count field 485 == len(files) == 485 EXACT.
- per_class block (line 38903) has 21 classes; every class count/bytes recomputed from
  records matches EXACTLY (flv-video 8/19,139,219; avi-video 82/665,859,540; ... hls-av 6).
- per_class OMITS 3 classes present in records: `dirty`(55), `dirty-lie-ext`(7), `garbage`(22).
- bytes_total 1,354,831,752 == clean-only sum EXACT; sum(485 records) = 1,654,423,109;
  difference 299,591,357 bytes = dirty+dirty-lie-ext+garbage bytes (84 files).
- 84 records (exactly the dirty/dirty-lie-ext/garbage set) carry no cmd/ffprobe_ok keys;
  401 do. 62 records carry derived_from+corruption; 6 carry segments.
- rejected_draws = 19 entries. Duplicate rel paths: 0. Duplicate sha256 values: 9 groups —
  8 are `dirty-lie-ext` records byte-identical to their clean source (by design: lying
  extension, clean content); 1 is `dirty/dup-range-043-master.m3u8` == `hls/0174/master.m3u8`.
- Spot-check of 5 records (idx 0, 100, 242, 484, 483): rel/class/bytes/sha256 all
  well-formed (sha256 64-hex); sizes 94603/6634/53062/420/1050 bytes plausible.

## 3. Claim ledger (manifest self-claims)
| # | Claim (verbatim) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "count": 485 | :6 | REAL | len(files)==485, python3 |
| 2 | "bytes_total": 1354831752 | :7 | SHALLOW | equals clean-only sum; silently excludes 84 dirty/garbage records' 299,591,357 bytes (S3) |
| 3 | "bytes_planned_est": 221551911 | :8 | REAL (as a number) / misleading as a gate | actual clean 1,354,831,752 = 6.1x the estimate (S4) |
| 4 | per_class 21 classes | :38903 | SHALLOW | internally exact but omits dirty/dirty-lie-ext/garbage (S3 pair with #2) |
| 5 | "rejected_draws": 19 | block | REAL | 19 entries counted; idx -1 + 18 more |

## 4. Findings
F-001 [S3] Where: docs/corpus/corpus-big-manifest-20261022.json:6-8,38903
Code: `"count": 485,` / `"bytes_total": 1354831752,` / per_class omits dirty classes.
What is wrong: count counts all 485 records; bytes_total and per_class silently exclude the
84 dirty/garbage records (299,591,357 bytes) with zero in-manifest disclosure. A consumer
computing mean file size or "corpus bytes" gets two mutually inconsistent answers.
Confidence: HIGH — Verification: ran (python3 census, exact).
F-002 [S4] bit-drift-region-033 + dup-range-043 "corruptions" are no-ops.
Where: :37616-37624 (verbatim):
```
   "path": "bit-drift-region-033-master.m3u8",
   "rel": "dirty/bit-drift-region-033-master.m3u8",
   "class": "dirty",
   "bytes": 127,
   "sha256": "25e177a7d4717c940cb8daa53163337acbc008bdc71c7972e9ed4e7a85f11916",
```
That sha256 is IDENTICAL to clean source `hls/0189/master.m3u8`; same for
`dirty/dup-range-043-master.m3u8` == `hls/0174/master.m3u8`. The corruption ops produced
byte-identical outputs — the "dirty" label is false for these two records.
Confidence: HIGH — Verification: ran (python3 duplicate-sha census).
F-003 [S4] bytes_planned_est 221,551,911 vs real clean 1,354,831,752 (6.1x). Any budget
gate keyed to the estimate gates nothing real. Confidence: HIGH — Verification: ran.

## 5. UNVERIFIED
- Per-record sha256/bytes vs actual media: `docs/corpus/media/` is ABSENT from the repo
  (gitignored by design); no ffmpeg/binaries budget to regenerate. UNVERIFIED.
- Replay determinism from seed (generator not re-run; cargo/ffmpeg budget).

## 6. Consumer
Referenced by corpus reports (docs/corpus/report-20261022-*.md), corpus-v4-sweep perf doc,
and tools/corpus consumers. Audit trail: W19-A01 original audit (audit/docs/corpus/...audit.md)
reached the same conclusions independently — numbers agree on every recomputed value.
