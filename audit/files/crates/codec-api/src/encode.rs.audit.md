# AUDIT: crates/codec-api/src/encode.rs
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-23 |
| Lines / functions | 180 / 6 (Encoder trait, EncodeOutcome, PacketSink, drop_stale_frame, 2 tests + helper) |
| File verdict | REAL as a contract surface (mock-verified) — but ZERO implementations exist anywhere in the repo and ZERO production consumers: this is a landed vocabulary, not an encoder |
| Reachable from shipped entry point? | NO — no production code implements `Encoder` (only `MockEncoder` in codec-api's own test suite, contract_suite.rs:280) and no production code calls it (transcode explicitly refuses: wav.rs:7-14 "This is a container writer, not a codec Encoder") |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/codec_api-8bf0f988e15216e7 encode::` -> 2 passed; `./target/debug/deps/contract_suite-d432d069532a5b73` -> 8 passed / 0 failed (MockEncoder drives the contract rules); rg `impl Encoder for` repo-wide -> contract_suite.rs only |

## 1. One-paragraph verdict
Hostile-focus answer: **no encoder exists.** The trait, outcome enum, sink and stale-guard are well-made and their contract rules are executable (the contract suite's `MockEncoder` — a fixed 0x42-byte emitter with a "reject" knob — validates outcome/sink agreement and stale-drop semantics, 8/8 green when I ran it), but the entire production encode surface of this repository is: one trait, zero implementors, zero callers. The module doc says so itself ("the trait is landed now so the vocabulary is stable before any implementation exists", L4-5), the transcode crate documents its refusal to fake one (wav.rs: "implementing codec-api's packet-emitting Encoder here would fabricate a PCM-packet vocabulary no consumer exists for"), and the feature matrix's convert-dialog row is not_started/empty-evidence on every platform. So: honest vocabulary-first landing, not fraud — but any claim that this repository "encodes" anything is false, and the two in-file unit tests include a pure tautology.

## 2. What actually works (verified)
— `./target/debug/deps/codec_api-8bf0f988e15216e7 encode::` -> 2/2: stale-frame guard fires exactly for past generations (FIRST vs current vs future asserted).
— `./target/debug/deps/contract_suite-d432d069532a5b73` -> **8/8**: `encode_outcome_agrees_with_sink` and the rest of the contract rules run against the MockEncoder, so the documented contract (Emitted{n} ⇒ exactly n accepts; sink error stops and propagates; stale drop-on-sight) is enforced against at least one implementor.
— The trait surface is a faithful mirror of decode.rs (same worker model, generational cancellation, bounded-work, panic rules), so plugin implementors get a coherent vocabulary.

## 3. Findings
F-001 [S4]
Where: encode.rs:162-179
Code:
    #[test]
    fn encode_outcome_mirrors_decode_outcome() {
        assert_eq!(
            EncodeOutcome::Emitted { packets: 3 },
            EncodeOutcome::Emitted { packets: 3 }
        );
What is wrong: a pure tautology — every assertion compares a literal to an identical literal (`Emitted{3} == Emitted{3}`, `NeedMoreInput != Drained`, `Recovered{0,HwFallback} == Recovered{0,HwFallback}`). It cannot fail unless derives are deleted, and the comment/name suggest more ("mirrors decode outcome" is a design statement, not something the test checks). The real mirror-check lives in the contract suite; this unit test is padding. Confidence: HIGH — Verification: read + ran.
F-002 [S4]
Where: encode.rs:3-5
Code:
//! Encoding serves the Phase-7 transcode/record paths
//! (doc 01 registry row `transcode`); the trait is landed now so the
//! vocabulary is stable before any implementation exists.
What is wrong: the first clause reads as present-tense capability ("Encoding serves …") while the second admits nothing exists; a skimming reader takes away the first. Minor wording, but this file is exactly where an over-claim would hide. The surrounding repo is honest (wav.rs refusal, matrix not_started), so this is phrasing, not deception. Confidence: HIGH — Verification: read + rg (no impls/callers).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| trait Encoder (configure/encode/flush) | 34-57 | REAL contract / NO implementors | — | mirror of Decoder; mock-conformant |
| EncodeOutcome | 68-88 | REAL | — | #[must_use]; agreement rule executable via suite |
| trait PacketSink | 99-106 | REAL contract | — | !Send by design; propagation contract |
| drop_stale_frame | 113-119 | REAL | — | generation semantics tested |
| tests: stale_frame_guard_fires_only_for_past_generations | 144-160 | REAL (ran) | — | past/current/future asserted |
| tests: encode_outcome_mirrors_decode_outcome | 162-179 | TAUTOLOGY (ran) | S4/F-001 | literal==literal only |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| stale_frame_guard_fires_only_for_past_generations | YES (ran) | exact outcome per generation relation |
| encode_outcome_mirrors_decode_outcome | NO (ran) | tautology (F-001) |
| (external) codec-api contract suite, 8 tests (MockEncoder) | YES (ran) | outcome/sink agreement, error propagation, stale rules — but against a MOCK, so they prove the contract, not any encoder |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the trait is landed now so the vocabulary is stable before any implementation exists" | encode.rs:4-5 | REAL (honest) | rg: only MockEncoder implements it |
| "Encoding serves the Phase-7 transcode/record paths" | encode.rs:3 | PAPER-ONLY | no implementation, no consumer; transcode/runner.rs:56-58 documents the 7.E3 [PROPOSAL] seam instead |
| wav writer refusal: "implementing … Encoder here would fabricate a PCM-packet vocabulary" | transcode/wav.rs:7-14 | REAL (honest refusal) | read |
| matrix `ui-dialog-convert` (WAV PCM encode + MP4 remux) | docs/feature-matrix.toml:2915-2922 | REAL (not_started, empty evidence — matches reality) | read |

## 7. Dead / duplicated / unreachable code
- The whole module is production-unreachable (no impl, no caller). It is exported in the prelude (lib.rs:120,135) for future plugin implementors — a deliberate seam, consistently documented.

## 8. Dependencies & flags
media-types only; no unsafe; no features. Exported via prelude.

## 9. Open questions for the next agent
- If card 7.E3 lands, audit whether the factory-built Encoder reuses `drop_stale_frame` and the outcome/sink agreement rules — the contract suite's rogue-decoder/rogue-encoder catchers (`rogue_decoders_are_caught_by_the_suite`, ran green) suggest the enforcement scaffolding is ready.
