# AUDIT: crates/playback-engine/src/recovery.rs
| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2025-06-14 |
| Lines / functions | 1135 / 41 |
| File verdict | REAL-UNPROVEN-as-shipped — real, correct policy code, but UNREACHABLE from any shipped entry point (only tests call it) |
| Reachable from shipped entry point? | NO. Chain: recovery.rs ← crates/playback-engine/src/hotplug.rs:85 (`use crate::recovery::{AudioDeviceGuard, ...}`) ← `Engine::observe_audio_device_lost`/`poll_hotplug`/`observe_audio_reopen`/`observe_renderer_backend` (hotplug.rs:182/225/264/311, pub methods on the shipped `Engine`) ← callers: ONLY crates/playback-engine/tests/hotplug.rs. rg over apps/ for `DeviceLost|poll_hotplug|observe_audio|SourceRetryGuard|RetryWindow|recovery::` → 0 matches. renderer-wgpu/src/negotiate.rs:32/244/305 mentions `observe_renderer_backend` in doc comments only — no call. `SourceRetryGuard` and `SurfaceGuard` have no consumer outside tests/recovery_env.rs at all. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 3 |
| Commands run | `cargo test -p playback-engine --test hotplug` -> 8 passed, exit 0; `cargo test -p playback-engine --lib recovery::` -> 0 tests (module has no unit tests); rg reachability sweeps (0 production callers); baseline cargo-test-workspace.txt: recovery_env 21 passed, hotplug 8 passed |

## 1. One-paragraph verdict
The policy cores are genuinely implemented and correct: the bounded retry window (attempts + time, exponential capped backoff, saturating arithmetic), the suspend/reopen device-loss state machine with exactly-once non-fatal notice and remembered-position resume, and the surface-loss present gate — all behave as documented, and the integration suites drive them through the REAL engine (hotplug 8/8 pass under my run; recovery_env 21/21 per baseline). But the module answers to the hostile question with a flat NO: it is NOT reachable from the shipped binary. It is compiled in (plain `pub mod`, no feature gate), yet no shipped host — lumen-cli, desktop, web — ever calls a single recovery type or hotplug hook; the only callers in the entire tree are the two test files. Doc 03's §4.1(d) interrupted-network recovery and §4.11(d) surface-loss recovery therefore exist as library code + tests, not as product behavior.

## 2. What actually works (verified)
- verified by `cargo test -p playback-engine --test hotplug` -> `8 passed; 0 failed ... finished in 0.00s` (exit 0) — exercises `AudioDeviceGuard` through the real `Engine` hooks (loss → suspend → reopen → resume, exhaustion notice, repeated loss).
- verified by baseline audit/runs/baseline/cargo-test-workspace.txt (recovery_env section): `21 passed; 0 failed` — includes `engine_device_loss_suspends_and_reopen_resumes_through_the_command_layer`, `device_reopen_failure_backs_off_then_notifies_once_non_fatal`.
- Backoff arithmetic hand-checked: `backoff_ns(1)=250ms, backoff_ns(2)=500ms`, shift capped at 63, `saturating_mul` — no overflow path.

## 3. Findings
F-001 [S2]
Where: crates/playback-engine/src/recovery.rs:6-31 (module docs) + hotplug.rs:182-317
Code:
```
//! The three **environment** failure policies — the failures that
//! come from the *world around the media*, not from the media's own
//! bytes (media-level recovery is card 3.C2, `tests/recovery_media.rs`):
```
What is wrong: all three policies are dead code in every shipped build. rg proves zero production callers: `SourceRetryGuard`/`SurfaceGuard` are consumed only by tests/recovery_env.rs; `AudioDeviceGuard` only by src/hotplug.rs, whose four `Engine` entry points (`observe_audio_device_lost`, `poll_hotplug`, `observe_audio_reopen`, `observe_renderer_backend`) are called only by tests/hotplug.rs. The CLI's audio path is a null-device ring with no device-loss seam (rg over apps/ for `DeviceLost|poll_hotplug|observe_audio` → 0 matches); the renderer-wgpu negotiate module references `observe_renderer_backend` in comments (negotiate.rs:32,244,305) but never invokes it.
Why it matters: the module presents itself as the product's environment-recovery layer ("the host ... drives them" — recovery.rs:39); there is no host. Any feature-matrix claim of shipped network-retry/device-loss/surface-loss recovery is paper-only at the product level even though the code is real.
What correct looks like: either a shipped host drives the hooks (CLI null-device loss, tauri audio backend, wgpu renderer fallback relay), or the docs state the policies are library-only until a host lands.
Confidence: HIGH — Verification: ran rg sweeps + read hotplug.rs and negotiate.rs.

F-002 [S3]
Where: crates/playback-engine/src/recovery.rs:101-108 vs 763-777
Code:
```
//! Recovery is an *event-rate* path, not a per-frame hot path:
//! losses happen at human timescales, and every guard method runs
//! at most once per observed fact or per poll. The `Vec<EventKind>`
//! in a step stays empty (`Vec::new()` allocates nothing) on the
//! healthy path
```
vs
```
        let (device_id, reopen_used, next_reopen_ns, window_end_ns, notified) = match &self.phase {
            DevicePhase::Suspended {
                device_id,
```
What is wrong: while a device is suspended, every `AudioDeviceGuard::poll` clones `device_id` (a `String` allocation) and `observe_reopen_result` clones it again on every failed attempt — before deciding the answer is quiet. The allocation note's "zero steady-state cost" holds only for the Active phase (poll returns quiet at line 778 before the clone). Bounded and human-timescale, so impact is small, but the code contradicts its own D-1 doctrine on the suspended path.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/playback-engine/src/recovery.rs:900-909
Code:
```
    /// The active device's id (`None` while suspended; the empty
    /// string before any loss was observed on a default-constructed
    /// guard).
    pub fn active_device(&self) -> Option<&str> {
```
What is wrong: `Some("")` for a guard that never saw a device is a lying sentinel — callers cannot distinguish "no device ever" from "device with empty id". Documented, but an API wart.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/playback-engine/src/recovery.rs:544-546
Code:
```
            // The retrying() check above pins the phase.
            _ => unreachable!("retrying implies Waiting"),
```
What is wrong: one of the tree's 4126 `unreachable!`s. This one is sound (the `retrying()` check at 517 pins the variant), but it is a panic path nonetheless — a future refactor of `retrying()` converts it into a crash.
Confidence: HIGH — Verification: read + logic-checked.

F-005 [S4]
Where: crates/playback-engine/src/recovery.rs:134-136, 154-155
Code:
```
/// The figures are `[PROPOSAL: policy defaults — doc 03 §4.1(d)
/// says only "retrying within policy"; the numbers await the
/// product-config card that wires `product.toml` limits]`.
```
What is wrong: the retry figures (6 attempts / 10 s / 250 ms→4 s) are `[PROPOSAL]` constants not wired to any config surface — and given F-001 nothing shipped reads them anyway.
Confidence: HIGH — Verification: read; rg confirms no product.toml wiring.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| RetryWindow (struct) | 137-151 | REAL | — | two independent bounds documented |
| RetryWindow::backoff_ns | 166-171 | REAL | — | shift≤63, saturating_mul, min cap — correct |
| RetryWindow::DEFAULT / Default | 156-178 | REAL | S4 | [PROPOSAL] figures (F-005) |
| RecoveryAction (enum) | 187-230 | REAL | — | payloads move; non_exhaustive |
| RecoveryStep / RecoveryStep::quiet | 237-254 | REAL | — | Vec::new() no-alloc healthy path |
| ReadOutcome (enum) | 262-275 | REAL | — | Data{units}/Interrupted |
| is_retryable_network | 292-300 | REAL | — | Timeout/ConnectionLost/Dns yes; Tls/ReconnectExhausted no — matches doc |
| retry_exhausted_error | 309-331 | REAL | — | typed Network{ReconnectExhausted{attempts}}, shallow-chained source |
| SourcePhase (enum) | 334-353 | REAL | — | Streaming/Waiting/Exhausted |
| SourceRetryGuard::new | 381-388 | REAL | — | |
| SourceRetryGuard::read | 402-407 | REAL | — | dispatch to observe_data/observe_failure |
| SourceRetryGuard::poll | 418-440 | REAL | — | time-expiry vs due-retry; permission semantics documented; idempotent terminal |
| SourceRetryGuard::retrying | 443-446 | REAL | — | |
| SourceRetryGuard::retries_used | 452-463 | REAL | — | Some(0) for non-network Exhausted — documented |
| SourceRetryGuard::next_retry_ns / window_end_ns / windows_opened | 466-490 | REAL | — | per-episode budget (fresh window per failure) — correct |
| SourceRetryGuard::observe_data | 493-502 | REAL | — | closes with 1.0 bookend only if open |
| SourceRetryGuard::observe_failure | 505-560 | REAL | — | non-retryable passthrough; fresh window; degenerate-window exhaustion; attempt/window bounds both enforced |
| SourceRetryGuard::fail_early | 564-576 | REAL | — | closes open bookend; keeps original error |
| SourceRetryGuard::exhaust_after_failure | 583-598 | REAL | — | typed exhaustion, settle bookend |
| SourceRetryGuard::exhaust_by_time | 602-615 | REAL | — | attempts-only variant |
| SourceRetryGuard::terminal_answer | 620-628 | REAL | — | idempotent shallow copy |
| shallow_error | 635-643 | REAL | — | Fatal-clone doctrine |
| DevicePhase (enum) | 650-678 | REAL | — | Active/Suspended{notified latch} |
| AudioDeviceGuard::new | 712-723 | REAL | — | Active{device_id: ""} (F-003) |
| AudioDeviceGuard::observe_device_lost | 735-757 | REAL | — | fresh window; first reopen due immediately (documented deviation from backoff(1)); emits audio.device-lost + SuspendClock |
| AudioDeviceGuard::poll | 762-799 | REAL | S3 | F-002 (per-poll String clone while suspended); one-time notice on time expiry; grants while attempts+time remain |
| AudioDeviceGuard::observe_reopen_result | 814-882 | REAL | — | success → device-changed + ResumeClock from remembered position; failure → backoff advance or exactly-once notice; count stored BEFORE latch (the card 7.C1 fix, self-documented) |
| AudioDeviceGuard::suspended / suspended_position_ns | 886-898 | REAL | — | |
| AudioDeviceGuard::active_device | 903-909 | REAL | S4 | F-003 |
| AudioDeviceGuard::losses / reopens / suspended_device | 911-931 | REAL | — | |
| AudioDeviceGuard::reopen_attempts_used / notified | 936-951 | REAL | — | |
| AudioDeviceGuard::last_reopen_failure | 960-962 | REAL | — | Display form; reset per loss |
| AudioDeviceGuard::mark_notified | 966-998 | REAL | — | non-fatal PlaybackError{AudioOutput, Device, fatal:false}; details map; both message forms |
| SurfaceGuard::new | 1030-1033 | REAL | — | Default |
| SurfaceGuard::observe_surface_lost | 1044-1066 | REAL | — | idempotent; non-fatal renderer event; RequestSurfaceRecreate |
| SurfaceGuard::observe_surface_recreated | 1074-1080 | REAL | — | emits nothing (recorded v1 gap, no video.surface-* kind) |
| SurfaceGuard::observe_resize | 1085-1088 | REAL | — | non-fatal by construction |
| SurfaceGuard::compositing / gate_present | 1092-1110 | REAL | — | suppression counter |
| SurfaceGuard::losses / recreations / suppressed_presents / resizes | 1112-1134 | REAL | — | counters |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| tests/recovery_env.rs (21, baseline pass) | YES | drives guards + real Engine: backoff schedule assertions, exactly-once notice, remembered-position resume, fresh-window tolerance, degenerate windows |
| tests/hotplug.rs (8, my run pass) | YES | loss→suspend→reopen→resume through the real Engine state table; exhaustion notice; wire payload vs catalog contract; direct-drive vs engine-hook agreement |
| No unit tests in-file | — | all coverage is integration-level (acceptable for a policy core consumed via Engine) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The guards never touch the engine directly; the host ... drives them" | recovery.rs:38-40 | REAL (code) / MISLEADING (product) | the "host" does not exist: no shipped binary calls any guard or hook (F-001) |
| "no sleeps anywhere ... deterministic under virtual time" | recovery.rs:62-65 | REAL | no Instant/sleep/timer in file; all windows take now_ns from caller |
| "playback is *never silently restarted*" (suspend-and-notify default) | recovery.rs:22-26 | REAL | ResumeClock carries remembered position; notice is non-fatal (mark_notified, fatal:false) — verified by hotplug test run |
| "v1 catalog has no surface event" gap | recovery.rs:88-95 | REAL | EventKind has VideoBackendChanged but no surface kind (host.rs event_name table, commands catalog) |

## 7. Dead / duplicated / unreachable code
- The entire module is runtime-dead in shipped builds (F-001) — library-public, test-consumed.
- `SourceRetryGuard` and `SurfaceGuard`: zero non-test consumers anywhere (not even hotplug.rs).
- `RecoveryAction::RetryRead`/`FailSource`/`RequestSurfaceRecreate` arms are consumed only by the guards' own tests; hotplug.rs:244-248 explicitly maps them to `HotplugStep::None` ("the honest unreachable: consumed, not panicked on").

## 8. Dependencies & flags
- Depends only on commands (EventKind/Stage/ErrorCode) + media-types (EngineError taxonomy) — both production deps of playback-engine. No feature gates: `pub mod recovery;` (lib.rs:102) unconditionally compiled into the shipped lib.

## 9. Open questions for the next agent
- Does any feature-matrix row claim shipped device-loss recovery / network retry / surface-loss handling citing recovery.rs or the hotplug hooks? If yes → FALSE/UNREACHABLE row (code exists, no shipped caller).
- The CLI's null audio backend: does it have ANY device-lost emission path of its own (audio-output crate DeviceEventQueue)? rg found none in apps/ — platform-* queues exist but nothing bridges them to the engine. Worth a platform-agent check.
