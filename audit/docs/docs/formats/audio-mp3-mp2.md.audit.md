# CLAIM AUDIT: docs/formats/audio-mp3-mp2.md
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R4) |
| Claims extracted | 12 |
| REAL 9 / REAL-UNPROVEN 0 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNVERIFIED 3 |
| Internal contradictions found | 0 (one resolved-since drift) |

## 1. Blunt verdict
The consolidation record's tree-side claims all verify: the symphonia pin
(=0.6.1), the dedicated Mp2Decoder with its three named typed refusals, the
pinned MD5 reference file containing the table's digests, the differential
test files, both fuzz targets, and the exact 27-test mp2 surface. Its own
front-matter complaint (matrix rows stale at not_started/empty-evidence) was
true when written and has since been backfilled exactly as the record
proposed — status now implemented with evidence citing this record. The
run-time numbers (differential re-runs, throughput, 251/295 test counts) are
not re-executable here.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "both rows ... carry `not_started` + empty `evidence` — stale vs the landed decoders; backfill proposed" | :7-8 | REAL (historical; backfilled since) | feature-matrix.toml:639/:652 rows now `status = { windows = "implemented", linux = "implemented", macos = "implemented", web = "not_started" }` + populated evidence citing this record — the proposed backfill landed. |
| 2 | "crates = [`symphonia-bundle-mp3 =0.6.1 (MPL-2.0)`]" | :13 | REAL | crates/codecs/Cargo.toml:90 `symphonia-bundle-mp3 = { version = "=0.6.1", optional = true }`. |
| 3 | "dedicated Mp2Decoder with a strict Layer II gate (`mp2.header.not_layer_ii` ...)" | :30-31 | REAL | crates/codecs/src/audio/mp2/error.rs:152 `"mp2.header.not_layer_ii"`. |
| 4 | "CBR geometry gate (`mp2.stream.cbr_mismatch`)" | :33 | REAL | decoder.rs:124 "(`mp2.stream.cbr_mismatch`)". |
| 5 | "`extra_data` refused typed (`mp2.facts.unexpected` ...)" | :36-37 | REAL | decoder.rs:293 `"mp2.facts.unexpected"`. |
| 6 | "27 unit tests" for the mp2 surface at landing | :18-19 | REAL (count) | `rg -c '#\[test\]'` over crates/codecs/src/audio/mp2/*.rs -> 1+4+3+10+2+7 = 27 (exact). |
| 7 | "tests/mp3_corpus_differential.rs ... FAILS on md5 drift from the recorded corpus reference" + `tests/data/mp3_bitexact_reference.txt` | :78-81 | REAL | Both files exist; reference carries the table's digests: :47 ffmpeg_pcm_md5 253c926ff193cfe2db3a5b092acba2f4, :48 lumen 52d32e3c30972087dadbd97998a0d3cd, :88 lumen 6db7b51bbe177897a71e213bf1f3a8f2, :107 ffmpeg 87ffd7a83802f439d60fad3a91783515 — matching the doc's table rows (cbr128, mp2_48k, 0139). |
| 8 | Window contract: "ffmpeg trim ... Lumen's +529-shifted trim — window contract in the test header" | :47-49 | REAL | mp3_corpus_differential.rs:8-14 module doc states the same window contract incl. "+529-shifted pipeline timeline". |
| 9 | "fuzz_codec_mp3 / fuzz_codec_mp2 (in-crate harness logic ...)" | :11-12 | REAL | fuzz/fuzz_targets/{fuzz_codec_mp3.rs,fuzz_codec_mp2.rs} exist; in-crate fuzz.rs modules in both mp3/ and mp2/. |
| 10 | "The corpus root's session-artifact ref_*.raw/mine_*.raw pairs ... NOT consumed by any in-tree test (rg over crates/ finds no reader)" | :72-74 | REAL (in-tree half) | `rg 'ref_.*raw|mine_.*raw' crates/ --glob '*.rs'` -> only an unrelated dnx variable; no corpus raw readers. The raw pairs themselves are external (corpus root gone — UNVERIFIED existence). |
| 11 | Differential re-run verdicts (±1 LSB, MD5s re-derived, 8/8 lanes) | :65-81 | UNVERIFIED | Requires live ffmpeg + the corpus media; external corpus dir gone, cargo/ffmpeg runs out of budget. The pinned reference digests corroborate the numbers' provenance. |
| 12 | "MP2 throughput ... 602-6021× realtime; 295 lib ... tests green" | :91-95 | UNVERIFIED | Run-time facts; not re-executable. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Matrix rows not_started/empty-evidence (2026-10-24) | Matrix rows now implemented with evidence | Both true at their dates; the doc itself proposed the backfill — the tree shows the proposal was executed. Not a conflict. |

## 4. Fabrication / inflation indicators
None. The record is adversarial toward its own card brief ("the card brief's
'bit-exact' assumption was empirically wrong and recorded as such" — the
24.5% Layer II row is printed with MD5s, not hidden). The honest-gaps section
matches the tree state (VLC oracle pending — none in sandbox per baseline).
