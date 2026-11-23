# AUDIT: crates/playback-engine/src/hotplug.rs
| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-23 |
| Lines / functions | 395 / 14 (4 pub hooks + 5 helpers + types) |
| File verdict | UNREACHABLE (real, tested engine code — but zero shipped-host callers; only playback-engine's own tests call the hooks) |
| Reachable from shipped entry point? | NO — repo-wide rg for `observe_audio_device_lost(|poll_hotplug(|observe_audio_reopen(|observe_renderer_backend(` matches ONLY crates/playback-engine/src/hotplug.rs (definitions) and crates/playback-engine/tests/hotplug.rs. renderer-wgpu/src/negotiate.rs mentions `observe_renderer_backend` in doc comments only (lines 32/244/305 — no call). apps/cli, apps/desktop, apps/web: no calls. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `cargo test -p playback-engine --test hotplug` → `8 passed; 0 failed` (0.00s); repo-wide caller searches (rg, cited above) |

## 1. One-paragraph verdict
The engine-side hotplug executor is real code with a real design: typed host hooks (`observe_audio_device_lost` → emit fact + apply the `Pause` row in-pipeline; `poll_hotplug` → drive the reopen window on caller-supplied virtual time; `observe_audio_reopen` → emit fact + apply `Play`; `observe_renderer_backend` → relay + cache for replay-at-open), all through the same single-writer `Session::apply` path as submitted commands, all deterministic (no clock reads, no sleeps), all rejection-tolerant. Its own test suite passes (8/8, executed). But the feature is dead in the shipped product: no host anywhere in apps/ calls any of the four hooks, so `audio.device-lost`, `audio.device-changed`, the suspension notice, and `video.backend-changed` can never emit in the release lane — doc 05 §3.2 (line 392–394) lists the device events as wire rows, and phase7-cards.md card 7.C1 ("Device hotplug, output fallback & discovery") is only half-landed: the engine half exists, the host half does not. With the CLI's null audio device (which cannot be lost) there is no user-visible defect in the sandbox lane, hence S3 not S2 — but any feature-matrix row claiming device hotplug as shipped is FALSE.

## 2. What actually works (verified)
- `cargo test -p playback-engine --test hotplug` → `test result: ok. 8 passed; 0 failed; 0 ignored` (executed by me; includes e.g. `session_scoped_hotplug_events_survive_intra_epoch_supersession`).
- The single-writer discipline holds: `apply_hotplug_input` (341–355) routes through `session.apply` — the same table path inbox commands travel (cross-checked in the session.rs audit); `push_session_event` (358–363) uses `Session::emit_hook`, the crate-internal stamped-emission path.
- The replay-at-open wiring is real: `Engine::open` calls `replay_backend_report_at_open` (session.rs:1094–1096 region, verified in the session.rs audit), so a pre-session backend report would land on the first session's stream — IF any host ever produced one.

## 3. Findings
F-001 [S3 — UNREACHABLE]
Where: crates/playback-engine/src/hotplug.rs:182–212, 225–249, 264–305, 311–330 (the four pub hooks)
Code:
```
    pub fn observe_audio_device_lost(&mut self, now_ns: u64, device_id: &str) -> HotplugStep {
```
What is wrong: nothing inside the function — the defect is that it (and `poll_hotplug`, `observe_audio_reopen`, `observe_renderer_backend`) has no shipped caller. Verified: `rg 'observe_audio_device_lost\(|poll_hotplug\(|observe_audio_reopen\(|observe_renderer_backend\('` across the repo matches only the defining file and `tests/hotplug.rs`. renderer-wgpu's negotiate.rs references the hook in comments only. Why it matters: doc 05 §3.2 lists `audio.device-lost`/`audio.device-changed` as wire events (docs/05-command-layer.md:392–394) and card 7.C1 is "Device hotplug, output fallback & discovery" — the engine half landed, the host half (OS seam → hook) never did, so the shipped product cannot produce these events and mid-playback device loss does nothing engine-side. What correct looks like: the CLI/desktop host wiring `SinkEvent::DeviceLost` → `observe_audio_device_lost` and a `poll_hotplug` cadence, or an honest "engine API only, host wiring pending" note in the phase-7 record. Confidence: HIGH — Verification: read + repo-wide rg (executed). Downstream: flag to matrix auditors — any row citing 7.C1/device hotplug as shipped evidence must cite these hooks' absence of callers.
F-002 [S4]
Where: crates/playback-engine/src/hotplug.rs:183–187
Code:
```
        let position_ns = self
            .session
            .as_ref()
            .map(|session| clock_position_ns(&session.runtime.clock))
            .unwrap_or(0);
```
What is wrong: a device loss with no live session records suspend position 0 — a fabricated "loss position" that surfaces in `HotplugStats::suspended_position_ns`. Harmless in effect (the resume uses the clock's own frozen value, not the guard's number; the guard's value is observability only), but the stat can lie. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/playback-engine/src/hotplug.rs:33–40 (module doc) vs 209–212 (code)
Code (doc):
```
//! Auto-resume against the user's explicit
//! pause: a loss that arrives while already paused records the loss
//! but applies nothing (the `Pause` row from `Paused` is a
```
Code (impl):
```
        if let RecoveryAction::SuspendClock { .. } = step.action {
            self.apply_hotplug_input(SessionInput::Pause);
        }
```
What is wrong: "applies nothing" contradicts the code — whenever the guard answers `SuspendClock`, the engine applies the Pause row even from `Paused` (a C8 self-transition that emits a `state-changed → paused` confirmation). The parenthetical in the same doc sentence admits the row runs, so the doc contradicts itself; the observable effect is one extra confirmation event, not "nothing". UNVERIFIED half: whether the guard (recovery.rs — not my file) ever answers SuspendClock while already suspended. Confidence: MEDIUM — Verification: read (engine side), UNVERIFIED (guard side).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `HotplugStep::is_action` | 89–91 | REAL | — | trivial |
| `HotplugStats` (struct) | 99–123 | REAL | — | observability snapshot; `renderer_backend` clones Strings (event-rate, fine) |
| `HotplugState::default` | 137–143 | REAL | — | guard + RetryWindow::DEFAULT |
| `HotplugState::stats` | 145–157 | REAL | — | |
| `Engine::observe_audio_device_lost` | 182–212 | REAL, UNREACHABLE | S3 | fact-then-suspend ordering; Pause row via table; rejection consumed; F-002 |
| `Engine::poll_hotplug` | 225–249 | REAL, UNREACHABLE | S3 | ReopenDevice handoff; notice relay exactly-once via guard latch; honest catch-all for the other guards' actions |
| `Engine::observe_audio_reopen` | 264–305 | REAL, UNREACHABLE | S3 | ResumeClock → device-changed then Play; failure → backoff/notice; `active_device().unwrap_or_default()` empty-string fallback |
| `Engine::observe_renderer_backend` | 311–330 | REAL, UNREACHABLE | S3 | cache + relay; no-session → cached for open replay |
| `Engine::hotplug_stats` | 333–336 | REAL | — | |
| `Engine::apply_hotplug_input` | 341–355 | REAL | — | same single-writer apply path as commands; no-session early return |
| `Engine::push_session_event` | 358–363 | REAL | — | no-session → dropped (documented gate) |
| `Engine::relay_suspend_notice` | 366–389 | REAL | — | pattern-gated on the guard's non-fatal AudioOutput Device PlaybackError; single-shot via guard latch |
| `Engine::replay_backend_report_at_open` | 391–395 | REAL | — | called from Engine::open (verified in session.rs audit) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| tests/hotplug.rs (8 tests, not my file) | YES (for the module in isolation) | executed 8/8 pass; names like `session_scoped_hotplug_events_survive_intra_epoch_supersession` assert event/staleness semantics |
| — but none can catch F-001 | NO | the tests ARE the only callers; no test asserts a shipped host wires the hooks (nothing could, from inside the crate) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the typed entry points a host calls when the world around the media changes" | hotplug.rs:7–9 | FALSE in the shipped product | repo-wide rg: no host calls them (F-001) |
| doc 05 §3.2 lists `audio.device-lost` / `audio.device-changed` as wire events | docs/05-command-layer.md:392–394 | PAPER-ONLY in the shipped lane | the only emitters are the uncalled hooks |
| card 7.C1 "Device hotplug, output fallback & discovery" | docs/plans/phase7-cards.md:84 | HALF-LANDED | engine half real+tested; host half absent |
| "hosts call `Engine::poll_hotplug` on their own cadence" | hotplug.rs:59–61 | FALSE in the shipped product | no host calls it |
| "a loss that arrives while already paused … applies nothing" | hotplug.rs:38–40 | FALSE (self-contradicting sentence) | F-003 |

## 7. Dead / duplicated / unreachable code
- The entire module is unreachable from shipped entry points (F-001) — the headline fact of this audit.
- No internal dead code found; every helper has a caller within the module.

## 8. Dependencies & flags
- Depends on crate::recovery (AudioDeviceGuard/RecoveryAction/RetryWindow — NOT in my assignment; the guard's semantics are trusted UNVERIFIED here), crate::session/event/state, media-types, commands (EventKind in relay gating). No feature gates; always compiled into the shipped binary as dead weight.

## 9. Open questions for the next agent
- crates/playback-engine/src/recovery.rs owner: does the guard answer `SuspendClock` on a second loss while suspended (F-003's UNVERIFIED half), and is `RetryWindow::DEFAULT`'s budget sane?
- Matrix auditors: check whether any of the 252 rows cites device hotplug (7.C1) as shipped — if so, that row is FALSE per F-001.
- Whoever owns apps/desktop: is device-loss wiring planned there (the natural first host)?
