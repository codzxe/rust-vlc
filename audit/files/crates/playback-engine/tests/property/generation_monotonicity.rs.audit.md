# AUDIT: crates/playback-engine/tests/property/generation_monotonicity.rs
| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-23 |
| Lines / functions | 495 / 8 (1 #[test]]; helpers: storm_timeline, generate_ops, op_kind, open_session, run, Live struct) + harness deps engine_fx.rs (465 loc) / rng.rs (191 loc, read for verification) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test crate) — runs under `cargo test -p playback-engine --test property`; I ran the prebuilt binary (see Commands). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `./target/debug/deps/property-25b39d1917495830 generation_monotonicity` → 1 passed, 0 failed, finished in 0.04s, exit 0 (64 seeds × 56 ops). Also `--list` confirmed 15 tests in the property binary. |

## 1. One-paragraph verdict
A real property suite with exact-oracle assertions, not a smoke pass: each of the 64 seeded storms (56 ops: pause/play/seek/select/deselect/stop/close over a 4-track timeline with a SEEKABLE scripted demuxer — unlike model_walk's unseekable fake) checks, after every op, the exact mint delta per receipt shape (`+1` for accepted seek/A-V switch/stop, `close_plan(state).len()` for completed closes, `0` for subtitle selects, play/pause, and every rejection), wire-generation non-rewind within the epoch, epoch ids exactly `n+1` per reopen, cross-epoch CancelToken staleness (equal pipeline counters included), typed `SessionClosed` refusals from dead handles, and the per-track delivered-frame generation floor. I executed it: passes in 0.04 s. The one coverage hole that matters: the storm can never reach the all-limbs-deselected state (audio can be deselected, video cannot), so the zero-limb seek wedge found in pipeline.rs (F-001) stays invisible to this suite.

## 2. What actually works (verified)
- `./target/debug/deps/property-25b39d1917495830 generation_monotonicity` → `test result: ok. 1 passed` — the full 64-seed bank.
- The mint-delta oracle is exact (not just monotone): a wrong mint (double-mint on a seek, a mint on a rejected play) fails with the seed+op in the message; the failure path also wires the documented shrinker (`run_case`, rng.rs:147-171, bisection to minimal failing prefix).
- The seek ops in the storm are REAL seeks: `build(tracks, slots, /* seekable */ true)` (run():159) arms `TimelineDemuxer::reanchor` (engine_fx.rs:112-133, compute-then-commit, keyframe-safe landing) — so seek mint + flush + pre-roll machinery actually executes under the storm, not just the rejection family.

## 3. Findings
F-001 [S3]
Where: crates/playback-engine/tests/property/generation_monotonicity.rs:97-117 (with pipeline.rs:1519-1521)
Code:
```
            5 | 6 => Op::SelectVideo,
            7 => Op::SelectAudio(2),
            8 => Op::SelectAudio(3),
            9 => Op::DeselectAudio,
```
What is wrong: the op vocabulary has `DeselectAudio` but no `DeselectVideo`, and `Op::SelectVideo` always selects track 1 — the video limb is permanently wired, so `Pipeline::limbs` can never become empty in this storm. The zero-limb state (`tracks.set-audio-track none` + `tracks.set-video-track none` → then seek) is therefore unreachable here, and the wedge it causes (pipeline.rs F-001: `seek_preroll` returns `NotReady` forever when `self.limbs.is_empty()`) passes this suite green. The suite's own header claims the storm covers "track-switch" storms comprehensively; the deselect-half of the switch surface is one track kind short. Why it matters: this is the engine's designated adversarial generation-storm suite; the one state that breaks the seek-completion law is the one state it cannot generate. What correct looks like: a `DeselectVideo` op (and/or `Op::SelectVideo(TrackSelector::None)`), which would expose the wedge as a per-seed failure (the seek would mint +1 and then never complete — caught either by the reopen/epoch bookkeeping or by a completion-timeout oracle). Confidence: HIGH — Verification: read + ran (the storm passes precisely because the wedge state is unreachable). UNVERIFIED-by-execution that adding the op would fail (I did not modify source — READ-ONLY rule).

F-002 [S4]
Where: crates/playback-engine/tests/property/generation_monotonicity.rs:279-281
Code:
```
        engine.poll();
        let outputs = engine.take_outputs();
        let generation_after = engine.generation().map_or(0, |g| g.0);
```
What is wrong (recorded as a sharp edge, not a defect): exactly ONE poll runs per op, so a seek's COMPLETION (Seeked terminal, pre-roll delivery, M10 hold release) is typically still in flight when the next op is submitted — the suite never asserts that a seeked session eventually answers its terminal under the storm (seek_roundtrip.rs owns completion on fixed scripts; the storm only checks the mint/token laws). The superseded-seek storm shape (doc 05 §4.2) is thus only half-stormed here. Documented division of labor (main.rs:14-21 lists seek_roundtrip as family 4), hence S4. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `storm_timeline` | 59-81 | REAL | — | 4 tracks (video/2 audio/subtitle), 320 steps, audio `dts < pts` B-lead shape — real adversarial stamps. |
| `Op` enum | 84-95 | REAL | S3 | F-001: no DeselectVideo — zero-limb state unreachable. |
| `generate_ops` | 97-117 | REAL | — | Weighted mix; seek is 7/16 of the mass. |
| `op_kind` | 119-130 | REAL | — | Op → SessionCommandKind mapping. |
| `Live` struct | 132-148 | REAL | — | Per-track delivered-generation floor map (correctly limb-scoped per card 3.A3 — matches pipeline.rs Limb::generation semantics I verified). |
| `open_session` | 151-201 | REAL | — | Exact epoch assert (`session_id != expected_session` fails), cross-epoch token staleness, fresh-pair currency. |
| `run` | 203-479 | REAL | S4 | The law engine: receipt-kind/mint-delta oracle (297-374), wire non-rewind (376-380), token staleness/refresh (382-400), per-track frame floor (402-427), close aftermath (dead handle typed refusal, reopen, re-prime). F-002 single-poll-per-op. |
| `generations_strictly_increase_and_supersede_forever_under_storms` | 486-495 | REAL | — | 64 seeds × 56 ops via `run_case` (shrinking wired). Executed: PASS. |
| (harness) engine_fx.rs: `TimelineDemuxer`, `ImmediateDecoder/Provider`, `Collector`, `build/open`, `landing_of`, `resolve_target_ns` | engine_fx.rs:77-465 | REAL | — | Seekable scripted demuxer with compute-then-commit reanchor; honest one-frame-per-packet decoder; recording collector with per-frame generation; the seek oracle mirrors the engine's clamp arithmetic and says so. |
| (harness) rng.rs: `XorShift`, `SEED_BANK`, `seeds`, `shrink_prefix`, `run_case` | rng.rs:25-171 | REAL | — | Deterministic seeded bank (first 64 odds), env-widening `LUMEN_PROP_SEEDS`, documented bisection shrinker, self-tested (rng.rs:179-191). |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| generations_strictly_increase_and_supersede_forever_under_storms | YES | Every law is an exact-oracle assert with seed+op context: an extra/missing mint, a rewound generation, a reused epoch id, a live-but-stale token, a dead handle accepting a command, or a stale frame delivering past a limb floor each fail the specific op. Mutation reasoning: making `seek_flush` skip the generation ratchet fails the `+1` mint law; making `teardown` not interrupt fails the dead-token law. The one mutation it CANNOT catch is the zero-limb seek wedge (F-001 — state unreachable). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "epochs strictly increase: every Engine::open mints a session id one beyond the last" | generation_monotonicity.rs:9-12 | REAL | open_session:167-172 exact assert; run():214/224/468. |
| "every mint is exactly the documented delta: +1 per accepted gen-bumping command (seek / A-V track switch / stop), close_plan(state).len() per completed close, 0 for class-S subtitle selects, play and pause, and every typed rejection" | 13-18 | REAL | run():297-374 — all five cases asserted with exact arithmetic. |
| "superseded pairs are stale forever … across epochs too (epoch N+1's pipeline 0 is not epoch N's pipeline 0)" | 19-23 | REAL | open_session:177-186 (cross-epoch, equal counters) + run():383-391 (post-mint staleness). |
| "per TRACK the delivered generation sequence never decreases" | 24-34 | REAL | run():402-427 per-track floor map; correctly modeled as limb-scoped (matches pipeline.rs Limb::generation floor — verified against production code in my pipeline.rs audit). |
| (harness) "proptest is not a workspace dependency … replaced by the documented bisection" | rng.rs:4-20 | REAL | Hand-rolled XorShift + shrink_prefix, self-tested at rng.rs:179-191. |

## 7. Dead / duplicated / unreachable code
- None dead in this file. Known duplication (documented in rng.rs:176-178): the property harness is duplicated across the playback-engine and packet-queue test crates — acceptable, cross-crate test deps are not a thing without a shared crate.
- No `#[ignore]`, no `todo!`.

## 8. Dependencies & flags
- Test-only: `playback_engine` (public API only — black-box as documented), `media_types`, `commands`, `testkit` + sibling modules engine_fx/rng. No external deps. Runs in the default test lane; env vars `LUMEN_PROP_SEEDS`/`LUMEN_PROP_SHRINK` are opt-in widening (not read by the default run — no env dependency in CI).

## 9. Open questions for the next agent
- seek_roundtrip.rs (845 loc, family 4) — same question: does its supersession storm ever deselect BOTH A/V kinds before a seek? If not, pipeline.rs F-001 remains fully uncovered by the property suites.
- The `Live.delivered_by_track` floor starts at 0 (`unwrap_or(0)`): if `GenerationId::FIRST` ever became nonzero-counter, a first frame of generation 0 would still pass — harmless today (FIRST is the initial counter; verified used as baseline in pipeline.rs:1007).
