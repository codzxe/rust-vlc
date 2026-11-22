# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_12_device_loss.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 197 / 2 tests + 1 helper |
| File verdict | REAL-UNPROVEN (pins the REACHABLE device-loss truth: a sink-contract fatal — not the unreachable suspend/resume policy) |
| Reachable from shipped entry point? | N/A — scenario test (doc 10 §6.4 row 12) over the command layer. The behavior it pins (consumer refusal → typed fatal) IS the shipped engine's pipeline behavior. |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | `head -40` + `sed -n '93,160p'` (exit 0). No cargo; run UNVERIFIED (W00: 1751/2/25). |

## 1. One-paragraph verdict

Two tests: (1) `audio.set-device`/`audio.list-devices` answer the production typed refusal naming the owning subsystem (ADR-005: real backends later) — the wire truth pinned; (2) the vanishing-sink fake (every frame accept answers the typed `Device{AudioOutput, Disconnected}`) drives the engine to exactly one fatal `playback.error { audio-output.device }` ordered before `state-changed → Error`, with mid-play position preserved, no frame delivered past the vanish arm, and the host cleanly closeable and reusable. The asserts are exact (one fatal, exact code, exact trail tail, frozen accepted-count). The important cross-file fact: this scenario pins the **sink-contract fatal** as the shipped device-loss behavior, while the module doc claims the suspend-and-reopen policy's `src/recovery.rs` is "not in the tree" — it IS in the tree (my recovery_env.rs and hotplug.rs audits prove it: `AudioDeviceGuard`, suspend/resume rows, all tested there). The tree therefore contains **two contradictory device-loss policies**: the reachable one (fatal, pinned here) and the unreachable one (suspend/reopen via the hotplug hooks no host calls — my hotplug.rs F-001). The doc comment is stale and the design intent unresolved on the wire.

## 2. What actually works (verified)

- `device_control_commands_refuse_typed` (61-92): production typed refusals asserted (the shipped build has no device control — pinned, not hidden).
- `vanishing_device_fails_typed_with_position_preserved` (93-197): mid-play loss (position strictly inside duration asserted), exactly one fatal with exact code, trail tail `(Playing, Error)`, `frames_while_vanished() >= 1` (the refusal was really observed), accepted-count frozen after the arm, clean close + fresh reopen. Read-verified line by line.

## 3. Findings

### F-001 [S3] Stale doc: "card 3.C3's `src/recovery.rs` (not in the tree)" — it is in the tree; two device-loss policies coexist
Where: scenario_12_device_loss.rs:20-26
Code (20-26, verbatim):
```rust
//! `playback.error` (stage `audio-output`, code `device`) ordered
//! before `state-changed → error`. There is no suspend-and-reopen
//! policy and no `audio.device-lost` emission yet — that is card
//! 3.C3's `src/recovery.rs` (not in the tree). This scenario pins
//! the CURRENT truth with full assertions and hands the
//! suspend/resume semantics to 3.C3 — the assertions here will
```
What is wrong: `crates/playback-engine/src/recovery.rs` exists (recovery_env.rs imports `AudioDeviceGuard, RetryWindow, SourceRetryGuard, SurfaceGuard` from `playback_engine::recovery` — recovery_env.rs:100-103), and hotplug.rs implements the suspend (`Pause` row) / resume (`Play` row) flow with `audio.device-lost`/`-changed` emissions. The comment's "not in the tree" is false today. The real, current truth is worse than the comment's: the suspend/resume policy EXISTS but is unreachable from every shipped entry point (hotplug.rs audit F-001 — no host calls the hooks), so the only reachable device-loss behavior is the fatal this scenario pins. Doc 03 §4.9(d)'s suspend policy is dead code in the shipped build.
Why it matters: a reader (or the matrix rows) could cite either file as "the" device-loss behavior; they are contradictory (fatal vs suspend-and-resume-at-position). The scenario's own final comment ("assertions here will need extension, not weakening, when it lands") anticipated the landing that already happened.
What correct looks like: the comment updated, and either the hooks wired into a host or the suspend policy deleted; the scenario extended to pin whichever survives.
Confidence: HIGH — Verification: read (this file, recovery_env.rs imports, hotplug.rs audit evidence).

### F-002 [S4] Reopen-after-fatal asserted only as "plays again from zero"
The closing reuse check (`SessionClose` then fresh open) asserts the host is reusable but not the 3.C3 resume-at-position semantics — correctly deferred by the comment, but the assert is weak (is_ok on close). Hygiene.

## 4. Per-function table

| Function | Lines | Verdict | Notes |
|---|---|---|---|
| `open_parent` | 40-59 | REAL | |
| `device_control_commands_refuse_typed` | 61-92 | REAL | production refusals pinned |
| `vanishing_device_fails_typed_with_position_preserved` | 93-197 | REAL | exact one-fatal/code/trail/freeze asserts |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| device_control_commands_refuse_typed | YES (reasoned) | exact refusal shapes |
| vanishing_device_fails_typed… | YES (reasoned) | mid-play precondition, exactly-one fatal with exact code, trail tail equality, frozen accepted-count, observed-refusal counter |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no suspend-and-reopen policy and no `audio.device-lost` emission yet — that is card 3.C3's src/recovery.rs (not in the tree)" | header 20-26 | FALSE (stale) | recovery.rs exists; hotplug.rs implements suspend/resume + device-lost; both unreachable in shipped hosts (F-001) |
| doc 10 §6.4 row 12: DeviceLost event; position preserved or explicit suspension; recovery on return | header 6-8 | SHALLOW-for-v0 | the shipped build answers a typed FATAL (no DeviceLost event on the wire — that emission lives in the unreachable hooks); position preserved and recovery-on-return (reopen) asserted |

## 7-9. Dead code / Dependencies / Open questions

None in-file. Open: waves 18/19 must adjudicate doc 10 row 12 against this pinned fatal (the row's "DeviceLost event" is unproducible in the shipped build — cf. hotplug.rs audit F-001).
