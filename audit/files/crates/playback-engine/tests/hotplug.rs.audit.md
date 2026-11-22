# AUDIT: crates/playback-engine/tests/hotplug.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 868 / 8 `#[test]` fns + 12 harness helpers |
| File verdict | REAL-UNPROVEN (a genuinely behavioral suite over the real engine — guarding an API surface no production host calls) |
| Reachable from shipped entry point? | N/A (test file). The hooks it exercises (`observe_audio_device_lost` / `poll_hotplug` / `observe_audio_reopen` / `observe_renderer_backend`) have **zero production callers** — see my recovery.rs audit F-001; this suite is their only driver anywhere. |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | full-file read; grep `#[test]` → 8; cross-reference to recovery.rs/hotplug.rs source audits (my files #3); sed quote verification |

## 1. One-paragraph verdict

The card-7.C1 acceptance suite, and it is the real thing as tests go: the **real Engine** over a scripted media and a shared virtual clock, with only the *origin* of the device-loss observation simulated (the module doc is explicit and honest about this, lines 31-39). The flagship test proves the full episode — vanish mid-play → suspend is the engine's own `Pause` row (same session id, same epoch, same generation — all three asserted) → reopen window offers an attempt → a *new* device resumes at the remembered position → cause-then-effect ordering asserted positionally in the wire trail → frames keep flowing. The exhaustion test pins the non-fatal one-time notice with its full payload (device, attempt count, context), position freeze, and the late-reopen-resumes behavior. Repeated loss, the backend-report relay (including the startup replay-at-open), the session-scoped drain rule with a pipeline-scoped control, the no-session honesty guard, the catalog payload shapes, and a one-bridge-rule agreement test between the engine's notice and the guard's own — thorough. The structural caveat that dominates everything: all 868 lines guard four public engine entry points that **no shipped binary ever calls** — the suite is the only caller of the feature it proves.

## 2. What actually works (verified)

- The harness drives the real `Engine` (`engine.open(config)`, `engine.poll()`, real `MediaClock::system_monotonic` over a shared atomic now) — the subject is not mocked.
- Ordering assertions are positional in the drained wire trail (332-362), not just presence checks.
- The no-restart proof (281-284, 312-314, 466-467) asserts all three identity tokens — the strongest form of the claim.
- W00 baseline: the playback-engine suite (including this file) ran green in the 1751-pass workspace run — UNVERIFIED by my own execution (ADAPTATION).

## 3. Findings

### F-001 [S3] — 868 lines of acceptance proof for an API surface with zero production callers

Where: the whole file; the reachability proof in my recovery.rs audit F-001 (hooks at crates/playback-engine/src/hotplug.rs:182/225/264/311, callers: this file only).

Code (hotplug.rs:33-37, verbatim):
```rust
//! No production audio seam in this build can lose a device (the
//! null device cannot vanish — the scenario-12 honesty note). The
//! simulated event is the **typed fact a host's seam would report**:
//! the test calls `Engine::observe_audio_device_lost` exactly where
```

What is wrong: not the tests — they are honest about the simulation and rigorous about the engine's behavior. The finding is the structural inversion this file embodies: the *only* driver of the hotplug machinery is its own acceptance suite. A green suite plus zero host callers means the feature is proven and unreachable simultaneously. The module doc's "exactly where an OS callback would" describes an integration that does not exist in any shipped app.

Why it matters: claim-ledger consumers citing "hotplug tests pass" as evidence of device-loss recovery infer a product capability; the shipped product cannot exhibit it (no real device, no callback, no caller). S3 (misleading evidence weight), consistent with my recovery.rs S0 being carried by the gate claim rather than this file.

Confidence: HIGH — Verification: read + cross-file grep.

### F-002 [S4] — Unused import silenced with a `const _` hack instead of removed

Where: crates/playback-engine/tests/hotplug.rs:54 and 864-867.

Code (864-867, verbatim):
```rust
// Silence the unused warning for the audio-format import the harness
// documents but does not need directly (the fixture's frames carry
// it).
const _: Option<AudioFormat> = None;
```

What is wrong: `use media_types::frame::AudioFormat` (line 54) is unused; rather than delete the import, the file adds a dummy const to suppress the warning. §3.5 lint-suppression hygiene. Delete the import.

Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (AC, simulation doctrine, time) | 1-44 | REAL (honest) | S3 | F-001 context |
| `HotplugFixture` (struct) | 72-80 | REAL-UNPROVEN | — | engine + handle + clock + sink + wire |
| `run_ms` | 84-93 | REAL-UNPROVEN | — | virtual-time pump cadence |
| `drain_wire` / `engine_events` | 98-124 | REAL-UNPROVEN | — | wire trail collection, dual-view |
| `open_playing` | 128-182 | REAL-UNPROVEN | — | asserts Playing + frames>0 before the loss |
| `position_ns` / `wire_since` / `kind_name` | 186-214 | REAL-UNPROVEN | — | assertion vocabulary |
| `AtomicU64Placement` alias | 218 | REAL-UNPROVEN | S4-adjacent | rename-for-readability; harmless |
| `device_error` | 222-231 | REAL-UNPROVEN | — | typed L0 vocabulary |
| `device_vanishes_mid_playback_engine_renegotiates_and_resumes_on_new_device` | 242-370 | REAL-UNPROVEN | — | the flagship; ids/ordering/position/frames |
| `exhausted_reopen_window_suspends_non_fatal_at_remembered_position` | 377-498 | REAL-UNPROVEN | — | budget drain loop with break-on-notify; full payload asserts; late-reopen resume |
| `repeated_device_loss_tolerated_with_fresh_windows` | 502-537 | REAL-UNPROVEN | — | 3 episodes, fresh budgets, position preserved |
| `renderer_backend_report_relays_as_video_backend_changed` | 548-644 | REAL-UNPROVEN | — | startup replay at open + live fallback relay |
| `session_scoped_hotplug_events_survive_intra_epoch_supersession` | 674-712 | REAL-UNPROVEN | — | drain rule with a pipeline-scoped control |
| `loss_without_a_live_session_is_recorded_not_emitted` | 721-736 | REAL-UNPROVEN | — | honesty guard |
| `hotplug_wire_payloads_match_the_catalog_contract` | 741-800 | REAL-UNPROVEN | — | 4 wire shapes |
| `engine_hook_and_guard_direct_drive_notices_agree` | 806-862 | REAL-UNPROVEN | — | one-bridge rule: identical message + details |
| `const _` lint hack | 864-867 | S4 | F-002 |

## 5. Test honesty (8 `#[test]`)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `device_vanishes_mid_playback…` (242-370) | YES | engine ids, event ordering, state, stats, frames, position — 20+ assertions |
| `exhausted_reopen_window…` (377-498) | YES | attempt loop with notified-break; notice payload field-by-field; freeze + late resume |
| `repeated_device_loss_tolerated…` (502-537) | YES | per-episode stats + position monotonicity |
| `renderer_backend_report_relays…` (548-644) | YES | cached report equality + wire presence + live relay |
| `session_scoped_hotplug_events…` (674-712) | YES | staleness matrix with control |
| `loss_without_a_live_session…` (721-736) | YES | recorded-not-emitted + reopen still runs |
| `hotplug_wire_payloads_match_the_catalog_contract` (741-800) | YES | exact wire shapes |
| `engine_hook_and_guard_direct_drive_notices_agree` (806-862) | YES | byte-level message + details equality across two emission paths |

The mock-of-the-subject concern does not apply: the engine is real; only the OS-event origin is injected — the sanctioned and documented doctrine.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "hotplug tests with simulated device events" (card 7.C1 AC) | module docs 4-9 | REAL (tests) / the capability is UNREACHABLE in shipped binaries | F-001; recovery.rs audit |
| "No production audio seam in this build can lose a device" | module docs 33-34 | REAL | honest; the null device cannot vanish |
| "the engine under test is the real one" | module docs 37-38 | REAL | verified in the harness code |

## 7. Dead / duplicated / unreachable code

The unused `AudioFormat` import + its silencing const (F-002). No duplicated logic.

## 8. Dependencies & flags

dev-deps via the crate: `playback_engine`, `media_clock`, `media_types`, `plugin_api`, `commands`, `testkit`; `mod common` (shared harness — another agent's file). No cfg gates; all 8 tests run in the default suite.

## 9. Open questions for the next agent

- `tests/common/` (Collector, SharedNow, config_with, packet, track, frames_of) — the shared fixture library used by many of my remaining test files; whoever audits it should confirm `config_with` builds a real pipeline (it appears to).
- When the tauri shell gains a real audio device (cpal admission, ADR-005), the hook wiring is the missing integration; this suite is its contract.
