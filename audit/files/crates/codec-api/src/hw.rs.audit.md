# AUDIT: crates/codec-api/src/hw.rs
| Field | Value |
|---|---|
| Auditor | W06-A03 (recovery: executed by W06-R1, 2026-11-23) |
| Date | 2026-11-23 |
| Lines / functions | 73 / 1 trait (2 methods) + 1 test |
| File verdict | REAL — a genuinely abstract 2-method contract seam; every doc claim in the file checked out against the tree |
| Reachable from shipped entry point? | YES as a type: `DecoderConfig.accelerator: Option<Arc<dyn HwAccelerator>>` (config.rs:89) rides the shipped decode path (playback-engine/pipeline.rs:1708 `provider.decoder_for(track, &config)`); but NO instance is ever minted in the default release lane — sole production mint site is h264 hw/capability.rs:360, inside the opt-in hw lane whose decoder wrapper has zero non-test instantiation (see capability.rs audit, W06-A10) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/codec_api-8bf0f988e15216e7 hw` -> 1 passed (exit 0); full lib run -> 16 passed / 0 failed (exit 0); rg impl/users census; sed quote verification |

## 1. One-paragraph verdict
This is the ADR-001 hardware seam: a two-method trait
(`backend_id` + `as_any`) that deliberately leaks no platform type, and it
does exactly what its documentation claims — nothing more, nothing hidden.
I verified every concrete claim: the only imports are std (`std::any::Any`,
`std::sync::Arc` in tests) — "no platform types" is true; the "Concrete
adapters (Phase 7.B) implement it" promise is real — doc 13:708-713 defines
Wave 7.B and the tree contains exactly two non-test implementors
(`impl HwAccelerator for VaApiAccelerator`, vaapi/mod.rs:151, Linux-gated;
`for VideoToolboxAccelerator`, videotoolbox/mod.rs:112, macOS-gated), with
the other four `impl` sites being test mocks; the dotted-id example
("hw.mock") matches the real minted ids "hw.vaapi"/"hw.videotoolbox"
(capability.rs:70-72); and the "unknown backend is a typed configure error,
never a panic" claim is upheld by the in-tree consumers (both `open()`
seams return Option, hevc decode.rs:568 maps None to the typed
hevc.hw refusal). The single test executes green. The only defect is
cosmetic: the crate's test coverage of this seam is entirely
self-referential (a mock testing the mock's own downcast).

## 2. What actually works (verified)
— verified by `./target/debug/deps/codec_api-8bf0f988e15216e7 hw` ->
`test hw::tests::accelerator_identity_and_downcast_round_trip ... ok` (1
passed, 0.00s); full prebuilt lib suite -> 16 passed / 0 failed, exit 0.

## 3. Findings

F-001 [S4]
Where: crates/codec-api/src/hw.rs:63-72
Code:
```rust
    #[test]
    fn accelerator_identity_and_downcast_round_trip() {
        let accel: Arc<dyn HwAccelerator> = Arc::new(MockAccelerator);
        assert_eq!(accel.backend_id(), "hw.mock");
        // The downcast seam finds the concrete type behind the erasure.
        let concrete = accel.as_any().downcast_ref::<MockAccelerator>().is_some();
        assert!(concrete, "downcast must find the concrete backend");
```
What is wrong: the test's entire evidence base is the mock defined 10 lines
above it. It proves the seam pattern compiles and behaves for a correct
impl — which is all a pure trait can prove in isolation — but no production
implementor is ever exercised through this file (the real downcast
consumers live in vaapi/videotoolbox/hevc and are covered by their own
tests). Blast radius: none today; the risk is purely that a future change
to the trait's object-safety or supertraits would be caught here, which is
in fact the test's honest purpose.
Confidence: HIGH — Verification: ran (test passes; mock-only coverage
confirmed by impl census).
Note: acceptable for a contract crate; recorded because the checklist
demands mock-of-thing-tested be flagged.

## 4. Per-function table (file <100 LOC, included for completeness)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `HwAccelerator::backend_id` | 37 | REAL | - | label only, no capability semantics (doc honest) |
| `HwAccelerator::as_any` | 43 | REAL | - | `[PROPOSAL]`-labeled downcast seam; consumers uphold typed-error claim |
| `tests::accelerator_identity_and_downcast_round_trip` | 64-72 | REAL (mock-scoped) | S4 | F-001 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| accelerator_identity_and_downcast_round_trip | PARTLY | Fails if `as_any` stops returning `self` or the trait loses object safety; cannot catch anything about real adapters (none in crate) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no platform types ... names no API, no device, no surface format" | hw.rs:6-7 | REAL | only std imports; 2 methods verified |
| "Concrete adapters (Phase 7.B) implement it" | hw.rs:8 | REAL | doc 13:708 Wave 7.B; impl sites vaapi/mod.rs:151, videotoolbox/mod.rs:112 |
| "instances exist only behind the audited per-OS adapters" | hw.rs:19-20 | REAL | impl census: 2 platform-gated adapters + 4 test mocks; sole Some() mint = capability.rs:360 (opt-in lane) |
| "backend_id ... Registry-minted dotted id (e.g. `hw.mock`)" | hw.rs:34-35 | REAL | real ids "hw.vaapi"/"hw.videotoolbox" (capability.rs:70-72) |
| "an unknown backend is a typed configure error, never a panic" | hw.rs:41-42 | REAL | vaapi/mod.rs:171 + videotoolbox/mod.rs:132 `open() -> Option`; hevc decode.rs:568 maps None to typed refusal |

## 7. Dead / duplicated / unreachable code
None dead in-file. Cross-file context (not this file's defect): every
`HwAccelerator` instance reachable in the default shipped lane is `None`;
the two real adapters are cfg-gated off the default release build lane
(Linux vaapi opt-in via playback.hw-decode-policy whose decoder wrapper is
itself uninstantiated — capability.rs audit; videotoolbox macOS-only).

## 8. Dependencies & flags
std-only (std::any, std::sync in tests). No features. No unsafe.

## 9. Open questions for the next agent
- If the hw lane ever ships, does `backend_id` need a stability guarantee
  (UI/config matching on the string)? Currently documented as a label only.
