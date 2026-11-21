# AUDIT: crates/streaming/src/dash/abr.rs
| Field | Value |
|---|---|
| Auditor | W10-A06 |
| Date | 2026-11-21 |
| Lines / functions | 486 / 20 (7 consts + 1 enum + 2 Display/Error impls + 1 config + 1 free fn + 11 methods + 4 tests + 2 test helpers) |
| File verdict | UNREACHABLE (REAL, tested policy code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | **NO.** `select_representation_index`/`DashAbr` ← `crates/streaming/src/dash/sched.rs` only (32, 303, 310, 359); `DashScheduler` itself has zero non-test callers (see resolve.rs#part2 audit header); apps/cli touches no dash code. Compiled into the shipped binary via streaming→cli, never called. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -n "DashAbr|decide|select_representation_index" sched.rs` → the full consumer map (decide called at sched.rs:359). `rg -n "BadRepresentation|BadLadder" crates/` → BadRepresentation never constructed. Baseline: `orchrun-test-workspace-nofailfast.log.txt:4990-4993` — 4/4 `dash::abr` tests ok. `sed -n` on all quotes. |

## 1. One-paragraph verdict
A small, disciplined policy object: pure (no I/O, no clock — verified: the only state mutations come from host-fed observations), overflow-safe arithmetic in u128 with explicit truncation guards, bounded switch ledger, and four genuinely adversarial tests (starvation collapse, watermark gating, stall override) that pass in the citable baseline. The defects are at the policy's edges: the stall/drain emergency path is gated behind a throughput sample that the doc's OR-list does not mention (a stall predicted before any fetch completes cannot force a down-step), the up-switch treats an unreported buffer level as "filled" contrary to the doc's AND-list, the "ties keep document order" comment is backwards (Rust's `max_by_key` returns the *last* of equal maxima), and one of the two public error variants is constructed nowhere in the crate. Unreachable from the shipped binary like the rest of the dash module.

## 2. What actually works (verified)
— All 4 tests green in baseline `orchrun-test-workspace-nofailfast.log.txt:4990-4993` (`hysteresis_steps_down_on_starvation_and_back_up`, `low_buffer_forces_down_high_buffer_gates_up`, `selection_is_highest_under_cap_skipping_none_rungs`, `stall_prediction_bypasses_everything`).
— EWMA math hand-checked: `observe_throughput(1000, 5_000_000)` → 200 B/ms (1.6 Mbps); eight samples of `(100, 10ms)` = 10 B/ms drag the 3/4-old EWMA to ≈28 B/ms < 640 kbps threshold → `starved` fires → down-step to mid, exactly what the test asserts (397-404).
— Ladder sort is by value with document indices preserved (`ladder.sort_by_key` at 259); the down/up neighbors are computed in bandwidth space while returned indices stay document-order — verified against the `low_buffer` test's index reasoning (448-451).
— `commit` bounds the history at `SWITCH_HISTORY_CAP` (301-303) and resets dwell — no unbounded growth on a thrashing link.

## 3. Findings

### F-001 [S3] — the emergency down-step (stall/drain) is unreachable until a throughput sample exists, contradicting the documented OR-policy
Where: abr.rs:253-272 (the `?` at 257 precedes the stall/drain checks)
Code:
```rust
    pub fn decide(&mut self, ladder_bandwidths: &[u64]) -> Option<usize> {
        if ladder_bandwidths.is_empty() {
            return None;
        }
        let throughput = self.throughput_bps()?;
```
(and the doc's promise, lines 20-25:)
```rust
//! * **Hysteresis, no thrash** — one band per axis: a down-step
//!   needs the EWMA below `down_margin_percent` of the current rung,
//!   OR a 6.D1 stall prediction, OR the buffer under the low
//!   watermark; an up-step needs EWMA headroom over the next rung
```
What is wrong: `decide` unwraps `throughput_bps()` with `?` *before* examining `stalled`/`drained`. With no EWMA sample yet (a session whose fetches all had `wall_ns == 0`, or a stall predicted before the first fetch completes), `decide` returns `None` — hold — even with `StallPredicted` armed or the buffer under the low watermark. The doc lists stall and drain as independent OR-conditions; the implementation makes every decision, including emergencies, conditional on a throughput sample existing.
Why it matters: the emergency path exists precisely for the moments where measurements are missing or degenerate; gating it on the very observation channel that may be absent in those moments inverts the design. Blast radius is small in the happy path (the first fetch observes throughput), but a zero-duration fetch (cached/local mirror) plus a stall prediction holds the rung — the stall-forced down-step the module advertises silently cannot happen.
What correct looks like: move the stall/drain check before the `?` (down-steps on `stalled || drained` need no throughput), or document that all decisions require one sample.
Confidence: HIGH — Verification: read + hand-trace; the test `stall_prediction_bypasses_everything` calls `observe_throughput` first (472-473), so the suite cannot catch this ordering.

### F-002 [S4] — unreported buffer level counts as "above the high watermark" on the up-switch
Where: abr.rs:281-283
Code:
```rust
            let filled = self
                .buffer_s
                .is_none_or(|b| b >= self.config.high_watermark_s);
```
What is wrong: `is_none_or` makes `buffer_s == None` satisfy the "buffer above the high watermark" conjunct. The doc (23-25) lists "AND the buffer above the high watermark" with no absence carve-out — unknown is treated as healthy.
Why it matters: a host that never wires `note_buffer_level` gets watermark-free up-switching (dwell + headroom only); the policy degrades silently to the throughput-only rule instead of holding. Cosmetic-to-minor, but it is a doc/behavior divergence on a safety conjunct.
What correct looks like: `is_some_and(|b| b >= high)` (absence = hold), or a doc sentence saying absence disables the gate.
Confidence: HIGH — Verification: read.

### F-003 [S4] — "Ties keep document order" is false: `max_by_key` returns the last of equal maxima
Where: abr.rs:125-138
Code:
```rust
/// `None`-bandwidth rungs are skipped (6.C1a's warn-tolerance
/// becomes an explicit skip at selection time — never a guess).
/// Ties keep document order. `Err(NoRepresentation)` = nothing
/// selectable.
pub fn select_representation_index(
```
What is wrong: `Iterator::max_by_key` is specified to return the **last** element among equal maxima, so two rungs with identical `@bandwidth` select the *later* document index. "Ties keep document order" reads as earliest-wins; the code does latest-wins.
Why it matters: doc nit with zero practical impact (equal-bandwidth rungs are interchangeable by definition), but a reviewer documenting selection behavior is told the opposite of what happens.
What correct looks like: `max_by_key(|(i, bw)| (bw, Reverse(i)))` for earliest-wins, or fix the comment to "ties take the later rung".
Confidence: HIGH — Verification: read + Rust std semantics of `max_by_key`.
What correct looks like: see above.

### F-004 [S4] — `DashAbrError::BadRepresentation` is constructed nowhere; its sched mapping arm is equally dead
Where: abr.rs:66-68 (definition), sched.rs:81 (the only reference)
Code:
```rust
    /// The requested representation index is outside the ladder
    /// (`streaming.dash.bad-representation`).
    BadRepresentation,
```
What is wrong: rg over `crates/` finds zero construction sites — the variant exists only in its own `Display` arm and in sched.rs's `From` mapping (`BadRepresentation => BadLadder`), while sched.rs constructs `BadLadder` directly (294, 363) without going through this error. Dead public API surface that implies a check that does not exist.
Why it matters: dead code in the error vocabulary misleads callers into handling an error nobody produces; the ladder-out-of-range condition is actually signaled as a silent `None` hold inside `decide` (260) instead.
What correct looks like: either `decide` returns `Result<Option<usize>, DashAbrError>` refusing `BadRepresentation` when `current` is outside the ladder (instead of silently holding via `position(...)?`), or delete the variant and its mapping arm.
Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| consts (7) | 37-57 | REAL | — | Defaults match docs (25 Mbps cap, 125%/80% margins, dwell 2, 6 s/20 s watermarks, history 16). |
| `DashAbrError` + Display + Error | 61-88 | REAL | S4 (F-004) | One dead variant. |
| `DashAbrConfig` + Default | 91-120 | REAL | — | Plain data. |
| `select_representation_index` | 127-139 | REAL | S4 (F-003) | Skip-None/under-cap/max — no panic, no guess; tie behavior mis-documented. |
| `DashAbr::new` | 171-183 | REAL | — | Trivial ctor. |
| `observe_throughput` | 187-196 | REAL | — | u128 math, zero-span skipped; shift-based 3/4+1/4 EWMA (truncation ≤ 1 B/ms — noise). |
| `on_buffer_signal` | 200-206 | REAL | — | StallPredicted arms; StallCleared/Filled disarm; Steady/Drained no-ops — matches BufferSignal vocabulary. |
| `note_buffer_level` | 209-213 | REAL | — | NaN/negative rejected. |
| `throughput_bps` | 217-220 | REAL | — | `min(u64::MAX)` clamp before cast — no truncation panic. |
| `current` / `snapshot` | 224-239 | REAL | — | Pure accessors. |
| `segment_served` | 242-245 | REAL | — | u32 dwell counter: a >4-billion-segment session could overflow — not realistic; `segments_seen` is u64. |
| `decide` | 253-293 | REAL | S3 (F-001) + S4 (F-002) | Sorting O(n log n) per boundary (n = rungs ≤ ~32 — fine); emergency-before-sample ordering bug; `position(...)?` silently holds on out-of-ladder `current` (see F-004). |
| `commit` | 296-305 | REAL | — | Bounded history, dwell reset. `stalled` intentionally persists until cleared (verified by test 483-484). |
| `DashAbrSnapshot` | 309-323 | REAL | — | Data. |
| tests (4 + helpers) | 328-486 | REAL | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `selection_is_highest_under_cap_skipping_none_rungs` (353) | YES | Three asserts incl. the None-rung skip and the all-broken refusal. |
| `hysteresis_steps_down_on_starvation_and_back_up` (374) | YES | Full cycle with hand-checkable EWMA numbers; recovery assert `current <= 1` is loose but `switch_count >= 2` pins movement. |
| `low_buffer_forces_down_high_buffer_gates_up` (433) | YES | Watermark semantics both directions with exact rung indices. |
| `stall_prediction_bypasses_everything` (465) | PARTIAL | Proves stall overrides health *given* a throughput sample; cannot catch F-001 (it feeds `observe_throughput` first). Mutation: swap the `?`-order in `decide` to sample-last → all four tests still pass. |
| Missing coverage | — | No test with zero throughput samples + stall; no tie case; no `buffer_s == None` up-switch case. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "a down-step needs the EWMA below … OR a 6.D1 stall prediction, OR the buffer under the low watermark" | abr.rs:20-25 | PARTIAL/FALSE at the edge | F-001: all three conditions are additionally gated on a throughput sample existing. |
| "an up-step needs … AND the buffer above the high watermark" | abr.rs:23-25 | PARTIAL | F-002: absence of a buffer report satisfies the conjunct. |
| "Ties keep document order" | abr.rs:125-126 | FALSE | F-003: `max_by_key` takes the last of equal maxima. |
| matrix network-dash evidence: "6.C2 buffer-based ABR — EWMA throughput, up/down margin bands, dwell, stall-forced down-steps over representation ladders" | feature-matrix.toml:1453 | REAL (code + 4 green tests) — UNREACHABLE from shipped binary | Baseline `:4990-4993`; sched.rs:359 is the only decide-caller, itself hostless. |
| "the controller never reads a clock" | abr.rs:18-19 | REAL | No Instant/SystemTime anywhere in the file (read). |

## 7. Dead / duplicated / unreachable code
- Dead: `DashAbrError::BadRepresentation` (zero construction sites) + sched.rs:81 mapping arm.
- Duplication note: this module is a deliberate re-aim of the HLS `AbrController` (6.B2b/6.D1) per its own header — the hls twin lives in hls/master.rs (A08); whether the two drifted is for A08 to compare (the doc claims "the hls controller's exact form" for the EWMA).
- Unreachable: whole module (see header).

## 8. Dependencies & flags
- `playlists::AdaptationSet` (model only), `crate::buffer::BufferSignal` — no I/O deps, no feature gates, no unsafe. Tests build MPDs through the real `parse_mpd_bytes` (not hand-built structs) — good: selection is tested against the actual parsed model.

## 9. Open questions for the next agent
- For A02 (sched.rs:359's caller): what happens to the `Some(target)` return of `decide` — is the rung switch actually applied to the fetch plan, and does `BadLadder` at 363 fire on the same out-of-ladder condition `decide` silently swallows (F-004)?
- For A08 (hls/master.rs): does the hls AbrController share F-001's sample-before-emergency ordering, or is the "exact form" claim in this file's header already drift?
