# AUDIT: apps/desktop/src/bridge.rs
| Field | Value |
|---|---|
| Auditor | W13-A06 |
| Date | 2026-11-21 |
| Lines / functions | 417 / 20 (12 production items + 5 tests + 2 test helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES via `lumen-desktop-tauri` (ADR-004): desktop-tauri/Cargo.toml:53 `desktop = { path = "../desktop" }` → `desktop::shell::ShellRuntime` (app.rs:93) → `frame_tap::FrameTap` (frame_tap.rs:46,59 constructs `CanvasBridge` over `MailboxTransport`). CAVEAT: the tauri shell itself is UNVERIFIED-BUILD in the baseline (#2, missing GUI dev libs) — the desktop crate compiles and its tests run, but the end-to-end shipped desktop lane has never been built in evidence. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -n "CanvasBridge\|present_repeat\|use desktop" apps/` (constructor sites: frame_tap.rs:59, tests, tauri app.rs:92-94); `rg -n "desktop" apps/desktop-tauri/Cargo.toml` (path dep :53); Read apps/desktop/tests/zero_alloc_transport.rs (the failing baseline test's measured path includes this bridge); `rg -n "fn send\|decode_record" apps/desktop/src/transport.rs` + wire.rs encode_into/validate_geometry/for_pixel_format (error attribution + geometry validation lives in wire.rs:205-233, 400-415); `rg -n "bridge::tests\|zero_alloc" orchrun-test-workspace-nofailfast.log.txt` (all 5 bridge tests passed :2256-2260; mailbox zero-alloc FAILED); `sed -n` quote verification |

## 1. One-paragraph verdict
The bridge is a real, small, correctly-shaped transport presenter: serialize a `FrameHeader` + payload into a reused scratch buffer, hand the bytes to the `FrameTransport`, echo the scheduled slot, never invent a draw instant. Its five unit tests are honest (round-trip through the real `WireDecoder`, repeat-flag semantics, typed refusals for YUV/hardware-handle/truncated frames) and all passed in the baseline rerun. The one substantive problem is inherited: this file sits on the desktop frame lane whose zero-allocation proof FAILED in the baseline (`apps/desktop/tests/zero_alloc_transport.rs:98` — 608 bytes / 1 allocation event over 256 cycles, baseline run #3's single failure), while this file's own doc claims "the hot path allocates once" — size arithmetic exonerates the bridge's scratch (608 ≠ a 12 288-byte payload buffer growth; same-size records cannot grow the warmed scratch), but the lane ships with a falsified proof and the allocation is attributed to files owned by other agents. Three minor debts follow (stats counted before a failed send, a dead error field, a mis-typed error kind).

## 2. What actually works (verified)
- All 5 bridge unit tests passed in the orchestrator rerun (`bridge::tests::{monotonic_emit_clock_advances, present_ships_a_decodable_record_and_echoes_the_slot, repeat_entry_flags_the_record_and_reships_the_payload, truncated_frames_are_refused_not_padded, yuv_and_opaque_frames_are_typed_refusals}`, orchrun-test-workspace-nofailfast.log.txt:2256-2260), plus the integration lanes `bridge_stream_round_trips_sixty_frames_varied_sizes ... ok` (protocol.rs) and pipeline.rs 4/4. — verified by reading the cited log.
- Geometry safety net verified downstream: `encode_into` → `validate_geometry` (wire.rs:205-233) enforces `payload_len == width×height×4`, dims 1..=65535, 1 GiB cap — the bridge's truncated-frame refusal (bridge.rs:393-405) is this check firing, proven by the passing test. — verified by reading both sites.
- Format table honesty: `FormatCode::for_pixel_format` maps only Rgba8/Bgra8; I420/Nv12/Yuyv422 → `None` → typed `render.stream.format-unsupported` (wire.rs:170-176 + passing test). — verified by reading.

## 3. Findings

### F-001 [S3]
Where: apps/desktop/src/bridge.rs:104-106
Code:
```rust
    /// Reused encode buffer (one record's worth of scratch — the
    /// transport copies what it keeps; the hot path allocates once).
    scratch: Vec<u8>,
```
What is wrong: The claim sits on a lane whose zero-allocation proof fails in the repo's own baseline: `cargo test --workspace` run #3's single failure is `apps/desktop/tests/zero_alloc_transport.rs:98` — "desktop mailbox frame path: present -> serialize -> send -> decode_record -> offer -> draw over 256 cycles: allocated 608 bytes across 1 allocation events … the path is NOT zero-allocation" (test-workspace.log.txt:2395-2405; identical in the no-fail-fast rerun). The bridge itself is exonerated by arithmetic — the measured window ships same-extent records (64×48×4 payload), so the warmed `scratch` cannot grow, and 608 bytes is not a 12 288-byte payload buffer — the allocation is in the mailbox/transport/decoder half (other agents' files). But this file's documentation is the lane's performance contract, and the lane's proof is red.
Why it matters: A shipped claim ("allocates once") with a failing proof is exactly the evidence-vs-docs drift this audit exists to catch; whoever reads bridge.rs believes the lane is allocation-free.
What correct looks like: Attribute the 608 bytes (mailbox queue growth and/or decoder pool second slot — candidates in mailbox.rs/transport.rs/wire.rs), fix it or adjust the claim, and get run #3 back to green.
Confidence: HIGH — Verification: baseline citation + read of the measured path; exact allocation site UNVERIFIED (needs the counting allocator under a debugger or per-crate isolation — REQUEST-ORCH-RUN candidate for the mailbox/transport owners).

### F-002 [S4]
Where: apps/desktop/src/bridge.rs:177-186
Code:
```rust
    fn ship(&mut self, request: &PresentRequest, repeat: bool) -> Result<PresentInfo, EngineError> {
        self.serialize(request, repeat).map_err(wire_to_engine)?;
        self.stats.frames_shipped += 1;
        if repeat {
            self.stats.repeats += 1;
        }
        self.stats.bytes_shipped += self.scratch.len() as u64;
        self.transport
            .send(&self.scratch)
            .map_err(transport_to_engine)?;
```
What is wrong: `frames_shipped`/`bytes_shipped` are incremented BEFORE `transport.send`. A send failure (disconnected webview) still counts the record as shipped — the accounting lies exactly on the error path it exists to observe.
Why it matters: `BridgeStats` is the "observable evidence" (line 154) for the lane's throughput; on a flapping transport it over-reports.
What correct looks like: Move the counter updates after a successful `send`.
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: apps/desktop/src/bridge.rs:201-217
Code:
```rust
                let code =
                    FormatCode::for_pixel_format(format).ok_or(WireError::UnknownFormatCode {
                        // The YUV family: the code table mirrors the
                        // renderer's upload path; converting here would
                        // duplicate the GPU shader card's job. Hardware
                        // handles need the renderer's import path
                        // (variant A territory); the bridge is the
                        // CPU-frame transport of v1.
                        found: 0,
                    })?;
```
What is wrong: `WireError::UnknownFormatCode { found: u8 }` is defined as "The offending code" (error.rs:37-40) and its Display prints it ("unknown pixel-format code {found}", error.rs:93-95) — the bridge always discards the real value and passes `found: 0`. Every refusal prints "unknown pixel-format code 0". The field is also semantically wrong for this site: the failure is "pixel format has no wire code" (an encode-side gap), not an unknown code read off the wire — the decode-side variant is being reused.
Why it matters: Diagnostics lose the one fact (which format arrived) that would explain the refusal; the misused variant conflates encode and decode failures.
What correct looks like: A dedicated `WireError::FormatUnshippable { format: PixelFormat }` (or at minimum propagate the format's discriminant into `found`).
Confidence: HIGH — Verification: read (error.rs:37-40, 93-95).

### F-004 [S4]
Where: apps/desktop/src/bridge.rs:249-271
Code:
```rust
    EngineError::new(
        PipelineStage::Present,
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        ErrorContext::new(code),
    )
```
What is wrong: `wire_to_engine` maps EVERY wire error — including the format-availability refusal (`render.stream.format-unsupported`) — to `ErrorKind::Corrupt`. An unsupported format is not corruption; the code string says "unsupported" while the kind says "corrupt" (the decode-side arms BadMagic/Truncated are genuinely Corrupt, but UnknownFormatCode-on-encode and BadGeometry are not).
Why it matters: Error-kind consumers (telemetry, the desktop shell's degrade policy) misclassify an availability gap as data corruption. Same defect family as pipeline_host.rs's everything-is-`Unsupported` (W13-A06 pipeline_host#part1 F-004).
What correct looks like: Match per-variant: format gap → `ErrorKind::Unsupported`, geometry → Corrupt is defensible, malformed → Corrupt.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-35 | REAL | — | Repeat policy + timestamp honesty accurate; perf claim → F-001 |
| `EmitClock` (trait) | 47-51 | REAL | — | injection seam, used by harness + production |
| `MonotonicEmitClock` + Default/new | 53-85 | REAL | — | saturating u64 cast documented; monotonicity test passes |
| `BridgeStats` | 87-96 | REAL | S4 | F-002 counting site |
| `CanvasBridge` (struct) | 98-107 | REAL | S3 | F-001 doc claim on `scratch` |
| `Debug` impl | 109-116 | REAL | — | no payload dump |
| `new` | 118-124 | REAL | — | production clock |
| `with_clock` | 129-137 | REAL | — | harness injection |
| `transport` / `transport_mut` | 139-151 | REAL | — | used by frame_tap + tests |
| `stats` / `now_ns` | 153-163 | REAL | — | observability |
| `present_repeat` | 165-174 | REAL | — | honest repeat entry (wiring matches on action — frame_tap.rs:113 calls it) |
| `ship` | 176-192 | REAL | S4 | F-002 stats-before-send; slot echo documented |
| `serialize` | 194-237 | REAL | S4 | F-003 `found: 0`; payload/geometry safety lives in wire.rs (verified); `frame_id` wrapping ok; OpaqueHandle refused typed |
| `Presenter::present` | 240-244 | REAL | — | fresh-slot entry |
| `wire_to_engine` | 246-271 | REAL | S4 | F-004 kind mismatch |
| `transport_to_engine` | 273-286 | REAL | — | Device/Disconnected — semantically right for the webview channel |
| tests: `FixedClock` | 300-306 | REAL | — | — |
| tests: `request` | 308-323 | REAL | — | builds real VideoFrame |
| `present_ships_a_decodable_record_and_echoes_the_slot` | 325-351 | REAL | — | round-trips through real WireDecoder; byte-exact payload; stats asserted |
| `repeat_entry_flags_the_record_and_reships_the_payload` | 353-370 | REAL | — | flag + frame_id + payload length asserted |
| `yuv_and_opaque_frames_are_typed_refusals` | 372-390 | REAL | — | both refusal arms + "nothing shipped" |
| `truncated_frames_are_refused_not_padded` | 392-405 | REAL | — | geometry refusal + kind asserted |
| `monotonic_emit_clock_advances` | 407-415 | REAL | — | weak (non-decreasing over two reads) but contract-documented |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| present_ships_a_decodable_record_and_echoes_the_slot | YES | full encode→decode round-trip, byte-exact payload, stats |
| repeat_entry_flags_the_record_and_reships_the_payload | YES | repeat flag + id sequencing |
| yuv_and_opaque_frames_are_typed_refusals | YES | code asserted + zero-shipped invariant |
| truncated_frames_are_refused_not_padded | YES | geometry code + Corrupt kind asserted |
| monotonic_emit_clock_advances | WEAK | two consecutive reads — passes for any non-regressing clock; nearly tautological |
| (integration) zero_alloc_transport mailbox lane | IT FAILS TODAY | the repo's own red proof — F-001 |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "never reordering, never blocking, never deciding" | module docs 5-8 | REAL-UNPROVEN | no reordering in code (single queue); non-blocking is the transport's contract (mailbox.rs — other agent); no run proves the composed lane |
| "the hot path allocates once" | bridge.rs:104-106 | SHALLOW for the lane | baseline run #3 failure: 608 bytes / 1 event on present→draw (F-001); bridge-side exonerated by size arithmetic only |
| "v1 re-ships the payload on repeats (correct everywhere)" | module docs 22-24 | REAL | repeat test re-ships full payload |
| "the actual draw instant is … observed downstream" | module docs 29-32 | REAL | `presented_at` echoes `request.when` (189-191); mailbox accounting owns drawn_ns (mailbox.rs) |

## 7. Dead / duplicated / unreachable code
- `WireError::UnknownFormatCode.found` is dead at this call site (always 0) — F-003.
- No dead functions; `now_ns` is diagnostics-only (used by shell/harness — verify there).

## 8. Dependencies & flags
Depends on media-types, video-engine (Presenter), crate::{error,transport,wire}. No feature gates. `prototype/lumen-frames.mjs` is the JS mirror of wire.rs (not this file's dependency).

## 9. Open questions for the next agent
1. (mailbox.rs / transport.rs / wire.rs owners) Attribute the 608-byte allocation in `mailbox_frame_path_is_zero_allocation_after_warm_up` — my candidates: mailbox queue Vec growth (608 = 38 × 16 is suspiciously factorable) or the decoder `RecyclePool`'s second slot. This file's F-001 resolves when that lands.
2. (shell.rs / desktop host.rs owners) The bridge refuses I420 (wire table). Confirm the desktop pipeline converts before present — the CLI pipeline_host's hold defect (W1-06, my pipeline_host#part1 F-001) would surface here as `render.stream.format-unsupported` and freeze the webview.
3. (desktop host.rs / shell.rs owners) Where is the settle loop that HANGS on a decode error (the wave's 4:4:4 H.264 context)? Not in this file — the bridge has no loops beyond the per-request path.
