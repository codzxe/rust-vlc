//! The pipeline composition the engine drives (card 2.D2): the
//! decoder provider and the frame consumer that carry the first
//! vertical's audio and video paths.
//!
//! # Decoder provider (the registry shim)
//!
//! [`HostDecoderProvider`] answers the engine's per-track
//! [`DecoderProvider`] seam with the real builtin decoders: MJPEG
//! (2.B4) as-is, and PCM (2.B5) seeded with the `strf` facts the
//! open-path pre-pass harvested ([`PcmDecoder::with_format`] — the
//! typed seam 2.B5 documented for exactly this host shape, because
//! the engine's `Pipeline::wire` passes no WAVEFORMATEX `extra_data`).
//!
//! # Frame consumer (audio + video halves)
//!
//! [`CliFrameConsumer`] implements the engine's [`FrameConsumer`]
//! seam on the poll thread:
//!
//! - **audio**: `AudioPipeline` (2.C3: s16→f32 conversion, unity
//!   volume, identity map, straight mix) → the null sink's ring
//!   (2.C3's null backend — no audio device in this sandbox). The
//!   ring's shared counter is the session clock's truth source.
//! - **video**: an I420→RGBA host conversion (BT.601 full-range, the
//!   JFIF equations — the renderer's v0 upload path is RGBA-only and
//!   the color-conversion card has not landed; 2.D1's worklog named
//!   this exact "converts upstream" slot) → the `PresentationScheduler`
//!   (2.C4) → the bounded `PresentationQueue` → the software `Renderer`
//!   (2.D1) on a headless surface. RGBA-native frames pass through
//!   unconverted.
//!
//! The device half of the null audio session and the scheduler's
//! tick/present driving live in [`crate::host`]'s pump — the consumer
//! only takes delivery; the pump advances time (audio renders move
//! the counter, which moves the clock, which gates video presents).
//!
//! # Determinism
//!
//! Everything here is driven, never scheduled: the pump renders audio
//! quanta and ticks the scheduler in lockstep, so the whole path —
//! decode, mix, ring, counter, clock, scheduler, presents — advances
//! deterministically with zero wall-clock reads on the audio-bearing
//! path. `[PROPOSAL: the render quantum (1470 frames ≈ 33.33 ms at
//! 44.1 kHz) matches the fixture's own audio chunk shape; a
//! device-period setting joins with real backends per ADR-005]`

use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use audio_engine::{AudioPipeline, PipelineSpec};
use audio_output::null::{NullDevice, NullSink};
use audio_output::{NullBackend, NullConfig, NullStream};
use codec_api::{Decoder, DecoderConfig};
use media_clock::MediaClock;
use media_types::frame::{
    AudioFormat, AudioFrame, AudioLayout, ColorMatrix, ColorRange, FrameStorage, SampleFormat,
    VideoFrame,
};
use media_types::time::Timestamp;
use media_types::{
    EngineError, ErrorKind, Frame, PipelineStage, Rate, Track, TrackId,
};
use playback_engine::{DecoderProvider, FrameConsumer};
use renderer_wgpu::color::SurfaceFormat;
use renderer_wgpu::params::Extent2D;
use renderer_wgpu::renderer::Renderer;
use renderer_wgpu::screenshot;
use renderer_wgpu::surface::{PresentMode, SurfaceConfig};
use video_engine::scheduler::{PresentAction, PresentationScheduler, PushOutcome, TickOutcome};
use video_engine::sink::{PresentRequest, Presenter};
use video_engine::{EnqueueOutcome, PresentationQueue, SchedulerCfg};

use crate::demux_host::MediaFacts;

/// The audio render quantum (frames per device render — see the module
/// docs' determinism note).
pub const RENDER_QUANTUM: usize = 1470;

/// The null output ring's capacity (frames; power of two — the ring's
/// own shape rule). Sizing math (the invariant): one engine poll
/// decodes at most `TickBudget::default().packets_per_limb` (16)
/// audio packets → 16 × 1470 = 23 520 frames pushed between the
/// pump's render passes, which drain to a < 1470 residue — so the
/// Playing steady state needs only ~25k. The hazard is the
/// **rebuffer** cycle: an underrun freezes the session clock (so the
/// pump's renders stop) while `Buffering` keeps decoding into the
/// ring until the queues re-prime (~2–3 pumps at 23 520 each). 131
/// 072 frames (≈3 s at 44.1 kHz, 1 MiB stereo f32) absorbs a full
/// rebuffer cycle with >2× margin; beyond that the typed
/// `audio.ring_full` refusal is the honest bound (a fatal, never a
/// drop). `[PROPOSAL: sizing tied to the default tick budget and the
/// fixture's quantum; a device-period setting joins with real
/// backends per ADR-005]`
pub const RING_FRAMES: usize = 131_072;

/// A presentation-queue slot count (one per scheduler tick, drained
/// in the same pump step).
#[allow(clippy::empty_loop)]
const QUEUE_CAPACITY: NonZeroUsize = match NonZeroUsize::new(4) {
    Some(value) => value,
    None => loop {}, // provably dead const arm (policy.rs precedent)
};

/// A host-minted engine error: stage + stable dotted code + a human
/// sentence. The sentence rides the `source` chain-root slot (L0's
/// own semantics for I/O roots) so [`crate::demux_host::wire_error`]
/// surfaces it as the command layer's message.
fn host_error(stage: PipelineStage, code: &'static str, message: impl Into<String>) -> EngineError {
    EngineError::new(
        stage,
        ErrorKind::Unsupported { format: None },
        media_types::error::ErrorContext::new(code),
    )
    .with_source(std::io::Error::other(message.into()))
}

// ---------------------------------------------------------------------------
// Decoder provider
// ---------------------------------------------------------------------------

/// The engine's decoder seam, answered with the builtin codecs
/// (the registry shim until the plugin registry card lands).
pub struct HostDecoderProvider {
    facts: Arc<MediaFacts>,
}

impl HostDecoderProvider {
    /// Build over the open path's harvested facts.
    #[must_use]
    pub fn new(facts: Arc<MediaFacts>) -> HostDecoderProvider {
        HostDecoderProvider { facts }
    }
}

impl DecoderProvider for HostDecoderProvider {
    fn decoder_for(
        &self,
        track: &Track,
        config: &DecoderConfig,
    ) -> Result<Box<dyn Decoder>, EngineError> {
        match track.codec.as_str() {
            "mjpeg" => {
                let mut decoder = codecs::video::mjpeg::MjpegDecoder::new();
                decoder.configure(config)?;
                Ok(Box::new(decoder))
            }
            codec if codec.starts_with("pcm_") => {
                // The strf facts the open path harvested (2.B5's
                // with_format seam — the engine passes no extra_data).
                let Some((channels, rate)) =
                    self.facts.fact_for(track.id).and_then(|fact| fact.pcm)
                else {
                    return Err(host_error(
                        PipelineStage::Decode,
                        "cli.pcm_facts_absent",
                        format!("no stream facts for PCM track {}", track.id.handle()),
                    ));
                };
                let mut decoder = codecs::audio::pcm::PcmDecoder::with_format(channels, rate)?;
                decoder.configure(config)?;
                Ok(Box::new(decoder))
            }
            other => Err(host_error(
                PipelineStage::Decode,
                "cli.codec_unavailable",
                format!("no decoder for `{other}` in this build (first vertical: mjpeg + pcm)"),
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// Frame consumer
// ---------------------------------------------------------------------------

/// The audio half: the processing pipeline plus the null sink.
pub struct AudioLimb {
    pipeline: AudioPipeline,
    sink: NullSink,
}

impl AudioLimb {
    /// The sink half's clock source (the device counter).
    #[must_use]
    pub fn clock_source(&self) -> Arc<dyn media_clock::SampleCounterSource> {
        self.sink.clock_source()
    }
}

/// The video half: scheduler + queue + renderer.
pub struct VideoLimb {
    scheduler: PresentationScheduler,
    queue: PresentationQueue,
    renderer: Renderer,
    /// Frames handed to the renderer (presents + repeats).
    pub presented: u64,
    /// Frames the scheduler refused (window full / unschedulable) —
    /// observable, never silent.
    pub refused: u64,
}

impl VideoLimb {
    /// One scheduler tick: enqueue the action's request and drain the
    /// queue to the renderer.
    ///
    /// # Errors
    /// The present path's typed errors (surface loss, upload refusal)
    /// — the pump surfaces them and degrades honestly.
    fn tick_once(&mut self) -> Result<(), EngineError> {
        let TickOutcome { action, .. } = self.scheduler.tick();
        match action {
            PresentAction::Present(request) | PresentAction::RepeatLast(request) => {
                self.enqueue_and_drain(request)
            }
            PresentAction::Background => Ok(()),
        }
    }

    fn enqueue_and_drain(&mut self, request: PresentRequest) -> Result<(), EngineError> {
        match self.queue.try_push(request) {
            EnqueueOutcome::Enqueued => {}
            EnqueueOutcome::Full { request, .. } => {
                // One slot per tick with spare capacity: unreachable
                // in this wiring; honored honestly by presenting the
                // refused request directly rather than dropping it.
                return self.render_request(request);
            }
            EnqueueOutcome::Stale { request, .. } => {
                // The generation moved between tick and drain (a
                // stop/seek landed): the request fell behind the
                // published generation — dropped at the boundary and
                // counted by the queue, never presented stale.
                let _ = request;
                return Ok(());
            }
        }
        let delivered = match self.queue.drain_to(&mut self.renderer) {
            Ok(count) => count,
            Err(error) => {
                // The delivered-before-failure count is the queue's own
                // truth (its stats also carry it); the presenter's error
                // propagates typed.
                self.presented = self.presented.saturating_add(error.delivered as u64);
                return Err(error.source);
            }
        };
        self.presented = self.presented.saturating_add(delivered as u64);
        Ok(())
    }

    fn render_request(&mut self, request: PresentRequest) -> Result<(), EngineError> {
        self.renderer.present(&request)?;
        self.presented = self.presented.saturating_add(1);
        Ok(())
    }
    /// The scheduler's lifetime counters.
    #[must_use]
    pub fn scheduler_stats(&self) -> video_engine::SchedulerStats {
        self.scheduler.stats()
    }

    /// The renderer's staging measurements (frames, bytes, copies).
    #[must_use]
    pub fn staging_stats(&self) -> renderer_wgpu::staging::StagingStats {
        self.renderer.staging_stats()
    }

    /// Encode the current surface as a PNG screenshot (2.D1's
    /// byte-stable encoder) — the headless frame evidence.
    ///
    /// # Errors
    /// The renderer's typed errors (no surface, readback shape).
    pub fn screenshot_png(&self) -> Result<Vec<u8>, EngineError> {
        let rgba = self
            .renderer
            .readback_rgba()
            .map_err(|err| err.to_engine_error())?;
        let extent = self
            .renderer
            .extent()
            .map_err(|err| err.to_engine_error())?;
        screenshot::encode_rgba(extent, &rgba).map_err(|err| err.to_engine_error())
    }
}

/// The consumer's mutable state (the engine calls `accept` on the poll
/// thread; the pump ticks the video half between polls — one mutex,
/// no contention in the single-writer model).
pub struct ConsumerState {
    audio: Option<AudioLimb>,
    video: Option<VideoLimb>,
    /// A present-path failure (renderer/queue). Surfaced by the pump;
    /// audio continues (the 2.D1 degrade pattern).
    video_error: Option<EngineError>,
    /// Frames that reached `accept`.
    pub frames_accepted: u64,
    /// Frames the scheduler refused (window full / unschedulable pts).
    pub frames_refused: u64,
}

impl ConsumerState {
    /// The video half (for the pump's ticking and the AC's accounting).
    #[must_use]
    pub fn video(&self) -> Option<&VideoLimb> {
        self.video.as_ref()
    }

    /// The audio half (for the pump's accounting).
    #[must_use]
    pub fn audio(&self) -> Option<&AudioLimb> {
        self.audio.as_ref()
    }

    /// Tick the video scheduler once (the pump's per-quantum step).
    ///
    /// # Errors
    /// The present path's typed errors (recorded, not swallowed).
    pub fn tick_video(&mut self) -> Result<(), EngineError> {
        match self.video.as_mut() {
            Some(video) => video.tick_once(),
            None => Ok(()),
        }
    }

    /// Record a present-path failure (surfaced once by the pump).
    pub fn note_video_error(&mut self, error: EngineError) {
        if self.video_error.is_none() {
            self.video_error = Some(error);
        }
    }
}

/// The engine's sink seam: decoded frames enter the audio/video paths
/// here, on the poll thread.
pub struct CliFrameConsumer {
    state: Mutex<ConsumerState>,
    facts: Arc<MediaFacts>,
    clock: Arc<MediaClock>,
}

impl CliFrameConsumer {
    /// Build the consumer with its audio half pre-wired (the open path
    /// constructs the null session before the engine exists, because
    /// the session clock needs the counter). The video half is built
    /// lazily on the first decoded video frame (the surface extent
    /// comes from the frames themselves).
    #[must_use]
    pub fn new(
        audio: Option<AudioLimb>,
        facts: Arc<MediaFacts>,
        clock: Arc<MediaClock>,
    ) -> CliFrameConsumer {
        CliFrameConsumer {
            state: Mutex::new(ConsumerState {
                audio,
                video: None,
                video_error: None,
                frames_accepted: 0,
                frames_refused: 0,
            }),
            facts,
            clock,
        }
    }

    /// Lock the consumer state (the pump's access path). A poisoned
    /// lock means a poll-thread panic already happened — the state is
    /// recovered, not trusted-blindly (std's own recovery semantics).
    #[must_use]
    pub fn lock(&self) -> std::sync::MutexGuard<'_, ConsumerState> {
        match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// A present-path failure, once (surfaced by the pump).
    #[must_use]
    pub fn take_video_error(&self) -> Option<EngineError> {
        self.lock().video_error.take()
    }
}

impl FrameConsumer for CliFrameConsumer {
    fn accept(&self, frame: Frame, track: TrackId) -> Result<(), EngineError> {
        let mut state = self.lock();
        state.frames_accepted = state.frames_accepted.saturating_add(1);
        match frame {
            Frame::Audio(audio) => accept_audio(&mut state, &audio),
            Frame::Video(video) => accept_video(&mut state, &video, track, &self.facts, &self.clock),
            // Subtitle/data limbs are not wired in the first vertical.
            Frame::Subtitle(_) => Ok(()),
        }
    }
}

/// Stage + mix + write one audio frame into the null ring.
fn accept_audio(state: &mut ConsumerState, frame: &AudioFrame) -> Result<(), EngineError> {
    let Some(audio) = state.audio.as_mut() else {
        // The engine only wires limbs the wrapper's track list named;
        // an audio frame with no audio limb means the composition is
        // inconsistent — the honest typed refusal, never a drop.
        return Err(host_error(
            PipelineStage::Mix,
            "cli.audio_limb_absent",
            "an audio frame arrived with no audio limb wired",
        ));
    };
    audio.pipeline.process(0, frame)?;
    audio.pipeline.mix()?;
    if let Some(buffer) = audio.pipeline.take_output() {
        audio.sink.write_bytes(buffer.data.as_slice())?;
    }
    Ok(())
}

/// Convert (when needed) and schedule one video frame.
fn accept_video(
    state: &mut ConsumerState,
    frame: &VideoFrame,
    track: TrackId,
    facts: &Arc<MediaFacts>,
    clock: &Arc<MediaClock>,
) -> Result<(), EngineError> {
    if state.video.is_none() {
        let frame_ns = facts
            .fact_for(track)
            .and_then(|fact| fact.frame_ns)
            .unwrap_or(33_333_333);
        let cfg = match SchedulerCfg::for_frame_duration(media_types::Nanoseconds(frame_ns)) {
            Ok(cfg) => cfg,
            Err(err) => {
                return Err(host_error(
                    PipelineStage::Present,
                    "cli.scheduler_cfg",
                    err.to_string(),
                ));
            }
        };
        let extent = Extent2D {
            width: frame.size.0,
            height: frame.size.1,
        };
        let renderer = Renderer::new_headless(
            extent,
            SurfaceConfig {
                format: SurfaceFormat::Bgra8Srgb,
                present_mode: PresentMode::Fifo,
            },
        )
        .map_err(|err| err.to_engine_error())?;
        let queue = PresentationQueue::with_generation(QUEUE_CAPACITY, frame.generation);
        state.video = Some(VideoLimb {
            scheduler: PresentationScheduler::new(
                cfg,
                Arc::clone(clock) as Arc<dyn media_clock::PlaybackClock>,
            ),
            queue,
            renderer,
            presented: 0,
            refused: 0,
        });
    }
    let Some(video) = state.video.as_mut() else {
        return Ok(());
    };
    // Convert to the renderer's upload vocabulary when needed.
    let uploadable = match frame.format {
        media_types::PixelFormat::Rgba8 | media_types::PixelFormat::Bgra8 => frame.clone(),
        media_types::PixelFormat::I420 => convert_i420_to_rgba(frame)?,
        media_types::PixelFormat::Nv12 | media_types::PixelFormat::Yuyv422 => {
            return Err(host_error(
                PipelineStage::Present,
                "cli.pixel_format_unconverted",
                format!(
                    "pixel format {:?} needs the color-conversion card (first vertical: I420 + RGBA)",
                    frame.format
                ),
            ));
        }
    };
    match video.scheduler.push(uploadable) {
        PushOutcome::Scheduled | PushOutcome::Inferred { .. } => Ok(()),
        PushOutcome::Held | PushOutcome::MissingDropped { .. } => Ok(()),
        PushOutcome::WindowFull { .. } | PushOutcome::UnschedulablePts { .. } => {
            video.refused = video.refused.saturating_add(1);
            state.frames_refused = state.frames_refused.saturating_add(1);
            Ok(())
        }
        PushOutcome::Stale { .. } => Ok(()),
    }
}


// ---------------------------------------------------------------------------
// I420 → RGBA (BT.601 full range, the JFIF equations)
// ---------------------------------------------------------------------------

/// Convert one 4:2:0 I420 frame to RGBA8 using the JFIF conversion
/// (BT.601 full-range — the codec's own tagged colorimetry). The
/// chroma planes are box-upsampled (nearest co-located sample), the
/// same v0 convention the codec's non-planar outputs use.
///
/// `[PROPOSAL: the general color-conversion card owns every matrix /
/// range / upsampler combination; the CLI hosts exactly the first
/// vertical's tagged JFIF case, and refuses anything else honestly]`
fn convert_i420_to_rgba(frame: &VideoFrame) -> Result<VideoFrame, EngineError> {
    if frame.color.matrix != ColorMatrix::Bt601 || frame.color.range != ColorRange::Full {
        return Err(host_error(
            PipelineStage::Present,
            "cli.i420_color_unsupported",
            format!(
                "I420 with {:?}/{:?} needs the color-conversion card (first vertical: BT.601 full)",
                frame.color.matrix, frame.color.range
            ),
        ));
    }
    let (width, height) = (frame.size.0 as usize, frame.size.1 as usize);
    if width == 0 || height == 0 {
        return Err(EngineError::new(
            PipelineStage::Present,
            ErrorKind::Corrupt {
                detail: media_types::error::CorruptDetail::InvalidStructure { offset: None },
            },
            media_types::error::ErrorContext::new("cli.frame_zero_extent"),
        ));
    }
    let chroma_width = width.div_ceil(2);
    let chroma_height = height.div_ceil(2);
    let y_len = width * height;
    let c_len = chroma_width * chroma_height;
    let bytes = match &frame.data {
        FrameStorage::Cpu(bytes) => bytes,
        FrameStorage::OpaqueHandle(_) => {
            return Err(host_error(
                PipelineStage::Present,
                "cli.opaque_handle_unsupported",
                "hardware frames need the wgpu backend (2.D3)",
            ));
        }
    };
    let data = bytes.as_slice();
    if data.len() < y_len + 2 * c_len {
        return Err(EngineError::new(
            PipelineStage::Present,
            ErrorKind::Corrupt {
                detail: media_types::error::CorruptDetail::Truncated {
                    expected: (y_len + 2 * c_len) as u64,
                    found: data.len() as u64,
                },
            },
            media_types::error::ErrorContext::new("cli.i420_truncated"),
        ));
    }
    let (y_plane, u_plane, v_plane) = (
        &data[..y_len],
        &data[y_len..y_len + c_len],
        &data[y_len + c_len..],
    );
    let mut rgba = Vec::with_capacity(width * height * 4);
    for row in 0..height {
        let chroma_row = row / 2;
        for column in 0..width {
            let y = i32::from(y_plane[row * width + column]);
            let chroma_column = column / 2;
            let cb = i32::from(u_plane[chroma_row * chroma_width + chroma_column]) - 128;
            let cr = i32::from(v_plane[chroma_row * chroma_width + chroma_column]) - 128;
            // JFIF (ITU-R BT.601 full range):
            //   R = Y + 1.402 Cr; B = Y + 1.772 Cb;
            //   G = Y − 0.344136 Cb − 0.714136 Cr   (Q8 fixed point)
            let r = clamp_u8(y + ((359 * cr + 128) >> 8));
            let g = clamp_u8(y - ((88 * cb + 183 * cr + 128) >> 8));
            let b = clamp_u8(y + ((453 * cb + 128) >> 8));
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    Ok(VideoFrame {
        format: media_types::PixelFormat::Rgba8,
        size: frame.size,
        data: FrameStorage::Cpu(media_types::packet::SharedBytes::from_vec(rgba)),
        pts: frame.pts,
        color: frame.color,
        interlaced: frame.interlaced,
        generation: frame.generation,
    })
}

fn clamp_u8(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

// ---------------------------------------------------------------------------
// The audio session wiring (open-path construction)
// ---------------------------------------------------------------------------

/// The null audio session halves plus the processing pipeline: what
/// the open path builds when the media has an audio track.
pub struct AudioSession {
    /// The consumer's audio half (pipeline + sink).
    pub limb: AudioLimb,
    /// The device half (the pump drives it).
    pub device: NullDevice,
    /// The session clock built over the sink's counter.
    pub clock: Arc<MediaClock>,
    /// The channel count (render buffer sizing).
    pub channels: u32,
}

/// Open the null audio session + the audio-master clock for a media
/// whose PCM facts are known. `anchor_ns` is the position the session
/// starts at (0, or the seek target).
///
/// # Errors
/// The null backend's typed errors (unsupported format shape) and the
/// clock's construction errors, both already stage-tagged.
pub fn open_audio_session(
    channels: u32,
    rate: u32,
    anchor_ns: i64,
) -> Result<AudioSession, EngineError> {
    let format = AudioFormat {
        sample: SampleFormat::F32,
        channels,
        layout: AudioLayout::Interleaved,
        rate,
    };
    let NullStream { sink, device } =
        NullBackend::open_stream(&format, RING_FRAMES, NullConfig { capture: false })?;
    let pipeline = AudioPipeline::new(PipelineSpec {
        rate,
        channels,
        tracks: 1,
    })?;
    let clock = Arc::new(MediaClock::audio_master(
        sink.clock_source(),
        Timestamp::new(anchor_ns, media_clock::NANO_BASE),
        Rate::ONE,
    )?);
    Ok(AudioSession {
        limb: AudioLimb { pipeline, sink },
        device,
        clock,
        channels,
    })
}

/// The no-audio fallback clock: system monotonic (wall time). The
/// audio-bearing path is the deterministic one; video-only media runs
/// on the real clock `[PROPOSAL: an injectable virtual monotonic
/// source for tests joins with the host clock seam 3.B2]`.
///
/// # Errors
/// The clock's construction errors (already stage-tagged).
pub fn wall_clock(anchor_ns: i64) -> Result<Arc<MediaClock>, EngineError> {
    Ok(Arc::new(MediaClock::system_monotonic(
        Arc::new(media_clock::InstantSource::new()),
        Timestamp::new(anchor_ns, media_clock::NANO_BASE),
        Rate::ONE,
    )?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::frame::ColorDescription;
    use media_types::packet::SharedBytes;
    use media_types::time::Stamp;
    use media_types::{GenerationId, Packet, PixelFormat};

    fn i420_frame(width: usize, height: usize, y: u8, cb: u8, cr: u8) -> VideoFrame {
        let chroma_w = width.div_ceil(2);
        let chroma_h = height.div_ceil(2);
        let mut data = vec![y; width * height];
        data.extend(vec![cb; chroma_w * chroma_h]);
        data.extend(vec![cr; chroma_w * chroma_h]);
        VideoFrame {
            format: PixelFormat::I420,
            size: (width as u32, height as u32),
            data: FrameStorage::Cpu(SharedBytes::from_vec(data)),
            pts: Stamp::Missing,
            color: ColorDescription {
                primaries: media_types::frame::ColorPrimaries::Bt601,
                transfer: media_types::frame::TransferCharacteristics::Srgb,
                matrix: ColorMatrix::Bt601,
                range: ColorRange::Full,
            },
            interlaced: false,
            generation: GenerationId::FIRST,
        }
    }

    fn pixel(frame: &VideoFrame, x: usize, y: usize) -> [u8; 4] {
        let bytes = match &frame.data {
            FrameStorage::Cpu(bytes) => bytes.as_slice(),
            FrameStorage::OpaqueHandle(_) => [].as_slice(),
        };
        let base = (y * frame.size.0 as usize + x) * 4;
        [bytes[base], bytes[base + 1], bytes[base + 2], bytes[base + 3]]
    }

    /// JFIF vectors: neutral chroma passes luma through; the extreme
    /// chroma rows match the standard equations to ±1 (Q8 rounding).
    #[test]
    fn i420_conversion_matches_jfif_vectors() {
        // Gray: Y=128, neutral chroma → (128,128,255).
        let gray = match convert_i420_to_rgba(&i420_frame(8, 8, 128, 128, 128)) {
            Ok(frame) => frame,
            Err(err) => panic!("{err}"),
        };
        assert_eq!(gray.format, PixelFormat::Rgba8);
        assert_eq!(pixel(&gray, 3, 5), [128, 128, 128, 255]);
        // Black / white.
        let black = match convert_i420_to_rgba(&i420_frame(8, 8, 0, 128, 128)) {
            Ok(frame) => frame,
            Err(err) => panic!("{err}"),
        };
        assert_eq!(pixel(&black, 0, 0), [0, 0, 0, 255]);
        let white = match convert_i420_to_rgba(&i420_frame(8, 8, 255, 128, 128)) {
            Ok(frame) => frame,
            Err(err) => panic!("{err}"),
        };
        assert_eq!(pixel(&white, 7, 7), [255, 255, 255, 255]);
        // Red: Cr=255 (Cr−128 = 127), Cb neutral, Y=50 → the JFIF
        // equations in-range: R = 50 + 1.402·127 ≈ 228,
        // G = 50 − 0.714·127 − 0 ≈ −41 → clamps to 0, B = 50.
        let red = match convert_i420_to_rgba(&i420_frame(8, 8, 50, 128, 255)) {
            Ok(frame) => frame,
            Err(err) => panic!("{err}"),
        };
        let [r, g, b, a] = pixel(&red, 2, 2);
        assert_eq!(a, 255);
        assert!((i32::from(r) - 228).abs() <= 1, "{r}");
        assert_eq!(g, 0, "green clamps below zero");
        assert_eq!(b, 50, "neutral Cb keeps blue at luma");
        // Blue: Cb=255 (Cb−128 = 127), Cr neutral, Y=128 →
        // B = 128 + 1.772·127 ≈ 353 → clamps to 255; the JFIF R has
        // NO Cb term, so R stays at luma.
        let blue = match convert_i420_to_rgba(&i420_frame(8, 8, 128, 255, 128)) {
            Ok(frame) => frame,
            Err(err) => panic!("{err}"),
        };
        let [r, _g, b, _a] = pixel(&blue, 5, 1);
        assert_eq!(b, 255, "Y=128 + 225 clamps to 255");
        assert_eq!(r, 128, "neutral Cr keeps red at luma (R has no Cb term)");
    }

    /// Odd dimensions use ceil chroma geometry; the buffer length is
    /// validated (truncated planes are typed corruption, never OOB).
    #[test]
    fn i420_conversion_validates_and_handles_odd_sizes() {
        let frame = i420_frame(7, 5, 200, 128, 128);
        let converted = match convert_i420_to_rgba(&frame) {
            Ok(frame) => frame,
            Err(err) => panic!("{err}"),
        };
        assert_eq!(converted.size, (7, 5));
        assert_eq!(pixel(&converted, 6, 4), [200, 200, 200, 255]);
        // Truncated data: typed corrupt.
        let mut truncated = i420_frame(8, 8, 128, 128, 128);
        let short = match &truncated.data {
            FrameStorage::Cpu(bytes) => bytes.as_slice()[..10].to_vec(),
            FrameStorage::OpaqueHandle(_) => Vec::new(),
        };
        truncated.data = FrameStorage::Cpu(SharedBytes::from_vec(short));
        match convert_i420_to_rgba(&truncated) {
            Err(err) => assert_eq!(
                err.kind,
                ErrorKind::Corrupt {
                    detail: media_types::error::CorruptDetail::Truncated { expected: 96, found: 10 }
                }
            ),
            Ok(_) => panic!("truncated planes must be refused"),
        }
    }

    /// A limited-range or BT.709 tag is refused honestly (the
    /// conversion card owns those).
    #[test]
    fn i420_conversion_refuses_non_jfif_tags() {
        let mut frame = i420_frame(8, 8, 128, 128, 128);
        frame.color.range = ColorRange::Limited;
        match convert_i420_to_rgba(&frame) {
            Err(err) => assert_eq!(err.context.code, "cli.i420_color_unsupported"),
            Ok(_) => panic!("limited range must be refused"),
        }
    }

    /// A media-types packet field sanity check for the imports this
    /// module's public surface touches (compile-level pin).
    #[test]
    fn shared_bytes_roundtrip() {
        let bytes = SharedBytes::from_vec(vec![1, 2, 3]);
        assert_eq!(bytes.as_slice(), &[1, 2, 3]);
        let _packet_shape = Packet {
            track: TrackId::new(0),
            pts: Stamp::Missing,
            dts: Stamp::Missing,
            duration: None,
            data: bytes,
            keyframe: true,
            discontinuity: false,
            generation: GenerationId::FIRST,
        };
    }
}
