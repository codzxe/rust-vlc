# AUDIT: crates/audio-engine/src/buffer.rs
| Field | Value |
|---|---|
| Auditor | W07-A10 |
| Date | 2026-11-23 |
| Lines / functions | 94 / 2 prod items (`AudioBuffer` + `frames()`, `AudioSink` trait with `write`/`latency`); 1 test |
| File verdict | REAL (clean carrier type + sink trait; test executed, passes) — but the trait's central method `write` is bypassed by every shipped entry point |
| Reachable from shipped entry point? | PARTIAL: `AudioBuffer` YES (pipeline `take_output` → `AudioLimb::stage` → `sink.write_bytes`, apps/cli/src/pipeline_host.rs:663); `AudioSink::latency` YES (pipeline_host.rs:619); `AudioSink::write` NO — both shipped hosts call the `NullSink`-inherent `write_bytes` directly (pipeline_host.rs:663, apps/web/src/host.rs:233); the only caller of the trait's `write` is `PcmPump` (crates/audio-output/src/pump.rs), whose callers are tests (audio-output/tests/pcm_pipeline.rs) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p audio-engine --lib buffer --offline` -> 1 passed / 0 failed (`buffer_frame_math`); sed verifications; cross-reference reads of null.rs/ring.rs (quoted below) |

## 1. One-paragraph verdict
First, the hostile-focus premise is wrong: **buffer.rs contains no ring buffer.** It is a 94-line dumb carrier (`AudioBuffer`: format + `SharedBytes` + PTS) plus the two-method `AudioSink` trait. The ring feeding the null device lives in `crates/audio-output/src/ring.rs` (`SpscRing`) behind `NullSink`/`Shared` in `crates/audio-output/src/null.rs` — I verified that chain's bounds and capacity math by read (section 2) and it is sound: power-of-two capacity with mask wrap, all-or-nothing writes with a pre-check, frame-aligned chunking, zero-guarded divisions. What buffer.rs itself gets wrong is architectural: its doc declares `AudioSink` "what the mixer feeds," yet both shipped hosts (CLI and web) bypass the trait's `write` entirely and call `NullSink::write_bytes` directly — the trait method is reachable only through `PcmPump`, which no shipped code constructs. The type-level code that is here is correct: `frames()` delegates to `frames_in`, which guards `frame == 0` (no division by zero) and refuses partial frames as `None`.

## 2. What actually works (verified)
— `cargo test -p audio-engine --lib buffer --offline` -> `buffer_frame_math ... ok` (whole frames → Some(2); 7-byte partial → None).
— `AudioFormat::frames_in` (media-types/src/frame.rs:77-83): `if frame == 0 || !(bytes as u64).is_multiple_of(frame) { return None; }` — zero-channel/zero-byte formats cannot divide by zero.
— Hostile-focus cross-reference (the actual ring, read-verified): `SpscRing::new` rejects zero channels, zero or non-power-of-two capacity typed (`ring.rs:70-73`); `capacity_frames() = mask + 1` (`ring.rs:94-96`); `NullSink::write_bytes` (null.rs:182-229) refuses partial frames typed, refuses whole-buffer on `free < frames` typed `audio.ring_full` BEFORE pushing (free can only grow between check and push under SPSC — single producer), pushes frame-aligned chunks (`step = (256 / frame_samples).max(1) * frame_samples` — the documented 6.FIX-pcm fix for `256 % 6 ≠ 0`), and re-checks `pushed_total == frames` after (honest refusal over silent drop); `Shared::open` (null.rs:74-83) rejects non-F32/non-interleaved/zero-channel/zero-rate formats typed, so `latency()`'s `nanos = frames * 1e9 / rate` cannot divide by zero; the shipped ring size is `RING_FRAMES: usize = 131_072` (pipeline_host.rs:118, power of two, ≈1 MB stereo, allocated once).

## 3. Findings
F-001 [S3]
Where: crates/audio-engine/src/buffer.rs:40-48, 58; apps/cli/src/pipeline_host.rs:663; apps/web/src/host.rs:233
Code (buffer.rs:40-41, 58):
```
/// The mixer's output target (doc 01 §2.5 `AudioSink`).
```
```
    fn write(&mut self, buf: &AudioBuffer) -> Result<(), media_types::EngineError>;
```
Code (pipeline_host.rs:662-663):
```
        if let Some(buffer) = self.pipeline.take_output() {
            self.sink.write_bytes(buffer.data.as_slice())?;
```
What is wrong: `AudioSink::write` — the abstraction this file exists to publish — has no shipped caller. The CLI's audio path (pipeline_host.rs:663) and the web host (host.rs:233) both call the `NullSink`-inherent `write_bytes` directly; the only code that calls the trait's `write` is `PcmPump` (audio-output/src/pump.rs), and `PcmPump` is constructed only by tests (audio-output/tests/pcm_pipeline.rs). Consequently the trait-level contract this file documents (format-mismatch refusal, partial-frame typing — `NullSink::write`'s checks at null.rs:242-252) is dead in production; the shipped path gets only `write_bytes`' checks. The doc's framing "AudioSink is what the mixer feeds" is false for both shipped entry points. Blast radius: architectural (a format-mismatch regression in the trait impl would not be caught by any shipped path), no user-visible defect. What correct looks like: route `stage` through the trait (`AudioSink::write(&mut self.sink, &buffer)`), which `write_bytes` already mirrors, or demote the trait doc to "the pump-facing contract (test/unwired lanes)". Confidence: HIGH — Verification: read (rg for `.write(` / `write_bytes` / `PcmPump` across apps/ and crates/, results quoted).
F-002 [S4]
Where: crates/audio-engine/src/buffer.rs:60-62
Code:
```
    /// The device's current output latency, when known. `None` is an
    /// honest "not measurable" (e.g. a discard sink), never a guess.
    fn latency(&self) -> Option<Duration>;
```
What is wrong: the doc's named example contradicts the only shipped implementation: the discard sink (`NullSink`, the headless output the CLI actually uses) returns `Some(...)` always — it computes ring fill (null.rs:255-260: `available_frames() * 1e9 / rate`). A "discard sink" is precisely the implementation the doc offers as the canonical `None` case, and it measures. Misleading example on a contract method. Confidence: HIGH — Verification: read (both sites quoted).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `AudioBuffer::frames` | 35-37 | REAL | — | Delegates to zero-guarded `frames_in`; partial frames → None |
| `AudioSink::write` | 58 | REAL code, UNREACHABLE in shipped build | S3 | F-001: bypassed by both shipped hosts |
| `AudioSink::latency` | 62 | REAL | S4 | Doc example contradicts NullSink behavior (F-002) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| buffer_frame_math | YES | Whole-frame count exact; 7-byte partial → None asserted |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The bytes are always interleaved in format's layout" | buffer.rs:17-19 | REAL (convention) | Stage D interleaves (pipeline.rs stage D); the type itself enforces nothing — acceptable for a carrier; sinks re-check frame alignment (null.rs:183-190, 245-251) |
| "AudioSink is what the mixer feeds" | buffer.rs:4-5, 40 | FALSE (for shipped paths) | F-001: both shipped hosts call `write_bytes` directly |
| "`None` is an honest 'not measurable' (e.g. a discard sink)" | buffer.rs:60-61 | SHALLOW | F-002: the shipped discard sink always measures |
| Hostile-focus premise "buffer.rs — the ring buffer feeding the null device" | task assignment | FALSE (misattributed) | The ring is `SpscRing` (ring.rs) + `NullSink` (null.rs); verified sound — section 2 |

## 7. Dead / duplicated / unreachable code
- `AudioSink::write`: unreachable from shipped entry points (F-001) — implementations exist (NullSink, StubSink, platform adapters) and are exercised by tests only.
- No duplication; the file is minimal.

## 8. Dependencies & flags
- Depends on media-types only (AudioFormat, SharedBytes, Duration/Stamp). No features, no unsafe, no allocation.

## 9. Open questions for the next agent
- Whoever audits `crates/audio-output/src/ring.rs` and `null.rs` (not in my assignment): the SPSC ring's memory-ordering discipline (Relaxed cursors on the producer, Acquire on `available_frames`) deserves a dedicated audit — I verified the capacity/bounds math only, and `render_into` (render.rs) is the real-time consumer half.
- `PcmPump` (pump.rs) — the intended production consumer of `AudioSink::write` — is itself constructed only by tests; if the Phase-3 session wiring ever lands, F-001 resolves itself. Trace card ownership.
