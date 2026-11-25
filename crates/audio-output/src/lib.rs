//! The Lumen audio output: the device abstraction, the real-time
//! render callback, and the null backend (card 2.C3; doc 03 §4.9;
//! doc 01 §2.6 registry row).
//!
//! # What this crate is (v0)
//!
//! The **adapter trait layer** between the engine and whatever
//! actually moves samples:
//!
//! - [`AudioBackend`] — extends [`audio_engine::AudioSink`] with
//!   device enumeration, open/pause/resume, the underrun count and
//!   the **sample counter** — the media-clock truth source (doc 03
//!   §4.7: "the device sample counter is the only trusted progress
//!   source"). Implemented by [`NullBackend`] (fully, CI-safe),
//!   [`UnsupportedBackend`] (typed not-supported stubs), and — behind
//!   the `backend-cpal` feature, since the ADR-005 amendment —
//!   `CpalBackend` (the admitted real-device adapter).
//! - [`SpscRing`] + [`render_into`] — the output ring and the
//!   real-time callback core under the **no-blocking contract**
//!   (doc 03 §4.9 c), with a per-call operation ledger
//!   ([`RenderStats`]) that the callback-budget AC asserts.
//! - [`SampleCounter`] — the atomic, offset-carried counter
//!   implementing [`media_clock::SampleCounterSource`] (monotonic
//!   while the device runs, frozen while paused, never rewinding
//!   across device switches, doc 03 §4.9 e).
//!
//! # The backend decision (recorded honestly, then amended)
//!
//! The card's first decision was the cpal `[VERIFY-P0]` — cpal 0.18.2
//! conditional-E2 and **not admitted** (R1–R10 evidence in
//! `docs/dependencies/records/cpal-0.18.2.md`, refreshed by the
//! wave-14 admission spike,
//! `docs/dependencies/records/audio-output-cpal.md`). The wave-FX3
//! Lead decision (FIX-PLAN §3 F1.1 **Route A**) amends ADR-005 and
//! admits it: cpal =0.18.2 is now an **optional** dependency behind
//! the `backend-cpal` Cargo feature, with the real adapter in
//! [`crate::cpal`] behind [`cpal_backend`]. The feature is **off by
//! default**: default builds stay workspace-internal, std-only and
//! byte-compatible with the pre-admission crate — the seam falls
//! back to the null backend, and [`cpal_linked`] reports whether a
//! given build links the substrate. The admission paperwork
//! (ADR-005 amendment, allowlist rows, deny.toml) is FX3-A02's in
//! this wave; the not-admitted marker vocabulary the seam used to
//! carry is deleted with the admission (the flip-point discipline's
//! same-change rule).
//!
//! The wave-14 seam also shipped the PCM pipeline proof behind it:
//! [`SinePattern`] (the exact test-pattern generator) and [`PcmPump`]
//! (decoded frames → bounded queue → sink, paced by the playback
//! clock — the `MediaClock::audio_master` integration), proven in
//! `tests/pcm_pipeline.rs`.
//!
//! # The no-blocking contract (doc 03 §4.9 c, binding)
//!
//! Inside the callback only: ring index reads (wait-free atomics),
//! frame copies into the caller's buffer, silence fills, the sample
//! counter increment, the underrun count. **Forbidden**: any lock,
//! allocation, logging, syscall, unbounded loop. The v0 core
//! ([`render_into`]) satisfies this *by construction*: it takes only
//! borrows, constructs no owned values, and its loop is bounded by
//! the power-of-two ring geometry (at most one wrap ⇒ at most two
//! contiguous segments per call). The budget is **measured**, not
//! just argued: [`RenderStats`] counts every operation class and the
//! callback-budget AC asserts the bounds on the null adapter. The
//! doc's fault-injecting allocator hook is `[PROPOSAL: tooling]`
//! testkit work (it needs a `#[global_allocator]`, i.e. `unsafe`,
//! which only platform adapters may carry — doc 01 §2.7); the
//! structural proof + op-count ledger + zero-growth assertions are
//! the v0 gate, exactly what doc 03 §4.9 c names ("code review plus
//! these tests are the gate").
//!
//! # Threading
//!
//! No threads are spawned here (the session thread is Phase-3 wiring,
//! doc 03 §9; same call the 2.C4 video scheduler made). The ring's
//! **SPSC discipline** is the contract: exactly one producer (the
//! pump, calling [`AudioSink::write`]) and exactly one consumer (the
//! device, calling [`render_into`]) — the null backend splits the two
//! halves explicitly ([`NullStream::sink`] vs [`NullStream::device`])
//! so a wrong-thread use is a type-level mistake, not a data race.
//!
//! # Never
//!
//! Decode/mix logic (audio-engine's job), OS types in public
//! signatures (ARCH-6), blocking or locking inside the callback.
//! Default build: std-only, no `unsafe`. The `backend-cpal` feature
//! links the ADR-005-admitted OS-service binding (all unsafe inside
//! cpal's host layers, per the E2 safety review) — the adapter code
//! in this crate stays std-only and unsafe-free either way.
//!
//! # Example
//!
//! The null adapter round-trips one buffer:
//!
//! ```
//! use audio_output::{NullBackend, NullConfig};
//! use audio_engine::AudioSink;
//! use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};
//! use media_types::packet::SharedBytes;
//!
//! # fn main() -> Result<(), media_types::EngineError> {
//! let format = AudioFormat {
//!     sample: SampleFormat::F32,
//!     channels: 2,
//!     layout: AudioLayout::Interleaved,
//!     rate: 44_100,
//! };
//! let mut stream = NullBackend::open_stream(&format, 64, NullConfig { capture: true })?;
//!
//! // The pump writes one stereo frame (0.5, -0.5).
//! let mut data = Vec::new();
//! data.extend_from_slice(&0.5f32.to_le_bytes());
//! data.extend_from_slice(&(-0.5f32).to_le_bytes());
//! stream.sink.write_bytes(&data)?;
//!
//! // The "device" renders one frame and captured what played.
//! let mut out = vec![0.0f32; 2];
//! let stats = stream.device.render(&mut out)?;
//! assert_eq!(stats.frames_read, 1);
//! assert_eq!(stats.silence_filled, 0);
//! assert_eq!(out, vec![0.5, -0.5]);
//! assert_eq!(stream.device.take_capture(), data);
//! # Ok(())
//! # }
//! ```

pub mod counter;
pub mod cpal;
pub mod device;
pub mod error;
pub mod null;
pub mod pattern;
pub mod pump;
pub mod render;
pub mod ring;
pub mod stub;

pub use counter::{SampleCounter, SinkStats};
#[cfg(feature = "backend-cpal")]
pub use cpal::CpalBackend;
pub use cpal::{AUDIO_ADR005_ADMITTED, cpal_backend, cpal_linked};
pub use device::{AudioBackend, DeviceDesc, DeviceId, SinkEvent};
pub use null::{NullBackend, NullConfig, NullStream};
pub use pattern::SinePattern;
pub use pump::{DropPolicy, PcmPump, PushOutcome};
pub use render::{RenderCallback, RenderStats, RingRenderer, render_into};
pub use ring::{RingRead, SpscRing};
pub use stub::UnsupportedBackend;

/// The backend set v0 ships: with the feature off (the default), the
/// null backend first (headless/CI), then the typed not-supported
/// stub for the current OS — exactly the pre-admission list.
/// Builds with `backend-cpal` put the admitted real adapter first
/// (its own `enumerate`/`open` tell the truth about the devices this
/// host actually has — empty is honest), falling back down the list.
///
/// Selection policy (fallback order per device loss, doc 03 §4.9 d)
/// is deliberately the caller's in v0 — the ADR-005 amendment owns
/// the configured policy `[PROPOSAL]`.
pub fn builtin_backends() -> Vec<Box<dyn AudioBackend>> {
    let mut backends: Vec<Box<dyn AudioBackend>> = Vec::new();
    #[cfg(feature = "backend-cpal")]
    backends.push(Box::new(crate::cpal::CpalBackend::new()));
    backends.push(Box::new(NullBackend::new()));
    backends.push(Box::new(UnsupportedBackend::for_current_os()));
    backends
}
