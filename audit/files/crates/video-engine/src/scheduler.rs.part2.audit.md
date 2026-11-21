# AUDIT: crates/video-engine/src/scheduler.rs#part2 (chunk: lines 571-1141 of 1712)
| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-21 |
| Lines / functions | 571 audited / 19 units (incl. 2 in-file tests, 3 free fns) |
| File verdict (chunk) | REAL (implemented, reachable from the shipped `lumen` CLI via the pipeline host, exercised by the crate's test suite which runs green in baseline) |
| Reachable from shipped entry point? | YES — `apps/cli/src/pipeline_host.rs:84-86` imports `PresentationScheduler`/`PresentAction`/`PushOutcome`; the CLI's video pipeline host constructs and ticks it. Baseline: video-engine tests green in audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt (4534 passed / 7 failed; none in video-engine); doc-test `crates/video-engine/src/lib.rs (line 95)` ok in the same run. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `sed -n 571,1141p` (full chunk); `rg -n "pub fn \|fn \|mod tests"` (inventory, true line numbers); Read-verification of every quote (Bash display eats `[m` after `#`); `rg` callers: `PresentationScheduler` used only by `apps/cli/src/pipeline_host.rs` + tests; `rg -n "pub struct VideoFrame" -A20 media-types/src/frame.rs` + `FrameStorage` (clone = Arc refcount, zero-copy claim plausible); `rg` over tests/scheduling.rs, zero_alloc_scheduler.rs for held/supersede coverage. |

## 1. One-paragraph verdict
This chunk is the scheduler's heart — `push` (window admission, PTS-gap snap
arming, missing-PTS inference/hold), `tick` (anchor-epoch seam, generation
purge, late purge with pre-roll hold exemption, largest-due selection with
supersede, bounded repeat, A/V error sampling, sync-ladder actuators), the
accessors, and the free helpers (`insert_sorted`, `saturating_add`, `is_late`).
The code is genuinely careful: i128 widening everywhere timestamps are
compared, saturating arithmetic, strict-inequality late bound, single-shot
purge-skip, published-generation gating for selection and wait hints, and
every drop path counted. I traced the M10 pre-roll hold interplay (future-gen
frames block neither purge judgment nor selection nor the wait hint) and the
WidenLate/ExtendEarly actuator bounds — both behave as documented. The one
substantive finding is a documented content-loss design in the missing-PTS
hold slot (last-wins drops all-but-newest leading frames); the rest is edge
hygiene. In-file tests are real and mutation-sensitive.

## 2. What actually works (verified)
— Crate test suite green in the no-fail-fast workspace run: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt (4534/7/49; zero video-engine failures listed in its summary footer).
— Zero-alloc discipline evidence exists: crates/video-engine/tests/zero_alloc_scheduler.rs::dropping_ticks_allocate_nothing_after_warm_up (asserts no allocation on the tick path after warm-up — the D-1/D-8 claim in the `tick` docs at line 693-694 has a real test behind it).
— PresentRequest clone cost claim verified at the type level: `FrameStorage::Cpu(SharedBytes)` (frame.rs:176-181) — clone is a refcount bump, so `self.last = Some(request.clone())` allocates nothing.

## 3. Findings

### F-001 [S3]
Where: crates/video-engine/src/scheduler.rs:668-675
Code:
```
                    _ => {
                        if self.held_missing.is_some() {
                            self.stats.missing_pts_dropped += 1;
                        }
                        self.held_missing = Some(frame);
                        self.stats.missing_held += 1;
                        PushOutcome::Held
                    }
```
What is wrong: the hold slot is one frame deep and **last-wins** — while no
inference base exists (no window tail, no last presented: i.e., before the
first schedulable frame of a stream), every newly arrived missing-PTS frame
permanently discards the previously held one. A stream that opens with N
missing-stamp frames loses N-1 of them by design; only the newest survives to
be inferred once a base appears.
Why it matters: it is real video-content loss on a plausible malformed/legacy
input (leading frames without timestamps), and a bounded alternative existed —
the window itself is capped at `max_pending`, so a small hold-queue (or
admitting held frames into the window with inferred-from-sequence slots) would
preserve them. The behavior is *documented* (PushOutcome::Held doc,
scheduler.rs:115-120: "at most one — a newer missing-stamp frame replaces an
older held one, which is dropped and counted") and *counted*
(`missing_pts_dropped`), so it is a deliberate trade, not an accident — but
the trade silently eats content where the packet-queue discipline (Q3: never
swallow fresh data) hands frames back instead.
What correct looks like: hold up to a small bounded queue of missing-stamp
frames (or infer from a demux-side sequence counter), so a leading
missing-PTS run loses nothing; keep the counter for the true overflow.
Confidence: HIGH — Verification: read (code + doc quote verified; inline test
`uninferable_missing_pts_is_held_never_presented_never_lost_silently` at line
1338, part3's range, exercises exactly one replacement).

### F-002 [S4]
Where: crates/video-engine/src/scheduler.rs:595-603 vs 604-642
Code:
```
                        self.stats.full_rejections += 1;
                        return PushOutcome::WindowFull {
                            frame,
                            capacity: self.cfg.max_pending.get(),
                        };
                    }
```
(the full-window refusal precedes the 3.B1 discontinuity seam at 604-642)
What is wrong: the window-cap check runs *before* the forward-PTS-gap
detector, so a frame that would arm snap mode is refused with `WindowFull`
without arming it. The seam arms one push late whenever the window is full at
a discontinuity.
Why it matters: the snap exemption's purpose (exempt the post-gap present from
late-drop accounting) is delayed by one admission in exactly the burst
condition (decoder running ahead) where a gap is likely. Impact is one tick of
purge accounting; the frame is handed back intact (Q3 discipline holds).
What correct looks like: run the gap check before the capacity check (it
mutates only sync state, not the window), or arm snap on the refused frame
too.
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: crates/video-engine/src/scheduler.rs:1031-1052
Code:
```
            (Some(base), Some(Nanoseconds(dur))) => {
                if self.window.len() >= self.cfg.max_pending.get() {
                    self.stats.missing_pts_dropped += 1;
                } else {
```
What is wrong: when the held frame finally becomes inferable but the window
is full at admission, the frame is dropped (counted) rather than re-held —
`held_missing.take()` at the top means it does not go back to the slot. The
full-window condition is transient (a present frees a slot next tick), so a
retry-one-tick-later policy would preserve it.
Why it matters: the held frame — the only survivor of F-001's funnel — can be
lost at the last step to a transient condition. Counted, bounded, rare.
What correct looks like: re-hold when the refusal is capacity (the same
distinction `push` makes between MissingDropped and Held).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PresentationScheduler::new` (tail; starts 564, part1) | 571-578 | REAL | — | plain field init |
| `push` | 583-679 | REAL | S3/S4 | generation boundary strict `<` (equal flows, future waits — correct Q2); convert-Err → UnschedulablePts with verbatim error; full-window refusal hands frame back; F-002 ordering; F-001 held last-wins; inference = base+duration saturating |
| `tick` | 701-905 | REAL | — | anchor-epoch seam (first-observe records, change resets+arms, single-tick purge_skip); step-1 generation purge retains current gen, clears held/last of old gens, counts stale; `admit_held` retry; tick_index wrapping_add — wrapping never reached in practice, `is_multiple_of` cadence sound; late purge stops at first future-gen frame (M10 verified) and honors purge_skip once; selection `take_while(gen<=published && pts<=now+lead)` — a held front frame blocks the scan (documented); superseded prefix drained into the fixed-capacity trail with exact lifetime counts; e_v sampled only when Running, i128→i64 clamped, EMA/abs-max saturating; repeat bounded by effective budget, `last.clone()` is refcount-only; band classify + cadence-gated actuators |
| `effective_late_threshold` | 909-915 | REAL | — | widened.max(configured) — never narrower; None policy → None (Never) |
| `effective_max_repeat` | 919-922 | REAL | — | saturating_add of the ladder extension |
| `apply_sync_ladder` | 932-982 | REAL | — | Hold: take widened, count change, reset extension; WidenLate: guards (Never policy, no duration, no EMA, drop budget, no-change, change limit) all present and ordered; ExtendEarly: take widened, set extension — bounds match the doc contract |
| `cfg` / `stats` / `pending_len` | 984-998 | REAL | — | pure accessors |
| `holds_missing` / `last_presented` / `current_generation` | 1002-1016 | REAL | — | pure accessors |
| `sequence_base` | 1021-1029 | REAL | — | window tail else last presented — the documented inference base |
| `admit_held` | 1031-1052 | REAL | S4 | F-003 drop-on-full at admission |
| `insert_sorted` | 1054-1066 | REAL | — | O(n) linear scan, stable among equals (arrival order); n ≤ max_pending (bounded) |
| `saturating_add` | 1068-1071 | REAL | — | i128 clamp — the media-clock discipline |
| `is_late` | 1078-1084 | REAL | — | strict `<` (on-bound presents); None → false (Never policy) |
| tests `cfg()` | 1094-1105 | REAL | — | hand-checkable constants (10 ns frames) |
| tests `presents_in_order_under_a_normally_advancing_clock` | 1108-1131 | REAL | — | asserts exact per-slot presentation + zero drops — honest |
| tests `early_frames_are_held_until_their_slot` (start; continues into part3) | 1133-1141 | REAL | — | boundary: chunk ends mid-test; part3 (W03-A03) owns the rest |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `presents_in_order_under_a_normally_advancing_clock` (1108) | YES | exact-slot assertions on 50 frames plus zero-drop counters — breaks on any off-by-one in the due bound or purge |
| `early_frames_are_held_until_their_slot` (1133) | YES | asserts background action + wait-hint values before the slot (visible in part3's half) |
| `dropping_ticks_allocate_nothing_after_warm_up` (tests/zero_alloc_scheduler.rs:54) | YES (alloc-based) | asserts zero allocations across dropping ticks — breaks if the drop path allocates (the D-1 inline-array claim) |
| inline `uninferable_missing_pts_is_held…` (1338, part3 range) | YES | drives F-001's exact replace-and-count path |
Fixtures: `VirtualVideoClock` is an injected clock (the sanctioned time double), not a mock of the scheduler.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Missing presentation timestamps are never invented silently … else hold the frame until they are" | scheduler.rs:44-47 module doc | REAL with content-loss caveat | push:652-677 + admit_held; caveat = F-001 (held frames can be *dropped*, though never silently) |
| "Zero heap traffic (D-1/D-8): the drop trails are fixed-capacity inline arrays — measured by tests/zero_alloc_scheduler.rs" | scheduler.rs:693-694 | REAL | tests/zero_alloc_scheduler.rs exists with the warm-up assertion; DropTrail (lines 208-281, part1) is a fixed array |
| "the purge stops at the first one [pre-roll hold]" (M10) | scheduler.rs:789-800 | REAL | purge loop breaks on `front.frame.generation > published` (verified in read) |
| "the largest pts ≤ now + target_lead, among PUBLISHED-generation frames only" | scheduler.rs:29-31 module doc | REAL | selection take_while + drain(..due_count-1) + pop_front |
| snap threshold "default ½ the doc 03 §4.4 audio queue duration cap" | scheduler.rs:616-617 | REAL-UNPROVEN (this chunk) | default lives in SchedulerCfg (policy.rs, part1/other agent's range) — not re-verified here |

## 7. Dead / duplicated / unreachable code
- None in this chunk. All accessors are consumed (`pipeline_host.rs` reads
  `scheduler_stats`, `queue_stats`; tests use the rest).

## 8. Dependencies & flags
- Depends on media-clock (`PlaybackClock`, `DriftEstimator`, `SyncBands`),
  media-types (VideoFrame/Stamp/Timestamp), crate policy/cfg. No feature
  gating. `#[must_use]` discipline on all outcomes/accessors (line 700
  verified via Read: `#[must_use = "a tick outcome ignored is a frame
  presented late, dropped silently, or never shown"]`).

## 9. Open questions for the next agent
- part3 (W03-A03): the inline test module 1086-1712 — verify
  `preroll_holds_present_in_pts_order_once_the_generation_publishes` (1632)
  actually flips the clock's published generation mid-test (it is the M10
  integration proof).
- The `purge_skip` flag is set by BOTH the push-side gap seam (639) and the
  tick-side anchor seam (~733) — if both fire between two ticks, one flag
  covers both (single skip). Confirm the one-tick skip is the intended
  coupling (I believe it is; both seams describe "the next tick").
