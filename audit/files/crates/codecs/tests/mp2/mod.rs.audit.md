# AUDIT: crates/codecs/tests/mp2/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-27 |
| Lines / functions | 165 / 8 helpers (shared test-support) |
| File verdict | REAL (helpers verified against the documented ffmpeg conversion rule; the consumer differential executed live in the baseline — fixtures unconditional, unlike the mp3 sibling) |
| Reachable from shipped entry point? | N/A (test-support). The decode path (`codecs::audio::mp2::decode_stream`) is library code behind the CLI `mp2` descriptor |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | sed reads; consumer (mp2_corpus_differential.rs) structure read; baseline cite |

## 1. One-paragraph verdict
Clean shared test-support, and its main consumer demonstrates the pattern the mp3 sibling got wrong: `mp2_corpus_differential.rs` gates only the corpus ROWS (`let Some(root) = corpus.as_ref() else { continue; }` per row) and runs the two committed fixtures through the live ffmpeg oracle unconditionally — the baseline's 0.37s / 3-tests-ok (:1232-1238) includes two real fixture differentials with the ±1-LSB bar asserted. The conversion helper `f32_to_s16` correctly implements the documented ffmpeg rule (`av_clip_int16(lrintf(x * (1<<15)))` — scale, round-ties-to-even, clip to [-32768, 32767]; `round_ties_even` matches lrintf's default FE_TONEAREST-even, and the clip bounds match), which the passing live differential transitively validates. `ffmpeg_pcm_md5` correctly forces `-c:a pcm_s16le` before `-f md5` with the reason documented. One hygiene nit: the stats helper silently truncates to the shorter stream.

## 2. What actually works (verified)
- Baseline :1232-1238: mp2_corpus_differential 3/3 ok in 0.37s — `corpus_and_fixture_differential_vs_ffmpeg` (fixture rows live-compared), `the_generated_lsf_fixture_walks_and_decodes_with_declared_geometry`, `cbr_seek_on_the_padded_corpus_stream_refines_exactly` (corpus row → skip path for the last one's corpus dependency, but the first two are fixture-based).
- Consumer length discipline: `run_row` asserts `bytes.len() == reference.len()` BEFORE calling `s16_delta_stats` (mp2_corpus_differential.rs:60-66) — the truncation nit is inert with this caller.
- `f32_to_s16` rule-diff vs documented ffmpeg formula (reasoned verification above; executionally validated by the passing live differential).

## 3. Findings
F-001 [S4]
Where: crates/codecs/tests/mp2/mod.rs:92
Code:
```
    let n = a.len().min(b.len()) / 2;
```
What is wrong: `s16_delta_stats` silently compares only `min(len)` samples — a length mismatch between the two streams hides a truncated tail (missing/extra samples invisible to both `differing` and `max_delta`). The doc says "Compare two s16le byte streams sample-by-sample" with no equal-length contract. The current caller pins lengths first, so no live hazard; future callers get a silent partial comparison. Same class as the opus helper's zip truncation. What correct looks like: assert equal lengths or report the length delta. Confidence: HIGH — Verification: read + caller trace.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fixture_path / fixture_bytes | 19-32 | REAL | — | committed-bytes contract; absence = unreachable! (repo-state error) |
| corpus_dir | 38-45 | REAL | — | LUMEN_CORPUS override; None → per-row skip in the consumer (correct pattern) |
| f32_to_s16 | 55-65 | REAL | — | ffmpeg's exact float→s16 rule (scale 32768, ties-even, clip) |
| to_s16le | 69-75 | REAL | — | little-endian, interleaved |
| decode_s16le | 79-83 | REAL | — | product `decode_stream` + conversion |
| s16_delta_stats | 89-103 | REAL | S4 | F-001 min-truncation |
| ffmpeg_available | 107-114 | REAL | — | |
| ffmpeg_decode_s16le / ffmpeg_pcm_md5 | 119-165 | REAL | — | `-map 0:a`, forced pcm_s16le before md5 (reason documented) |

## 5. Test honesty
No tests in this file (support module). The consumer suites' assertions are real and two of three ran on committed fixtures in the baseline (see §2).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "ffmpeg's own float→s16 rule (av_clip_int16(lrintf(x * (1<<15))))" | mod.rs:47-50 | REAL | rule-diff (ties-even = lrint default; clip = av_clip_int16) + passing live differential |
| "any deviation shows up as an MD5 mismatch, never silently" | mod.rs:51-52 | REAL | the differential compares full-stream digests with pinned lengths |
| "a bare `-f md5` would hash whatever default encoder the muxer picks" | mod.rs:140-142 | TRUE | ffmpeg md5 muxer default-encodes; forcing pcm_s16le is the correct pin |

## 7. Dead / duplicated / unreachable code
`#![allow(dead_code)]` for helper subsets across consumers (mp2_corpus_differential, mp2_bench, mp2 fuzz suites) — acceptable; masks truly-dead helpers if a consumer disappears.

## 8. Dependencies & flags
Test-support; no features. External: ffmpeg binary for oracle helpers.

## 9. Open questions for the next agent
- mp2_corpus_differential.rs and mp2_bench.rs are other agents' files; this module's helpers are their substrate — the corpus-row MD5 pins there are hardcoded `expect_md5` constants (contrast with mp3's reference-file indirection): check their provenance comments.
- The mp3 suite should adopt this module's per-row corpus gating (my file-13 F-001).
