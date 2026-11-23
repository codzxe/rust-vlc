# AUDIT: crates/media-clock/tests/zero_alloc_backends.rs
| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-23 |
| Lines / functions | 163 / 5 (1 test + 2 fakes + 2 builders) |
| File verdict | REAL (measured zero-allocation proof over the multi-backend clock surface; executed pass) |
| Reachable from shipped entry point? | N/A (test file) — the surface under test is shipped: apps/cli/src/pipeline_host.rs:1605 constructs `MediaClock::audio_master(sink.clock_source(), …)` (the audio-limb lane) and :1630 `MediaClock::system_monotonic` (the fallback) — the backend core this test drives is the same machinery |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 (zero-findings disclosure in §3) |
| Commands run | `./target/debug/deps/zero_alloc_backends-d031acc3325166e5` → `1 passed; 0 failed` in 0.00s (prebuilt binary; cargo blocked by full disk) |

## 1. One-paragraph verdict
Exactly what its doc claims, measured rather than argued: a process-global counting allocator, one test per binary, warm-up that touches every measured path under every master it will run (first-touch sizing lands outside the windows), then two `assert_zero` windows — (1) 4 096 video-master steady-state ticks exercising `now`/`try_now`/`snapshot`/`anchor`/snapped `observe_video` steers/empty-ring `take_master_change` polls, and (2) the control plane itself: a 256-switch master storm with drains, the full loss-demotion chain to `FreeRunning`, 4 096 free-run ticks, the `Ended` latch, `seek_back` re-arm, resume, and 4 096 more ticks. State pins (`AudioOutput` default, `FreeRunning` after losses, `Ended`→`Paused` latches) anchor the sequence so the zero-alloc claim cannot pass on a no-op clock. No defects found.

## 2. What actually works (verified)
- `./target/debug/deps/zero_alloc_backends-d031acc3325166e5` → `test backend_surface_is_zero_allocation_after_warm_up ... ok` (executed).
- The measured surface is the shipped audio path's clock core (`MediaClock::audio_master` in the CLI, pipeline_host.rs:1605) — not an orphan API.
- The state pins are load-bearing: `assert_eq!(clock.master(), ClockMaster::AudioOutput)` (setup), `FreeRunning` after the three losses, `ClockState::Ended` then `Paused` from `seek_back` — a clock that silently no-ops `switch_master`/`on_master_lost` would fail these before the alloc windows.

## 3. Findings
None. What was checked and how (zero-findings disclosure per spec §12):
- Fakeness: the two fakes (`Counter`, `Wall`) are injected-truth sources (the crate's own trait seams), not mocks of the unit under test — the unit under test is the real `MediaClock::with_backends` over all four backends (`audio`/`external`/`video(true)`/wall).
- Measurement soundness: warm-up precedes both guards; setup allocations excluded; `serialize_tests()` + one-test-per-binary prevents cross-thread counting; drains (`while clock.take_master_change().is_some()`) keep the 8-slot ring from interfering inside window 2.
- Claim scope: the file asserts allocation-zero only (plus the state pins); positional correctness is explicitly out of scope (owned by the crate's other tests) — the module doc says exactly this, no overclaim.
- No `#[ignore]`, no feature gates, no sleeps; `debug_assert!` on the snapped steer is active in the debug test profile.
- Units: all timestamps on `NANO_BASE`; wall steps in ns — consistent.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Counter::sample_count/sample_rate_hz` | 27–34 | REAL | — | atomics-only fake source |
| `Wall::now_ns` | 39–44 | REAL | — | atomics-only fake source |
| `ts` | 46–48 | REAL | — | NANO_BASE builder |
| `backend_surface_is_zero_allocation_after_warm_up` | 50–163 | REAL | — | the two-window proof; executed pass; window 1 = 4 096 video-master ticks, window 2 = 256 switches + loss chain + Ended/seek_back + free-run ticks |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| `backend_surface_is_zero_allocation_after_warm_up` | YES | any steady-state or control-plane allocation fails `guard.assert_zero` (2 windows × precise message); a no-op control plane fails the master/latch state pins |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "after warm-up, the steady-state tick paths … plus the control-plane transitions themselves … allocate **zero bytes**" | zero_alloc_backends.rs:2–9 | REAL | both `assert_zero` windows passed (executed) |
| "The switches are Arc refcount moves into fixed-size core state; the event trail is an 8-slot ring; the outcomes are `Copy` values" | zero_alloc_backends.rs:9–12 | REAL-UNPROVEN (mechanism) | the measured zeros are consistent with it; the media-clock internals are not my assignment |

## 7. Dead / duplicated / unreachable code
- None.

## 8. Dependencies & flags
- Test-only; deps media-clock, media-types, testkit (alloc_count). `#[global_allocator]` swap is binary-local. No features, no ignores.

## 9. Open questions for the next agent
- crates/media-clock/src (the backends implementation) is not in my assignment — the "8-slot ring / Arc moves / Copy outcomes" mechanism claim above should be confirmed there, along with whether `master_changes_lost()` can ever be nonzero under the shipped hosts' drain cadence.
