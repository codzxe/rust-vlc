# CLAIM AUDIT: docs/formats/audio-adpcm.md
| Field | Value |
|---|---|
| Auditor | W19-A07 (recovered by W19-R5) |
| Claims extracted | 9 |
| REAL 7 / TRUE-at-time 1 / REAL-UNPROVEN 1 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Every structural claim verifies: one AdpcmDecoder behind 8 descriptors with per-flavor bit
laws documented in the module table exactly as the record states; the lettered
adpcm_oki refusal (adpcm.oki.no_reference) exists with its "no ffmpeg encoder or decoder
exists" honesty note; the breadth differential and fuzz-survival files exist; in-house with
zero symphonia-codec-adpcm dep; and the matrix row the record called STALE has since been
backfilled to implemented ×3 with evidence citing this record — the record's own "backfill
proposed" next-step was executed. Measured magnitudes are at-time.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | Matrix row `not_started` + empty evidence is stale vs landed decoder — backfill proposed | :7-8 | TRUE-at-time-since-backfilled | feature-matrix.toml:793-802 now: `codec-audio-adpcm` status implemented ×3 (web not_started), evidence = "crates/codecs/src/audio/adpcm/** ... record docs/formats/audio-adpcm.md, 5.REC-2: 7 descriptors, corpus 0042/0102 + 8/8 breadth bit-exact" — the proposed backfill landed. |
| 2 | status implemented for 7 descriptors; typed-Unsupported for lettered adpcm_oki | :9-10 | REAL | mod.rs descriptor table :16-28 lists all 8 flavors; oki.rs:33 + mod.rs:486 `"adpcm.oki.no_reference"` + test :1026 assert. |
| 3 | self_built = codecs::audio::adpcm (mod/ms/ima/yamaha/oki/swf/misc) | :17-19 | REAL | ls -> fuzz.rs ima.rs misc.rs mod.rs ms.rs oki.rs swf.rs yamaha.rs (rg pub mod adpcm in audio/mod.rs:37). |
| 4 | crates = [] — in-house, zero new dependencies (NOT symphonia-codec-adpcm) | :15-16 | REAL | rg -c symphonia-codec-adpcm Cargo.toml -> 0; audio/mod.rs:24 comment "the in-house modules ([adpcm], [pcm])". |
| 5 | fuzz-lite survival suites in-tree; fuzz_codec_adpcm planned name | :13-14 | REAL | crates/codecs/tests/adpcm_fuzz_survival.rs exists (+ adpcm_bench.rs, adpcm_corpus_differential.rs, adpcm_breadth_differential.rs). |
| 6 | 8-descriptor accepts() union, one AdpcmDecoder; WAV flavors via extra_data; raw flavors via 16-byte WAVEFORMATEX skeleton | :24-29 | REAL | mod.rs:199-242 CodecDescriptor doctests for adpcm_ima_wav/adpcm_yamaha; module table :16-28 with the flavor laws (sed read). |
| 7 | 49 unit tests post-fusion | :18-19 | REAL (grew) | current `rg -c "#\[test\]"` across adpcm src = 55 (49 at record time + 6 later). |
| 8 | Breadth five 8/8 bit-exact pinned in tests/adpcm_breadth_differential.rs with in-tree RFC-1321 MD5 | :50-56 | REAL-UNPROVEN | file exists; the 8/8 and MD5 magnitudes are at-time measurements (no ffmpeg re-run in sandbox). |
| 9 | Fusion gate 457/0 across 42 binaries; throughput 105-253 MB/s; 512+ fuzz rounds zero panics | :57-65 | REAL-UNPROVEN | at-time gates; consistent with the green codecs packages in baseline cargo-test-workspace.txt; not re-executed. |

## 3. Contradictions
None. The honest-gaps section (raw .vox/.adp/SWF/MOV/AUD demuxers "not landed", vqa3 owed,
QT boundary [VERIFY]) matches the tree: no such demuxers exist in crates/demux/src.

## 4. Fabrication / inflation indicators
None — the record flags its own matrix staleness rather than exploiting it, and the
adpcm_oki refusal is the honest-gap pattern (accept-and-refuse with a named code).

Commands run: rg codec-audio-adpcm matrix (+status/evidence); ls adpcm module + codecs/tests; rg -c "#\[test\]" (55); rg no_reference/AdpcmDecoder/descriptor table in mod.rs; rg symphonia-codec-adpcm; audio/mod.rs read.
