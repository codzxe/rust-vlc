# AUDIT: crates/codec-api/src/contract.rs
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2025-07-14 |
| Lines / functions | 868 / 19 public checks/sinks (+5 tests) |
| File verdict | REAL (the suite itself) — with an S2 deployment-claim violation |
| Reachable from shipped entry point? | BY DESIGN TEST-ONLY: no production call site; exercised via `crates/codecs/tests/contract.rs` (MjpegDecoder, 9 tests) and `crates/codec-api/tests/contract_suite.rs` (MockDecoder/MockEncoder, 5 lib tests). Both run clean by me. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 3 |
| Commands run | `./target/debug/deps/codec_api-8bf0f988e15216e7 contract` → 5 passed (exit 0); `./target/debug/deps/contract-245c161f566fcb85` → 9 passed (exit 0); rg consumers |

## 1. One-paragraph verdict
The checks themselves are real, sound and honestly documented about their own limits (panic-freedom not enforceable; "bounded work" enforced only as loop termination). I traced every check's logic: agreement math is right (Emitted{n} ⇔ n accepts; Recovered legally unconstrained), the stale-drop check verifies fixture staleness before blaming the decoder, flush draining requires per-call progress and stickiness, sink-failure detection survives error re-minting via a source-chain marker walk, and corrupt-input handling demands either bounded recovery with a nonzero skip or a typed Decoder/Corrupt error. The one big lie is scope: the module doc (and codec-api/src/lib.rs:26-27) claims this is "the reusable test suite **every codec in `codecs`** ... runs against its implementations" — rg proves exactly ONE consumer (crates/codecs/tests/contract.rs, MjpegDecoder only). 14 of 15 `Decoder` implementors in the codecs crate never run a single contract check.

## 2. What actually works (verified)
— verified by running both prebuilt suites: codec-api lib tests `contract` filter → "5 passed; 0 failed"; codecs integration `contract-245c161f566fcb85` → "9 passed; 0 failed" (incl. `corrupt_input_recovers_or_errors_typed`, `sink_errors_propagate_verbatim`, `stale_packets_are_dropped_on_sight` against the real MjpegDecoder). Binaries newer than source.

## 3. Findings
F-001 [S2]
Where: contract.rs:1-3 (claim), lib.rs:26-27 (same claim), vs crates/codecs/tests/contract.rs (sole consumer)
Code:
```
//! The reusable codec contract suite (doc 01 §2.4(e), card 1.D1-R C2) —
//! the executable version of the trait contracts, for every codec in
//! `codecs` and the conformance kit (MP §6/§8).
```
What is wrong: the claim "every codec in `codecs`" runs this suite is false. Repo-wide rg for `contract::` / check names finds exactly one production-repo consumer: crates/codecs/tests/contract.rs, which drives only `MjpegDecoder` (fixture_frames/configured/packet all mjpeg; verified by function inventory). The 14 other `impl Decoder for` implementors (vorbis, opus, flac, pcm, mp2, mp3, alac, aac, adpcm, av1, h264 decode.rs + sw.rs, hevc, vp9, dnx) never call any contract check. The only other suite consumer is codec-api's own contract_suite.rs against deliberately-tiny mocks. Why it matters: any reader of codec-api's docs (or a matrix row citing "contract suite") believes all codecs are contract-gated when 93% of them are not; contract drift in those 14 decoders is invisible to this suite. What correct looks like: either wire each decoder's test module to the suite or downgrade the doc to "the suite mjpeg runs today; other codecs owe wiring". Confidence: HIGH — Verification: read + rg (consumers enumerated) + executed the one wired suite.
F-002 [S4]
Where: contract.rs:390-397
Code:
```
    let mut drain = CountingFrameSink::new();
    match decoder.flush(&mut drain) {
        Ok(_) => Ok(()),
        Err(err) => Err(violation(format!(
            "decoder unusable after a stale drop: {err}"
        ))),
    }
```
What is wrong: the post-stale-drop drain check accepts ANY `Ok` outcome — including `NeedMoreInput`, which `flush_drains` itself defines as a violation ("flush cannot ask for more input"). The doc "the decoder still drains afterwards" is verified only as "flush does not error". Same weak pattern at L480-486 (corrupt-input recovery drain) and the encoder mirrors (L588-594, L669-675, L707-713). Confidence: HIGH — Verification: read.
F-003 [S4]
Where: contract.rs:435-441
Code:
```
            Ok(outcome @ DecodeOutcome::Emitted { .. }) => {
                decode_outcome_agrees_with_sink(outcome, sink.accepted() - before)?;
                if sink.accepted() == before {
                    return Err(violation(
                        "flush made no progress: Emitted without accepting a frame",
```
What is wrong: the "made no progress" branch is unreachable — `Emitted { frames: 0 }` already fails inside `decode_outcome_agrees_with_sink` (the `?` returns its violation), and `Emitted { frames: n>=1 }` requires `n == accepts-since-before`, so `accepted() > before` always. Dead defensive branch; harmless but noise. Mirror at L631-637. Confidence: HIGH — Verification: read (case analysis).
F-004 [S4]
Where: contract.rs:551-731 (encoder mirrors)
Code:
```
/// Contract mirror of [`stale_packet_is_dropped_on_sight`] for frames.
```
What is wrong: the four encoder checks have zero non-mock consumers — `rg "impl Encoder for"` finds only `MockEncoder` (codec-api/tests/contract_suite.rs:280). No production encoder exists in the repo, so half the module is exercised exclusively against mocks. Not a defect of the checks (they are honest mirrors, logic-verified), but the suite's encoder half is currently speculative surface. Confidence: HIGH — Verification: rg across crates/ and apps/.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ContractViolation + detail/Display/Error | 76-101 | REAL | — | stringly by design (documented L73-75) |
| CountingFrameSink | 107-142 | REAL | — | saturating count, retains frames (test-only memory growth acceptable) |
| CountingPacketSink | 144-180 | REAL | — | mirror |
| SinkFailureMarker / sink_failure | 186-209 | REAL | — | Present-stage Device error no decoder mints — good marker choice |
| is_sink_failure | 213-225 | REAL | — | context-code verbatim OR source-chain downcast walk — re-mint-safe, test-pinned |
| FailingFrameSink / FailingPacketSink | 230-277 | REAL | — | counts attempts → distinguishes swallow from never-called |
| decode_outcome_agrees_with_sink | 288-314 | REAL | — | math verified: Emitted{n}: n≥1 and n==accepts; NeedMoreInput/Drained: 0; Recovered unconstrained (documented) |
| encode_outcome_agrees_with_sink | 321-347 | REAL | — | exact mirror |
| stale_packet_is_dropped_on_sight | 360-397 | REAL | F-002 | fixture-staleness pre-check prevents misblaming decoder; drain check weak |
| flush_drains | 405-453 | REAL | F-003 | 1024-call bound; per-call progress; sticky-Drained re-check with fresh sink; Recovered-spin bounded by loop end → final violation |
| corrupt_input_recovers_or_errors_typed | 467-504 | REAL | — | CorruptInput recovery must skip ≥1; typed kinds enforced; silent Emitted/NeedMoreInput/Drained rejected |
| sink_errors_propagate | 515-549 | REAL | — | distinguishes swallowed vs fixture-bug vs non-sink error; post-failure drainability |
| stale_frame_is_dropped_on_sight | 558-595 | REAL | — | mirror |
| encoder_flush_drains | 602-647 | REAL | — | mirror |
| corrupt_frame_recovers_or_errors_typed | 656-693 | REAL | — | mirror |
| packet_sink_errors_propagate | 700-731 | REAL | — | mirror |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| agreement_rejects_lies_and_accepts_truth (740) | YES | truth+lie matrix for both outcomes incl. Recovered-5-accepts legality |
| sink_failure_is_detectable_verbatim_and_chained (769) | YES | verbatim, re-minted-with-source, and unrelated-error discrimination |
| failing_sinks_refuse_and_count_attempts (796) | YES | attempts counter pinned |
| counting_sinks_retain_and_reset (822) | YES | retain/reset behavior pinned |
| violation_reports_its_detail (863) | YES (trivially) | accessor round-trip |
| codecs/tests/contract.rs (9, external) | YES | real MjpegDecoder driven through stale/flush/corrupt/sink checks with damaged fixtures (per-frame damage cases L163-222) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the executable version of the trait contracts, for every codec in `codecs`" | contract.rs:2-3, lib.rs:26-27 | FALSE (scope) | F-001: sole consumer mjpeg; 14/15 implementors unwired |
| "Panic-freedom cannot be enforced without poisoning the worker; it stays a documented contract violation" | contract.rs:57-58 | REAL | honest limit statement, consistent with decode.rs docs |
| "'Bounded work' is enforced as termination" | contract.rs:54-56 | REAL | every driving loop is finite (traced: MAX_FLUSH_CALLS + single-call checks) |
| "Fixtures are honest ... The suite reports fixture bugs as violations with distinct wording" | contract.rs:44-47 | REAL | stale-check pre-verifies staleness; sink check separates attempts==0 fixture bug |

## 7. Dead / duplicated / unreachable code
- Encoder half (L551-731): zero production consumers (mock-only) — F-004.
- flush_drains/encoder_flush_drains "no progress" branches: unreachable (F-003).
- No duplication within the file beyond the documented decoder/encoder mirroring.

## 8. Dependencies & flags
Depends only on crate::decode/encode + media-types error types. No unsafe, no feature gates. Test-only module inside a shipped lib (compiled into release; never called at runtime — by design).

## 9. Open questions for the next agent
- decode.rs owner (trait docs): confirm the trait-level promises match these checks (esp. "fatality is per-generation" L462, and `drop_stale_packet` helper semantics).
- Whoever owns codecs/tests/contract.rs: the 14 unwired decoders (F-001) — each decoder's own unit tests may cover similar ground informally; the suite-wiring debt is the claim violation.
