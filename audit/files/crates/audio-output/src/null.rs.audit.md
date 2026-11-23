# AUDIT: crates/audio-output/src/null.rs
| Field | Value |
|---|---|
| Auditor | W08-A04 |
| Date | 2026-11-23 |
| Lines / functions | 748 / 29 production fns + 8 tests + 2 helpers |
| File verdict | REAL (the shipped sink — a discard device; product-level audio claim cross-ref F-001) |
| Reachable from shipped entry point? | YES: lumen-cli `main` → `run.rs repl_mode` → `lumen_dispatch` → `open_audio_session` (apps/cli/src/pipeline_host.rs:1587-1599 `NullBackend::open_stream`) → driven by `CliHost::pump/advance_time/pump_wallclock` → `NullDevice::render` (host.rs:614/726/824); desktop: apps/desktop/src/shell.rs:280 `pump_session` → `CliHost::pump_wallclock`; web host also imports `NullDevice/NullSink` (apps/web/src/host.rs:75-76) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 4 |
| Commands run | `cargo test -p audio-output` → 33 lib + 4 callback_budget + 1 clock_truth + 4 pcm_pipeline + 3 sine_sink_hash + 1 zero_alloc + 1 doctest, ALL PASS (exit 0); rg reachability traces; sed quote verification |

## 1. One-paragraph verdict
This is the only audio sink any shipped lane ever opens, and it is a
discard device: the pump writes decoded PCM into an SPSC ring, the host
ticker "renders" it into a scratch buffer, and the bytes are dropped —
no OS audio API is touched anywhere (verified: `rg "cpal|alsa|wasapi|coreaudio"` over apps/ finds no native audio driver call). As code it
is honest and genuinely built: real wait-free ring, real counter
semantics, real backpressure, typed errors everywhere, and the tests
execute it (I ran them). The fraud is one level up (product.toml
`audio-output = true` — already S0 in audit/files/product.toml.audit.md
F-001); this file's own docs say "headless discard" plainly. The one
in-file defect worth flagging is a false invariant comment:
`written − consumed` is NOT the ring's live content once any underrun
silence has been consumed (F-002).

## 2. What actually works (verified)
— verified by `cargo test -p audio-output` (exit 0): 47/47 pass,
including `sine_sink_hash` (byte-stable capture hash), `zero_alloc_null_sink`
(zero heap growth on the write path after warm-up), `clock_truth`
(pause freezes counter, 44 100 instants = exactly 1 s), and
`backpressure_is_typed_and_all_or_nothing` (FIFO intact on refusal).

## 3. Findings
F-001 [S2]
Where: crates/audio-output/src/null.rs:434-437 (the shipped device
enumeration), vs product.toml:150.
Code:
```
    fn enumerate(&self) -> Vec<DeviceDesc> {
        vec![DeviceDesc {
            id: NULL_DEVICE_ID,
            name: "null (headless discard)".to_string(),
```
What is wrong: every user on every OS gets "null (headless discard)".
"Playback" through this sink means: decode → ring → `NullDevice::render`
into a host-allocated scratch Vec → drop. No sound is possible from any
shipped binary. The crate is honest about this; product.toml:150 is not
(`audio-output = true # default audio device output` — S0 already filed
as F-001 in audit/files/product.toml.audit.md, verified there). Why it
matters: the shipped product's core audio feature is a no-op by
construction, and every underrun/latency/backpressure behavior below is
unobservable by a listener. What correct looks like: the ADR-005 flip
(see cpal.rs audit) or product.toml set to false. Confidence: HIGH —
Verification: read + rg (no audio backend construction outside
NullBackend/stub in any apps/ lane).
F-002 [S3]
Where: crates/audio-output/src/null.rs:146-153 (doc claim on
`written_frames`), vs crates/audio-output/src/render.rs:106-108 and
apps/cli/src/pipeline_host.rs:1143.
Code:
```
    /// flush reads this (6.FIX-ogg-reanchor): `written − consumed`
    /// is the ring's live content, and a generation change-point
```
vs render.rs:
```
    // One counter advance: every rendered frame (data or silence)
    // was consumed by the device.
    counter.advance(frames_requested as u64);
```
and the consumer, pipeline_host.rs:1143: `written_old.saturating_sub(consumed)`.
What is wrong: `consumed` (the device counter) counts data + silence
(null.rs's own test asserts `sample_count() == 3` after 1 data + 2
silence frames, line 558-559); `written` counts data only. After any
underrun, `written − consumed` undercounts the live ring content and
`saturating_sub` clamps it to 0 — a seek boundary would then compute a
stale prefix of 0 and discard nothing, re-introducing exactly the
pre-seek audio play-through the 6.FIX claims to have fixed. Latent only
because every shipped driver render is sized to the available frames
(host.rs:601/715 gate on `buffered_frames() >= RENDER_QUANTUM`;
drain_tail/discard are sized renders), so shipped flows never consume
silence. Why it matters: the invariant is documented as exact and is
not; the first real audio backend (callback-driven, underruns real)
inherits the wrong arithmetic silently. What correct looks like:
`consumed_data = counter − total_silence_rendered`, or track ring
occupancy directly (`buffered_frames()`), and fix the doc.
Confidence: HIGH (mismatch proven from code+tests); reachability of the
bad case in shipped lanes: MEDIUM (reasoned, not executed — I did not
construct a seek-after-underrun session).
F-003 [S4]
Where: crates/audio-output/src/lib.rs:152-157 (this file's only
registry entry point).
Code:
```
pub fn builtin_backends() -> Vec<Box<dyn AudioBackend>> {
    vec![
        Box::new(NullBackend::new()),
        Box::new(UnsupportedBackend::for_current_os()),
    ]
}
```
What is wrong: zero callers anywhere in the repo (rg over crates/ and
apps/ finds only the definition). The documented "backend set v0
ships" is never shipped — every lane constructs `NullBackend::open_stream`
directly. Dead export; the doc's "Selection policy … is deliberately
the caller's" is moot because the set is never offered. Confidence:
HIGH — Verification: rg (no matches beyond definition).
F-004 [S4]
Where: crates/audio-output/src/null.rs:296-300.
Code:
```
        if self.shared.paused.load(Ordering::Acquire) {
            // The device is not pulling: no callback, no counter, no
            // capture (doc 03 §4.7 e — paused freezes the counter).
            return Ok(RenderStats::default());
```
What is wrong: a paused render reports `frames_requested == 0` for a
non-empty `out`; stats cannot distinguish a paused no-op from a
zero-frame request. Callers must consult `paused()` separately.
Minor honesty defect in an otherwise meticulously-typed stats API.
Confidence: HIGH — Verification: read + test at lines 604-607.
F-005 [S4]
Where: crates/audio-output/src/null.rs:409-415.
Code:
```
        let shared = self.shared()?;
        let format = self
            .session
            .as_ref()
            .map(|(_, f)| *f)
            .ok_or_else(|| error::unsupported("audio.sink_not_open"))?;
```
What is wrong: dead second check — `shared()?` already fails when no
session, so `ok_or_else` is unreachable; duplicated not-open handling.
Confidence: HIGH — Verification: read.
F-006 [S4]
Where: crates/audio-output/src/null.rs:307-311.
Code:
```
        if let Some(capture) = self.capture.as_mut() {
            for sample in out.iter() {
                capture.extend_from_slice(&sample.to_le_bytes());
            }
        }
```
What is wrong: per-sample `extend_from_slice` (4 bytes at a time) —
O(frames×channels) tiny calls instead of one chunked copy. Capture is
an AC-suite-only configuration (shipped lanes open with
`capture: false`, pipeline_host.rs:1599), so blast radius is test time
only. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Shared::open | 74-91 | REAL | — | validates F32/interleaved/ch≠0/rate≠0; typed refusal |
| NullSink::clock_source | 123-125 | REAL | — | shares device counter (verified by clock_truth) |
| NullSink::underruns | 128-130 | REAL | — | Relaxed load, stats-only |
| NullSink::sample_count | 134-136 | REAL | — | counter.count() |
| NullSink::buffered_frames | 142-144 | REAL | — | ring.available_frames() |
| NullSink::written_frames | 154-156 | REAL | F-002 | doc invariant false under underrun |
| NullSink::write_bytes | 182-226 | REAL | — | all-or-nothing via free-check + single producer; frame-aligned step fix (256%6) real |
| NullSink::push_aligned | 232-241 | REAL | — | as_chunks::<4>, last-chunk tail handled, no garbage push |
| AudioSink::write (NullSink) | 245-258 | REAL | — | format match + alignment checks |
| AudioSink::latency | 260-265 | REAL | — | u128 math, try_from .ok()? — sound |
| NullDevice::render | 292-313 | REAL | F-004, F-006 | pause no-op; capture driver-side |
| NullDevice::take_capture | 318-323 | REAL | — | take-drains |
| NullDevice::paused | 326-328 | REAL | — | |
| NullDevice::underruns | 331-333 | REAL | — | |
| NullDevice::sample_count | 336-338 | REAL | — | |
| NullDevice::buffered_frames | 341-343 | REAL | — | |
| NullBackend::new | 357-359 | REAL | — | Default |
| NullBackend::open_stream | 370-387 | REAL | — | the constructor shipped lanes use |
| NullBackend::take_device | 391-393 | REAL | — | Option::take |
| NullBackend::is_open | 396-398 | REAL | — | |
| NullBackend::shared (private) | 400-405 | REAL | — | typed sink_not_open |
| AudioSink::write (NullBackend) | 409-421 | REAL | F-005 | dead second check |
| AudioSink::latency (NullBackend) | 423-430 | REAL | — | delegates |
| AudioBackend::enumerate | 434-445 | REAL | F-001 | "null (headless discard)" — the whole product truth |
| AudioBackend::open | 447-466 | REAL | — | parks driver; discard mode |
| AudioBackend::pause/resume | 468-476 | REAL | — | Acquire/Release flag |
| AudioBackend::clock_source | 478-481 | REAL | — | |
| AudioBackend::underruns | 483-487 | REAL | — | 0 when not open |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| open_stream_roundtrip_and_capture | YES | exact float + byte capture equality |
| underruns_are_counted_and_silence_captured | YES | pins silence fill, underrun count, counter=3 |
| backpressure_is_typed_and_all_or_nothing | YES | FIFO content checked after refusal |
| pause_freezes_counter_and_renders | YES | NaN canary proves paused render touches nothing |
| stream_mode_shares_the_counter_between_the_halves | YES | cross-half identity of counter/underruns |
| format_and_shape_errors_are_typed | YES | 8 distinct error codes asserted |
| latency_reports_the_ring_fill | YES | 16 frames @8 kHz = exactly 2 ms |
| renderer_callback_composes_over_the_shared_state | YES | write_bytes→render equality |
External AC suites (ran, pass): sine_sink_hash (byte-stable hash incl.
wrap/refusal), callback_budget (op-count ledger), clock_truth, zero_alloc.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "null backend for tests/headless … NullSink for determinism" | null.rs:1-3 quoting doc 01/03 | REAL | code is exactly that; tests ran |
| "an undriven session fills its ring and refuses further writes with typed audio.ring_full" | null.rs:20-23 | REAL | backpressure test passes |
| "v0 thread model is manual driving on the caller's thread" | null.rs:25-28 | REAL | no thread spawn in crate; host pump drives |
| "`written − consumed` is the ring's live content" | null.rs:149-150 | FALSE under underrun | render.rs:106-108 counts silence; null.rs:559 asserts counter=3 after 1 data frame |
| "default audio device output" | product.toml:150 | FALSE | this file's enumerate() = "null (headless discard)"; see product.toml.audit.md F-001 |
| matrix row platform-aout-cpal "partial … no real-device evidence exists" | docs/feature-matrix.toml:2361-2365 | REAL (honest) | matches code state |

## 7. Dead / duplicated / unreachable code
- `builtin_backends()` (lib.rs:152): zero callers — F-003.
- `NullStream`/`NullSink`/`NullDevice` all reachable (CLI open path).
- `RenderCallback`/`RingRenderer` (render.rs) not used by null.rs's
  driver path — next agent should check reachability of render.rs's
  trait exports.

## 8. Dependencies & flags
Std-only + workspace crates (audio-engine, media-clock, media-types);
testkit dev-only. No feature gates on this file — it compiles in the
release lane unconditionally (audio-output is a dependency of apps/cli
and apps/web, verified in Cargo.toml). No unsafe.

## 9. Open questions for the next agent
- Host-side: every render allocates a fresh `vec![0.0_f32; RENDER_QUANTUM * channels]` per quantum inside the pump loop (apps/cli/src/host.rs:613, 725, 823, 1273) — contradicts the D-1 zero-heap discipline the sink itself upholds (zero_alloc_null_sink tests the SINK, not the driver). Which agent owns host.rs?
- The wall-clock render gate `buffered_frames() >= RENDER_QUANTUM` (host.rs:601) strands a sub-quantum residue while Playing (only Ended drains it via drain_tail_at_end) — can a decode-stall + sub-quantum fill freeze the clock below the park threshold? UNVERIFIED (needs a scripted stall scenario).
