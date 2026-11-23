# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_06_repeated_seeks.rs
| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-23 |
| Lines / functions | 235 / 3 (1 test + Lcg with 2 methods) |
| File verdict | REAL (100-seek storm over the real AVI/MJPEG/PCM parent with per-seek terminal, staleness, tolerance, and allocation-steady oracles; executed pass) |
| Reachable from shipped entry point? | N/A (test file) — drives the real engine via scenarios_b/host.rs `dispatch` (the in-test CLI-mirror; same architecture as soak/dispatch.rs) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/scenarios_b-3501eaa5a90abc4a hundred_random_seeks` → `1 passed; 0 failed` in 0.48s (prebuilt binary; cargo blocked by full disk) |

## 1. One-paragraph verdict
A genuinely adversarial seek storm: 100 deterministic LCG targets over the 2 s parent, each dispatch pumping to its own terminal, asserting per-seek completion counting (`completed_now == completed + 1`), zero typed failures on in-range targets, landing within the parent's 100 ms tolerance, a per-seek no-stale-frames window oracle (frames delivered under this seek stamped ≥ landing − 40 ms grace, units verified: ring stores ns, `landed` µs × 1000), a bounded-memory claim with an EXPLICIT accounting bound for the host's own recording (2× events × size_of(WireEvent) + the 1024×12 B ring + the deltas scratch), a 5-seek allocation-steady probe with a tight 4 096-byte ceiling, and an honest `Ended` + close at the end. Executed: passes in 0.48 s (real decode work). One fidelity note: "no waits" is interpreted as back-to-back terminal-awaited dispatches, so seek supersession is not exercised in this scenario (cross-covered elsewhere).

## 2. What actually works (verified)
- `./target/debug/deps/scenarios_b-3501eaa5a90abc4a hundred_random_seeks` → `test scenario_06_repeated_seeks::hundred_random_seeks_complete_without_stale_output_or_growth ... ok` (executed).
- Units cross-checked against the harness: `ObservingSink::ring_push(track, audio, pts_ns)` (host.rs:514) stores NANOSECONDS; the oracle's `*pts < landed * 1_000 - 40_000_000` is µs→ns with a 40 ms grace — correct.
- Harness facts relied on and verified: `PARENT = "generated/valid/avi_mjpeg_pcm.avi"` (host.rs:137), `RING_CAP = 1024` (host.rs:508), `dispatch` at host.rs:1988, `count_events` at host.rs:2029, `run_until_terminal` at host.rs:1156.
- Mutation reasoning: a per-seek engine leak fails the 4 096-byte probe; a lost terminal fails the `completed + 1` chain; a stale pre-flush frame fails the window oracle; a landing outside 100 ms fails the tolerance assert; a media that cannot end post-storm fails `terminal == Ended`.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_06_repeated_seeks.rs:88–91 (module doc 4) vs the loop at 99–160
Code:
```
    // The storm: 100 deterministic random targets, no waits between
    // submissions (each dispatch awaits its own terminal — the
    // synchronous host's "no waits" is back-to-back dispatches).
```
What is wrong: doc 10 §6.4 row 6's "no waits" is narrowed to sequential, terminal-awaited dispatches — each seek completes before the next submits, so the scenario never exercises seek SUPERSESSION (a second seek racing an in-flight one, the doc 05 §4.2 single-slot rule) or inbox backpressure under a seek storm. The narrowing is disclosed in the comment, and supersession is covered by other suites (soak/scenario.rs skip-rate accounting; source_swap.rs per session.rs docs) — but this scenario alone does not prove the row's rapid-fire shape. Confidence: HIGH — Verification: read (both this file's loop and the cited cross-coverage).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Lcg::next_u64` | 34–41 | REAL | — | Park–Miller-style constants, wrapping, deterministic |
| `Lcg::target_us` | 43–46 | REAL | — | `[0, duration_us)`; modulo bias negligible for 2e6 |
| `hundred_random_seeks_complete_without_stale_output_or_growth` | 49–235 | REAL | S4 | warm-up (3 seeks + session-cumulative baseline capture), storm (terminal chain, tolerance, stale window), memory bound with host-recording accounting, 5-seek ≤4 096 B probe, Ended + close; F-001 narrowing |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| `hundred_random_seeks_complete_without_stale_output_or_growth` | YES | itemized in §2 — every claim has a failing oracle; the warm-up/`serialize_tests` discipline keeps the allocator measurement clean |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "100 random targets, no waits — no stale output ever; bounded memory; each completes or typed-fails" (doc 10 §6.4 row 6) | scenario_06.rs:1–3 | REAL with a narrowing | all oracles pass (executed); "no waits" = sequential dispatches (F-001) |
| "the seek-completed counter is SESSION-CUMULATIVE … the storm's baseline is the post-warm-up count" | scenario_06.rs:70–75 | REAL | baseline captured after warm-up; chain asserts `+1` per seek (executed) |
| "an earlier landing's frames sit behind a later landing by construction … they are not stale output" | scenario_06.rs:94–99 | REAL | per-seek window `[ring_before..]` is the correct scope for backward targets |

## 7. Dead / duplicated / unreachable code
- None in this file. (host.rs at 88 KB is a god-module for the whole scenario suite — not my assignment, noted for its owner.)

## 8. Dependencies & flags
- Test-only; sibling module deps (host/main/probe via `crate::`), commands, testkit (fixtures + alloc_count). No #[ignore] (verified by run: 23 tests in the binary, 22 filtered + 1 run).

## 9. Open questions for the next agent
- scenarios_b/host.rs (88 KB) and main.rs are NOT in my assignment — the `dispatch`/`settle`/`run_until_terminal` fidelity to the CLI shapes lives there.
- scenario_12_device_loss.rs is the natural companion to my hotplug.rs finding (F-001 there) — check whether it drives the hotplug hooks directly (library-level) and note it cannot prove shipped-host wiring either.
