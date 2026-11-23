# AUDIT: crates/streaming/src/dash/abr.rs
| Field | Value |
|---|---|
| Auditor | W10-A04 |
| Date | 2026-02-14 |
| Lines / functions | 487 / 9 prod fns + 3 types + 4 tests |
| File verdict | UNREACHABLE (real ABR logic; no shipped entry point) |
| Reachable from shipped entry point? | NO — only consumer is `dash::sched` (itself unreachable) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `cargo test -p streaming --lib dash::` → 27 passed/0 failed (own run, 0.08s); rg hunts |

## 1. One-paragraph verdict
This is REAL adaptive-bitrate logic, not a stub: an EWMA throughput estimator (`bytes/ms`, ¾-old+¼-new shift merge), a bandwidth-sorted ladder walk, down-steps on EWMA-starvation (<80% of current rung) OR 6.D1 stall prediction OR buffer below the 6 s low watermark, up-steps gated on EWMA headroom (≥125% of next rung) AND min-dwell (2 segments) AND buffer ≥ 20 s AND cap — one band per axis, one rung per decision, bounded 16-entry switch ledger. Arithmetic is u128 (no overflow), tests exercise every axis and ran green. Defects: the up-switch treats a never-reported buffer level as "filled" (contradicting the module doc), `select_representation_index`'s "ties keep document order" claim is wrong (Rust `max_by_key` returns the LAST tied element), and the switch `history`/`segments_seen` ledger is write-only (never read by any decision or snapshot). Like the rest of the dash module it is unreachable from any shipped binary.

## 2. What actually works (verified)
— `cargo test -p streaming --lib dash::` → `test result: ok. 27 passed; 0 failed; 0 ignored` (own run; includes this file's 4 tests: selection caps/skips, starvation down + recovery up, drained-buffer forced down vs filled-buffer up, stall bypass). Hand-verified unit math: `observe_throughput(1000, 5_000_000)` = 200 B/ms = 1.6 Mbps; `throughput_bps` = r·8000 exactly (1 B/ms = 8000 bit/s).

## 3. Findings
F-001 [S3]
Where: crates/streaming/src/dash/abr.rs:22-25 vs 281-283
Code:
```
//! an up-step needs EWMA headroom over the next rung
//!   AND dwell served AND the buffer above the high watermark AND
//!   the rung under the cap. Between the margins it holds.
```
vs:
```
            let filled = self
                .buffer_s
                .is_none_or(|b| b >= self.config.high_watermark_s);
```
What is wrong: the doc says an up-step NEEDS the buffer above the high watermark; the code treats a host that never reported a buffer level (`buffer_s == None`) as satisfying that condition. A clock-less/buffer-less host can up-switch on throughput alone. Why it matters: a real host that simply forgets to wire `note_buffer_level` gets aggressive up-switching precisely when its buffer state is unknown — the opposite of the conservative posture the doc promises. What correct looks like: `is_some_and` (unknown = hold), or an explicit doc sentence blessing unknown-as-filled. Confidence: HIGH — Verification: read; no test covers up-switch with `buffer_s == None`.
(Reachability caveat: since the module is unreachable from shipped code, user impact is deferred until a lane wires it — kept S3 for the doc/code contract break.)

F-002 [S4]
Where: crates/streaming/src/dash/abr.rs:125-137
Code:
```
/// Ties keep document order. `Err(NoRepresentation)` = nothing
/// selectable.
```
(implementation uses `.max_by_key(|(_, bw)| *bw)`)
What is wrong: Rust's `Iterator::max_by_key` returns the LAST element among equal maxima, so two rungs with identical `@bandwidth` select the later document index, not "document order" (first). Untested claim; no test has a tied ladder. What correct looks like: `.rev()` before `max_by_key`, or fix the doc. Confidence: HIGH — Verification: read (Rust std semantics) — UNVERIFIED by execution (no tied-ladder test exists; mutation reasoning only).

F-003 [S4]
Where: crates/streaming/src/dash/abr.rs:163-164, 300
Code:
```
    /// Bounded switch ledger: `(segments_seen, target_rung)`.
    history: VecDeque<(u64, usize)>,
```
and its only write:
```
        self.history.push_back((self.segments_seen, target));
```
What is wrong: `history` (and `segments_seen`, read only to stamp it) is write-only — no decision, no snapshot, no getter ever reads it. The "anti-thrash ledger" records nothing that influences behavior or observability. Dead data. Confidence: HIGH — Verification: rg — `history` appears only at declaration + push_back + cap-trim.

F-004 [S4]
Where: crates/streaming/src/dash/abr.rs:192-194
Code:
```
        self.ewma_bytes_per_ms = Some(match self.ewma_bytes_per_ms {
            Some(prev) => prev - (prev >> 2) + (rate >> 2),
            None => rate,
        });
```
What is wrong: shift-only merge truncates — for `rate < 4` B/ms (≈32 kbps), `rate >> 2 == 0`, so a stream of sub-4 B/ms samples can never lift the EWMA off a lower value (e.g. prev=1: 1−0+0=1 forever). Downward convergence is fine; upward recovery from near-zero is quantized to the 4 B/ms granularity. Cosmetic for realistic ladders (lowest rung 150 kbps ≈ 18.75 B/ms), real only for ultra-low-rate streams. Confidence: MEDIUM — arithmetic traced by hand; not exercised by tests.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| 7 consts | 37-57 | REAL | — | mirror hls defaults; SWITCH_HISTORY_CAP unused-in-effect (F-003) |
| DashAbrError + Display | 62-88 | REAL | — | dotted codes |
| DashAbrConfig + Default | 91-120 | REAL | — | plain data; all fields read by decide |
| select_representation_index | 127-139 | REAL | S4 | skip-None + cap filter correct; tie claim wrong (F-002) |
| DashAbr::new | 171-183 | REAL | — | — |
| observe_throughput | 187-196 | REAL | S4 | zero-span skip; EWMA truncation (F-004) |
| on_buffer_signal | 200-206 | REAL | — | StallPredicted arms, StallCleared/Filled disarm; Drained/Steady ignored (correct — Drained is a level, not a stall) |
| note_buffer_level | 209-213 | REAL | — | rejects NaN/negative |
| throughput_bps | 217-220 | REAL | — | u128 math, u64::MAX clamp; unit-correct |
| current / snapshot | 224-239 | REAL | — | counters; no history exposure |
| segment_served | 242-245 | REAL | — | dwell + seen tick |
| decide | 253-293 | REAL | S3 | full hysteresis policy; F-001 None-buffer up-switch; defensive None on unknown current index; single-step per call |
| commit | 296-305 | REAL | S4 | bounded history (write-only, F-003) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| selection_is_highest_under_cap_skipping_none_rungs | YES | exact indices under 3 caps incl. nothing-selectable refusal |
| hysteresis_steps_down_on_starvation_and_back_up | YES | down to 400k then 150k by exact bandwidth; recovery off lo asserted; switch_count ≥ 2 |
| low_buffer_forces_down_high_buffer_gates_up | YES | Some(2) down vs Some(0) up — exact indices |
| stall_prediction_bypasses_everything | YES | Some(1) despite healthy EWMA; None after clear |
Untested: tied-bandwidth selection (F-002), up-switch with buffer None (F-001), EWMA recovery from <4 B/ms (F-004).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "an up-step needs … the buffer above the high watermark" | abr.rs:23-24 | FALSE when buffer never reported | F-001 |
| "Ties keep document order" | abr.rs:125 | FALSE (max_by_key keeps last) | F-002 |
| "Rungs with a missing/junk @bandwidth … are skipped, never guessed" | abr.rs:10-12 | REAL | filter_map at L134 + test |
| "[PROPOSAL] algorithm class … stays a Lead decision" | abr.rs:26-28 | REAL (honest label) | watermark/hysteresis shape is what ships |

## 7. Dead / duplicated / unreachable code
Module unreachable from shipped entry points. Write-only: `history`, `segments_seen` (F-003). No duplication (comment claims the hls AbrController's exact EWMA form — consistent with `crate::buffer` pattern).

## 8. Dependencies & flags
No feature gates. Deps: playlists (AdaptationSet), crate::buffer (BufferSignal). No unsafe.

## 9. Open questions for the next agent
- Compare with the HLS `AbrController` (hls/master.rs area): is the claimed "exact form" EWMA identical, or did the two drift? (Cross-file check, not done here.)
