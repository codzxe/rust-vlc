# AUDIT: crates/codecs/src/video/h264/hw/capability.rs
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2025-07-14 |
| Lines / functions | 584 / 14 (+7 tests) |
| File verdict | UNREACHABLE (real, correct, tested policy code — no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — `negotiate` ← `hw/mod.rs:501 negotiate_lane` ← `decode.rs:312 H264Decoder` configure; `H264Decoder` has ZERO instantiation sites outside `crates/codecs/tests/h264_hw_path.rs` (rg over crates/ + apps/; the sole apps/ mention is a comment, pipeline_host.rs:342 "stays the opt-in Phase-7 path"). The shipped CLI dispatches `H264SwDecoder` (pipeline_host.rs:346). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/codecs-bae0f1756cc0a809 h264::hw::capability` → 7 passed / 0 failed (exit 0) |

## 1. One-paragraph verdict
The negotiation logic itself is genuinely good: pure policy-over-facts (no device touch in this file), every outcome typed, SoftwareOnly provably never activates (test-pinned), "usable probe but no accelerator minted" reported as typed DriverIncompatible instead of a fake Active, and the OFF-means-off probe refusal is synthesized by the caller without touching hardware (hw/mod.rs:484-496, verified consistent with the L257-260 trail semantics). All 7 unit tests pass under my run. The problem is reachability: this entire layer — the "only place the hw-active truth is decided" per its own header — is dead code in the shipped product. The H.264 hw decoder that drives it is never constructed by any shipped entry point, and the `hw-active` event trail it produces is consumed only by tests (`hw_events()` readers: h264_hw_path.rs only). The module's own docs mark the engine seam `[VERIFY: engine-side wiring is the registry card's scope]` — an honest admission that the report surface does not exist yet.

## 2. What actually works (verified)
— verified by `./target/debug/deps/codecs-bae0f1756cc0a809 h264::hw::capability` → "7 passed; 0 failed" (policy vocabulary round-trip + rejection matrix, software-only-never-activates, prefer-hardware activation, typed unavailability fallback, honest-inactive-on-missing-accelerator, label stability, user-renderable reasons). Cross-file consistency of the OFF-means-off claim verified by reading hw/mod.rs:484-496.

## 3. Findings
F-001 [S3]
Where: capability.rs:24-29 (claim), decode.rs:312 (only caller chain)
Code:
```
//! 3. **What actually happened?** — [`negotiate`] returns a
//!    [`Negotiation`] carrying the accelerator (when hardware won)
//!    plus the [`HwDecodeEvent`] trail for the truthful active-
```
What is wrong: the module claims to be "the only place the `hw-active` truth is decided" and to feed "the `hw-active` surface the engine surfaces through its reporting seam" — but that engine surface is unwired (self-marked `[VERIFY: engine-side wiring is the registry card's scope]`, L29 and L268), and no shipped entry point reaches `negotiate` at all: `H264Decoder` (the only `negotiate_lane` caller) is instantiated nowhere outside tests. 584 lines of correct, tested policy code are dead weight in the shipped binaries. Why it matters: any doc/matrix row citing "truthful hw-active reporting" describes a surface that does not exist end-to-end; the code is Phase-7 pending (as pipeline_host.rs:338-342 admits). What correct looks like: either wire the lane or state the unwired status in the module header (the [VERIFY] markers do this — the residual defect is the header's present-tense "the engine surfaces" phrasing). Confidence: HIGH — Verification: read + rg (callers enumerated) + ran tests.
F-002 [S4]
Where: capability.rs:394-400
Code:
```
        // Unreachable arm shape (policy that does not probe handled
        // above); kept total rather than panicking.
        _ => Negotiation {
            accelerator: None,
```
What is wrong: the catch-all is genuinely unreachable (SoftwareOnly is arm 1; both remaining policies satisfy `probes()`), and if it were ever reached it would return a malformed trail — `[Probed]` with no Active/Inactive decision event. Defensive totality is fine; the malformed-trail shape is the latent hazard for a future fourth policy. Confidence: HIGH — Verification: read (arm-coverage analysis).
F-003 [S4]
Where: capability.rs:179-181 vs 383-393
Code:
```
    /// `hardware-only` — probe first; absence is a typed refusal.
    HardwareOnly,
```
What is wrong: `negotiate` treats `HardwareOnly`-with-unavailable exactly like `PreferHardware` (Inactive + fallback = on_absence); the typed refusal is delegated to the caller (decode.rs:458,607 answer `h264.hw.required` — verified). A caller relying on `negotiate` alone would silently fall back under a hardware-only policy. The layering works today because the only caller enforces it; the module docs don't say the refusal lives elsewhere. Confidence: HIGH — Verification: read + rg `h264.hw.required`.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| HwBackend::backend_id/Display | 64-81 | REAL | — | stable labels, test-pinned |
| HwUnavailableReason + Display | 87-133 | REAL | — | 6 typed variants, user-renderable, test-pinned |
| HwAvailability::backend | 156-165 | REAL | — | accessor |
| HwDecodePolicy::from_setting/as_setting/probes | 184-220 | REAL | — | exact vocabulary, typed rejection, default SoftwareOnly (OFF), round-trip test incl. 5 bad spellings |
| HwPolicyError | 222-239 | REAL | — | carries the rejected spelling |
| FallbackTarget | 242-252 | REAL | — | Software vs UnsupportedPending — honest HEVC posture |
| HwDecodeEvent | 255-283 | REAL | — | Probed/Active/Inactive trail; engine consumer unwired (F-001) |
| Negotiation + Debug | 287-311 | REAL | — | opaque accelerator in Debug (label not pointer) — deliberate |
| negotiate | 322-402 | REAL | F-002/F-003 | pure; arm analysis: SoftwareOnly→Inactive(DisabledByPolicy when available); Available+accel→Active; Available+no-accel→typed DriverIncompatible; Unavailable→typed Inactive |
| unavailable_reason | 406-411 | REAL | — | Available→DisabledByPolicy stand-in (only reachable from arm 1's else) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| policy_parses_the_settings_vocabulary_exactly (443) | YES | 3 acceptances + 5 rejections + default/probes predicates |
| software_only_never_activates_even_with_hardware_present (467) | YES | passes a live mock accelerator; asserts none returned + DisabledByPolicy trail shape (positional event match) |
| prefer_hardware_activates_when_available (489) | YES | asserts accelerator + Active event with backend/detail |
| unavailable_falls_back_with_the_typed_reason (511) | YES | both probing policies × NoDeviceNode, trail shape pinned |
| available_without_an_accelerator_is_an_honest_inactive (535) | YES | the no-fake-Active rule |
| backend_labels_are_stable (553) / reasons_render_for_users (564) | YES | string contracts |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the only place the hw-active truth is decided" | capability.rs:3-4 | REAL code / UNREACHABLE in ship | F-001 caller census |
| "no probe outcome is ever guessed" | capability.rs:11-15 | REAL | every Unavailable carries a typed reason; WrongOs synthesized only by cfg-shape in hw/mod.rs nominal_lane_backend (label is nominal, reason carries truth — documented L506-515) |
| "the events are the hw-active surface the engine surfaces through its reporting seam" | capability.rs:27-29 | PAPER-ONLY at engine level | [VERIFY] marker admits unwired; hw_events() read only by tests |
| "shipped default software-only + features.hw-decode = false" | capability.rs:20-21 | REAL | default() == SoftwareOnly test-pinned; settings key exists (settings-schema runtime.rs:216) |
| MediaFoundation "not yet implemented" | capability.rs:55-58 | REAL (honest) | no MF lane code exists in tree |

## 7. Dead / duplicated / unreachable code
The whole module is unreachable from shipped entry points (F-001) while remaining `pub` API of the codecs crate. HEVC does NOT share it (hevc/hw.rs has its own vocabulary — verified no import). No duplication found within the file.

## 8. Dependencies & flags
`pub mod hw` unconditional (h264/mod.rs:53); no feature gate, no unsafe, depends only on codec-api HwAccelerator + std. The sw decoder (codec-h264-sw) is what ships; this hw layer compiles everywhere but is called by nothing.

## 9. Open questions for the next agent
- h264/hw/mod.rs + videotoolbox/mod.rs owners: same unreachability applies to the lane probes (my next files will confirm the VT lane's status).
- Settings owner: `playback.hw-decode-policy` key is parsed only by this unreachable lane — a declared setting whose only consumer never runs in shipped builds (cross-file, for the settings audit).
