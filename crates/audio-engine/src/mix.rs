//! The straight mixer: sample-wise summing of tracks with per-track
//! gain (card 2.C3; doc 03 §4.8 b, insertion point C).
//!
//! "Straight" is the card's word and the contract: `out[c][i] =
//! Σ_t volume_t · in_t[c][i]`, accumulated in ascending track order —
//! no reverb, no EQ, no crossfade, no dither. Float addition is not
//! associative, so the fixed order is what makes the mix
//! **deterministic** (byte-stable across runs and hosts, which the
//! sink-buffer hash AC depends on).
//!
//! Sum overflow stays in the f32 domain (values may exceed ±1.0 or
//! grow toward infinity — *data* at this stage); stage D's saturating
//! clamp is where magnitude is bounded (doc 03 §4.8 e's soft limiter
//! is the later hardening).

use media_types::EngineError;

use crate::error;
use crate::volume::Volume;

/// A straight-sum mixer over f32 planar chunks.
#[derive(Debug, Clone)]
pub struct Mixer {
    volumes: Vec<Volume>,
}

impl Mixer {
    /// A mixer over `tracks` inputs, all at [`Volume::UNITY`] — the
    /// neutral starting point (a single track at unity passes through
    /// bit-exact, which the AC suite relies on).
    ///
    /// # Errors
    /// [`Unsupported`](media_types::ErrorKind::Unsupported) with code
    /// `audio.track_count_zero`: mixing zero tracks is a wiring bug.
    pub fn new(tracks: usize) -> Result<Mixer, EngineError> {
        if tracks == 0 {
            return Err(error::track_range("audio.track_count_zero"));
        }
        Ok(Mixer {
            volumes: vec![Volume::UNITY; tracks],
        })
    }

    /// Track count.
    pub fn tracks(&self) -> usize {
        self.volumes.len()
    }

    /// Set one track's gain.
    ///
    /// # Errors
    /// [`Unsupported`](media_types::ErrorKind::Unsupported) with code
    /// `audio.track_index_out_of_range` (the gain's own validation
    /// errors propagate from [`Volume`]).
    pub fn set_volume(&mut self, track: usize, volume: Volume) -> Result<(), EngineError> {
        let slot = self
            .volumes
            .get_mut(track)
            .ok_or_else(|| error::track_range("audio.track_index_out_of_range"))?;
        *slot = volume;
        Ok(())
    }

    /// One track's gain (unity when unset).
    ///
    /// # Errors
    /// [`Unsupported`](media_types::ErrorKind::Unsupported) with code
    /// `audio.track_index_out_of_range`.
    pub fn volume(&self, track: usize) -> Result<Volume, EngineError> {
        self.volumes
            .get(track)
            .copied()
            .ok_or_else(|| error::track_range("audio.track_index_out_of_range"))
    }

    /// Mix `inputs` (one planar chunk per track) into `out`, ascending
    /// track order. `out` is resized to the input geometry; capacity
    /// is reused across calls (doc 03 §4.8 c). Returns the frame
    /// count.
    ///
    /// # Errors
    /// [`Corrupt`](media_types::ErrorKind::Corrupt) with code
    /// `audio.planar_plane_mismatch` (a track's planes are ragged) or
    /// `audio.chunk_length_mismatch` (tracks disagree on plane count
    /// or frame count — the v0 mixer mixes whole aligned chunks;
    /// cross-track timing alignment is the session's job, doc 03
    /// §4.6).
    pub fn mix(
        &self,
        inputs: &[&[Vec<f32>]],
        out: &mut Vec<Vec<f32>>,
    ) -> Result<usize, EngineError> {
        if inputs.len() != self.volumes.len() {
            return Err(error::chunk_alignment());
        }
        let channels = inputs.first().map_or(0, |c| c.len());
        let frames = inputs.first().map_or(0, |c| c.first().map_or(0, Vec::len));
        for chunk in inputs {
            if chunk.len() != channels || chunk.iter().any(|p| p.len() != frames) {
                return Err(error::chunk_alignment());
            }
        }
        out.resize_with(channels, Vec::new);
        for plane in out.iter_mut() {
            plane.clear();
            plane.resize(frames, 0.0);
        }
        for (track, chunk) in inputs.iter().enumerate() {
            let gain = self.volumes[track].linear();
            for (c, plane) in chunk.iter().enumerate() {
                let out_plane = &mut out[c];
                for (dst, sample) in out_plane.iter_mut().zip(plane) {
                    // Ascending-track accumulation: the deterministic
                    // evaluation order (float addition is not
                    // associative).
                    *dst += *sample * gain;
                }
            }
        }
        Ok(frames)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    fn planes(data: &[&[f32]]) -> Vec<Vec<f32>> {
        data.iter().map(|p| p.to_vec()).collect()
    }

    #[test]
    fn single_track_at_unity_is_bit_exact() {
        let mixer = Mixer::new(1).must();
        let input = planes(&[&[0.0, 0.5, -0.25, 1.0], &[0.125, -1.0, 0.75, -0.5]]);
        let refs: &[&[Vec<f32>]] = &[&input];
        let mut out = Vec::new();
        let frames = mixer.mix(refs, &mut out).must();
        assert_eq!(frames, 4);
        assert_eq!(out, input);
    }

    #[test]
    fn two_tracks_sum_in_track_order() {
        let mixer = Mixer::new(2).must();
        let a = planes(&[&[1.0, 2.0], &[4.0, 8.0]]);
        let b = planes(&[&[10.0, 20.0], &[40.0, 80.0]]);
        let refs: &[&[Vec<f32>]] = &[&a, &b];
        let mut out = Vec::new();
        mixer.mix(refs, &mut out).must();
        assert_eq!(out, planes(&[&[11.0, 22.0], &[44.0, 88.0]]));
    }

    #[test]
    fn per_track_volumes_apply_before_summing() {
        let mut mixer = Mixer::new(2).must();
        mixer.set_volume(0, Volume::from_linear(0.5).must()).must();
        mixer.set_volume(1, Volume::from_linear(2.0).must()).must();
        let a = planes(&[&[1.0], &[1.0]]);
        let b = planes(&[&[1.0], &[1.0]]);
        let refs: &[&[Vec<f32>]] = &[&a, &b];
        let mut out = Vec::new();
        mixer.mix(refs, &mut out).must();
        assert_eq!(out, planes(&[&[2.5], &[2.5]]));
        assert_eq!(mixer.volume(0).must().linear(), 0.5);
        assert_eq!(mixer.volume(1).must().linear(), 2.0);
    }

    #[test]
    fn mute_leaves_the_other_track_untouched() {
        let mut mixer = Mixer::new(2).must();
        mixer.set_volume(0, Volume::MUTE).must();
        let a = planes(&[&[5.0]]);
        let b = planes(&[&[7.0]]);
        let refs: &[&[Vec<f32>]] = &[&a, &b];
        let mut out = Vec::new();
        mixer.mix(refs, &mut out).must();
        assert_eq!(out, planes(&[&[7.0]]));
    }

    #[test]
    fn geometry_and_track_errors_are_typed() {
        let mut mixer = Mixer::new(2).must();
        let mut out = Vec::new();
        // Wrong track count.
        let a = planes(&[&[0.0]]);
        let refs: &[&[Vec<f32>]] = &[&a];
        let err = mixer.mix(refs, &mut out).err().must();
        assert_eq!(err.context.code, "audio.chunk_length_mismatch");
        // Ragged planes.
        let b = planes(&[&[0.0, 1.0]]);
        let refs: &[&[Vec<f32>]] = &[&a, &b];
        let err = mixer.mix(refs, &mut out).err().must();
        assert_eq!(err.context.code, "audio.chunk_length_mismatch");
        // Channel-count disagreement.
        let c = planes(&[&[0.0], &[0.0]]);
        let refs: &[&[Vec<f32>]] = &[&a, &c];
        let err = mixer.mix(refs, &mut out).err().must();
        assert_eq!(err.context.code, "audio.chunk_length_mismatch");
        // Track index out of range.
        let err = mixer.set_volume(9, Volume::UNITY).err().must();
        assert_eq!(err.context.code, "audio.track_index_out_of_range");
        let err = mixer.volume(9).err().must();
        assert_eq!(err.context.code, "audio.track_index_out_of_range");
        // Zero tracks.
        let err = Mixer::new(0).err().must();
        assert_eq!(err.context.code, "audio.track_count_zero");
    }

    #[test]
    fn output_capacity_is_reused() {
        let mixer = Mixer::new(1).must();
        let input = planes(&[&[0.5; 1024], &[0.25; 1024]]);
        let refs: &[&[Vec<f32>]] = &[&input];
        let mut out = Vec::new();
        mixer.mix(refs, &mut out).must();
        let capacity: usize = out.iter().map(Vec::capacity).sum();
        mixer.mix(refs, &mut out).must();
        assert_eq!(out.iter().map(Vec::capacity).sum::<usize>(), capacity);
    }
}
