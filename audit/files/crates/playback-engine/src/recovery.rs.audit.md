# AUDIT: crates/playback-engine/src/recovery.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 1136 / 48 production items (3 guards, 2 windows, 6 enums, helpers); **0 in-file tests** — tests live in `tests/recovery_env.rs` (not my file) and `tests/hotplug.rs` (my file, audited separately) |
| File verdict | UNREACHABLE (library cores genuine: REAL-UNPROVEN in isolation) |
| Reachable from shipped entry point? | **NO** — proof in F-001 |
| Findings | S0:1 S1:0 S2:0 S3:2 S4:2 |
| Commands run | rg for guard/hook consumers across crates/ + apps/; sed quote verification; `grep -c "#\[test\]"` → 0 |

## 1. One-paragraph verdict

Three "environment recovery" policy cores — `SourceRetryGuard` (bounded network-read retry with exponential backoff inside a bounded virtual-time window), `AudioDeviceGuard` (suspend-and-notify on device loss, bounded reopen window, resume from remembered position), `SurfaceGuard` (renderer-surface loss → hold compositing, audio continues, resume on recreate). The state machines are genuinely implemented: saturating arithmetic, typed errors with chained sources, no sleeps (virtual time injected), bookend events that never strand, idempotent terminals. And all of it is speculative scaffolding: **no shipped entry point constructs any guard or calls any of the Engine hotplug hooks that consume `AudioDeviceGuard`** (F-001). The only audio backend in the tree is the null device (which never reports loss), no network source is wired to `SourceRetryGuard`, and no renderer thread drives `SurfaceGuard`. The phase-3 exit gate lists "recovery" as a MET capability family citing card 3.C3's merge — the same paper-completion pattern as the clock backends.

## 2. What actually works (verified)

- The pure state machines read correct under hostile review: dual-bound retry (attempts AND window), backoff `min(base·2^(n-1), cap)` with shift clamped ≤63 (recovery.rs:167-171), degenerate-window exhaustion (522-526), `unreachable!("retrying implies Waiting")` backed by a preceding `retrying()` check (513-517, 544-545), one-time notice latch, position preserved across episodes.
- W00 baseline shows the playback-engine test suite green as part of the workspace 1751-pass run (cargo-test.log) — `tests/recovery_env.rs` and `tests/hotplug.rs` ran green there, but I could not execute them myself (ADAPTATION); their *content quality* is assessed in my hotplug.rs audit file.
- `is_retryable_network` (293-300) correctly refuses to retry `Tls` (no downgrade) and `ReconnectExhausted` (no double budget) — a thoughtful taxonomy honor.

## 3. Findings

### F-001 [S0] — The whole module is unreachable: no host constructs the guards and no host calls the Engine hooks that wrap them, while the gate claims "recovery … MET (merged)"

Where: crates/playback-engine/src/recovery.rs:373-377 (SourceRetryGuard), 698-708 (AudioDeviceGuard), 1020-1026 (SurfaceGuard); wrapper hooks at crates/playback-engine/src/hotplug.rs:182, 225, 264, 311.

Code (recovery.rs:373-377, verbatim):
```rust
pub struct SourceRetryGuard {
    policy: RetryWindow,
    phase: SourcePhase,
    windows_opened: u64,
}
```

What is wrong: repo-wide grep (run by me) shows:
- `SourceRetryGuard` consumers: `lib.rs:132` (re-export) + `tests/recovery_env.rs` only.
- `SurfaceGuard` consumers: `lib.rs:133` + `tests/recovery_env.rs` only.
- `AudioDeviceGuard` consumers: `src/hotplug.rs` (Engine wiring) + `tests/recovery_env.rs`/`tests/hotplug.rs`.
- The Engine-facing hooks `observe_audio_device_lost` / `poll_hotplug` / `observe_audio_reopen` / `observe_renderer_backend` (hotplug.rs:182/225/264/311) have **zero callers** in `apps/`, `crates/input/`, `crates/audio-output/`, `crates/video-engine/` — only `tests/hotplug.rs`.

So no `lumen` CLI or `lumen-tauri` session can ever execute a recovery transition: the null audio device never loses, no network layer reports through `SourceRetryGuard`, no renderer drives `SurfaceGuard`.

Why it matters: docs/gates/phase3-exit.md:31 declares the capability family "recovery" **"MET (merged)"** citing "wave-5 merges … a181b81 (3.C3)". A capability whose every path requires a host call that no host makes is not MET; it is a library plus unwired hooks. This is the S0 §4 pattern (doc/matrix asserts something the shipped code contradicts) — the claim, not the code, is the defect.

What correct looks like: apps' audio pump calls `observe_audio_device_lost` on `SinkEvent::DeviceLost` and polls `poll_hotplug` on its cadence; the input layer runs `SourceRetryGuard` around live reads — or the gate says "policy cores landed; host integration pending".

Confidence: HIGH — Verification: read (exhaustive grep over crates/ and apps/).

### F-002 [S3] — `ReadOutcome::Data { units }` is accepted and never read

Where: crates/playback-engine/src/recovery.rs:269-272 and 493-502.

Code (269-272, verbatim):
```rust
    Data {
        /// How many units the read delivered.
        units: u64,
    },
```

Code (493-495, verbatim):
```rust
    fn observe_data(&mut self) -> RecoveryStep {
        if self.retrying() {
            self.phase = SourcePhase::Streaming;
```

What is wrong: the `units` payload is parsed into the enum and then dropped — `observe_data` takes no parameters; nothing in the file or its consumers reads `units`. §3.1 "parameters accepted but ignored". The doc comment even frames it as meaningful ("only the count of *facts* matters to the window" — then the count is never taken).

Why it matters: an API that solicits data it ignores invites callers to believe progress accounting exists; there is none.

What correct looks like: either record `units` into a lifetime counter (observable progress) or make the variant unit-struct.

Confidence: HIGH — Verification: read.

### F-003 [S3] — After the one-time suspension notice, `AudioDeviceGuard::poll` never grants a reopen again — a later re-plug is invisible until a *fresh* loss

Where: crates/playback-engine/src/recovery.rs:780-798.

Code (792-797, verbatim):
```rust
        if !notified && reopen_used < self.policy.max_attempts && now_ns >= next_reopen_ns {
            return RecoveryStep {
                action: RecoveryAction::ReopenDevice,
                events: Vec::new(),
            };
        }
```

What is wrong: once `notified` latches (window expired / budget spent), both poll branches are dead (`!notified` guards both) — `poll` answers `quiet` forever. A user re-plugging the device 30 s later gets no automatic recovery; the only escape is `observe_reopen_result(Ok(..))` spontaneously offered by a host (819-832 still accepts it), but the host was told the episode is terminal and nothing in the shipped tree ever calls it (F-001 anyway). The module docs call this "suspend-and-notify default" — deliberate, but the asymmetry (a late reopen result is accepted while no reopen is ever again *requested*) is a policy seam the docs do not name.

Why it matters: recovery UX hole: suspend-until-user-notices; combined with no `audio.device-changed` on manual resume-by-play, the wire story is incomplete.

What correct looks like: either a fresh reopen window on re-detection (device-detection is admittedly out of scope — then say "manual play is the only resume" in the module docs) or a `resume_now()` host entry.

Confidence: HIGH — Verification: read (branch analysis; no execution).

### F-004 [S4] — Policy figures are self-authored `[PROPOSAL]` numbers, never product-wired

Where: crates/playback-engine/src/recovery.rs:134-136 and 156-161.

Code (134-136, verbatim):
```rust
/// The figures are `[PROPOSAL: policy defaults — doc 03 §4.1(d)
/// says only "retrying within policy"; the numbers await the
/// product-config card that wires `product.toml` limits]`.
```

What is wrong: the shipped defaults (6 attempts / 10 s / 250 ms / 4 s cap) were invented by the card; no product.toml key reads them (grep: `RetryWindow::DEFAULT` is the only construction default, used by hotplug.rs:147 and tests). Honest self-labeling keeps this at S4, but the "policy" in "bounded retries within policy" is a placeholder.

Confidence: HIGH — Verification: read + grep for RetryWindow constructions.

### F-005 [S4] — `AudioDeviceGuard::observe_reopen_result` counts backoff one step ahead of `SourceRetryGuard`'s convention

Where: crates/playback-engine/src/recovery.rs:849-850 vs 550.

Code (849-850, verbatim):
```rust
                let reopen_used = reopen_used + 1;
                let next_reopen_ns = now_ns.saturating_add(self.policy.backoff_ns(reopen_used + 1));
```

What is wrong: source-guard failed retries schedule the next attempt at `backoff_ns(retries_used + 1)` where `retries_used` counts *completed failed retries* — after the 1st failed retry (retries_used=1), next waits `backoff(2)` = 2×base. The audio guard, after its 1st failed reopen (reopen_used=1), also waits `backoff(2)` = 2×base — but its *first* attempt was immediate (no base wait), so the audio schedule is: 0, 2×base, 4×base… while the source schedule is base, 2×base, 4×base…. Two guards in one file, two conventions, no comment reconciling them. Cosmetic-but-confusing; the immediate-first-attempt choice is documented (732-734) so this is hygiene.

Confidence: MEDIUM — Verification: read; arithmetic by hand.

## 4. Per-function table

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `RetryWindow` (struct) | 138-151 | REAL-UNPROVEN | S4 | F-004 proposal figures |
| `RetryWindow::DEFAULT` | 156-161 | REAL-UNPROVEN | — | 6/10s/250ms/4s |
| `RetryWindow::backoff_ns` | 167-171 | REAL-UNPROVEN | — | saturating, shift ≤63, capped |
| `Default for RetryWindow` | 174-178 | REAL-UNPROVEN | — | = DEFAULT |
| `RecoveryAction` (enum) | 189-230 | REAL-UNPROVEN | — | 7 variants, `#[non_exhaustive]` |
| `RecoveryStep` + `quiet` | 238-254 | REAL-UNPROVEN | — | action + events; quiet allocates nothing |
| `ReadOutcome` (enum) | 264-275 | REAL-UNPROVEN | S3 | F-002 `units` ignored |
| `is_retryable_network` | 293-300 | REAL-UNPROVEN | — | Tls/Exhausted excluded — correct |
| `retry_exhausted_error` | 310-331 | REAL-UNPROVEN | — | typed + shallow-chained source |
| `SourcePhase` (enum) | 335-353 | REAL-UNPROVEN | — | Streaming/Waiting/Exhausted |
| `SourceRetryGuard::new` | 382-388 | UNREACHABLE | S0 | F-001 (test-only consumers) |
| `read` | 402-407 | UNREACHABLE | — | dispatch to observe_* |
| `poll` | 418-440 | UNREACHABLE | — | time-expiry beats due-retry; idempotent terminal |
| `retrying`/`retries_used`/`next_retry_ns`/`window_end_ns`/`windows_opened` | 444-490 | UNREACHABLE | — | observability accessors |
| `observe_data` | 493-502 | UNREACHABLE | — | closes window, 1.0 bookend |
| `observe_failure` | 505-560 | UNREACHABLE | — | open/advance/terminate; degenerate window handled (522-526); `unreachable!` guarded by `retrying()` |
| `fail_early` | 564-576 | UNREACHABLE | — | non-retryable passthrough; closes bookend |
| `exhaust_after_failure` | 583-598 | UNREACHABLE | — | typed exhaustion + bookend |
| `exhaust_by_time` | 602-615 | UNREACHABLE | — | no-failure-at-hand variant |
| `terminal_answer` | 620-628 | UNREACHABLE | — | idempotent dead-source answer |
| `shallow_error` (free fn) | 635-643 | REAL-UNPROVEN | — | Fatal-clone doctrine |
| `DevicePhase` (enum) | 651-678 | REAL-UNPROVEN | — | Active/Suspended |
| `AudioDeviceGuard::new` | 713-723 | UNREACHABLE | S0 | F-001; only hotplug.rs constructs it, and its hooks are uncalled |
| `observe_device_lost` | 735-757 | UNREACHABLE | — | fresh window, immediate first reopen |
| `poll` | 762-799 | UNREACHABLE | S3 | F-003 post-notice dead branches |
| `observe_reopen_result` | 814-882 | UNREACHABLE | S4 | F-005 backoff convention; count-before-latch fix documented (851-863) |
| `suspended`/`suspended_position_ns`/`active_device`/`losses`/`reopens`/`suspended_device`/`reopen_attempts_used`/`notified`/`last_reopen_failure` | 887-962 | UNREACHABLE | — | accessors |
| `mark_notified` | 966-998 | UNREACHABLE | — | one-time non-fatal notice; BTreeMap details |
| `SurfaceGuard::new` | 1031-1033 | UNREACHABLE | S0 | F-001 |
| `observe_surface_lost` | 1044-1066 | UNREACHABLE | — | idempotent; RequestSurfaceRecreate + non-fatal event |
| `observe_surface_recreated` | 1074-1080 | UNREACHABLE | — | emits nothing (recorded gap) |
| `observe_resize` | 1085-1088 | UNREACHABLE | — | counter only |
| `compositing`/`gate_present` | 1093-1110 | UNREACHABLE | — | suppression counter |
| `losses`/`recreations`/`suppressed_presents`/`resizes` | 1114-1134 | UNREACHABLE | — | counters |

## 5. Test honesty

No tests in this file. The behavioral suites live in `tests/recovery_env.rs` (another agent's assignment) and `tests/hotplug.rs` (mine — see that audit file). From a grep of recovery_env.rs, the guards are driven directly (guard-level) and through a threaded source harness — the subject is not mocked, but the harness is the *only* driver that exists anywhere.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "recovery … **MET (merged)**" citing merge a181b81 (3.C3) | docs/gates/phase3-exit.md:31 | FALSE (as shipped capability) / REAL-UNPROVEN (as library) | F-001: zero host callers of guards or Engine hooks |
| "The guards never touch the engine directly; the host … drives them" | recovery.rs:38-40 | REAL (the doctrine) — and that is exactly why nothing runs | no host exists |
| "suspend-and-notify default … playback is *never silently restarted*" | recovery.rs:23-26 | REAL-UNPROVEN | implemented (mark_notified 966-998); post-notice re-plug gap F-003 |
| "no v1 catalog event for surface loss/reattachment" | recovery.rs:88-95 | REAL | honest gap, self-recorded |
| Card 3.C3 row (imp): "environment recovery: source retry, device suspend/reopen, surface loss" | docs/13-agent-wave-program.md (wave-3 cards, referenced at recovery.rs:1-2) | REAL-UNPROVEN / UNREACHABLE | cores exist; no integration |

## 7. Dead / duplicated / unreachable code

Everything (F-001): all three guards + the hotplug Engine hooks that consume `AudioDeviceGuard` (hotplug.rs:182-311) are dead in shipped binaries. `windows_opened`, `losses`, `reopens`, `resizes` counters are observable only from code nobody runs.

## 8. Dependencies & flags

Depends on `commands` (EventKind/Stage/ErrorCode), `media-types` (EngineError/NetworkCause). No feature gates. `hotplug.rs` is compiled unconditionally into playback-engine (lib.rs module list) — the dead code ships in every binary.

## 9. Open questions for the next agent

- Does `crates/streaming/` (HTTP/HLS) implement its own retry loop instead of using `SourceRetryGuard`? If yes, that is duplicated policy (W10's wave). Grep shows streaming does not import recovery.rs.
- `crates/audio-output/src/cpal.rs` "BackendNotAdmitted" — if cpal is ever admitted, the pump must call `observe_audio_device_lost`; today nothing does (W08's wave).
- `tests/recovery_env.rs` quality — assigned elsewhere; my hotplug.rs audit covers the engine-hook half.
