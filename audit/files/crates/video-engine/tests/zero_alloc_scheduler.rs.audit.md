# AUDIT: crates/video-engine/tests/zero_alloc_scheduler.rs

| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-22 |
| Lines / functions | 150 / 1 `#[test]` + cfg() helper + CountingGlobalAlloc setup |
| File verdict | **REAL** — a genuine zero-allocation proof, not a decorative perf test |
| Reachable from shipped entry point? | The *subject* is: `PresentationScheduler::new` runs in the shipped CLI (pipeline_host.rs → scheduler; per W03-A02's sync.rs chain). The test itself runs under `cargo test`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Read of the full file; cross-check of `PresentationScheduler` production construction (W03-A02 audit chain); no cargo (static). |

## 1. One-paragraph verdict

This is the model zero-alloc test: a counting global allocator with a scoped guard (92), one-time costs hoisted outside the measured window by an explicit warm-up cycle (78-89), pools built outside the window with the Arc-refcount rationale documented (66-68), 384 aggressive dropping ticks inside it (128 cycles × storm+burst+repeat), exact per-cycle behavioral assertions *inside* the measured loop (drop counts, supersede counts, present/repeat actions — 100-133), and post-window lifetime accounting that ties the totals (129 storms, 129 presents, nothing lost, 142-149). It would fail under three independent classes of regression: any per-tick allocation (guard), any drop-counting arithmetic (per-cycle asserts), any window truncation (`drops.late.len() == STORM`).

## 2. What was checked (no clean bill without proof)

- The allocator is process-global with `serialize_tests()` (55) and exactly one `#[test]` in the binary (the doc at 9-10 states the constraint — verified: one test function in the file).
- Warm-up correctness: the warm-up (81-89) runs the same three paths as the measured cycles, so first-touch costs (deque growth, last-request slot) genuinely land outside the guard.
- The storm frames are `i * FRAME` with the clock parked at `10_000 * FRAME` (63-64) — every storm frame is ~10 000 frames late, so "hopelessly late" holds; the due pool is `[now-3, now]` — inside `[now − threshold, now + lead]` for the stated cfg (one-frame threshold, half-frame lead). The arithmetic is local and spelled out (37-39) — verified against the constants.
- Assertions are exact values (`STORM as u64`, `(BURST-1) as u64`, `storms * …`), not `>`/`is_ok` smoke — mutation-resistant.
- The `matches!(RepeatLast | Background)` looseness (110-113, 130-133) is justified: which arm fires depends on whether a present preceded the tick (Background before the first present, RepeatLast after) — the strict half is pinned by (2)'s `PresentAction::Present` assert (126).
- Subject reachability: PresentationScheduler is the production video path (video-engine consumed by apps/cli/web/desktop; W03-A02 verified the constructor chain).

## 3. Findings

None. (Checked for: mock-of-the-subject — no, the real scheduler is driven; tautology — no, exact-value asserts; hidden relaxation — no `assert!(result.is_ok())`-style smokes; measurement laundering — pools and warm-up are honestly placed and documented.)

## 4. Per-function table

| Item | Lines | Verdict | Notes |
|---|---|---|---|
| `#[global_allocator]` setup | 12-14 | REAL | testkit counting allocator |
| `cfg()` | 40-51 | REAL | doc 03 §4.10 e default shape, constants local |
| `dropping_ticks_allocate_nothing_after_warm_up` | 53-150 | REAL | see §1; guard + behavior + accounting triple |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| dropping_ticks_allocate_nothing_after_warm_up | YES | three independent failure classes: allocation (guard), decision correctness (per-cycle exact counts), boundedness (len == STORM not truncated) |

UNVERIFIED by execution (no cargo); the W00 baseline ran the workspace suite green except two first-vertical failures, consistent with this passing.

## 6-9. Doc claims / dead code / dependencies / open questions

- Doc claim "One `#[test]` per binary by design" (9-10): verified — one test.
- No dead code; deps via the crate's dev-dependencies (testkit, media-types, video-engine) — consistent.
- Open question for wave 16: this file is the positive control for zero-alloc test methodology; `epoch_staleness_alloc.rs` and `zero_alloc_resample.rs` follow the same pattern (cited in other agents' audits).
