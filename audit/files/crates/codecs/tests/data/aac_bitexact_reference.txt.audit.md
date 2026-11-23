# AUDIT: crates/codecs/tests/data/aac_bitexact_reference.txt (reference data)
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2025-07-14 |
| File verdict | REAL (independently re-derived by me) |
| Rows | 2 fixture + 3 corpus sections (0059, 0110, 0029-bridge) + fixture-budget ledger |
| Consumed by | tests/aac_bitexact.rs (2/2 pass — executed earlier), aac_corpus_differential.rs |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `ls -la` fixtures → 25219 / 13201 / 2500 B, all three match the recorded budget ledger exactly; `ffmpeg -map 0:a -c:a pcm_s16le -f s16le - | md5sum` on both fixtures → `ebb4ae6edb452a22fe301f3f0efb58a4` and `0b5b973fe9f32823deee93130bab892b`, both equal the recorded `ffmpeg_pcm_md5` values |

## 1. One-paragraph verdict
Honest reference data in the established convention: provenance recorded (same-session ffmpeg 7.x-authored fixtures, digests measured with the exact command living in tests/aac/mod.rs), the lossy-codec tolerance stated openly (max_delta = 1 LSB recorded per row instead of pretending sample-identity), corpus rows pin corpus bytes so a re-imaged corpus fails loudly, and the ISO 14496-4 conformance vectors are explicitly NOT redistributed with the license/size rationale. I re-derived the oracle side myself: both ffmpeg PCM digests match, all three fixture sizes match the budget ledger, and the frames×1024 sample arithmetic checks out (131×1024=134144; 142×1024=145408). The Lumen-side digests are asserted by the passing aac_bitexact suite. One corroborating detail: the 0029 m4a bridge row is 2 frames / 2048 samples (~8 KB of PCM) — which explains why the stdin-bridge deadlock documented in my aac/mod.rs audit has never fired (the recorded file is far below the pipe-buffer threshold). Zero findings.

## 2. What actually works (verified)
— ffmpeg digest re-derivation (2/2 exact), size ledger (3/3 exact), arithmetic (2/2), consumer suite executed (aac_bitexact 2/2 earlier in this audit).

## 3. Findings
None. Checked: oracle digests, file budget, sample-count arithmetic, corpus-pinning semantics, tolerance honesty, license rationale for the absent conformance vectors.

## 4. Doc claims
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every ffmpeg_pcm_md5 ... was measured against that same ffmpeg's decode" | L5-8 | REAL | re-derived with sandbox ffmpeg — exact match on both fixtures |
| "sample counts are `frames x 1024` on both sides" | L13-15 | REAL | arithmetic verified for both fixture rows |
| "the measured class of this engine pair is max_delta = 1 LSB (s16)" | L16-18 | REAL-UNPROVEN (recorded measurement) | consistent with the passing bitexact suite; not re-measured sample-by-sample by me |
| "a re-imaged corpus FAILS the row loudly" | L9-11 | REAL | md5-pinned corpus rows + SKIP semantics observed in sibling suites |

## 5. Open questions for the next agent
None (but see aac/mod.rs F-001: the 0029-bridge row's tiny size is the only thing standing between the corpus lane and the stdin deadlock).
