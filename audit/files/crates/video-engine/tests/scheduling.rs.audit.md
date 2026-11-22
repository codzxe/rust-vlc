# AUDIT: crates/video-engine/tests/scheduling.rs

| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-22 |
| Lines / functions | 757 / 21 `#[test]` + helpers (`step`, `wide_queue`, `default_cfg`, `boundary_step`) over `tests/common/mod.rs` fakes |
| File verdict | REAL — the strongest scheduling suite in the repo: virtual-clock determinism, exact slot-sequence oracles, boundary pins, conservation |
| Reachable from shipped entry point? | YES — the subject `PresentationScheduler` is the shipped CLI's present path: `apps/cli/src/pipeline_host.rs:84` imports it, `:1324` constructs it (`apps/cli/src/lib.rs:31`: "I420→RGBA host conversion → `PresentationScheduler`"). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | Read (full file, 2 chunks + tests/common/mod.rs in full); Grep `PresentationScheduler` reachability (pipeline_host.rs:84/:704/:1324); `#[test]` count = 21, no `#[ignore]`/cfg/env gates, no `thread::sleep` in the file (the "zero sleeps" claim verified by grep); sed verification of quoted ranges. No cargo — W00 baseline cited. |

## 1. One-paragraph verdict

The card 2.C4 acceptance suite for the presentation scheduler — and
it is what an acceptance suite should be: 21 ungated tests (all in the
W00 baseline's 1751) driving the real `PresentationScheduler` +
`PresentationQueue` wiring over injected clocks (settable monotonic
virtual clock, 48 kHz fake sample counter for the audio-master shape)
with exact oracles: full slot-sequence equality, per-test
drop-accounting (late/superseded/stale with counts), the conservation
law `presented + dropped_late + dropped_superseded + dropped_stale +
pending == pushed` cross-checked against the recording presenter's own
log, six boundary pins (pts==now, exactly-on-late-threshold presents,
one-past drops, exactly-on-due-bound presents, one-past holds), the
generation-stale end-to-end story (clock mint + queue publish + stale
push refusal with `current` echo), both typed bound rejections at the
wiring level, and a per-present error-bounds check through a stall and
recovery. The fakes are at the honest seams (clock source, presenter
sink) and the tested code is the shipped CLI's present path. Two
hygiene findings: the `step` helper swallows both fallible queue
outcomes, and the "shuffled arrival" in-order test samples 5 of the
120 permutations of its 5-frame window.

## 2. What actually works (verified)

— 21 ungated `#[test]`s → all ran in W00's `cargo test --workspace` (1751 pass); zero sleeps in the file (grep) — the doc 03 §8 determinism claim holds.
— Exact-sequence oracles: `sink.slots_seen() == want` for the full 60-frame steady run (:88-98), the stall/recovery tail `[9×FRAME] + (40..50)×FRAME` (:155-161), the early-hold `[0, 0, FRAME]` (:217-236), the paused freeze `[0,0,0,0]` (repeat budget 3, :634-638), the Never-policy `(3..6)×FRAME` (:574-577). A wrong present, a lost present, or a spurious repeat fails.
— Boundary semantics pinned as named laws: `pts == now` presents (:265-269), `pts == now − threshold` presents (strict `<`, :272-277), `pts == now − threshold − 1` drops (:280-284), far-late same accounting (:287-291), `pts == now + lead` presents (inclusive `≤`, :294-298), `pts == now + lead + 1` holds with `next_due == pts − lead` (:301-321).
— Conservation: :652-684 (`accounted == pushed`, `sink.len() == stats.presented`) — a frame lost between scheduler and presenter fails.
— Generation staleness end-to-end (:328-382): pending old-generation frames drop (`drops.stale == 3`), a late old-generation push is refused with the typed `Stale` carrying `current == next`, the new generation presents on its slot, and `gens_seen()` pins the exact delivery generations.
— The audio-master shape (:491-533): a fake 48 kHz sample counter drives the clock; the video grid (33,333,334 ns) vs sample grid (33,333,333⅓ ns) mismatch is absorbed by the lead across 60 frames with `dropped_late == 0` — the real A/V grid-drift shape at the scheduler layer, plus the 5-second stall dropping all 10.
— Typed bounds at the wiring level (:692-757): `WindowFull` with capacity and the handed-back frame's exact pts; queue `Full` with capacity, then drain-frees-slot-and-retry-lands.
— The common fakes (tests/common/mod.rs, read in full) are honest: `VirtualClock::set_ns` uses `fetch_max` (monotonic-safe, documented), `RecordingPresenter` records `(when_ns, generation)` per present and `assert_in_order` checks non-decreasing; a scripted `failing_at` presenter exists for the neighbor suites.
— Mutation reasoning (UNVERIFIED, no cargo): present late frames anyway → the late-boundary and slots-seen asserts fail; reorder presents → `assert_in_order` + sequence equality fail; lose a frame between tick and presenter → conservation + sequence fail; off-by-one on either bound → the six boundary pins fail; drop old-generation frames silently without counting → `drops.stale == 3` fails.

## 3. Findings

### F-001 [S4] The `step` wiring helper swallows both fallible queue outcomes

Where: crates/video-engine/tests/scheduling.rs:49-52 (helper used by ~15 of the 21 tests)
Code:
```rust
    if let Some(request) = request {
        let _ = queue.try_push(request);
    }
    let _ = queue.drain_to(sink);
```
What is wrong: `try_push` can refuse (`EnqueueOutcome::Full`) and `drain_to` returns a `Result` (the neighbor suite's `.must()` at :755 proves the error path exists — a presenter refusal). Both are discarded with `let _ =`. The sizing comment (:56-58, "bound refusal never interferes") makes the push infallible-by-construction (≤1 resident per step against capacity 64) and the recording presenter never fails — so today nothing is actually swallowed — but the helper's contract is enforced by a comment, not an assert: a future edit that ticks twice before draining, or swaps in the failing presenter, would turn a wiring refusal into a silently-missing slot whose only symptom is a downstream sequence mismatch with no pointer at the refusal.
Why it matters: §3.2 swallowed-error shape in the very helper that models "the production shape at v0" (:6-8); diagnosis quality, not correctness.
What correct looks like: `assert!(matches!(queue.try_push(request), EnqueueOutcome::Enqueued))` and `queue.drain_to(sink).must()`.
Confidence: HIGH — Verification: read (both sites + the `Full`/`.must()` evidence at :739-755).

### F-002 [S4] "Shuffled arrival" coverage is 5 hand-picked permutations of a 5-frame window (5 of 120)

Where: crates/video-engine/tests/scheduling.rs:390-398
Code:
```rust
    // Hand-picked arrival permutations (a property framework is a doc
    // 04 candidate; deterministic permutations cover the same ground).
    let permutations: [Vec<i64>; 5] = [
        vec![4, 2, 0, 1, 3],
```
What is wrong: the test's name promises in-order presentation "under shuffled arrival"; the body samples 5 fixed permutations of 5 elements — 4% of the permutation space, all on a 5-frame window with a clean walk-to-each-slot schedule. The comment is honest about the missing property framework, but "deterministic permutations cover the same ground" overstates: interleaved push/tick schedules (arrival racing the clock, the shape the soak and real decode produce) are not generated here at all — every permutation is pushed up front, then the clock walks.
Why it matters: the in-order-under-arrival-race law is the scheduler's core claim; this test pins only the batch-arrival corner of it.
What correct looks like: a seeded op-generator (the packet-queue property suite's shape) mixing pushes and ticks; or at least all 120 permutations at this window size (trivial cost).
Confidence: HIGH — Verification: read (the whole test :388-415 — pushes all 5, then walks the clock).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FRAME`/`default_cfg`/`step`/`wide_queue` | 28-61 | REAL | S4 | F-001 (swallowed outcomes in `step`) |
| `boundary_step` | 245-262 | REAL | — | fresh scheduler per boundary; explicit cfg |
| `ac_in_order_presentation...` | 67-99 | REAL | — | 60 frames, exact order + stats + conservation |
| `ac_late_frames_dropped...` | 105-164 | REAL | — | stall/recovery; drop counts; repeat accounting (`presented` excludes repeats, `repeats == 1`) |
| `drops_are_never_resurrected...` | 166-187 | REAL | — | stable accounting across repeated stalled ticks |
| `ac_early_frames_are_held...` | 193-237 | REAL | — | hold/repeat/wait-hint (`next_due == pts − lead`) pins |
| 6 boundary tests | 264-321 | REAL | — | both bounds' inclusivity pinned both sides |
| `ac_generation_stale...` | 327-382 | REAL | — | end-to-end seek mint story |
| `no_frame_is_presented_out_of_order_under_shuffled_arrival` | 388-415 | REAL | S4 | F-002: 5/120 permutations, batch arrival only |
| `presented_error_stays_within_policy_bounds...` | 417-484 | REAL | — | per-present ±(threshold/lead) bounds through stall+recovery |
| `audio_master_sample_counter...` | 490-516 | REAL | — | grid drift absorbed, 0 drops |
| `audio_master_stall_drops_late_frames` | 518-533 | REAL | — | 5 s stall, all 10 dropped |
| `never_policy_presents_late...` | 539-579 | REAL | — | supersede-not-late accounting pinned |
| `auto_policy_resolves_to_doc_default` | 581-605 | REAL | — | one-frame threshold behavior |
| `paused_clock_freezes...` | 611-649 | REAL | — | mechanical freeze + repeat budget 3 |
| `stats_conserve_every_admitted_frame` | 651-684 | REAL | — | conservation + presenter-vs-stats cross-check |
| `window_bound_surfaces_as_typed_window_full...` | 691-720 | REAL | — | capacity + handed-back pts |
| `queue_bound_surfaces_as_typed_full...` | 722-757 | REAL | — | Full→drain→retry lands |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 21 | YES | exact sequences/counts/conservation/boundary pins — see §2 mutation list; the two S4s are coverage/diagnosis quality, not vacuity |

No `#[ignore]`, no cfg/env gates → all 21 in the W00 baseline.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "virtual-clock determinism … zero sleeps (doc 03 §8)" | header :1-4 | REAL | grep: no sleep in the file; VirtualClock hand-set |
| "the production shape at v0: scheduler over an injected PlaybackClock, requests through the bounded presentation queue" | header :6-8 | REAL | `step` wiring matches pipeline_host.rs's use (:704, :1324); subject shipped-reachable |
| "deterministic permutations cover the same ground" (as a property framework) | :390-391 | SHALLOW | 5/120 permutations, batch arrival only (F-002) |

## 7. Dead / duplicated / unreachable code

- None in this file. `tests/common/mod.rs` carries `#![allow(dead_code)]` (helpers shared across suites — some used only by neighbors; acceptable for a shared fixture module, noted for the record).

## 8. Dependencies & flags

No feature flags, no env knobs. The suite is part of the default `cargo test --workspace` run (W00 baseline). The tested scheduler has no feature gate in video-engine (crate has no `[features]` per its manifest audit).

## 9. Open questions for the next agent

- The scheduler is proven here and reachable from the CLI — but the W00/W03 baseline says the CLI's clock is never pumped between commands (W03-A01/W13-A01: `pump_wallclock` test-only). So these 21 passing tests prove a scheduler that, in the shipped CLI, only ever ticks at command boundaries. The claim ledger should reconcile "scheduler suite green" with "GUI/CLI never present continuously".
- `presented_error_stays_within_policy_bounds` checks the scheduler's own bounds; the full-pipeline ±10 ms `[PROPOSAL]` tolerance (2.C3/2.E) is untested here — who owns that integration test?
