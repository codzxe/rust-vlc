# AUDIT: crates/playback-engine/tests/property/generation_monotonicity.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 496 / 1 `#[test]` fn + 8 harness fns/types |
| File verdict | REAL-UNPROVEN (a genuine model-checked property suite over the real engine) |
| Reachable from shipped entry point? | N/A (test file). The laws it checks (epoch minting, generation deltas, token staleness) are exercised by the shipped engine on every session (state.rs/epoch.rs/session.rs — my state.rs audit chain). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |
| Commands run | full-file read; read of the shared harness (tests/property/rng.rs: seeds/run_case/shrink_prefix + self-test); grep `#[test]` → 1; cross-check of mint-delta oracle against state.rs's `close_plan` and class-A/S taxonomy (my state.rs audit) |

## 1. One-paragraph verdict

The strongest test file in my assignment: a seeded command-storm property (64 seeds × 56 ops, up to 7 sessions per case) over the **real Engine**, checking doc 03 §5's cancellation laws after every single op with an exact oracle — epochs strictly increase (minted session id must equal expected), every accepted class-A command mints exactly `+1` (seek/video-switch/audio-switch/stop), play/pause mint `0`, close mints exactly `close_plan(state).len()`, class-S subtitle selects mint `0`, every typed rejection mints `0`, the wire generation never rewinds, pre-mint token pairs are stale forever (cross-epoch too), dead handles refuse typed `SessionClosed`, and the delivered-frame generation floor holds per track (the limb-scoped 3.A3 law, correctly modeled as per-track rather than global — the comment at 138-146 explains why). Failures shrink to a minimal failing prefix via the harness's bisection (`LUMEN_PROP_SHRINK`), with a rerun seed printed — reproducible-triage discipline. The oracle is derived from the same table the shipped engine runs (state.rs), so a mutation in the mint logic, the close composition, or the token staleness would fail here with a precise step label.

## 2. What was checked and how (zero-findings file, §13 requirement)

- The oracle (310-322, 341-357) cross-checked against state.rs: `close_plan` returns stop-then-close (2 inputs) from active states and close-only (1) from settled ones — the per-state delta the test computes is the table's own composition. Class-A/S taxonomy matches `SessionInput::is_command` semantics and the receipt contract.
- The per-track floor law (402-427): `generation > generation_after` fails delivery above the wire generation; per-track rewind fails — both asserted per delivered frame per op, not post-hoc.
- Mutation reasoning (UNVERIFIED by execution): an off-by-one in any mint path fails the exact-delta assert (323-329); a stale pair resurrection fails 385-390; a straggler frame crossing a flush fails 409-425; a dead-handle acceptance fails 440-448. Each failure message carries seed, op index, and the op — the shrink harness then bisects.
- The storm timeline (59-81) includes B-lead audio (`dts < pts`) and two audio tracks + subtitle — the switch-storm vocabulary is real.
- W00 baseline: the playback-engine suite ran green (this file included) in the 1751-pass run — cited from baseline cargo-test.log; not re-run by me (ADAPTATION).
- Harness shared modules (rng.rs, engine_fx.rs) read for the parts this file depends on: `seeds()` = bank + `LUMEN_PROP_SEEDS` env widening; `run_case` wraps with reproduce/shrink messaging; `build()` constructs a real pipeline config.

## 3. Per-function table

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (the four laws) | 1-34 | REAL-UNPROVEN | — | matches the asserts below |
| `MS`/`STEPS`/`REOPENS` consts | 49-53 | REAL-UNPROVEN | — | 320 steps, ≤6 reopens/case |
| `storm_timeline` | 59-81 | REAL-UNPROVEN | — | 4 tracks, B-lead audio, alternating audio |
| `Op` enum + `generate_ops` | 84-117 | REAL-UNPROVEN | — | seek-heavy distribution (deliberate) |
| `op_kind` | 119-130 | REAL-UNPROVEN | — | — |
| `Live` (struct) | 133-148 | REAL-UNPROVEN | — | per-track delivered floor map |
| `open_session` | 151-201 | REAL-UNPROVEN | — | epoch law + cross-epoch staleness + fresh-pair currency |
| `run` | 203-479 | REAL-UNPROVEN | — | the storm body: receipt oracle, token laws, frame floor, close aftermath + reopen |
| `generations_strictly_increase_and_supersede_forever_under_storms` | 486-495 | REAL-UNPROVEN | — | 64 seeds via `run_case` |

## 4. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `generations_strictly_increase…` (486-495) | YES | exact-delta oracles per command class; per-frame floor; token staleness; dead-handle refusal — ~10 independent law families, each with a labeled failure |

## 5. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "epoch strictly increase … generations never rewind … superseded pairs stale forever … stale packets never carry a current generation" | module docs 9-34 | REAL-UNPROVEN | the four law families implemented as asserts (167-172, 376-380, 383-400, 402-427) |
| "per TRACK the delivered generation sequence never decreases … a seek mint bumps every limb's floor" | module docs 26-34 | REAL-UNPROVEN | per-track map at 146, 418-426; the limb-scoped rationale documented |

## 6. Dead / duplicated / unreachable code

None in this file. (The harness itself is duplicated between playback-engine and packet-queue property suites — noted in rng.rs's own comment, "the harness is duplicated across the two test crates"; rng.rs is not my file.)

## 7. Dependencies & flags

`playback_engine`, `media_types`, `commands`; shared `super::engine_fx` (fixture builder) and `super::rng` (seed bank + shrink). Env vars `LUMEN_PROP_SEEDS` / `LUMEN_PROP_SHRINK` widen/triage without code changes.

## 8. Open questions for the next agent

- `tests/property/engine_fx.rs` (the `build`/`Collector`/`Delivered` fixture) and `rng.rs` (the duplicated harness) — other agents; the fixture's realism bounds this suite's meaning.
- The sibling property families (`seek_roundtrip.rs`, `queue_invariants.rs`, `timestamp_arithmetic.rs`, `main.rs`) — not in my assignment; same harness, presumably same quality bar.
