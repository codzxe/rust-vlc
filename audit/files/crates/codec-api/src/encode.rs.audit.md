# AUDIT: crates/codec-api/src/encode.rs
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-21 |
| Lines / functions | 180 / 8 (Encoder trait, EncodeOutcome, PacketSink, drop_stale_frame, 2 tests + 1 helper) |
| File verdict | UNREACHABLE |
| Reachable from shipped entry point? | NO implementation and NO shipped caller exist: rg `impl Encoder for` over crates/ apps/ → zero hits outside codec-api's own test mocks; transcode's own docs admit it (crates/transcode/src/lib.rs:126-128: "No codec encoders — WAV PCM is container serialization; the codec-api Encoder seam joins with card 7.E3"); product.toml:164-165 `recording = false`, `transcode = false` (Phase 7); `lumen engine-info` (baseline lumen-engine-info.txt) lists decoders only |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg "impl Encoder for"/"EncodeOutcome|PacketSink" repo-wide (callers: plugin-api re-export only); rg Encoder in crates/transcode/src (doc admission); baseline test log :253-254 (both tests executed ok); read of engine-info baseline |

## 1. One-paragraph verdict
A deliberately pre-landed vocabulary: the Encoder trait, EncodeOutcome enum,
PacketSink seam and the stale-frame guard, all mirroring the decode side
correctly (I verified the symmetry variant-for-variant and the stale-guard's
three-generation behavior is genuinely tested and executed in baseline
:253-254). The module doc is honest that no implementation exists ("the trait
is landed now so the vocabulary is stable before any implementation exists").
Nothing shipped reaches it — no production impl, no caller, transcode/recording
both Phase-7 false in product.toml. Two findings: the test named
`encode_outcome_mirrors_decode_outcome` is a tautology — it asserts derived
`PartialEq` on the enum's own variants and cannot fail on any divergence from
DecodeOutcome (the name's promise); and the trait ships in the production
library for consumers that do not exist yet (same class as the contract-suite
S4 in my contract.rs#part1 audit).

## 2. What actually works (verified)
— `stale_frame_guard_fires_only_for_past_generations` (144-160): executed,
baseline :254 — past generation → Some(NeedMoreInput), current → None, FUTURE
generation → None (correctly not stale). Would fail if `drop_stale_frame` or
`Frame::is_stale` regressed.
— Symmetry with decode verified by read: EncodeOutcome's four variants mirror
DecodeOutcome's (Emitted/NeedMoreInput/Drained/Recovered{skipped,reason}),
`drop_stale_frame` mirrors `drop_stale_packet` (decode.rs:203-209), the
sink-propagation and no-panic contract text matches the Decoder contract.
— The contract suite exercises the encode side with mocks:
`mock_encoder_passes_the_contract_suite` + `mock_encoder_gop_and_recovery_
behave_as_documented` — baseline :265-266 ok (the CHECKS are real even though
no production encoder exists).

## 3. Findings

### F-001 [S3] `encode_outcome_mirrors_decode_outcome` is a tautology — it cannot fail on mirror divergence
Where: encode.rs:162-179
Code:
```rust
    #[test]
    fn encode_outcome_mirrors_decode_outcome() {
        assert_eq!(
            EncodeOutcome::Emitted { packets: 3 },
            EncodeOutcome::Emitted { packets: 3 }
        );
        assert_ne!(EncodeOutcome::NeedMoreInput, EncodeOutcome::Drained);
```
What is wrong: every assertion compares literals of the SAME variants — this
exercises the compiler's derived `PartialEq`/`assert_ne`, not any subject
code. The name promises a mirror-structure check against `DecodeOutcome`
(variant parity, same payload shapes, same agreement semantics), but if
DecodeOutcome gained a variant, changed a payload type, or diverged in any
way, this test still passes. It is vacuous coverage: green output that
certifies nothing.
Why it matters: the checklist's "tests asserting nothing" class; a future
wave touching the outcome vocabulary will see this test pass and trust the
mirror is enforced.
What correct looks like: a real mirror test — e.g. exhaustively match both
enums and assert corresponding variants (a compile error on divergence), or
assert `encode_outcome_agrees_with_sink` accepts/rejects the same outcome
shapes as its decode twin.
Confidence: HIGH — Verification: read; executed in baseline (:253) — passing
proves nothing.
Severity: S3 (misleading coverage signal on a contract type).

### F-002 [S4] Unimplemented vocabulary ships in the production library
Where: crates/codec-api/src/lib.rs (pub mod encode) + this file's own doc 3-5
Code:
```rust
//! Encoding serves the Phase-7 transcode/record paths
//! (doc 01 registry row `transcode`); the trait is landed now so the
//! vocabulary is stable before any implementation exists.
```
What is wrong: the trait, outcome and sink types compile into every shipped
`lumen` binary (codec-api is a dependency of codecs → apps) with zero
implementations and zero callers; plugin-api re-exports them
(crates/plugin-api/src/codec.rs:20) as if a plugin surface existed to consume
them today.
Why it matters: dead shipping surface plus a re-export that advertises an
encoder plugin API that cannot work end-to-end yet (nothing can construct a
useful Encoder).
What correct looks like: fine to pre-land (the doc says why — vocabulary
stability), but the plugin-api re-export should be feature-gated or documented
as inert until 7.E3.
Confidence: HIGH — Verification: rg callers; transcode crate's own admission
(transcode/src/lib.rs:126-128).
Severity: S4 (documented decision, bounded).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| trait Encoder (configure/encode/flush) | 34-57 | REAL | — | correct mirror of Decoder's contract text; Send supertrait; no impl exists anywhere (UNREACHABLE) |
| enum EncodeOutcome (4 variants) | 66-88 | REAL | — | mirrors DecodeOutcome exactly; #[must_use] |
| trait PacketSink | 99-106 | REAL | — | mirror of FrameSink; not-Send reasoning documented |
| drop_stale_frame | 113-119 | REAL | — | mirror of drop_stale_packet; behavior pinned by executed test |
| tests::audio_frame_at (helper) | 128-141 | REAL | — | minimal valid audio frame fixture |
| test stale_frame_guard_fires_only_for_past_generations | 144-160 | REAL | — | executed (:254); real three-generation pin; would fail on regression |
| test encode_outcome_mirrors_decode_outcome | 162-179 | FALSE (as a test) | S3 | tautology (F-001); executed (:253) — passing is meaningless |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| stale_frame_guard_fires_only_for_past_generations | Yes | real behavior pin incl. the future-generation-is-not-stale case |
| encode_outcome_mirrors_decode_outcome | No | all operands are literals of the same enum; derived PartialEq is compiler behavior, not subject code (F-001) |
| (cross-file) mock_encoder_passes_the_contract_suite | Yes — for the SUITE's encode checks | baseline :265 ok; the contract checks (part1/part2 of contract.rs) are real even though the encoder is a mock of a trait with no production impl |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the trait is landed now so the vocabulary is stable before any implementation exists" | encode.rs:3-5 | REAL (honest) | rg: zero impls; transcode/src/lib.rs:126-128 admits the gap |
| "Encoding serves the Phase-7 transcode/record paths (doc 01 registry row transcode)" | encode.rs:3-4 | PAPER-ONLY today | product.toml:164-165 transcode=false, recording=false (Phase 7); registry row is future work |
| Outcome/sink agreement "applies symmetrically" | encode.rs:62-65 | REAL | contract::encode_outcome_agrees_with_sink exists and mirrors (my contract.rs#part1 audit) |

## 7. Dead / duplicated / unreachable code
- The entire module is unreachable from shipped entry points (no impl, no
  caller). Unlike the vaapi lane (compiled per-OS, waiting on a policy), this
  is vocabulary-only: nothing can construct an Encoder at all.
- plugin-api's re-export (codec.rs:20) is the only consumer surface — an
  inert advertisement.

## 8. Dependencies & flags
- No features, no unsafe, media-types + crate-internal config only.
- Compiles into shipped binaries via the codecs→codec-api edge (see F-002).

## 9. Open questions for the next agent
- Wave owning transcode (7.E3): when the first real Encoder lands, replace
  the tautology (F-001) with a structural mirror test, and re-check the
  plugin-api re-export's feature story.
- Is `EncodeOutcome::Recovered::reason`'s `HwFallback` variant meaningful for
  encode? It exists because RecoverReason is shared with decode — harmless
  today, worth a look when an encoder exists.
