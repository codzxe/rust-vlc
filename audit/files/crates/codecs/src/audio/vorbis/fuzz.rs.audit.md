# AUDIT: crates/codecs/src/audio/vorbis/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W07-A06 |
| Date | 2026-11-23 |
| Lines / functions | 174 / 4 (incl. 3 tests) |
| File verdict | SHALLOW (passes 2 and 3 are real code that NO committed test ever executes — see F-001) |
| Reachable from shipped entry point? | NO — fuzz/test infrastructure by design (cargo-fuzz shim fuzz/fuzz_targets/fuzz_codec_vorbis.rs calls it; campaign itself unwired: nightly Lead-decision pending per fuzz/README.md:20-27) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | prebuilt `./target/debug/deps/codecs-a0d7a93b7072ff58 vorbis::fuzz` -> 3 passed; prebuilt vorbis_fuzz_survival binary -> 1 passed in 0.00s; python3 fixture/seed head inspection; rg repo-wide discontinuity/decode_block sweeps |

## 1. One-paragraph verdict
The harness code itself is competent (three passes: header chain, free block decode, contract path with a discontinuity reset; bounded output math that matches decoder.rs:343-345 exactly), but its central claim — "One call = one untrusted byte buffer through **all three** decode surfaces" — is false for every input any committed test feeds it. Passes 2 and 3 run only after `VorbisHeaders::from_extra_data` fully validates, which requires the input to BEGIN with a raw `\x01vorbis` identification header. Every input in the in-crate tests, every one of the 324+ inputs in tests/vorbis_fuzz_survival.rs (four committed `.ogg` CONTAINER fixtures, all starting `OggS` — verified bytewise — plus their mutants/truncations, plus the corpus `.ogg` lane), and both committed libFuzzer seeds (`seed_ogg_vorbis8k_st.ogg`, `seed_ogg_vorbis16k.ogg`, both `OggS`-headed) fail the strict magic check at headers.rs:102 and the harness returns at fuzz.rs:82 before decode_block or the contract path ever run. The survival suite's own doc repeats the three-surface claim. Consequence: the ONLY code in the repo that constructs a `discontinuity: true` packet (fuzz.rs:113) is dead in every test run, so the decoder's seek-reanchor reset path (decoder.rs:761-764) ships with zero test coverage anywhere.

## 2. What actually works (verified)
- `./target/debug/deps/codecs-a0d7a93b7072ff58 vorbis::fuzz` -> 3 passed (empty, marker/short 1..40, hostile 0xFF/0x00 4096 + crafted id-header prefix) — all exercising pass 1 only.
- `vorbis_fuzz_survival` binary -> 1 passed, 0.00s — consistent with 324 instant pass-1 rejections, corroborating F-001.
- Bounds doc cross-checked: `max_output_len` = `blocksize_long/2 × channels × 4` (decoder.rs:343-345), blocksize_long = `1 << bs1` with bs1 ≤ 13 (headers.rs:139-161) → ≤ 131,072 B; the lapping yield (bs+bs)/4 = bs/2 is the correct per-packet maximum.
- fuzz/ scaffolding exists and the shim calls this harness verbatim (fuzz/fuzz_targets/fuzz_codec_vorbis.rs:19-21).

## 3. Findings
F-001 [S2]
Where: crates/codecs/src/audio/vorbis/fuzz.rs:4-16 and 74-84; tests/vorbis_fuzz_survival.rs:1-5
Code:
```
//! One call = one untrusted byte buffer through **all three** decode
//! surfaces:
```
```
    let headers = match VorbisHeaders::from_extra_data(data) {
        Ok(headers) => headers,
        Err(_) => {
            ...
            let _ = parse_identification(data);
            return;
        }
    };
```
What is wrong: passes 2 (free block decode) and 3 (contract path, clean + discontinuity) are gated behind a full header-chain validation that NO committed test input can pass. parse_identification demands `data[0] == 0x01 && data[1..7] == "vorbis"` (headers.rs:102); every committed fixture and both libFuzzer seeds are Ogg containers starting `OggS` (python3-verified: `vorbis_sine_44100_stereo.ogg` head = `b'OggS\x00\x02\x00\x00'` etc.), and no byte-flip mutant (max 4 flips, 7 magic bytes to fix) can cross the gate. The survival suite's doc claims "all three decode surfaces: header chain, free block, contract path" — every one of its 324+ inputs exercises surface 1 only. Why it matters: the fuzz coverage the registry row `fuzz_codec_vorbis` advertises does not exist for 2 of 3 surfaces; a panic in decode_block or in the contract decode under hostile (but chain-valid) input would not be caught by any committed test. What correct looks like: feed the harness the raw header chain (the survival suite already extracts header packets via `decode_through_ogg`/`header_packets` — join those bytes and mutate THAT as one input class), or seed the corpus with a chain-shaped blob. Confidence: HIGH — Verification: read (gate + fixture heads) + ran (0.00s survival run consistent with instant rejection); the actual pass-2/3 non-execution inferred, not instrumented (UNVERIFIED-instrumented, HIGH confidence).
F-002 [S3]
Where: crates/codecs/src/audio/vorbis/fuzz.rs:113-124 (the only `discontinuity: true` construction in the tree)
Code:
```
    for discontinuity in [false, true] {
        let packet = Packet {
```
What is wrong: repo-wide rg finds no other test constructing `discontinuity: true` (the Ogg demuxer emits such packets in production — crates/demux/src/ogg/mod.rs:443 "reanchor carries `discontinuity: true`"). Since pass 3 never runs (F-001), the decoder's overlap-reset logic (decoder.rs:761-764) has zero test coverage anywhere: not in the fuzz suite, not in the bitexact suites (all use `decode_through_ogg` with no seeks), not in the decoder's own tests (`discontinuity: false` only, decoder.rs:1086). A production-reachable seek-reanchor path ships untested. What correct looks like: a seek/reanchor test that drives discontinuity packets through the contract decoder, or fix F-001 so pass 3 runs. Confidence: HIGH — Verification: rg sweeps (commands above).
F-003 [S4]
Where: crates/codecs/src/audio/vorbis/fuzz.rs:23-34
Code:
```
//! The cargo-fuzz target file is proposed at
//! `fuzz/fuzz_targets/fuzz_codec_vorbis.rs` exactly as doc 04 §8.5
//! registers it. Building it needs the `libfuzzer-sys` dev-dependency
//! plus the `fuzz/Cargo.toml` scaffolding — both Lead-owned
```
What is wrong: stale. fuzz/Cargo.toml and fuzz/fuzz_targets/fuzz_codec_vorbis.rs exist (card W1-12 delivered the scaffold; the shim compiles on stable per its own header). Only the nightly campaign wiring is still owed. Additionally the provenance is inconsistent: this file cites card 5.B2, the shim cites 5.Q1, fuzz/README.md's registry table says 5.A5 — three different card numbers for the same harness. Confidence: HIGH — Verification: read all three files.
F-004 [S4]
Where: crates/codecs/src/audio/vorbis/fuzz.rs:38-40
Code:
```
//! Memory is bounded by the input: the decode output is at most
//! `blocksize_long (≤ 8192) / 2 × channels (≤ 8) × 4` bytes with the
```
What is wrong: "channels (≤ 8)" is not enforced anywhere I can read: parse_identification rejects only `channels == 0` (headers.rs:117-125); the id-header channel byte allows up to 255. A chain-valid 255-channel identification makes decode_block allocate 8192/2 × 255 × 4 ≈ 4.2 MB per buffer — still input-bounded (the sentence's core claim holds), but 32× the documented ceiling. UNVERIFIED whether the symphonia engine rejects >8 channels at build_engine (would restore the ≤8 claim); no test pins either behavior. Confidence: MEDIUM — Verification: read headers.rs; engine gate not executed.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CountingSink (+impl) | 55-65 | REAL | — | Saturating count, accepts everything |
| fuzz_codec_vorbis | 70-131 | REAL (pass 1) / DEAD (passes 2-3 in tests) | S2/S3 | Panic-free by construction; `cut` arithmetic in bounds (`% (len+1)`, `.min()`); early-return gate starves passes 2-3 (F-001/F-002) |
| tests ×3 | 137-173 | REAL (surface 1 only) | — | Hostile patterns are good pass-1 abuse; none can validate a chain |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| empty_input_survives_all_passes | NO for passes 2/3 | Empty input bails at pass 1; name promises all passes, body exercises one |
| marker_only_and_short_inputs_survive | NO for passes 2/3 | All inputs fail the chain gate; good pass-1 sweep |
| hostile_wide_inputs_survive | NO for passes 2/3 | All-FF/00 and crafted-prefix inputs die at comment/setup parse; the crafted id header itself does exercise parse_identification's version/channels/rate/blocksize/framing checks |
| tests/vorbis_fuzz_survival.rs::mutated_and_truncated_inputs_never_panic | NO for passes 2/3 | 324+ container-headed inputs, all rejected at the identification magic (F-001); as a pass-1 sweep it is real (mutants do reach version/blocksize/framing fields) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "One call = one untrusted byte buffer through **all three** decode surfaces" | fuzz.rs:4-5 | FALSE in practice | F-001: no committed input passes the chain gate |
| "the seek-reanchor reset path" exercised by pass 3 | fuzz.rs:15-16 | FALSE in practice | F-002: pass 3 never runs; no other discontinuity test exists |
| "Memory is bounded by the input ... blocksize_long (≤ 8192) / 2 × channels (≤ 8) × 4" | fuzz.rs:38-40 | SHALLOW | Core bound matches decoder.rs:343-345 and lapping math; the ≤8 channel clause unenforced (F-004) |
| "the harness logic is proven by the in-crate deterministic smoke tests here and by the committed-fixture + seeded-mutation survival run" | fuzz.rs:30-32 | SHALLOW | Proven for pass 1 only |
| "all three decode surfaces: header chain, free block, contract path" | tests/vorbis_fuzz_survival.rs:3-5 | FALSE | Same proof as F-001 |

## 7. Dead / duplicated / unreachable code
Passes 2-3 of `fuzz_codec_vorbis` are dead in every committed test run (F-001). The CountingSink exists solely for the dead pass 3. The cargo-fuzz campaign (35 targets) is scaffolded but unwired (nightly Lead-decision) — honest, documented in fuzz/README.md.

## 8. Dependencies & flags
Uses codec_api/media_types + the vorbis module's own surfaces. No unsafe. Part of `codec-vorbis` feature (default-on). The fuzz/ package is a separate stable-checkable workspace (fuzz/Cargo.toml self-contained `[workspace]`).

## 9. Open questions for the next agent
- vorbis/decoder.rs owner: F-002 — add a seek-reanchor/discontinuity contract test; decoder.rs:761-764 currently has no coverage.
- Does symphonia's Vorbis engine reject >8 channels (F-004's open half)?
- Same three-surface starvation pattern may afflict fuzz_codec_aac/flac/mp3/mp2/alac/adpcm — their survival suites likely also feed container files; worth a cross-check by those owners.
