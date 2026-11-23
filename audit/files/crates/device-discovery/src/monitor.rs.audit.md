# AUDIT: crates/device-discovery/src/monitor.rs
| Field | Value |
|---|---|
| Auditor | W10-A05 |
| Date | 2026-11-23 |
| Lines / functions | 436 / 12 production items (MonitorConfig×3, Handle×5+Drop, VolumeMonitor::spawn, MonitorWorker×2, poll_once) + 7 test items |
| File verdict | REAL (genuine thread, real diff events, honest failure accounting) — but the CRATE is UNREACHABLE |
| Reachable from shipped entry point? | NO — zero callers of `VolumeMonitor`/`poll_once` outside the crate (proof below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n 'VolumeMonitor|poll_once' --glob '*.rs' apps crates tests fuzz` → only self + lib.rs re-export + a volume.rs doc mention; `rg 'device-discovery' --glob 'Cargo.toml'` → no dependents; baseline citation |

## 1. One-paragraph verdict
The watcher is real: one named bounded thread, poll-diff against the previous snapshot, first pass diffed against empty (initial Added events from real enumeration — no fabricated seed), failed passes counted and NEVER translated into removal events, capacity-clamped drop-oldest queue, sliced sleep for ≤50 ms shutdown latency, join-on-drop. Tests use a scripted `VolumeSource` (the sanctioned in-process seam for enumeration — correct layer to fake) plus real threads and real timing; all 6 in-module tests plus 3 e2e tests in tests/volume_monitor.rs pass in the baseline. Two defects: (1) the spawn-failure comment promises `is_running` false, but a failed `thread::Builder::spawn` leaves `shutdown=false`, so `is_running()` returns TRUE for a dead watcher — the comment is false and the accessor lies; (2) like `transcode`, the whole `device-discovery` crate has zero dependents — no app, no platform adapter (lib.rs itself admits the platform adapters have not landed and `Discovery::probe` wires an all-`Unavailable` fallback), so no shipped entry point ever spawns this monitor.

## 2. What actually works (verified)
— Baseline audit/runs/baseline/cargo-test-workspace.txt (PACKAGE device-discovery): `test monitor::tests::*` all 6 ok (38 passed lib total), `tests/volume_monitor.rs` 3 passed in 3.00s (real-thread timing tests). Shutdown-join verified by test `spawn_first_pass_reports_mounted_volumes_as_added` asserting `!handle.is_running()` after stop().

## 3. Findings
F-001 [S3]
Where: crates/device-discovery/src/monitor.rs:164-182
Code:
```
            .spawn(move || worker.run(config.poll_interval))
            // A thread-spawn failure is an environment-level refusal
            // (rlimits); the honest handle reports a dead watcher
            // (is_running false, zero passes) rather than crashing the
            // host.
            .ok();
```
What is wrong: on spawn failure the handle is built with `shutdown = false` and `join = None`; `is_running()` is `!self.shutdown.load(Ordering::Relaxed)` (L113-115), so it returns TRUE for a watcher that does not exist. The comment's promised honesty ("is_running false") is not implemented — nothing sets the shutdown flag when spawn fails. Why it matters: the one diagnostic a host has to detect a dead watcher reports the opposite; the doc claim is false. What correct looks like: `let join = ...; if join.is_none() { shutdown.store(true, ...) }` or construct the handle only on success with a typed error. Confidence: HIGH — Verification: read (both branches traced).

F-002 [S3]
Where: crates/device-discovery/src/monitor.rs:148 (and crate reachability)
Code:
```
    pub fn spawn(source: Box<dyn VolumeSource>, config: MonitorConfig) -> VolumeMonitorHandle {
```
What is wrong: nobody calls it. `rg -n 'VolumeMonitor|poll_once' --glob '*.rs' apps crates tests fuzz` matches only monitor.rs itself, lib.rs:65 (re-export), and volume.rs:160 (doc mention). No Cargo.toml in the repo depends on `device-discovery` (platform-* manifests name it only in comments as future issue-#42 work). The crate's own lib.rs concedes the platform adapters (Linux udisks2 / Windows volume notifications / macOS disk-arbitration) have not landed and probe wires `UnavailableVolumes`. Why it matters: 41 passing tests suggest a working hotplug subsystem; zero shipped hosts can reach it. What correct looks like: platform adapters + host wiring, or a crate-level "not yet wired" marker. Confidence: HIGH — Verification: ran rg over apps/crates/tests/fuzz and all Cargo.tomls.

F-003 [S4]
Where: crates/device-discovery/src/monitor.rs:79-81
Code:
```
/// A running watcher. Dropping it signals shutdown and JOINS the
/// thread (bounded shutdown latency: one poll-interval slice, 50 ms
/// max — see the loop).
```
What is wrong: the 50 ms bound covers only the sliced sleep; a stop signal arriving while `one_pass` is mid-`enumerate` (a potentially blocking platform call) delays the join by the enumerate duration, which this crate's own platform adapters (unimplemented) would define. Minor doc overclaim; the slicing itself is correct (L209-216). Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory, >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MonitorConfig::new / with_limits / Default | 48-77 | REAL | — | clamps: interval ≥100 ms, capacity ≥1; tested |
| VolumeMonitorHandle::events | 90-95 | REAL | — | pure accessor to bounded queue |
| VolumeMonitorHandle::errors / passes | 97-109 | REAL | — | honesty instruments, Relaxed loads (counters — fine) |
| VolumeMonitorHandle::is_running | 111-115 | REAL | S3 | lies after spawn failure (F-001) |
| VolumeMonitorHandle::stop / Drop | 117-134 | REAL | — | idempotent, joins; `drop(join.join())` observes poison without panicking |
| VolumeMonitor::spawn | 142-184 | REAL | S3 | spawn-failure path contradicts its comment (F-001); crate unreachable (F-002) |
| MonitorWorker::run | 198-218 | REAL | — | pass + 50 ms-sliced sleep; shutdown checked before pass and per slice |
| MonitorWorker::one_pass | 220-244 | REAL | — | passes++ before enumerate; failure → errors++, NO events; max_devices violation → errors++ (loud, no truncation); last updated only on success |
| poll_once | 246-271 | REAL | — | loop-free same computation; typed CapacityExceeded; documented wasm-host rationale |
| tests: Scripted + 6 tests | 273-436 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| config_clamps_to_floors | YES | exact clamp values asserted |
| poll_once_diffs_and_returns_the_snapshot | YES | 1 Added, then Removed+Added pair |
| poll_once_propagates_the_typed_failure | YES | typed Unavailable surfaces |
| poll_once_enforces_the_device_bound | YES | CapacityExceeded{limit:256, attempted:300} exact |
| spawn_first_pass_reports_mounted_volumes_as_added | YES | real thread, 2 Added from real diff, passes≥1, errors==0, joined |
| failed_passes_count_errors_and_never_emit_removals | YES | errors≥1, passes≥2, all drained events Added — removal fabrication would fail |

Note: the spawn tests poll with 10 ms drains under a 2 s deadline — genuine synchronization, not sleeps-passed-as-asserts.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Events come only from real enumeration diffs ... An enumerate failure pushes NOTHING" | monitor.rs:17-22 | REAL | one_pass L226-232; test at 403-435 |
| "The queue is capacity-clamped drop-oldest with counters" | monitor.rs:23 | REAL (queue itself audited in events.rs — outside my list) | queue via VolumeEventQueue::with_capacity(config.queue_capacity) L149 |
| "Shutdown is responsive (the sleep is sliced) and the handle joins the thread on drop" | monitor.rs:24-25 | REAL (with F-003 caveat re: mid-enumerate stop) | L199-218, L130-134 |
| "the honest handle reports a dead watcher (is_running false, zero passes)" | monitor.rs:171-173 | FALSE | F-001: is_running() returns true after spawn failure |
| "the engine host relays into the catalog's devices.volume-added / -removed rows (the relay itself is host wiring)" | lib.rs:12-14 | UNREACHABLE | no host wiring exists — F-002 |

## 7. Dead / duplicated / unreachable code
The crate is unreachable from shipped entry points (F-002): no dependents, no platform adapters, `Discovery::probe` returns the all-`Unavailable` fallback (lib.rs:21-25). Within monitor.rs: no dead code; every item is exercised by the crate's own tests.

## 8. Dependencies & flags
crate::{context, error, events, volume} + std only. No features. No unsafe, no unwrap/expect (matches workspace lints). Ordering::Relaxed on counters is correct for monotonic stats.

## 9. Open questions for the next agent
- Does any feature-matrix row claim volume hotplug / device browsing as a working feature? If yes, cite F-002 (UNREACHABLE crate) against it.
- events.rs (queue drop-oldest + counters) is the untested-here half of the "bounded queue" claim — its auditor should verify push/drain semantics and the D-3 clamp counters.
