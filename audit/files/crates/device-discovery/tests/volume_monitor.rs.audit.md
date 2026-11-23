# AUDIT: crates/device-discovery/tests/volume_monitor.rs
| Field | Value |
|---|---|
| Auditor | W10-A10 |
| Date | 2026-11-23 |
| Lines / functions | 235 / 3 tests + 5 helpers/doubles |
| File verdict | REAL as a test (executed, 3/3 pass, would fail if the monitor core broke) — but the machinery it exercises is UNREACHABLE from shipped entry points and has NO production data source |
| Reachable from shipped entry point? | The test target itself runs in `cargo test`. The code under test (`VolumeMonitor::spawn`): NO — no app depends on device-discovery; the only non-test `VolumeSource` impl in the repo is `UnavailableVolumes` (always `Err`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `cargo test -p device-discovery --test volume_monitor` -> 3 passed / 0 failed (exit 0, 3.00 s); grep for VolumeSource impls and manifest dependents; `sed -n` quote verification |

## 1. One-paragraph verdict
This is an honest integration test of a monitor that has nothing real to watch. It drives the real `VolumeMonitor::spawn` thread/loop/diff machinery over a scripted `VolumeSource`, asserts the initial mount set as three exact Added wire shapes, the scripted removal as an exact Removed, zero errors, a genuinely strong bounded-queue accounting invariant (`pushed == dropped + retained + drained` under a 64-event storm), and idempotent stop + drop-safety — all 3 tests pass under my run. But the repo's only non-test `VolumeSource` implementation is `UnavailableVolumes` (returns `Err` forever); no platform crate ships a real enumerator, no app depends on device-discovery, and the "host relay" whose contract the test pins is a test-local struct with no production counterpart. Volume watching, from a shipped user's perspective, does not exist — and the test file says so in its own header.

## 2. What actually works (verified)
— verified by `cargo test -p device-discovery --test volume_monitor` -> `test result: ok. 3 passed; 0 failed` (watcher initial+removal with exact wire shapes, concurrent drain accounting, stop idempotence).
— verified by grep: `impl VolumeSource` in the repo = monitor.rs:295 (in-crate test double), volume.rs:199 (`UnavailableVolumes`, always Err), and this file's `ScriptedMounts` — no platform integration exists; `device-discovery` appears in no app manifest (platform-{linux,macos,windows} Cargo.tomls mention it as future issue #42).

## 3. Findings
F-001 [S3]
Where: crates/device-discovery/tests/volume_monitor.rs:118-125 (code under test reachability)
Code:
```
    let mut handle = VolumeMonitor::spawn(
        Box::new(ScriptedMounts {
            initial,
            edited,
            served_edit: false,
        }),
        MonitorConfig::with_limits(Duration::from_millis(100), 32),
    );
```
What is wrong: the machinery this test legitimizes (monitor thread, table diffing, event queue, error counting) has no production data source and no shipped consumer — every real `VolumeSource` would be `UnavailableVolumes` (volume.rs:199-203, `Err(self.reason.clone())` forever). The test cannot fail because of any platform-integration breakage because there is none to break; it proves the core loop only. The test header discloses this honestly ("production sources are the platform-* integrations that land with their cards"), so this is a scope limitation, not test fraud — but any matrix row citing this test as evidence of volume watching working for users would be inflation.
Confidence: HIGH — Verification: ran (test) + grep (impls, manifests).
F-002 [S3]
Where: crates/device-discovery/tests/volume_monitor.rs:49-81
Code:
```
/// The catalog payload shape (doc 05 §6.2, the 7.C1 additive rows):
/// what the engine-side relay copies out of a `VolumeEvent`.
#[derive(Debug, PartialEq, Eq)]
struct VolumeAddedWire {
```
and
```
/// The mechanical relay: field copy, no re-derivation (this is the
/// contract the emission ledger's "host wiring" step depends on).
fn relay_added(event: &VolumeEvent) -> Option<VolumeAddedWire> {
```
What is wrong: the "contract" is defined and asserted entirely inside the test — `VolumeAddedWire`/`VolumeRemovedWire`/`relay_added`/`relay_removed` are test-local re-implementations of an engine-side relay that does not exist. If the future production relay (or doc 05 §6.2) diverges from this hand-rolled shape, nothing in the repo fails; the pin is aspirational documentation dressed as a test. Mutation reasoning: deleting the production relay-to-be cannot break this test because there is nothing to delete.
Confidence: HIGH — Verification: read + grep (no other definition of the wire shape outside this file).
F-003 [S4]
Where: crates/device-discovery/tests/volume_monitor.rs:85-100
Code:
```
    let end = std::time::Instant::now() + deadline;
    let mut seen = Vec::new();
    while std::time::Instant::now() < end {
        seen.extend(queue.drain());
        if pred(&seen) {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
```
What is wrong: wall-clock polling harness (10 ms sleeps against a 100 ms monitor tick, 3 s deadlines) — timing-sensitive by construction. The predicates are monotone accumulations, so it fails only on genuine starvation, and 3 s ≫ 100 ms tick makes flakes unlikely; still, this is the classic shape that flakes on a loaded 2-core CI box. The observed 3.00 s runtime confirms the deadline path is being eaten (phase 2 waits out its full window? No — the removal predicate breaks early; the 3 s is the stop/drop test's 30 s config? The measured 3.00 s matches the two drain phases' sleep granularity — benign).
Confidence: MEDIUM — Verification: ran (3.00 s total); timing analysis read.

## 4. Per-function table (test-file inventory)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ScriptedMounts (VolumeSource double) | 20-38 | REAL double | — | Initial-then-edited-then-steady script; sanctioned per header |
| volume() helper | 40-47 | REAL | — | Fixture constructor |
| VolumeAddedWire / VolumeRemovedWire | 51-61 | TEST-LOCAL CONTRACT | S3 | F-002: no production counterpart |
| relay_added / relay_removed | 63-81 | TEST-LOCAL CONTRACT | S3 | F-002 |
| drain_until | 85-100 | REAL | S4 | F-003 polling harness |
| watcher_reports_initial_mounts_then_the_scripted_removal | 102-168 | REAL | — | Exact wire shapes ×3, exact removal, errors()==0 |
| concurrent_drain_while_the_watcher_pushes_is_safe | 170-219 | REAL | — | Strong accounting invariant under storm; join-panic surfacing honest |
| stop_is_idempotent_and_drop_safe | 221-235 | REAL | — | stop×2 + Drop |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| watcher_reports_initial_mounts_then_the_scripted_removal | YES (monitor core) | Exact event sequence + wire shapes + zero errors; breaks if diff, queue, or thread loop regresses. Cannot catch platform-source breakage (none exists — F-001) |
| concurrent_drain_while_the_watcher_pushes_is_safe | YES | pushed==dropped+len+drained is a real invariant; breaks on queue accounting bugs, lost/dup events |
| stop_is_idempotent_and_drop_safe | YES | Double-stop + drop after stop; breaks on join/idempotence regressions |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "production sources are the platform-* integrations that land with their cards" | volume_monitor.rs:3-4 | REAL (honest disclosure) | grep: no platform VolumeSource impl exists |
| "the event vocabulary maps 1:1 onto the catalog's devices.volume-added/-removed payload shape" | volume_monitor.rs:6-8 | UNVERIFIED | The catalog payload shape lives in doc 05 §6.2 / the commands catalog — the test's wire structs are hand-copied from the doc, not generated from it; no cross-check exists |
| "concurrent drain is safe; shutdown joins" | volume_monitor.rs:8-9 | REAL | Tests 2 and 3, executed |

## 7. Dead / duplicated / unreachable code
- Code under test unreachable from shipped entry points (no app depends on device-discovery).
- The wire-shape structs duplicate (by hand) the catalog payload vocabulary — the duplication F-002 flags.

## 8. Dependencies & flags
- Test-only crate-internal: device_discovery public API only (VolumeMonitor, MonitorConfig, VolumeEventQueue, Volume, VolumeId, VolumeKind, VolumeSource, DiscoveryContext). No feature gates. No external deps.

## 9. Open questions for the next agent
- Check the feature-matrix rows for `device-volume-*`/`disc-*`: if any cites this test as user-visible evidence, that row is inflated (the test proves the core only).
- When platform integrations land (issue #42), the wire-shape structs should move to the real relay and this test should keep a cross-check against the catalog's actual types.
