# AUDIT: apps/web/src/io/staged_source.rs#part2 (lines 610–1218)
| Field | Value |
|---|---|
| Auditor | W12-A09 |
| Date | 2026-11-21 |
| Chunk | part2 of 2 (part1 = lines 1–609: module docs, config, types, `open`/`absorb`/`ready`/`pending_request`, `seek_to` head — A10's). part2 = 610–1218: `seek_to` tail, cache internals, `mint_request`, `gather`, `SourceStream` impl, all tests. `seek_to` (562–624) straddles the boundary; audited here in full with the straddle noted. |
| Lines / functions | 609 / 24 (15 fns/methods + 3 test helpers + 9 tests, incl. straddling `seek_to`) |
| File verdict | UNREACHABLE (real, internally consistent, natively unit-tested code; the fetch lane is not wired into the web host's open path — nothing constructs a `StagedSource` outside this file's tests) |
| Reachable from shipped entry point? | NO. Grep `StagedSource` over `apps/web`: definition + `io/mod.rs:76` re-export + this file's tests only. `open.rs`/`host.rs` never construct it (the blob lane is the wired one). `ui/e2e/web/site/harness.js:504-516` asserts `session.open` with a `url` source REFUSES (`unsupported-for-host`, capability `http-streaming`) — the browser matrix's own scenario pins the lane as pending. The wasm module does build (baseline `gate-web_budget___check.log.txt`: `floor` raw 1,302,846 B / gzip 301,101 B, PASS) but nothing packages or serves it outside the e2e harness. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | Grep `StagedSource` (apps/web → def/re-export/tests only); Grep `fetch\|Fetch` + `blocked` (apps/web/src → capabilities.rs rows, harness assertions); Read harness.js:495-530,660-710; Read apps/web/Cargo.toml + src/lib.rs; python3 walk of docs/web/browser-matrix-20261110.json (scenario outcomes); grep baseline test logs for `lumen.web` (compiled, never tested); sed -n verification of every quote below. |

## 1. One-paragraph verdict
The second half of the staged-window fetch source is careful, honest-about-limits code: a sorted window cache with aging + cursor-safe eviction (I traced the `unwrap_or(0)` eviction fallback — it is sound, because when every window covers the cursor, removing the lowest-start one still leaves the cursor covered), a monotonic scan frontier, saturating arithmetic throughout, and a genuinely strong native test suite that drives the REAL AVI fixture through the REAL probe and demuxer. But it is a lane to nowhere: `StagedSource` has zero callers outside its own tests, the web host refuses `url` opens with the typed `unsupported-for-host` error, and the browser-matrix scenarios named "fetch-range-*" test the HARNESS'S OWN static server, not the engine. The defects found are real but bounded: a complexity comment that lies ("never quadratic" — `covers_staged`/`gather` restart the window scan per chained segment), a seek-refusal error message that claims a request "was minted" in the branch where none was, a vestigial dead variable, and a `read()` that allocates the full requested length (≤64 MiB) regardless of staged bytes.

## 2. What actually works (verified)
— Compilation: `audit/runs/baseline/build-release-workspace.log.txt:238` "Compiling lumen-web v0.1.0" (native); wasm32 artifact proven by `audit/runs/baseline/gate-web_budget___check.log.txt` (floor raw 1,302,846 B, gzip 301,101 B, PASS).
— Test execution: NONE in baseline. Neither workspace test log contains any `lumen-web` test run (`test-workspace.log.txt` aborts at `desktop`, `test-workspace-allfeatures.log.txt` at `first-vertical` — package ordering puts `lumen-web` after both). The in-crate tests have never been run in any citable baseline. All "works" statements below are code-trace only.
— Code-traced correct: eviction never strands the cursor (all-cover case removes index 0, remaining windows still cover); scan `consumed` frontier is monotonic (`(at + take).max(scan.consumed)`); `pos >= consumed` invariant holds in scan mode (seek only sets `pos` to targets `>= base + consumed` or the `== len` early return); `covers_staged` scan branch cannot underflow (`from >= base` guaranteed by callers).

## 3. Findings

### F-001 [S3]
Where: apps/web/src/io/staged_source.rs:302-304 (claim), 653-661 and 748-760 (code)
Code:
```rust
    /// The window cache, sorted by `start` (insertion keeps the
    /// order; scans are linear, never quadratic).
    windows: Vec<Window>,
```
and
```rust
                let mut cursor = from;
                'outer: while cursor < to {
                    for window in &self.windows {
                        if window.covers(cursor) {
                            cursor = window.start + window.data.len() as u64;
                            continue 'outer;
```
What is wrong: `covers_staged` (and `gather`, same `'outer`/`continue 'outer` shape at 748-760) restarts the linear window scan from index 0 for every chained window, making the scan O(windows²) in the worst case, not linear. A cursor-advancing index (windows are sorted and non-overlapping) would make it linear.
Why it matters: the comment asserts a complexity invariant the code does not have. With the default config (8 MiB budget / 256 KiB chunks ≈ 33 windows) the cost is trivial, but a legal config (`chunk_bytes` small, e.g. window 8 MiB / chunk 1 KiB → 8192 windows) makes every `ready()` poll — the pump gate, a hot path — do up to ~67M window checks. `ready()` gates every engine poll slice.
What correct looks like: advance a scan index across the sorted windows instead of `continue 'outer`, or fix the comment to state the quadratic-in-window-count bound.
Confidence: HIGH — Verification: read (both loops); config space validated by `StagedConfig::validate` (chunk ≤ window only, no floor).

### F-002 [S3]
Where: apps/web/src/io/staged_source.rs:593-600 and 617-621 (message text at 105-109, part1)
Code:
```rust
                if self.outstanding.is_none() {
                    let range = RangeRequest {
                        start: target,
                        end_exclusive: target.saturating_add(self.config.chunk_bytes as u64),
                    };
                    let _ = self.mint_request(Some(range));
                }
                Err(window_not_staged(target, false))
```
and
```rust
                self.stats.refetches = self.stats.refetches.saturating_add(1);
                if self.outstanding.is_none() {
                    let _ = self.mint_request(None);
                }
                Err(window_not_staged(target, true))
```
What is wrong: the `window_not_staged` error tells the caller "{mode} request was minted; retry the seek once the response lands" (lines 106-107) — but when a DIFFERENT request is already outstanding, the `if self.outstanding.is_none()` guard skips minting, so nothing was minted for the target and the message is false. In the scan branch the `refetches` counter is incremented unconditionally, so a refused seek during a busy transport inflates the "honest linear cost" statistic without any re-fetch having been requested.
Why it matters: the caller following the error's guidance (retry after the next response lands) hits a SECOND refusal before the target request is even minted — a two-step stall the message does not describe; and `StagedStats::refetches` — documented as "the honest linear cost, counted" — overcounts.
What correct looks like: mint unconditionally is impossible (one-at-a-time lane), so the message must be conditional ("a request is in flight; once it lands, retry to mint the target window") and the counter should move inside the mint branch (or a separate `deferred_seeks` counter added).
Confidence: HIGH — Verification: read of both branches + the message text; no test covers seek-during-outstanding.

### F-003 [S4]
Where: apps/web/src/io/staged_source.rs:677-688
Code:
```rust
    fn age_behind_cursor(&mut self) {
        let cursor = self.pos;
        let before = self.windows.len();
        self.windows.retain(|window| {
```
and
```rust
        });
        let _ = before;
    }
```
What is wrong: `before` is computed at entry and explicitly discarded at exit (`let _ = before;`) — a vestigial variable, almost certainly a removed assertion's leftover.
Why it matters: dead statement in the aging hot path; reads as an incomplete refactor and invites "why is this here" archaeology.
What correct looks like: delete both lines.
Confidence: HIGH — Verification: read; grep confirms no other use of `before` in the function.

### F-004 [S4]
Where: apps/web/src/io/staged_source.rs:800-806
Code:
```rust
    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        let len = len.min(MAX_READ_BYTES);
        let mut out = vec![0; len];
        let gathered = self.gather(&mut out);
        out.truncate(gathered);
        Ok(out)
    }
```
What is wrong: the result is allocated at the full requested length (zero-filled) regardless of how much is staged, and `truncate` keeps the capacity — a demuxer requesting a large read against a mostly-unstaged cache pays a full `len`-byte zeroing allocation and returns a `Vec` holding that capacity with a short `len`.
Why it matters: bounded (≤ `MAX_READ_BYTES` = 64 MiB, documented at 71-76 as the one allocation outside the window budget) but wasteful in exactly the slow case (cold cache); in a wasm32 Worker each such read is a 64 MiB zeroed heap spike the GC must then reclaim.
What correct looks like: `read_into`-style staging — gather into a scratch buffer bounded by the staged span, or `Vec::with_capacity` + `spare_capacity_mut` copy, and `shrink_to(gathered)` (or document the retained capacity).
Confidence: HIGH on the code shape; MEDIUM on real-world impact (the AVI lane's largest read is the 32 KiB probe window per the `READY_MARGIN` comment, so today's callers stay small).
Verification: read.

### F-005 [S3]
Where: apps/web/src/io/staged_source.rs:1-3 (module claim) vs apps/web/src/capabilities.rs:269-274 and ui/e2e/web/site/harness.js:504-516
Code:
```rust
//! The staged window source (card 7.H4; doc 09 §6.1) — the fetch
//! twin of `crates/streaming`'s `RangedResource` (card 6.A2a/6.A2b),
//! serving the same [`SourceStream`] contract demuxers already
```
vs harness.js:507-509:
```js
    const error = response.result.error;
    assert(error, "http open must refuse (the staged fetch lane is not wired to open)");
    assertEq(error.code, "unsupported-for-host", "http refusal code");
```
What is wrong: ~1,218 lines of implemented, tested fetch-lane engine code (this file + `fetch_port` + `seekability` + `mock_fetch`) are dormant: no production code path constructs `StagedSource`; `session.open` with a `url` source refuses with the typed `unsupported-for-host` citing `http-streaming`. The capability row (capabilities.rs:269-274) still reads `NotStarted` with reason "fetch + Range seekability detection is card 7.H4" — present-tense-future wording for work that is already committed in-tree.
Why it matters: dead-in-practice code compiled into a size-budgeted wasm module (linker may strip it, but the maintenance cost and the misleading row wording remain); the matrix's "fetch-range-supported-server PASS" rows describe the harness server, not the engine lane — a reader skimming the record could conclude the fetch lane works in-browser when the engine half has never executed outside native unit tests.
What correct looks like: either wire the lane (7.I1) or move the row's reason to past tense ("engine half landed card 7.H4; host wiring pending 7.I1") so the row's status and the tree agree.
Confidence: HIGH — Verification: grep caller map; browser-matrix record `docs/web/browser-matrix-20261110.json` (open-http-pending-lane → pass BY refusing; fetch-range-* → transport probes of the harness server); capabilities.rs blocked-rows read.
Note: this is HONESTLY labeled overall (lib.rs "honest posture", harness assertion, NotStarted row) — no fraud; it is dead code plus stale row wording.

## 4. Per-function table (part2 range; straddles marked)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `seek_to` (straddles boundary; Scan branch + ranged-unstaged path audited here) | 562-624 | REAL | S3 | Negative/over-len targets typed-refused; `== len` EOF fast path correct; ranged: staged serves, unstaged mints (F-002 message lie); scan: `serves()` gate correct, refetch counted even unminted (F-002). |
| `out_of_range` | 626-633 | REAL | — | Typed `input.seek.out-of-range`; trivial, correct. |
| `frontier` | 635-641 | REAL | — | `windows.last()` end; relies on sorted invariant (held by `insert_window`); empty→0. |
| `covers_staged` | 643-672 | REAL | S3 | Correct chaining; quadratic restart scan (F-001); scan branch underflow-safe. |
| `age_behind_cursor` | 674-689 | REAL | S4 | Correct retain + budget decrement + eviction count; dead `before` var (F-003); closure disjoint-field borrow of `self.cached` is legal (edition 2024). |
| `insert_window` | 691-717 | REAL | — | Sorted insert via `partition_point`; eviction loop sound — `unwrap_or(0)` fallback only fires when ALL windows cover the cursor, in which case removing the lowest-start still leaves coverage (traced); budget may be exceeded by at most the in-flight window (transport limit = window budget). |
| `mint_request` | 719-739 | REAL | — | One outstanding; `limit` = window budget on every shape (documented why chunk extent isn't the bound); ids saturate. |
| `gather` | 741-781 | REAL | S3 | Correct window chaining + scan drain; same quadratic restart as F-001; `cache_hits` counts per-call not per-window (cosmetic). |
| `SourceStream::uri` | 785-787 | REAL | — | Trivial. |
| `SourceStream::size` | 789-791 | REAL | — | Server-declared len; honest. |
| `SourceStream::supports_seek` | 793-798 | REAL | — | Server truth (`Seekability::is_seekable`); the staged-serving boundary documented at 47-58. |
| `SourceStream::read` | 800-806 | REAL | S4 | `MAX_READ_BYTES` clamp; full-length zeroed alloc + retained capacity (F-004). |
| `SourceStream::read_into` | 808-810 | REAL | — | Delegates to gather; no clamp possible (caller-sized). |
| `SourceStream::seek` | 812-814 | REAL | — | Delegates. |
| `SourceStream::position` | 816-818 | REAL | — | Trivial. |
| test helper `serve` | 831-837 | REAL | — | Drives outstanding→transport→absorb. |
| test helper `pipeline` | 840-847 | REAL | — | Bounded 64 iterations (≥ 8 MiB/256 KiB window count for any fixture that fits the budget). |
| test helper `tiny_ranged` | 850-871 | REAL | — | 12-byte ranged fixture; pins probe range `(0, 4)`. |
| test `ranged_server_reads_across_windows` | 875-895 | REAL | — | Crosses window boundary; short read only at EOS. |
| test `no_range_server_scans_forward_only` | 898-948 | REAL | — | Pins scan mode, refetch counting, skip accounting, `web.fetch.window-not-staged` code. |
| test `window_cache_ages_and_progress_never_deadlocks` | 953-975 | REAL | — | Budget invariant `cached <= 16 + 4` asserted; whole resource consumed. |
| test `ready_gate_tracks_the_margin_and_the_end` | 980-998 | REAL | — | Pins not-ready/ready semantics + EOS. |
| test `correlation_refuses_unsolicited_and_mismatched` | 1004-1038 | REAL | — | Pins `web.fetch.correlation`/`web.fetch.unsolicited` + outstanding survival. |
| test `mid_stream_200_resyncs_into_scan` | 1042-1061 | REAL | — | Pins resync count + post-flip reads. |
| test `truncated_windows_are_typed_or_wire_end` | 1065-1115 | REAL | — | Mid-resource typed truncation vs honest wire-end. |
| test `config_validation_refuses_typed` | 1119-1148 | REAL | — | Three typed codes incl. `ftp://` refusal. |
| test `fetch_lane_serves_the_real_fixture_to_the_demuxer` | 1159-1217 | REAL (scope-limited) | — | Real fixture + real `media_probe` + real `AviDemuxer`; would fail if gather/insert/ready broke. Does NOT exercise mid-stream mint/serve interleave during demuxing (whole resource staged first — comment admits "[PROPOSAL] with 7.I1"). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| ranged_server_reads_across_windows | YES | Byte-exact reads across a window boundary; mutation of `gather`'s chaining breaks `b"012345"`. |
| no_range_server_scans_forward_only | YES | Pins `refetches == 1`, `skipped_bytes == 1`, refusal code — kills counter and boundary mutations. |
| window_cache_ages_and_progress_never_deadlocks | YES | `consumed == 12` + `cached <= 20` + `cache_evictions >= 2` — kills aging/eviction mutations. |
| ready_gate_tracks_the_margin_and_the_end | YES | `!ready` with known-but-unstaged end is exactly the gate semantic. |
| correlation_refuses_unsolicited_and_mismatched | YES | Exact error codes + outstanding id survival. |
| mid_stream_200_resyncs_into_scan | YES | `resyncs == 1` + post-flip read bytes. |
| truncated_windows_are_typed_or_wire_end | YES | Distinguishes typed truncation from honest wire-end — kills the "silent EOF" mutation. |
| config_validation_refuses_typed | YES | Three exact codes. |
| fetch_lane_serves_the_real_fixture_to_the_demuxer | PARTIAL | Catches cache-serving bugs (probe→demux→packets>0 over the real AVI); does NOT catch mid-stream pump-interleave bugs (whole resource pre-staged), and transport is `MockFetch`, never a browser. |
| Baseline run evidence for ANY of the above | NO RUN | `lumen-web` tests absent from both baseline workspace test logs (both aborted at earlier packages). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "http-streaming … status NotStarted … fetch + Range seekability detection is card 7.H4" | docs/feature-matrix.toml web column (via capabilities.rs:269-274 same wording) | SHALLOW-ish/STALE | The 7.H4 engine half exists and is tested (this file); the LANE is indeed unusable (unwired). Row wording predates the code. |
| "the canvas/AudioWorklet/fetch lanes named as typed blocked rows" | apps/web/src/lib.rs:42-43 | TRUE | capabilities.rs blocked rows + harness refusal assertions. |
| "real fetch, real postMessage ordering … 10/17 scenarios passed" | docs/web/browser-matrix-20261110.md | TRUE (honest) | Record: `open-play-pause-stop`, `play-to-end`, `media-info-and-tracks`, `seek-not-served` etc. FAIL in Chromium; `fetch-range-*` scenarios are harness-server transport probes, not engine-lane proofs. |
| "The mid-stream pump-gate … is the browser message lane's wiring — [PROPOSAL] with 7.I1" | staged_source.rs:1155-1158 test comment | TRUE (honest admission) | The proof test pre-stages the whole fixture; mid-stream interleave untested. |
| "scans are linear, never quadratic" | staged_source.rs:303 | FALSE | F-001. |

## 7. Dead / duplicated / unreachable code
- The entire `StagedSource` lane is production-unreachable (F-005): exported by `io/mod.rs:76`, consumed by nobody.
- `before` variable in `age_behind_cursor` (F-003).
- No duplicated logic found vs the blob lane (different mechanism, shared `SourceStream` contract only).

## 8. Dependencies & flags
- Uses `plugin_api::{SeekOrigin, SourcePosition, SourceStream}` (the L1 trait seam), `media_types` errors, in-crate `fetch_port`/`seekability`. No feature gates on this file; `builtin-codecs*` features do not touch it. wasm32 target compiles it (web-budget floor build PASS) where it is dead weight for the linker to strip.

## 9. Open questions for the next agent
- A10 (part1): in `absorb`'s mid-stream-200 resync (506-518), a lying `Content-Length` larger than the delivered body sets `len` past the scan buffer's end — scan-mode `pending_request` always returns `None`, `ready()` stays false, no error is surfaced: a permanent silent stall (I flagged the shape; the code lives in your half — please confirm and rate).
- A10 (part1): `absorb`'s resync `windows.clear()` (509) drops windows without counting them in `stats.cache_evictions` — counter semantics drift.
- fetch_port/seekability auditors: does any check reject a 206 body LONGER than the requested range extent (staged as one oversized window), and does `detect_seekability` cross-check the response's `Content-Range` start against the request's start (this file trusts `outstanding.range.start` at 501-502)?
- Orchestrator: REQUEST-ORCH-RUN `cargo test -p lumen-web --locked` — the web crate's native tests (incl. this file's 9) have zero baseline run evidence.
