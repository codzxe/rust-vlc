# AUDIT: crates/media-clock/tests/zero_alloc_backends.rs

| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-22 |
| Lines / functions | 163 / 1 `#[test]` + Counter/Wall fakes + ts helper |
| File verdict | **REAL** — genuine two-window zero-allocation proof over the multi-backend clock's tick and control planes |
| Reachable from shipped entry point? | The *subject*: `MediaClock` with backends — the shipped CLI constructs MediaClock (audio-master mode) on the live path (session runtime clock); the full multi-backend surface (video master, external, free-run, master-switch storms) is engine API. The test runs under `cargo test`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read of the full file; cross-check of MediaClock production construction (session.rs runtime clock). No cargo (static). |

## 1. One-paragraph verdict

Same disciplined pattern as video-engine's zero_alloc_scheduler (counting global allocator, serialize_tests, one test per binary, warm-up covering every measured path at every master it will run under — 79-89), extended over two measured windows: 4 096 steady-state video-master ticks (interpolate + steer + read + drain, window 1, 91-112) and a 256-switch master-storm + full loss chain to FreeRunning + Ended latch + seek_back re-arm (window 2, 114-162). Assertions are behavioral *and* measured: the steer must be `Snapped` (debug_assert, 107), the loss chain must end FreeRunning (134), the latch must report Ended (146), seek_back must re-arm Paused (151) — then both windows assert zero bytes. It would fail under allocation regressions and under control-plane state-machine regressions alike.

## 2. What was checked

- Fakes are allocation-free (atomics only, 30-51) — the fakes cannot pollute the measurement.
- Warm-up (79-89) covers each measured call once, including a master switch, so first-touch costs are outside the windows. The switch storm's *per-switch drains* (126) are inside window 2 by design (they are the measured control-plane work).
- `debug_assert!(matches!(steer, Snapped))` (107): only checked in debug builds — in release the steer result is unasserted inside the loop. Benign (cargo test runs debug by default), noted as F-001.
- State assertions embedded in the windows (134, 146, 151) tie the zero-alloc claim to *correct* behavior, not just cheap behavior.
- Subject reachability: MediaClock is the session's runtime clock (session.rs `runtime.clock`), production-live.

## 3. Findings

### F-001 [S4] — in-window steer assertion is debug-only
Where: crates/media-clock/tests/zero_alloc_backends.rs:107
Code:
```rust
            debug_assert!(matches!(steer, media_clock::VideoSteer::Snapped { .. }));
```
What is wrong: if the suite is ever run `--release`, the only per-tick behavioral assertion inside window 1 disappears (the window still asserts zero bytes, so an allocator regression is caught but a steering regression is not).
Why it matters: `cargo test --release` is a plausible CI leg; the test silently weakens.
What correct looks like: a plain `assert!` (the match is cheap) or a counted summary assert after the loop.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Item | Lines | Verdict | Notes |
|---|---|---|---|
| Counter / Wall fakes | 30-51 | REAL | atomic-only, allocation-free |
| ts helper | 53-55 | REAL | — |
| backend_surface_is_zero_allocation_after_warm_up | 57-163 | REAL | two windows + warm-up + state asserts (§1) |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| backend_surface_is_zero_allocation_after_warm_up | YES | allocation (two guards), state machine (FreeRunning/Ended/Paused asserts), loss-chain order; the only gap is the debug-only steer assert (F-001) |

UNVERIFIED by run (no cargo); W00 baseline consistency applies.

## 6-9

- Doc claim "One `#[test]` per binary by design" (13-15): verified — one test.
- No dead code. Deps: media-clock, media-types, testkit (dev) — consistent with the crate manifests.
- Open question: none.
