# AUDIT: crates/codecs/tests/contract.rs
| Field | Value |
|---|---|
| Auditor | W06-A08 |
| Date | 2025-06-27 |
| Lines / functions | 312 / 4 fns + 9 tests |
| File verdict | REAL — 9/9 executed; scope narrower than the doc quote promises |
| Reachable from shipped entry point? | NO — integration test (exercises the shipped first-vertical MjpegDecoder contract path) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `./target/debug/deps/contract-245c161f566fcb85` -> 9/9 ok (0.08s); rg usage analysis of codec_api::contract |

## 1. One-paragraph verdict
A well-built contract suite and I ran all of it: outcome/sink agreement over
all 60 real fixture frames, stale-generation drop, flush-drains-and-sticks,
four damage classes (entropy truncation, broken SOI, empty packet, marker
soup) through the shared recovery checker, per-frame damage isolation with
exact sink counts (the M9 regression: one corrupt JPEG killing the session),
sink-error propagation, progressive-JPEG-as-Unsupported (not Corrupt),
unconfigured-use typed errors, foreign-descriptor refusal. The one finding is
a coverage claim: the header quotes doc 01 §2.4(e) — "the reusable test suite
every codec in `codecs` … runs against its implementations" — but this shared
suite has exactly one consumer in the whole tree: this file, against
MjpegDecoder. Every other decoder (flac/pcm/alac/opus/vorbis/h264/vp9/av1/dnx)
has hand-rolled contract-ish tests in its own files instead; none runs
`codec_api::contract`. The doc's "every codec" is false as of this tree.

## 2. What actually works (verified)
— `./target/debug/deps/contract-245c161f566fcb85` -> "9 passed; 0 failed"
(0.08s), my run: outcome_and_sink_agree_across_the_corpus,
stale_packets_are_dropped_on_sight, flush_drains_immediately_and_sticks,
corrupt_input_recovers_or_errors_typed, per_frame_damage_leaves_the_decoder_usable,
sink_errors_propagate_verbatim, unsupported_features_are_typed_unsupported_not_corrupt,
unconfigured_use_is_a_typed_decoder_state_error,
configure_rejects_foreign_codec_descriptors — all ok.
— rg `codec_api::contract|contract::` over crates/codecs/tests/*.rs: exactly
one file (this one).

## 3. Findings
F-001 [S3]
Where: crates/codecs/tests/contract.rs:1-4 (quoting doc 01 §2.4(e))
Code:
//! The codec-api contract suite run against `MjpegDecoder` (doc 01
//! §2.4(e): "the reusable test suite every codec in `codecs` … runs
//! against its implementations"). Real fixture frames are the
//! emitting packets; damaged variants drive the recovery paths.
What is wrong: the quoted doc-01 promise ("every codec … runs against its
implementations") is not reality: the reusable suite
(`codec_api::contract::{CountingFrameSink, decode_outcome_agrees_with_sink,
stale_packet_is_dropped_on_sight, flush_drains,
corrupt_input_recovers_or_errors_typed, sink_errors_propagate}`) is invoked by
exactly one test binary, this one, for exactly one decoder (MjpegDecoder).
The other decoders' contract edges are covered by per-codec hand-rolled tests
(vp9/decoder.rs tests, h264_sw_differential's contract_state_rules, etc.) —
real coverage, but not THE suite, and not uniformly (each hand-rolled variant
picks its own subset of rules). Why it matters: the doc claim would lead an
auditor to assume uniform contract enforcement; a decoder whose hand-rolled
tests skip a rule (e.g. sink-error propagation) has no backstop. What correct
looks like: either port the remaining decoders onto the shared suite or fix
doc 01 §2.4(e).
Confidence: HIGH — Verification: rg (one consumer) + ran the suite.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fixture_frames | 20-29 | REAL | — | sha256-pinned extraction (by fixtures suite) |
| configured | 31-51 | REAL | — | |
| packet | 53-64 | REAL | — | |
| 9 tests | 66-312 | REAL | — | all executed, all pass (§2) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| outcome_and_sink_agree_across_the_corpus | Yes | shared agreement checker over 60 real frames + exact count 1 |
| stale_packets_are_dropped_on_sight | Yes | shared checker + post-cancel decode/flush |
| flush_drains_immediately_and_sticks | Yes | shared checker (MJPEG statelessness pinned) |
| corrupt_input_recovers_or_errors_typed | Yes | 4 damage classes through the shared recovery rule |
| per_frame_damage_leaves_the_decoder_usable | Yes | exact sink counts 1→1→2→4 across mixed damage classes (the M9 regression) |
| sink_errors_propagate_verbatim | Yes | shared checker |
| unsupported_features_are_typed_unsupported_not_corrupt | Yes | synthesized SOF2 → Unsupported kind + Decode stage |
| unconfigured_use_is_a_typed_decoder_state_error | Yes | decode + flush both typed Decoder errors |
| configure_rejects_foreign_codec_descriptors | Yes | h264 descriptor → Unsupported |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the reusable test suite every codec in `codecs` … runs against its implementations" (doc 01 §2.4(e), quoted) | L2-3 | FALSE | one consumer, one decoder (F-001) |
| "Real fixture frames are the emitting packets; damaged variants drive the recovery paths" | L3-4 | REAL | my run |

## 7. Dead / duplicated / unreachable code
None in this file. The unused-by-others shared harness is the inverse problem
(F-001): reusable, reused once.

## 8. Dependencies & flags
codec-api (contract module), codecs::video::mjpeg, common helpers, media-types.
No feature gates.

## 9. Open questions for the next agent
- codec-api's contract module itself (not in my list): are its checkers
  decoder-generic (they are used generically here)? If so, porting cost for
  the other 10+ decoders is low — the gap looks like drift, not design.
