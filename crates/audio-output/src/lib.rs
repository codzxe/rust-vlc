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
//!   source"). Implemented by [`NullBackend`] (fully, CI-safe) and
//!   [`UnsupportedBackend`] (typed not-supported stubs for real
//!   devices).
//! - [`SpscRing`] + [`render_into`] — the output ring and the
//!   real-time callback core under the **no-blocking contract**
//!   (doc 03 §4.9 c), with a per-call operation ledger
//!   ([`RenderStats`]) that the callback-budget AC asserts.
//! - [`SampleCounter`] — the atomic, offset-carried counter
//!   implementing [`media_clock::SampleCounterSource`] (monotonic
//!   while the device runs, frozen while paused, never rewinding
//!   across device switches, doc 03 §4.9 e).
//!
//! # The backend decision (recorded honestly)
//!
//! The card's first decision was the cpal `[VERIFY-P0]`: **cpal
//! 0.18.2 is conditional-E2 and NOT admitted** — its admission
//! vehicle is ADR-005 (Lead, unwritten), doc 01 §2.6/§2.7 place OS
//! backends in `platform-*` (L5) rather than this L4 crate, and this
//! sandbox can neither build it (no ALSA dev package: the alsa-sys
//! pkg-config probe fails) nor test it (no audio device). The full
//! R1–R10 evidence lives in
//! `docs/dependencies/records/cpal-0.18.2.md`. The card's named
//! fallback is what ships: real-device adapters are stubbed with
//! typed not-supported errors; when ADR-005 decides cpal-vs-direct,
//! implementations land in `platform-*` against this trait layer.
//!
//! The wave-14 admission spike (issue #1) refreshed that evidence
//! (`docs/dependencies/records/audio-output-cpal.md`: real
//! `cargo tree` graphs, the C-FFI unsafe counts, the alternatives
//! survey — verdict unchanged, **not admitted**) and added this
//! crate's flip point: [`cpal_backend`], the typed
//! [`BackendNotAdmitted`] constructor marker behind the
//! [`AUDIO_ADR005_ADMITTED`] gate (the streaming TLS seam's
//! pattern). The same wave shipped the PCM pipeline proof behind the
//! seam: [`SinePattern`] (the exact test-pattern generator) and
//! [`PcmPump`] (decoded frames → bounded queue → sink, paced by the
//! playback clock — the `MediaClock::audio_master` integration),
//! proven in `tests/pcm_pipeline.rs`.
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
//! Std-only; no `unsafe`.
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

pub use counter::SampleCounter;
pub use cpal::{AUDIO_ADR005_ADMITTED, BackendNotAdmitted, cpal_backend};
pub use device::{AudioBackend, DeviceDesc, DeviceId, SinkEvent};
pub use null::{NullBackend, NullConfig, NullStream};
pub use pattern::SinePattern;
pub use pump::{DropPolicy, PcmPump, PushOutcome};
pub use render::{RenderCallback, RenderStats, RingRenderer, render_into};
pub use ring::{RingRead, SpscRing};
pub use stub::UnsupportedBackend;

/// The backend set v0 ships: the null backend first (headless/CI),
/// then the typed not-supported stub for the current OS (the real
/// backends' placeholder until ADR-005 lands them in `platform-*`).
///
/// Selection policy (fallback order per device loss, doc 03 §4.9 d)
/// is deliberately the caller's in v0 — ADR-005 owns it when the
/// real backends exist `[PROPOSAL]`.
pub fn builtin_backends() -> Vec<Box<dyn AudioBackend>> {
    vec![
        Box::new(NullBackend::new()),
        Box::new(UnsupportedBackend::for_current_os()),
    ]
}
