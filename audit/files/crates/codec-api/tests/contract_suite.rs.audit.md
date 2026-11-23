# AUDIT: crates/codec-api/tests/contract_suite.rs
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-23 |
| Lines / functions | 721 / 7 tests + 21 helpers/impls (2 mock codecs, 1 rogue codec) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — integration test for codec-api's generic contract suite; test-only by design (not a defect). The suite it exercises (`codec_api::contract`, src/contract.rs, 868 loc) IS shipped in the codec-api lib that lumen-cli links. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cargo test -p codec-api --test contract_suite` → **8 passed / 0 failed** (0.00s, compiled within budget). `./target/debug/deps/contract-245c161f566fcb85` (codecs' contract.rs, the real-decoder consumer of the same suite) → 9 passed / 0 failed. `./target/debug/deps/codec_api-8bf0f988e15216e7 --list` (lib unit tests exist separately). |

## 1. One-paragraph verdict
A meta-test that does exactly what its header claims: drives the generic `codec_api::contract` suite with two deliberately tiny mock codecs (documented opcode mini-language) and — the part that makes it honest — three rogue decoders that violate the contract in specific ways (emit-on-stale, swallowed sink errors, non-draining flush) and asserts the suite CATCHES each one with a matching violation. This is the correct way to test a test-suite: prove both acceptance of compliant codecs and rejection of non-compliant ones. The mocks are not masquerading as real decoders; the doc says so up front. The chain is closed by codecs' own contract.rs, which runs the same suite functions against the REAL MJPEG/PCM decoders (9/9 pass, verified). One hygiene finding only.

## 2. What actually works (verified)
- `cargo test -p codec-api --test contract_suite` → 8/8 pass: mock_decoder_passes_the_contract_suite, contract_suite_accepts_a_typed_fatal_error_for_corrupt_input, mock_encoder_passes_the_contract_suite, rogue_decoders_are_caught_by_the_suite, mock_decoder_outcome_language_behaves_as_documented, mock_encoder_gop_and_recovery_behave_as_documented, unconfigured_use_and_configure_rejection_are_typed_errors, trait_objects_and_configs_are_send.
- The suite functions invoked are real assertions in src/contract.rs (L288-868: `decode_outcome_agrees_with_sink`, `stale_packet_is_dropped_on_sight`, `flush_drains`, `corrupt_input_recovers_or_errors_typed`, `sink_errors_propagate`, encoder mirrors) — each returns `Result<(), ContractViolation>` and the rogues prove the `Err` arms fire (test asserts on violation detail text).
- Real-codec closure: codecs/tests/contract.rs passes 9/9 against actual MjpegDecoder/PcmDecoder (run pasted above), so the suite is not mock-only in effect.

## 3. Findings
F-001 [S4]
Where: crates/codec-api/tests/contract_suite.rs:507-532
Code:
```rust
    match contract::stale_packet_is_dropped_on_sight(
        &mut rogue,
        &packet(OP_EMIT, GenerationId::FIRST),
        current,
    ) {
        Err(v) => assert!(v.detail().contains("NeedMoreInput"), "{}", v.detail()),
```
What is wrong: the rogue tests assert the suite's *human-readable* violation detail strings (`.contains("NeedMoreInput")`, `.contains("swallowed")`, `.contains("never reached Drained")`); a reworded message silently breaks the test's intent-check while still passing semantically different wording. A typed violation enum would pin the class, not the prose. Test-only brittleness. Confidence: HIGH — Verification: read.
(Note: the underlying suite does distinguish violations structurally via `ContractViolation`; only this test's match is stringly.)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| current_generation | 21-25 | REAL | — | FIRST.next() |
| packet | 28-39 | REAL | — | opcode mini-language packet |
| nv12_frame / video_frame / corrupt_video_frame / audio_frame | 42-83 | REAL | — | 2×2 NV12 (6 bytes), corrupt = wrong plane math |
| bad_state | 85-92 | REAL | — | typed Decoder{BadState} |
| MockDecoder::new / configured_at | 117-132 | REAL | — | |
| decoder_config | 134-142 | REAL | — | descriptor "mock.video" |
| frame_from | 144-146 | REAL | — | echoes packet pts/generation into the frame |
| MockDecoder::configure | 149-161 | REAL | — | "reject" substring → typed Unsupported |
| MockDecoder::decode | 163-199 | REAL | — | stale guard first; EMIT/BUFFER/CORRUPT/FATAL/default-recovered opcodes |
| MockDecoder::flush | 201-223 | REAL | — | releases pending frames then Drained sticks |
| MockEncoder::{new, configured_at} + configure/encode/flush | 225-352 | REAL | — | 2-frame GOP buffering, NV12 plane-math corruption check |
| nv12_bytes / packet_from | 258-278 | REAL | — | 3/2-plane byte math |
| Rogue enum + RogueDecoder impl | 445-496 | REAL | — | three contract violations, one per variant |
| mock_decoder_passes_the_contract_suite (test) | 354-389 | REAL | — | 4 suite fns driven |
| contract_suite_accepts_a_typed_fatal_error_for_corrupt_input | 390-398 | REAL | — | fatal branch is legal |
| mock_encoder_passes_the_contract_suite | 400-432 | REAL | — | 4 encoder suite fns |
| rogue_decoders_are_caught_by_the_suite | 498-532 | REAL | S4 | stringly detail matching (F-001) |
| mock_decoder_outcome_language_behaves_as_documented | 534-590 | REAL | — | frame generation echo, buffering, unknown opcode, sink failure propagation + drainable after |
| mock_encoder_gop_and_recovery_behave_as_documented | 592-637 | REAL | — | GOP semantics + malformed skip |
| unconfigured_use_and_configure_rejection_are_typed_errors | 639-694 | REAL | — | BadState + Unsupported paths |
| trait_objects_and_configs_are_send | 696-720 | REAL | — | compile-time Send/Sync; documented non-assertion for FrameSink |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| rogue_decoders_are_caught_by_the_suite | YES | a suite that stopped detecting stale-emit/swallow/no-drain would hit the `unreachable!` arms |
| mock_decoder_passes_the_contract_suite | YES (suite-side) | suite fns return Err on rule violations, unwrapped here |
| mock_decoder_outcome_language_behaves_as_documented | YES | asserts exact outcomes, sink counts, generation echo, error context codes |
| unconfigured_use_and_configure_rejection_are_typed_errors | YES | matches! on ErrorKind + context codes |
| trait_objects_and_configs_are_send | YES (compile-time) | trait bounds fail compilation if removed |
Honesty caveat: none material — the mocks are of *callers'* codecs on purpose, and the file proves the suite itself has teeth rather than pretending a mock is a real codec.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "proves the suite itself catches misbehavior via rogues" | header L3-5 | REAL | rogue test passes with 3 violations caught |
| "mocks are deliberately tiny fake codecs ... not real decoders" | header L7-9 | REAL | honest labeling; real-decoder coverage lives in codecs/tests/contract.rs (9/9 pass) |
| "the compile_fail doctest on `Decoder` is the negative proof" (Send) | L706-708 | REAL-UNPROVEN | doctest not executed in this run; compile-time positive assertions executed |

## 7. Dead / duplicated / unreachable code
None. Every helper is consumed by at least one test in the file.

## 8. Dependencies & flags
codec-api + media-types only; no external crates; no unsafe; no feature gates; not cfg-gated out of any lane (integration test compiled by `cargo test -p codec-api`).

## 9. Open questions for the next agent
- src/contract.rs itself (868 loc) is NOT in my assignment — the suite's internal assertion quality (e.g. `flush_drains` iteration bound, `decode_outcome_agrees_with_sink` corner cases) should get its own audit pass.
- Does any CI lane run `cargo test -p codec-api --test contract_suite` (the orchestrator baseline ran per-package tests: 4744 passed — presumably included)?
