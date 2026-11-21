# AUDIT: crates/streaming/src/hls/master.rs#part1 (lines 1–594)
| Field | Value |
|---|---|
| Auditor | W10-A07 |
| Date | 2026-11-21 |
| Lines / functions | 594 (of 1190) / 20 fns + 6 types + 5 consts |
| File verdict | UNREACHABLE (real, tested code — no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — `open_master` callers: in-file tests + `crates/streaming/tests/hls_master_e2e.rs` only |
| Findings | S0: 0 S1: 1 S2: 1 S3: 2 S4: 2 |
| Commands run | `rg -n "open_master" --type rust apps crates fuzz tests` → only master.rs + tests/hls_master_e2e.rs; `rg "streaming::hls" apps/` → none; `rg -n "BANDWIDTH" crates/playlists/src/hls.rs` (rows without BANDWIDTH are skipped, none fabricated as 0); `sed -n` on all quoted ranges; baseline INDEX #3 (all streaming tests green). |

## 1. One-paragraph verdict
The HLS master-playlist lane (variant selection, EWMA/hysteresis ABR controller, boundary-switching adaptive session) is real, careful code — typed refusals everywhere, bounded history, u128 band math — and it is exercised end-to-end by an in-repo e2e test over a real mockhttp socket. But no shipped entry point can reach it (`open_master` has zero callers outside the crate's own tests), so the feature-matrix "network-hls: partial" row describes a library, not a product capability. Inside the dead lane there is one genuine crash path (a segment body larger than the retention budget empties the retained window and the next `serve` iteration subtracts `pos - retained_from` under u64 and panics on the slice — the call is made from this range at :592, the panic sites sit in part2's `absorb`/`serve`), a validation gap that arms it (`max_retained_bytes = 0` passes `validate`), an ABR state divergence on failed switches (controller commits before the switch is confirmed), and a tie-break that contradicts the module doc.

## 2. What actually works (verified)
— Selection under cap: `select_variant_index(&master, cap)` filter+max — pinned by tests at :875-887 (would fail if the filter or cap comparison were inverted). Baseline `cargo test --workspace` (INDEX #3) passed all streaming tests on 2026-11-21, including `hls_master_e2e` (mockhttp real-socket lane, cited in feature-matrix row `network-hls` evidence).
— Hysteresis band math verified by trace: down = `t·100 < bw·down%` (:279-280), up = `t·100 ≥ next_bw·(100+up%)` AND `next_bw ≤ cap` AND dwell (:287-297) — all in u128, no overflow for realistic values; history bounded at 16 (:308-310).

## 3. Findings

### F-001 [S1] Segment body larger than the retention budget → u64 underflow + slice panic in the next serve iteration (panic sites in part2; trigger call in this range)
Where: crates/streaming/src/hls/master.rs:586-592 (trigger; `absorb`/`serve` at :603-636 are part2/A08's range)
Code:
```
        self.boundaries.push(SegmentBoundary {
            offset: self.fetched_upto,
            media_sequence: seg.abs_seq,
            duration: seg.duration,
            discontinuity: seg.discontinuity,
        });
        self.absorb(body);
```
What is wrong: `fetch_next_segment` runs when `pos >= fetched_upto` (serve's guard, :629). `absorb` (part2, :603-617) then does: `fetched_upto += body.len()`; when `retained.len() + body.len() > max_retained_bytes` AND `body.len() > max_retained_bytes` it keeps nothing and sets `retained_from = fetched_upto` (:614). The serve loop's next iteration sees `pos < fetched_upto` (true: `pos == old fetched_upto < new fetched_upto`) and computes `let at = (self.pos - self.retained_from) as usize;` (:624) — `pos < retained_from` → u64 underflow (wraps in release) → `&self.retained[at..at + take]` slices an empty Vec at a wrapped index → **panic**, in release too (slice bounds are always checked).
Why it matters: `max_retained_bytes` is a memory budget, not a correctness invariant; a caller legitimately configuring e.g. 1 MiB retention (passes `validate`, ≤ 1 GiB) with an ordinary 4 MiB HLS segment crashes the session on the first read. Retention overflow should be a typed refusal (the file already has the vocabulary: `streaming.hls.seek-unretained`), never a panic.
What correct looks like: `absorb` records the unretained span and `serve` refuses typed (`streaming.hls.segment-unretained`) when `pos` falls outside `[retained_from, fetched_upto)`; or refuse bodies over the budget at fetch time.
Confidence: HIGH — Verification: read (full trace across the :555-599/:603-636 boundary); execution UNVERIFIED (no cargo). Cross-reference for A08 (part2 owner).

### F-002 [S2] `validate` accepts `max_retained_bytes = 0` — a config that deterministically arms F-001
Where: crates/streaming/src/hls/master.rs:362-384
Code:
```
        if self.playlist_response_limit == 0 || self.segment_response_limit == 0 {
            return Err(mint(
                ErrorKind::ResourceLimit {
                    limit: LimitKind::Memory,
                    budget: 0,
                    attempted: 0,
                },
                "streaming.hls.config",
            ));
        }
        if self.max_retained_bytes > MAX_RETAINED_BYTES {
```
What is wrong: zero response budgets are refused; a zero *retention* budget is not (only `> MAX_RETAINED_BYTES` is). With `max_retained_bytes = 0`, the first absorbed body takes the `body.len() <= max` == false branch (`0 < body.len()`), `retained_from = fetched_upto`, and the very first `read` panics exactly as F-001 — with a config the API itself blessed.
Why it matters: `validate`'s documented purpose is "typed refusal, never a clamp" for budgets; the zero case is the cheapest possible refusal and it is missing.
What correct looks like: `if self.max_retained_bytes == 0 { return Err(... ResourceLimit ... "streaming.hls.config") }`.
Confidence: HIGH — Verification: read.

### F-003 [S3] `decide` commits the switch before it is confirmed — failed switches leave divergent ABR state
Where: crates/streaming/src/hls/master.rs:560-564 (with :258-261, :303-312)
Code:
```
        if self.next > 0
            && let Some(target) = self.abr.decide(&self.master.variants)
        {
            self.switch_variant(target)?;
        }
```
What is wrong: `AbrController::decide` is documented as "the controller has already committed: dwell reset, history pushed" (:258-261) and `commit` (:303-312) mutates `current`, resets dwell, counts the switch — all *before* `switch_variant` loads the new playlist. If that load fails (transport error, or `streaming.hls.variant-sequence-missing` at :532-542), the error propagates but the controller now believes it is serving `target`: `current` points at the unswitched-to variant, dwell was reset, `switch_count`/history recorded a switch that never happened. On a host retry the next `decide` compares throughput against the wrong rung's bandwidth and can immediately "up-switch" back, double-counting.
Why it matters: a single transient fetch failure mid-session poisons every later ABR decision and the metrics; the old plan (still loaded, still playable) is the correct rollback.
What correct looks like: two-phase — `decide` proposes, the session confirms via a `confirm_switch()`/`abort_switch()` pair after `switch_variant` succeeds.
Confidence: MEDIUM-HIGH — Verification: read (control flow traced; no execution of the failure path in tests — part2 tests only exercise successful switches).
### F-004 [S3] Tie-break contradicts the documented policy ("ties keep playlist order")
Where: crates/streaming/src/hls/master.rs:141-150
Code:
```
pub fn select_variant_index(master: &MasterPlaylist, max_bandwidth_bps: u64) -> Option<usize> {
    master
        .variants
        .iter()
        .enumerate()
        .filter(|(_, v)| v.bandwidth <= max_bandwidth_bps)
        .max_by_key(|(_, v)| v.bandwidth)
        .map(|(i, _)| i)
}
```
What is wrong: Rust's `max_by_key` returns the **last** of equally-maximal elements. The module doc (:13-14) says "Ties keep playlist order (the producer's own ladder order is the tie-break…)" — the natural reading (and the common ABR convention) is first-listed wins; the code selects the last-listed of a tied pair. No test pins a tie (tests at :875-887 use distinct bandwidths), so the doc's contract is unverified and the code contradicts it.
Why it matters: masters with duplicate `BANDWIDTH` rows (real packagers emit them) silently pick the opposite rendition from the documented policy; whichever way is intended, doc and code disagree and nothing tests it.
What correct looks like: first-wins via `filter(...).enumerate().rev().max_by_key(...)` on `(bandwidth, Reverse(index))`-equivalent, or amend the doc to "last wins" and pin it with a test.
Confidence: MEDIUM (doc reading) / HIGH (max_by_key semantics) — Verification: read.
### F-005 [S4] EWMA truncation: samples under 4 bytes/ms contribute exactly zero
Where: crates/streaming/src/hls/master.rs:205-210
Code:
```
        let rate = u128::from(bytes) * 1_000_000 / u128::from(wall_ns);
        self.ewma_bytes_per_ms = Some(match self.ewma_bytes_per_ms {
            // 3/4 old + 1/4 new — shift-only fixed point (D-2).
            Some(prev) => prev - (prev >> 2) + (rate >> 2),
            None => rate,
        });
```
What is wrong: `rate >> 2` floors; any sample below 4 bytes/ms (< 32 kbps) adds 0 while the decay term `prev >> 2` keeps subtracting — on links that slow the EWMA can only fall, never rise, so up-steps are permanently unreachable there.
Why it matters: correctness corner on very slow links (realistic ladders start ≥ 100 kbps, hence S4); the doc's "milliseconds, not nanoseconds" mitigation (:163-165) addresses a different truncation.
What correct looks like: accumulate the remainder (fixed-point numerator), or compare in bytes/ms directly against `bandwidth/8000` thresholds without the shift merge.
Confidence: HIGH — Verification: read.

### F-006 [S4] `100 + up_margin_percent` is u32 arithmetic — overflow on extreme configs; margins never validated
Where: crates/streaming/src/hls/master.rs:290-291
Code:
```
            let next_bw = variants[next].bandwidth;
            let headroom = u128::from(throughput) * 100
                >= u128::from(next_bw) * u128::from(100 + self.config.up_margin_percent);
```
What is wrong: `100 + self.config.up_margin_percent` is evaluated in `u32`; any margin > `u32::MAX − 100` overflows (debug panic, release wrap → nonsense headroom). `validate()` (:362-384) never bounds the margins or dwell either.
Why it matters: the config struct is public API; pathological values produce a panic (debug) or inverted switching (release) instead of the file's signature typed refusals.
What correct looks like: `u128::from(100) + u128::from(self.config.up_margin_percent)` plus margin range checks in `validate`.
Confidence: HIGH — Verification: read.

## 4. Per-function table (part1 range; `fetch_next_segment` audited in full — its tail :595-599 crosses into part2)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| consts (5) | 91-109 | REAL | — | values match docs (25 Mbps cap, 25%/80% band, dwell 2, history 16) |
| `AbrConfig` + `Default` | 112-136 | REAL | — | plain data; defaults consistent |
| `select_variant_index` | 141-150 | REAL | S3 | F-004 tie-break; empty → None → `no-variant` refusal ✓ |
| `AbrController::new` | 183-194 | REAL | — | trivial, correct |
| `observe_throughput` | 199-211 | REAL | S4 | F-005 truncation; zero-span skipped ✓; u128 product ✓ |
| `on_buffer_signal` | 215-221 | REAL | — | matches documented signal semantics |
| `throughput_bps` | 227-230 | REAL | — | ×8000 bytes/ms→bits/s; saturating clamp ✓ |
| `current` / `snapshot` | 234-249 | REAL | — | trivial accessors |
| `segment_served` | 253-256 | REAL | — | counters only; u32 dwell could overflow after 4G segments on one variant — unreachable in practice |
| `decide` | 263-300 | REAL | S4 | F-006; band math u128 ✓; stall bypasses dwell per doc ✓; `current` not-in-ladder → hold (None) ✓ |
| `commit` | 303-312 | REAL | S3 | F-003 (commit-before-confirm); history bounded ✓ |
| `AbrSnapshot` | 318-330 | REAL | — | data |
| `HlsMasterConfig` + `Default` | 333-354 | REAL | — | budgets explicit ✓ |
| `HlsMasterConfig::validate` | 362-384 | REAL | S2 | F-002 zero-retention gap |
| `HlsMasterStats` | 388-406 | REAL | — | data |
| `PlannedSegment` | 409-418 | REAL | — | data |
| `HlsAdaptiveStream` accessors (`master`/`abr_snapshot`/`stats`/`segment_boundaries`) | 448-474 | REAL | — | trivial |
| `note_throughput`/`on_buffer_signal` | 479-487 | REAL | — | host seam, delegates |
| `load_variant_plan` | 491-522 | REAL | — | B1a refusals complete (ENDLIST/empty/KEY/BYTERANGE/MAP) ✓; `requests` counted only on success (failed variant fetches undercount — cosmetic); `abs_seq = media_sequence + i` ✓ |
| `switch_variant` | 528-552 | REAL | — | sequence alignment + typed `variant-sequence-missing` ✓; `plan[0].discontinuity = true` safe (drain leaves ≥1) ✓; failure leaves controller divergent (F-003) |
| `fetch_next_segment` | 555-599 | REAL | S1,S3 | F-001 trigger (:592 absorb); F-003 (decide-then-switch); status mapping ✓; boundaries recorded pre-absorb ✓ |

## 5. Test honesty
Tests live at :772-1189 (part2, A08). Selection tests (:875-887) would fail if the cap filter or comparison broke; the hysteresis/switching tests script throughput through `note_throughput` and assert exact variant sequences — they would fail if the band math or `commit` regressed. **No test pins**: tie-break (F-004), failed-switch state (F-003), segment > retention budget (F-001), zero-retention config (F-002).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Ties keep playlist order" | master.rs:13-14 | FALSE (last-wins `max_by_key`) | F-004 |
| "highest BANDWIDTH at or under the cap … refuses typed (streaming.hls.no-variant)" | master.rs:11-17 | REAL | :141-150 + test :883-887 |
| "a StallPredicted forces a down-step at the next boundary" | master.rs:32-34 | REAL | :281-283 (and keeps stepping down each boundary while stalled — consistent with "an emergency is an emergency" :102-103) |
| "The switch history is bounded; the counters are not" | master.rs:43-44 | REAL | :308-310 cap 16; u64 counters |
| "A variant whose window does not carry that sequence refuses typed" | master.rs:49-51 | REAL | :532-542 |
| "every variant playlist must pass the B1a playability table" | master.rs:59-61 | REAL | :496-510 |
| network-hls "partial" (win/linux/macos) with 6.B2a/6.B2b evidence | docs/feature-matrix.toml:1431-1441 | REAL-UNPROVEN + UNREACHABLE from shipped binary | code+tests exist; `open_master` unreachable (call chain §1) |

## 7. Dead / duplicated / unreachable code
Whole module unreachable from shipped entry points (see §1). `AbrSnapshot`/`stats()` are "[PROPOSAL] wiring point" surfaces (:238-239, :315-317) with no consumer outside tests — the milestone-3 event-hub wiring does not exist yet.

## 8. Dependencies & flags
Depends on `playlists` (master parse), `plugin_api` (SourceStream), `media_types` (errors), `crate::buffer::BufferSignal`, `crate::fetch`. No feature gates (`hls/mod.rs:29 pub mod master` unconditional). Part of the workspace build (baseline #1 clean, #3 tests green).

## 9. Open questions for the next agent
- A08 (part2): own `absorb`/`serve` — confirm F-001's underflow trace and the `read`/`seek`/`size` impls; check `supports_seek() -> true` (:651-655) against the seek-unretained refusal (doc says "within the retained prefix" — `true` overstates capability to hosts that probe before seeking).
- Whoever owns the buffer model (6.D1): confirm `BufferSignal::Steady`/`Drained` no-op semantics match the host's expectations.
- Orchestrator: no REQUEST-ORCH-RUN for this file (all findings are code-trace deterministic).
