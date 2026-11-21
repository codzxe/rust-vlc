# AUDIT: apps/desktop/src/wire.rs#part1 (lines 1–579 of 1159)
| Field | Value |
|---|---|
| Auditor | W13-A03 |
| Date | 2026-11-21 |
| Lines / functions | 579 (chunk of 1159; part2 lines 580–1159 → W13-A02) / 34 items |
| File verdict | REAL |
| Reachable from shipped entry point? | YES, via lumen-tauri (call chain: apps/desktop-tauri/src/app.rs ticker → `ShellRuntime::take_frame_records` (apps/desktop/src/shell.rs:219) → `FrameTap` = `CanvasBridge<MailboxTransport>` (apps/desktop/src/frame_tap.rs:46,59) → `MailboxTransport::send` → `WireDecoder::decode_record` (wire.rs:572) and → `frame_tap::encode_record` → `wire::encode_into` (wire.rs:400)); tauri build itself UNVERIFIED in sandbox (baseline #2, missing gtk — infra). NOT reachable from the `lumen-desktop` binary: apps/desktop/src/main.rs is a skeleton that prints a notice and exits 86 (WEBVIEW_PENDING) without constructing any bridge. Unit tests exercised the code directly (baseline: 14/14 `wire::tests` ok, test-workspace.log.txt:2347-2360). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n "fn parse_header|mod tests" wire.rs` → parse_header at 626 (A02 range) — header_len bounds validated there, so part1's `total = header_len + payload_len` is overflow-safe; `rg "wire::tests" audit/runs/baseline/test-workspace.log.txt` → 14 ok; `rg zero_alloc baseline/test-workspace.log.txt` → mailbox lane FAILED 608 B/1 event (lines 2394-2401), streaming lane ok; `rg --files -g 'harness*'` → no `apps/desktop/prototype/harness.mjs`; `rg -ln "MailboxTransport|CanvasBridge" apps/` → frame_tap.rs/bridge.rs/transport.rs + tests only; `rg "lumen-frames|LUF1" ui/src` → MediaSurface.tsx/frame-channel.ts (webview consumes the records); Read transport.rs:184-237 (MailboxTransport::send → decode_record), shell.rs frame_tap wiring, bridge.rs:100-219 (scratch reuse, YUV refusal) |

## 1. One-paragraph verdict
This is the tightest file audited so far: the LUF1 record codec is exactly what its spec table says, geometry validation is overflow-proof (u64 intermediates, 65,535-dimension and 1 GiB caps enforced on both encode and decode), the recycle pool is bounded and honest about its bounds, and the streaming decode path is proven zero-allocation by a recorded passing test. The defects are citation-honesty defects, not code defects: the module doc claims cross-language conformance via a harness file that does not exist and lists WebSocket/SharedArrayBuffer transports that exist nowhere in the tree; and the "steady-state path allocates nothing (proven by tests/zero_alloc_transport.rs)" claim is contradicted by the recorded baseline run in which that very test file FAILS on the mailbox lane (608 bytes, 1 allocation). The `lumen-desktop` binary itself never reaches this code — the wire only lives behind the tauri shell's frame tap.

## 2. What actually works (verified)
- All 14 `wire::tests` passed — verified by `rg "wire::tests" audit/runs/baseline/test-workspace.log.txt` → lines 2347-2360 (`round_trip_exact_fields`, `golden_bytes_match_the_spec_table`, `chunked_feed_at_every_split_point`, `malformed_inputs_are_typed_refusals`, `one_decoder_recycles_payload_buffers_across_records`, …).
- `streaming_decoder_path_is_zero_allocation_after_warm_up ... ok` (test-workspace.log.txt:2393) — the feed+next_frame steady state allocates nothing, under the counting allocator.
- Layout-table arithmetic re-derived by hand: offsets 0/4/6/8/9/10/12/16/20/24/32/40/48 match `write_to` (wire.rs:236-249) and `parse_header` (wire.rs:626-679, A02 range); 1080p overhead 48/8,294,400 = 0.00058% ✓; 4K 0.00014% ✓; 8K RGBA = 132 MB ✓.

## 3. Findings

### F-001 [S3] Zero-allocation claim cites `tests/zero_alloc_transport.rs` as proof — the recorded baseline run has that test file FAILING on the mailbox lane
Where: apps/desktop/src/wire.rs:434-438
Code:
```rust
/// bounded [`RecyclePool`] and cycle through the handed-out frames —
/// after one warm-up record, the steady-state path allocates nothing
/// (proven by `tests/zero_alloc_transport.rs` under the counting
/// allocator).
```
What is wrong: the baseline run records `mailbox_frame_path_is_zero_allocation_after_warm_up ... FAILED ... allocated 608 bytes across 1 allocation events (dealloc events: 0) — the path is NOT zero-allocation` (audit/runs/baseline/test-workspace.log.txt:2394-2401; INDEX.md row 3). That path includes this file's `decode_record` (via `MailboxTransport::send`, transport.rs:225-231). Only the streaming lane (`feed`+`next_frame`) passed. "Proven by tests/zero_alloc_transport.rs" is therefore half-true: the citation presents a failing test as proof.
Why it matters: the zero-alloc property is this module's headline performance claim (the C7 fix); a reader trusting the doc would believe the whole mailbox production path is proven allocation-free when the repo's own recorded evidence says otherwise. (The 608-byte single allocation is NOT in wire.rs code by trace — parse_header is alloc-free, acquire is pool-warm after the first cycle, `Arc::clone` does not allocate — the root cause sits in the bridge/mailbox half; see §9.)
What correct looks like: scope the claim to the lane that passes ("the streaming decode path is proven zero-allocation; the mailbox hand-off lane is tracked by the known failing test") or fix the 608-byte allocation and re-run.
Confidence: HIGH — Verification: read + baseline run cited (test-workspace.log.txt:2394-2401).

### F-002 [S3] Module doc cites a nonexistent conformance harness and names transports that do not exist in the tree
Where: apps/desktop/src/wire.rs:10-16
Code:
```rust
//! byte stream. The same bytes flow over a Tauri IPC channel
//! (`[PROPOSAL: TauriChannelTransport`, `transport.rs`]`), a
//! WebSocket, or a `SharedArrayBuffer` ring — the decoder never knows.
//! The reference canvas-side implementation is
//! `apps/desktop/prototype/lumen-frames.mjs` (byte-for-byte the same
//! state machine, cross-language conformance via
//! `apps/desktop/prototype/harness.mjs`).
```
What is wrong: (a) `apps/desktop/prototype/harness.mjs` does not exist — `rg --files -g 'harness*'` finds harness files only under tests/ and ui/, and `ls apps/desktop/prototype/` shows a single file, `lumen-frames.mjs` (451 lines). The "cross-language conformance" claim has no runnable artifact. (b) `TauriChannelTransport` exists nowhere in transport.rs — only `NullTransport`, `CaptureTransport`, `MailboxTransport`, and the `WebviewHostKind` enum whose `status()` strings admit "tauri/wry not imported", "pending ADR-004 + wire protocol v2", "pending wire protocol v2" (transport.rs:239-277). (c) No WebSocket transport and no SharedArrayBuffer transport exist anywhere in the workspace. Only the first item carries a `[PROPOSAL]` marker; the WebSocket/SAB clause reads as existing capability.
Why it matters: this is the same defect class the baseline's own xtask feature-audit flags elsewhere (row 23/163: evidence strings citing missing paths) — citations that name files/components that are not in the tree inflate the protocol's demonstrated surface. The actual shipped consumer path (tauri `frame_channel` → ui/src/client/frame-channel.ts) exists and is different from all three named transports.
What correct looks like: cite `lumen-frames.mjs` alone (it exists), mark all three transport futures as `[PROPOSAL]`, and reference the real consumer (`apps/desktop-tauri/src/frame_channel.rs` + `ui/src/client/frame-channel.ts`).
Confidence: HIGH — Verification: read + `rg`/`ls` (files enumerated above).

### F-003 [S4] `PayloadLengthMismatch::implied` means two different things in the two constructors
Where: apps/desktop/src/wire.rs:406-410 (vs 226-230)
Code:
```rust
    if payload.len() != header.payload_len as usize {
        return Err(WireError::PayloadLengthMismatch {
            declared: header.payload_len,
            implied: payload.len() as u64,
        });
    }
```
What is wrong: in `validate_geometry` (wire.rs:226-230) `implied` is the geometry-derived length `width·height·4`; in `encode_into` (wire.rs:406-410) `implied` is the actual payload slice length. Same variant, same field name, two semantics — an error log reading `PayloadLengthMismatch { declared: 12288, implied: 12000 }` is ambiguous about whether 12000 is the buffer the caller handed over or the geometry's demand.
Why it matters: misleading names in error payloads cost real debugging time on the exact (malformed-frame) path where they are printed; the instruction's own checklist names "naming that lies".
What correct looks like: rename the encode-site field usage (or add a third field `actual`) so the variant distinguishes geometry-implied vs provided.
Confidence: HIGH — Verification: read (both construction sites compared).

## 4. Per-function table (lines 1–579 only; parse_header/finish/tests are part2 → W13-A02)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs + record layout | 1-60 | REAL (2 claims FALSE) | S3 | Layout table verified byte-for-byte against write_to/parse_header; F-001/F-002 doc claims. |
| `MAGIC`/`VERSION`/`MIN_HEADER_LEN`/`MAX_HEADER_LEN`/`MAX_PAYLOAD_LEN`/`MAX_DIMENSION` | 70-89 | REAL | — | Caps consistent: 65,535² ·4 ≈ 16 GiB > 1 GiB cap, so u32 payload arithmetic cannot wrap past the cap check. |
| `FrameFlags` | 92-98 | REAL | — | One bit, doc'd. |
| `FrameFlags::to_bits` | 103-105 | REAL | — | Bit 0 only. |
| `FrameFlags::from_bits` | 112-119 | REAL | — | `bits & 0xFE != 0` → typed ReservedNotZero; strict ✓. |
| `FormatCode` | 126-131 | REAL | — | Closed two-value enum — YUV unrepresentable by type, matching "encoder refuses what it cannot express". |
| `FormatCode::code` | 136-141 | REAL | — | 1/2 per spec table. |
| `FormatCode::from_code` | 148-154 | REAL | — | Anything else typed UnknownFormatCode. |
| `FormatCode::pixel_format` | 159-164 | REAL | — | Bijection to media-types vocabulary. |
| `FormatCode::for_pixel_format` | 170-176 | REAL | — | Nv12/I420/Yuyv422 → None (the typed YUV refusal seam; bridge.rs:203-212 consumes it). |
| `FrameHeader` | 182-204 | REAL | — | Copy type; field docs match wire offsets. |
| `FrameHeader::validate_geometry` | 209-233 | REAL | — | u64 intermediates; zero-dim, 65,535 cap, 1 GiB cap, exact payload match. No overflow path (hand-checked extremes 1×1×4 and 65,535×65,535×4). Shared by encode+decode — one rule set ✓. |
| `FrameHeader::write_to` | 236-249 | REAL | — | Field order == layout table; header_len always 48; reserved hard-zero ✓. |
| `RECYCLE_SLOTS` | 258 | REAL | — | 3 = mailbox default capacity 2 + 1 in hand (mailbox.rs:60 `TWO`); wider mailboxes documented as bounded by queue not pool. |
| `RecyclePool` | 270-272 | REAL | — | Mutex<Vec<Vec<u8>>>. |
| `RecyclePool::new` | 275-279 | REAL | — | — |
| `RecyclePool::acquire` | 285-303 | REAL | — | Best-fit (smallest capacity ≥ needed); falls back to pop-last + grow-once on geometry change; fresh alloc only on empty pool (warm-up). Poison recovery via `into_inner` ✓. O(3) scan — no perf issue. |
| `RecyclePool::release` | 306-317 | REAL | — | Hard-bounded at RECYCLE_SLOTS; full pool drops returnee (documented). |
| `DecodedFrame` | 328-335 | REAL | — | Clone shares the pool Arc (documented); clone's drop returns its own buffer — bounded. |
| `Drop for DecodedFrame` | 337-345 | REAL | — | `mem::take` avoids double-free of pooled buffer ✓ correct. |
| `DecodedFrame::detached` | 352-358 | REAL | — | `#[cfg(test)]` ✓ test-only builder. |
| `Debug for DecodedFrame` | 361-370 | REAL | — | Same surface as old derive. |
| `PartialEq`/`Eq` | 372-380 | REAL | — | Pool handle excluded ✓ documented. |
| `encode` | 388-392 | REAL | — | Allocating convenience wrapper; exact `with_capacity(48+len)`. |
| `encode_into` | 400-415 | REAL | S4 | F-003 naming. Validates geometry AND payload_len==payload.len() before writing — malformed record never shipped ✓. |
| `WireDecoder` | 440-454 | REAL | — | Cursor ring; counters u64. |
| `Default for WireDecoder` | 456-460 | REAL | — | → new(). |
| `WireDecoder::new` | 465-473 | REAL | — | One pool per decoder; clones share (documented). |
| `WireDecoder::feed` | 476-490 | REAL | — | Compacts consumed prefix only when pos>0 (O(live tail)); fully-drained path already reset by next_frame ✓. |
| `WireDecoder::bytes_fed` | 494-496 | REAL | — | Diagnostics. |
| `WireDecoder::bytes_consumed` | 501-503 | REAL | — | Diagnostics; decode_record maintains both counters consistently. |
| `WireDecoder::next_frame` | 517-553 | REAL | — | Sticky-by-position on error (pos never advances past a bad record); BadMagic offset rewritten to stream coordinates (523-530); waits on partial header/record (519-534); one-copy pooled payload; in-place reset on full drain. Slice bounds: parse reads only 48 guaranteed bytes; `total = header_len(≤1024, validated in part2 parse_header) + payload_len(≤1 GiB)` cannot overflow usize. |
| `WireDecoder::decode_record` (head; body 580-600 → A02) | 572-579 | REAL | — | Boundary note: function spans the part1/part2 cut at 579/580. In-range: Truncated with exact missing count (no underflow — len<48 checked first). Steady-state alloc-free by trace (pool warm, Arc::clone refcount-only). |

## 5. Test honesty
Tests for this file live at wire.rs:683-1160 (part2, W13-A02) plus apps/desktop/tests/{pipeline,protocol,zero_alloc_transport}.rs. For the part1 surface: the 14 baseline-passing wire tests cover encode/decode round-trips, chunked feed at every split point, recycle across records, and typed refusals — a mutation of `validate_geometry` (drop the cap check) fails `encoder_refuses_unexpressible_records`; a mutation of the pool (never release) fails `one_decoder_recycles_payload_buffers_across_records`. `zero_alloc_transport.rs` would (and did, for the mailbox lane) catch a steady-state allocation — an honest, failing test, but see F-001 for the doc's misuse of it as proof.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| Record layout table (offsets/lengths/fields) | wire.rs:18-41 | REAL | Byte-compared against write_to/parse_header; `golden_bytes_match_the_spec_table` ok (baseline:2354). |
| "same bytes flow over a Tauri IPC channel, a WebSocket, or a SharedArrayBuffer ring" | wire.rs:10-12 | FALSE (WebSocket/SAB), PAPER-ONLY (Tauri, marked PROPOSAL) | transport.rs has only Null/Capture/Mailbox; WebviewHostKind statuses all "pending" (transport.rs:252-266). |
| "cross-language conformance via apps/desktop/prototype/harness.mjs" | wire.rs:14-16 | FALSE (file absent) | `rg --files -g 'harness*'`; prototype/ contains only lumen-frames.mjs. |
| "The encoder refuses what it cannot express (zero dims, YUV formats, payload/dimension disagreement)" | wire.rs:43-47 | REAL | validate_geometry + for_pixel_format→None + encode_into len check; `encoder_refuses_unexpressible_records` ok (baseline:2351). |
| "The decoder is strict … never guesses. There is no resync" | wire.rs:48-52 | REAL | parse_header typed refusals (part2) + next_frame pos-holding; `malformed_inputs_are_typed_refusals` ok (baseline:2356). |
| "after one warm-up record, the steady-state path allocates nothing (proven by tests/zero_alloc_transport.rs)" | wire.rs:436-438 | FALSE (as cited) | Baseline: mailbox lane FAILED 608 B (test-workspace.log.txt:2394-2401); streaming lane ok. F-001. |
| "pts_ns vs emitted_ns live on different clocks by design" | wire.rs:53-60 | REAL | i64 media slot vs u64 monotonic emit stamp; bridge stamps via EmitClock (bridge.rs:101,123). |

## 7. Dead / duplicated / unreachable code
- None dead in range. `DecodedFrame::detached` is cfg(test) (intentional).
- Unreachability note (architecture, not dead code): the `lumen-desktop` binary (apps/desktop/src/main.rs) exits 86 without touching wire.rs — the ONLY shipped consumer is the tauri shell's FrameTap chain (see reachability row). The decode half (WireDecoder) is reachable in production solely because `MailboxTransport::send` decodes what `CanvasBridge` just encoded in-process (transport.rs:172-183 admits the "double decode cost"), then frame_tap re-encodes for the webview.

## 8. Dependencies & flags
- `media_types::PixelFormat` and `crate::error::WireError` only — no feature gates on this path; part of the `desktop` crate (workspace member, built in baseline #1).
- No unsafe blocks in range (whole-file unsafe count: none — consistent with baseline hygiene concentration in platform-FFI crates).

## 9. Open questions for the next agent
- W13-A02 (wire.rs part2): confirm `finish()`'s missing-byte arithmetic and the 14 tests' honesty; `parse_header` bounds were read by me for the overflow-safety argument only.
- W13-A04/A05/A06: root-cause the single 608-byte allocation in the mailbox lane (one alloc, zero deallocs — a retained buffer; candidates: bridge present-path lazy init or mailbox queue growth; NOT wire.rs by trace).
- W13-A05 (transport.rs): the `[PROPOSAL]` TauriChannelTransport vs the real `frame_channel.rs` in apps/desktop-tauri — the doc's transport story needs one owner.
- Orchestrator: `lumen-desktop` exiting 86 (WEBVIEW_PENDING) means the ONLY way this whole crate's production path runs is the tauri shell, whose build is UNVERIFIED in this sandbox (baseline #2). Any "desktop works" claim downstream rests on the tauri-check CI lane, which is a doc claim I could not verify.
