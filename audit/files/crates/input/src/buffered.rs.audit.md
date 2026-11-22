# AUDIT: crates/input/src/buffered.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 827 / 16 production fns + 14 tests |
| File verdict | REAL — fixed-capacity buffered reader with a correct position invariant and honest typed errors; reached from the shipped CLI file-open path |
| Reachable from shipped entry point? | YES: `lumen open` → `demux_host::open_source` (apps/cli/src/demux_host.rs:394/:422 `input::FileAccess::new().open(&uri)`) → `FileAccess::open` → `BufferedReader::new` (crates/input/src/file.rs:192) → `FileStream` (plugin-api SourceStream) → every demuxer read |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | Read/Grep/sed only (no cargo); quotes verified with Read/Grep (unfiltered tools) |

## 1. One-paragraph verdict

The best file audited so far in this wave: a fixed-capacity window reader
whose position invariant (`source.position() == buf_pos +
buffered_bytes()`, :27-33) I re-derived on every path (small read,
bypass read, borrow twins, seek, refill) and found upheld; bounds are
typed errors (`capacity-zero`, `capacity-out-of-range`,
`request-over-limit`, `input.file.truncated`); short reads are data,
`Ok(0)` only at EOF; the shrink-detection contract is implemented at
:409-416. The two-regime design (window locality for small asks, direct
source transfer for payload-class asks) is proven by call-counting tests
(`reads_served`, `max_read_seen`). Two hygiene-level notes only.

## 2. What actually works (verified)

— Reachability chain (static, Grep): apps/cli/src/demux_host.rs:394 and
:422 (`input::FileAccess::new().open(&uri)`) →
crates/input/src/file.rs:192 (`let reader = BufferedReader::new(source,
self.capacity)?;`) — the reader is under every local-file open of the
shipped CLI; W00's mjpeg+pcm AVI decode exercised it.
— Position invariant re-derived by hand for: `read` small path
(:208-225, refill only when `start == end`), `read_bypassing_window`
(:237-260, `buf_pos += got` per direct read), `read_into` both arms
(:310-354), `refill_read_ahead` (:266-272, called only with an empty
window), `seek` (:153-173, in-window index math guarded by
`to < window_end`). No path breaks it.
— Bounds: `want > MAX_READ_BYTES` refused typed (:202-203) before any
allocation; capacity validated at construction (:85-90).

## 3. Findings

### F-001 [S4] Truncation promise is conditional on `length()` observability, and the module doc hides the condition
Where: buffered.rs:13-14 (module claim) vs :409-416 (implementation)

Code (:13-14):
```
//! A source that shrinks below its open-time length surfaces
//! `Corrupt::Truncated`, never a silent EOF.
```
Code (:409-415):
```
    fn confirm_eof(&mut self) -> Result<(), EngineError> {
        if let (Some(declared), Some(live)) = (self.declared_len, self.source.length())
            && live < declared
        {
            return Err(error::truncated(declared, live));
        }
        Ok(())
    }
```
What is wrong: for a source whose `length()` answers `None` (a pipe, a
growing file, a network stream), `confirm_eof` can never fire — the
shrink surfaces as a silent clean EOF, exactly what the module doc
promises never happens. The condition is documented only on
`source_length` (:128-129 "`None` when unobservable"), not where the
promise is made.
Why it matters: a doc-reader wiring a `None`-length source believes
truncation is detected. It is not.
What correct looks like: state the condition on the module-doc promise
(or refuse `None`-length sources for the truncation contract).
Confidence: HIGH — Verification: read.

### F-002 [S4] Bypass `Vec` twin pays a `want`-sized memset the borrow twin avoids
Where: buffered.rs:234-238

Code:
```
    /// `RawRead` needs initialized memory, and one `want`-sized memset
    /// replaces `want / capacity` syscall-sized copies.
    fn read_bypassing_window(&mut self, want: usize) -> Result<Vec<u8>, EngineError> {
        let mut out = vec![0u8; want];
```
What is wrong: every payload-class `read(want)` (up to
`MAX_READ_BYTES` = 64 MiB) zero-fills and then overwrites the whole
buffer. The comment is honest about the cost, and `read_into`
(:310-334) exists precisely to avoid it — but any demuxer calling the
`Vec` form for bulk payloads pays a dead memset per call.
Why it matters: 64 MiB of dead memset per payload read is a real cost
if any hot caller uses the wrong twin; it is invisible at the API
boundary.
What correct looks like: nothing in this file (safe-Rust constraint);
ensure hot callers use `read_into` (open question below).
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `BufferedReader::new` | 84-101 | REAL | — | capacity bounds typed; snapshot `declared_len`/`buf_pos` |
| `with_default_capacity` | 108-110 | REAL | — | 64 KiB |
| `position` | 114-116 | REAL | — | `&self`, no I/O |
| `buffer_capacity` / `buffered_bytes` / `source_length` | 119-132 | REAL | — | window math, `end - start` |
| `source` / `source_mut` | 135-142 | REAL | — | fault-scripting seams |
| `seek` | 153-173 | REAL | — | in-window/exact-end/source tiers; backward in-window not cached (source I/O) — documented design |
| `read` | 198-226 | REAL | — | 0 → empty; over-limit typed; small path refills whole capacity on miss (read-ahead) |
| `read_bypassing_window` | 237-260 | REAL | S4 | direct transfer, `buf_pos` accounting verified; memset (F-002); short source reads re-asked |
| `refill_read_ahead` | 266-272 | REAL | — | only with empty window — invariant safe |
| `drain_into` | 276-284 | REAL | — | capacity-bounded take, no realloc |
| `read_into` | 310-354 | REAL | — | borrow twin; same two regimes; no request ceiling (caller owns memory — documented) |
| `drain_leftover_into` | 360-368 | REAL | — | shared first step |
| `read_exact_into` | 380-391 | REAL | — | typed truncation with (expected, found) arithmetic |
| `peek_into` | 399-403 | REAL | — | window-bound, no advance, no I/O; `&mut self` unneeded (trivia) |
| `confirm_eof` | 409-416 | REAL | S4 | `None`-length sources never truncate (F-001) |

## 5. Test honesty (:419-827)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `capacity_bounds_are_typed_errors` | YES | pins kinds, codes, budget figures |
| `read_requests_over_the_ceiling_are_typed_limit_errors` | YES | over-limit refused; at-limit served (allocates 64 MiB in-test — heavy but bounded) |
| `positions_and_readahead_are_exact` | YES | pins I/O counts (1 refill, 64-byte chunks, 54 leftover, buffer-hit seek 0 I/O) |
| `large_reads_bypass_the_window_in_one_source_call` | YES | 2 source calls not 32; max ask = request; buffer never grows |
| `bypass_reads_serve_leftover_read_ahead_first` | YES | remainder ask is exactly 136 |
| `bypass_reads_survive_short_source_reads` | YES | dribble source (≤40 B/call), byte-exact, asks stay caller-sized |
| `bypass_read_past_end_short_reads_cleanly` | YES | whole file short of ask; sticky EOF |
| `clean_eof_at_declared_length` | YES | repeatable empty reads; seek-to-length clean; one-past typed; failed seek never moves |
| `shrink_mid_read_surfaces_typed_truncation` | YES | (expected, found) = (600, 300) |
| `non_zero_start_positions_are_respected` | YES | opens at 40 |
| `read_into_reconstructs_and_bypasses_for_payload_asks` | YES | 1000 bytes, 2 source calls, no window growth; Ok(0) only at EOS |
| `read_into_serves_small_asks_from_read_ahead` | YES | second ask 0 I/O |
| `read_exact_into_fills_or_fails_typed` | YES | expected=250/found=200 arithmetic pinned |
| `peek_into_never_advances_and_never_fabricates` | YES | 0 on empty; never more than buffered; no I/O |

The suite proves the behavioral claims (I/O counts, buffer sizes) via
`ScriptSource` instrumentation — not mocks of the subject; the subject
(`BufferedReader`) runs for real. Not executed by me (no cargo);
UNVERIFIED per-test, but W00's workspace run was green.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| fixed buffer "allocated once … never grown" | :4-5, :59-61 | REAL | no realloc path; `buffer_capacity` pinned by tests |
| requests ≥ capacity bypass the window | :6-10 | REAL | :205-206, :315-318 + call-count tests |
| single request above MAX_READ_BYTES refused typed | :11-12 | REAL | :202-203 + test |
| shrink → `Corrupt::Truncated`, "never a silent EOF" | :13-14 | SHALLOW | true only for `Some`-length sources (F-001) |
| position invariant always holds | :29-33 | REAL | hand-verified on all paths (§2) |

## 7. Dead / duplicated / unreachable code

None. `source`/`source_mut` (:135-142) are test/instrumentation seams,
documented as such.

## 8. Dependencies & flags

`media_types` (EngineError), `crate::{error, seekable}`. No feature
gates. The `input` crate is a workspace member; `demux` depends on it
(crates/demux/Cargo.toml:39) — though `crates/demux/src` itself never
names `input::` (only file.rs inside the input crate does the wrapping;
demuxers consume the `FileStream` through plugin-api). Shipped in the
`lumen` CLI (W00 release build PASS).

## 9. Open questions for the next agent

- Wave 4/5 (demuxers): do the demux arenas use `read_into` (borrow
  twin) for payload reads, or `read` (Vec twin, paying the F-002
  memset)? Grep `\.read\(` in crates/demux/src/avi + mp4 for
  big-`want` call sites.
- Wave 10 (streaming): the http lane (`open_http_source`,
  demux_host.rs:427) uses `RangedResource` — does it wrap this reader
  or its own buffering? (Not in my file set.)
