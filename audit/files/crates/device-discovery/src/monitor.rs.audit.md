# AUDIT: crates/device-discovery/src/monitor.rs
| Field | Value |
|---|---|
| Auditor | W10-A08 |
| Date | 2026-11-21 |
| Lines / functions | 436 / 12 production items (MonitorConfig: new, with_limits, Default; VolumeMonitorHandle: events, errors, passes, is_running, stop, Drop; VolumeMonitor::spawn; MonitorWorker: run, one_pass; poll_once) + 2 test helpers + 6 tests |
| File verdict | UNREACHABLE (real, tested library code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO. Import census: `device_discovery` is imported only by its own tests (`crates/device-discovery/tests/{ssdp,fallback_paths,volume_monitor}.rs`); no app imports it. The playback engine documents the seam as future work (crates/playback-engine/src/events.rs:106 "`devices.volume-added` | wired-by-later 7.D1"); the CLI's volume lanes honestly refuse (baseline `probe_reports_the_volume_lanes_as_not_yet_implemented`, fallback_paths.rs, orchrun log :2407). Re-exported at crates/device-discovery/src/lib.rs:65. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 0 |
| Commands run | Read tool full file; `grep -n` fn pins; `sed -n` verbatim verification of every quote; `od` byte-check of `#[must_use]` line 112 (display-pipeline artifact ruled out); `rg "VolumeMonitor\|poll_once\|VolumeSource"` repo-wide (caller census); `rg -ln "use device_discovery"` (import census); baseline: `orchrun-test-workspace-nofailfast.log.txt` :2392-2396, :2411 (6/6 unit ok), :2426-2437 (volume_monitor.rs integration 3/3 ok); all-features log :2379-2385 same-green |

## 1. One-paragraph verdict
The volume watcher is a small, disciplined poll-diff engine: one bounded thread, capacity-clamped drop-oldest event queue, sliced sleeps for responsive shutdown, and an honesty rule (enumerate failure ⇒ counter bump, never fabricated removals) that its tests genuinely pin against a real spawned thread. The pure half (`poll_once`) and the threaded half share the same diff computation, and all nine tests (6 unit + 3 integration) ran green in the baseline. Three real defects, all about promises the code does not keep: the `poll_interval` field doc promises a 100 ms floor clamp that only the `with_limits` constructor performs — a struct-literal config with `Duration::ZERO` busy-loops the watcher thread at 100% CPU; the spawn-failure path swallows the `io::Error` and then reports the dead watcher as `is_running() == true` (its own comment claims the opposite); and the "50 ms max" shutdown-latency bound is false whenever `enumerate` blocks — the classic stuck-mount case for a volume watcher — because `Drop`→`stop`→`join` has no timeout. None of this is reachable from a shipped binary: the crate is tests-only until the 7.D1 host wiring lands.

## 2. What actually works (verified)
— All 6 unit tests + 3 integration tests executed ok: baseline `orchrun-test-workspace-nofailfast.log.txt:2392-2396, :2411` and `:2426-2437` (`stop_is_idempotent_and_drop_safe`, `watcher_reports_initial_mounts_then_the_scripted_removal`, `concurrent_drain_while_the_watcher_pushes_is_safe`).
— Read-traced the honesty rule end to end: `one_pass` (222-244) bumps `passes` first, maps enumerate failure to `errors += 1` + return (never touching `last`, so no removal events can be fabricated — `failed_passes_count_errors_and_never_emit_removals` pins this against a real thread), and treats over-`max_devices` snapshots as failed passes (234-239) rather than truncating.
— First-pass Added semantics verified: `last` starts empty (157), so the first successful pass diffs against nothing and reports the mounted set as Added — real enumeration data, matching the module docs (:17-22).
— `run`'s sliced sleep (199-218): shutdown checked before each 50 ms slice, so latency is bounded by slice+pass-time when the pass returns; `stop` is idempotent via `Option::take` (119-127); Drop joins (130-134) — no watcher leaks (integration-tested).
— `poll_once` (256-271) is the same computation minus the loop, with the over-bound case surfaced as typed `CapacityExceeded` instead of a counter — the asymmetry is coherent (the thread has no caller to return an error to).

## 3. Findings

### F-001 [S3]
Where: crates/device-discovery/src/monitor.rs:40-43 (field doc) with 65-70, 148-149, 199-218
Code:
```
    /// Cadence between enumerate passes. Clamped to at least
    /// [`MonitorConfig::MIN_POLL_INTERVAL`] (a tighter spin is a busy
    /// loop, not responsiveness).
    pub poll_interval: Duration,
```
What is wrong: The clamp exists only in `MonitorConfig::with_limits` (:67 `poll_interval.max(MIN_POLL_INTERVAL)`); the fields are `pub`, so `MonitorConfig { poll_interval: Duration::ZERO, queue_capacity: 128 }` compiles and reaches `VolumeMonitor::spawn` unclamped. `run` then executes `one_pass()` back-to-back forever — `while remaining > Duration::ZERO` never sleeps — a 100% CPU busy loop on a core, and the enumerate source is hammered at full speed. Neither `spawn` (148) nor `run` (199) re-clamps. (The sibling `queue_capacity: 0` IS defended downstream — `VolumeEventQueue::with_capacity` clamps zero per its own test `zero_capacity_is_clamped_not_a_black_hole` — only `poll_interval` is undefended.)
Why it matters: The field doc promises a safety property the type does not enforce; a future host wiring this (the 7.D1 seam) that constructs the config as a struct literal — the natural Rust move — gets a spin loop. The doc's own words name the consequence ("a tighter spin is a busy loop") while the code permits exactly that.
What correct looks like: Clamp in `spawn` (`let poll_interval = config.poll_interval.max(MonitorConfig::MIN_POLL_INTERVAL);`) or in `run`, and make `MonitorConfig`'s fields private behind the two constructors.
Confidence: HIGH — Verification: read (no clamp site outside `with_limits`; direct-construction path traced through `spawn` → `run`).

### F-002 [S3]
Where: crates/device-discovery/src/monitor.rs:164-174 (with 113-115, 150)
Code:
```
            .spawn(move || worker.run(config.poll_interval))
            // A thread-spawn failure is an environment-level refusal
            // (rlimits); the honest handle reports a dead watcher
            // (is_running false, zero passes) rather than crashing the
            // host.
            .ok();
```
What is wrong: On spawn failure the `io::Error` is discarded by `.ok()` and nothing marks the handle dead: `shutdown` was initialized `false` (:150) and is never set, so `is_running()` (:113-115, `!shutdown`) returns **true** for a watcher with no thread. The comment's promised behavior ("is_running false") is the opposite of what the code produces. `errors()` also stays 0 — that counter counts enumerate failures only — so a spawn-failed watcher presents as a healthy running watcher that simply never produces passes, events, or errors.
Why it matters: The one failure mode this path exists to handle (environment-level spawn refusal) is reported as success by every observable accessor; a host would wait forever on a queue that can never fill, with no signal. The comment-vs-code contradiction guarantees the next maintainer believes the honest behavior exists.
What correct looks like: On `.ok()` → `None`, set `shutdown` to true (making `is_running()` false), or add an explicit `spawned: bool`/`Option<JoinHandle>`-derived accessor; log or expose the `io::Error`.
Confidence: HIGH — Verification: read (state initialization and accessor traced; no test covers the spawn-failure path — confirmed against the 6-test list).

### F-003 [S3]
Where: crates/device-discovery/src/monitor.rs:79-81 (doc) with 119-127, 222-224
Code:
```
/// A running watcher. Dropping it signals shutdown and JOINS the
/// thread (bounded shutdown latency: one poll-interval slice, 50 ms
/// max — see the loop).
```
What is wrong: The 50 ms bound holds only between passes. `Drop` → `stop` → `join.join()` blocks until the worker thread returns, and the worker can be inside `self.source.enumerate(&self.ctx)` (:224) for unbounded time — a stuck mount table read (dead NFS/USB export, the canonical volume-watcher hazard) freezes the join with no timeout. The sliced sleep (202-216) only bounds the *sleep* portion of the cadence.
Why it matters: Dropping a handle runs on the dropper's thread — in the future host wiring that is an engine/UI teardown path; a blocked enumerate turns it into an indefinite hang, while the type documents a hard 50 ms bound. `stop()`'s join-error comment (:122-125) shows the author considered thread-death states but not thread-*stuck* states.
What correct looks like: Either document the true bound ("50 ms after the current pass returns; a blocked enumerate holds the join"), or give the worker a deadline-aware source contract / detach-with-flag shutdown.
Confidence: MEDIUM — Verification: read (no timeout mechanism anywhere in the file; `VolumeSource::enumerate` is a blocking trait method by construction); stuck-mount scenario reasoned from the trait shape, not executed.
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MonitorConfig::new / Default | 56-61, 73-77 | REAL | — | 2 s cadence, 128-event queue; pinned by config_clamps_to_floors. |
| MonitorConfig::with_limits | 65-70 | REAL | — | Clamps interval ≥ 100 ms, capacity ≥ 1. The only clamp site (F-001). |
| VolumeMonitorHandle::events | 93-95 | REAL | — | Accessor to the bounded queue. |
| VolumeMonitorHandle::errors / passes | 101-109 | REAL | — | Relaxed atomic loads; monotonic counters. Fine for telemetry. |
| VolumeMonitorHandle::is_running | 113-115 | REAL | S3 (F-002) | `!shutdown` conflates "not stopped" with "thread alive" — lies after spawn failure. |
| VolumeMonitorHandle::stop | 119-127 | REAL | S3 (F-003) | Idempotent via take; join can block unboundedly inside enumerate. |
| Drop for VolumeMonitorHandle | 130-134 | REAL | — | Delegates to stop; no leak (integration-tested). |
| VolumeMonitor::spawn | 148-183 | REAL | S3 (F-002) | Builder-spawn; `.ok()` swallow + comment contradiction. Thread named for diagnostics. |
| MonitorWorker::run | 199-218 | REAL | S3 (F-001) | Sliced-sleep loop; no interval re-clamp (F-001); shutdown checked per slice. |
| MonitorWorker::one_pass | 222-244 | REAL | — | Passes-first accounting; failure ⇒ counter, `last` untouched (no fake removals); over-max ⇒ loud failure. Correct on every branch traced. |
| poll_once | 256-271 | REAL | — | Pure loop-free API; typed error propagation; CapacityExceeded loud. Executed 3/3 tests. |
| tests: volume, Scripted, 6 tests | 278-435 | REAL | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| config_clamps_to_floors (304) | YES | Direct clamp assertions on with_limits + new. |
| poll_once_diffs_and_returns_the_snapshot (312) | YES | Event counts + Removed/Added ordering asserted on the scripted diff. |
| poll_once_propagates_the_typed_failure (333) | YES | Typed variant matched, unreachable! on success. |
| poll_once_enforces_the_device_bound (347) | YES | Exact limit/attempted destructured (256/300). |
| spawn_first_pass_reports_mounted_volumes_as_added (368) | YES | Real thread, 2 s deadline drain; asserts 2 Added + passes ≥ 1 + errors == 0 + `!is_running` after stop — fails if the worker never runs, misclassifies events, or stop doesn't join. Timing-based (10 ms poll, 2 s budget — generous). |
| failed_passes_count_errors_and_never_emit_removals (403) | YES | The honesty rule pinned against a real thread: errors ≥ 1, passes ≥ 2, all events Added. |
| Integration: stop_is_idempotent_and_drop_safe / watcher_reports_initial_mounts_then_the_scripted_removal / concurrent_drain_while_the_watcher_pushes_is_safe (tests/volume_monitor.rs) | YES (per baseline names) | Ran ok (:2432-2437); drop-safety, scripted-removal diff, and concurrent drain race covered — the last is the one real concurrency pin. |
| Untested paths | — | Spawn failure (F-002), zero-interval construction (F-001), blocked enumerate (F-003) — no coverage for any of the three defects. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Clamped to at least MIN_POLL_INTERVAL" (field doc) | monitor.rs:40-43 | FALSE for direct construction | F-001: clamp only in with_limits; pub fields bypass. |
| "the honest handle reports a dead watcher (is_running false, zero passes)" | monitor.rs:170-173 | FALSE | F-002: is_running() == true after spawn failure. |
| "bounded shutdown latency: one poll-interval slice, 50 ms max" | monitor.rs:79-81 | FALSE under blocked enumerate | F-003. |
| "An enumerate failure pushes NOTHING… the error counter moves" | monitor.rs:20-22 | REAL | :226-232; pinned by failed_passes test. |
| "exactly one watcher thread per handle, join on drop, no spawn storms" | monitor.rs:10-12 | REAL | spawn→one thread; Drop→stop→join (:130-134); integration `stop_is_idempotent_and_drop_safe` ok. |
| "`devices.volume-*` wired-by-later 7.D1" | playback-engine/src/events.rs:106 | REAL (honest gap) | No app imports the crate; fallback test reports volume lanes not-yet-implemented (baseline :2407). |

## 7. Dead / duplicated / unreachable code
- Whole module unreachable from shipped binaries (see header) — library + tests until the 7.D1 host wiring lands.
- No dead items; `poll_once` is the loop-free twin of `one_pass` by design (documented wasm-host rationale, :247-254), not duplication.

## 8. Dependencies & flags
std-only (thread, sync, time); crate-internal `DiscoveryContext`, `DiscoveryError`, `events::{VolumeEvent, VolumeEventQueue, diff_volumes}`, `volume::{Volume, VolumeSource}`. No feature gates, no unsafe. Relaxed atomics justified (counters + flag; `join` provides the final synchronization). `#[must_use]` annotations present (od-verified bytes).

## 9. Open questions for the next agent
- W10-A07 (device.rs) / events.rs owner: does `VolumeSource` document a blocking-or-timeout contract for `enumerate`? F-003's blast radius depends on the platform integrations' enumerate behavior (platform-linux/windows/macos crates).
- Whoever wires 7.D1 into a host: F-001's struct-literal hazard becomes live the moment a host constructs `MonitorConfig` by field — re-check then; the three doc contradictions here should be fixed before that wiring, not after.
