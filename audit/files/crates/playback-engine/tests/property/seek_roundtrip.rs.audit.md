# AUDIT: crates/playback-engine/tests/property/seek_roundtrip.rs
| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-23 |
| Lines / functions | 845 / 13 helpers + 6 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test file) — wired: `tests/property/main.rs:40 mod seek_roundtrip;` compiles into the `property` test binary that `cargo test -p playback-engine` runs (baseline: 4744 passed workspace-wide) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/property-25b39d1917495830 seek_roundtrip` -> 6 passed, 0 failed, exit 0 (binary 19:29 newer than source 17:35) |

## 1. One-paragraph verdict
This is a genuine property suite driving the REAL engine (`playback_engine::Engine::open` via the engine_fx fixture) through 64 fixed seeds × 6-8 generated seeks over synthetic timelines (A/V stride grids, B-lead and pts<dts adversarial stamps, known/unknown extents, out-of-range/negative/percent/chapter targets), and it would fail if the seek machinery broke: it asserts exact clamped targets, anchor-truth actuals (first audio frame at/after the landing, computed independently from the timeline), per-frame generation attribution, pre-landing flush boundaries, clock re-anchors (reading the injected MediaClock), typed refusals with position preservation, bounded completion budgets, and a supersession storm with five named laws. All 6 tests executed and pass (0.03s — the frozen-clock determinism makes the oracle sound: the engine's position between seeks provably equals the last seek's actual). The one structural weakness is honestly documented in-file: the target-resolution oracle MIRRORS the engine's seek.rs arithmetic, so a spec-level bug shared by both would be invisible — only the hand-computed boundary tests (zero, past-end) pin the spec independently.

## 2. What actually works (verified)
- `./target/debug/deps/property-25b39d1917495830 seek_roundtrip` -> `test result: ok. 6 passed; 0 failed` (exit 0): the 64-seed roundtrip law test, the supersession storm, zero/past-end/unresolvable/unseekable boundary anchors.
- Mutation reasoning (no edits): broken clamping fails `requested != want_requested` (out-of-range ops are 5 of 8 generator arms); broken anchor truth fails `actual != want_actual` whenever the landing is a video frame (50% of A/V cases); pre-landing frames fail the `pts < *landing_ns` check (380-385); stale generations fail the `generation != seek_generation` check (369-374); un-re-anchored clock fails `anchor != *want_actual` (402-407).
- The seek-from-Ended branch (273-292) asserts the typed rejection AND that no generation was minted — pins the refusal contract, not just absence of a crash.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/property/seek_roundtrip.rs:15-16 (doc) + engine_fx.rs:442-465 (mirror)
Code:
```
//! - **the reported `requested` is the clamped target** (clamping is
//!   truth, doc 05 §2.4 — the oracle mirrors `resolve_target`);
```
What is wrong: the target-resolution oracle (`resolve_target_ns`) is a test-side copy of the engine's seek.rs algorithm (engine_fx.rs:458-461 "The engine's exact arithmetic (seek.rs `resolve_target`), mirrored"), so the property sweep cannot detect a spec-level resolution error shared by implementation and mirror (wrong clamp bounds, wrong percent rounding, wrong saturating semantics would all pass if mirrored). Mitigated by the two hand-computed boundary anchors (`seek_to_zero` expects (0,0); `past_end_clamps` expects clamp-to-extent) — but those pin only two points of the resolution spec. Confidence: HIGH — Verification: read (both files) + ran the suite.
Why it matters / correct form: an independent spec-derived oracle (doc 05 §2.4's clamp language coded from the doc, not from seek.rs) or more fixed-value anchors would close the echo.

F-002 [S4]
Where: crates/playback-engine/tests/property/seek_roundtrip.rs:216-226 (terminal_of), 777 (Law 4 skip)
Code:
```
fn terminal_of(outputs: &[EngineOutput], generation: u64) -> Option<&EngineEventKind> {
    outputs.iter().find_map(|output| match output {
        EngineOutput::Event(event) if event.generation.0 == generation => match &event.kind {
```
What is wrong: (a) `run()` takes the FIRST terminal of a generation and never asserts exactly-one-terminal — that law is enforced only by the storm test's Law 2; (b) Law 4's post-terminal boundary check is silently skipped when the final accepted seek expects a refusal (`if let Expected::Completed { landing_ns, .. } = expect(...)` at 777, no else) — correct (a refused seek does not re-own the pipeline under D4) but unstated in the assertion, so a reader cannot tell refusal-finals are uncovered rather than covered. Confidence: HIGH — Verification: read.
Why it matters: weak-form choices are fine but should be pinned by an explicit skip-assert or comment; today the storm's refusal-final path exercises no boundary law.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| extent_of | 63-65 | REAL | — | grid span |
| generate_timeline | 69-74 | REAL | — | stride/av/declared/dts_shape draws |
| build_timeline | 80-122 | REAL | — | video even strides, audio odd offsets, 3 dts shapes incl. adversarial pts<dts; B-lead dips negative at head — good adversarial shape |
| SeekOp::target / position_dependent | 133-148 | REAL | — | storm filter |
| generate_seeks | 150-177 | REAL | — | 8 arms: in/out-of-range absolute, negative, past-end, ±relative, percent, chapter |
| expect | 192-212 | REAL | S4 | resolution via mirrored oracle (F-001); anchor truth computed independently from slots |
| terminal_of | 216-226 | REAL | S4 | first-match only (F-002a) |
| run | 228-443 | REAL | — | the law set; Ended-refusal branch, class-A accept+1 mint, budgets, settle |
| seeks_roundtrip_with_first_frame_boundaries | 450-463 | REAL | — | 64 seeds × 6 seeks; ran, passes |
| prime_to_playing | 470-482 | REAL | — | bounded, unreachable! on failure |
| drive_to_terminal | 485-494 | REAL | — | bounded, unreachable! on hang |
| seek_to_zero_lands_at_the_first_frame | 500-546 | REAL | — | hand-computed (0,0) — independent spec pin |
| past_end_clamps_then_ends_cleanly | 551-591 | REAL | — | clamp-to-duration + clean end, bounded 2048 polls |
| unresolvable_targets_refuse_typed | 595-638 | REAL | — | percent-without-duration + chapter refuse OutOfRange; D4 resume check |
| unseekable_source_refuses_typed_and_keeps_playing | 643-674 | REAL | — | NotSeekable + position 0 + frames keep flowing |
| supersession_storm_is_silent_for_superseded_generations | 691-806 | REAL | — | 64 seeds, receipt-driven attribution, Laws 1-5 |
| storm_poll | 811-832 | REAL | — | FIFO receipt attribution |
| collect_terminals | 834-845 | REAL | — | |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| seeks_roundtrip_with_first_frame_boundaries | YES | exact-value oracles (requested/actual/anchor/first-audio-pts), generation attribution, budgets (ran) |
| supersession_storm_... | YES | Laws 1-3,5 are counting invariants; Law 4 checks frame generation/pts in the post-terminal window (ran) |
| seek_to_zero / past_end_clamps | YES | hand-computed expectations, independent of the mirrored oracle (ran) |
| unresolvable_targets_refuse_typed | YES | typed reason + session-still-plays check (ran) |
| unseekable_source_refuses_typed_and_keeps_playing | YES | reason + position + continued delivery (ran); position==0 is weak (frozen clock makes 0 trivially true) but the delivery-growth check carries the weight |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "seek-to-time → first-emitted-frame boundary properties" (1-3) | module doc | REAL | run() checks 356-407; suite ran green |
| "the oracle mirrors `resolve_target`" (16) | module doc | REAL (honest about the mirror) | engine_fx.rs:458-461 |
| "a hang is a failed test" (26-27, 48-49) | module doc | REAL | TERMINAL_BUDGET=96 loops end in unreachable! messages |
| "one terminal per completed generation" (13, 30-31) | module doc | PARTIAL | enforced in the storm test only; `run()` never checks duplicates (F-002a) |
| 64 seeds (rng.rs:77-89 SEED_BANK) | harness | REAL | executed full bank in 0.03s |

## 7. Dead / duplicated / unreachable code
None. All helpers are used by the suite; no `#[ignore]`; no cfg-gating beyond the module tree (main.rs:40 includes it unconditionally).

## 8. Dependencies & flags
testkit::Must, playback_engine public API only (black-box claim verified: no `use playback_engine::...` of private modules; fixture injects through SessionConfig seams — plugin_api::Demuxer, codec_api::Decoder, FrameConsumer). Env knobs LUMEN_PROP_SEEDS/LUMEN_PROP_SHRINK are test-only.

## 9. Open questions for the next agent
- seek.rs itself (W03-A01's pipeline domain) — is `resolve_target`'s clamp the doc-05 §2.4 semantics or just self-consistent with this mirror? The two boundary anchors say yes for 0 and MAX/2; the interior is unpinned by independent values.
- The engine_fx fixture's `TimelineDemuxer::reanchor` and `landing_of` implement the same rposition landing algorithm twice (fixture 119-123 vs oracle 428-435) — fine for a scripted source, but any real-demuxer keyframe selection is untested here (that is seek_flow.rs/recovery_media.rs's domain).
