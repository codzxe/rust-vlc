# CLAIM AUDIT: docs/formats/audio-vorbis.md (87 lines)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 7 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 5 REAL / 1 REAL-UNPROVEN (differential numbers) / 0 / 0 / 0 / 0 / 1 UNVERIFIED (VLC oracle pending — honestly disclosed) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Accurate per-format record whose stale-matrix self-diagnosis was acted on: it flags the
matrix row as "not_started + empty evidence is stale vs the landed decoder — backfill
proposed", and the backfill landed (matrix :725-734 codec-audio-vorbis implemented ×3,
evidence citing this record). The code structure it describes exists exactly
(crates/codecs/src/audio/vorbis/{headers,comments,seek,decoder,error,fuzz}.rs — 2,558
lines today vs 2,045 at authoring), fuzz_codec_vorbis.rs is one of the 35 landed fuzz
targets, and the 5-row differential table is corroborated by the bitexact test's 5
assertions passing in baseline (vorbis_bitexact: 5 passed / 0 failed). The record also
documents its own prior-session honesty correction (reference-data re-derivation) rather
than hiding it.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "matrix row `not_started` + empty `evidence` is stale vs the landed decoder — backfill proposed" | :7-9 | REAL — backfill landed | matrix :725-734 status implemented ×3 + evidence string cites this record (5.REC-2). |
| 2 | "self_built = [codecs::audio::vorbis (2 045 lines: headers/comments/seek/decoder/error/fuzz...)]" | :14-16 | REAL — exists (grew) | ls + wc: 7 files, 2,558 lines today. |
| 3 | "fuzz_target = fuzz_codec_vorbis (3 surfaces ...)" | :10-11 | REAL | fuzz/fuzz_targets/fuzz_codec_vorbis.rs exists (35-target census). |
| 4 | 5-row differential table (silence BIT-EXACT, others <=6e-8) | Evidence § | REAL-UNPROVEN (numbers) / structure VERIFIED | crates/codecs/tests/vorbis_bitexact.rs: 5 tests pass in baseline (awk over cargo-test-workspace.txt -> "5 passed; 0 failed"). |
| 5 | "MD5 pins per row in tests/data/vorbis_reference.txt" | evidence § | REAL | tests/data/vorbis reference data exists (tests/data listing). |
| 6 | "Tests at card gate: 278 passed / 0 failed (27 binaries)" | evidence § | UNVERIFIED at that count; family green now | Baseline codecs tests all green (vorbis_bench/bitexact/fuzz_survival Running + ok). |
| 7 | "Honest gaps ... VLC oracle: PENDING-VLC (no binary ...)" | gaps § | REAL disclosure | Consistent with pin record (binaries gitignored, absent). |

## 3. Contradictions
None. The record's honest-gaps list (MKV-vorbis composition unexercised, libFuzzer CI
unwired) matches the repo-wide fuzz-wiring state my doc-10 audit found.
## 4. Fabrication / inflation indicators
None — it downgrades its own prior session's wrong measurement (the 9e-2 head-trim
error) in writing.
