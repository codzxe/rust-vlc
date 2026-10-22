//! The composed audio path: decode → convert → volume + channel map →
//! straight mix → device format (card 2.C3; doc 03 §4.8 a/c/d — the
//! v0 instantiation of the `AudioPipeline` sketch with insertion
//! point B empty).
//!
//! One pipeline is built per session at format negotiation
//! ([`PipelineSpec`]: the mixer rate and channel shape, and the track
//! count). The pump then runs one **quantum** at a time:
//!
//! 1. [`process`](AudioPipeline::process) each active track's decoded
//!    [`AudioFrame`] — stage A converts it to f32 planes and the
//!    track's [`ChannelMap`] routes them into the mixer shape;
//! 2. [`mix`](AudioPipeline::mix) once — per-track [`Volume`],
//!    straight sum, stage D interleave;
//! 3. [`take_output`](AudioPipeline::take_output) — the
//!    [`AudioBuffer`] headed for the [`AudioSink`](crate::AudioSink).
//!
//! All working buffers are owned by the pipeline and reused (doc 03
//! §4.8 c — no per-frame allocation after negotiation; the first
//! quantum warms the capacities). Frame counts are conserved end to
//! end; a track whose rate differs from the negotiated mixer rate is
//! a typed error (the micro-resampler is a later card, doc 03 §4.7 e).

use media_clock::DriftCorrection;
use media_types::EngineError;
use media_types::frame::{AudioFormat, AudioFrame, AudioLayout, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::time::Stamp;

use crate::buffer::AudioBuffer;
use crate::channel_map::ChannelMap;
use crate::convert::{frame_to_f32_planes, planes_to_f32_interleaved_into};
use crate::error;
use crate::governor::{DriftGovernor, DriftStats};
use crate::mix::Mixer;
use crate::resample::MicroResampler;
use crate::volume::Volume;

/// The output-slot rotation depth: how many pooled quantum buffers the
/// pipeline keeps (D-1 object pool). Four covers the pump's own
/// pending quantum plus two still referenced downstream with slack —
/// a sink that hands buffers off asynchronously never starves the
/// pool into the allocate path. Bounded memory: at most
/// `OUT_SLOTS · max-quantum-bytes`.
const OUT_SLOTS: usize = 4;

/// The negotiated shape of one session's audio path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipelineSpec {
    /// The mixer rate — the first track's rate in practice (doc 03
    /// §4.8 e); every later track must match in v0 (rate conversion
    /// is the micro-resampler card).
    pub rate: u32,
    /// The mixer channel count. v0 pins the device shape to it (the
    /// device-adaptation channel conversion joins with real backends,
    /// ADR-005 era).
    pub channels: u32,
    /// How many tracks feed the mixer.
    pub tracks: usize,
}

/// The composed per-session audio path (see the module docs for the
/// quantum flow).
#[derive(Debug, Clone)]
pub struct AudioPipeline {
    spec: PipelineSpec,
    mixer: Mixer,
    maps: Vec<ChannelMap>,
    /// Stage-A scratch: the just-decoded f32 planes (reused).
    scratch: Vec<Vec<f32>>,
    /// Staged (converted + channel-mapped) chunk per track.
    stage: Vec<Vec<Vec<f32>>>,
    /// Staged frame count per track (`0` = nothing staged).
    staged: Vec<usize>,
    /// The quantum's pts (lowest track index that staged).
    timing: Option<Stamp>,
    /// Mixer output planes (reused).
    mixed: Vec<Vec<f32>>,
    /// The bounded drift correction in force (card 3.B1): `ZERO`
    /// keeps the sample-count-conserving bypass below; non-zero
    /// routes the mixed planes through the micro-resampler between
    /// `mix` and the stage-D interleave (doc 03 §4.8 e-f).
    drift: DriftCorrection,
    /// The micro-resampler (consumer #1 of `DriftCorrection` —
    /// audit M12's missing half; dormant at `ZERO`).
    resampler: MicroResampler,
    /// The drift governor (consumer #2 — the `for_divergence`
    /// derivation site), when the wiring arms it. `None` = the
    /// pipeline applies corrections from `set_drift` only (direct
    /// control — what a host with its own measurement loop wants).
    governor: Option<DriftGovernor>,
    /// Stage-D output rotation: pooled [`SharedBytes`] slots the
    /// quantum is interleaved into in place (`get_mut` when the pool
    /// is the sole owner — no per-quantum Arc allocation after
    /// warm-up; the old single `out_bytes` Vec was `mem::take`n into a
    /// fresh `Arc<[u8]>` copy every quantum, the C6b violation).
    out_slots: Vec<SharedBytes>,
    /// Round-robin cursor into `out_slots`.
    out_next: usize,
    /// The mixed quantum awaiting `take_output`.
    pending: Option<AudioBuffer>,
}

impl AudioPipeline {
    /// Build the pipeline for a negotiated spec. Tracks start at
    /// [`Volume::UNITY`] with an identity channel map (a frame whose
    /// channels equal `spec.channels` passes the map unchanged).
    ///
    /// # Errors
    /// [`Unsupported`](media_types::ErrorKind::Unsupported) with code
    /// `audio.pipeline_shape_invalid` when the rate, channel count or
    /// track count is zero.
    pub fn new(spec: PipelineSpec) -> Result<AudioPipeline, EngineError> {
        if spec.rate == 0 || spec.channels == 0 || spec.tracks == 0 {
            return Err(error::unsupported_format("audio.pipeline_shape_invalid"));
        }
        let maps = (0..spec.tracks)
            .map(|_| ChannelMap::identity(spec.channels))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(AudioPipeline {
            spec,
            mixer: Mixer::new(spec.tracks)?,
            maps,
            scratch: Vec::new(),
            stage: vec![Vec::new(); spec.tracks],
            staged: vec![0; spec.tracks],
            timing: None,
            mixed: Vec::new(),
            drift: DriftCorrection::ZERO,
            resampler: MicroResampler::new(spec.channels),
            governor: None,
            out_slots: Vec::new(),
            out_next: 0,
            pending: None,
        })
    }

    /// Arm the drift governor toward `target_ns` of ring fill (card
    /// 3.B1): the wiring's one-call loop closure — feed it the sink's
    /// measured latency per quantum and the corrections land in the
    /// resampler automatically. Without a governor the pipeline still
    /// accepts direct [`AudioPipeline::set_drift`] control.
    #[must_use]
    pub fn with_drift_governor(mut self, target_ns: u64) -> AudioPipeline {
        self.arm_drift_governor(target_ns);
        self
    }

    /// The late-arming half of [`AudioPipeline::with_drift_governor`]
    /// (same contract): for pipelines built by a composition that
    /// decides the pacing regime after construction — the composition
    /// root opens the session unarmed (its decode-ahead regime makes
    /// ring fill a buffering metric, not a drift signal) and arms the
    /// governor where production is paced at the consumption rate.
    pub fn arm_drift_governor(&mut self, target_ns: u64) {
        self.governor = Some(DriftGovernor::new(target_ns));
    }

    /// Apply a drift correction directly (the micro-resampler's
    /// ratio; consumer #1 of `DriftCorrection` — audit M12).
    /// `ZERO` restores the sample-count-conserving bypass exactly.
    pub fn set_drift(&mut self, correction: DriftCorrection) {
        self.drift = correction;
        self.resampler.set_correction(correction);
    }

    /// The correction in force.
    #[must_use]
    pub fn drift(&self) -> DriftCorrection {
        self.drift
    }

    /// One per-quantum drift observation through the armed governor
    /// (see [`AudioPipeline::with_drift_governor`]): feeds the fill
    /// sample, re-evaluates at the ≤1 Hz window, applies the
    /// step-limited correction to the resampler, and answers it.
    /// A pipeline without a governor ignores the sample (`ZERO`).
    ///
    /// * `now_ns` — the injected device-time stamp (the wiring
    ///   derives it from the sample counter; a frozen device freezes
    ///   the window — zero updates, deterministically).
    /// * `fill_ns` — the sink's measured latency, `None` when
    ///   unmeasurable (parks the governor at zero — honest
    ///   degradation, never a fabricated estimate).
    pub fn observe_drift(&mut self, now_ns: u64, fill_ns: Option<u64>) -> DriftCorrection {
        let Some(governor) = self.governor.as_mut() else {
            return DriftCorrection::ZERO;
        };
        let correction = governor.observe(now_ns, fill_ns);
        self.set_drift(correction);
        correction
    }

    /// A new timing regime (generation / anchor-epoch bump): reset
    /// the governor's measurement and the resampler's stream phase.
    /// The correction drops to zero with the measurement (the S7
    /// transient is one step-limiter increment at most).
    pub fn reset_drift(&mut self) {
        if let Some(governor) = self.governor.as_mut() {
            governor.reset();
        }
        self.set_drift(DriftCorrection::ZERO);
        self.resampler.reset();
    }

    /// The governor's polled observability (counters only — the
    /// no-event-storm policy; all-zero when no governor is armed).
    #[must_use]
    pub fn drift_stats(&self) -> DriftStats {
        self.governor
            .as_ref()
            .map_or(DriftStats::default(), DriftGovernor::stats)
    }

    /// The negotiated spec.
    pub fn spec(&self) -> PipelineSpec {
        self.spec
    }

    /// The device format this pipeline emits: interleaved f32 at the
    /// mixer rate and channel shape (doc 03 §4.9's callback payload).
    pub fn device_format(&self) -> AudioFormat {
        AudioFormat {
            sample: SampleFormat::F32,
            channels: self.spec.channels,
            layout: AudioLayout::Interleaved,
            rate: self.spec.rate,
        }
    }

    /// Set one track's gain.
    ///
    /// # Errors
    /// [`Unsupported`](media_types::ErrorKind::Unsupported) with code
    /// `audio.track_index_out_of_range` for an unknown track (the
    /// gain's own validation errors propagate from [`Volume`]).
    pub fn set_volume(&mut self, track: usize, volume: Volume) -> Result<(), EngineError> {
        self.mixer.set_volume(track, volume)
    }

    /// One track's gain.
    ///
    /// # Errors
    /// [`Unsupported`](media_types::ErrorKind::Unsupported) with code
    /// `audio.track_index_out_of_range`.
    pub fn volume(&self, track: usize) -> Result<Volume, EngineError> {
        self.mixer.volume(track)
    }

    /// Set one track's channel map (decoded shape → mixer shape).
    ///
    /// # Errors
    /// [`Unsupported`](media_types::ErrorKind::Unsupported) with code
    /// `audio.track_index_out_of_range` (unknown track) or
    /// `audio.channel_count_mismatch` (the map's output shape is not
    /// the negotiated mixer shape).
    pub fn set_channel_map(&mut self, track: usize, map: ChannelMap) -> Result<(), EngineError> {
        if map.out_channels() != self.spec.channels {
            return Err(error::channel_range("audio.channel_count_mismatch"));
        }
        let slot = self
            .maps
            .get_mut(track)
            .ok_or_else(|| error::track_range("audio.track_index_out_of_range"))?;
        *slot = map;
        Ok(())
    }

    /// Stage one decoded frame for its track: stage A converts to f32
    /// planes, the track's channel map routes them into the mixer
    /// shape (per-track volume is applied at mix time). The slot holds
    /// the latest chunk — the pump stages each active track once per
    /// quantum.
    ///
    /// # Errors
    /// [`Unsupported`](media_types::ErrorKind::Unsupported) with code
    /// `audio.track_index_out_of_range`, `audio.rate_mismatch` (the
    /// frame's rate is not the negotiated mixer rate — rate conversion
    /// is the micro-resampler card, doc 03 §4.7 e) or
    /// `audio.channel_count_mismatch` (the frame's channel count is
    /// not the track map's input shape); [`Corrupt`] with
    /// `audio.partial_frame` from the conversion (the payload is not
    /// whole frames).
    pub fn process(&mut self, track: usize, frame: &AudioFrame) -> Result<(), EngineError> {
        if track >= self.spec.tracks {
            return Err(error::track_range("audio.track_index_out_of_range"));
        }
        if frame.format.rate != self.spec.rate {
            return Err(error::unsupported_format("audio.rate_mismatch"));
        }
        let map = &self.maps[track];
        if frame.format.channels != map.in_channels() {
            return Err(error::channel_range("audio.channel_count_mismatch"));
        }
        // Stage A into the scratch planes, then route into the staged
        // slot (two disjoint buffer groups, both pipeline-owned).
        let mut planes = std::mem::take(&mut self.scratch);
        let frames = frame_to_f32_planes(frame, &mut planes)?;
        let mut staged = std::mem::take(&mut self.stage[track]);
        let staged_result = map.apply(&planes, &mut staged);
        self.scratch = planes;
        staged_result?;
        self.stage[track] = staged;
        self.staged[track] = frames;
        if self.timing.is_none() {
            self.timing = Some(frame.pts);
        }
        Ok(())
    }

    /// Mix the staged quantum (per-track volume, straight sum, stage D
    /// interleave). Staging is consumed; the result waits at
    /// [`take_output`](Self::take_output). Returns whether a quantum
    /// was produced (`false` when every track staged nothing).
    ///
    /// Zero allocation after warm-up: the mix reads the staged chunks
    /// where they live (no `refs` Vec), and stage D writes into a
    /// pooled output slot through [`SharedBytes::get_mut`] — the Arc
    /// allocation happens only on geometry changes or when every
    /// pooled slot is still referenced downstream (both are cold
    /// paths; the rotation is bounded at [`OUT_SLOTS`]).
    ///
    /// # Errors
    /// [`Corrupt`](media_types::ErrorKind::Corrupt) with code
    /// `audio.chunk_length_mismatch` when staged tracks disagree on
    /// frame count (v0 mixes whole aligned quanta; alignment is the
    /// session's job, doc 03 §4.6), or the mixer's geometry errors.
    pub fn mix(&mut self) -> Result<bool, EngineError> {
        let frames = self.staged[0];
        for staged in &self.staged {
            if *staged != frames {
                return Err(error::chunk_alignment());
            }
        }
        if frames == 0 {
            self.timing = None;
            return Ok(false);
        }
        // Field-disjoint borrows: the mixer reads `stage` and its
        // volumes, `mixed` is written — no refs Vec is built.
        {
            let mut mixed = std::mem::take(&mut self.mixed);
            let mix_result = self.mixer.mix(&self.stage, &mut mixed);
            self.mixed = mixed;
            mix_result?;
        }
        // The 3.B1 drift seam — between `mix` and stage D (doc 03
        // §4.7 e / §4.8 e-f): a non-zero correction resamples the
        // mixed planes through the micro-resampler into its own
        // reused plane set; `ZERO` is a bit-exact bypass by ownership
        // (no interpolation, no copy — the mixed planes feed stage D
        // directly, the pre-3.B1 path byte for byte).
        let (planes, out_frames) = if self.drift.ppm() == 0 {
            (self.mixed.as_slice(), frames)
        } else {
            // Field-disjoint borrows: the resampler mutates while the
            // mixed planes are read (no `mem::take`, no Default).
            let out_frames = self.resampler.process(&self.mixed, frames);
            (self.resampler.planes(), out_frames)
        };
        // Stage D: interleave into a pooled output slot. The slot
        // rotation is taken out of `self` so the `planes` borrow
        // (mixed or the resampler's set — field-disjoint from the
        // pool) survives the call.
        let data = {
            let mut slots = std::mem::take(&mut self.out_slots);
            let filled = pooled_slot(
                &mut slots,
                &mut self.out_next,
                planes,
                out_frames * self.spec.channels as usize * 4,
            );
            self.out_slots = slots;
            filled?
        };
        let pts = self.timing.take().unwrap_or(Stamp::Missing);
        self.pending = Some(AudioBuffer {
            format: self.device_format(),
            data,
            pts,
        });
        for staged in self.staged.iter_mut() {
            *staged = 0;
        }
        // The staged chunks themselves are deliberately NOT cleared:
        // `Vec::clear` on `Vec<Vec<f32>>` would drop the inner plane
        // allocations (the same false-reuse bug class as C6a) and the
        // next quantum would reallocate every plane. `staged == 0` is
        // the emptiness marker; a mix only ever reads a track's chunk
        // after that track staged this quantum (the alignment check
        // refuses anything else), and `process` → `ChannelMap::apply`
        // clears and refills the inner planes in place.
        Ok(true)
    }

    /// Take the mixed quantum (once per [`mix`](Self::mix) that
    /// returned `true`).
    pub fn take_output(&mut self) -> Option<AudioBuffer> {
        self.pending.take()
    }
}

/// Fill the next pooled output slot with `planes`' interleaved bytes
/// and hand out a clone (refcount 2: pool + the pending buffer; the
/// clone the sink saw returns to refcount 1 when its buffer drops,
/// making the slot refillable in place). Warm path: an in-rotation
/// slot of exactly `byte_len` whose `get_mut` succeeds (the pool is
/// the sole owner). Cold path (first quanta, a length change — a
/// correction shifts the quantum by ≤ +0.1 %, or an all-slots-busy
/// rotation): allocate a fresh slot and enter it into the
/// round-robin (bounded at [`OUT_SLOTS`], then evicting).
fn pooled_slot(
    slots: &mut Vec<SharedBytes>,
    out_next: &mut usize,
    planes: &[Vec<f32>],
    byte_len: usize,
) -> Result<SharedBytes, EngineError> {
    let count = slots.len();
    for i in 0..count {
        let idx = (*out_next + i) % count;
        if slots[idx].len() != byte_len {
            continue;
        }
        if let Some(bytes) = slots[idx].get_mut() {
            planes_to_f32_interleaved_into(planes, bytes)?;
            *out_next = (idx + 1) % count;
            return Ok(slots[idx].clone());
        }
        // The slot still has downstream owners — try the next one.
    }
    // Cold path: allocate, fill in place, then enter the rotation.
    let mut fresh = SharedBytes::from_vec(vec![0; byte_len]);
    let filled = match fresh.get_mut() {
        Some(bytes) => planes_to_f32_interleaved_into(planes, bytes),
        None => unreachable!("fresh SharedBytes has refcount 1"),
    };
    if count < OUT_SLOTS {
        slots.push(fresh.clone());
        *out_next = (count + 1) % OUT_SLOTS;
    } else {
        let evict = *out_next;
        slots[evict] = fresh.clone();
        *out_next = (evict + 1) % OUT_SLOTS;
    }
    filled?;
    Ok(fresh)
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::ids::GenerationId;
    use media_types::time::Timestamp;
    use testkit::Must;

    const RATE: u32 = 44_100;

    fn stereo_s16_frame(pairs: &[(i16, i16)], pts_ticks: i64) -> AudioFrame {
        let mut bytes = Vec::new();
        for (l, r) in pairs {
            bytes.extend_from_slice(&l.to_le_bytes());
            bytes.extend_from_slice(&r.to_le_bytes());
        }
        AudioFrame {
            format: AudioFormat {
                sample: SampleFormat::S16,
                channels: 2,
                layout: AudioLayout::Interleaved,
                rate: RATE,
            },
            samples: SharedBytes::from_vec(bytes),
            pts: Stamp::Present(Timestamp::from_parts(pts_ticks, 1, RATE).must()),
            duration: None,
            generation: GenerationId::FIRST,
        }
    }

    fn read_f32_pairs(buffer: &AudioBuffer) -> Vec<(f32, f32)> {
        buffer
            .data
            .as_slice()
            .as_chunks::<8>()
            .0
            .iter()
            .map(|c| {
                (
                    f32::from_le_bytes([c[0], c[1], c[2], c[3]]),
                    f32::from_le_bytes([c[4], c[5], c[6], c[7]]),
                )
            })
            .collect()
    }

    #[test]
    fn unity_single_track_is_bit_exact_end_to_end() {
        // The identity the sink-buffer hash AC relies on: s16 in,
        // f32 out, exactly v / 32768.
        let mut pipeline = AudioPipeline::new(PipelineSpec {
            rate: RATE,
            channels: 2,
            tracks: 1,
        })
        .must();
        let frame = stereo_s16_frame(&[(0, 0), (16384, -16384), (32767, -32768)], 0);
        pipeline.process(0, &frame).must();
        assert!(pipeline.mix().must());
        let buffer = pipeline.take_output().must();
        assert_eq!(buffer.format, pipeline.device_format());
        assert_eq!(buffer.pts, frame.pts);
        assert_eq!(
            read_f32_pairs(&buffer),
            vec![
                (0.0, 0.0),
                (16384.0 / 32_768.0, -16384.0 / 32_768.0),
                (32_767.0 / 32_768.0, -1.0),
            ]
        );
        // Taken once; nothing pending after.
        assert!(pipeline.take_output().is_none());
        // An empty quantum mixes to false.
        assert!(!pipeline.mix().must());
    }

    #[test]
    fn volume_halves_the_output() {
        let mut pipeline = AudioPipeline::new(PipelineSpec {
            rate: RATE,
            channels: 2,
            tracks: 1,
        })
        .must();
        pipeline
            .set_volume(0, Volume::from_linear(0.5).must())
            .must();
        let frame = stereo_s16_frame(&[(16384, -16384)], 5);
        pipeline.process(0, &frame).must();
        pipeline.mix().must();
        let buffer = pipeline.take_output().must();
        assert_eq!(
            read_f32_pairs(&buffer),
            vec![(8192.0 / 32_768.0, -8192.0 / 32_768.0)]
        );
    }

    #[test]
    fn channel_swap_routes_the_pair() {
        let mut pipeline = AudioPipeline::new(PipelineSpec {
            rate: RATE,
            channels: 2,
            tracks: 1,
        })
        .must();
        pipeline
            .set_channel_map(0, ChannelMap::swap_stereo())
            .must();
        let frame = stereo_s16_frame(&[(16384, 8192)], 0);
        pipeline.process(0, &frame).must();
        pipeline.mix().must();
        let buffer = pipeline.take_output().must();
        assert_eq!(
            read_f32_pairs(&buffer),
            vec![(8192.0 / 32_768.0, 16384.0 / 32_768.0)]
        );
    }

    #[test]
    fn mono_track_maps_into_a_stereo_pipeline() {
        let mut pipeline = AudioPipeline::new(PipelineSpec {
            rate: RATE,
            channels: 2,
            tracks: 1,
        })
        .must();
        pipeline
            .set_channel_map(0, ChannelMap::mono_to_stereo())
            .must();
        let mut bytes = Vec::new();
        for v in [16384i16, -16384] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let frame = AudioFrame {
            format: AudioFormat {
                sample: SampleFormat::S16,
                channels: 1,
                layout: AudioLayout::Interleaved,
                rate: RATE,
            },
            samples: SharedBytes::from_vec(bytes),
            pts: Stamp::Missing,
            duration: None,
            generation: GenerationId::FIRST,
        };
        pipeline.process(0, &frame).must();
        pipeline.mix().must();
        let buffer = pipeline.take_output().must();
        assert_eq!(
            read_f32_pairs(&buffer),
            vec![
                (16384.0 / 32_768.0, 16384.0 / 32_768.0),
                (-16384.0 / 32_768.0, -16384.0 / 32_768.0),
            ]
        );
    }

    #[test]
    fn two_tracks_mix_straight() {
        let mut pipeline = AudioPipeline::new(PipelineSpec {
            rate: RATE,
            channels: 2,
            tracks: 2,
        })
        .must();
        let a = stereo_s16_frame(&[(8192, 8192)], 0);
        let b = stereo_s16_frame(&[(4096, -4096)], 0);
        pipeline.process(0, &a).must();
        pipeline.process(1, &b).must();
        pipeline.mix().must();
        let buffer = pipeline.take_output().must();
        assert_eq!(
            read_f32_pairs(&buffer),
            vec![(12_288.0 / 32_768.0, 4_096.0 / 32_768.0)]
        );
        // The pts is the first staging track's.
        assert_eq!(buffer.pts, a.pts);
    }

    #[test]
    fn shape_and_geometry_errors_are_typed() {
        let err = AudioPipeline::new(PipelineSpec {
            rate: 0,
            channels: 2,
            tracks: 1,
        })
        .err()
        .must();
        assert_eq!(err.context.code, "audio.pipeline_shape_invalid");
        let err = AudioPipeline::new(PipelineSpec {
            rate: RATE,
            channels: 2,
            tracks: 0,
        })
        .err()
        .must();
        assert_eq!(err.context.code, "audio.pipeline_shape_invalid");

        let mut pipeline = AudioPipeline::new(PipelineSpec {
            rate: RATE,
            channels: 2,
            tracks: 1,
        })
        .must();
        // Rate mismatch (48 kHz frame into a 44.1 kHz pipeline).
        let wrong_rate = AudioFrame {
            format: AudioFormat {
                sample: SampleFormat::S16,
                channels: 2,
                layout: AudioLayout::Interleaved,
                rate: 48_000,
            },
            samples: SharedBytes::from_vec(vec![0; 4]),
            pts: Stamp::Missing,
            duration: None,
            generation: GenerationId::FIRST,
        };
        let err = pipeline.process(0, &wrong_rate).err().must();
        assert_eq!(err.context.code, "audio.rate_mismatch");
        // Channel mismatch (mono frame, identity stereo map).
        let mono = AudioFrame {
            format: AudioFormat {
                sample: SampleFormat::S16,
                channels: 1,
                layout: AudioLayout::Interleaved,
                rate: RATE,
            },
            samples: SharedBytes::from_vec(vec![0; 2]),
            pts: Stamp::Missing,
            duration: None,
            generation: GenerationId::FIRST,
        };
        let err = pipeline.process(0, &mono).err().must();
        assert_eq!(err.context.code, "audio.channel_count_mismatch");
        // Track index out of range.
        let frame = stereo_s16_frame(&[(0, 0)], 0);
        let err = pipeline.process(3, &frame).err().must();
        assert_eq!(err.context.code, "audio.track_index_out_of_range");
        let err = pipeline.set_volume(3, Volume::UNITY).err().must();
        assert_eq!(err.context.code, "audio.track_index_out_of_range");
        // A map with the wrong output shape is rejected at set time.
        let err = pipeline
            .set_channel_map(0, ChannelMap::stereo_to_mono())
            .err()
            .must();
        assert_eq!(err.context.code, "audio.channel_count_mismatch");
    }

    #[test]
    fn misaligned_quanta_are_typed_errors() {
        let mut pipeline = AudioPipeline::new(PipelineSpec {
            rate: RATE,
            channels: 2,
            tracks: 2,
        })
        .must();
        pipeline
            .process(0, &stereo_s16_frame(&[(1, 1), (2, 2)], 0))
            .must();
        pipeline.process(1, &stereo_s16_frame(&[(3, 3)], 0)).must();
        let err = pipeline.mix().err().must();
        assert_eq!(err.context.code, "audio.chunk_length_mismatch");
    }

    #[test]
    fn frame_counts_are_conserved_through_the_quantum() {
        // The clock-correctness invariant (doc 03 §4.8 e): frames in
        // == frames out, byte-exact accounting.
        let mut pipeline = AudioPipeline::new(PipelineSpec {
            rate: RATE,
            channels: 2,
            tracks: 1,
        })
        .must();
        let pairs: Vec<(i16, i16)> = (0..1470)
            .map(|i| ((i % 100) as i16 * 100, -((i % 97) as i16 * 100)))
            .collect();
        let frame = stereo_s16_frame(&pairs, 0);
        pipeline.process(0, &frame).must();
        pipeline.mix().must();
        let buffer = pipeline.take_output().must();
        assert_eq!(buffer.frames().must(), 1470);
        assert_eq!(buffer.data.len(), 1470 * 2 * 4);
    }

    #[test]
    fn quantum_output_rotation_is_bounded_and_values_stay_exact() {
        // The C6b fix's observable contract: the pooled rotation grows
        // only as far as downstream retention forces it (bounded at
        // OUT_SLOTS even under full retention), shrinks nowhere, and
        // every quantum still carries its exact bytes — the reuse
        // never leaks stale samples between quanta (each slot is
        // fully rewritten before being handed out).
        let mut pipeline = AudioPipeline::new(PipelineSpec {
            rate: RATE,
            channels: 2,
            tracks: 1,
        })
        .must();
        // Phase 1 — retain every quantum (the adversarial consumer):
        // the pool must absorb the pressure and cap at OUT_SLOTS.
        let mut held: Vec<AudioBuffer> = Vec::new();
        for quantum in 0..(3 * OUT_SLOTS + 5) {
            let amp = ((quantum % 7) as i16) * 1000 + 1;
            let frame = stereo_s16_frame(&[(amp, -amp), (amp / 2, -amp / 2)], quantum as i64);
            pipeline.process(0, &frame).must();
            assert!(pipeline.mix().must());
            held.push(pipeline.take_output().must());
        }
        assert_eq!(
            pipeline.out_slots.len(),
            OUT_SLOTS,
            "retention pressure caps at the rotation depth"
        );
        // Every held quantum kept its exact bytes (no cross-quantum
        // leakage through the recycled slots).
        for (quantum, buffer) in held.iter().enumerate() {
            let amp = ((quantum % 7) as i16) * 1000 + 1;
            assert_eq!(
                read_f32_pairs(buffer),
                vec![
                    (f32::from(amp) / 32_768.0, f32::from(-amp) / 32_768.0),
                    (
                        f32::from(amp / 2) / 32_768.0,
                        f32::from(-amp / 2) / 32_768.0
                    ),
                ]
            );
        }
        // Phase 2 — release everything: the steady state reuses the
        // existing slots (pool never grows past the cap again) and
        // values stay exact. Same quantum geometry as phase 1, so the
        // slot lengths match and reuse is in-place.
        held.clear();
        for quantum in 0..(2 * OUT_SLOTS) {
            let amp = ((quantum % 5) as i16) * 900 + 7;
            let frame = stereo_s16_frame(&[(amp, -amp), (amp / 3, -amp / 3)], quantum as i64);
            pipeline.process(0, &frame).must();
            assert!(pipeline.mix().must());
            let buffer = pipeline.take_output().must();
            assert_eq!(
                read_f32_pairs(&buffer),
                vec![
                    (f32::from(amp) / 32_768.0, f32::from(-amp) / 32_768.0),
                    (
                        f32::from(amp / 3) / 32_768.0,
                        f32::from(-amp / 3) / 32_768.0
                    ),
                ]
            );
        }
        assert!(pipeline.out_slots.len() <= OUT_SLOTS);
    }

    // ---- 3.B1: the drift seam (set_drift / observe_drift / reset) -----

    const TARGET_FILL_NS: u64 = 40_000_000;

    fn drift_pipeline() -> AudioPipeline {
        AudioPipeline::new(PipelineSpec {
            rate: RATE,
            channels: 2,
            tracks: 1,
        })
        .must()
        .with_drift_governor(TARGET_FILL_NS)
    }

    fn quantum(index: u64) -> AudioFrame {
        let pairs: Vec<(i16, i16)> = (0..1470_u64)
            .map(|i| {
                let v = (((index * 37 + i * 11) % 4096) as i16) - 2048;
                (v, -v)
            })
            .collect();
        stereo_s16_frame(&pairs, index as i64 * 1470)
    }

    #[test]
    fn zero_correction_is_the_bit_exact_bypass() {
        // The pre-3.B1 identity, pinned: with `DriftCorrection::ZERO`
        // in force (the default, and what an unarmed pipeline stays
        // at forever), the quantum's bytes are exactly the pre-3.B1
        // path's bytes — no interpolation, no copy, no resampler
        // call (bypass by ownership).
        let mut plain = AudioPipeline::new(PipelineSpec {
            rate: RATE,
            channels: 2,
            tracks: 1,
        })
        .must();
        let mut corrected = drift_pipeline();
        corrected.set_drift(media_clock::DriftCorrection::ZERO);
        for index in 0..4_u64 {
            let frame = quantum(index);
            plain.process(0, &frame).must();
            corrected.process(0, &frame).must();
            assert!(plain.mix().must());
            assert!(corrected.mix().must());
            let a = plain.take_output().must();
            let b = corrected.take_output().must();
            assert_eq!(a.frames().must(), b.frames().must());
            assert_eq!(a.data.as_slice(), b.data.as_slice());
        }
        assert_eq!(corrected.drift(), media_clock::DriftCorrection::ZERO);
    }

    #[test]
    fn nonzero_correction_shifts_the_quantum_by_the_exact_ratio() {
        // +1000 ppm over 1000 quanta of 1470: out = in × 1.001 ± 1
        // (the sub-sample phase straddling quantum boundaries) — the
        // doc 03 §4.8 f "filter declares a rate change" exception,
        // bounded by the ±1000 ppm limiter.
        let mut pipeline = drift_pipeline();
        pipeline.set_drift(media_clock::DriftCorrection::new(1000));
        let mut total_in = 0_u64;
        let mut total_out = 0_u64;
        for index in 0..1000_u64 {
            let frame = quantum(index);
            total_in += 1470;
            pipeline.process(0, &frame).must();
            assert!(pipeline.mix().must());
            total_out += pipeline.take_output().must().frames().must();
        }
        let want = total_in * 1001 / 1000;
        assert!(
            total_out.abs_diff(want) <= 1,
            "out {total_out} vs exact {want}"
        );
        // And −1000 ppm symmetric.
        let mut pipeline = drift_pipeline();
        pipeline.set_drift(media_clock::DriftCorrection::new(-1000));
        let mut total_out = 0_u64;
        for index in 0..1000_u64 {
            let frame = quantum(index);
            pipeline.process(0, &frame).must();
            assert!(pipeline.mix().must());
            total_out += pipeline.take_output().must().frames().must();
        }
        let want = total_in * 999 / 1000;
        assert!(
            total_out.abs_diff(want) <= 1,
            "out {total_out} vs exact {want}"
        );
    }

    #[test]
    fn armed_governor_closes_the_loop_through_observe_drift() {
        // The pipeline-level loop closure (the wiring's one call): a
        // draining device (fill below target) drives the correction
        // positive, the correction reaches the resampler, and the
        // quantum's device-sample count grows — the S1 plant, run
        // through the pipeline's own seam.
        let mut pipeline = drift_pipeline();
        // Start at the target: no correction wanted.
        let mut now = 0_u64;
        let quantum_ns = 1470 * 1_000_000_000_u64 / u64::from(RATE);
        let correction = pipeline.observe_drift(now, Some(TARGET_FILL_NS));
        assert_eq!(correction, media_clock::DriftCorrection::ZERO);
        // A persistently draining fill (20 ms under target): after
        // the 1 s window the governor commands positive correction
        // (emit MORE device samples to refill), step-limited. One
        // quantum is 33 333 333 ns, so the window elapses at the
        // 31st post-opening sample (31 × q = 1.033 s ≥ 1 s).
        for _ in 0..40 {
            now += quantum_ns;
            let correction = pipeline.observe_drift(now, Some(TARGET_FILL_NS - 20_000_000));
            assert!(correction.ppm() >= 0, "{}", correction.ppm());
        }
        // Exactly one full window has evaluated by then.
        let stats = pipeline.drift_stats();
        assert_eq!(stats.updates, 1, "{}", stats.updates);
        assert_eq!(stats.samples, 41);
        let ppm = pipeline.drift().ppm();
        // for_divergence(−20 ms, 20 s) = +1000 ppm desired, but the
        // step limiter caps the approach at +250 per window.
        assert_eq!(ppm, 250, "step-limited first window: {ppm}");
        // The correction is live in the resampler: the next quantum
        // carries ~1.00025 × its input frames.
        let frame = quantum(0);
        pipeline.process(0, &frame).must();
        assert!(pipeline.mix().must());
        let frames = pipeline.take_output().must().frames().must();
        assert!(
            frames == 1470 || frames == 1471,
            "1470 × 1.00025 ⇒ 1470.37: {frames}"
        );
    }

    #[test]
    fn unarmed_pipeline_ignores_drift_observations() {
        // Without a governor the observations are honest no-ops
        // (`ZERO`): direct `set_drift` remains the only control —
        // what a host with its own measurement loop wants.
        let mut pipeline = AudioPipeline::new(PipelineSpec {
            rate: RATE,
            channels: 2,
            tracks: 1,
        })
        .must();
        let mut now = 0_u64;
        for _ in 0..90 {
            now += 33_333_333;
            let correction = pipeline.observe_drift(now, Some(0));
            assert_eq!(correction, media_clock::DriftCorrection::ZERO);
        }
        assert_eq!(pipeline.drift_stats(), DriftStats::default());
        // And an unmeasurable sink parks an ARMED governor at zero.
        let mut armed = drift_pipeline();
        armed.set_drift(media_clock::DriftCorrection::new(600));
        let mut now = 0_u64;
        for _ in 0..90 {
            now += 33_333_333;
            let correction = armed.observe_drift(now, None);
            assert_eq!(correction, media_clock::DriftCorrection::ZERO);
        }
        assert_eq!(armed.drift_stats().unmeasurable, 90);
    }

    #[test]
    fn reset_drift_restarts_the_measurement_regime() {
        let mut pipeline = drift_pipeline();
        let mut now = 0_u64;
        for _ in 0..60 {
            now += 33_333_333;
            let _ = pipeline.observe_drift(now, Some(TARGET_FILL_NS - 30_000_000));
        }
        assert!(pipeline.drift().ppm() > 0);
        let samples_before = pipeline.drift_stats().samples;
        pipeline.reset_drift();
        assert_eq!(pipeline.drift(), media_clock::DriftCorrection::ZERO);
        assert_eq!(pipeline.drift_stats().resets, 1);
        // The resampler's stream phase restarted too: the next
        // quantum at zero correction conserves its count exactly.
        let frame = quantum(0);
        pipeline.process(0, &frame).must();
        assert!(pipeline.mix().must());
        assert_eq!(pipeline.take_output().must().frames().must(), 1470);
        // The estimator's windowed-sample count is monotonic across
        // resets by contract (media-clock sync.rs: "callers track
        // resets through their own epochs"; the governor's own
        // `resets` counter is the regime boundary).
        let _ = pipeline.observe_drift(now, Some(TARGET_FILL_NS));
        assert_eq!(pipeline.drift_stats().samples, samples_before + 1);
    }
}
