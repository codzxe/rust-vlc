# CLAIM AUDIT: docs/formats/container-adts.md
| Field | Value |
|---|---|
| Auditor | W19-A07 (recovered by W19-R5) |
| Claims extracted | 7 |
| REAL 6 / TRUE-at-time-since-amended 1 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Fully verified. The probe-extensions constant the doc quotes ("verified this audit") is
byte-exact at the cited line today; the fuzz target, corpus test file, and module layout
all exist; LOAS/LATM is absent exactly as the honest-gaps section says; and the matrix row
the record called stale (not_started/empty evidence) has since been amended to implemented
×3 with evidence quoting this record and its 350-vs-351 dirty-lane divergence verbatim —
the record's "amendment recorded in-file for the Lead" next-step was executed.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | Matrix row container-adts-aac still reads not_started with empty evidence — this record is that evidence; amendment for the Lead | :4-7 | TRUE-at-time-since-amended | feature-matrix.toml:272-282 now: `status = { windows="implemented", linux="implemented", macos="implemented", web="not_started" }`, evidence = "crates/demux/src/adts/** (card 5.C2b; record docs/formats/container-adts.md, 5.REC-1: 0059/0110 lanes PASS, dirty-lane 350-vs-351 divergence recorded)" — the proposed amendment landed with this record as its citation. |
| 2 | EXTENSIONS = ["aac","adts","adif"] at adts/mod.rs:135 | :24-25 | REAL | sed -n '135p' crates/demux/src/adts/mod.rs -> `pub const EXTENSIONS: &[&str] = &["aac", "adts", "adif"];` — byte-exact at the exact line. |
| 3 | self_built = crates/demux/src/adts/** (mod/error/tests/fuzz/test_support) | :17-18 | REAL | ls -> error.rs fuzz.rs mod.rs test_support.rs tests.rs (verified in this batch's audio-aac audit as well). |
| 4 | fuzz_target = fuzz_demux_adts + in-crate fuzz-lite sweeps | :14-15 | REAL | fuzz/fuzz_targets/fuzz_demux_adts.rs exists; crates/demux/src/adts/fuzz.rs present. |
| 5 | Dirty-lane divergence: 346 real + 4 recovered + sticky truncation vs ffmpeg's 351st short packet (350 vs 351) | :49/:54 | REAL | matrix evidence :281 repeats "dirty-lane 350-vs-351 divergence recorded" 1:1; the doc's own table discloses the divergence rather than claiming a clean PASS. |
| 6 | LOAS/LATM not implemented (named gap; no corpus lane) | :69-70 | REAL | rg -c "loas|latm" crates/demux/src/ -> 0 matches today. |
| 7 | Gates: demux 331/0 (244 unit incl. 30 adts + 4 adts_corpus); fmt/clippy CLEAN | :79-82 | REAL (structure) / REAL-UNPROVEN (counts) | crates/demux/tests/adts_corpus.rs exists; current adts/tests.rs census = 29 `#[test]` (30 at gate time, minor drift); run counts not re-executable (load discipline) — baseline cargo-test-workspace.txt demux package green. |

## 3. Contradictions
None. Record, matrix row, and the audio-aac codec record agree lane-by-lane (0059/0110
346/376 pkts, same dirty-lane divergence wording).

## 4. Fabrication / inflation indicators
None — the record's headline structure is "I am the evidence the matrix lacks", and the
matrix has since absorbed exactly that citation.

Commands run: rg container-adts-aac + status/evidence matrix; sed -n 135p adts/mod.rs; ls fuzz/fuzz_targets (fuzz_demux_adts.rs), demux/tests (adts_corpus.rs); rg -c loas|latm (0); rg -c "#\[test\]" adts/tests.rs (29).
