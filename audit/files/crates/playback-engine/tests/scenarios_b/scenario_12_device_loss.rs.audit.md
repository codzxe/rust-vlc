# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_12_device_loss.rs
| Field | Value |
|---|---|
| Auditor | W03-A09 |
| Date | 2026-11-17 |
| Lines / functions | 197 / 3 (1 helper + 2 #[test]) |
| File verdict | SHALLOW (relative to its AC row: it deliberately proves the OPPOSITE of the pass criteria, honestly documented, with a now-stale justification) |
| Reachable from shipped entry point? | Test file; the behavior pinned (sink-refusal → typed fatal) is real engine behavior (pipeline.rs consumer-refusal contract). The device-loss POLICY (recovery.rs/hotplug.rs) is in the tree but not driven here and not driven by any shipped host (see my recovery_env.rs audit). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | baseline cargo-test-workspace.txt:4403-4404 (both tests pass); sed docs/10-testing-ci-quality.md:692 (row 12); sed host.rs:570-600 (vanish fake). |

## 1. One-paragraph verdict
This scenario is honest about being a stand-in, and the stand-in has rotted. Doc 10 §6.4 row 12's pass criteria are: "`DeviceLost` event; position preserved or explicit suspension; recovery on return." The file's own header admits v0 delivers NONE of that (fatal error instead of DeviceLost, reopen from zero instead of position preserved, Error instead of suspension) and defers the real semantics to "card 3.C3's `src/recovery.rs` (not in the tree)". That justification is now FALSE: recovery.rs IS in the tree (I audited its test suite in File 1) and hotplug.rs (card 7.C1) implements the `audio.device-lost` emission plus suspend-and-notify through `Engine::poll_hotplug`. The file's own promise — "the assertions here will need extension, not weakening, when it lands" — was never honored: the landed device-loss policy is not exercised here at all. As it stands, the scenario proves only that a vanishing sink produces one typed fatal, a clean close, and a from-zero reopen — the exact opposite of the row's "position preserved" and "recovery on return" criteria — and the AC row's real semantics have no scenario-level evidence anywhere (the policy code itself is production-unwired per my recovery_env audit, so the row is doubly unproven).

## 2. What actually works (verified)
— verified by baseline `audit/runs/baseline/cargo-test-workspace.txt:4403-4404` -> both tests pass ("scenario_12_device_loss::device_control_commands_refuse_typed … ok", "…vanishing_device_fails_typed_with_position_preserved … ok").
— verified by sed host.rs:570-583: `ObservingSink::vanish` arms a typed `Device{AudioOutput, Disconnected}` error — a genuine local fake matching doc 01 §3.4.
— The two tests are real behavior pins for what they cover: typed refusals for `audio.list-devices`/`audio.set-device` (owner-named, ADR-005), and the sink-refusal fatal path (exactly one fatal `audio-output.device`, ordered before `→ Error`, no frames delivered past the vanish, reusable host).

## 3. Findings
F-001 [S2]
Where: crates/playback-engine/tests/scenarios_b/scenario_12_device_loss.rs:21-26
Code:
```
//! error) ordered
//! before `state-changed → error`. There is no suspend-and-reopen
//! policy and no `audio.device-lost` emission yet — that is card
//! 3.C3's `src/recovery.rs` (not in the tree). This scenario pins
//! the CURRENT truth with full assertions and hands the
```
What is wrong: (a) The claim "src/recovery.rs (not in the tree)" is false — crates/playback-engine/src/recovery.rs exists (AudioDeviceGuard with suspend/reopen/notify) and src/hotplug.rs (card 7.C1) implements the `audio.device-lost` emission and suspend-and-notify through `Engine::poll_hotplug` (hotplug.rs:189-191 calls `observe_device_lost`, event.rs:240 documents the emission). (b) The promised extension ("will need extension, not weakening, when it lands") never happened: this scenario still pins only the pre-3.C3 fatal path, so doc 10 §6.4 row 12's pass criteria — "`DeviceLost` event; position preserved or explicit suspension; recovery on return" (docs/10-testing-ci-quality.md:692) — have NO scenario evidence, and the landed policy has no scenario coverage either. Why it matters: the AC row cites "transcript" as evidence; the transcript of this scenario shows criteria VIOLATED (fatal, position zero on reopen). What correct looks like: drive the loss through `Engine::poll_hotplug` (as tests/hotplug.rs does) inside the scenario, assert device-lost + suspension + resume-at-position, and delete the stale "not in the tree" comment. Confidence: HIGH — Verification: read + sed + rg (recovery.rs/hotplug.rs present).
F-002 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_12_device_loss.rs:94,108-111
Code:
```
fn vanishing_device_fails_typed_with_position_preserved() {
```
and
```
    assert!(
        position_at_loss > 0 && position_at_loss < duration,
        "the loss lands mid-play (position {position_at_loss} µs of {duration})"
    );
```
What is wrong: The test NAME promises "position preserved" but no preservation is asserted — `position_at_loss` is only checked to be mid-media; the reopen is explicitly asserted to start from ZERO (line 184-188). The name describes the criterion the AC wants, not what the test proves. Confidence: HIGH — Verification: read.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| open_parent | 40-56 | REAL | — | Fixture open + settle to Playing. |
| device_control_commands_refuse_typed | 61-85 | REAL | — | Typed UnsupportedForHost with owner string — honest refusal surface. |
| vanishing_device_fails_typed_with_position_preserved | 93-197 | REAL (as a fatal-path pin) / SHALLOW (as row 12) | S2/S4 | One typed fatal, ordering, no-delivery-past-vanish, clean close, from-zero reopen; F-001 stale deferral, F-002 name. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| device_control_commands_refuse_typed | YES | Exact stage/code/owner string. |
| vanishing_device_fails_typed… | YES (for the fatal path) | Exactly-one fatal, code string, state-trail end, frame counts pinned, reopen-from-zero pinned. It would NOT fail if the 3.C3/7.C1 device-loss policy regressed — that code is never driven here. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| Row 12 pass criteria: "`DeviceLost` event; position preserved or explicit suspension; recovery on return" | docs/10-testing-ci-quality.md:692 | NOT MET by this scenario (self-admitted) | Test asserts fatal + reopen from zero; F-001. |
| "that is card 3.C3's `src/recovery.rs` (not in the tree)" | scenario_12:22-23 | FALSE (stale) | recovery.rs + hotplug.rs exist; rg confirmed. |
| "the assertions here will need extension, not weakening, when it lands" | scenario_12:24-26 | UNFULFILLED | 3.C3/7.C1 landed; this file unchanged in substance. |

## 7. Dead / duplicated / unreachable code
None in-file. The unexercised surface is F-001's subject.

## 8. Dependencies & flags
commands, testkit (Must, alloc_count::serialize_tests — serializes tests that share the observing-sink/alloc-counting harness), scenarios_b host. No feature gates.

## 9. Open questions for the next agent
- Whoever audits tests/hotplug.rs: it drives `Engine::poll_hotplug` — but does any SCENARIO (doc 10 §6.4) cover the suspend-and-notify policy end-to-end through the command layer? If not, row 12's evidence remains absent even with the policy in-tree.
- The sink-refusal fatal path vs the hotplug suspend path are two different device-loss responses in one engine; which one does doc 03 §4.9(d) actually mandate for a vanished sink? (The scenario pins the fatal; recovery_env pins the suspend for a host-driven loss.)
