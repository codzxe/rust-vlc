# AUDIT: crates/playback-engine/src/recovery.rs#part2 (lines 568-1135)
| Field | Value |
|---|---|
| Auditor | W03-A10 |
| Date | 2026-11-21 |
| Lines / functions | 568 (chunk 568-1135 of 1136; boundary note: `fail_early`'s signature at 564 is part1/A06's, its body spans into this chunk) / 26 items in range |
| File verdict | UNREACHABLE (real policy code, test-executed; no shipped entry point drives any of it) |
| Reachable from shipped entry point? | NO, all three guards: `AudioDeviceGuard` ← hotplug.rs:135 `HotplugState` ← session.rs:961 — but `poll_hotplug`/`observe_audio_device_lost`/`observe_audio_reopen` are called ONLY by `crates/playback-engine/tests/hotplug.rs` (rg over tree, excl. hotplug.rs itself); apps/cli has zero hotplug references. `SourceRetryGuard` and `SurfaceGuard` are referenced ONLY by `tests/recovery_env.rs`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg "SourceRetryGuard\|SurfaceGuard" -g '!target'` → tests only; `rg -i "hotplug\|device_lost" apps/cli/src/` → empty; `rg "poll_hotplug\|observe_audio_device_lost\|observe_audio_reopen"` → only tests/hotplug.rs; `sed -n` on every quoted range; `rg "#\[test\]" recovery_env.rs` → 21 tests |

## 1. One-paragraph verdict
Three recovery policies live here: the source-retry guard's terminal tail, the audio-device suspend/reopen guard, and the renderer-surface-loss guard. The logic itself is careful and correct as far as tracing goes — bookend discipline matches the wire-ordering guard's rules, the terminal-count fix (card 7.C1) is present and ordered correctly, loss handling is idempotent, window arithmetic saturates — and all of it is exercised by 21+ mutation-sensitive tests that passed in the no-fail-fast baseline. But the entire chunk is a policy library with no product driver: the audio guard's hotplug hooks are called only by tests, and the source-retry and surface guards have no callers outside their own test file. Doc 03's §4.9(d)/§4.11(d) recovery behaviors are, in the shipped `lumen` binary, fiction.

## 2. What actually works (verified)
- Episode-bookend discipline: `fail_early`/`exhaust_after_failure`/`exhaust_by_time` (566-568, 585-589, 604-606) each push `SessionBuffering { progress: 1.0 }` when a retry episode was open — exactly what `OrderingGuard`'s `EpisodeUnclosedAtExit`/`EpisodeNeverClosed` rules demand (events.rs:1118-1119, 1087-1089, cross-checked in my events.rs part2 audit).
- The card 7.C1 terminal-count ordering (851-864): the incremented `reopen_used` is stored into the phase BEFORE `mark_notified` embeds it, so `reopen_attempts_used()` and the notice agree — traced, and the comment says tests/hotplug.rs found the original discrepancy.
- Time-expiry notice names the last observed failure or honestly reports none (780-791 + `last_failure` reset at 742 on every fresh loss) — "None must mean no failure was ever seen" holds by construction.
- Idempotent surface loss (1045-1047 dirty-flag early return) and loss-then-recreate-then-loss episode freshness (1048-1049 vs 1075-1078) — traced.
- Saturating window arithmetic: `now_ns.saturating_add` at 748 and 850 — no u64 overflow on hostile clocks.
- Tests executed green: `orchrun-test-workspace-nofailfast.log.txt` — playback-engine not among the 7 failing binaries (recovery_env.rs 21 tests + tests/hotplug.rs included in the 4534 passed).

## 3. Findings

### F-001 [S3]
Where: crates/playback-engine/src/recovery.rs:697-708 (AudioDeviceGuard), 1019-1026 (SurfaceGuard), and the SourceRetryGuard tail 564-628
Code:
```rust
#[derive(Debug)]
pub struct AudioDeviceGuard {
    policy: RetryWindow,
    phase: DevicePhase,
    losses: u64,
    reopens: u64,
```
What is wrong: none of the three guards has a shipped driver. The only caller chain that exists at all (`AudioDeviceGuard` ← hotplug.rs:135 ← session.rs:961 `pub(crate) hotplug: HotplugState`) dead-ends: `Engine::observe_audio_device_lost` / `poll_hotplug` / `observe_audio_reopen` are invoked exclusively by `crates/playback-engine/tests/hotplug.rs`. `SourceRetryGuard` (streaming, Phase 6) and `SurfaceGuard` (renderer) have zero non-test callers anywhere in the tree.
Why it matters: doc 03 §4.9(d) (device suspend-and-notify) and §4.11(d) (surface loss → audio-only degradation) describe shipped-engine behavior; in the `lumen` binary none of it can ever run. The events.rs catalog marks `AudioDeviceLost`/`AudioDeviceChanged` "Emitted" — true of the library API, unreachable in the product. Every recovery behavior this chunk implements is test-only reality.
What correct looks like: the desktop/web host wires device-loss callbacks into `Engine::observe_audio_device_lost` and a tick into `poll_hotplug`; until then the catalog rows and doc sections should carry the same "WiredLater" honesty the events catalog uses elsewhere.
Confidence: HIGH — Verification: rg over the whole tree (globs excluding target), apps/cli/src read for hotplug references (none).

### F-002 [S3]
Where: crates/playback-engine/src/recovery.rs:1072-1080
Code:
```rust
    /// Emits nothing — the recorded gap: v1 has no surface event
    /// kind (see the module docs' `[PROPOSAL]`).
    pub fn observe_surface_recreated(&mut self) -> RecoveryStep {
        if self.dirty {
            self.dirty = false;
            self.recreations += 1;
        }
        RecoveryStep::quiet()
    }
```
What is wrong: surface LOSS emits a non-fatal `playback.error` (stage renderer, code device, 1056-1064) but surface RECOVERY emits nothing. A host that learns "audio-only degradation" from the wire has no wire signal that compositing resumed — the degradation notice is permanent from the event stream's perspective.
Why it matters: the only non-fatal renderer error in the catalog can never be cleared by an event; hosts must poll `SurfaceGuard::compositing()` — a guard that (per F-001) no shipped code even holds. The asymmetry is recorded as a `[PROPOSAL]` gap, which keeps it honest but does not keep it correct.
What correct looks like: a `video.surface-restored` (or `VideoBackendChanged`-reuse) emission on recreate, or at minimum the degradation event's details naming the polling contract.
Confidence: HIGH — Verification: read (both emission paths); the gap is self-declared at 1014-1015 and tested-as-gap at recovery_env.rs:1118-1126.

### F-003 [S4]
Where: crates/playback-engine/src/recovery.rs:792-797
Code:
```rust
        if !notified && reopen_used < self.policy.max_attempts && now_ns >= next_reopen_ns {
            return RecoveryStep {
                action: RecoveryAction::ReopenDevice,
                events: Vec::new(),
            };
        }
```
What is wrong: `poll` does not latch "attempt in flight": every poll at/after `next_reopen_ns` answers `ReopenDevice` again until `observe_reopen_result` advances the schedule. The one-result-per-attempt contract lives in the caller's discipline, not the state machine.
Why it matters: a host with a tick-driven poll loop and an async reopen would double-fire attempts. Today's synchronous test host cannot, so this is latent.
What correct looks like: an `attempt_in_flight` flag set by `poll`, cleared by `observe_reopen_result`.
Confidence: HIGH on the code fact / MEDIUM on materiality — Verification: read; no test polls twice before reporting (checked tests/hotplug.rs call shapes at 292-301, 394-399).

### F-004 [S4]
Where: crates/playback-engine/src/recovery.rs:747,850
Code:
```rust
            next_reopen_ns: now_ns,
            window_end_ns: now_ns.saturating_add(self.policy.window_ns),
```
and
```rust
                let next_reopen_ns = now_ns.saturating_add(self.policy.backoff_ns(reopen_used + 1));
```
What is wrong: the first reopen attempt is scheduled immediately (747: `next_reopen_ns: now_ns`), and after the Nth failure the next attempt waits `backoff(N+1)` (850) — so `backoff_ns(1)` is never consulted by this chunk. Whether that is deliberate (`backoff(1) == 0` documenting "first retry immediate") or an off-by-one in the schedule is undecidable from part2 alone.
Why it matters: if `backoff_ns(1) > 0`, the documented "immediate first attempt" is a special case silently diverging from the schedule; if `backoff_ns(1) == 0`, the +1 indexing is right but unobvious.
What correct looks like: schedule attempt 1 at `loss_time + backoff(1)` with `backoff(1) == 0` documented, or a comment at 747 naming the exemption.
Confidence: MEDIUM — Verification: read; `RetryWindow::backoff_ns` body is in part1 (A06's chunk) — flagged as cross-chunk question Q-1.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SourceRetryGuard::fail_early` | 564-576 | REAL | — | Bookend + keep-original-error; body spans chunk boundary |
| `SourceRetryGuard::exhaust_after_failure` | 583-598 | REAL | — | Typed `retry_exhausted_error(attempts, Some(&error))` |
| `SourceRetryGuard::exhaust_by_time` | 602-615 | REAL | — | `None` failure honest (no failure observed) |
| `SourceRetryGuard::terminal_answer` | 620-628 | REAL | — | Idempotent shallow copy; quiet when not terminal |
| `shallow_error` | 635-643 | REAL | — | Fatal-clone doctrine (stage/kind/context, no source) — consistent with event.rs |
| `DevicePhase` (enum) | 651-678 | REAL | — | Active/Suspended with full window state |
| `AudioDeviceGuard` (struct) + `new` | 697-723 | REAL | S3 | F-001 (no shipped driver); Active with empty device id before first loss — documented at 900-902 |
| `observe_device_lost` | 735-757 | REAL | — | Fresh window per loss; `last_failure` reset; SuspendClock + event |
| `poll` | 762-799 | REAL | S4 | Expiry-before-due ordering correct; F-003 (no in-flight latch) |
| `observe_reopen_result` | 814-882 | REAL | — | Ok: resume from remembered position (never zero); Err: backoff or one-time terminal; count stored before latch (card 7.C1) |
| `suspended` / `suspended_position_ns` / `active_device` / `losses` / `reopens` / `suspended_device` / `reopen_attempts_used` / `notified` / `last_reopen_failure` | 887-962 | REAL | — | Plain accessors; `last_reopen_failure` Display form matches the notice by construction (960-962) |
| `mark_notified` | 966-998 | REAL | — | Non-fatal PlaybackError, BTreeMap details, two message shapes (with/without last failure) |
| `SurfaceGuard` (struct) + `new` | 1019-1033 | REAL | S3 | F-001 |
| `observe_surface_lost` | 1044-1066 | REAL | — | Idempotent via dirty flag; degradation event non-fatal |
| `observe_surface_recreated` | 1074-1080 | REAL | S3 | F-002 (silent recovery) |
| `observe_resize` | 1085-1088 | REAL | — | Counter only, non-fatal by design |
| `compositing` / `gate_present` | 1093-1110 | REAL | — | Present gate + suppression counter — correct |
| `losses` / `recreations` / `suppressed_presents` / `resizes` | 1114-1134 | REAL | — | Plain accessors |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| recovery_env.rs SurfaceGuard tests (1082-1135) | YES | Event payload (stage/code/fatal/details context), action kind, gate behavior, and all four counters asserted exactly; removing the dirty-flag idempotency or the suppression counter flips asserts |
| recovery_env.rs SourceRetryGuard tests (639-860) | YES | 21 tests pin window/budget/bookend/terminal-answer behavior (spot-read 639-860 headers) |
| tests/hotplug.rs (256-518) | YES | Drives the real Engine through loss → poll → reopen cycles, asserts emitted payloads and counters (call sites listed in Commands run) |
| Baseline: all green — `orchrun-test-workspace-nofailfast.log.txt` (playback-engine not in the 7 failures) | — | cited |

No test covers: a host polling twice before reporting a reopen result (F-003); the schedule exemption of `backoff(1)` (F-004).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| §4.9(d) suspend-and-notify on device loss | doc 03, guard doc 657-696 | SHALLOW (library-real, product-unwired) | F-001: only tests drive it |
| "resume continues from here, never from zero" | field doc 664-666 | REAL | observe_reopen_result Ok path reads the remembered `position_ns` (821-830); tests/hotplug.rs asserts it |
| §4.11(d) surface loss → audio-only degradation with explicit event | doc 03, guard doc 1005-1018 | SHALLOW (library-real, product-unwired; recovery side silent — F-002) | rg: SurfaceGuard has no non-test caller |
| AudioDeviceLost/Changed "Emitted" | events.rs catalog rows 709-716 | REAL for the library / UNREACHABLE for the product | emission sites hotplug.rs:196,281; drivers test-only (F-001) |

## 7. Dead / duplicated / unreachable code
- The entire chunk is unreachable from shipped entry points (F-001) — the most extensive unwired-surface case in my domain so far.
- No internal duplication; the three exhaustion helpers (fail_early/exhaust_after_failure/exhaust_by_time) share a shape but differ in error construction — acceptable.

## 8. Dependencies & flags
- No feature gates, no unsafe, no allocation beyond event payloads and cloned device-id strings (bounded, per-episode).
- Depends on `RetryWindow` (part1) for policy constants and `retry_exhausted_error` — part1/A06's to audit.

## 9. Open questions for the next agent
- Q-1 (recovery.rs part1/A06): does `RetryWindow::backoff_ns(1) == 0` (F-004's schedule question)? And what are DEFAULT window/max/backoff — are they cited in docs?
- Q-2 (hotplug.rs — my next file): confirm no hidden non-test driver exists for `HotplugState` inside session.rs's own pump (I searched direct method names only).
- Q-3 (events catalog owner): should AudioDeviceLost/Changed carry "Emitted (engine API; host wiring pending)" like the WiredLater rows do for streaming?
