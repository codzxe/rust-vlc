# AUDIT: apps/desktop/tests/protocol.rs
| Field | Value |
|---|---|
| Auditor | W13-A02 |
| Date | 2026-11-24 |
| Lines / functions | 223 / 4 tests (+ shared `common/mod.rs` fixtures) |
| File verdict | REAL — a genuine wire-protocol conformance suite over production encode/decode/compositing |
| Reachable from shipped entry point? | Test file: runs under `cargo test -p desktop` (baseline cargo-test-workspace.txt:2609-2612 — all 4 `ok`). The code under test ships: `desktop::wire` (encode/WireDecoder, apps/desktop/src/wire.rs), `desktop::CanvasBridge` (bridge.rs), and the real wgpu renderer (`renderer_wgpu::Renderer::new_headless`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | sed quote verification; rg coverage searches (wire.rs unit-test inventory, harness.mjs CI search); baseline citation for the 4 test results. |

## 1. One-paragraph verdict
This is what a protocol conformance suite should look like: it drives the PRODUCTION
encoder (`wire::encode`), the PRODUCTION bridge (`CanvasBridge::present`/`present_repeat`),
and — in the composited-output test — the REAL wgpu renderer headless, then decodes with
the production `WireDecoder` and asserts byte-exact payloads, monotonic ids, PTS
round-trip, repeat-flag placement, header size (48 B), and full byte accounting at 1080p.
The corrupt-record test proves the diagnostic offset is in stream coordinates, not
per-chunk. The `CaptureTransport` sink is a self-declared fixture type (transport.rs:134-136),
which is the correct shape — the thing under test (the format) is production code. Only
two hygiene-level gaps: the JS reference mirror's conformance harness is manual-only
(no CI gate), and the suite's "stream diagnostics" bullet exercises one error kind,
deferring the rest to wire.rs's unit tests without saying so.

## 2. What actually works (verified)
— baseline cargo-test-workspace.txt:2609-2612: `bad_magic_reports_stream_offset_after_consumed_records ... ok`,
`bridge_stream_round_trips_sixty_frames_varied_sizes ... ok`,
`renderer_composited_output_streams_byte_exact ... ok`, `full_hd_frame_round_trips ... ok`
(orchestrator run; I did not re-run — cargo-build-budget, and the suite's wgpu headless
render is not a ~2s compile).
— Read-verified arithmetic: 60 frames; repeat indices i%7==0 (i=7..56) = 8 repeats
(:90 `assert_eq!(stats.repeats, 8)` correct); 1920×1080×4 = 8 294 400 + 48 = 8 294 448
(:171 correct); `MIN_HEADER_LEN = 48` (wire.rs:76) consistent with both size asserts.

## 3. Findings

F-001 [S4]
Where: apps/desktop/tests/protocol.rs:3-6 (module doc) + the absence of any automated
tie to `apps/desktop/prototype/lumen-frames.mjs`
Code:
```
//! streamable record, the bridge's stream over varied sizes, stream
//! diagnostics — everything the canvas-side reference implementation
//! (`prototype/lumen-frames.mjs`) mirrors, proven here on the Rust
//! side first.
```
What is wrong: the Rust side is proven here, but the MIRROR's conformance is a manual
Node harness (`cargo run -p desktop --example emit_stream` + `node
apps/desktop/prototype/harness.mjs`, examples/emit_stream.rs:13-17) and NO workflow
runs it (rg over .github/workflows → only ui.yml matches "prototype", as a path
filter). A JS-side format drift (magic, header layout, the 48..=1024 window) never
fails CI; the "mirrors" relationship is maintained by hand.
What correct looks like: one CI row running the emit_stream→harness.mjs pair, or a
shared golden bytes fixture both sides must match (wire.rs's
`golden_bytes_match_the_spec_table` :720 is the Rust half of exactly that).
Confidence: HIGH — Verification: read (workflow grep + emit_stream doc).

F-002 [S4]
Where: apps/desktop/tests/protocol.rs:179-180 (the "stream diagnostics" test)
Code:
```
/// Stream diagnostics: a corrupt record's offset is in fed-stream
/// coordinates (after N consumed records), not per-chunk.
```
What is wrong: the module doc's "stream diagnostics" bullet (:4) promises a category;
this suite exercises exactly one error kind (BadMagic). The rest (malformed inputs,
truncated streams, extension headers, encoder refusals) IS covered — but in wire.rs's
own unit tests (`malformed_inputs_are_typed_refusals` :800, `truncated_streams_report_exact_missing_bytes`
:904, `extension_headers_are_skipped_not_guessed` :920, `encoder_refuses_unexpressible_records`
:940), which the doc does not point to. A reader of this suite alone would believe
diagnostics coverage is one test. Doc nit; coverage itself is adequate across the crate.
Confidence: HIGH — Verification: read (wire.rs test inventory).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| bridge_stream_round_trips_sixty_frames_varied_sizes | 25-95 | REAL | — | 3 extents, 60 frames, repeat flags at i%7, byte-exact payload + marker-pixel content identity, full accounting (frames/repeats/bytes) |
| renderer_composited_output_streams_byte_exact | 102-149 | REAL | — | REAL wgpu headless renderer; 32×24 letterboxed into 64×48; readback → encode → decode byte-exact |
| full_hd_frame_round_trips | 155-177 | REAL | — | 1080p size class; header 48 of 8 294 448; bytes_shipped accounting |
| bad_magic_reports_stream_offset_after_consumed_records | 182-223 | REAL | — | two good records then garbage; asserts stream-coordinate offset (sum of consumed record lens) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| bridge_stream_round_trips… | YES | byte-eq per frame + marker pixel + repeat-flag placement + stats; any encode/decode drift fails it |
| renderer_composited_output_streams_byte_exact | YES | real compositing (letterbox placement) byte-compared; a placement regression shifts pixels |
| full_hd_frame_round_trips | YES | exact record size + payload + accounting |
| bad_magic_reports… | YES | pins the offset VALUE (sum of two record lengths); a per-chunk offset regression fails it |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the renderer's composited output as a streamable record" | protocol.rs:2-3 | REAL | test 2, baseline :2611 ok |
| "everything the canvas-side reference implementation mirrors, proven here on the Rust side first" | protocol.rs:4-6 | REAL for the Rust half / UNGATED for the mirror | F-001 |
| "the header is 48 of 8 294 448 bytes" | protocol.rs:152-153 | REAL | wire.rs:76 MIN_HEADER_LEN=48; 1920*1080*4+48 |
| "a 2-second 30 fps burst" | protocol.rs:23 | REAL | 60 × 33_333_334 ns ≈ 2 s |

## 7. Dead / duplicated / unreachable code
None. `CaptureTransport` is a deliberate fixture sink (transport.rs:134-136
"never wired into a playback session") — the in-session transport (MailboxTransport)
has its own suite (tests/zero_alloc_transport.rs, separate file, not mine).
`FrameStorage::OpaqueHandle` arm at :50-51 is a defensive match (synthetic fixtures
are Cpu) — dead in practice, harmless.

## 8. Dependencies & flags
Test-only deps on desktop, renderer_wgpu, video_engine, media_types, testkit. wgpu
headless runs in CI (baseline ok). No feature gates.

## 9. Open questions for the next agent
- If the wire format ever bumps to v2 (emitted_ns / `performance.now()` per wire.rs:58's
  PROPOSAL), this suite plus harness.mjs and wire.rs's golden test must move together —
  worth a single golden-bytes fixture shared by all three (F-001's fix).
