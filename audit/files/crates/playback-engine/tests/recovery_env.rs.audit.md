# AUDIT: crates/playback-engine/tests/recovery_env.rs
| Field | Value |
|---|---|
| Auditor | W03-A09 |
| Date | 2026-11-17 |
| Lines / functions | 1848 / 46 (21 #[test], 25 helpers/fakes) |
| File verdict | REAL (with overstated "end-to-end" claims for device/surface — see F-001/F-002) |
| Reachable from shipped entry point? | NO for the code under test: `SourceRetryGuard`/`SurfaceGuard` have ZERO production callers (repo-wide rg: defined in src/recovery.rs, re-exported src/lib.rs:132-133, used only by tests/recovery_env.rs + tests/hotplug.rs). `AudioDeviceGuard` is wired into `Engine` via src/hotplug.rs (`Engine::poll_hotplug`), but no shipped host (apps/lumen-cli, apps/control-server, apps/web) calls `poll_hotplug`/`observe_device_lost` (rg over apps/: 0 hits). |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 1 |
| Commands run | rg for guard usages (exit 0); sed quote verification (exit 0); baseline citation (no re-run needed) |

## 1. One-paragraph verdict
This is the strongest kind of test file in the tree: virtual-clock determinism, no sleeps, exact wire-order assertions, and 21/21 tests pass (baseline `cargo-test-workspace.txt:4330-4356`: "running 21 tests … test result: ok. 21 passed; 0 failed"). The policy tests (backoff schedule, exhaustion, TLS pass-through) are direct unit tests of real production code in src/recovery.rs and would fail if that code broke. The two network engine tests are genuine integrations (fake demuxer → real `Engine` demux step → real state machine → real wire mapping). BUT the file's own headline claim — "driven end-to-end with the guards wired into fake stage objects" — is false for the device-loss and surface-loss engine tests: those guards are test-local objects driven by hand, with the test itself performing the pause/play glue and appending guard events to the wire log. And the entire policy surface under test (SourceRetryGuard, SurfaceGuard, and the Engine's hotplug hooks) is dead code in the shipped product — no host drives it.

## 2. What actually works (verified)
— verified by baseline `audit/runs/baseline/cargo-test-workspace.txt` lines 4320-4356 -> all 21 tests pass, "finished in 0.00s" (deterministic virtual time, no sleeps).
— verified by rg `SourceRetryGuard|SurfaceGuard` over the repo -> only src/recovery.rs (definition), src/lib.rs:132-133 (re-export), tests/recovery_env.rs, tests/hotplug.rs. No production caller.
— verified by rg `poll_hotplug|observe_device_lost|observe_reopen_result` over apps/ -> zero hits; callers of `Engine::poll_hotplug` are only crates/playback-engine/tests/hotplug.rs.

## 3. Findings
F-001 [S2]
Where: crates/playback-engine/tests/recovery_env.rs:1474,1490-1498 and 1747,1764-1769
Code:
```
    let mut guard = AudioDeviceGuard::new(RetryWindow::DEFAULT);
...
    let step = guard.observe_device_lost(fixture.now.now_ns(), "dev0", position);
    wire.extend(step.events);
...
    // The suspend flows through the command layer: `playback.pause`.
    fixture.handle.pause(RequestId(100)).must();
```
What is wrong: The module doc (lines 3-6) claims the scenarios are "driven end-to-end with the guards wired into fake stage objects". For the device-loss and surface-loss engine tests this is false: the guard is a test-local variable, `observe_device_lost`/`observe_surface_lost` are called by the test, and the SuspendClock→pause / ResumeClock→play glue is performed BY THE TEST (`fixture.handle.pause(...)`), not by any engine code. The production wiring (`Engine::poll_hotplug` / `observe_reopen_result` in src/hotplug.rs:225,269) is never exercised in this file. Mutation reasoning (UNVERIFIED, no build budget): delete src/hotplug.rs entirely and these tests still pass. Why it matters: the doc-comment narrative ("the suspend is `playback.pause`") presents test-authored glue as engine behavior. What correct looks like: drive the loss through `Engine::poll_hotplug` as tests/hotplug.rs does, or delete the "wired into" claim. Confidence: HIGH — Verification: read.
F-002 [S2]
Where: crates/playback-engine/tests/recovery_env.rs:1-6 (claims) vs repo-wide usage
Code:
```
//! the three environment policies of `playback-engine/src/recovery.rs`
//! proven first as **policy** (pure virtual-time schedules, exact
//! nanosecond bounds, typed terminals) and then **through the real
//! engine** (the doc 03 §8 scenario rows, driven end-to-end with the
```
What is wrong: The proven policies are unreachable product features. `SourceRetryGuard` (interrupted network) is consulted by no production source; `SurfaceGuard` (renderer loss) by no production renderer; `AudioDeviceGuard` only via `Engine::poll_hotplug`, which no shipped host calls. The "interrupted network source" recovery AC is therefore proven for an API nothing ships against — the real input path has no retry guard. Why it matters: green tests here say nothing about whether a shipped `lumen open https://…` ever retries or emits `ReconnectExhausted`. What correct looks like: the demux/source host loop in production constructing and driving `SourceRetryGuard`, then these tests double as regression cover. Confidence: HIGH — Verification: read + rg.
F-003 [S3]
Where: crates/playback-engine/tests/recovery_env.rs:1572-1609
Code:
```
    let lost_at = wire_index(
        &wire,
        &EventKind::AudioDeviceLost {
            device_id: "dev0".to_string(),
```
What is wrong: The "causal ordering" assertions (`lost_at < paused_at`, `changed_at < playing_at`, doc 05 §4.3 cited) are tautologies: the test appended the guard's events to `wire` itself (lines 1491, 1525) before issuing the commands, so the ordering can only hold. No engine ordering is proven. Why it matters: the assertion is dressed as an engine-contract check but tests the test's own sequencing. Confidence: HIGH — Verification: read.
F-004 [S3]
Where: crates/playback-engine/tests/recovery_env.rs:163-168,1773-1798
Code:
```
fn stall_tick() -> DemuxEvent {
    DemuxEvent::Recovered {
        skipped: 0,
        reason: "network read pending (the blocked-read stand-in)",
    }
}
```
plus the surface "present gate" loop at 1774-1789 where the test itself consults `guard.gate_present()` and counts `presented`.
What is wrong: Two stand-ins keep the engine tests from the real phenomena: (a) a stalled source is simulated by an observably-inert `Recovered{skipped:0}` token (honestly documented at lines 67-79, with a `[PROPOSAL]` for a real stall seam — but the never-hang proof is only as strong as the fake's cooperation); (b) "video presents are suppressed" (assert at 1794-1798) is the guard's own boolean bookkeeping — `presented == 0` because the test asks a dirty guard and no renderer exists in the test. Confidence: HIGH — Verification: read.
F-005 [S4]
Where: crates/playback-engine/tests/recovery_env.rs:133-138
Code:
```
fn clock_ns(clock: &MediaClock) -> i64 {
    match clock.now().as_nanos() {
        Ok(Nanoseconds(ns)) => i64::try_from(ns).unwrap_or(i64::MAX),
        Err(_) => 0,
    }
}
```
What is wrong: A clock error silently becomes 0 and any downstream `assert_eq!(clock_ns(...), position)` would compare a fabricated 0. Documented in-comment as "the honest floor", and unreachable in practice (virtual clock never errs), but the helper converts a typed error into a silent value. Confidence: HIGH — Verification: read.

## 4. Per-function table (file >100 LOC — every item)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| advance_now | 124-127 | REAL | — | Virtual clock bump, saturating. |
| clock_ns | 133-138 | REAL | S4 | F-005 Err→0. |
| NetworkLink::new | 151-158 | REAL | — | Shared atomics + guard under Mutex. |
| stall_tick | 163-168 | REAL | S3 | F-004 inert stall token. |
| NetworkDemuxer::start | 184-186 | REAL | — | Returns track list. |
| NetworkDemuxer::next_event | 188-233 | REAL | — | Genuine guard integration; feeds, interrupts, time-expiry, retry gating. |
| NetworkDemuxer::seek | 235-241 | REAL | — | Typed refusal; unused in this file. |
| PassDecoder::configure/decode/flush | 253-277 | REAL | — | Kind-aware 1:1 packet→frame; stale-generation filter. |
| PassProvider::decoder_for | 284-293 | REAL | — | Fake seam, kind from track. |
| video_frame | 296-306 | REAL | — | 4-byte NV12 16x16 dummy — fine for flow tests. |
| network_down/tls_failed/corrupt_media/device_gone | 313-355 | REAL | — | Typed error fixtures. |
| EnvFixture::link/frames_of_kind/frames_total | 380-405 | REAL | — | Panic-free lock recovery, honest counters. |
| frame_kind | 408-414 | REAL | — | Trivial classifier. |
| open_env | 420-454 | REAL | — | Real Engine + SessionHandle, virtual clock, tiny queues, frozen throttle. |
| open_network | 458-469 | REAL | — | Wires guard INTO the fake demuxer (the only true "wired" fake). |
| open_script_audio / open_script_av | 473-512 | REAL | — | Scripted FakeDemuxer from common. |
| poll_round | 521-534 | REAL | — | Collects wire+raw outputs. |
| poll_engine_until | 538-557 | REAL | — | Bounded driver with assert on failure. |
| state_changes / buffering_values / wire_index / fatals / take_fail_source | 560-600 | REAL | — | Honest extractors. |
| 8 policy tests (607-904) | — | REAL | — | Direct unit tests of recovery.rs; would fail if policy broke. |
| 4 device policy tests (912-1072) | — | REAL | — | Direct unit tests of AudioDeviceGuard. |
| 4 surface policy tests (1080-1175) | — | REAL | — | Direct unit tests of SurfaceGuard. |
| engine_interrupted_network_rebuffers_then_resumes | 1187-1315 | REAL | — | True engine integration; exact state trace + 0.0/1.0 bookends. |
| engine_permanent_network_interruption_is_the_typed_fatal | 1322-1457 | REAL | — | True engine integration; fatal-before-error ordering, no fabrication. |
| engine_device_loss…command_layer | 1469-1616 | SHALLOW | S2 | F-001/F-003: guard manual, glue test-authored. |
| engine_device_reopen_exhaustion…non_fatal | 1623-1730 | SHALLOW | S2 | Same manual-guard pattern. |
| engine_surface_loss…without_restart | 1742-1848 | SHALLOW | S3 | Engine part real (stays playing, 120 frames); gate part tautological. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| source_retry_* (5 policy tests) | YES | Exact-value asserts on real guard. |
| retryability_classification / exhausted_error_maps | YES | Pins the classification set and wire mapping. |
| device_* policy tests (4) | YES | Direct on AudioDeviceGuard. |
| surface_* policy tests (4) | YES | Direct on SurfaceGuard. |
| engine_interrupted_network_rebuffers_then_resumes | YES (engine parts) | State trace, frame counts, delivered counts all cross the real Engine. |
| engine_permanent_network_interruption_is_the_typed_fatal | YES (engine parts) | Fatal payload, wire ordering, no-fabrication counters cross the real Engine. |
| engine_device_loss…command_layer | NO for hotplug wiring; YES for pause/play | Guard driven manually; deleting src/hotplug.rs leaves it green (mutation reasoning, UNVERIFIED). |
| engine_device_reopen_exhaustion… | NO for hotplug wiring | Same pattern; the "session stays paused" part is real engine behavior under pause. |
| engine_surface_loss… | PARTIAL | Engine flow real; suppression/gate assertions are guard-internal. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "driven end-to-end with the guards wired into fake stage objects" | file lines 3-6 | FALSE for device/surface tests | Lines 1474/1490-1498/1764: guard is test-local, glue is test-authored; only NetworkDemuxer wires a guard. |
| "interrupted network source … recovery" as product behavior | module doc §What is proven | UNREACHABLE | rg: SourceRetryGuard has no production caller. |
| stall stand-in inertness | lines 67-79 | REAL | Test at 1301-1307 asserts no PlaybackError surfaced. |

## 7. Dead / duplicated / unreachable code
None inside the file (all helpers used). The unreachable production surface is F-002's subject (recovery.rs guards + Engine::poll_hotplug, no shipped caller).

## 8. Dependencies & flags
No feature gates in this file; depends on playback_engine, media_clock, media_types, commands, codec_api, plugin_api, testkit (Must = real panicking unwrap, testkit/src/lib.rs:53-63), tests/common (SharedNow virtual clock, Collector, FakeDemuxer).

## 9. Open questions for the next agent
- tests/hotplug.rs (not assigned here) exercises `Engine::poll_hotplug` — confirm whether any of ITS scenarios would fail if the engine wiring broke, and whether any shipped host is ever expected to call it (docs 03 §4.9(d) ownership).
- Who is the intended production driver of `SourceRetryGuard`? If the plan is the demux host loop, the integration is missing, not the policy.
