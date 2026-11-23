# AUDIT: crates/codecs/src/audio/opus/order.rs
| Field | Value |
|---|---|
| Auditor | W07-A02 |
| Date | 2026-11-23 |
| Lines / functions | 91 / 1 production fn (`vorbis_to_house`) + 3 tests |
| File verdict | REAL (6-row executed-verified bit-audibly against ffmpeg 7.1.5 on the real 5.1 corpus; rows 3/4/5/7/8 analytically correct per RFC 7845 + WAV order but carry no differential lane — REAL-UNPROVEN rows, honestly documented) |
| Reachable from shipped entry point? | YES — opus/decoder.rs:289-293 applies `vorbis_to_house(head.channels)` for mapping-family-1 streams; OpusDecoder wired at apps/cli/src/pipeline_host.rs:279-281 and apps/web/src/host.rs. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `./target/debug/deps/codecs-a0d7a93b7072ff58 opus::order` → 3 passed (exit 0); `./target/debug/deps/opus_corpus_differential-06f3a6620b4b27bb` → 2 passed incl. `corpus_0077_surround_51` (the 5.1 channel-order lane); python3 RFC/WAV derivation of all 8 rows; ffmpeg multichannel roundtrip probe (inconclusive for decoder-side isolation — see §2); `sed`/python byte-verification of quoted rows |

## 1. One-paragraph verdict
This is the rare table file whose central claim I could execute end-to-end: the 5.1 permutation `[0,2,1,5,3,4]` is verified by the corpus 0077 differential, which decodes a real 6-channel family-1 Opus file through the repo decoder (Vorbis order → this permutation) and compares DIRECTLY against ffmpeg 7.1.5's native decode with zero samples beyond one s16 LSB — I ran that test in this sandbox and it passed. I also re-derived all eight rows against RFC 7845 §5.1.1's Vorbis orders and the WAV/WAVEFORMATEXTENSIBLE orders: every row is the correct bijection. The remaining debt: rows 3/4/5/7/8 have no corpus lane (the file's own doc admits this), so a bijective-but-semantically-wrong row in that range would ship silently to any user opening a multichannel family-1 file; and the "measured permutation" unit test is a literal-vs-literal pin, not a measurement.

## 2. What actually works (verified)
- `./target/debug/deps/opus_corpus_differential-06f3a6620b4b27bb` → `corpus_0047_mono ... ok`, `corpus_0077_surround_51 ... ok` (exit 0). The 5.1 lane asserts `max|Δ| < 1e-3` and **zero samples beyond 1 s16 LSB** vs ffmpeg's f32le decode of the same file — a wrong permutation would fail catastrophically (the test's own comment, opus_corpus_differential.rs:117-119, and the channels are non-interchangeable signals). This simultaneously proves ffmpeg's native decoder outputs WAV/house order and that this table's 6-row is exactly right.
- Analytical verification (mine, against the published conventions): Vorbis orders (1:M; 2:FL FR; 3:FL FC FR; 4:FL FC FR BC; 5:FL FC FR BL BR; 6:FL FC FR BL BR LFE; 7:FL FC FR SL SR BC LFE; 8:FL FC FR SL SR BL BR LFE) vs WAV orders (3:FL FR FC; 4:FL FR FC BC; 5:FL FR FC BL BR; 6:FL FR FC LFE BL BR; 7:FL FR FC LFE BC SL SR; 8:FL FR FC LFE BL BR SL SR) yield precisely the table's rows — all eight are correct bijections.
- Unit tests: 3 passed (total bijection for 1..=8, 5.1 row pin, None for 0/9/255).
- Call-site gating verified: decoder.rs:289-293 applies the permutation only for `mapping_family == 1`; family 0/255 pass through (matches order.rs:16-20 doc).

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/audio/opus/order.rs:22-26 (doc) and rows 39-52
Code:
```
//! The 6-channel (5.1) row is corpus-verified (0077's differential
//! asserts the mapped order matches ffmpeg's native output, sample
//! for sample); the 3/4/5/7/8 rows are the documented Vorbis↔WAV
//! table entries (ffmpeg's own `vorbis` layout conversions), untested
//! against a lane this corpus does not carry.
```
What is wrong: nothing false — the doc is honest — but rows 3/4/5/7/8 are live production code for any user opening a 3–8 channel mapping-family-1 Opus stream, with zero executed verification anywhere in the tree. A transposed pair (e.g., swapping SL/SR in the 7-row) would pass the bijection test, pass all corpus lanes, and silently swap surround channels in shipped output. My RFC/WAV derivation says the rows are correct, but derivation is not a differential.
Why it matters: channel swaps are exactly the class of bug users hear immediately and tests never see.
What correct looks like: one generated differential per row (ffmpeg can synthesize n-channel family-1 Opus: my probe encoded 5/6/7/8-channel files successfully) with per-channel identifying signals, or at minimum a unit test pinning each row's semantic (channel-name) mapping rather than only bijectivity.
Confidence: HIGH (rows correct analytically; coverage gap certain) — Verification: read + ran (corpus tests; ffmpeg multichannel encode probe — 3/4ch encode failed only on amerge layout guessing: "Invalid channel layout 2.1/4.0 for specified mapping family", fixable with explicit `-channel_layout`; 5–8ch encoded fine).

F-002 [S4]
Where: crates/codecs/src/audio/opus/order.rs:76-83
Code:
```
    fn the_51_row_is_the_measured_vorbis_to_wav_permutation() {
        // Vorbis: [FL FC FR BL BR LFE], house: [FL FR FC LFE BL BR].
        let perm = vorbis_to_house(6).unwrap_or_else(|| unreachable!("5.1 row"));
        // out FL = in FL(0), out FR = in FR(2), out FC = in FC(1),
        // out LFE = in LFE(5), out BL = in BL(3), out BR = in BR(4).
        assert_eq!(perm, &[0, 2, 1, 5, 3, 4]);
    }
```
What is wrong: the name says "the measured ... permutation" but the body asserts the table equals the same literal written out again — a tautology against accidental edits, not a measurement. The actual measurement lives in the corpus differential (which I executed); this test cannot fail while the table stays self-consistent but wrong.
Confidence: HIGH — Verification: read + ran (test passes; mutation reasoning: change the table to any other bijection of the same length and this test fails, but change BOTH table and test literal together — as a single bad "measurement" would — and nothing catches it).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| vorbis_to_house | 32-55 | REAL | S3 (F-001 coverage) | 8 rows + None; 6-row executed-verified via corpus 0077; rows 3/4/5/7/8 analytically verified only |

(91-line file; the table's single production function plus consts is the entire surface.)

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| permutations_are_total_bijections | YES for malformed rows | catches wrong length, out-of-range, duplicate/missing slots |
| the_51_row_is_the_measured_vorbis_to_wav_permutation | only for accidental edits | literal-vs-literal pin (F-002); the real measurement is corpus 0077 (executed, passed) |
| unknown_counts_have_no_row | YES | 0/9/255 → None |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "ruopus emits mapping-family-1 output in the Vorbis channel order" + house = WAV order | order.rs:4-14 | REAL | corpus 0077 executed: repo(Vorbis→perm) == ffmpeg native decode, 0 samples > 1 LSB |
| "family 0 ... order-identical; family 255 is discrete ... bytes pass through" | order.rs:16-20 | REAL | decoder.rs:289-296 gating verified (read); None rows |
| "The 6-channel (5.1) row is corpus-verified ... sample for sample" | order.rs:22-23 | REAL | I executed corpus_0077_surround_51 (passed) |
| "the 3/4/5/7/8 rows are ... untested against a lane this corpus does not carry" | order.rs:24-26 | REAL (honest gap) | rg: no other corpus lane; F-001 |

## 7. Dead / duplicated / unreachable code
- None. Single function, one caller (decoder.rs:293), always compiled.

## 8. Dependencies & flags
- None (std-only, no consts imported). `#[must_use]` present (byte-verified — the `#ust_use` rendering in some tool output is a display artifact; python byte count shows the real attribute).

## 9. Open questions for the next agent
- W07-A07 (opus/decoder.rs): confirm `RuopusMultistream` truly emits Vorbis order for every family-1 channel count (my corpus execution covers only 6ch); the 3/4/5/7/8 rows' correctness is only as good as that premise plus this table.
