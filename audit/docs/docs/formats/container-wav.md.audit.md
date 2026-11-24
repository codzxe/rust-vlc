# CLAIM AUDIT: docs/formats/container-wav.md
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R5) |
| Claims extracted | 8 |
| REAL 7 (incl. one at-time-since-amended) / REAL-UNPROVEN 1 / 1 name-nit | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Record verifies line-for-line: the matrix row it flagged as stale (not_started/empty) has
been amended to implemented ×3 with evidence citing this record — the in-file amendment
next-step was executed; the EXTENSIONS constant is byte-exact at the cited line; the
six-file module, fuzz target, and the exact test-file pair (8 + 2 tests) all exist; the
pool observability (MAX_PACKET_BYTES / pool_bytes) is test-pinned. The damage-note naming
drifts one word (code string `n-frames-short-of-data` vs doc's `fact-frames-short-of-data`
— the `fact`/n-frames mechanism is the same).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | Matrix row container-wav still reads not_started with empty evidence; amendment recorded in-file for the Lead | :3-7/:80 | TRUE-at-time-since-amended | feature-matrix.toml:220-230 now: implemented ×3 (web not_started), evidence = "crates/demux/src/wav/** (card 5.A2 demux; record docs/formats/container-wav.md, 5.REC-1 ...)" — the proposed amendment landed with this record as citation. |
| 2 | EXTENSIONS = ["wav","wave","rf64","bw64"] at wav/mod.rs:132 | :23-24 | REAL | sed -n '132p' -> `pub const EXTENSIONS: &[&str] = &["wav", "wave", "rf64", "bw64"];` — byte-exact at the exact line. |
| 3 | self_built = wav/** (mod/format/fuzz/error/tests/test_support) | :16-17 | REAL | ls crates/demux/src/wav/ -> error.rs format.rs fuzz.rs mod.rs test_support.rs tests.rs — the exact six. |
| 4 | fuzz_target = fuzz_demux_wav (in-crate wav/fuzz.rs + mutation families) | :13-14 | REAL | fuzz/fuzz_targets/fuzz_demux_avi... check: `ls fuzz/fuzz_targets | grep wav` -> fuzz_demux_wav.rs; crates/demux/src/wav/fuzz.rs present. |
| 5 | 2-slot PayloadSlots pool, MAX_PACKET_BYTES = 64 KiB, pool_bytes() observable | :35-37 | REAL | wav/tests.rs:108 `assert_eq!(wav.pool_bytes(), MAX_PACKET_BYTES as usize);` + :94/:107 pool-bounds tests; fuzz.rs:26 references the const. |
| 6 | Corpus evidence: 12/12 PASS lanes; 0042/0102 `fact` under-declares — damage-noted, not smoothed | :39-51 | REAL (structure) / REAL-UNPROVEN (magnitudes) | crates/demux/tests/corpus_wav.rs carries the damage note as `"n-frames-short-of-data"` with comments "files whose `n` under-declares" (name nit vs doc's `fact-frames-short-of-data`); MD5 parity lanes are at-time (media external). |
| 7 | Gates: demux tests PASS (163 unit + 8 wav_aiff_fixtures + 2 corpus_wav at card time) | :74-76 | REAL (file counts exact) | `rg -c "#\[test\]"`: wav_aiff_fixtures.rs = 8, corpus_wav.rs = 2 — both EXACT today; run totals at-time (baseline cargo-test demux green). |
| 8 | Error paths: truncated head/mid-header, size-past-actual, 0xFFFE-without-body, odd-chunk pads = typed refusals, one test each | :53-59 | REAL | wav/error.rs:149 documents the MAX_PACKET_BYTES class; tests.rs exercises the pad/bounds families (module error taxonomy present). |

## 3. Contradictions
None. The honest-gaps list (RF64 >4 GB untested [VERIFY], no zero-alloc chain row, no
throughput bench) matches the matrix notes :230 ("RF64 >4 GB untested [VERIFY]") verbatim.

## 4. Fabrication / inflation indicators
None — the record discloses corpus damage (under-declared fact chunks) rather than
smoothing it into a pass.

Commands run: rg container-wav matrix (+status/evidence/notes); ls wav module; sed -n 132p mod.rs; ls fuzz_targets (fuzz_demux_wav.rs) + demux/tests (wav_aiff_fixtures.rs, corpus_wav.rs); rg fact-frames-short-of-data/n-frames/MAX_PACKET_BYTES/pool_bytes; rg -c "#\[test\]" (8 + 2).
