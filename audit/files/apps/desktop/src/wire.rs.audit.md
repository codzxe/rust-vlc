# AUDIT: apps/desktop/src/wire.rs
| Field | Value |
|---|---|
| Auditor | W13-A05 |
| Date | 2025-06-27 |
| Lines / functions | 1159 / 27 prod fns + 14 tests + 2 test helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | YES via lumen-tauri (see chain); NO via lumen-desktop skeleton bin; NO via lumen CLI |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 2 |
| Commands run | `cargo test -p desktop --lib wire::` → 14 passed/0 failed; `find . -name harness.mjs` → 0 hits; `rg` reachability chains; sed quote verification (all below quotes sed-verified) |

## 1. One-paragraph verdict
This is a real, strict, well-tested little-endian record codec for LUF1 (RGBA8/Bgra8 only, 48-byte header, payload_len = width·height·4, YUV refused, unknown magic/version/format/reserved-bits = typed errors, no resync). The encoder refuses unexpressible geometry; the decoder is incremental with cursor-based compaction and a bounded (3-slot) payload recycle pool. I executed all 14 unit tests — pass. Hostile-focus answers: (1) frame format is RGBA8 (code 1) / Bgra8 (code 2), row-major, w·h·4 bytes — verified by a hand-written golden-bytes test; (2) flow control is NOT in this file — the bounded latest-wins `FrameMailbox` (capacity 2) upstream is the flow control, while `feed()` itself has no bound; (3) it IS constructed in production — the shipped lumen-tauri GUI drives it via ShellRuntime→FrameTap→CanvasBridge/MailboxTransport on every presented frame. Defects: `finish()` underflows usize when called with complete records buffered; two phantom-evidence doc claims (missing harness.mjs, nonexistent "§4" measurement); and the shipped GUI frame path pays three full-payload copies per frame while the file's "steady-state path allocates nothing" claim covers only the transport window.

## 2. What actually works (verified)
— `cargo test -p desktop --lib wire::` → `test result: ok. 14 passed; 0 failed` (includes `golden_bytes_match_the_spec_table`, `chunked_feed_at_every_split_point` (every split point 0..=len), `malformed_inputs_are_typed_refusals` (8 mutations), `format_code_table_is_exactly_the_upload_path` (cross-checks `renderer_wgpu::uploads_format`)).
— Baseline (audit/runs/baseline/cargo-test-workspace.txt:2572-2585, 2588, 2596-2616): all 14 wire tests + pipeline 4/4 + protocol 4/4 + zero_alloc 2/2 pass.
— Production reachability chain (all read, sed-verified): apps/desktop-tauri/src/app.rs:288-292 `tauri::Builder...manage(ShellState(Mutex::new(ShellRuntime::new(` → shell.rs:160-162 `if let Command::SessionSetFrameChannel { channel } = &request.command { return self.bind_frame_channel(...)` → frame_tap.rs:57-59 `MailboxTransport::new(...)`/`CanvasBridge::new(transport)` → bridge.rs:234 `encode_into(&header, payload, &mut self.scratch)?` (wire.rs:400) and transport.rs:225-227 `.decode_record(record)` (wire.rs:572) → shell.rs:219 `take_frame_records` → frame_tap.rs:127 `encode_into` re-encode → app.rs:211 `runtime.take_frame_records(&mut records)` → app.rs:223 `records.last()` shipped over the tauri channel → ui/src/components/player/MediaSurface.tsx:199 `presentLuf1Frame` decodes and blits via putImageData. The pure-CLI `lumen` binary does not depend on desktop (apps/cli/Cargo.toml has no desktop dep); the sandbox-built `lumen-desktop` skeleton (main.rs) never touches wire and exits 86.

## 3. Findings
F-001 [S3]
Where: apps/desktop/src/wire.rs:617
Code: (verbatim)
```
            let (header, header_len) = Self::parse_header(unconsumed)?;
            (header_len + header.payload_len as usize - unconsumed.len()) as u64
```
What is wrong: `finish()` computes `total − unconsumed.len()` with no check that `total > unconsumed.len()`. If called while ≥1 complete record is still buffered (e.g. `feed(two_records); finish()` without draining), `unconsumed.len() > total` and the usize subtraction underflows: debug builds panic (`attempt to subtract with overflow`), release builds report `Truncated { missing: ~1.8e19 }`. The drained-path callers are safe only because `next_frame` returning `None` guarantees `remaining < total`; `finish()` is a public API with no such precondition enforced. Why it matters: a public-API panic/garbage on a plausible misuse (finish-at-teardown with a full buffer). What correct looks like: `unconsumed.len().saturating_sub(total)` inverted branch, or reject with `TrailingBytes` when `total <= unconsumed.len()`. Confidence: HIGH — Verification: read (arithmetic; not executed — no test exercises it, which is exactly why it survived). Blast radius: no shipped caller (rg: only wire.rs tests, tests/protocol.rs, examples/emit_stream.rs call `.finish()`, all after full drain).

F-002 [S3]
Where: apps/desktop/src/wire.rs:15-16 and 39-41
Code: (verbatim)
```
//! state machine, cross-language conformance via
//! `apps/desktop/prototype/harness.mjs`).
```
and
```
//! a latency factor (the measured serialization cost is recorded in
//! `docs/prototypes/video-in-webview.md` §4, never asserted here).
```
What is wrong: Both evidence pointers are phantoms. `find . -name harness.mjs` (whole tree, excluding audit/) returns ZERO files — `apps/desktop/prototype/` contains only `lumen-frames.mjs`. The "cross-language conformance" harness does not exist; apps/desktop/README.md:36 instructs `node prototype/harness.mjs target/proto` (a command that cannot run) and docs/prototypes/video-in-webview.md:52 claims it "also runs headless in CI with Node" (no .github/workflows file references it). Likewise video-in-webview.md (77 lines) has no "§4" and contains no serialization-cost measurement at all (rg for serial|copy|cost|µs: zero hits). Why it matters: the file's honesty section leans on measurements and conformance runs that are not in the tree — exactly the fabricated-evidence pattern this audit hunts. What correct looks like: commit the harness or strike the claims. Confidence: HIGH — Verification: ran (find, rg).

F-003 [S2]
Where: apps/desktop/src/wire.rs:436-438 (claim) — locus spans transport.rs:218-231 + frame_tap.rs:83-88,126-133 (not my assigned files)
Code: (verbatim)
```
/// after one warm-up record, the steady-state path allocates nothing
/// (proven by `tests/zero_alloc_transport.rs` under the counting
/// allocator).
```
What is wrong: True only for the measured transport window (tests/zero_alloc_transport.rs measures present→send→decode_record→offer→draw). The SHIPPED GUI path adds frame_tap.rs:126 `let mut record = Vec::with_capacity(frame.header.payload_len as usize + MIN_HEADER_LEN);` — a fresh full-frame allocation per drained record — plus a second full-payload encode copy, on top of bridge encode (copy #1) and transport decode (copy #2). Net: three full-frame memcpys + ≥1 full-frame alloc per presented frame; app.rs:223 then ships `records.last()` and discards the rest (each discarded record was also allocated and copied). At 1080p60 that is ~1.5 GB/s of avoidable memcpy on the GUI hot path; the mailbox's "records are *decoded* into the mailbox ... the real Tauri IPC host will hand the raw record bytes to the webview instead" (transport.rs:172-182) acknowledges the double work but the raw-bytes shape was never built. Why it matters: major perf defect in the one path the desktop product exists for. What correct looks like: mailbox stores the encoded record (or the bridge hands the scratch buffer), drain ships it verbatim. Confidence: HIGH — Verification: read (composition traced end-to-end; copies counted by code reading, not profiled — UNVERIFIED as a wall-clock number).

F-004 [S3]
Where: apps/desktop/src/wire.rs:488
Code: (verbatim)
```
        self.pending.extend_from_slice(chunk);
        self.fed += chunk.len() as u64;
```
What is wrong: `feed()` has no cap on `pending`; a producer outpacing the consumer grows memory without bound (up to whatever the feeder pushes). No shipped code calls `feed` (transport uses `decode_record`; frame_tap/shell never feed) — all `feed` callers are tests and the prototype claims. Why it matters: the file's own pitch is WebSocket/ring-segment consumption (wire.rs:418-419), where an unbounded buffer is a remote-memory-pressure hole the moment that pitch becomes real. What correct looks like: a documented byte cap or `try_feed` returning backpressure. Confidence: HIGH — Verification: read + rg (zero production `feed` callers).

F-005 [S4]
Where: apps/desktop/src/wire.rs:299-302
Code: (verbatim)
```
        match best {
            Some(i) => slots.swap_remove(i),
            None => slots.pop().unwrap_or_default(),
        }
```
What is wrong: when no pooled buffer covers `needed`, the fallback pops the LAST slot arbitrarily — may grow a small buffer while retaining a larger one for a future small frame. Harmless in practice (bounded at 3 slots, one grow). Confidence: HIGH — Verification: read.

F-006 [S4]
Where: apps/desktop/src/wire.rs:84
Code: (verbatim)
```
pub const MAX_PAYLOAD_LEN: u64 = 1 << 30;
```
What is wrong: the decode path will allocate up to 1 GiB per record from wire-declared geometry (`pool.acquire(payload_len)`). Documented as a corruption heuristic, and in the shipped build the only producer is the in-process bridge, so not attacker-controlled — but any future real byte transport (the file's own pitch) makes this a header-driven 1 GiB alloc. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FrameFlags::to_bits | 103-105 | REAL | — | bit 0 = repeat |
| FrameFlags::from_bits | 112-119 | REAL | — | bits 1..=7 → typed refusal |
| FormatCode::code / from_code | 136-154 | REAL | — | 1=Rgba8, 2=Bgra8, else UnknownFormatCode |
| FormatCode::pixel_format / for_pixel_format | 159-176 | REAL | — | YUV family → None (refusal) |
| FrameHeader::validate_geometry | 209-233 | REAL | — | zero-dim/cap/mismatch checks; u64 math, no overflow |
| FrameHeader::write_to | 236-249 | REAL | — | exactly 48 LE bytes, matches golden test |
| RecyclePool::new | 275-279 | REAL | — | |
| RecyclePool::acquire | 285-303 | REAL | S4 | F-005 arbitrary fallback pop |
| RecyclePool::release | 306-317 | REAL | — | bounded at 3 slots |
| DecodedFrame::drop | 338-344 | REAL | — | mem::take returns buffer to pool |
| DecodedFrame::detached | 352-358 | REAL | — | cfg(test) builder |
| DecodedFrame::Debug/PartialEq | 362-378 | REAL | — | pool handle excluded, correct |
| encode | 388-392 | REAL | — | allocating wrapper |
| encode_into | 400-415 | REAL | — | validates geometry + payload len before writing |
| WireDecoder::default/new | 456-473 | REAL | — | |
| WireDecoder::feed | 476-490 | REAL | S3 | F-004 unbounded pending |
| WireDecoder::bytes_fed/bytes_consumed | 493-503 | REAL | — | diagnostics |
| WireDecoder::next_frame | 517-553 | REAL | — | cursor advance, no byte shift; sticky error (pos held) verified; offset patch only for BadMagic |
| WireDecoder::decode_record | 572-600 | REAL | — | the production entry (transport.rs:227); refusal leaves no state (test-pinned) |
| WireDecoder::finish | 608-620 | REAL | S3 | F-001 underflow |
| WireDecoder::parse_header | 626-679 | REAL | — | strict; reads only buf[0..48]; reserved/magic/version/geometry all enforced |
| tests ×14 + helpers | 682-1159 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| round_trip_exact_fields | YES | full field echo incl. i64 pts, u64::MAX frame_id |
| golden_bytes_match_the_spec_table | YES | expected bytes hand-written from layout table, field by field — independent of encoder |
| chunked_feed_at_every_split_point | YES | every split 0..=len + two-record mid-payload split; also asserts no early complete record |
| malformed_inputs_are_typed_refusals | YES | 8 byte-mutations each pinned to the exact typed error |
| truncated_streams_report_exact_missing_bytes | YES | exact missing counts 28/14 |
| extension_headers_are_skipped_not_guessed | YES | header_len 56 + 0xAB ext bytes round-trip |
| encoder_refuses_unexpressible_records | YES | 4 refusal cases incl. 1 GiB cap |
| format_code_table_is_exactly_the_upload_path | YES | cross-crate pin against renderer_wgpu::uploads_format |
| header_overhead_is_recorded_math_not_assertions | WEAK | arithmetic tautology (48/(payload+48) < 1e-5) — proves nothing about code, honest about it |
| decode_record_is_the_streaming_result_direct_from_the_slice | YES | direct == streaming equality |
| decode_record_refuses_truncated_and_trailing_input | YES | exact Truncated/TrailingBytes counts + no-state-left-behind |
| one_decoder_recycles_payload_buffers_across_records | YES | 8 ids + geometry change through one pool |
| partial_records_compact_without_losing_bytes | YES | 3 records split across feeds |
| clones_and_hand_built_frames_behave_like_plain_owners | YES | clone carries pool handle (returns buffer too — bounded) |
Gap: no test calls `finish()` with undrained complete records — the F-001 underflow window.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "cross-language conformance via apps/desktop/prototype/harness.mjs" | wire.rs:16 | FALSE (file missing) | `find . -name harness.mjs` → 0 hits; prototype/ holds only lumen-frames.mjs |
| "the measured serialization cost is recorded in docs/prototypes/video-in-webview.md §4" | wire.rs:40-41 | FALSE (no §4, no such measurement) | rg serial|copy|cost|µs in that doc → 0 hits; doc has unnumbered sections only |
| "steady-state path allocates nothing (proven by tests/zero_alloc_transport.rs)" | wire.rs:437-438 | SHALLOW (true only inside the measured transport window) | zero_alloc test passes (baseline:2616-2618) but excludes frame_tap drain's per-record Vec (F-003) |
| "The same bytes flow over a Tauri IPC channel, a WebSocket, or a SharedArrayBuffer ring" | wire.rs:10-12 | WebSocket/SAB: PAPER-ONLY ([PROPOSAL] in transport.rs:18-22); Tauri: REAL | app.rs:223 ships records over tauri channel; no WS/SAB transport exists |
| "There is no resync: a corrupted stream is dropped" | wire.rs:50-52 | REAL | error paths never advance `pos`; sticky (L421-424) |
| Record layout table (offsets/LE/48B/payload=w·h·4) | wire.rs:18-36 | REAL | golden_bytes test, executed |

## 7. Dead / duplicated / unreachable code
Nothing dead. `feed`/`next_frame`/`finish` are UNREACHABLE from shipped binaries today (decode_record is the only production entry; verified by rg) — they are the forward-compat surface for real byte transports. `emit_stream.rs` example uses feed/finish but examples are not shipped.

## 8. Dependencies & flags
Deps: media-types (PixelFormat), crate::error::WireError; test-only here: testkit. renderer-wgpu is declared a PRODUCTION dependency of the desktop crate (apps/desktop/Cargo.toml) but every use inside the crate is `#[cfg(test)]` (wire.rs:999, tests/, examples/emit_stream.rs) — rg shows zero non-test uses; a dev-dependency masquerading as a real one (S4 hygiene, belongs to Cargo.toml's owner). No feature gates; no unsafe; std-only. RECYCLE_SLOTS=3 (L258).

## 9. Open questions for the next agent
- F-003's locus (transport.rs double decode + frame_tap.rs per-record Vec + app.rs:223 `records.last()` discard) lives in files owned by other agents — the path-level S2 should land on them.
- shell.rs:211 `frame_channel_bound()` returns true if `frame_tap.is_some()` OR channel bound — a tap installed but channel never taken still drains/encodes/discards every tick? (app.rs only takes records when `frame_channel.is_some()`; the shell still encodes on drain — check for wasted encode when bind failed.)
- lumen-frames.mjs (the JS twin) is unaudited here; its "byte-for-byte the same state machine" claim (wire.rs:14-15) is UNVERIFIED by me beyond its existence.
