# AUDIT: crates/codec-api/src/contract.rs#part2 (lines 434–868)
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-21 |
| Lines / functions | 435 audited (chunk part2 of 869; nominal split 434 — `flush_drains` (405-453) crosses it, audited whole here with boundary note; part1 = 1-433 is another agent's: docs, sinks, helpers, agreement checks, `stale_packet_is_dropped_on_sight`, `flush_drains` head) / 7 production check fns + 5 tests |
| File verdict | REAL (test-kit API, by design production-unreachable) |
| Reachable from shipped entry point? | NO at runtime by design — consumer census (`rg "codec_api::contract" crates/ apps/`): exactly `crates/codecs/tests/contract.rs` (the MJPEG conformance run) and `crates/codec-api/tests/contract_suite.rs` (mocks + rogues). EXECUTED in baseline: 5/5 in-file tests ok (log :246-250), 8/8 contract_suite ok incl. `rogue_decoders_are_caught_by_the_suite` (:259-269), 9/9 MJPEG suite with real fixture frames (:911-923). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 1 |
| Commands run | Read tool full file + `contract_suite.rs` (rogue coverage census); `rg "codec_api::contract\|contract::"` over crates/+apps/ (2 consumers); baseline log greps (:246-250, :259-269, :911-923); `grep -c "#\[test\]" contract_suite.rs` = 8 vs baseline 8 passed; `grep "RogueEncoder"` → absent |

## Chunk boundary note
Part2 = 434–868: tail of `flush_drains` (boundary function — audited whole), `corrupt_input_recovers_or_errors_typed`, `sink_errors_propagate`, the four encoder mirrors (558-731), `mod tests` (733-868).

## 1. One-paragraph verdict
This chunk is the enforcement half of the codec contract suite: flush termination with sticky-Drain semantics and a 1024-call bound, corrupt-input disposition (Recovered-with-skips or typed Decoder/Corrupt error, never silent success), sink-error propagation proven through a source-chain marker, and four encoder mirrors. The logic is correct on every branch I traced, and it is executed three ways in the baseline (unit, mock+rogue, real MJPEG fixture run). Three real defects: the "every codec runs this suite" claim is false (one codec does), the encoder half is a ~170-line copy-paste of the decoder half, and the rogue (negative) tests cover only the decoder side — every encoder check in this chunk could be gutted to `Ok(())` and the entire suite would still pass.

## 2. What actually works (verified)
— `contract::tests` 5/5 ok (baseline :246-250); `contract_suite` 8/8 ok incl. the rogue test (:259-269); MJPEG conformance `tests/contract.rs` 9/9 ok on the 60 real AVI fixture frames (:911-923) — this chunk's `flush_drains`, `corrupt_input_recovers_or_errors_typed`, and `sink_errors_propagate` all ran against a real decoder with real bytes.
— Read-traced: the `MAX_FLUSH_CALLS` progress/termination argument holds (an always-`Recovered` decoder with zero accepts is caught by the loop bound, not just per-call agreement); sticky-Drain is verified with a fresh sink; the `is_sink_failure` source-chain walk handles re-minted errors (unit-tested both ways, :768-793).
— `corrupt_input_recovers_or_errors_typed` pins the full outcome matrix: Recovered needs `skipped ≥ 1` for CorruptInput, Err must be Decoder/Corrupt kind, and all three silent-success outcomes are named violations (494-502).

## 3. Findings

### F-001 [S3]
Where: crates/codec-api/src/contract.rs:2-3 (module doc, part1 range — claim verified here because the consumers are this suite's), crates/codec-api/src/lib.rs:26-27
Code:
```
//! The reusable codec contract suite (doc 01 §2.4(e), card 1.D1-R C2) —
//! the executable version of the trait contracts, for every codec in
//! `codecs` and the conformance kit (MP §6/§8).
```
What is wrong: "every codec in `codecs`" — the census shows exactly ONE codec test file consumes the suite (`crates/codecs/tests/contract.rs`, MJPEG). No other decoder (pcm, flac, aac, mp3, mp2, vorbis, opus, alac, adpcm, h264-sw, hevc, av1, vp9, dnx) runs it; they have their own local tests (flac has a local `contract_tests` mod, others nothing contract-shaped).
Why it matters: the suite is the claimed mechanism enforcing doc 01 §2.4(e) uniformly; if only the first-vertical codec is actually enrolled, every later codec's contract conformance (stale-drop, drain, corrupt-input, sink-propagation) is asserted nowhere — precisely the class of defect my other audits found (e.g., hevc configure's non-atomic error path, dnx pool tautology test) would have been caught by enrollment.
What correct looks like: either enroll each decoder (a one-line-per-codec conformance test file, as MJPEG's), or weaken the claim to "available to every codec; currently run by the MJPEG conformance kit".
Confidence: HIGH — Verification: read + census commands; baseline log lines cited.

### F-002 [S3]
Where: crates/codec-api/src/contract.rs:558-731 (four mirrors of 360-549)
Code:
```
pub fn stale_frame_is_dropped_on_sight<E: Encoder>(
    encoder: &mut E,
    stale: &Frame,
    current: GenerationId,
) -> Result<(), ContractViolation> {
```
What is wrong: the four encoder checks are near-verbatim copies of the decoder checks with Packet↔Frame and DecodeOutcome↔EncodeOutcome substituted (~170 duplicated lines; `stale_frame_is_dropped_on_sight` 558-595 vs `stale_packet_is_dropped_on_sight` 360-397 differ only in nouns; same for the flush/corrupt/sink-propagation pairs). The comments even say "Contract mirror of …".
Why it matters: any future contract amendment (a new outcome variant, a changed drain rule) must be applied twice; the decoder/encoder halves can drift — and per F-003 nothing would catch the encoder half regressing.
What correct looks like: a generic core (parameterized over unit type, sink trait, and outcome enum via a small trait or macro) with the four thin wrappers, halving the file.
Confidence: HIGH — Verification: read (all eight functions compared).

### F-003 [S3]
Where: crates/codec-api/src/contract.rs:558-731 (the uncovered half); evidence: crates/codec-api/tests/contract_suite.rs:443-450 (Rogue enum) and :499-533 (the only rogue test)
Code:
```
enum Rogue {
    /// Decodes stale packets instead of dropping them.
    EmitOnStale,
    /// Swallows sink errors and reports success.
    SwallowSinkErrors,
    /// `flush` never reaches Drained.
    NeverDrain,
}
```
What is wrong: the suite's negative tests exist only for THREE decoder misbehaviors. There is no `RogueEncoder` at all (`grep RogueEncoder` → absent), no rogue for `corrupt_input_recovers_or_errors_typed`'s "corruption went undetected" branches (an `Ok(Emitted)`-on-corrupt rogue), and none for the fixture-bug branches. Consequence: every check function in THIS chunk except `sink_errors_propagate` and `flush_drains`'s bound is tested only in the pass direction — `stale_frame_is_dropped_on_sight`, `encoder_flush_drains`, `corrupt_frame_recovers_or_errors_typed`, `packet_sink_errors_propagate` (and `corrupt_input_recovers_or_errors_typed`'s violation arms) could be replaced with `Ok(())` and `mock_encoder_passes_the_contract_suite` + all 8 contract_suite tests would still pass.
Why it matters: the file's own doc header claims it "proves the suite itself catches misbehavior via rogues" (:3-4) — for the encoder half that proof does not exist. A suite that cannot fail is not evidence.
What correct looks like: a `RogueEncoder` with the same three variants plus a `SilentOnCorrupt` variant, and a `rogue_encoders_are_caught_by_the_suite` test mirroring :499-533; plus a decoder rogue returning `Ok(Emitted)` on the corrupt fixture.
Confidence: HIGH — Verification: read (test file enumerated; 8/8 baseline tests enumerated against the 8 `#[test]` fns).

### F-004 [S4]
Where: crates/codec-api/src/contract.rs:533-535 (and mirror 715-717)
Code:
```
        Err(err) => Err(violation(format!(
            "decode failed for a non-sink reason — supply a packet that emits: {err}"
        ))),
```
What is wrong: when `decode` fails with a non-sink error, the message instructs the caller to "supply a packet that emits" — diagnosing a fixture bug. But the branch is also reached when the FIXTURE is fine and the DECODER errors before its first accept for its own reason (e.g. a typed BadState on a valid packet). The check correctly fails either way, but the message misattributes decoder bugs to the fixture.
Why it matters: a codec author running this suite against a broken decoder gets pointed at their fixtures, not their decoder; wasted triage time. The error text is included (`: {err}`), which mitigates.
What correct looks like: neutral wording — "decode failed before/without a sink failure (decoder error or non-emitting fixture): {err}".
Confidence: HIGH — Verification: read (branch reachability traced).

## 4. Per-function table (part2: lines 434–868)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `flush_drains` (boundary fn) | 405-453 | REAL | — | MAX_FLUSH_CALLS=1024 bound; per-call progress on Emitted; Recovered tolerated per-call but caught by loop bound; sticky-Drain via fresh sink; Drained-with-accepts-delta is a violation (via agreement check) — all correct; NOTE: insists flush never errs while draining — hevc's `flush` CAN err (`hevc.hw.decode` on DeviceLost, hevc/decode.rs:745) — latent contract-vs-impl mismatch, unenforced since hevc never runs the suite (F-001) |
| `corrupt_input_recovers_or_errors_typed` | 467-504 | REAL | S3 | full outcome matrix; skipped ≥ 1 for CorruptInput; post-Recovered drainability; no rogue for the undetected branches (F-003) |
| `sink_errors_propagate` | 515-549 | REAL | S4 | marker-chain detection; drainability after downstream failure; F-004 message; rogue-covered (EmitOnStale sibling) ✓ |
| `stale_frame_is_dropped_on_sight` | 558-595 | REAL | S3 | F-002 mirror; F-003 no rogue |
| `encoder_flush_drains` | 602-647 | REAL | S3 | F-002 mirror; F-003 no rogue |
| `corrupt_frame_recovers_or_errors_typed` | 656-693 | REAL | S3 | F-002 mirror; F-003 no rogue |
| `packet_sink_errors_propagate` | 700-731 | REAL | S3/S4 | F-002 mirror + F-004 mirror; no rogue |
| test `agreement_rejects_lies_and_accepts_truth` | 739-766 | REAL | — | exhaustive truth/lie table both enums — strong |
| test `sink_failure_is_detectable_verbatim_and_chained` | 768-793 | REAL | — | verbatim + chained + unrelated-negative — strong |
| test `failing_sinks_refuse_and_count_attempts` | 795-819 | REAL | — | attempts counter pinned |
| test `counting_sinks_retain_and_reset` | 821-860 | REAL | — | retain/reset pinned |
| test `violation_reports_its_detail` | 862-867 | REAL | — | trivial, correct |

## 5. Test honesty
In-file: 5/5 ok (baseline :246-250). Integration: 8/8 contract_suite ok (:259-269) incl. `rogue_decoders_are_caught_by_the_suite`; 9/9 MJPEG conformance ok (:911-923). Gap analysis = F-003.
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| agreement_rejects_lies_and_accepts_truth | YES | both directions, both enums, Emitted{0} lie included |
| rogue_decoders_are_caught_by_the_suite | YES (decoder checks) | three misbehaviors → matching violation substrings; but decoder-only (F-003) |
| mock_decoder/encoder_passes_the_contract_suite | NO (for a gutted check) | pass-direction only — a check stubbed to Ok(()) still "passes" (F-003) |
| contract_suite_accepts_a_typed_fatal_error_for_corrupt_input | YES | pins the Err-typed acceptance branch |
| MJPEG conformance (codecs/tests/contract.rs, 9 tests) | YES | real fixture frames; stale/drain/corrupt/sink paths driven with real bytes |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the reusable test suite every codec in `codecs` and the conformance kit runs against its implementations" | lib.rs:26-27, contract.rs:2-3 | FALSE (one codec runs it) | F-001 census |
| "'Bounded work' is enforced as termination: every check is a finite driving loop" | contract.rs:54-56 | REAL | MAX_FLUSH_CALLS loops; NeverDrain rogue caught |
| "Panic-freedom cannot be enforced without poisoning the worker; it stays a documented contract violation" | contract.rs:57-58 | REAL (honest limit) | no catch_unwind anywhere in the suite |
| "corrupt input producing bounded recovery (`Recovered` with a sane `skipped` count) or a *typed* fatal error; silent success is a violation" | contract.rs:51-53 | REAL | 467-504 implements exactly this matrix; MJPEG run green |
| "proves the suite itself catches misbehavior via rogues" | contract_suite.rs:3-4 | SHALLOW | rogues cover 3 decoder behaviors only; encoder half unproven (F-003) |

## 7. Dead / duplicated / unreachable code
Production-unreachable by design (test-kit; see header). Duplication: F-002's four mirror pairs. No other dead code — every pub check is consumed by the two test files (census).

## 8. Dependencies & flags
Internal: `crate::decode`, `crate::encode`, media-types error/frame/packet types. No external deps, no unsafe, no feature gates. `#[must_use = "a contract check's result is the test's verdict"]` on all checks — correct and load-bearing.

## 9. Open questions for the next agent
- decode.rs/encode.rs trait-doc owners: does the trait contract actually promise "flush must not fail while draining"? hevc/decode.rs:744-745 CAN fail on flush (DeviceLost → Err) — either the trait doc carves out hardware-loss or the hevc flush is non-conforming; nothing today enforces either answer (F-001).
- Who owns enrolling the other 13 decoders in this suite (F-001)? That single change would have caught hevc configure's non-atomic error path (my hevc/decode.rs F-002) mechanically.
- corpus.rs observation for the wave lead (not my file): `tests/corpus.rs` has 2 `#[ignore]`d randomized-corpus tests that never run in CI (baseline :925-931 "ignored, randomized-corpus harness") — the "randomized-corpus vertical (ED-FIX-5)" runs only via a manual Python harness.
