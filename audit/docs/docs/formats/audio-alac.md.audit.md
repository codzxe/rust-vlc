# CLAIM AUDIT: docs/formats/audio-alac.md (75 lines)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 7 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 5 REAL / 1 REAL-UNPROVEN (bit-exact digests) / 0 / 0 / 0 / 0 / 1 UNVERIFIED (VLC oracle pending) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Accurate record; the 5/5 bit-exact claim is structurally corroborated:
crates/codecs/tests/alac_bitexact.rs pins exactly 5 fixture MD5s (incl. the noise
stressor `1ee6321067a82b421e33038f4fc00b70` at :27/:111, verbatim from this record) and
passes in baseline (1 test fn asserting the 5 rows, ok). The self-built module exists
(decoder/error/fuzz/mod), the matrix backfill it proposed landed
(codec-audio-alac implemented ×3, evidence citing this record + docs/corpus/compat-matrix
run). The "suspected multi-frame bug: NOT REPRODUCED" narrative is a downgraded alarm
honestly recorded (audit committed in decoder.rs docs; no engine patch, no #[ignore]).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "all pinned interleaved-dump MD5 truths recomputed — 5/5 fixtures bit-exact" | Evidence § | REAL-UNPROVEN (digests) / structure VERIFIED | alac_bitexact.rs pins 5 MD5 rows; baseline run "1 passed; 0 failed" (the 1 test asserts all 5). |
| 2 | "alac_stereo24_noise.m4a ... white noise = max-entropy rice-escape stressor ... (1ee6321067a82b421e33038f4fc00b70)" | table row | REAL — VERBATIM PIN | alac_bitexact.rs:27,:111 carry the exact digest. |
| 3 | "self_built = [codecs::audio::alac (decoder/error modules ...)]" | :14 | REAL | ls crates/codecs/src/audio/alac/ -> decoder.rs error.rs fuzz.rs mod.rs. |
| 4 | "id = codec-audio-alac (matrix row ... stale — backfill proposed)" | :7-8 | REAL — backfill landed | matrix :712-721 implemented ×3 (windows/macos implemented, linux partial) with evidence citing this record. |
| 5 | "Fuzz-lite: 512 packet + 256 cookie LCG mutation rounds — Ok-or-typed only" | Evidence § | REAL (harness exists) | alac_fuzz_survival.rs ran ok in baseline (:961). |
| 6 | "Tests at card gate: 372 passed / 0 failed / 15 ignored" | Evidence § | UNVERIFIED at that count; family green | Baseline codecs suite green overall. |
| 7 | "Honest gaps: 32-bit differential row impossible with this ffmpeg" | gaps § | REAL disclosure | Consistent with ffmpeg-side encoder limitation; S32 unit-tested only. |

## 3. Contradictions
None.
## 4. Fabrication / inflation indicators
None — the record's centerpiece is a *non*-bug (suspected issue re-derived as correct).
