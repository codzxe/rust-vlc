# AUDIT: apps/desktop/src/bridge.rs
| Field | Value |
|---|---|
| Auditor | W13-A01 |
| Date | 2026-11-23 |
| Lines / functions | 416 / 17 (incl. 5 tests) |
| File verdict | REAL (genuine serializer + transport, honest typed refusals, real round-trip tests) — but UNREACHABLE from any shipped binary |
| Reachable from shipped entry point? | NO. The shipped `desktop` bin (src/main.rs:23-87) prints a skeleton report and exits 86 — it never constructs a `CanvasBridge`. Chain that WOULD reach it: apps/desktop-tauri/src/app.rs:290 `ShellRuntime::new` -> shell.rs:180 `FrameTap::new` -> frame_tap.rs:59 `CanvasBridge::new(transport)` — but `apps/desktop-tauri` is excluded from the workspace and NOT in the release lane, and it cannot compile in this environment (baseline cargo-build-desktop-tauri.txt:310-345: `gdk-sys` build fails, `gdk-3.0` not found). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | rg (multiple, exit 0); sed -n quotes (exit 0); baseline citations: cargo-test-workspace.txt:2522-2526, cargo-build-desktop-tauri.txt:310-345, cargo-build-release-desktop.txt:4 |

## 1. One-paragraph verdict
The bridge is real code doing what its docs claim: `CanvasBridge::present`/`present_repeat` serialize a `PresentRequest` into a LUF1 record (`serialize` :195-237) and hand the bytes to a `FrameTransport`, never reordering or blocking, with an honest slot-echo `PresentInfo` and typed refusals for YUV/OpaqueHandle frames and oversize payloads. The Wave-13 question — "does bridge.rs actually connect the engine to the Tauri shell?" — answers NO three times over: (1) the only transport compiled in this crate is `MailboxTransport`, an IN-PROCESS hand-off (transport.rs:184-227); the actual IPC transport is a proposal string in a comment; (2) the Tauri shell does not exist — no tauri/wry dependency (Cargo.toml:11-16 "NO tauri/wry import"), main.rs exits 86 "no webview host compiled: ADR-004 pending"; (3) the one caller that composes the whole chain (`apps/desktop-tauri/src/app.rs`) is workspace-excluded and fails to build (gdk-sys). What exists is a well-tested protocol library whose consumer is a prototype (`apps/desktop/prototype/lumen-frames.mjs`) and whose shipped-binary reachability is zero.

## 2. What actually works (verified)
— verified by audit/runs/baseline/cargo-test-workspace.txt:2522-2526 -> all 5 `bridge::tests` pass (round-trip decode, repeat flag, YUV/opaque refusals, truncated refusal, monotonic clock).
— verified by read: serialize/present path is allocation-stable (`self.scratch.clear()` + `encode_into`, :233-234), frame_id wraps safely (:235), payload length checked against u32 (:219-222), `PresentInfo::presented_at` echoes the scheduled slot rather than inventing a draw instant (:189-191).

## 3. Findings
F-001 [S3]
Where: apps/desktop/src/bridge.rs:5-10
Code:
```
//! The scheduler still owns *which frame is on screen when* — the
//! bridge is a transport, not a policy: `present()` serializes the
//! request's frame and hands the record to the
//! [`FrameTransport`], never reordering, never blocking, never
//! deciding. The canvas side (`prototype/lumen-frames.mjs`) is the
//! other half of the same contract.
```
What is wrong: the module doc's framing implies a live engine->canvas pipeline; in the shipped tree the "other half" is a prototype script and the record never leaves the process (MailboxTransport) or never ships at all (bin exits 86). The doc never states that the bridge is dead code from every shipped entry point. Why it matters: a reader concludes the desktop streams frames today; it does not — the honest deferral lives in lib.rs/main.rs/Cargo.toml, not here. What correct looks like: one sentence naming the reachability state (ADR-004 pending, consumer = prototype). Confidence: HIGH — Verification: read + rg reachability chain + baseline build log.
F-002 [S3]
Where: apps/desktop/src/bridge.rs:201-217
Code:
```
        let (format, payload): (FormatCode, &[u8]) = match (&frame.data, frame.format) {
            (FrameStorage::Cpu(bytes), format) => {
                let code =
                    FormatCode::for_pixel_format(format).ok_or(WireError::UnknownFormatCode {
```
```
            (FrameStorage::OpaqueHandle(_), _) => {
                return Err(WireError::UnknownFormatCode { found: 0 });
            }
```
What is wrong: both refusal paths report `found: 0` regardless of the actual pixel format — an Nv12 refusal and an OpaqueHandle refusal produce byte-identical diagnostics; the real format is discarded at the error boundary. Why it matters: the typed-refusal discipline this crate advertises ("surface what was found") is undercut for exactly the cases the v1 bridge cannot carry; debugging a YUV frame reaching the bridge gives "found: 0" — false. What correct looks like: carry the `PixelFormat` (or its discriminant) into the error. Confidence: HIGH — Verification: read (WireError::UnknownFormatCode field is `found`; the test at :373-390 only asserts the code string, not the payload — the lie is untested). NOTE: severity is S3 not S2 because the stable code string (`render.stream.format-unsupported`) is correct and this is a diagnostics detail.
F-003 [S4]
Where: apps/desktop/src/bridge.rs:275-286
Code:
```
fn transport_to_engine(err: TransportError) -> EngineError {
    use media_types::error::{ErrorContext, ErrorKind, PipelineStage};
    EngineError::new(
        PipelineStage::Present,
        ErrorKind::Device {
            which: media_types::error::DeviceSlot::Display,
            cause: media_types::error::DeviceCause::Disconnected,
```
What is wrong: EVERY transport error maps to `DeviceCause::Disconnected` — a `MailboxMisconfigured` (transport.rs:195) or `Hosted` decode failure (:225-230) is reported as a display disconnect. Wrong cause taxonomy, honest stage/code. Confidence: HIGH — Verification: read + rg TransportError variants (transport.rs:195, :225-230, mailbox.rs MailboxError).
F-004 [S4]
Where: apps/desktop/src/bridge.rs:240-244
Code:
```
impl<T: FrameTransport> Presenter for CanvasBridge<T> {
    fn present(&mut self, request: &PresentRequest) -> Result<PresentInfo, EngineError> {
        self.ship(request, false)
    }
}
```
What is wrong: the `Presenter` impl exists to satisfy the video-engine seam, but the desktop wiring never installs the bridge AS a `Presenter` — pipeline_host's `VideoLimb` owns the renderer and the bridge rides the `PresentTap` seam (frame_tap.rs:34 `use lumen_cli::pipeline_host::PresentTap`). The impl is a shape-keeper; no non-test code path calls `CanvasBridge::present` outside the tap. Dead-ish surface, documented as the v1 contract. Confidence: HIGH — Verification: read + rg (only frame_tap.rs:113 calls `present_repeat`; no caller of the `Presenter::present` impl outside tests).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| EmitClock::emit_ns (trait) | 48-51 | REAL | — | clock seam |
| MonotonicEmitClock::default/new | 62-76 | REAL | — | Instant::now origin |
| MonotonicEmitClock::emit_ns | 78-85 | REAL | — | saturating u128->u64, documented clamp; test-pinned monotonic |
| BridgeStats | 88-96 | REAL | — | plain counters |
| CanvasBridge::Debug | 109-116 | REAL | — | non-exhaustive, transport named |
| CanvasBridge::new | 121-124 | REAL | — | production clock |
| CanvasBridge::with_clock | 128-137 | REAL | — | harness clock injection |
| CanvasBridge::transport/transport_mut | 139-151 | REAL | — | consumer-side access |
| CanvasBridge::stats/now_ns | 153-163 | REAL | — | observability |
| CanvasBridge::present_repeat | 172-174 | REAL | — | honest repeat entry (flag set) |
| CanvasBridge::ship | 177-192 | REAL | — | serialize->stats->send; bytes_shipped counts scratch len incl. header |
| CanvasBridge::serialize | 195-237 | REAL | S3 | F-002 found:0; u32 length gate; scratch reuse; wrapping frame_id |
| Presenter::present | 240-244 | REAL | S4 | F-004 shape-keeper impl |
| wire_to_engine | 249-271 | REAL | — | per-variant stable codes; Corrupt/InvalidStructure kind for all (geometry errors as Corrupt is arguably wrong kind — minor) |
| transport_to_engine | 275-286 | REAL | S4 | F-003 Disconnected for everything |
| tests (5) | 300-415 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| present_ships_a_decodable_record_and_echoes_the_slot | YES | full encode->WireDecoder->decode round-trip, payload byte-eq, stats asserted |
| repeat_entry_flags_the_record_and_reships_the_payload | YES | flag + frame_id + payload len asserted on record #2 |
| yuv_and_opaque_frames_are_typed_refusals | YES (partially) | asserts code + frames_shipped==0; does NOT assert the `found` payload — F-002's lie is untested |
| truncated_frames_are_refused_not_padded | YES | geometry code + Corrupt kind asserted |
| monotonic_emit_clock_advances | WEAK | b >= a on two immediate reads — near-tautology, but the contract is only non-decreasing; acceptable |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The canvas side (`prototype/lumen-frames.mjs`) is the other half of the same contract" | bridge.rs:9-10 | REAL (file exists) but misleading | file exists at apps/desktop/prototype/lumen-frames.mjs (Glob verified); it is a prototype, not shipped — reachability absent (F-001) |
| "never reordering, never blocking, never deciding" | bridge.rs:6-8 | REAL | ship() is serialize+send, no queueing; MailboxTransport::offer is latest-wins but that is the transport's own policy, the bridge itself holds nothing |
| "v1 re-ships the payload on repeats (correct everywhere, no differential records)" | bridge.rs:22-23 | REAL | present_repeat -> ship(repeat=true) reships full payload; test pins payload len 8 |
| "The record's `emitted_ns` comes from the injected EmitClock: virtual in the harness, monotonic wall in production" | bridge.rs:33-35 | REAL | with_clock injection + MonotonicEmitClock; FixedClock(1_234) pinned in test |
| "The same bytes flow over a Tauri IPC channel" (wire.rs:10, this module's consumer claim) | wire.rs:10-11 | PAPER-ONLY | `[PROPOSAL: TauriChannelTransport]` — no such transport exists; tauri not imported (Cargo.toml:11-16) |

## 7. Dead / duplicated / unreachable code
The whole module is unreachable from shipped binaries (see header). Within the crate: the `Presenter` impl (:240-244) has no non-test caller (F-004). No duplication found.

## 8. Dependencies & flags
media-types, video-engine, crate-local wire/transport/error. No feature gates. The crate is std-only (Cargo.toml:16 "Std-only by C5"); lumen-cli dep is used by shell.rs/dispatch.rs, not by bridge.rs itself.

## 9. Open questions for the next agent
1. frame_tap audit (mine, next): confirm the double-encode path the wire.rs audit flagged (mailbox decodes record->frame, tap re-encodes frame->record for drain_records) — per-frame cost + honesty of "zero-alloc" claim in that path. 2. Whoever owns apps/desktop-tauri: the wrapper is the ONLY production consumer; its build failure (gdk-sys) means the LUF1 lane has never run end-to-end in CI — every "works" claim for the desktop frame path is tests-only.
