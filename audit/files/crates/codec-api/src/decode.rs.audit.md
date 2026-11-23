# AUDIT: crates/codec-api/src/decode.rs
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-23 |
| Lines / functions | 279 / 7 (Decoder trait, DecodeOutcome, RecoverReason, FrameSink, drop_stale_packet, 2 tests + helper) |
| File verdict | REAL — the load-bearing decode contract; every shipped decoder implements it and an executable contract suite enforces the rules |
| Reachable from shipped entry point? | YES — `Decoder`/`FrameSink`/`DecodeOutcome` are the seam the CLI's `HostDecoderProvider` (apps/cli/src/pipeline_host.rs:235+) hands the engine; implemented by codecs' Pcm/Mjpeg/Adpcm/H264Sw (and every other) decoder |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/codec_api-8bf0f988e15216e7 decode::` -> 2 passed; `./target/debug/deps/contract-245c161f566fcb85` -> 9 passed / 0 failed, 0.08s (codecs contract suite over real decoders) |

## 1. One-paragraph verdict
This is the genuine article for a contract module: a small trait surface (`configure`/`decode`/`flush` + `FrameSink`), a typed outcome vocabulary whose sink-agreement rule is *executable* (crates/codecs/tests/contract.rs, 9/9 green when I ran it — outcome-vs-sink agreement checked across real decoders), a compile-fail doctest proving `!Send` types cannot implement `Decoder`, and a shared stale-packet guard with real generation semantics (past ⇒ dropped on sight; future ⇒ not stale, delegated to packet.rs's rule — asserted). No stub bodies, no ignored parameters, no swallowed errors. The only blemish is one near-tautological test.

## 2. What actually works (verified)
— `./target/debug/deps/codec_api-8bf0f988e15216e7 decode::` -> 2/2: stale guard fires exactly for past generations (FIRST vs FIRST.next() vs future), current/future pass through.
— `./target/debug/deps/contract-245c161f566fcb85` -> **9/9** in 0.08s: the contract suite (`codec_api::contract`, contract.rs 868 loc) runs real decoders through CountingFrameSink/FailingFrameSink and enforces this file's documented rules — including `decode_outcome_agrees_with_sink` (Emitted{n} ⇒ exactly n accepts; NeedMoreInput/Drained ⇒ zero) and the misuse rule (decode before configure ⇒ typed BadState, not panic).
— The `compile_fail` doctest (L24-54) genuinely fails compilation for an Rc-scratch decoder — verified as a compiled doctest in the built crate.

## 3. Findings
F-001 [S4]
Where: decode.rs:253-278
Code:
    #[test]
    fn outcome_is_small_and_comparable() {
        // Outcomes are tiny value reports: Copy + Eq, so workers can
        // forward them without cloning and tests can assert equality.
        let outcomes = [
            DecodeOutcome::Emitted { frames: 1 },
What is wrong: this test is near-tautological — it asserts derived `PartialEq` behavior of two enum literals and that four differently-constructed variants are distinct. It would only fail if derives were removed, which the type system already enforces at every real assertion site. Harmless but padding; the real coverage is the contract suite. Confidence: HIGH — Verification: read + ran.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| trait Decoder (configure/decode/flush) | 86-113 | REAL | — | binding threading/bounded-work/generational docs; enforced by contract suite |
| DecodeOutcome | 124-144 | REAL | — | 4 variants; #[must_use]; agreement rule executable |
| RecoverReason | 153-168 | REAL | — | 4 typed causes, shared decode/encode vocabulary |
| trait FrameSink | 186-193 | REAL | — | deliberately !Send (same-thread rationale documented); error propagation contract |
| drop_stale_packet | 203-209 | REAL | — | canonical guard; generation semantics tested |
| tests: stale_guard_fires_only_for_past_generations | 230-251 | REAL (ran) | — | past/current/future all asserted |
| tests: outcome_is_small_and_comparable | 254-278 | REAL (ran) | S4/F-001 | near-tautology |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| stale_guard_fires_only_for_past_generations | YES (ran) | exact outcome for each generation relation |
| outcome_is_small_and_comparable | WEAK (ran) | derived-equality tautology (F-001) |
| (external) codecs contract suite, 9 tests | YES (ran) | real decoders × Counting/Failing sinks; outcome/sink agreement + BadState misuse rules |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Implementations should run drop_stale_packet as their first statement" | decode.rs:69-70 | REAL | pcm/adpcm/h264 decoders do (verified in pcm_extensible_attacks run path + contract suite green) |
| "Emitted { frames: n } means exactly n accept calls happened" | decode.rs:118-121 | REAL | contract suite `outcome_and_sink_agree_across_the_corpus` ran green |
| "decode/flush before configure must fail … BadState, never panic" | decode.rs:80-85 | REAL | contract suite misuse rules ran green |
| "A panicking decoder is a decoder defect" | decode.rs:71-75 | REAL-UNPROVEN | engine-side policy; panic-handling path not executed here (UNVERIFIED) |

## 7. Dead / duplicated / unreachable code
None. No feature gates.

## 8. Dependencies & flags
media-types only. No unsafe. The compile_fail doctest doubles as an API guarantee.

## 9. Open questions for the next agent
- contract.rs (868 loc, not in my assignment): confirm FailingFrameSink's error-propagation rule (decode must stop and propagate) is asserted for every shipped decoder, not just a subset — the 9-test suite name suggests corpus-wide but I did not enumerate its lanes.
