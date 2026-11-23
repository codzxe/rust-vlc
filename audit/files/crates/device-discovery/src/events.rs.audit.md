# AUDIT: crates/device-discovery/src/events.rs
| Field | Value |
|---|---|
| Auditor | W10-A07 |
| Date | 2026-11-27 |
| Lines / functions | 306 / 17 (incl. 6 tests) |
| File verdict | REAL — but the whole crate is ORPHANED (no workspace member depends on it) |
| Reachable from shipped entry point? | NO — no Cargo.toml in the workspace declares `device-discovery` as a dependency; platform crates explicitly exclude it ("manifest is an allowlist", platform-linux/Cargo.toml:10) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | for-loop rg over all crate/app Cargo.tomls (exit 0, zero DEP hits); rg DeviceEventQueue platform crates exit 0; sed -n verifications exit 0; baseline citation |

## 1. One-paragraph verdict
The queue and the diff are correct, small, and honestly documented: bounded
VecDeque with drop-oldest plus monotonic dropped/pushed counters, poison-tolerant
push/drain, zero-capacity clamped to 1, and a pure `diff_volumes` whose
removed-before-added ordering and changed-as-pair semantics are pinned by tests that
would fail if any of it broke. All 6 tests pass in the baseline. The file's fatal
context: NOTHING depends on the device-discovery crate — not the CLI, not desktop,
not the platform crates (whose manifests name it only to record its exclusion). The
"mirrors the wave-22 platform crates' DeviceEventQueue deliberately" claim is TRUE
(push/poison/drain bodies are line-for-line the same shape as
platform-macos/src/events.rs:95-118), which also means the duplication is real and
tracked by a [PROPOSAL] seam.

## 2. What actually works (verified)
— verified by baseline cargo-test-workspace.txt L2635-2721 (PACKAGE
device-discovery exit_code=0): events::tests 6/6 `ok` inside 38 unit tests.
— verified by `for f in crates/*/Cargo.toml apps/*/Cargo.toml; rg -q '^device-discovery'`:
ZERO dependent manifests.
— verified by rg/sed side-by-side with crates/platform-macos/src/events.rs:95-118:
push/drain/len bodies structurally identical (mirror claim holds).

## 3. Findings
F-001 [S3]
Where: crates/device-discovery/src/events.rs:1-2 (crate scope)
Code:
```
//! The bounded volume-event queue: where mount-table diffs land (card
//! 7.D1; the event-plumbing half of the volume lane).
```
What is wrong: the "volume lane" has no consumer. No workspace member declares a
dependency on `device-discovery`; the platform crates' manifests record it as
excluded (platform-linux/Cargo.toml:6-10, platform-macos:5-10, platform-windows:6-10:
"device-discovery is issue #42 — the manifest is an allowlist"). Correct, tested code
that no shipped or even sibling crate can reach. Confidence: HIGH — Verification:
read (exhaustive manifest scan).
F-002 [S4]
Where: crates/device-discovery/src/events.rs:4-6
Code:
```
//! The design mirrors the wave-22 platform crates' `DeviceEventQueue`
//! deliberately (same bounds, same counters, same drain shape) so the
```
What is wrong: verified true, but it is copy-paste duplication across four crates
(device-discovery + 3 platform crates) awaiting a [PROPOSAL] unification — drift risk
lives until then (e.g., a fix here does not propagate). Confidence: HIGH —
Verification: read (bodies compared).
F-003 [S4]
Where: crates/device-discovery/src/events.rs:113-118
Code:
```
        } else {
            // A poisoned queue means a consumer panicked mid-drain;
            // dropping the event is the honest, non-crashing choice.
            self.dropped.fetch_add(1, Ordering::Relaxed);
            false
        }
```
What is wrong: poison policy here (drop the event) diverges from the workspace's
dominant `unwrap_or_else(|p| p.into_inner())` recovery convention (e.g.
transcode/src/runner.rs:984, streaming client.rs:358-359). Deliberate and documented,
but a consumer that panics mid-drain permanently silences the queue (poison never
clears; every future push drops). A poison-clearing drain would self-heal.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| VolumeEvent::volume_id / is_added / is_removed | 45-66 | REAL | — | pure accessors |
| VolumeEventQueue::with_capacity | 82-89 | REAL | — | clamp max(1) |
| VolumeEventQueue::new | 94-96 | REAL | — | 128 |
| push | 101-119 | REAL | F-003 | drop-oldest, counters, poison drop |
| drain | 122-127 | REAL | — | whole batch, poison → empty |
| len / is_empty / capacity / dropped / pushed | 131-160 | REAL | — | pure observation |
| diff_volumes | 178-205 | REAL | — | removed-first, changed→pair; compares label/kind/root (all mutable fields) |
| 6 tests | 221-305 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| queue_bounds_and_counts_drop_oldest | YES | push return values, dropped=1, pushed=3, drained order b-first |
| zero_capacity_is_clamped_not_a_black_hole | YES | capacity()==1, push retained |
| diff_reports_adds_and_removals | YES | exact event sequence |
| diff_expresses_changes_as_removed_then_added | YES | order + new payload fields |
| diff_of_identical_snapshots_is_empty | YES | no-op diff |
| volume_id_accessor_covers_both_variants | YES | accessor on both arms |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "hard capacity ... oldest event dropped, monotonic dropped counter" | events.rs:7-9 | REAL | push:106-111 + test |
| "a mount storm can never grow the process" | events.rs:9-10 | REAL | capacity clamp + drop-oldest |
| "mirrors the wave-22 platform crates' DeviceEventQueue" | events.rs:4-5 | REAL | byte-shape comparison vs platform-macos events.rs |
| "the production path is identical" (tests drive same push) | events.rs:15-17 | REAL-UNPROVEN | no production path exists to differ (F-001) |

## 7. Dead / duplicated / unreachable code
Whole crate unreachable (F-001). Duplication with platform crates documented (F-002).

## 8. Dependencies & flags
std-only + crate::volume. No externals, no features, no unsafe, no env reads.

## 9. Open questions for the next agent
- Issue #42 was supposed to wire this crate somewhere (platform allowlists, a UI
  devices panel?). Confirm the wiring card exists in planning docs or the lane was
  abandoned finished-but-unshipped.
