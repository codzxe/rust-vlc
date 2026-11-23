# AUDIT: crates/playback-engine/tests/hotplug.rs
| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2025-06-14 |
| Lines / functions | 867 / 12 (4 harness fns, 4 helpers, 8 tests) |
| File verdict | REAL — real `Engine` + real state table + real wire conversion, driven by simulated typed device facts; all 8 tests executed and passing |
| Reachable from shipped entry point? | N/A (test file). What it tests — the hotplug hooks (`Engine::observe_audio_device_lost`/`poll_hotplug`/`observe_audio_reopen`/`observe_renderer_backend`, src/hotplug.rs) — is NOT reachable from any shipped entry point; this suite is its ONLY caller in the tree (see recovery.rs audit F-001). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `cargo test -p playback-engine --test hotplug` -> `8 passed; 0 failed ... finished in 0.00s`, exit 0; rg reachability sweeps (hotplug hooks: only this file calls them) |

## 1. One-paragraph verdict
This is what honest simulation-based testing looks like: the module states up front that no production audio seam in this build can lose a device (the null device cannot vanish) and injects the *typed fact* an OS seam would report, at the exact engine entry point a host would call. Every assertion is against real engine behavior — emission order (device-lost → tick → state-changed), the no-restart proof (session id, epoch, pipeline generation all unchanged across suspend/resume), remembered-position resume, exactly-once non-fatal exhaustion notice with exact payload, fresh windows per loss episode, and a one-bridge agreement test proving the guard's direct-drive notice and the engine-hook translation are byte-identical. The one scope limit — and it is the file's only real issue — is that the machinery it proves is dead in every shipped binary (this suite is the sole caller), and the "startup negotiation report" it replays is fabricated by the test because no real startup emits it.

## 2. What actually works (verified)
- verified by `cargo test -p playback-engine --test hotplug` -> all 8 tests ok, exit 0 (list pasted in run log): flagship vanish/renegotiate/resume, exhaustion non-fatal, repeated loss, backend-report relay + replay-at-open, session-scoped drain rule, no-session loss honesty, wire payload contract, one-bridge notice agreement.
- Baseline agrees (audit/runs/baseline/cargo-test-workspace.txt, hotplug section: 8 passed).

## 3. Findings
F-001 [S3]
Where: crates/playback-engine/tests/hotplug.rs:31-39, 549-553
Code:
```
//! No production audio seam in this build can lose a device (the
//! null device cannot vanish — the scenario-12 honesty note). The
//! simulated event is the **typed fact a host's seam would report**:
```
and
```
    // The startup negotiation happens BEFORE any session exists: the
    // report is cached (this build: no hardware backend compiled in —
    // the honest software-present verdict, see renderer-wgpu's
    // fallback chain).
```
What is wrong: the second comment narrates a "startup negotiation" that does not exist — nothing in any startup path calls `observe_renderer_backend` (rg: zero production callers); the test itself fabricates the call at line 579. The first comment is honest about simulation, but the suite as a whole is the ONLY consumer of every hook it tests, so its green run must not be read as evidence of shipped hotplug behavior. The tests are honest; the capability they prove is unshipped.
Why it matters: a card AC ("hotplug tests with simulated device events") is satisfied at engine level while no shipped host can ever produce the triggering fact — the blast radius of any future regression is a test failure, not a user-visible one, because users cannot reach the code.
Confidence: HIGH — Verification: ran the suite + rg sweeps (recovery.rs audit F-001).

F-002 [S4]
Where: crates/playback-engine/tests/hotplug.rs:864-867
Code:
```
// Silence the unused warning for the audio-format import the harness
// documents but does not need directly (the fixture's frames carry
// it).
const _: Option<AudioFormat> = None;
```
What is wrong: an unused import (`media_types::frame::AudioFormat`, line 54) is retained and silenced with a const hack instead of removed.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/playback-engine/tests/hotplug.rs:95-98
Code:
```
/// Drain the engine's outputs into the wire trail (events convert;
/// receipts are counted away — none of these scenarios submit
/// commands after the open).
```
What is wrong: the comment claims receipts are "counted away"; the body drops them without counting. Comment/code mismatch (cosmetic — no scenario depends on the count).
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/playback-engine/tests/hotplug.rs:750, 759, 772, 798
Code:
```
        other => unreachable!("the wire form is the device-lost row: {other:?}"),
```
What is wrong: four `unreachable!` match guards (tree-wide pattern; test-only blast radius — a catalog rename would panic the test, which is the intended loud failure).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| HotplugFixture (struct) | 72-80 | REAL | — | engine + handle + virtual clock + collector + wire trail |
| run_ms | 84-93 | REAL | — | ms→ns advance, poll per step; `left -= step` safe (step = min(step, left)) |
| drain_wire | 98-106 | REAL | S4 | F-003 comment mismatch; drops receipts |
| engine_events | 113-124 | REAL | — | raw kinds + wire trail dual capture |
| open_playing | 128-182 | REAL | — | primes to Playing, plays 1.5 s, asserts frames>0 before loss; loud assert if Playing not reached |
| position_ns | 186-193 | REAL | — | clock.now().as_nanos(), i64::try_from fallback |
| wire_since / kind_name | 196-214 | REAL | — | display-name vocabulary (no as_str on enum) |
| AtomicU64Placement alias | 216-218 | REAL | — | readability alias, fine |
| device_error | 222-231 | REAL | — | typed Device{AudioOutput, Disconnected} |
| pair / stamped_event | 652-667 | REAL | — | drain-rule test constructors |
| device_vanishes_mid_playback_engine_renegotiates_and_resumes_on_new_device | 242-370 | REAL | — | flagship: order (lost→tick→paused), no-restart ids, new-device migration, resume ≥ loss position, wire ordering, clean close |
| exhausted_reopen_window_suspends_non_fatal_at_remembered_position | 377-498 | REAL | — | budget drain, exactly-one notice, exact payload (device/attempts/context), alive-paused-at-position, late-reopen resume, clean close; `attempts >= 1` is weak but the details assertion pins the count |
| repeated_device_loss_tolerated_with_fresh_windows | 502-537 | REAL | — | 3 episodes, losses/reopens counters, position preserved |
| renderer_backend_report_relays_as_video_backend_changed | 548-644 | REAL | S3 | F-001: replay-at-open works, but the "startup negotiation" is test-fabricated |
| session_scoped_hotplug_events_survive_intra_epoch_supersession | 674-712 | REAL | — | is_pipeline_scoped false for the 3 hotplug kinds; intra-epoch survive, cross-epoch stale; pipeline-scoped control still suppressed — real drain-rule coverage |
| loss_without_a_live_session_is_recorded_not_emitted | 721-736 | REAL | — | honesty guard: recorded, zero emissions, reopen still runs |
| hotplug_wire_payloads_match_the_catalog_contract | 741-800 | REAL | — | all four wire shapes incl. AudioDeviceSuspended → non-fatal PlaybackError details |
| engine_hook_and_guard_direct_drive_notices_agree | 806-862 | REAL | — | one-bridge rule: message + details byte-equal between guard and engine translation |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| device_vanishes_mid_playback... | YES | exact event order, state transitions, id stability, position monotonicity, frame-flow after resume |
| exhausted_reopen_window... | YES | one-shot latch, exact payload fields, frozen clock equality |
| repeated_device_loss... | YES | counters must equal episodes exactly |
| renderer_backend_report... | YES (engine half) | replay-at-open + live relay + cache agreement; the *source* of the report is simulated (F-001) |
| session_scoped_hotplug_events... | YES | direct assertions on the drain-rule API |
| loss_without_a_live_session... | YES | zero-output assertion |
| hotplug_wire_payloads... | YES | exhaustive shape match |
| engine_hook_and_guard..._agree | YES | byte-equality across the two emission paths |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the engine under test is the real one; only the *origin* of the observation is simulated" | hotplug.rs:36-39 | REAL | real Engine/hooks; injected facts only |
| "no sleeps anywhere" (virtual time) | hotplug.rs:41-44 | REAL | shared atomic clock; no sleep in file |
| "the startup report is cached before any session" | hotplug.rs:583-592 | REAL (mechanism) / MISLEADING (narrative) | the cache/replay works; no real startup produces a report (F-001) |

## 7. Dead / duplicated / unreachable code
- `const _: Option<AudioFormat> = None;` silencer (F-002).
- No dead tests; all 8 run (verified).

## 8. Dependencies & flags
- Uses `mod common` (tests/common/mod.rs) harness; dev-deps only (testkit, media-clock, media-types, plugin-api, commands). No feature gates.

## 9. Open questions for the next agent
- Is the ENGINE-side by-time expiry path (poll_hotplug emitting the notice when the window expires with attempts remaining) covered anywhere? This file exhausts by budget; recovery_env.rs covers guard-level windows — UNVERIFIED whether the engine relay of the by-time terminal has a test. Suggest a recovery_env auditor check.
- If any matrix row cites "hotplug tests" as evidence of SHIPPED device-loss recovery, downgrade it to engine-level-only (no shipped caller — recovery.rs audit F-001).
