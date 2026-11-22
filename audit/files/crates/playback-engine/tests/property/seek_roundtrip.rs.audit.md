# AUDIT: crates/playback-engine/tests/property/seek_roundtrip.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 845 / 7 tests + oracle + drivers |
| File verdict | REAL-UNPROVEN (strongest engine test asset audited this wave; one inherited oracle-mirror caveat) |
| Reachable from shipped entry point? | N/A — property test binary. W00 baseline: 1751/2/25; this binary's individual result UNVERIFIED (no cargo). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | `cat` full file (exit 0). No cargo. |

## 1. One-paragraph verdict

This is what a seek suite should look like: 64 seeds × 6 generated seeks over generated timelines (A/V interleaved or audio-only, known/unknown extents, B-lead and adversarial `pts < dts` stamps), each completed seek verified against a fixture-side truth model on six axes — exactly-one terminal, `requested` == clamped target, `actual` == the first audio frame at/after the keyframe-safe landing, every post-seek frame carrying the seek's generation and a pts ≥ landing, the clock anchor re-anchored at `actual`, and bounded completion (96 polls or the test fails). Plus four fixed boundary regressions (zero, past-end clamp-to-duration + clean end, percent/chapter typed refusals with playback resuming per D4, unseekable typed refusal with continued delivery) and a supersession storm (8 no-wait seeks) whose five laws include the doc 05 §4.3 exactly-one-terminal-per-generation invariant. The single structural weakness is inherited from the fixture: `expect()` builds `requested` from `resolve_target_ns` — the admitted mirror of the engine's own `resolve_target` math — so target-resolution defects are invisible to this suite (engine_fx audit F-001); everything downstream of resolution (landing, flush, anchor, generation discipline) is genuinely independently verified.

## 2. What actually works (verified)

- `expect` (192-212): landing from `landing_of` (fixture model), anchor truth = first audio-frame pts at/after landing else the landing — pure timeline arithmetic, independent of the engine.
- The per-seek verification block (330-427): exact-value asserts on `requested`/`actual`, per-frame generation + pts-≥-landing checks, "delivered no frames at all" refusal, first-audio-frame == anchor truth, clock anchor equality (`fixture.clock.anchor().media.ticks`).
- Ended-state seeks pinned: typed rejection, no mint (273-292) — matches the table's `EndedNeedsWireEdge` refusal.
- Supersession storm laws (749-804): terminal⊂accepted, no duplicate terminals (HashSet), final-seek completion, post-terminal window governed by the final mint (generation ≥ final, pts ≥ final landing), and the honest labeling of Law 5 as "the weak form".
- Determinism for shrinking: `run` rebuilds timeline+fixture from the seed; the canonical path regenerates ops from the same rng draws (documented at 454-457).

## 3. Findings

### F-001 [S3] The `requested` oracle is the engine-mirrored `resolve_target_ns` — resolution bugs pass
Where: crates/playback-engine/tests/property/seek_roundtrip.rs:192-195
Code (verbatim):
```rust
fn expect(op: SeekOp, timeline: &Timeline, seekable: bool, position_ns: i64) -> Expected {
    let Some(requested) = resolve_target_ns(op.target(), timeline.duration_ns, position_ns) else {
        return Expected::Refused(SeekFailure::OutOfRange);
    };
```
What is wrong: full statement and evidence in the engine_fx.rs audit F-001 (`resolve_target_ns` is copied from seek.rs to avoid "false counterexamples"). This suite therefore verifies landing/anchor/generation/terminal behavior for real, but clamping/saturation/percent-rounding only engine-vs-itself. The doc's own claim (line 15-16: "the oracle mirrors `resolve_target`") is the disclosure.
Confidence: HIGH — Verification: read both files.

### F-002 [S4] `unreachable!` as the failure vehicle (again)
Where: 460, 536, 580, 590, 612, 626, 637, 663, 493, 481 — same pattern as queue_invariants F-002. Functional; intent-inverting. Cosmetic.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Timeline`/`extent_of`/`generate_timeline`/`build_timeline` | 54-122 | REAL | — | stride 3/5/7 ms; A/V split; dts shapes 0/1/2 incl. adversarial pts<dts |
| `SeekOp` + `target`/`position_dependent` | 125-148 | REAL | — | |
| `generate_seeks` | 150-177 | REAL | — | in-range, negative, past-end, relative ±40 ms, percent full-i64 range, chapter 0-3 |
| `Expected`/`expect` | 180-212 | REAL | S3 | resolution via mirror (F-001); landing/anchor independent |
| `terminal_of` | 216-226 | REAL | — | generation-matched terminal scan |
| `run` | 228-443 | REAL | — | prime → per-seek verify (six axes) → bounded settle; Ended-refusal path |
| `seeks_roundtrip_with_first_frame_boundaries` | 450-463 | REAL | — | full seed bank |
| `prime_to_playing`/`drive_to_terminal` | 470-494 | REAL | — | bounded, unreachable! on budget exhaustion |
| `seek_to_zero_lands_at_the_first_frame` | 500-546 | REAL | — | exact (0,0) + first frame pts 0 + generation |
| `past_end_clamps_then_ends_cleanly` | 551-591 | REAL | — | i64::MAX/2 clamps to extent; clean Ended within 2048 polls |
| `unresolvable_targets_refuse_typed` | 595-638 | REAL | — | percent-unknown + chapter → OutOfRange; playback resumes (D4) |
| `unseekable_source_refuses_typed_and_keeps_playing` | 643-674 | REAL | — | NotSeekable + position 0 + continued delivery |
| `supersession_storm_is_silent_for_superseded_generations` | 691-806 | REAL | — | laws 1-5; receipt-driven attribution (FIFO by request id) |
| `storm_poll`/`collect_terminals` | 811-845 | REAL | — | |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| seeks_roundtrip_with_first_frame_boundaries | YES (reasoned) for landing/flush/anchor/generation/terminal/hang defects; NO for resolution-math defects (F-001) | exact-value asserts on all six axes with seed+step context |
| seek_to_zero / past_end_clamps / unresolvable_targets / unseekable_source | YES (reasoned) | exact boundary values, typed reasons, bounded loops with failure panics |
| supersession_storm_is_silent_for_superseded_generations | YES (reasoned) | five laws incl. the no-duplicate-terminals HashSet assert and the post-terminal window boundary |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the reported `requested` is the clamped target (… the oracle mirrors `resolve_target`)" | header 15-16 | REAL-as-stated / SHALLOW-as-proof | mirror disclosed; see F-001 |
| doc 05 §4.3 invariant 4: one terminal per completed generation | storm laws | REAL-UNPROVEN | Law 2 HashSet + Law 3; run UNVERIFIED |
| "clamping is truth, doc 05 §2.4" | header + past_end test | REAL-UNPROVEN | past_end pins clamp-to-extent exactly |

## 7. Dead / duplicated / unreachable code

None.

## 8. Dependencies & flags

playback-engine, media-types, testkit, shared fixture+rng. Env: LUMEN_PROP_SEEDS / LUMEN_PROP_SHRINK.

## 9. Open questions for the next agent

- seek.rs: adjudicate whether `resolve_target`'s clamp/saturation/percent-rounding matches doc 05 §2.4 prose independent of the mirrored oracle (file: crates/playback-engine/src/seek.rs).
