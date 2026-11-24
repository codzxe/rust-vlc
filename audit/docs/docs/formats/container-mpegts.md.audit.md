# CLAIM AUDIT: docs/formats/container-mpegts.md (124 lines)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 6 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 4 REAL / 1 REAL-UNPROVEN (corpus lanes) / 0 / 0 / 0 / 0 / 1 UNVERIFIED (per-lane counts) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Accurate record whose self-diagnosed stale matrix row was amended: it says "Feature-matrix
row container-mpegts still reads not_started with empty evidence — this record is that
evidence; amendment recorded in-file for the Lead (docs-only card 5.REC-1)" — and the
amendment landed (matrix :167-180 container-mpegts implemented x3, evidence citing this
record + "5 ts corpus lanes"). Code exists exactly as scoped (crates/demux/src/ts/
{mod,psi,pes,fuzz,test_support,tests}.rs), fuzz_demux_ts.rs is one of the 35 landed fuzz
targets, and the corpus tests exist (ts_corpus.rs, ts_fuzz_survival.rs) and ran in the
baseline. Zero external deps claim verified (no mpeg2ts-reader in the graph).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Feature-matrix row container-mpegts still reads not_started ... amendment recorded in-file for the Lead" | :3-7 | REAL — amendment LANDED | matrix :167-180: status implemented x3 + evidence "record docs/formats/container-mpegts.md, 5.REC-1: 5 ts corpus lanes". |
| 2 | "self_built = [crates/demux/src/ts/** (mod/psi/pes/tests/test_support)]" | :14 | REAL — EXISTS | ls crates/demux/src/ts/ -> mod.rs psi.rs pes.rs fuzz.rs test_support.rs tests.rs. |
| 3 | "crates = [] — zero external deps (mpeg2ts-reader NOT used)" | :12 | REAL — REPRODUCED | rg 'mpeg2ts' Cargo.lock -> 0. |
| 4 | "fuzz_target = fuzz_demux_ts (tests/ts_fuzz_survival.rs + in-suite +FB)" | :11 | REAL | fuzz_demux_ts.rs in 35-target census; crates/demux/tests/ts_fuzz_survival.rs exists. |
| 5 | Corpus lanes 5/5 (PSI/PES vs ffprobe identity) | evidence § | REAL-UNPROVEN (counts) | ts_corpus.rs exists and ran in baseline (green). |
| 6 | "Seek: SeekSupport::None typed (no container index)" | scope § | REAL (typed refusal) | Consistent with typed-error discipline verified in error-surface audit. |

## 3. Contradictions
None.
## 4. Fabrication / inflation indicators
None.
