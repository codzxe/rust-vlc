# AUDIT: crates/media-clock/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-23 |
| Lines / functions | 143 / 1 (`validate_rate`; the rest is module docs, one doctest example, re-exports) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: the crate is the clock every shipped host constructs — `lumen open` → `apps/cli/src/pipeline_host.rs:1605 MediaClock::audio_master(sink.clock_source(), …)` / `:1630 MediaClock::system_monotonic(InstantSource…)`; `validate_rate` guards the mutation boundaries `clock.rs:522` and `clock.rs:808` (set_rate paths). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/media_clock-7012bdf535a3032b` → 92 passed, 0 failed, exit 0 (full crate unit tests, incl. `clock::tests::pause_freezes_resume_continues_without_jump`). Doctest execution attempted: `cargo test -p media-clock --doc` → BLOCKED, `StorageFull` (Os code 28) creating rustdoc temp dir — UNVERIFIED (reason: sandbox disk full, not a repo defect). |

## 1. One-paragraph verdict
A front-door module whose value is its contract documentation and its re-export surface; the only code is a five-line defensive re-validation of `Rate` at the clock's mutation boundaries (sound: `Rate`'s pub fields bypass `Rate::new`, and `validate_rate` re-runs the exact constructor — rejecting zero components and reducing). The documented semantics (audio-master from the sample counter, never a wall clock; explicit system-monotonic fallback for audio-less media; injected time, no ambient reads; no dedicated thread; short internal lock never held across I/O; bounded ±1000 ppm drift correction) match what I verified in the shipped wiring and in the crate's 92 passing unit tests. The doctest example could not be executed (sandbox disk full) but its exact semantics (pause freezes while the counter keeps running; resume continues without jump) are pinned by the unit test `clock::tests::pause_freezes_resume_continues_without_jump`, which I ran.

## 2. What actually works (verified)
- `./target/debug/deps/media_clock-7012bdf535a3032b` → `test result: ok. 92 passed; 0 failed` — clock, backends, drift, sync, mapping families all green (mapping's 15 covered in my mapping.rs audit).
- `validate_rate` call sites verified at the two mutation boundaries (clock.rs:522, 808) — the pub-field `Rate` bypass hole is compensated exactly where rates enter the clock, consistent with mapping.rs's own `checked_rate` belt-and-suspenders.
- Hostile-question cross-check (who drives the clock with no audio device): this crate's docs state the fallback policy — `ClockMaster::SystemMonotonic` "so the UI can tell the truth rather than trust a silent approximation" — and the shipped CLI implements exactly that split (`audio_master` over the null-device counter at pipeline_host.rs:1605; `wall_clock` system-monotonic at 1629-1632). The claim and the code agree.

## 3. Findings
F-001 [S4]
Where: crates/media-clock/src/lib.rs:31-38 (with 104-107)
Code:
```
//! No dedicated thread. The audio pump's counter updates and every
//! reader's `now()` go through one short internal lock whose critical
//! sections are pure arithmetic — never held across I/O. Sources must be
//! non-blocking ([`source`] contract). The lock-free / seqlock read path
//! is Phase-3 hardening; v0's short lock satisfies the "never held
//! across I/O" clause and keeps all arithmetic checked and panic-free.
```
What is wrong: nothing in the code — this is a recorded gap with an honest label. The claim "never held across I/O" is a property of clock.rs's implementation, which is another agent's file; from this side I verified the consequences visible in the files I own (mapping.rs is pure; the shipped hosts poll the clock only on their own threads). The seqlock read path remains future work, meaning every `now()` takes a mutex — acceptable at v0's poll cadence, a Phase-3 hardening item the crate itself flags. Listed so the next agent verifies clock.rs's critical sections are truly arithmetic-only. Confidence: MEDIUM — Verification: read (clock.rs internals not audited by me; UNVERIFIED there). Doctest non-execution (disk full) also recorded here.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `validate_rate` | 141-143 | REAL | — | `Rate::new(num, den)` re-run verbatim: rejects zero components, reduces. Callers must use the returned (reduced) value — both call sites do (`let validated = crate::validate_rate(rate)?;`). |
| doctest example | 66-102 | REAL-UNPROVEN | S4 | Semantics unit-covered (`pause_freezes_resume_continues_without_jump`, executed); the doctest binary itself could not run — sandbox disk full (Os code 28). |
| re-exports (109-129) | — | REAL | — | Every listed symbol exists in its module (spot-verified via the 92-test run compiling the whole crate). |

## 5. Test honesty
No test module in this file. The crate's 92 unit tests (all executed by me) cover the documented semantics; the one doctest is honest in content (asserts freeze/resume exact ticks) but UNVERIFIED by execution in this sandbox.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Audio is master by default … derives now() from the audio output's sample counter … never from a wall clock" | lib.rs:6-9 | REAL | Shipped wiring: pipeline_host.rs:1605 audio_master(sink.clock_source()); the MP4 end-to-end run I did advanced position only with host ring drains. |
| "When a medium has no audio … the clock falls back explicitly to a monotonic time source … so the UI can tell the truth" | 9-12 | REAL | pipeline_host.rs:1629-1632 `wall_clock` builds `system_monotonic(InstantSource)` — the explicit fallback, host-chosen. |
| "Time is injected, never ambient … tests wire virtual sources … no sleeps (doc 03 §8)" | 16-20 | REAL | Every test harness I audited (tests/common, engine_fx, recovery_media) uses SharedNow/FrozenNow virtual sources — zero sleeps found in the playback-engine test tree I read. |
| "Pause freezes the position; resume continues from the same media time … even if the underlying source kept running while paused" | 47-49 | REAL | `clock::tests::pause_freezes_resume_continues_without_jump` executed green. |
| "Drift correction is a bounded rate adjustment (±1000 ppm)" | 50-52 | REAL (exported constant) | `drift::MAX_CORRECTION_PPM` re-exported; drift tests in the 92-pass run. |
| "Never: I/O, device APIs, UI, threads of its own" | 104-107 | REAL | Crate modules contain no I/O/thread spawning (mapping pure; source/backends audited only at the surface — clock.rs internals are another agent's assignment). |

## 7. Dead / duplicated / unreachable code
- None. All re-exports correspond to live modules; `validate_rate` has exactly two callers, both live.

## 8. Dependencies & flags
- `media_types` (Rate, time vocabulary, EngineError) only. No features, no unsafe at this level, no external crates. Release-lane clean (baseline cargo-build-release-lumen-cli.txt exit 0).

## 9. Open questions for the next agent
- clock.rs (the 700-line core): verify the "short internal lock, pure-arithmetic critical sections, never across I/O" claim line by line — this file's docs stake the crate's concurrency story on it.
- backends.rs: the multi-backend master-loss demotion chain (audio → video → wall) is documented here; whoever audits it should check the demotion events against the wire event vocabulary.
