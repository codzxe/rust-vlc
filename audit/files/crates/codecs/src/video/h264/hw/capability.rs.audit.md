# AUDIT: crates/codecs/src/video/h264/hw/capability.rs
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2026-11-21 |
| Lines / functions | 584 / 17 (incl. 6 tests + 3 test helpers) |
| File verdict | UNREACHABLE (real, correct pure-logic module; zero production callers) |
| Reachable from shipped entry point? | NO — sole caller chain: `capability::negotiate` (line 322) ← `hw/mod.rs:501 negotiate_lane` ← `decode.rs:312 H264Decoder::from_setting` ← nothing outside `crates/codecs` (rg `H264Decoder` repo-wide: only codecs-internal + a comment at apps/cli/src/pipeline_host.rs:342). `product.toml:168 hw-decode = false`, `:191 hw_decode_policy = "software-only"`; shipped dispatch is `H264SwDecoder` (pipeline_host.rs:346). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg `negotiate\(` (callers: hw/mod.rs:501 only); rg `HwDecoder|H264Decoder` (no host construction); rg `hw-decode-policy` (product.toml + settings-schema only); baseline test log lines 550-556 (all 6 tests ran, ok). Byte-level python check of hw/mod.rs:449 attribute (resolved a display-mangling false alarm — see log). |

## 1. One-paragraph verdict
The policy/availability/negotiation logic itself is correct — every arm of `negotiate` hand-verified against the policy×availability×accelerator matrix, case-sensitive setting parsing with typed rejection, honest `Inactive` on every non-active path, and six real tests that assert exact event-trail shapes (all ran in baseline, log lines 550-556). But the module is a policy engine wired to no engine: its own docs admit the reporting consumer does not exist (`[VERIFY: engine-side wiring is the registry card's scope]`, line 29), and its only caller chain dead-ends at `H264Decoder::from_setting`, which no host constructs. "The only place the `hw-active` truth is decided" decides a truth nobody reads — the actual shipped `engine-info` output is a hard-coded literal (baseline INDEX.md standing defect).

## 2. What actually works (verified)
— Hand-traced `negotiate` (lines 322-402) across all reachable input combinations: SoftwareOnly+Available+accel → Inactive{DisabledByPolicy}, accelerator dropped (matches test `software_only_never_activates_even_with_hardware_present`); PreferHardware+Available+Some → Active; PreferHardware+Available+None → Inactive{DriverIncompatible("probe succeeded for … but no accelerator was minted")}; HardwareOnly+Unavailable → Inactive with the lane's reason. No path fabricates an Active.
— `HwDecodePolicy::from_setting` rejects `"", "auto", "prefer-hw", "Software-Only", "hardware"` — verified by test `policy_parses_the_settings_vocabulary_exactly` (baseline :552 ok).
— Module doc's policy facts cross-checked: `product.toml:168` `hw-decode = false`, `product.toml:191` `hw_decode_policy = "software-only"` — consistent.

## 3. Findings
### F-001 [S3]
Where: crates/codecs/src/video/h264/hw/capability.rs:24-29 (+ decode.rs:312)
Code:
```rust
//! 3. **What actually happened?** — [`negotiate`] returns a
//!    [`Negotiation`] carrying the accelerator (when hardware won)
//!    plus the [`HwDecodeEvent`] trail for the truthful active-
//!    decoder report. The events are the `hw-active` surface the
//!    engine surfaces through its reporting seam
//!    `[VERIFY: engine-side wiring is the registry card's scope]`.
```
What is wrong: the module's deliverable — the `hw-active` event trail — has no consumer anywhere in the repo, and the module itself flags the missing wiring. The full call chain (`negotiate` ← `negotiate_lane` ← `H264Decoder::from_setting`) is constructed by no shipped host; `system.get-engine-info`'s decoder list is a hard-coded literal in `apps/cli/src/host.rs` (baseline INDEX.md, standing defects).
Why it matters: 584 lines of correct-looking negotiation code whose "truthful reporting" purpose is unfalsifiable at runtime — the exact "typed seam wired to nothing" pattern the audit hunts. Nothing user-visible breaks because nothing user-visible reads it.
What correct looks like: either an engine-side consumer of `HwDecodeEvent` (the registry card this doc defers to) or the module held out of shipped builds until that card lands.
Confidence: HIGH — Verification: read + rg callers (header).

### F-002 [S4]
Where: crates/codecs/src/video/h264/hw/capability.rs:256-264
Code:
```rust
    /// A lane probe ran and produced this outcome (emitted even for
    /// the `software-only` default, where the "probe" is the
    /// policy refusal itself — the report then says why nothing
    /// was touched).
    Probed {
```
What is wrong: under `SoftwareOnly` the caller (`hw/mod.rs:492-498 negotiate_lane`) fabricates `HwAvailability::Unavailable{DisabledByPolicy}` without probing, and `negotiate` (line 329-331) unconditionally emits `HwDecodeEvent::Probed` for it. The event name asserts a probe ran; none did. This sits awkwardly against the module's own contracts: "no probe outcome is ever guessed" (lines 12-14) and `probes()`'s "no device or library touches — OFF means off" (lines 213-215).
Why it matters: a trail consumer cannot distinguish "probed and found nothing" from "policy refused to probe" without parsing reasons — naming that lies, in a module whose entire brand is truthful reporting. Mitigated: documented, and no consumer exists today (F-001).
What correct looks like: `HwDecodeEvent::PolicyRefused` variant (or `Probed(Option<…>)`), emitted instead of a synthetic `Probed` under the OFF default.
Confidence: HIGH — Verification: read of both sites (capability.rs:329-351, hw/mod.rs:483-501).

### F-003 [S4]
Where: crates/codecs/src/video/h264/hw/capability.rs:394-400
Code:
```rust
        // Unreachable arm shape (policy that does not probe handled
        // above); kept total rather than panicking.
        _ => Negotiation {
            accelerator: None,
            fallback: Some(on_absence),
            events,
        },
```
What is wrong: provably dead arm: the only policy with `!probes()` is `SoftwareOnly`, fully captured by arm 1 (line 337), so the catch-all cannot match any input. Comment acknowledges it.
Why it matters: none functionally — a totality guard, not a bug; recorded because the checklist requires dead-code rows.
What correct looks like: acceptable as-is; alternative is exhausting the policy enum in arm 1's pattern.
Confidence: HIGH — Verification: read (arm order + `probes()` definition line 217-219).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `HwBackend` + `backend_id` + `Display` | 51-81 | REAL | — | stable labels, test-asserted (:553-561) |
| `HwUnavailableReason` + `Display` | 87-133 | REAL | — | typed vocabulary; render tests (:564-583) |
| `HwAvailability` + `backend()` | 135-165 | REAL | — | pure accessor |
| `HwDecodePolicy::{from_setting, as_setting, probes}` | 167-220 | REAL | — | exact-vocabulary parse, case-sensitive; round-trip + negative tests (:442-464) |
| `HwPolicyError` + `Display` + `Error` | 222-239 | REAL | — | carries the rejected spelling |
| `FallbackTarget` | 241-252 | REAL | — | data; HEVC `UnsupportedPending` matches baseline (sample-hevc.mp4 → Error) |
| `HwDecodeEvent` | 254-283 | REAL | S4 | `Probed` misnomer under OFF policy (F-002) |
| `Negotiation` + `Debug` | 285-311 | REAL | — | opaque accelerator handle — correct (no pointer leak in reports) |
| `negotiate` | 313-402 | REAL | S3 | all arms hand-verified correct; unreachable in production (F-001); dead catch-all (F-003) |
| `unavailable_reason` | 404-411 | REAL | — | `Available → DisabledByPolicy` stand-in only reachable from arm 1 |
| tests: `mock_accel`/`va_available`/`VA_NODE_MISSING` | 417-440 | REAL (test-only) | — | mock is of the *input* trait object (legitimate seam), not of the subject |
| test `policy_parses_the_settings_vocabulary_exactly` | 442-464 | REAL | — | would fail on any vocabulary/round-trip regression |
| test `software_only_never_activates_even_with_hardware_present` | 466-486 | REAL | — | exact 2-event trail shape asserted |
| test `prefer_hardware_activates_when_available` | 488-508 | REAL | — | exact Active shape + fields |
| test `unavailable_falls_back_with_the_typed_reason` | 510-532 | REAL | — | both enabling policies × typed reason |
| test `available_without_an_accelerator_is_an_honest_inactive` | 534-550 | REAL | — | the never-fake-Active invariant |
| test `backend_labels_are_stable` / `reasons_render_for_users` | 552-583 | REAL | — | string-contract locks |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| policy_parses_the_settings_vocabulary_exactly | YES | exact Eq on Ok/Err + round-trip + default/probes predicates |
| software_only_never_activates_even_with_hardware_present | YES | mutation: make arm 1 activate → `accelerator.is_none()` assert fails |
| prefer_hardware_activates_when_available | YES | exact event-trail destructuring; shape mismatch → unreachable! panic |
| unavailable_falls_back_with_the_typed_reason | YES | reason equality per policy |
| available_without_an_accelerator_is_an_honest_inactive | YES | variant-shape match on events[1] |
| backend_labels_are_stable / reasons_render_for_users | YES | literal equality/contains on public strings |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the only place the `hw-active` truth is decided" | module doc :3-4 | SHALLOW | decision logic real (negotiate), but no consumer reads the decision; engine-info is a hard-coded literal (baseline standing defect; F-001) |
| "engine surfaces [events] through its reporting seam" | :27-29 | PAPER-ONLY | `[VERIFY: engine-side wiring …]` — wiring absent, admitted in-line |
| "shipped default `software-only` + `features.hw-decode = false`" | :20-21, 168-171 | REAL | product.toml:168/:191 verified |
| "HEVC … no software decoder exists there yet … typed Unsupported" | :39-42 | REAL | baseline: sample-hevc.mp4 → `Error` (INDEX.md corrected matrix) |
| "H.264 … falls back to software truthfully" | :33-37 | REAL (code) / UNREACHABLE (lane) | sw lane reachable via pipeline_host.rs:346; the *negotiated* fallback (this module) unreachable |

## 7. Dead / duplicated / unreachable code
Entire module unreachable from shipped entry points (F-001). Dead catch-all arm (F-003). No duplication: `apps/web/src/render/backend.rs:183 negotiate` is an unrelated renderer-caps function (name collision only, different crate).

## 8. Dependencies & flags
Depends on `codec_api::HwAccelerator` (codec-api/src/hw.rs). No OS cfg gates — compiled on every target including Windows (where it describes a MediaFoundation lane that does not exist — honestly labeled, :55-58). No unsafe. Not feature-gated: ships in every build including the wasm tree-shakes.

## 9. Open questions for the next agent
- W06-A07 owns `hw/mod.rs` — confirm `negotiate_lane`'s `nominal_lane_backend()` MediaFoundation label on Windows doesn't leak into any user-visible report once wiring lands.
- Tooling note logged for all agents: Bash-tool output strips `[m` sequences (ANSI-reset-like text) — `#[must_use` displayed as `#ust_use` in sed/grep output but the file bytes are intact (python byte-dump verified). Verify quotes with the Read tool or python, not sed, when `[m` appears.
