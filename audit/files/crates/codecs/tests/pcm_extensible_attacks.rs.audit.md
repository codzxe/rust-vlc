# AUDIT: crates/codecs/tests/pcm_extensible_attacks.rs
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-23 |
| Lines / functions | 617 / 14 (6 helpers + PRNG + builders, 5 tests, 1 sink impl) |
| File verdict | REAL (adversarial suite, executed 5/5 pass) |
| Reachable from shipped entry point? | N/A (integration test) — runs in the codecs test suite; I executed the prebuilt binary directly |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `./target/debug/deps/pcm_extensible_attacks-24a3ec3edf4091f7 --nocapture` -> 5 passed / 0 failed, 0.02s; read pcm/decoder.rs:374-440 + 86-91 to confirm the fixes the tests pin |

## 1. One-paragraph verdict
A genuine adversarial suite, not theater: 512 seeded SplitMix64 mutants across every WAVEFORMATEXTENSIBLE field plus structural flips/truncations/splices, driven through the PUBLIC `PcmDecoder::configure` path, with every failure checked against an 8-code typed taxonomy (stage Decode + `Unsupported` kind + exact code) and every acceptance cross-accounted against the blob's own facts (channels/rate/instants/duration recomputed independently). B2/B3/B4 pin concrete historical bugs (42-byte vendor-tail stage disagreement, `as u16` block-align wrap, cbSize lies). One real defect: the B3 "65-channel ceiling" branch claims the fix ("65 channels is now the typed refusal below", L502) but the test accepts BOTH outcomes — a regression that re-admits 65 channels would still pass via the Ok arm. Everything else executes and bites.

## 2. What actually works (verified)
— `./target/debug/deps/pcm_extensible_attacks-24a3ec3edf4091f7 --nocapture` -> **5 passed, 0 failed** (all five tests, no skips — no corpus/env gate in this file).
— Cross-checked the pinned fixes exist in production: `parse_waveformatex` length gate `!matches!(bytes.len(), 16 | 18) && bytes.len() < 40` (decoder.rs:375), channel ceiling `channels > MAX_PCM_CHANNELS` (64) typed refusal (decoder.rs:404-415), `MAX_PCM_CHANNELS` doc tying it to the demuxer's `demux::avi::header::MAX_PCM_CHANNELS` (decoder.rs:86-91). The wrap-attack vectors named in B3 (32768×16→0, 65535×16→65534, 16384×32→0, 8192×64→0) are exactly the products the old `as u16` cast would wrap — the test table is technically literate.

## 3. Findings
F-001 [S3]
Where: pcm_extensible_attacks.rs:499-517
Code:
    // Adjacent observation — the ceiling disagreement: the demuxer caps
    // nChannels at 64 (damage-counted skip); this seam used to accept
    // 65 with an honest align. Fixed to the same ceiling (stage
    // agreement): 65 channels is now the typed refusal below.
    let blob = plain_blob(1, 65, 8_000, 130, 16);
    match configure("pcm_s16le", blob) {
        Ok((mut decoder, generation)) => {
What is wrong: the comment asserts the 65-channel blob "is now the typed refusal below", but the match has two live arms: if `configure` succeeds (i.e. the ceiling regression returns), the Ok arm asserts honest decode (130-byte frame, 65 channels) and the test still PASSES. The claimed fix is not pinned. I verified the Err arm is the one that fires today (decoder.rs:404-415 refuses channels > 64), so the comment is accurate about current behavior — but the test cannot detect the regression it was written to guard. Why it matters: the W2-15 g1 stage-agreement fix is protected by nothing; a one-line revert of the ceiling in decoder.rs keeps this suite green. What correct looks like: assert the Err arm unconditionally (`Err(err) => assert_typed(...)`, `Ok(_) => unreachable!`), matching the style of the four wrap vectors above it.
Confidence: HIGH — Verification: read both arms + read the decoder's ceiling; ran the test (passes on the fixed code).

F-002 [S4]
Where: pcm_extensible_attacks.rs:204-208
Code:
/// One seeded mutation of the canonical blob: a mutation class drawn
/// from the PRNG, applied to a fresh copy. Classes 0-2 are structural
/// (flips / truncation / splice — the splice can grow the blob past 40,
/// pinning the length gate's 41..=48 margin); classes 3-10 target one
What is wrong: "the length gate's 41..=48 margin" is stale — the production gate is `!matches!(len, 16|18) && len < 40` (decoder.rs:375), i.e. any length ≥ 40 is accepted; there is no 48 bound anywhere. The splice class exercises the open-ended ≥ 40 acceptance, not a 41..=48 window. Misleading comment only; behavior is correct.
Confidence: HIGH — Verification: read decoder.rs:375.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| GUID_PCM / GUID_IEEE_FLOAT consts | 42-48 | REAL | — | correct KSDATAFORMAT bytes (verified against known GUID) |
| WAVEFORMAT_CODES | 53-62 | REAL | — | matches decoder taxonomy (all 8 codes exist in decoder.rs) |
| SplitMix64 (new/next_u64/below/byte) | 67-91 | REAL | — | reference generator, deterministic |
| ext_blob / plain_blob | 96-131 | REAL | — | correct little-endian layouts; byte_avg = align×rate |
| next_generation / configure / packet | 133-169 | REAL | — | public-path configure, no private API abuse |
| CaptureSink | 171-184 | REAL | — | collects frames, unreachable! on non-audio |
| duration_nanos | 189-194 | REAL | — | independent floor-div recompute |
| mutate | 209-275 | REAL | S4/F-002 | 11 classes incl. coherent-channel accepted-lie generator |
| assert_accounted | 279-325 | REAL | — | exact Emitted/Recovered accounting vs blob facts |
| assert_typed | 330-342 | REAL | — | stage+kind+code triple check |
| seeded_extensible_blob_mutations_… | 346-423 | REAL (ran) | — | 512 mutants, `typed+accepted == 515` exact count |
| forty_two_byte_vendor_tail_… | 427-461 | REAL (ran) | — | unreachable!-guarded pin of the 42-byte form |
| block_align_cross_check_must_not_wrap_u16 | 465-518 | REAL (ran) | S3/F-001 | 4 wrap vectors hard-pinned; 65-channel branch two-armed |
| cbsize_zero_with_a_complete_guid_… | 522-541 | REAL (ran) | — | pins cross-stage cbSize-ignored agreement, honest about ffprobe divergence |
| with_format_channel_extremes_… | 545-617 | REAL (ran) | — | u32::MAX channels skip-whole + Drained; free fn tail accounting |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| seeded_extensible_blob_mutations_are_typed_or_honestly_decoded | YES (ran) | any untyped failure, wrong code, or wrong concealment accounting aborts; exact total count |
| forty_two_byte_vendor_tail_extensible_configures_like_the_demuxer | YES (ran) | unreachable! if the 42-byte form is refused again |
| block_align_cross_check_must_not_wrap_u16 | PARTIAL (ran) | the 4 wrap vectors: YES (unreachable!-guarded); the 65-channel ceiling: NO — both arms pass (F-001) |
| cbsize_zero_with_a_complete_guid_is_stage_consistent_today | YES (ran) | unreachable! if cbSize starts gating |
| with_format_channel_extremes_never_panic_and_skip_honestly | YES (ran) | exact Recovered/Drained outcomes asserted |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every acceptance must then decode a payload with exact concealment accounting" | pcm_extensible_attacks.rs:13-15 | REAL | assert_accounted ran 500+ times in the seeded loop (5/5 pass) |
| "ffprobe 7.1.5 plays it as pcm_s16le (measured …)" (42-byte form) | pcm_extensible_attacks.rs:432-433 | REAL-UNPROVEN | ffmpeg present but the measurement is historical, not re-run here; the decoder-side acceptance is verified |
| "65 channels is now the typed refusal below" | pcm_extensible_attacks.rs:502 | REAL behavior / SHALLOW pin | decoder.rs:404-415 refuses; but the test cannot detect regression (F-001) |
| wasm32 saturation claim ("untestable from this 64-bit host") | pcm_extensible_attacks.rs:553-556 | UNVERIFIED | no wasm run performed; host test pins 64-bit behavior only |

## 7. Dead / duplicated / unreachable code
None. All helpers used; no #[ignore]; no cfg gates.

## 8. Dependencies & flags
codec-api prelude, codecs pcm, media-types — all in-workspace. No features. Test binary built (target/debug/deps/pcm_extensible_attacks-24a3ec3edf4091f7).

## 9. Open questions for the next agent
- pcm/decoder.rs full audit: confirm the ≥40 length gate cannot be tricked by a 40-byte blob with tag ≠ 0xFFFE and garbage bytes 16..40 (plain 16-byte form + 24 junk bytes would pass the length gate as a "16|18" match? — no: len 40 fails `matches!(16|18)` and passes `< 40` as false → accepted as plain form reading only fields 0..16; check whether that is intended stage agreement with the demuxer).
