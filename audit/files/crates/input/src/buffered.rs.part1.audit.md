# AUDIT: crates/input/src/buffered.rs#part1 (lines 1–413)
| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-21 |
| Lines / functions | 413 audited (1-413; boundary cuts `confirm_eof` at :413 — body completes :414-416, covered for coherence; unit tests at :419+ are part2/A09's) / 14 items in range |
| File verdict | REAL |
| Reachable from shipped entry point? | YES. `lumen open` on any local file: `apps/cli/src/demux_host.rs:394/:422` `input::FileAccess::new()` → `file.rs:136 with_buffer_capacity` → `BufferedReader::new`; demux consumes `read_into`/`read_exact_into`/`peek_into` (`crates/demux/src/mkv/ebml.rs`, `avi/riff.rs`). Executed: every baseline smoke open (INDEX.md mp4-smoke + avi-firstvertical transcripts — demux works, 2 tracks, duration, seekable). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | `sed -n '1,420p'` (chunk + confirm_eof tail) · `rg -n 'BufferedReader' crates apps -l` (file.rs, contract.rs, seekable.rs, demux riff.rs, tests) · `rg -n 'FileAccess' apps/cli/src/demux_host.rs` (:394/:422 shipped entry) · `sed -n '2603,2680p' orchrun…log` (input 33/33 unit, 5/5 bounded_reads, 9/9 file_stream, **0 tests** truncation) · `rg -n 'cfg\(feature' crates/input/tests/truncation.rs` (:15 gate) · `grep` over `test-workspace-allfeatures.log.txt` (died at first-vertical evidence BEFORE input binaries — 116 "Running" lines, no truncation binary) · `rg -n 'truncat' buffered.rs` tests (:654/:678/:780/:796 in-file coverage exists, part2) · quote verification sed :205-210, :274-284, truncation.rs :11-15 |

## 1. One-paragraph verdict
The bounded reader is genuinely well-engineered: I traced the position invariant (`source.position() == buf_pos + buffered_bytes()`) through construction, all three seek branches, both read regimes (windowed and bypass), the read-ahead refill, and both borrow twins — it holds on every path; the bypass path (want ≥ capacity) really does drain leftover read-ahead first, write the source bytes straight into caller memory, and do exactly one capacity-bounded read-ahead refill; truncation (source shrunk below open-time length) surfaces a typed `Corrupt::Truncated` rather than a silent EOF; and short reads are data, not errors. Two real defects: `drain_into` bounds its take by `out.capacity()` instead of the caller's requested `want`, so `read()` silently depends on `Vec::with_capacity` returning *exactly* `want` capacity (std only documents "at least") to honor its "read up to want bytes" contract; and the file's headline acceptance suite — `tests/truncation.rs`, 211 lines of card-2.A1 AC tests — is `#![cfg(feature = "contract")]`-gated and has executed ZERO tests in every committed baseline run, while its header claims "CI's `--all-features` always does" — a claim the committed all-features log contradicts (it died fail-fast 116 binaries earlier, never reaching input).

## 2. What actually works (verified)
— Position invariant: traced symbolically through `new` (:84-101, window empty), `seek` (:153-173, all three branches re-derive consistently), `read`/`read_bypassing_window` (:198-260, buf_pos advances by exactly the bytes delivered), `read_into` (:310-354, both regimes), `drain_into`/`drain_leftover_into` (:276-284/:360-368, +take each), `refill_read_ahead` (:266-272, empty-window precondition documented and checked at both call sites — :239 drains ALL leftover since `want ≥ capacity ≥ buffered`, :314 same for `out.len() ≥ capacity`).
— Bypass economics (:228-236): one `want`-sized transfer + one capacity-bounded refill — verified against the code; `Vec::with_capacity` zero-memset claim contrast honest (:234-236).
— Seek-in-window costs no I/O (:155-159); seek-at-window-end drops the window only (:160-167) — the source cursor is provably there by the invariant.
— Bounds: capacity ∈ [1, 16 MiB] typed errors otherwise (:85-90); single reads capped at 64 MiB typed (:202-204); `read_into` has no cap and none is needed (caller owns the memory — correct reasoning, :302-304).
— Truncation semantics: `confirm_eof` (:409-416) compares live `source.length()` to the open-time snapshot — only a *shrink* errors; growth reads on. In-file unit tests cover it and ran: `clean_eof_at_declared_length` (:654), `shrink_mid_read_surfaces_typed_truncation` (:678), exact-read truncation (:780, :796) — within input's 33/33 baseline pass (orchrun :2605-2607).
— Execution: every smoke open rides this reader (demux_host → FileAccess → BufferedReader); AVI lane decodes 60/60 frames through it (INDEX.md).

## 3. Findings

### F-001 [S3]
Where: crates/input/src/buffered.rs:274-284
Code:
```
    /// Move buffered bytes into `out`'s spare capacity, advancing the
    /// window. Never reallocates: the take is bounded by the room left.
    fn drain_into(&mut self, out: &mut Vec<u8>) {
        let room = out.capacity() - out.len();
        let take = room.min(self.end - self.start);
```
What is wrong: The take is bounded by `out.capacity() - out.len()`, but `read()`'s contract is to return **at most `want`** bytes (:175 "Read up to `want` bytes"). `read()` builds `out` with `Vec::with_capacity(want)` (:208) and relies on `capacity() == want` for the contract to hold. std documents `with_capacity` as returning **at least** the requested capacity (Vec docs), not exactly; the exactness is an implementation detail of today's `RawVec`. If a future std (or an allocator-aware change) ever hands back `capacity() > want`, `take` can exceed `want - out.len()` and `read()` returns more bytes than asked — the loop guard `while out.len() < want` (:210) exits on the first over-fill.
Why it matters: A demuxer asking for exactly one box header (say `want = 100`, capacity rounds to 128, window holds 64+ fresh bytes) would receive 128 bytes and silently mis-parse the stream as if 100 were delivered; the corruption is downstream and invisible. Latent today (current std is exact), but the correct quantity to bound by is the request, not the allocation.
What correct looks like: Bound the take by the remaining request: pass `want` down (`let take = (want - out.len()).min(self.end - self.start);`), or have `drain_into` take a `limit` parameter. The borrow twin `read_into` already does this correctly at :347 (`(out.len() - filled).min(...)`) — the two paths disagree on discipline.
Confidence: HIGH on the code fact (read), MEDIUM on triggerability (depends on std behavior the docs do not promise) — Verification: read + std API contract comparison.

### F-002 [S3]
Where: crates/input/tests/truncation.rs:11-15 (evidence for buffered.rs:409-416, this chunk's `confirm_eof`)
Code:
```
//! run with `--features contract` (CI's `--all-features` always
//! does).
#![cfg(feature = "contract")]
```
What is wrong: The card-2.A1 acceptance suite for this chunk's headline behavior — 211 lines, ≥4 tests (`file_truncated_mid_read_surfaces_the_typed_error`, `truncation_below_the_cursor_drains_the_snapshot_then_errors_typed`, `truncation_to_zero_is_typed`, `truncation_lands_mid_refill`) — is compiled out of every default test run, and in the committed baseline evidence it has NEVER executed: (a) baseline #3 (`cargo test --workspace --locked`) — "running 0 tests" for this binary (orchrun :2668-2672 shows the same for the repair run); (b) baseline #4 (`--all-features`) — the run died fail-fast at `-p first-vertical --test evidence` (log tail: `error: test failed, to rerun pass '-p first-vertical --test evidence'`, EXIT=101) before ever reaching the input binaries; (c) the no-fail-fast repair run — no `--all-features`, so 0 tests again. The header's parenthetical "CI's `--all-features` always does" is contradicted by the only committed all-features artifact.
Why it matters: The integration-level truncation story (real temp files shrunk with `set_len` underneath live readers, at several cursor/window positions) has zero committed executions — the exact "cfg-gated tests that never run" pattern, and it hid from the Wave-0 baseline the same way the localization failure did (the no-fail-fast run's stated purpose was to surface exactly this class). The in-file `ScriptSource` unit tests (:678 etc.) cover the semantics, which keeps this at S3 rather than S2 — but the acceptance evidence the card cites does not exist in any run log.
What correct looks like: Either drop the `contract` feature gate (testkit/contract deps are dev-dependencies anyway — `input::contract` is compiled for tests by `dev-dependencies`), or commit a run log of `cargo test -p input --features contract` and stop claiming CI covers it.
Confidence: HIGH — Verification: read + three baseline logs checked directly (plain, all-features, no-fail-fast).

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs + constants | 1-55 | REAL | — | Bounds (64 KiB default, 16 MiB buffer cap, 64 MiB read cap) match code; memory-accounting proof consistent with the implementation. |
| `BufferedReader` struct | 57-74 | REAL | — | Fixed window + indices + `buf_pos` + open-time `declared_len`. No unbounded field. |
| `new` | 84-101 | REAL | — | Typed capacity errors; invariant initialized (window empty, buf_pos = source.position()). |
| `with_default_capacity` | 103-110 | REAL | — | Delegates; Result kept for uniformity (documented). |
| `position` | 112-116 | REAL | — | `&self`, no source touch — matches doc 03's SourceStream shape. |
| `buffer_capacity`/`buffered_bytes`/`source_length` | 118-132 | REAL | — | Pure observers; `source_length` queries live (growth truth). |
| `source`/`source_mut` | 134-142 | REAL | — | Fault-scripting escape hatch; a caller seeking the raw source breaks the position invariant undetected — documented intent ("fault scripting"), noted not flagged. |
| `seek` | 144-173 | REAL | — | Three-branch logic traced; invariant preserved in each; source errors propagate typed. |
| `read` | 175-226 | REAL | S3 | F-001 via `drain_into`; regime split at `want >= buf.len()` (:205) matches the doc's "at or above" (:7). |
| `read_bypassing_window` | 237-260 | REAL | — | Drains ALL leftover (want ≥ capacity ≥ buffered), writes through, one refill, truncate(filled) — correct. |
| `refill_read_ahead` | 266-272 | REAL | — | Empty-window precondition holds at both call sites (traced). |
| `drain_into` | 274-284 | REAL | S3 | F-001: capacity-bounded take. |
| `read_into` | 286-354 | REAL | — | Borrow twin; bypass split at `out.len() >= buf.len()`; small path bounds the take correctly (:347) — the discipline `drain_into` lacks. |
| `drain_leftover_into` | 356-368 | REAL | — | Correct bound (out.len()); advances buf_pos by take. |
| `read_exact_into` | 370-391 | REAL | — | Loop can't spin (slice non-empty when filled < len); typed truncated(expected=target position, found=stream end). |
| `peek_into` | 393-403 | REAL | — | Non-advancing, no I/O, never fabricates (≤ buffered). Takes `&mut self` unnecessarily (pure read — API wart, noted). |
| `confirm_eof` | 405-416 | REAL | S3 | Semantics correct (shrink-only); its dedicated acceptance suite never runs (F-002). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| in-file unit tests (part2, :419-828; 14 tests) | YES | `positions_and_readahead_are_exact` (:494) pins the invariant; `large_reads_bypass_the_window_in_one_source_call` (:541) counts source calls; `bypass_reads_serve_leftover_read_ahead_first` (:565); `shrink_mid_read_surfaces_typed_truncation` (:678); `read_exact_into_fills_or_fails_typed` (:768); `peek_into_never_advances_and_never_fabricates` (:802). All ran: input 33/33 (orchrun :2605-2607). |
| `tests/bounded_reads.rs` (5) | YES | Ran and passed (orchrun :2613). |
| `tests/file_stream.rs` (9) | YES | Ran and passed (orchrun :2617). |
| `tests/truncation.rs` (4) | NEVER RAN | `#![cfg(feature = "contract")]` — 0 tests in every committed lane (F-002). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "no unbounded buffering" proof (steady-state = fixed buffer + one bounded read) | buffered.rs:16-25 | REAL | Code matches; bounded_reads.rs ran in baseline (5/5). |
| "A source that shrinks below its open-time length surfaces Corrupt::Truncated, never a silent EOF" | buffered.rs:13-14 | REAL | `confirm_eof` :409-416 + unit test :678 pass; but the integration suite for it never runs (F-002). |
| "requests at or above the capacity bypass the window entirely" | buffered.rs:6-10 | REAL | :205-206, :315 — verified. |
| "run with `--features contract` (CI's `--all-features` always does)" | truncation.rs:12-14 | FALSE (as evidence) | The committed all-features run died at first-vertical before reaching input (log tail EXIT=101); no committed run has executed this suite. |

## 7. Dead / duplicated / unreachable code
None dead in range. Note: `read()` (Vec-returning) and `read_into` (borrow) intentionally duplicate the regime logic — documented twins (7-c part 2), acceptable.

## 8. Dependencies & flags
`media_types::EngineError`, `crate::error` (typed constructors), `crate::seekable::{RawRead, Seekable}` — all used. The `contract` feature (`input/Cargo.toml:30`, empty feature) gates only `src/contract.rs` + this test suite — the gate that causes F-002.

## 9. Open questions for the next agent
- Part2 (A09, :414-828): the unit tests' `ScriptSource` — does it model `length()` returning live values (needed for the truncation tests to be meaningful)? And confirm the 14-test inventory matches my table.
- Whoever owns `crates/input/tests/truncation.rs` (not in any wave-3 list I can see): F-002's fix (ungate or commit evidence).
- REQUEST-ORCH-RUN: `cargo test -p input --features contract --release --locked` — executes the never-run truncation acceptance suite (4 tests) and closes the F-002 evidence gap.
