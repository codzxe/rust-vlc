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

use media_types::EngineError;
use media_types::frame::{AudioFormat, AudioFrame, AudioLayout, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::time::Stamp;

use crate::buffer::AudioBuffer;
use crate::channel_map::ChannelMap;
use crate::convert::{frame_to_f32_planes, planes_to_f32_interleaved};
use crate::error;
use crate::mix::Mixer;
use crate::volume::Volume;

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
    /// Stage-D output bytes (reused).
    out_bytes: Vec<u8>,
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
            out_bytes: Vec::new(),
            pending: None,
        })
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
        // Field-disjoint borrows: the refs read `stage`, the mixer
        // reads its volumes, `mixed` is written.
        let refs: Vec<&[Vec<f32>]> = self.stage.iter().map(Vec::as_slice).collect();
        {
            let mut mixed = std::mem::take(&mut self.mixed);
            let mix_result = self.mixer.mix(&refs, &mut mixed);
            self.mixed = mixed;
            mix_result?;
        }
        let mut out = std::mem::take(&mut self.out_bytes);
        let interleave_result = planes_to_f32_interleaved(&self.mixed, &mut out);
        self.out_bytes = out;
        interleave_result?;
        let pts = self.timing.take().unwrap_or(Stamp::Missing);
        self.pending = Some(AudioBuffer {
            format: self.device_format(),
            data: SharedBytes::from_vec(std::mem::take(&mut self.out_bytes)),
            pts,
        });
        for staged in self.staged.iter_mut() {
            *staged = 0;
        }
        for chunk in self.stage.iter_mut() {
            chunk.clear();
        }
        Ok(true)
    }

    /// Take the mixed quantum (once per [`mix`](Self::mix) that
    /// returned `true`).
    pub fn take_output(&mut self) -> Option<AudioBuffer> {
        self.pending.take()
    }
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
}
