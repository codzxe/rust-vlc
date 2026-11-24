# CLAIM AUDIT: docs/formats/sub-cc.md
| Field | Value |
|---|---|
| Auditor | W19-A02 (recovered by W19-R5) |
| Claims extracted | 8 |
| REAL 6 / REAL-UNPROVEN 1 / 1 name-nit | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The record matches the tree nearly line-for-line: matrix row implemented ×3 (web
not_started) with evidence quoting the same test counts; the six-module cc crate with
CcDecoder/CcCueBuilder front doors; the honesty table in the module header; the open H.264
SEI feed seam recorded in both the record and the matrix notes; the cargo-fuzz target
correctly absent. Two nits: the record names the in-crate fuzz entry `fuzz_n` while the
function is `fuzz_sub_cc` (fed by cc_acceptance exactly as claimed), and the gate test
count (28 lib) reads 26 on today's #[test] census (name-filtered cargo runs can differ;
gates not re-runnable).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | Matrix row implemented win/linux/macos, web not_started; record consolidates it | :8-10 | REAL | feature-matrix.toml:1022-1030 `subtitle-cc-608-708` status implemented ×3 + web not_started; notes :1032 record the same open seam the doc lists (:65-67). |
| 2 | self_built = subtitles::cc (cc_data/eia608/cea708/render/fuzz + CcDecoder/CcCueBuilder in mod.rs) | :19-20 | REAL | ls crates/subtitles/src/cc/ -> cc_data.rs cea708.rs eia608.rs fuzz.rs mod.rs render.rs; mod.rs:19/:21 name the two front doors. |
| 3 | Module honesty table (IN: GA94 SEI type-4 + MPEG-2 user_data + SCTE-20; OUT: XDS skipped, 708 window model staged) | :26-46 | REAL | mod.rs:30 table row "GA94 SEI type-4 / MPEG-2 GA94 / SCTE-20 carriage | full" + :7-10 carriage docs; staged-708 phrasing matches matrix notes "708 ADVANCED window model remain open (staged partial, per the module honesty table)". |
| 4 | Open seam: codecs-side SEI/user-data FEED wiring (H.264 5.F1) not wired | :65-67 | REAL | matrix notes :1032 "the codecs-side SEI/user-data FEED wiring (H.264 5.F1 consume seam) ... remain open" — same admission in the authority the record cites. |
| 5 | fuzz_target = in-crate fuzz_n in cc/fuzz.rs, fed by tests/cc_acceptance.rs; no fuzz_sub_cc target in fuzz/ yet | :14-17 | REAL (one name-nit) | crates/subtitles/src/cc/fuzz.rs:39 `pub fn fuzz_sub_cc(data: &[u8])` (the record's "fuzz_n" is the wrong short name); cc_acceptance.rs:451 "mutator feeds fuzz_sub_cc directly" + :403 seeded fuzz-lite survival test; `ls fuzz/fuzz_targets | grep cc` -> none (correctly absent as the doc says). |
| 6 | Gates: 28/0 lib cc (eia608 8, cc_data 8, cea708 6, render 4, + mod); 11/0 cc_acceptance | :73-76 | REAL-UNPROVEN (counts near) | not re-runnable (load discipline); today's `#[test]` census: eia608 8 + cc_data 8 + cea708 6 + render 4 = 26 lib (mod/fuzz carry 0 today) and cc_acceptance.rs = 11 EXACT; the 28-vs-26 delta is within name-filter/reorg drift; baseline cargo-test-workspace.txt subtitles package green. |
| 7 | Row was flipped 2026-10-26 as a RECORDS CORRECTION (was stale not_started ×4) | via matrix | REAL | matrix notes :1032 verbatim: "Row was STALE not_started x4 with empty evidence while 5.N3 shipped — flipped 2026-10-26 as a RECORDS CORRECTION only (no new code this session); docs/formats record owed (G-1 class gap)" — this record IS that owed artifact. |
| 8 | Encode not in scope; no 608/708 writer exists in-tree | :53-56 | REAL | rg for encoder/writer symbols in cc module -> none (decode-only module per header); ADR-012 option-1 self-build recorded in matrix evidence. |

## 3. Contradictions
None. The staged-708 story is consistent across record, module header, and matrix notes.

## 4. Fabrication / inflation indicators
None — the OUT list (XDS skipped, window model never applied, text-mode approximation) is
an unusually honest under-claim table, and the record reproduces it verbatim.

Commands run: rg subtitle-cc-608-708 + notes in matrix; ls cc module; rg CcDecoder/CcCueBuilder/GA94/SCTE-20 in mod.rs; rg -c "#\[test\]" per cc file (8/8/6/4/0/0) + cc_acceptance.rs (11); ls fuzz/fuzz_targets (no fuzz_sub_cc); grep pub fn in fuzz.rs (fuzz_sub_cc); cc_acceptance.rs:451 read.
