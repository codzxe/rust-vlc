# AUDIT: crates/playback-engine/src/hotplug.rs
| Field | Value |
|---|---|
| Auditor | W03-A10 |
| Date | 2026-11-21 |
| Lines / functions | 395 / 14 items |
| File verdict | UNREACHABLE (all four host-facing hooks are test-driven only; the one production-reachable internal is a guaranteed no-op) |
| Reachable from shipped entry point? | NO for the hooks: `observe_audio_device_lost`/`poll_hotplug`/`observe_audio_reopen`/`observe_renderer_backend` are invoked ONLY by `crates/playback-engine/tests/hotplug.rs` (rg over tree). `replay_backend_report_at_open` IS called by `Engine::open` (session.rs:1092, shipped `lumen open` path) but always no-ops in production because nothing ever calls `observe_renderer_backend` (renderer-wgpu/src/negotiate.rs:30-34 references the call in DOC COMMENTS only — the relay is not wired). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg "\.observe_renderer_backend\("` → tests only; `rg "observe_renderer_backend\|replay_backend_report_at_open\|RendererBackendReport" -g '!target'` → event.rs conversions + session.rs:1092 + negotiate.rs docs; `sed -n '1085,1093p' session.rs` → replay call site; `sed -n` on all quoted ranges |

## 1. One-paragraph verdict
The engine-side hotplug executor is coherent, well-documented, ordering-correct (fact-before-reaction traced at both emission sites), and honestly gated on session liveness — but it is a policy shell with no product driver: every host-facing hook is called exclusively by the test suite, and the single production-reachable internal (`replay_backend_report_at_open`, wired into `Engine::open`) can never see a cached report because the renderer's negotiate module documents the relay call instead of making it. The consequence cascades to the events catalog: `VideoBackendChanged` is marked "Emitted" yet cannot fire in any shipped binary. One fabricated-zero bookkeeping wart (position 0 on a session-less loss) rounds it out.

## 2. What actually works (verified)
- Event ordering: `AudioDeviceLost` is pushed at 196-198 BEFORE `apply_hotplug_input(SessionInput::Pause)` at 208-210; `AudioDeviceChanged` at 281 BEFORE `SessionInput::Play` at 282 — the doc 05 §3.3 fact-precedes-reaction rule holds at both sites (traced).
- The one-bridge rule: the guard's wire-form events are never re-bridged; the engine emits once, session-stamped (193-195, 272-274). The `relay_suspend_notice` match (363-372) recognizes exactly the guard's non-fatal AudioOutput/Device notice shape and re-derives structured facts from guard state — single source of truth.
- Session-liveness gates: `push_session_event` (348-352) drops rather than panics with no session; `apply_hotplug_input` (332-343) consumes typed rejections (`if let Ok`) per the documented no-requirement doctrine.
- Exactly-once suspension notice: the guard's `notified` latch (recovery.rs, my part2 audit) is the single-shot rule; both surfacing paths (poll expiry 229-235, budget exhaustion 285-292) route through the same relay.
- Tests: tests/hotplug.rs drives the real `Engine` through loss→poll→reopen cycles in virtual time and pins payloads/ordering/counters (call sites 256-301, 384-399, 579, 616); green in `orchrun-test-workspace-nofailfast.log.txt` (playback-engine not among the 7 failures).

## 3. Findings

### F-001 [S3]
Where: crates/playback-engine/src/hotplug.rs:311-317 (and the module's four hooks)
Code:
```rust
    pub fn observe_renderer_backend(&mut self, backend: &str, reason: &str) {
        self.hotplug.backend_report = Some((backend.to_string(), reason.to_string()));
        self.push_session_event(EngineEventKind::RendererBackendReport {
            backend: backend.to_string(),
            reason: reason.to_string(),
        });
    }
```
What is wrong: no production code calls any hotplug hook. `observe_renderer_backend` is invoked only by tests (hotplug.rs's own test file at 579, 616). renderer-wgpu's negotiate.rs — the module this hook was built for — describes the relay in a doc comment (negotiate.rs:30-34: "a host relays the report through `playback_engine::Engine::observe_renderer_backend`") but never performs it. Therefore `replay_backend_report_at_open` (390-394), though genuinely wired into the shipped `Engine::open` (session.rs:1092), always sees `backend_report: None`.
Why it matters: the events catalog marks `VideoBackendChanged` "Emitted" (events.rs:724-727) and its doc cites "the initial auto-negotiation report" replay — in the shipped `lumen` binary that event can never occur; the wgpu→software fallback (a card 7.C1 headline) is invisible on the wire. Same product-vs-library split as the audio hooks (recovery.rs part2 F-001).
What correct looks like: apps/cli's pipeline_host (which imports renderer_wgpu, pipeline_host.rs:79-82) calls `negotiate()` and relays `BackendReport::relay_pair()` into `observe_renderer_backend` at startup; until then the catalog row should read like the WiredLater rows.
Confidence: HIGH — Verification: rg for the call across the tree; negotiate.rs and pipeline_host.rs import lists read; session.rs:1085-1093 read.

### F-002 [S4]
Where: crates/playback-engine/src/hotplug.rs:183-187
Code:
```rust
        let position_ns = self
            .session
            .as_ref()
            .map(|session| clock_position_ns(&session.runtime.clock))
            .unwrap_or(0);
```
What is wrong: a device loss with no live session records suspend position `0` — a fabricated position, contra the project's own taxonomy doctrine ("absence is data, never `0`", MP §8, quoted in mapping.rs:15 and recovery-adjacent docs). The guard's `suspended_position_ns()` then reports `Some(0)` (a real-looking position) instead of `None`.
Why it matters: observability only — the resume path does not use the remembered value (the `Play` row resumes the clock from its own frozen state; the `ResumeClock { position_ns }` payload is ignored by this module), so no playback corruption. But `HotplugStats.suspended_position_ns` lies about a session-less episode.
What correct looks like: make the guard's remembered position `Option<i64>` (or keep the clock-position map but expose `None` when the loss was session-less).
Confidence: HIGH — Verification: read; ResumeClock payload non-use traced (271-283 uses only the action discriminant).
Note: the module doc (71-74) already flags the paused-loss corner honestly; this is the session-less sibling it missed.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `HotplugStep` (enum) + `is_action` | 91-108 | REAL | — | Two-variant host directive; trivial correct |
| `HotplugStats` (struct) | 112-129 | REAL | — | Snapshot; `renderer_backend` clone per call — event-rate, fine |
| `HotplugState` (struct) + `Default` | 133-151 | REAL | — | `RetryWindow::DEFAULT` policy (part1 constant) |
| `HotplugState::stats` | 155-164 | REAL | — | Pure projection |
| `Engine::observe_audio_device_lost` | 182-214 | REAL | S4 | F-002 (fabricated 0); ordering verified; no-session drop documented |
| `Engine::poll_hotplug` | 225-250 | REAL | — | ReopenDevice passthrough; expiry relay; catch-all consumed-not-panicked (244-248) — documented honest-unreachable |
| `Engine::observe_audio_reopen` | 264-295 | REAL | — | device-changed-before-playing verified; failure path relays notice |
| `Engine::observe_renderer_backend` | 311-317 | REAL | S3 | F-001 (test-only caller; renderer relay unwired) |
| `Engine::hotplug_stats` | 322-324 | REAL | — | Delegates |
| `Engine::apply_hotplug_input` | 332-343 | REAL | — | Typed rejections consumed per doctrine; outputs pushed to outbox |
| `Engine::push_session_event` | 348-352 | REAL | — | Session-liveness gate; no panic |
| `Engine::relay_suspend_notice` | 362-386 | REAL | — | Guard-notice shape match; structured re-derivation; exactly-once via guard latch |
| `Engine::replay_backend_report_at_open` | 390-394 | REAL | S3 | Reachable via Engine::open (session.rs:1092) but structurally a no-op in production (F-001) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| tests/hotplug.rs loss→suspend ordering tests (256-301) | YES | Real Engine, virtual time; asserts the device-lost event precedes the paused state-change and the position is preserved |
| tests/hotplug.rs backoff/expiry tests (384-399) | YES | Poll/result interleavings assert reopen counts and the one-time notice |
| tests/hotplug.rs backend report tests (579, 616) | YES | Cache + replay-at-open pinned with a real open |
| events_contract.rs RendererBackendReport conversion (934) | YES | Engine-private kind → wire VideoBackendChanged pinned |
| Baseline: green — playback-engine absent from `orchrun-test-workspace-nofailfast.log.txt`'s 7 failures | — | cited |
| No test covers: a session-less loss's stats position (F-002) — the fabricated 0 is unasserted | — | gap |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "a host relays the report through `Engine::observe_renderer_backend`" | renderer-wgpu/negotiate.rs:30-34 | PAPER-ONLY | No host does; call sites are tests exclusively (F-001) |
| VideoBackendChanged "Emitted" | events.rs catalog 724-727 | REAL for the library / UNREACHABLE for the product | Only producer is the unwired hook (F-001) |
| "audio.device-lost lands before the suspend's state-changed → paused" | module doc 50-52 | REAL | Traced at 196-210 |
| "the initial auto-negotiation report" replay at open | events.rs:724-727 context + hotplug.rs 136-141 | SHALLOW | Replay code real (390-394) and reachable, but its input never fires in production |
| "Auto-resume against the user's explicit pause … records the loss but applies nothing" | module doc 71-74 | REAL | Pause row from Paused is a self-transition; typed rejection consumed (332-343) |

## 7. Dead / duplicated / unreachable code
- All four pub hooks: unreachable from shipped entry points (F-001) — the module is otherwise internally live (session.rs holds `HotplugState`, Engine::open calls the replay).
- No duplication found.

## 8. Dependencies & flags
- No feature gates, no unsafe; allocations only at event rate (module doc 76-82 claim verified by reading: per-call Vec from apply + event strings).
- Depends on recovery.rs guards (my part2 audit), session.rs `emit_hook`/`apply`/`clock_position_ns`, commands EventKind vocabulary.

## 9. Open questions for the next agent
- Q-1 (apps/cli or renderer-wgpu owner): wire `negotiate()`'s `BackendReport` into `observe_renderer_backend` — it is a two-line relay the negotiate module already documents.
- Q-2 (events catalog owner): VideoBackendChanged and AudioDeviceLost/Changed should carry an honest "engine API emitted; host relay pending" qualifier until Q-1 lands.
- Q-3 (session.rs owner): does `emit_hook` stamp seq/envelope identically to command-path emissions? Assumed yes (tests pin it); verify when auditing session.rs.
