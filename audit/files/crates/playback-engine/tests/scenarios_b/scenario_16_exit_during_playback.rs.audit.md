# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_16_exit_during_playback.rs
| Field | Value |
|---|---|
| Auditor | W03-A07 |
| Date | 2026-11-23 |
| Lines / functions | 238 / 4 (1 const + 1 helper + 2 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test-only scenario, but it drives the production stack: real `playback_engine::Engine`, real `input` access + real `demux` AVI + real `codecs` MJPEG/PCM over the real generated fixture (tests/fixtures/generated/valid/avi_mjpeg_pcm.avi, MANIFEST-verified, sha256-pinned), via the scenarios_b host's LOCAL command-layer shape (the production `lumen_cli::dispatch::lumen_dispatch` is unreachable from this crate — dev-edge cycle, honestly documented at host.rs:6-25). The consumer seam is the named fake `ObservingSink`; the demux stage is wrapped by a counting forwarder for the drop oracle. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Direct run `./target/debug/deps/scenarios_b-bee4b59599013918 scenario_16` -> 2 passed / 0 failed (exit 0, 0.02s); baseline audit/runs/baseline/cargo-test-workspace.txt:4410-4411 (same 2 ok); sed/rg verifications incl. host.rs DropHooks/arm_drop_hooks/open_fds/thread_count/TempDir, fixture structure.json, first-vertical baseline failures. |

## 1. One-paragraph verdict
An honest, well-constructed teardown scenario: the "abrupt exit" is disclosed up front as the in-process shape (drop the whole host mid-playback, no stop, no close — "in the real product the process exits and the OS reclaims; here the reclaim is Rust's drop glue", lines 7-13), and every pass criterion from doc 10 §6.4 row 16 is measured with a real instrument: drop-hook counters on the actual stage objects the engine owns (demuxer_drops == 1, sink_drops == 1), the /proc fd count returning to baseline immediately after the drop, a bounded 5 s thread-count wait, a scratch-dir listing proving zero sidecar/journal/cache writes, and a wall-clock teardown bound of 2 s measured around the drop alone (the thread wait is correctly OUTSIDE the deadline window). The mid-playback precondition is verified, not assumed — frames accepted ∈ (30, 120) against a fixture that is exactly 120 frames (60 MJPEG video + 60 PCM audio chunks, verified in the fixture's structure.json: chunk_count 120), so a fully-drained (Ended) session would fail the assert. The second test repeats the drop on a synthesized timeline:// source proving the memory-source path. Both tests pass in my direct run. Two S4s: the file-source test hard-unwraps the fd baseline (panics rather than skips on non-/proc platforms, while its sibling test degrades gracefully), and the "exit code clean" criterion's process-level arm is delegated to the first-vertical suite, which is currently RED in the orchestrator baseline (2 failing evidence tests).

## 2. What actually works (verified)
— `./target/debug/deps/scenarios_b-bee4b59599013918 scenario_16` -> `test result: ok. 2 passed; 0 failed` (exit 0). Verified by read: (1) DropHooks are armed BEFORE the open (88) and wrap every session's stage objects (host.rs:1078-1085 "consulted at wiring time so every session's stage objects are counted"); (2) the fd assertion ordering is correct — the fixture copy (82) happens before the baseline read (84), so the copy's own fd cannot pollute it; (3) the teardown deadline (114-116, 167-170) brackets only `drop(host)`, while the 5 s thread wait (142-153) runs after — the wait cannot mask a slow teardown; (4) the (30..120) mid-playback bound is sound against this fixture (structure.json: chunk_count 120 = 60 `00dc` + 60 `01wb`; full drain would present 120 and fail the `< 120` arm); (5) the no-temp-state oracle compares the sorted directory listing before/after AND against the exact expected single file (156-165); (6) `timeline://14400x1000000` is a real on-demand demux (host.rs:1530-1549), and test 2 proves a memory source never moves the fd baseline (224-228) and writes nothing (229-233).

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_16_exit_during_playback.rs:105-110
Code:
```
    assert_eq!(
        open_fds(),
        Some(fds_before.must() + 1),
        "the playing session holds one fd"
    );
```
What is wrong: `open_fds()` returns `Option<usize>` (None off `/proc`), and this test hard-unwraps the baseline with `.must()` — on a non-Linux host the test PANICS ("unwrap on None") instead of skipping, while its sibling test in the same file (224-228) compares `Option == Option` and degrades to a vacuous pass on the same platform. Two different platform disciplines in one file; neither is "skip with a printed note" (the vm_rss precedent in tests/soak/memory.rs:344-355). Blast radius: CI is Linux, so this is latent portability debt only. What correct looks like: match on `(open_fds(), fds_before)` and eprintln-skip when None, consistently in both tests. Confidence: HIGH — Verification: read.
Confidence: HIGH — Verification: read

F-002 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_16_exit_during_playback.rs:32-35
Code:
```
//! - **exit code clean** — the in-process arm of this criterion is
//!   the test binary itself exiting 0; the process-level arm (real
//!   CLI exit mid-playback) is the first-vertical suite's evidence
//!   (W5-07 scenario 5's stop/teardown and the CLI's own suites).
```
What is wrong: the delegated process-level evidence suite is currently failing in the orchestrator baseline: first-vertical reports `committed_transcripts_match_fresh_runs` and `final_frame_is_current` FAILED (audit/runs/baseline/cargo-test-workspace.txt:2771-2789 — "the committed transcript for `open` drifted from a fresh run"). The cited W5-07 scenario-5 stop/teardown test itself is not among the failures, so the specific delegation still holds, but the evidence crate's committed artifacts are stale, and a reader following this pointer lands in a red suite. What correct looks like: regenerate the first-vertical transcripts (its own error message says `--ignored regen`) so the delegated evidence chain is green. Confidence: HIGH — Verification: read + baseline citation.
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| TEARDOWN_DEADLINE (const) | 54-55 | REAL | — | doc 03 §4.6 S2's 2 s, generous by design |
| open_file | 57-72 | REAL | — | command-layer open + settle + Playing assert |
| abrupt_drop_mid_playback_releases_everything_within_deadline | 78-171 | REAL | S4 | all five oracles measured; fd baseline hard-unwrap (F-001) |
| abrupt_drop_mid_timeline_releases_stage_objects | 177-238 | REAL | — | memory-source arm; graceful Option compare; nothing written |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| abrupt_drop_mid_playback... | YES | a leaked demux/sink stage, fd, thread, or a temp-file write, or teardown > 2 s each fail a named assert; a regression to orderly-close-only would also fail (no stop/close is issued — drop glue must do all the work) |
| abrupt_drop_mid_timeline... | YES | same drop-hook/fd/no-write/bounded asserts for the synthesized source |
| (disclosed gaps) | — | drop during Opening/Buffering is named NOT-tested (37-42) with the reason (synchronous open leaves no deterministic transient); process-level exit delegated (F-002) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every pipeline stage object the engine owned is observed to drop" | scenario_16:15-17 | REAL | DropHooks armed pre-open, asserted == 1 each (120-131); my run ok |
| "the open FileAccess fd count returns to baseline immediately after the drop" | scenario_16:21-23 | REAL | /proc/self/fd count assert (133-137); immediate (before the thread wait) |
| "no temp state written ... no sidecars, no journals, no caches" | scenario_16:24-27 | REAL | sorted-dir equality + exact single-entry assert (156-165) |
| "teardown within deadline" (2 s) | scenario_16:28-31 | REAL | Instant-bracketed drop (114-116, 167-170); measured 0.02s for both tests in my run |
| "the process-level arm ... is the first-vertical suite's evidence" | scenario_16:32-35 | REAL-UNPROVEN (delegation target currently red adjacent) | first-vertical baseline: 2 evidence tests FAILED (F-002); the cited scenario-5 test is not among them |
| "the engine is poll-driven and threadless" | scenario_16:19-20 | REAL | consistent with session.rs (single-writer poll) and the CLI host docs |

## 7. Dead / duplicated / unreachable code
None. No #[ignore], no todo!/unimplemented! (rg verified). The `demuxer_dropped`/`sink_dropped` helper methods on DropHooks (host.rs:332-342) are unused by this scenario (it loads the counters directly) — dead-in-this-consumer, other scenarios may use them (host.rs is shared; not this file's defect).

## 8. Dependencies & flags
commands catalog (WireRequest/Command/WireResponse vocabulary), testkit (Must, serialize_tests, fixture_path), the local host (crate::host) over the real engine/demux/codecs. `serialize_tests` held for both tests (global counting allocator in the binary). Linux-leaning instruments with one inconsistent degradation (F-001). No feature gates.

## 9. Open questions for the next agent
- host.rs (2307 loc, not my file): confirm `pump_pipeline_until_within`'s bound of 256 pumps is derived from the fixture's frame budget, and whether `settle()` can land in `Ended` for this 2 s fixture (making "mid-playback" depend solely on the frames bound — my analysis says the bound is sound either way).
- The first-vertical transcript drift (baseline failure) — which agent owns regenerating it? It weakens two delegations I found (this file's F-002; also relevant to any doc citing first-vertical evidence).
