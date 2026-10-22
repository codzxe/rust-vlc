//! The Lumen audio path after decode: volume, channel map, straight
//! mix (card 2.C3; doc 03 §4.8; doc 01 §2.2 registry row — "the audio
//! master"'s engine side).
//!
//! # What this crate is (v0 scope, doc 13 card 2.C3)
//!
//! The deterministic DSP between decode and the output device:
//!
//! 1. **Stage A — decoder-output normalization** ([`convert`]): any
//!    decoded [`AudioFrame`] (s16/s24/s32/u8/f32/f64, interleaved or
//!    planar) becomes f32 **planar** samples — the mixer format
//!    (doc 03 §4.8 e, floating-point planar `[PROPOSAL]`).
//! 2. **Per-track volume + channel map** ([`Volume`], [`ChannelMap`]):
//!    linear or dB gain, and stereo-pair routing (identity, swap,
//!    mono→stereo, stereo→mono, explicit routes).
//! 3. **Stage C — the mixer** ([`Mixer`]): a *straight* sum of all
//!    active tracks with per-track gain. No reverb, no EQ, no
//!    crossfade — those are later filter-chain cards (doc 03 §4.8 b,
//!    insertion point B is empty in v0).
//! 4. **Stage D — device adaptation** ([`convert`]): the mixed f32
//!    planes become interleaved f32 bytes (the device format v0
//!    devices speak; integer targets are converted with saturating
//!    clamp for the backends later cards add).
//!
//! [`AudioPipeline`] composes 1–4 over preallocated buffers (doc 03
//! §4.8 c — no per-frame allocation after format negotiation) and
//! hands the result to an [`AudioSink`] as an [`AudioBuffer`].
//!
//! # Sample-count conservation (the clock contract)
//!
//! Doc 03 §4.8 e: *"Sample counts leaving the pipeline map 1:1 to
//! device samples advanced (clock correctness depends on this)."* The
//! pipeline is rate-preserving by construction: every stage maps one
//! input sample-instant to one output sample-instant (the mixer sums
//! tracks sample-wise; the channel map changes channel count, never
//! frame count). The **bounded drift micro-resampler** (card 3.B1,
//! doc 03 §4.7 e — [`MicroResampler`], governed by [`DriftGovernor`])
//! is the one sanctioned exception: a non-zero [`DriftCorrection`]
//! shifts the quantum's device-sample count by ≤ ±1000 ppm — exactly
//! doc 03 §4.8 f's "unless the filter declares a rate change" — while
//! `DriftCorrection::ZERO` restores the 1:1 path bit-exactly. A track
//! whose *rate* differs from the negotiated mixer rate remains a
//! typed [`Unsupported`](media_types::ErrorKind::Unsupported) error,
//! never a silent resample (track-rate conversion is a different,
//! later card).
//!
//! # Errors
//!
//! Everything is [`media_types::EngineError`] at stage
//! [`Mix`](media_types::PipelineStage::Mix) with stable dotted context
//! codes (see [`error`]). No panics, no unwraps; malformed payloads
//! are typed [`Corrupt`](media_types::ErrorKind::Corrupt) errors.
//!
//! # Never
//!
//! Device APIs (backends' job — `audio-output`), blocking work,
//! threads, UI (doc 01 §2.2 d). Std-only; no `unsafe`.
//!
//! # Example
//!
//! One decoded stereo chunk at unity gain through to the sink format:
//!
//! ```
//! use audio_engine::{AudioPipeline, PipelineSpec, Volume};
//! use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};
//! use media_types::time::{Stamp, Timestamp};
//! use media_types::ids::GenerationId;
//! use media_types::frame::AudioFrame;
//! use media_types::packet::SharedBytes;
//!
//! # fn main() -> Result<(), media_types::EngineError> {
//! let spec = PipelineSpec {
//!     rate: 44_100,
//!     channels: 2,
//!     tracks: 1,
//! };
//! let mut pipeline = AudioPipeline::new(spec)?;
//! pipeline.set_volume(0, Volume::UNITY)?;
//!
//! // A decoded s16le stereo chunk: frames (L, R) = (0, 0), (16384, -16384).
//! let format = AudioFormat {
//!     sample: SampleFormat::S16,
//!     channels: 2,
//!     layout: AudioLayout::Interleaved,
//!     rate: 44_100,
//! };
//! let mut pcm = Vec::new();
//! pcm.extend_from_slice(&0i16.to_le_bytes());
//! pcm.extend_from_slice(&0i16.to_le_bytes());
//! pcm.extend_from_slice(&16384i16.to_le_bytes());
//! pcm.extend_from_slice(&(-16384i16).to_le_bytes());
//! let frame = AudioFrame {
//!     format,
//!     samples: SharedBytes::from_vec(pcm),
//!     pts: Stamp::Present(Timestamp::from_parts(0, 1, 44_100)?),
//!     duration: None,
//!     generation: GenerationId::FIRST,
//! };
//!
//! pipeline.process(0, &frame)?;
//! let had_output = pipeline.mix()?;
//! assert!(had_output);
//! let buffer = pipeline.take_output();
//! // 2 frames * 2 channels * 4 bytes of interleaved f32.
//! assert_eq!(buffer.map(|b| b.data.len()), Some(16));
//! # Ok(())
//! # }
//! ```

pub mod buffer;
pub mod channel_map;
pub mod convert;
pub mod error;
pub mod governor;
pub mod mix;
pub mod pipeline;
pub mod resample;
pub mod volume;

pub use buffer::{AudioBuffer, AudioSink};
pub use channel_map::ChannelMap;
pub use governor::{DriftGovernor, DriftStats};
pub use mix::Mixer;
pub use pipeline::{AudioPipeline, PipelineSpec};
pub use resample::MicroResampler;
pub use volume::Volume;
