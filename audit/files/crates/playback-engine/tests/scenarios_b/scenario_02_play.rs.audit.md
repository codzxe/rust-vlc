# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_02_play.rs
| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-23 |
| Lines / functions | 98 / 1 test fn + 1 const |
| File verdict | REAL (a genuinely end-to-end play-through oracle over the real engine, real AVI/MJPEG/PCM stack, real wire vocabulary) |
| Reachable from shipped entry point? | NO — test binary only (`cargo test -p playback-engine --test scenarios_b`); the ENGINE it drives is shipped code (session.rs reachable YES via cli host.rs:91/448), but this exact composition (demux+codecs+input into playback-engine) exists only on dev edges (host.rs.audit.md, boundaries.toml:294-303) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `~/.cargo/bin/cargo test -p playback-engine --test scenarios_b scenario_02` -> **1 passed; 0 failed; 0 ignored** (finished 0.12s, compile 8.19s, exit 0) — EXECUTED THIS RUN; baseline audit/runs/baseline/cargo-test-workspace.txt:4391 same test `... ok` (suite 23 passed in 5.06s); rg/sed reads of host.rs, fixtures_parent.rs |

## 1. One-paragraph verdict
This is an honest, strong test. It opens the real shared fixture (`generated/valid/avi_mjpeg_pcm.avi`, the same PARENT used by demux/codecs/audio-output tests) through the real `Engine` (`pump_pipeline()` is literally `engine.poll()` + output drain, host.rs:1107-1110), plays to the end with a bounded 512-pump deadline, and asserts an EXACT count oracle (120 frames = 60 audio + 60 video, matching the fixture's declared pts streams pinned at crates/demux/tests/fixtures_parent.rs:176-177), a first-frame pts==0 pin, exactly-one `playback.end-of-media`, a zero-error lane, an end-of-media A/V drift bound, and a clean close. Every mutation I reasoned through (engine stalls, dropped/duplicated frames, swallowed errors, duplicated EOM, A/V divergence) trips a specific assert; nothing is mocked on the engine path. The three caveats are scope-level, all disclosed or inherited: the "command layer" is the suite's LOCAL dispatch mirror (host.rs:4-39, ARCH-5 note — dev-edge cycle makes the real `lumen_cli::dispatch` unreachable from this crate), the EOM-vs-Ended ordering claim in the doc header is not actually asserted (count only), and A/V stamp agreement is checked only at the end-of-media checkpoint (intermediate checkpoints belong to scenario_07, which exists and passes).

## 2. What actually works (verified)
- EXECUTED: `~/.cargo/bin/cargo test -p playback-engine --test scenarios_b scenario_02` -> `test scenario_02_play::play_flows_first_frame_samples_and_honest_stamps ... ok` / `1 passed; 0 failed; 0 ignored; 22 filtered out; finished in 0.12s` (exit 0). Baseline cargo-test-workspace.txt:4391 agrees (suite: 23 passed, 5.06s).
- Real engine chain, verified by read: `ScenarioHost { engine: Engine }` (host.rs:1019, `Engine::new()` at 1060) -> `dispatch()` (host.rs:1988) -> local `lumen_dispatch` over the real `commands` catalog types -> `pump_pipeline()` = `self.engine.poll()` (host.rs:1107-1110) -> real `session.rs` executor -> real demux AVI + MJPEG/PCM codecs (dev-edge composition, disclosed host.rs:20-23) -> `impl FrameConsumer for ObservingSink::accept` (host.rs:632-672) counting both limbs into atomics and pushing `(track, is-audio, pts-ns)` into the bounded ring.
- The count oracle is externally pinned, not self-referential: the PARENT fixture's streams are independently asserted in crates/demux/tests/fixtures_parent.rs:176-177 — `assert_eq!(video_pts, (0..60).collect::<Vec<i64>>());` / `let audio_want: Vec<i64> = (0..60).map(|i| i * 1470).collect();` — so "60 + 60 = 120" is the fixture's truth, not the engine's own report echoed back.
- Bounded liveness with a diagnosable failure: `run_until_terminal(512)` (host.rs:1156-1167) returns the ACTUAL state at bound exhaustion, so a stalled engine fails the `assert_eq!(terminal, PlaybackState::Ended, "the parent ends cleanly ({pumps} pumps)")` with the pump count, never a hang.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_02_play.rs:10 and 64-68
Code (L10):
```
//! and exactly one `playback.end-of-media` precedes `Ended`.
```
Code (L64-68):
```
    assert_eq!(
        count_events(&host, "playback.end-of-media"),
        1,
        "exactly one end-of-media"
    );
```
What is wrong: The doc header claims ordering ("precedes Ended"); the body asserts only the COUNT of end-of-media events plus that the terminal state is Ended. If the engine ever emitted the state-changed-to-Ended wire event BEFORE the end-of-media event, this test would still pass — the causal order the header promises is unverified here (the harness records events in causal order, host.rs "pub fn events() ... (causal order)", so the assert was one `.position()` away). Confidence: HIGH — Verification: read both files.
F-002 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_02_play.rs:60-94
Code (L61-63):
```
    let sink = host.sink().must();
    let frames = sink.frames_accepted();
    assert_eq!(frames, 120, "every declared frame delivered ({frames})");
```
What is wrong: The oracle pins totals (120 both-limb, 60 audio) and the two LAST stamps' distance, but not per-limb sequence integrity: a duplicated video frame plus a dropped video frame (or an out-of-order accept) keeps 120/60 and can keep the end-drift within tolerance, and nothing here would notice — the ring is only queried for `.first()` and the two last-by-scan entries. Cross-checked: the stale-frame/monotonicity/pts-order pins live in scenario_04/06/07 (all exist, all pass in baseline), so this is a division-of-labor gap inside row 2's own scope, not a suite-wide hole. Confidence: HIGH — Verification: read; scenario_04/07 baseline lines cited in log.
F-003 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_02_play.rs:4-5
Code:
```
//! The parent plays end-to-end through the command layer: the first
//! delivered frame carries pts 0 (the fixture's `[first_frame]` stamp),
```
What is wrong: "through the command layer" is true only of the suite's LOCAL command-layer mirror: the shipped router `lumen_cli::dispatch::lumen_dispatch` is not compiled into this test (host.rs:4-39 discloses why: a dev-dependency on lumen-cli would close a dev-edge cycle boundaries.toml rejects, and the card pins the suite's location; the wire TYPES are the production `commands` catalog, the ROUTER is local and diverges from the shipped CLI — W03-A04's host.rs audit flags those divergences S2/S3). The engine/pipeline evidence in this file is nonetheless fully real. A reader of this file alone would not know the router is not the shipped one. Confidence: HIGH — Verification: read host.rs:4-39 + W03-A04 host.rs.audit.md.

## 4. Per-function table (file is 98 LOC; inventory included for completeness)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| const FRAME_NS | 20-21 | REAL | — | 33_333_400 ns; exact 1/30 s is 33_333_333.3 — the constant is the doc's 33.4 ms rounding, 66.7 ns looser than "one frame duration"; self-described as 33.4 ms, honest |
| play_flows_first_frame_samples_and_honest_stamps | 23-98 | REAL (executed) | S4 x3 | 10 distinct asserts; exact-count oracle; failure modes enumerated in §5 |

## 5. Test honesty (mutation reasoning, no edits made)
| Mutation (code broken how?) | Would the test fail? | Which assert |
|---|---|---|
| Engine never advances past Buffering | YES | L41 `assert_eq!(host.state(), PlaybackState::Playing)` (settle is bounded by AWAIT_PUMPS=64, host.rs:133) |
| No frame ever decoded/delivered | YES | L44-50 `.first()` -> `unreachable!("the first frame landed at settle")` panic |
| First frame pts nonzero | YES | L51 `assert_eq!(first.2, 0, ...)` |
| Pipeline stalls mid-media | YES | L54-59 `run_until_terminal(512)` returns the real non-terminal state -> `assert_eq!(terminal, Ended, "... ({pumps} pumps)")` |
| Frame dropped or duplicated | YES | L62 `frames == 120` (exact; dup+drop SWAP within one limb escapes — F-002) |
| Audio limb short | YES | L63 `audio_accepted == 60` |
| end-of-media never/duplicated | YES | L64-68 count == 1 (ORDER not checked — F-001) |
| Error swallowed into an event | YES | L69-73 `errors.is_empty()` over the recorded wire `playback.error` lane |
| A/V stamps diverge > one frame at EOM | YES | L90-94 drift bound |
| Close rejected | YES | L96-97 `close.result.is_ok()` |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "first-frame event + samples flow; A/V stamps within tolerance" (doc 10 §6.4 row 2) | scenario_02_play.rs:1-2 | REAL | executed: 1 passed 0.12s; asserts L41-L94 |
| "first delivered frame carries pts 0 (the fixture's `[first_frame]` stamp)" | :5-6 | REAL | L51 asserts pts==0; fixture pinned pts 0..60 (fixtures_parent.rs:176) |
| "samples flow to the full declared counts (60 audio + 60 video = 120 frames)" | :6-7 | REAL | L62-63; declared counts independently pinned at fixtures_parent.rs:176-177 |
| "exactly one playback.end-of-media precedes Ended" | :10 | SHALLOW (count half real; "precedes" unasserted) | F-001 |
| "A/V stamp distance at the end stays within one frame duration" | :7-9 | REAL (end checkpoint only, disclosed) | L90-94; intermediate checkpoints = scenario_07 (`scenario_07_av_sync::full_playback_keeps_av_stamps_within_tolerance_at_checkpoints ... ok`, baseline) |

## 7. Dead / duplicated / unreachable code
None in this file. Context: the harness helper `ObservingSink::last_pts_ns_of_track` is `#[allow(dead_code)]` (host.rs, the gate doc §2 row 4c hit — W03-A04's audit), unused by this scenario as designed.

## 8. Dependencies & flags
Test-binary only: `commands` (catalog types — the production wire vocabulary), `testkit` (Must, fixture_path, serialize_tests — the counting-allocator serialization guard), `crate::host` (the audited local harness). No feature gates; runs in the default test lane (baseline: not ignored).

## 9. Open questions for the next agent
- Whether the wire event stream ever orders `session.state-changed(Ended)` before `playback.end-of-media` in practice (F-001) — a one-line `.position()` assert in this scenario or a contract test in events_contract.rs would pin it.
- The 512-pump bound passed at 0.12 s; the actual pumps spent are printed only on failure — a passing run does not record the headroom, so a slow creep toward the bound across future cards would be invisible until it flakes. UNVERIFIED: actual pump count on the passing run (not observable without editing the test).
