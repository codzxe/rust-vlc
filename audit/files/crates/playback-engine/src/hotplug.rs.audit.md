# AUDIT: crates/playback-engine/src/hotplug.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 395 / 10 functions (0 tests in-file; integration tests live in tests/hotplug.rs) |
| File verdict | UNREACHABLE (real code, zero shipped callers) |
| Reachable from shipped entry point? | NO — `rg observe_audio_device_lost|poll_hotplug|observe_audio_reopen|observe_renderer_backend` over apps/ (all four apps) and crates/: the ONLY callers are crates/playback-engine/tests/hotplug.rs (256, 292, 301, 384, 394, 399, 476, 484, 508, …). renderer-wgpu/src/negotiate.rs references `Engine::observe_renderer_backend` in doc comments only (lines 32, 244, 305) — no call. No app bridges platform device events to these hooks (apps/web has its own WebGPU `LossReason::DeviceLost` recovery path, apps/web/src/render/backend.rs:509-581, which never touches the engine's hotplug API). |
| Findings | S0:0 S1:0 S2:1 S3:0 S4:2 |
| Commands run | `sed -n` verbatim-quote checks; `rg` caller searches over apps/ and crates/ (results above). No cargo (adaptation). |

## 1. One-paragraph verdict

The engine-side hotplug executor is coherent, careful code — suspend-on-loss through the same transition table a `pause` command rides, a caller-driven reopen window with a one-time non-fatal notice, a cached renderer backend report replayed at open — and it is **dead in the shipped build**: no shipped entry point (`lumen` CLI, desktop, desktop-tauri, web) ever calls any of the four hooks. The module's own doc frames the contract correctly ("hosts call [`Engine::poll_hotplug`] on their own cadence"), but no host exists. Since the only audio backend in this build is the null device (W00/ADR-005 era; apps/cli/Cargo.toml comment: "the audio path over the null device — no audio hardware in this sandbox"), there is also nothing that could report a loss. The wave-3 hostile question "hotplug: reachable at all in this build?" answers: no. Wire events `audio.device-lost`/`-changed` and `video.backend-changed` cannot be produced by any shipped binary; the `RendererBackendReport` replay in session.rs's open path (session.rs:1092 `replay_backend_report_at_open`) is equally dead in production.

## 2. What actually works (verified)

- The suspend/resume flow applies real table rows through the single-writer path: `apply_hotplug_input` (332-343) → `session.apply(SessionInput::Pause/Play, ...)` — the same `Session::apply` audited in session.rs; a loss while Playing genuinely freezes the clock (Pause row's FreezeClock effect).
- Event ordering claims (fact before reaction) are structurally guaranteed by emission order (196-198 before 209; 281 before 282).
- The catch-all arms in `poll_hotplug` (244-248) are documented as unreachable-for-this-guard with the reasoning spelled out.
- Integration coverage exists and is specific (tests/hotplug.rs drives loss→poll→reopen cycles against a real Engine); but tests cannot establish shipped reachability — they call the hooks directly.

## 3. Findings

### F-001 [S2] The entire hotplug surface is unreachable from every shipped entry point
Where: crates/playback-engine/src/hotplug.rs:182, 225, 264, 311 (the four pub hooks); caller search results above
Code (hotplug.rs:20-24, verbatim — the doc's own contract):
```rust
//! - [`Engine::poll_hotplug`] — drive the reopen window on the
//!   caller's virtual time. Answers
//!   [`HotplugStep::ReopenDevice`] when an attempt is due; emits the
//!   one-time non-fatal `AudioDeviceSuspended` notice when the window
//!   expires (the session stays suspended at the remembered position
//!   — a non-fatal `playback.error`, never a session kill).
```
Code (hotplug.rs:61-64, verbatim — the admission that the host must drive):
```rust
//! does NOT drive the reopen window — it has no `now_ns` to give;
//! hosts call [`Engine::poll_hotplug`] on their own cadence (the
//! same call shape the platform event queues impose).
```
What is wrong: the design requires a host to (a) observe device loss at an OS seam, (b) call `observe_audio_device_lost`, (c) poll `poll_hotplug` on a cadence, (d) perform reopens and report via `observe_audio_reopen`, and (e) relay renderer negotiation via `observe_renderer_backend`. rg proves no shipped host performs any of these. The doc 05 §3.2 wire events this module owns (`audio.device-lost`, `audio.device-changed`, `video.backend-changed`) are therefore unproducible by the product.
Why it matters: any doc/matrix claim that device hotplug or renderer-backend reporting works end-to-end in the shipped build is false at the wiring level, however correct the engine half is. In the current build the null audio device cannot be lost, so the gap is invisible — but the first build with a real audio backend ships a device-loss policy that does nothing.
What correct looks like: one host bridge — e.g. the desktop shell's ticker calling `poll_hotplug(now_ns)` and the platform `DeviceEventQueue` drain calling `observe_audio_device_lost` — plus renderer-wgpu's negotiation actually calling `observe_renderer_backend` instead of only documenting it (negotiate.rs:32/244/305).
Confidence: HIGH — Verification: rg over apps/ and crates/ (call sites enumerated above); read of the module and its integration test.
Verdict for the feature: UNREACHABLE per §5.

### F-002 [S4] Hotplug applies consume rejections and empty sessions silently, with no counter
Where: crates/playback-engine/src/hotplug.rs:332-343, 348-352
Code (332-343, verbatim):
```rust
    fn apply_hotplug_input(&mut self, input: SessionInput) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        // The pause-from-Playing immediate tick and every row effect
        // flow exactly as a submitted command's would.
        if let Ok(outputs) = session.apply(input, InputContext::default()) {
            for output in outputs {
                self.outbox.push(output);
            }
        }
    }
```
What is wrong: a typed rejection (e.g. resume after the user paused or stopped) is dropped with no observability — `HotplugStats` has no `rejected_applies` counter, unlike the engine's outbox counters. Documented as intended ("the honest answers are no-ops, not errors", 329-331), and defensible, but a suspend/resume that silently fails to apply is exactly the class of fact a stats surface exists to carry.
Confidence: HIGH — Verification: read.

### F-003 [S4] Loss with no live session emits nothing — the device-lost fact is unobservable
Where: crates/playback-engine/src/hotplug.rs:179-181, 345-347
Code (179-181, verbatim):
```rust
    /// With no live session the loss is still recorded in
    /// [`Engine::hotplug_stats`] (the guard's truth), but nothing is
    /// emitted — there is no session whose stream could carry it.
```
What is wrong: a device loss between sessions (or before the first open) produces no wire event at all — a host UI learns of it only by polling `hotplug_stats()`, which no shipped host does (F-001). The design choice is documented and internally consistent (the engine's stream is session-scoped), but combined with F-001 the loss fact is doubly invisible. Recorded as hygiene: the constraint should be stated in doc 05's event catalog if the wire is session-scoped.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `HotplugStep::is_action` | 102-108 | REAL | — | trivial |
| `HotplugState::default` | 144-151 | REAL | — | guard with RetryWindow::DEFAULT |
| `HotplugState::stats` | 155-164 | REAL | S4 | clones String pair per call (event-rate, fine) |
| `Engine::observe_audio_device_lost` | 182-214 | UNREACHABLE | S2 | only tests/hotplug.rs calls it; position `unwrap_or(0)` with no session (documented) |
| `Engine::poll_hotplug` | 225-250 | UNREACHABLE | S2 | documented catch-all arms; no host cadence exists |
| `Engine::observe_audio_reopen` | 264-295 | UNREACHABLE | S2 | device-changed-then-Play ordering structurally sound |
| `Engine::observe_renderer_backend` | 311-317 | UNREACHABLE | S2 | renderer-wgpu documents but never calls |
| `Engine::hotplug_stats` | 322-324 | UNREACHABLE | S2 | no shipped caller either |
| `Engine::apply_hotplug_input` | 332-343 | REAL (via unreach callers) | S4 | silent rejection (F-002) |
| `Engine::push_session_event` | 348-352 | REAL | — | no-session gate |
| `Engine::relay_suspend_notice` | 362-386 | REAL | — | pattern-matches the guard's wire event to rebuild the engine-side payload — a shape-coupled translation; if recovery.rs changes the event shape this silently stops firing (no test pins the guard→engine join here; the contract suite allegedly does — UNVERIFIED) |
| `Engine::replay_backend_report_at_open` | 390-394 | REAL (dead in prod) | S2 | called by session.rs:1092, but nothing ever populates `backend_report` in production (only `observe_renderer_backend` does, which has no caller) |

## 5. Test honesty

No in-file tests. Integration tests in crates/playback-engine/tests/hotplug.rs drive the hooks directly (not through any shipped host path) — they pin the engine-side behavior honestly but say nothing about shipped reachability. That suite belongs to another agent's file list; not audited here beyond the caller enumeration.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the engine-side executor half of the environment-recovery policies" (card 7.C1) | hotplug.rs:7-12 | REAL-UNPROVEN | code exists and applies real table rows; runtime behavior not executed (no cargo) |
| "hosts call [`Engine::poll_hotplug`] on their own cadence" | hotplug.rs:63-64 | FALSE for the shipped build | rg: no host calls any hook (F-001) |
| doc 05 §3.2 `audio.device-lost`/`-changed`, `video.backend-changed` events | hotplug.rs:2-3 | UNREACHABLE | no production caller ⇒ the wire events cannot occur |
| "The report is cached … replayed onto the first session's stream at open" | hotplug.rs:307-310 | REAL mechanism / DEAD in prod | replay_backend_report_at_open runs at open (session.rs:1092) but the cache is only populated by the caller-less observe_renderer_backend |

## 7. Dead / duplicated / unreachable code

The whole module, relative to shipped entry points (F-001). Within the crate it is exercised by tests only.

## 8. Dependencies & flags

- No feature gates. Compiles into playback-engine unconditionally (W00: release PASS) — the dead code ships in every binary.

## 9. Open questions for the next agent

- Does feature-matrix.toml claim device-hotplug rows as implemented with evidence pointing at this file? (waves 18/19 — if yes, those rows are UNREACHABLE, not REAL.)
- Do the platform-* `DeviceEventQueue` implementations have any consumer at all in apps/? If not, the entire observation seam is also dead (files: crates/platform-{linux,macos,windows}/src/*).
- tests/hotplug.rs quality: does its "contract suite" pin the guard→engine payload join that `relay_suspend_notice` shape-matches (362-386)? If recovery.rs's event shape drifts, the notice silently vanishes.
