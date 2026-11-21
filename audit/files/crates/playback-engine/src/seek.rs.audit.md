# AUDIT: crates/playback-engine/src/seek.rs
| Field | Value |
|---|---|
| Auditor | W03-A07 |
| Date | 2026-11-21 |
| Lines / functions | 545 / 17 (4 enums + 2 manual impls, 3 structs, 7 fns, module docs; 7 tests) |
| File verdict | REAL (seek flow shipped, unit + integration suites pass — but the first-vertical seek journey FAILS in the current tree: F-001) |
| Reachable from shipped entry point? | YES (chain: `lumen` seek command → session.rs `seek_pipeline_step` (:1985) → `resolve_target` (:2002), `pipeline_seek` (:2043), `complete_seek` (:2533), `not_seekable` (:2025); the first-vertical journey runs seek through the real binary) |
| Findings | S0:0 S1:0 S2:2 S3:0 S4:2 |
| Commands run | `rg -n "pipeline_seek\|complete_seek\|resolve_target\|not_seekable\|InFlightSeek" crates/playback-engine/src/session.rs` (all wired) · `rg -n "frames_accepted"` (CLI consumer pipeline_host.rs:1231) · baseline log: seek_flow.rs 21/21 ok (:4061-4086); journey_seek FAILED (:2521-2537) · `rg -n "fn duration_ns" pipeline.rs` (:1198-1203, max of possibly-negative stamps) · `rg -n "fn seek_error_wire" event.rs` (:741-763) · arithmetic/std review of clamp/NaN paths |

## 1. One-paragraph verdict
The seek flow's design is honest — clamping-is-truth resolution, D4 rollback, M10 presentation hold, typed refusals at every refusal point — and its fake-decoder integration suite (seek_flow.rs, 21 tests) passes in the no-fail-fast baseline. Two material defects: (1) the REAL-pipeline seek journey (`first-vertical` `journey_seek`) is one of the repo's 7 true baseline failures — the decode-accounting check "keeps the pre-seek deliveries" observed 90 against an expected 154, so the seek flow's own accounting contract is broken on the real MJPEG pipeline (root cause unresolved without a run; the failing check is the seek scenario's core "pipeline was not rebuilt" proof); (2) `resolve_target`'s clamp closure panics (`i64::clamp`'s `assert!(min <= max)`) if a demuxer ever reports a negative duration — `Pipeline::duration_ns` takes an unvalidated `max()` of track-span ends and nothing floors it at zero. Two hygiene items: NaN percent fractions silently seek to position 0, and decode-stage flush failures are carried in a variant whose documentation claims demuxer positioning.

## 2. What actually works (verified)
— `tests/seek_flow.rs`: 21/21 passed (`orchrun-test-workspace-nofailfast.log.txt:4061-4086`) — the in-file seven-step flow (flush → reanchor → pre-roll → complete) under fake decoders.
— In-file unit tests 7/7 passed (playback_engine unittests, baseline :3740-3742, 83 tests): absolute/relative/percent clamping, percent-without-duration typed refusal, chapter typed refusal, span round-trip, failure clone/eq without source chains.
— Wiring: every pub/pub(crate) item has a session.rs call site (§ header) — no dead code in this file.
— `resolve_target` Relative uses `saturating_add` (:289) — no overflow; `nanos_i64` saturates documented (:330-332).
— D4 rollback shape verified by reading: `pipeline_seek` only records `inflight` after `demuxer_reanchor` succeeds; on failure the outcome carries the typed error and the session re-fills from the standing demuxer (session.rs:2061-2069).

## 3. Findings
### F-001 [S2]
Where: crates/playback-engine/src/seek.rs:1-143 (the contract), evidence: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:2529-2530, tests/integration/first-vertical/src/scenarios.rs:523-532
Code (baseline log :2529-2530, verbatim):
```
thread 'journey_seek' (7272) panicked at tests/integration/first-vertical/tests/common/mod.rs:11:9:
scenario `seek` — check failed: the decode accounting keeps the pre-seek deliveries — observed 90, expected 154
```
Code (scenarios.rs:528-532):
```
        checks.push(Check::int(
            "the decode accounting keeps the pre-seek deliveries",
            i64::try_from(accounting.frames_accepted).unwrap_or(i64::MAX),
            154,
        ));
```
What is wrong: the seek scenario's central accounting proof — pre-seek decode-ahead deliveries stay counted plus the landing's 60 frames ("a stop→close→reopen would have answered exactly the landing's 60", scenarios.rs:524-527) — fails on the current tree: 64 frames are missing from `frames_accepted` (counted at the CLI's `CliFrameConsumer::accept`, apps/cli/src/pipeline_host.rs:1231). This is one of the 7 true failures the fail-fast Wave-0 baseline hid (it never reached first-vertical). Either the engine under-delivers frames around the seek boundary (pre-roll hold discards eating deliveries — see `take_preroll_discarded`/`fold_preroll_discards`, session.rs:2100-2105) or the scenario's committed oracle drifted; both possibilities are defects, and I cannot distinguish them without execution.
Why it matters: if the engine is under-delivering, a seek visually skips ~64 frames of the landing half; if the oracle drifted, the repo's committed journey evidence is unreliable. Either way the seek flow's own documented accounting contract (this file's module docs, steps 5-7) is not currently demonstrated.
What correct looks like: a green `journey_seek` with the accounting identity pre-seek + landing = 154 (or a corrected oracle with a documented reason for the change).
Confidence: HIGH (the failure is verbatim baseline evidence) / root cause UNRESOLVED — Verification: baseline log read; check source read.
REQUEST-ORCH-RUN: `cargo test -p first-vertical --test journey journey_seek -- --nocapture` (plus `--exact`) to dump the accounting fields.

### F-002 [S2]
Where: crates/playback-engine/src/seek.rs:278-283 (and the Percent arm :291-294)
Code:
```
    let clamp = |ns: i64| -> i64 {
        match duration_ns {
            Some(duration) => ns.clamp(0, duration),
            None => ns.max(0),
        }
    };
```
What is wrong: `i64::clamp` is specified to panic when `min > max`. `duration_ns` comes from `Pipeline::duration_ns` (pipeline.rs:1198-1203): the unvalidated `.max()` of `stamp_ns(track.timing.span.end)` across tracks — a max of signed nanosecond stamps that is nowhere floored at zero. A container whose track spans carry negative end stamps (corrupt/hostile input the demuxer accepted) makes `duration_ns = Some(negative)`, and the next seek command panics in the engine: `ns.clamp(0, -x)` → `assert!(min <= max)` failure. The Percent arm passes its computed value through the same closure, so every target kind hits it.
Why it matters: a panic in the seek path on malformed input — exactly the class the recovery-matrix tests exercise (truncated/flipped/corrupt fixtures) — crashes the session instead of producing the typed `SeekFailed` this module's philosophy promises. The panic is unconditional in release (std assert, not debug-only).
What correct looks like: `Some(duration) if duration >= 0 => ns.clamp(0, duration)`, treating a negative duration as unknown (`ns.max(0)`) or surfacing it as a typed `Demux` failure; plus a floor (or refusal) in `Pipeline::duration_ns`.
Confidence: MEDIUM — the clamp panic is proven by code reading (std `clamp` contract); whether a real demuxer can emit a negative span end is UNVERIFIED (would need a crafted container; the demux crate is another wave's).
### F-003 [S4]
Where: crates/playback-engine/src/seek.rs:293
Code:
```
                ns: clamp((f64::from(fraction.clamp(0.0, 1.0)) * duration as f64).round() as i64),
```
What is wrong: `SeekTarget::Percent` is a pub enum with a pub `f32` field — a `NaN` fraction survives `clamp(0.0, 1.0)` (f32::clamp propagates NaN), NaN × duration = NaN, `.round()` = NaN, and `NaN as i64` = 0: the seek silently goes to position 0 instead of a typed refusal. Also `duration as f64` loses integer precision for durations above 2^53 ns (~104 days) — theoretical for media.
Why it matters: contradicts the module's "typed refusals, never guesses" stance; a host bug (NaN fraction) becomes a silent jump to media start.
What correct looks like: `if fraction.is_nan() { return ResolvedSeek::Failure(SeekFailure::OutOfRange) }` before the arithmetic.
Confidence: HIGH — Verification: read + Rust float semantics (clamp/`as` casts saturate, NaN→0).
### F-004 [S4]
Where: crates/playback-engine/src/seek.rs:213-216 vs :382-384
Code (seek.rs:382-384):
```
    if let Err(error) = pipeline.seek_flush(generation, flush_budget, hold_cap) {
        outcome.stale_frames_flushed = pipeline.take_seek_flush_stale();
        outcome.failure = Some(SeekFailure::Demux(error));
```
What is wrong: a decoder flush/configure failure (the comment at :377-378 itself calls it "a decode-stage refusal") is carried in `SeekFailure::Demux`, whose doc (:213-215) says "The demuxer could not position the stream". The wire stays honest — `seek_error_wire` (event.rs:756-760) forwards the carried error's own stage/code — but the Rust-level variant mislabels decode failures as demux failures for every pattern-matching consumer.
Why it matters: misleading name/doc on a pub-adjacent error type; pattern matches on `SeekFailure::Demux` (e.g. metrics) will bucket flush failures as demux failures.
What correct looks like: a `SeekFailure::Flush(EngineError)` variant (or rename `Demux` to `Pipeline`).
Confidence: HIGH — Verification: read both sites + event.rs mapping.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (7 steps, M10, D4, supersession, paused carve-out) | 1-143 | REAL | — | design narrative; each checkable claim traced (D4 shape §2, M10 hold in pipeline.rs parts, paused carve-out PROPOSAL-flagged honestly at :121-128) |
| `enum SeekTarget` | 155-182 | REAL | — | pub; Chapter refuses typed (honest, :176-181) |
| `enum ResolvedSeek` | 186-196 | REAL | — | internal, consumed :2002 |
| `enum SeekFailure` | 204-219 | REAL | S4 | F-004 variant-doc mismatch |
| `impl Clone for SeekFailure` | 221-233 | REAL | — | shallow rebuild, tested |
| `impl PartialEq` | 235-247 | REAL | — | compares stage/kind/code only — documented, tested |
| `shallow` | 250-258 | REAL | — | drops source chain per event convention |
| `not_seekable` | 263-266 | REAL | — | used session.rs:2025 |
| `resolve_target` | 273-304 | REAL | S2/S4 | F-002 clamp panic on negative duration; F-003 NaN; Relative saturating_add correct; clamping tests passed |
| `target_span` | 308-313 | REAL | — | round-trip tested |
| `span_start_ns` | 318-324 | REAL | — | Missing → None surfaced, tested |
| `nanos_i64` | 330-332 | REAL | — | documented saturate |
| `struct InFlightSeek` | 336-348 | REAL | — | origin set by session (:2058) — not dead |
| `struct SeekOutcome` | 353-362 | REAL | — | consumed :2043-2069 |
| `pipeline_seek` | 369-412 | REAL | S4 | flush-failure wrap F-004; landing-missing typed stand-in (:400-405, :416-424) honest; covered by seek_flow 21/21 |
| `landing_missing` | 416-424 | REAL | — | typed Corrupt for a contract violation — good |
| `complete_seek` | 434-450 | REAL | — | audio-truth anchor; Clock failure typed; covered by seek_flow |

## 5. Test honesty (7 in-file tests, all passed in baseline; seek_flow.rs 21/21)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `absolute_clamps_to_the_duration_when_known` | YES | exact clamp asserts incl. negative→0 and unknown-duration floor |
| `relative_resolves_against_the_position_then_clamps` | YES | rewind-past-start → 0 |
| `percent_needs_a_known_duration` | YES | typed OutOfRange without duration; clamping of 1.5/-0.5 |
| `chapter_refuses_typed_until_a_chapter_source_exists` | YES | refuses even with a duration |
| `target_span_and_back_is_lossless_in_nano_base` | YES | round-trip + UNKNOWN→None |
| `failure_clones_and_compares_without_source_chains` | YES | clone/eq semantics |
| Gap 1 | — | no test for negative `duration_ns` (F-002's panic — a test with `Some(-1)` would panic today, proving the defect) |
| Gap 2 | — | no test for NaN fraction (F-003) |
| Gap 3 | — | `pipeline_seek`/`complete_seek` have no in-file unit tests; covered only by seek_flow.rs fake-decoder suites — the REAL-pipeline accounting gap is exactly F-001 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "never a fake success (MP §8 bullet 4)" | seek.rs:16-17 | REAL | unseekable/resolution refusals emit SeekFailed (session.rs:2004-2030); seek_flow passed |
| "the clock never moved (it re-anchors only at step 6, on success)" | seek.rs:87-88 | REAL | complete_seek is the only reanchor call (session.rs:2533) |
| "superseded seeks are silent" (no terminal for replaced seek) | seek.rs:101-103 | REAL (code read) | session.rs:2052-2060 replaces slot without terminal |
| "decode accounting keeps the pre-seek deliveries" (the journey's proof) | scenarios.rs:529 (against this file's flow) | **FALSE in current tree** | F-001: baseline FAILED, observed 90 expected 154 |
| "A second seek that fails resolution ... leaves the older in-flight seek untouched" | seek.rs:108-110 | REAL (code read) | resolution failure returns before the slot is touched (session.rs:2002-2013) |
| Paused-seek decode carve-out | seek.rs:116-128 | REAL-UNPROVEN | PROPOSAL-flagged; behavior lives in pipeline tick (parts 3-5, other agents); seek_flow covers paused seeks |

## 7. Dead / duplicated / unreachable code
- None: every item has a live consumer (session.rs wiring §2; `origin` set at session.rs:2058).

## 8. Dependencies & flags
- Depends on commands (RequestId), media-clock (NANO_BASE, PlaybackClock), media-types, crate::pipeline. No feature gates. Built + tested in baseline (83 playback_engine unittests + 21 seek_flow integration).

## 9. Open questions for the next agent
- Pipeline parts 3-5 auditors: where do the 64 missing `frames_accepted` go — pre-roll hold discard accounting (`take_preroll_discarded`, `fold_preroll_discards`) or the landing decode window? F-001 needs a run to settle.
- Demux wave: can any demuxer emit a track span end that converts to negative ns (making F-002's panic reachable)? Check mp4 stts/stts edge cases and AVI dwLength arithmetic.
- `Check::int` for `scheduler_pushed` expected 46 and `scheduler_dropped_stale` expected 16 (scenarios.rs:533-545) were never evaluated (panic at the first check) — same run should print them.
