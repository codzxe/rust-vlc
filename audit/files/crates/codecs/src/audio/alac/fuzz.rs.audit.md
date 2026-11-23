# AUDIT: crates/codecs/src/audio/alac/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W07-A04 |
| Date | 2026-11-23 |
| Lines / functions | 127 / 2 (+1 impl, 2 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (fuzz harness) — reachable via `fuzz/fuzz_targets/fuzz_codec_alac.rs:19` (cargo-fuzz target, nightly NOT wired per its own comment) and its `#[cfg(test)]` sweeps; the ALAC decoder it exercises IS shipped (CLI "alac" arm, apps/cli/src/pipeline_host.rs:293) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `cargo test -p codecs --lib audio::alac::fuzz` → 2 passed (0.00s); read-side bounds check of `parse_alac_config_inner` (decoder.rs:189-243) and `decode_frame` (decoder.rs:513-524) |

## 1. One-paragraph verdict
A real three-pass harness (cookie walk → free frame decode with input-derived cookie/packet split → full configure/decode/flush contract cycle) with genuinely bounded targets: `parse_alac_config_inner` enforces channels 1..=8, frame_length 1..=65536, bit_depth 1..=32 (decoder.rs:214-236), so `decode_frame`'s `vec![0u8; max_output_len(&config)]` caps at 2 MiB — the panic/OOM contract is enforced by construction, and the deterministic sweeps (empty, "frmaalac", 40 short patterns, 0xFF/0x00/0xA5 wide) pass. Two honest gaps: the actual corpus fuzzing never ran (nightly not wired — admitted in the module doc, but it means panic-freedom is proven on ~45 fixed inputs, not the seeded corpus `fuzz/corpus/fuzz_codec_alac/` which exists with 5 seeds), and the doc's "Memory is bounded by the input" is wrong as stated — the bound is the fixed 2 MiB ceiling regardless of input length (a 60-byte input can force it), which is a fine DoS gate but not input-proportional.

## 2. What actually works (verified)
— `cargo test -p codecs --lib audio::alac::fuzz` → 2 passed (both sweeps run all three passes incl. the ≥48-byte split decode).
— Bounds verified by read: `parse_alac_config_inner` rejects bitDepth>32 / channels>8 / frameLength>65536 / rate 0 (decoder.rs:214-243); pass 3's 36-byte cookie CAN reach a valid configure (a 12-byte `frma`/`alac` prefix at data[4..8] leaves a 24-byte body — skip_cookie_prefixes, decoder.rs:137-153).

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/audio/alac/fuzz.rs:20-25 + fuzz/fuzz_targets/fuzz_codec_alac.rs:12
Code:
```
//! The cargo-fuzz target is `fuzz/fuzz_targets/fuzz_codec_alac.rs`
//! (Lead-owned nightly wiring, see `fuzz/README.md`); the harness
//! logic is meanwhile proven on stable by the deterministic sweeps
//! below and the crate's ALAC differential suite.
```
What is wrong: the admission is honest, but the consequence is that the harness's absence-of-panic contract has never been tested against the fuzz corpus — `fuzz/corpus/fuzz_codec_alac/` holds 5 seeds (seed_alac_mono16.m4a, seed_alac_stereo16.m4a, seed_cookie_badch.bin, seed_cookie_badver.bin, seed_cookie_valid.bin) that no run has ever executed; coverage is ~45 hand-picked inputs plus the differential suite. The fuzz target file itself says "Build (once nightly is wired)". Confidence: HIGH — Verification: read (no nightly toolchain asserted; not executed — cargo-fuzz-budget).
F-002 [S4]
Where: crates/codecs/src/audio/alac/fuzz.rs:29-32
Code:
```
//! Memory is bounded by the input: the frame decode's output is at
//! most `samples-per-packet × channels × 4` bytes with the facts
//! derived from the input's own cookie bytes, a fixed number of
//! passes per call.
```
What is wrong: the bound is NOT "by the input" — it is a fixed 2 MiB ceiling (65536 frames × 8 channels × 4 B) that a 60-byte input can force via its cookie claims; input-proportional it is not. The ceiling itself is a sane DoS gate (mirrors the engine's own, decoder.rs:229-236) — the claim's wording is what's wrong. Confidence: HIGH — Verification: read (arithmetic over the validated ranges).
F-003 [S4]
Where: crates/codecs/src/audio/alac/fuzz.rs:39-50
Code:
```
/// A frame sink that only counts (the panic-free emission target).
#[derive(Default)]
struct CountingSink {
    frames: usize,
}
```
What is wrong: the counter is never read — `fuzz_codec_alac` ignores `sink.frames`; the "counting" is decorative (the contract is absence-of-panic, so this is harmless, but the doc sells a capability the harness does not use; a bytes-accepted sanity bound could have been asserted for free). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CountingSink + FrameSink impl | 40-50 | REAL | S4 | counter never read (F-003) |
| fuzz_codec_alac | 55-102 | REAL | S3 | 3 passes, bounded; never corpus-run (F-001) |
| tests: empty_and_short_inputs_survive | 108-116 | REAL | — | 42 inputs incl. "frmaalac" magic |
| tests: hostile_wide_inputs_survive | 118-126 | REAL | — | 0xFF/0x00/0xA5 maximal-magic sweeps |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| empty_and_short_inputs_survive | YES (panic class only) | any panic on those 42 inputs fails the test; no assertion beyond survival — which IS the documented contract |
| hostile_wide_inputs_survive | YES (panic class only) | same; exercises the ≥48 split-decode pass |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "One call = one untrusted byte buffer through every ALAC surface" (L4-5) | fuzz.rs | REAL | 3 passes cover parse/decode/contract paths |
| "Never panics, never hangs, never allocates beyond input-derived bounds — by construction" (L52-54) | fuzz.rs | REAL-UNPROVEN | bounds construction verified by read (fixed caps); panic-freedom only sweep-tested, never corpus-fuzzed (F-001) |
| "Memory is bounded by the input" (L29) | fuzz.rs | FALSE (wording) | F-002: fixed 2 MiB cap, not input-proportional |
| fuzz/README.md:74 registry row `fuzz_codec_alac` | fuzz/README.md | REAL | target exists and calls this fn (fuzz/fuzz_targets/fuzz_codec_alac.rs:19) |

## 7. Dead / duplicated / unreachable code
`CountingSink.frames` (F-003). No duplication; the harness shape mirrors the other codec fuzz harnesses by design (family pattern).

## 8. Dependencies & flags
codec_api prelude (DecoderConfig/Packet/FrameSink/ThreadBudget), media_types (GenerationId/SharedBytes/TrackId/Stamp). No feature gates; #[cfg(test)] sweeps run on stable.

## 9. Open questions for the next agent
- Who owns the nightly fuzz wiring ("Lead-owned", per the doc)? Until it lands, every `fuzz_codec_*` harness in this repo has the same corpus-never-run gap (12 codec + 15 demux targets) — worth a wave-level finding, not per-file.
- decoder.rs auditor: `decode_frame` allocates the full 2 MiB worst-case output on EVERY call (`vec![0u8; max_output_len(&config)]`) — fine for the fuzz/conformance lane, but confirm the contract decoder (`AlacDecoder::decode`) pools instead.
