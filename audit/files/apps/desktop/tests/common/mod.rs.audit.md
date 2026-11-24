# AUDIT: apps/desktop/tests/common/mod.rs
| Field | Value |
|---|---|
| Auditor | W13-A04 |
| Date | 2026-11-24 |
| Lines / functions | 150 / 8 items (FakeMonotonic, Timeline×6, TimelineEmitClock, marked_bytes, marked_frame, marked_request, payload_marker) |
| File verdict | REAL (test-fixture module; both consuming suites executed green) |
| Reachable from shipped entry point? | N/A — integration-fixture module (tests/common), never compiled into any shipped binary. Consumed by apps/desktop/tests/{pipeline,protocol,zero_alloc_transport}.rs — verified by running the first two suites. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | ./target/debug/deps/pipeline-154e6f81feed74d6 → 4 passed (0.01s); ./target/debug/deps/protocol-dc6ee80a98a67ea6 → 4 passed (0.16s); rg usage map of every helper. |

## 1. One-paragraph verdict
A clean, genuinely-shared test fixture: one atomic virtual timeline serving both the scheduler clock and the bridge emit clock (the single-coordinate-system trick that makes `nominal_pipeline_delivers_every_frame_with_exact_latency` an exact-arithmetic test), plus content-addressed synthetic frames — pixel (5,5) carries the frame index, so the protocol suite's round-trip tests assert WHICH frame the bytes carry, not just how many. Both consuming suites pass (4+4, executed). The only debt is the blanket `#![allow(dead_code)]` (every helper is in fact used — the allow hides future rot) and the u8 marker index that silently wraps at 256 frames (harmless today: the largest suite drives 60).

## 2. What actually works (verified)
— `./target/debug/deps/pipeline-154e6f81feed74d6` → 4 passed: `nominal_pipeline_delivers_every_frame_with_exact_latency`, `generation_bump_flushes_in_flight_frames`, `slow_consumer_latest_wins_with_structural_bounds`, `stub_binary_reports_pending_state`.
— `./target/debug/deps/protocol-dc6ee80a98a67ea6` → 4 passed: `full_hd_frame_round_trips`, `bridge_stream_round_trips_sixty_frames_varied_sizes`, `renderer_composited_output_streams_byte_exact`, `bad_magic_reports_stream_offset_after_consumed_records`.
— Helper usage map (rg): Timeline{new,media_clock,emit_clock,set_ns,advance_ns,now_ns} + marked_request in pipeline.rs; marked_request + payload_marker in protocol.rs; marked_bytes + marked_request in zero_alloc_transport.rs. No orphan helpers.

## 3. Findings
### F-001 [S4] Blanket `#![allow(dead_code)]` on a module where nothing is dead
Where: apps/desktop/tests/common/mod.rs:16
Code (sed-verified):
```rust
#![allow(dead_code)]
```
What is wrong: every public helper is consumed by at least one of the three suites (verified by rg, §2) — the allow is unnecessary today and will mask a genuinely orphaned fixture tomorrow (the exact rot the workspace's lint discipline exists to catch). Confidence: HIGH — Verification: read + rg.

### F-002 [S4] The marker pixel index is u8 — silent wrap at 256 frames
Where: apps/desktop/tests/common/mod.rs:105-108
Code (sed-verified):
```rust
pub fn marked_bytes(extent: Extent2D, index: u8) -> Vec<u8> {
    let mut rgba = synthetic::synthetic_pattern(extent);
    let at = (5 * extent.width as usize + 5) * 4;
    rgba[at..at + 4].copy_from_slice(&[index, 0xEE, 0x77, 255]);
```
What is wrong: a hypothetical suite driving ≥256 frames would see frame 256's marker equal frame 0's — a content assertion silently weakened, not a wrong assertion. Current maximum is 60 (`for i in 0..60u64` in pipeline.rs:173/214 and protocol.rs:44). Latent only. Confidence: HIGH — Verification: read + rg of loop bounds.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FakeMonotonic::now_ns | 38-42 | REAL | — | Relaxed load of the shared counter; correct for a test clock |
| Timeline::new | 52-56 | REAL | — | zeroed AtomicU64 |
| Timeline::media_clock | 59-67 | REAL | — | MediaClock::system_monotonic over the fake source; used by pipeline suite (ran) |
| Timeline::emit_clock | 70-74 | REAL | — | same timeline as the emit stamps; the exact-latency seam (ran) |
| Timeline::set_ns | 77-79 | REAL | — | fetch_max keeps monotonicity |
| Timeline::advance_ns | 82-84 | REAL | — | fetch_add |
| Timeline::now_ns | 87-89 | REAL | — | read |
| TimelineEmitClock::emit_ns | 97-101 | REAL | — | EmitClock impl over the same atomics |
| marked_bytes | 105-110 | REAL | S4 | content marker; u8 wrap (F-002); (5,5) needs ≥6×6 extents — all suites use ≥32×24 (verified) |
| marked_frame | 114-129 | REAL | — | sRGB full-range color description; internal caller marked_request |
| marked_request | 132-138 | REAL | — | PTS-stamped PresentRequest; used by all three suites (two ran green) |
| payload_marker | 142-150 | REAL | — | symmetric read; protocol suite's content assertions (ran) |

## 5. Test honesty
This file IS the fixture library; the honesty question is whether its consumers assert content. Verified: protocol.rs's `full_hd_frame_round_trips` / `bridge_stream_round_trips_sixty_frames_varied_sizes` read the marker back through `payload_marker` (rg: 2 uses) — a broken wire encode/decode would fail on content, not count. Both suites executed green.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "one `AtomicU64` nanosecond source serving BOTH the media clock … and the bridge's emit clock" | :6-8 | REAL | same Arc handed to both (media_clock/emit_clock); exact-latency test ran |
| "pixel (5,5) carries the frame index, so every claim is checked against *content*, not counters" | :11-14 | REAL | payload_marker assertions in protocol suite (ran) |
| "the `unreachable!`s live behind the testkit `must` vocabulary" | :2-4 | REAL | module uses `.must()`, no bare unwrap/panic (read) |

## 7. Dead / duplicated / unreachable code
None dead despite the blanket allow (F-001). No duplication.

## 8. Dependencies & flags
Test-only imports: desktop (EmitClock), media_clock, media_types, renderer_wgpu::synthetic, testkit::Must, video_engine::PresentRequest. No feature gates. `#![allow(dead_code)]` is the only lint suppression.

## 9. Open questions for the next agent
- zero_alloc_transport.rs has no prebuilt binary in target/debug/deps (only .d files for protocol variants) — its suite was not executed here; UNVERIFIED (cargo-build-budget: would require compiling the desktop test target).
