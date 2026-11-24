# CLAIM AUDIT: docs/formats/container-aiff.md
| Field | Value |
|---|---|
| Auditor | W19-R6 (credit A08) |
| Date | 2026-11-24 |
| Claims extracted | 10 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 9 / 0 / 0 / 0 / 0 / 0 / 1 |

## 1. Blunt verdict
Accurate record with an explicit self-flag: at write time (2026-10-24) it said
the matrix row was `not_started`/empty and asked the Lead to amend it — the
amendment HAS since landed (row now `implemented` citing this file as evidence,
matching the doc's own next-step). All structural claims byte-verify.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "row `container-aiff` still reads `not_started` with empty evidence" | :4-5 | SUPERSEDED (amended) | feature-matrix.toml:246 row now `status = { windows = "implemented", linux = "implemented", macos = "implemented", web = "not_started" }` + evidence cites this record (5.REC-1 applied) |
| 2 | "`EXTENSIONS = [\"aiff\",\"aif\",\"aifc\"]` (aiff/mod.rs:79, verified this audit)" | :22-23 | REAL | `sed -n '79p' crates/demux/src/aiff/mod.rs` -> `pub const EXTENSIONS: &[&str] = &["aiff", "aif", "aifc"];` byte-exact |
| 3 | self_built `crates/demux/src/aiff/**` (mod/chunks/fuzz/error/tests/test_support) | :16-17 | REAL | `ls crates/demux/src/aiff/` -> chunks.rs error.rs fuzz.rs mod.rs test_support.rs tests.rs |
| 4 | fixture lanes `aiff_pcm_s16be_44k1_st`, `aiff_pcm_s24be_48k_st` | :38 | REAL | crates/demux/tests/wav_aiff_fixtures.rs:126,134 fixture paths present |
| 5 | "typed (`demux.aiff.*` codes: `truncated`, `no-ssnd`, `bad-comm`, …)" | :43-44 | REAL | aiff/error.rs:47 `demux.aiff.truncated`, :75 `demux.aiff.bad-comm`, :114 `demux.aiff.no-ssnd` |
| 6 | "No corpus-v4 .aiff lane exists (143 media files, zero aiff)" | :34-35 | REAL | `find tests/fixtures -name '*.aiff'` -> only the 2 generated ffmpeg-authored fixtures; no corpus lane |
| 7 | "no aiff row in `tests/zero_alloc_read_chain.rs` (5.X2)" | :52-53 | REAL | `rg -c aiff crates/demux/tests/zero_alloc_read_chain.rs` -> 0 hits |
| 8 | fuzz_target `fuzz_demux_aiff` (registry name per doc 04 §8.5) | :13-14 | REAL | fuzz/README.md row `fuzz_demux_aiff` -> `demux::aiff::fuzz::fuzz_demux_aiff`; fuzz/Cargo.toml `name = "fuzz_demux_aiff"` (byte-checked via python, display-safe) |
| 9 | "AU sibling (`container-au`) ... NOT landed — still not_started" | :54-55 | REAL | matrix container-au row: `status = { windows = "not_started", ... all not_started }` |
| 10 | "fmt clean, clippy `-D warnings` clean, demux suite PASS (163 unit + 8 fixtures + 2 corpus_wav)" | :59-61 | UNVERIFIED | exact per-suite counts not in baseline logs (baseline: clippy CLEAN overall; per-test recount would need cargo build budget) |

## 3. Contradictions
None — the one stale statement is the doc's own requested-and-pending amendment
that has since been executed.

## 4. Fabrication / inflation indicators
None; the doc's central move is *admitting* absent corpus coverage and an empty
matrix row, then asking for the amendment. Findings: none.
