# AUDIT: crates/playback-engine/src/seek.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 546 / 14 production items + 6 `#[test]` fns |
| File verdict | REAL-UNPROVEN (a genuine seven-step seek flow, not a reopen hack) |
| Reachable from shipped entry point? | **YES** — `lumen seek …` → `lumen_dispatch` → `CliHost::dispatch_seek` (apps/cli/src/host.rs:1058) → `SessionHandle::seek` (crates/playback-engine/src/session.rs:368) → session executor `seek_pipeline_step` (session.rs:1985, which calls this module's `resolve_target`, `pipeline_seek`, `complete_seek`) → `Pipeline::seek_flush` (pipeline.rs:1239) + `Pipeline::demuxer_reanchor` (pipeline.rs:1498) → the AVI demuxer's real `reanchor` (crates/demux/src/avi/mod.rs:971, with D4 tests at avi/tests.rs:2695-2877). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | rg call-chain hops (`dispatch_seek` → `SessionHandle::seek` → `seek_pipeline_step` → `pipeline_seek`); rg for demuxer `reanchor` implementations; sed quote verification; `grep -c "#\[test\]"` → 6 |

## 1. One-paragraph verdict

The hostile question — "is seek real seeking or a reopen-hack?" — answers: **real seeking.** This module implements card 3.A1's seven steps as claimed: typed target resolution with clamping-is-truth semantics, capability check (`SeekSupport::None` → `SeekFailed`, never a fake success), generation-minted pipeline flush, demuxer re-anchor with compute-then-commit D4 rollback, an in-flight seek slot with supersession semantics, clock re-anchor on the *first audio frame's own timestamp* (anchor on truth), and a `Seeked{requested, actual}` terminal. The CLI's seek verb drives this natively (`dispatch_seek` explicitly: "never a fake success, never a reopen"), and the AVI demuxer has a real `reanchor` underneath. The defects are peripheral: the CLI's own help text still describes the retired reopen implementation ("seek (v0: reopen at the target…)"), `Chapter` targets are a typed refusal (honest v0), and the in-file tests cover only target math — the flow's behavioral proof lives in `tests/seek_flow.rs` (my file #24) and the scenario suites.

## 2. What actually works (verified)

- The full call chain above, read hop by hop. Every function this module promises (`pipeline_seek` → `seek_flush`/`demuxer_reanchor`; `complete_seek` → `clock.reanchor` + `set_generation`) exists at the cited sites.
- The D4 (failed-seek rollback) design is coherent across three files I audited independently: seek.rs:82-95 (contract), state.rs's `SeekPipeline` effect (state.rs:341-352), and the AVI demuxer's own `reanchor_failure_leaves_the_demuxer_at_its_position_d4` test (demux/src/avi/tests.rs:2877).
- M10 pre-roll hold interlocks correctly with the video scheduler (my scheduler.rs audit: the scheduler's generation gate + `take_while(p.frame.generation <= published)` is the consumer side of `clock.set_generation` at seek.rs:447).
- W00 baseline shows the AVI vertical playing to Ended (advance-test.txt) — the seek path shares that pipeline; seek itself was not exercised in the baseline transcripts I read (mp4-smoke/advance-test), so run-proof of seek is UNVERIFIED from baseline; see my seek_flow.rs audit for the test-side proof.

## 3. Findings

### F-001 [S3] — The CLI help text misdescribes the implementation: "seek (v0: reopen at the target…)" is the retired hack, the shipped code is a native seek

Where: apps/cli/src/help.rs:57 vs the actual flow at apps/cli/src/host.rs:1053-1056 and this file.

Code (apps/cli/src/help.rs:55-58, verbatim):
```rust
        "seek 90s | +10s | -1m | 50% | chapter 3",
        "playback.seek",
        "seek (v0: reopen at the target through the engine's real transitions)",
    ),
```

Code (apps/cli/src/host.rs:1055-1057, verbatim):
```rust
    /// typed failures (unseekable source, unresolvable target,
    /// reanchor refusal) are NOT errors here: they answer the class-A
    /// accept and surface as the `timing.seek-failed` event with the
```

What is wrong: the help string (user-facing, shown by `lumen seek --help` / `--help`, and quoted in the W00 baseline lumen-help.txt) says v0 seek is "reopen at the target" — a session destroy-and-reopen. The implementation (`dispatch_seek` → `SessionHandle::seek` → seven-step native flow, whose module docs say "never a fake success, never a reopen") is a real in-session seek with generation mint, pipeline flush and demuxer re-anchor. The help text is stale from the pre-3.A1 era (2.D2's reopen) and undersells the shipped feature; any user or auditor reading `--help` concludes seek is a hack when it is not.

Why it matters: §3.7 doc-vs-reality — the project's own CLI lies about its best-engineered feature. It also seeded this audit's hostile question; the answer required tracing five files to refute one help line.

What correct looks like: "seek … (in-session seek: flush + re-anchor at the nearest keyframe, per doc 03 §6)".

Confidence: HIGH — Verification: read (help.rs + host.rs + session.rs + this file; the reopen vocabulary appears nowhere in the seek path).

### F-002 [S4] — In-file tests cover only the pure target math; no in-file test touches `pipeline_seek`/`complete_seek`

Where: crates/playback-engine/src/seek.rs:452-545.

What is wrong: all 6 tests exercise `resolve_target`, `target_span`/`span_start_ns`, and `SeekFailure` clone/eq. The two functions that carry the actual flow (`pipeline_seek` 369-412, `complete_seek` 434-450) have no in-file test; their proof is entirely in `tests/seek_flow.rs` (2023 LOC — my file #24, audited separately) and the scenarios. Acceptable layering, but the module's own safety net is thin if the integration tests rot.

Confidence: HIGH — Verification: read + test enumeration.

### F-003 [S4] — `Percent` resolution carries f32 fraction → f64 product rounding; a 0.1 seek on a multi-hour file can land a few ns off the exact 10% mark

Where: crates/playback-engine/src/seek.rs:291-294.

Code (291-294, verbatim):
```rust
        SeekTarget::Percent { fraction } => match duration_ns {
            Some(duration) => ResolvedSeek::Position {
                ns: clamp((f64::from(fraction.clamp(0.0, 1.0)) * duration as f64).round() as i64),
            },
```

What is wrong: `fraction` is `f32` end-to-end (the wire type); `0.1f32` is 0.100000001490116… in f64, so `10%` of a 3-hour file (10.8e12 ns) lands ~16 µs past the mark. The `Seeked{actual}` event reports the demuxer landing anyway (keyframe-quantized), so the error is invisible in practice. Hygiene: accept f64 at the wire or document the tolerance.

Confidence: HIGH — Verification: read + hand arithmetic.

## 4. Per-function table

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (7 steps, D4, supersession, paused carve-out, v0 limits) | 1-144 | REAL-UNPROVEN | — | the flow described matches the code I traced |
| `SeekTarget` (enum) | 156-182 | REAL-UNPROVEN | — | 4 variants; Chapter refuses typed (v0, honest) |
| `ResolvedSeek` (enum) | 187-196 | REAL-UNPROVEN | — | Position/Failure |
| `SeekFailure` (enum) | 205-219 | REAL-UNPROVEN | — | NotSeekable/OutOfRange/Demux/Clock |
| `Clone for SeekFailure` | 221-233 | REAL-UNPROVEN | — | shallow rebuild (event doctrine) |
| `PartialEq for SeekFailure` | 235-247 | REAL-UNPROVEN | — | stage+kind+code equality, chains excluded |
| `shallow` (free fn) | 250-258 | REAL-UNPROVEN | — | — |
| `not_seekable` | 264-266 | REAL-UNPROVEN | — | engine-side policy fact |
| `resolve_target` | 273-304 | REAL-UNPROVEN | S4 | F-003 f32 percent; clamping semantics tested |
| `target_span` | 308-313 | REAL-UNPROVEN | — | open-ended span |
| `span_start_ns` | 318-324 | REAL-UNPROVEN | — | None on missing stamp → typed failure, never guessed |
| `nanos_i64` | 330-332 | REAL-UNPROVEN | — | saturating, never wraps |
| `InFlightSeek` (struct) | 337-348 | REAL-UNPROVEN | — | target/landing/generation/origin |
| `SeekOutcome` (struct) | 353-362 | REAL-UNPROVEN | — | inflight/failure/stale count |
| `pipeline_seek` | 369-412 | REAL-UNPROVEN | — | steps 3-4; flush-failure counted-and-reported; D4 on reanchor refusal; landing-missing typed |
| `landing_missing` | 416-424 | REAL-UNPROVEN | — | contract-violation stand-in |
| `complete_seek` | 434-450 | REAL-UNPROVEN | — | step 6-7; audio-frame truth anchor; `set_generation` publishes to scheduler; clock failure typed |
| tests (6) | 456-544 | SHALLOW (unit-only) | S4 | F-002 |

## 5. Test honesty (6 `#[test]`)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `absolute_clamps_to_the_duration_when_known` (457-485) | YES | clamp semantics incl. negative → 0, unknown-duration backstop |
| `relative_resolves_against_the_position_then_clamps` (488-501) | YES | saturating add + clamp |
| `percent_needs_a_known_duration` (504-522) | YES | typed OOR + fraction clamping both signs |
| `chapter_refuses_typed_until_a_chapter_source_exists` (525-528) | YES | refusal pinned |
| `target_span_and_back_is_lossless_in_nano_base` (531-536) | YES | round-trip + missing-stamp None |
| `failure_clones_and_compares_without_source_chains` (539-544) | YES | shallow-eq semantics |

None test the flush/re-anchor/rollback flow itself — that is `tests/seek_flow.rs`'s job (my later file).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "seek (v0: reopen at the target through the engine's real transitions)" | apps/cli/src/help.rs:57 (also W00 baseline lumen-help.txt) | FALSE | F-001: native in-session seek implemented and wired |
| "never a fake success, never a reopen" | apps/cli/src/host.rs:1056-1057 | REAL-UNPROVEN | matches code; not run by me |
| Card 3.A1 seven-step flow | module docs 5-65 | REAL-UNPROVEN | every step traced to a real function |
| "Chapter targets refuse typed … hosts resolve chapters to Absolute" | seek.rs:132-134, 176-178 | REAL | resolve_target:297-302 + CLI NotFound at host.rs (dispatch_seek) |
| "clamping is truth, not failure" (doc 05 §2.4) | seek.rs:158-159 | REAL-UNPROVEN | resolve_target semantics + tests |

## 7. Dead / duplicated / unreachable code

None found. All pub(crate) items consumed by session.rs; `not_seekable`/`SeekTarget` re-exported through lib.rs for the host.

## 8. Dependencies & flags

Depends on `commands` (RequestId), `media_clock` (PlaybackClock/NANO_BASE), `media-types` (TimeSpan/Stamp/GenerationId). No feature gates. Always compiled.

## 9. Open questions for the next agent

- Is the MP4/MOV demuxer's `reanchor` real (keyframe-index seek) or does it refuse `SeekSupport::None`? That decides whether seek works on the litmus MP4 — demux wave 5 (crates/demux/src/mp4/). The AVI path is proven by avi/tests.rs:2695-2877.
- The Paused-seek decode carve-out (`[PROPOSAL: Lead decision 3.HARD-AUDIT D-a]`, seek.rs:116-128) bypasses the state machine's no-decode-in-Paused rule — flagged for 3.C1 revisit; verify the carve-out is what tests/seek_flow.rs actually pins.
- `Pipeline::seek_flush` budget/hold-cap arithmetic (session.rs:2027-2031) — pipeline.rs is another agent's file.
