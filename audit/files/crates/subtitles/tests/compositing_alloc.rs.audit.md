# AUDIT: crates/subtitles/tests/compositing_alloc.rs
| Field | Value |
|---|---|
| Auditor | W09-A05 |
| Date | 2026-11-24 |
| Lines / functions | 101 / 4 (3 helpers + 1 test) |
| File verdict | REAL (the zero-alloc measurement is genuine, instrumented, and passing — but two of its coverage claims are false) |
| Reachable from shipped entry point? | N/A (test file) — runs in the workspace test lane; `cargo test -p subtitles --test compositing_alloc` → 1 passed (run by me); baseline `PACKAGE subtitles exit_code=0` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `cargo test -p subtitles --test compositing_alloc` → 1 passed / 0 failed (exit 0); read of testkit/src/alloc_count.rs:160-205 (instrument); python3 window arithmetic; sed quote verification |

## 1. One-paragraph verdict
The zero-allocation steady-state proof is real and honestly instrumented: a `#[global_allocator]` counting wrapper (testkit, verified: `assert_zero` panics on any alloc event or byte inside the window), a warm-up that grows the compositor's scratch before the guard, 240 compose calls inside the window, and a loud assert — it passes when run, and the drawn_total=480 assertion proves the loop actually composited. But the file's own description of what the window covers is wrong twice: the header promises "0-, 1- and 2-cue frames" and the comment promises "24 fps x 10 s of cue flips", while the actual frame step is 4.17 ms (the ` / 10` makes it 240 fps), the whole window spans ~1.0 s, every single frame draws exactly 2 cues, and no cue ever flips. The scaling (CIF 352×288 → 1080p), translucent-blend, and timing-rejection branches genuinely run; the frame-count-transition and empty-frame branches do not exist in this test at all — and allocation-on-transition is precisely the risk a zero-alloc proof should cover.

## 2. What actually works (verified)
— verified by `cargo test -p subtitles --test compositing_alloc` → 1 passed: 240 in-window `compose` calls allocate 0 bytes / 0 events (instrument panics otherwise — testkit/src/alloc_count.rs:194-204 read), 480 cue draws counted, scaling + alpha-128 blend + future-cue rejection branches all inside the window.

## 3. Findings
F-001 [S3]
Where: crates/subtitles/tests/compositing_alloc.rs:9-11 and 86-92
Code (9-11, verbatim):
```
//! measured window; inside it, only `compose` runs, across a cue mix
//! that exercises 0-, 1- and 2-cue frames, scaling, and translucent
//! blending.
```
Code (86-92, verbatim):
```
    // Measured window: 240 frames of the steady render loop —
    // 24 fps x 10 s of cue flips, zero allocation expected.
    let guard = ALLOC.state().guard();
    let mut drawn_total = 0usize;
    for i in 0..240i64 {
        // Sample-ish frame times with active/inactive mixes.
        let now = ns(i * 41_666_667 / 10);
```
What is wrong: Both claims are false. (1) The frame step is `41_666_667 / 10` = 4,166,666 ns ≈ 4.17 ms — 240 frames span ~0.996 s, not "10 s", and the rate is 240 fps, not "24 fps" (a 24 fps step would be `i * 41_666_667` with no division). (2) No cue flip ever occurs: both active cues span `[0, 10 s)` and the window never reaches 1 s, so the "future" cue (start 10 s) is rejected in all 240 frames and the drawn count is exactly 2 every frame — confirmed by the test's own `assert_eq!(drawn_total, 480)` (240 × 2). There are no 0-cue frames and no 1-cue frames anywhere in the file, despite the header's "0-, 1- and 2-cue frames" claim. Why it matters: a zero-alloc proof whose window never changes the active-cue count does not exercise allocation behavior on cue-count transitions (scratch re-sort/regrowth, capacity changes) — the exact steady-state-vs-transition distinction the "steady state" framing demands. What correct looks like: span the window across 10 s (drop the `/ 10`), let the future cue enter and the early cues leave, and assert the alloc guard across those flips. Confidence: HIGH — Verification: read + python3 arithmetic (`window_ns = 995833341; 10s? False`) + the test's own 480 assertion.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ns / span helpers | 24-33 | REAL | — | |
| compose_steady_state_is_zero_allocation | 39-100 | REAL | S3 | measurement valid; F-001 false coverage claims (0/1-cue frames absent, no flip, 240 fps × 1 s not 24 fps × 10 s) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| compose_steady_state_is_zero_allocation | YES for the alloc claim | CountingGlobalAlloc.assert_zero panics on any in-window allocation (testkit read + its own self-test `assert_zero_fails_loudly_on_a_nonzero_window`); drawn_total=480 pins that compose actually ran and drew |
| (coverage claims in docs) | NO | The promised 0-/1-cue frames and cue flips are never constructed — nothing fails if those branches allocate |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the compositing STEADY STATE allocates NOTHING — measured … runtime evidence, D-6" | compositing_alloc.rs:1-3 | REAL | 1 passed with the counting allocator (run); instrument verified by read |
| "exercises 0-, 1- and 2-cue frames" | compositing_alloc.rs:9-11 | FALSE | F-001: every frame draws exactly 2 (drawn_total=480) |
| "24 fps x 10 s of cue flips" | compositing_alloc.rs:86-87 | FALSE | F-001: step is 4.17 ms, window ~1.0 s, zero flips |
| "the third is scanned and rejected by the timing filter every frame" | compositing_alloc.rs:96-98 | REAL | all 240 nows < 10 s (python3) |

## 7. Dead / duplicated / unreachable code
None. Single test per binary as documented (one `#[test]`; the process-global-allocator rationale is sound and stated).

## 8. Dependencies & flags
dev-deps: testkit (CountingGlobalAlloc), media-types. No feature gates; deterministic; no threads beyond the harness (and only one test, so no sibling-thread noise in the counters).

## 9. Open questions for the next agent
- Does `SubtitleCompositor` allocate on active-cue-count change (scratch `order`/`xmap` capacity)? This test cannot answer it (F-001); compositing/compose.rs is the place to check.
- If a later wave fixes the window to actually flip cues, the zero-alloc contract may legitimately fail — that is the point of running it.
