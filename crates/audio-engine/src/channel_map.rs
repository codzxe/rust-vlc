//! Channel routing: stereo pair routing and friends (card 2.C3;
//! doc 01 §2.5 — the mixer's channel-shape negotiation).
//!
//! A [`ChannelMap`] routes each **output** channel as a sum of zero or
//! more **input** channels (unity gain — weighted mixing is the
//! mixer's volume stage, not the map's). The v0 vocabulary is the
//! stereo-pair set the card names: identity, swap, mono→stereo
//! (duplicate), stereo→mono (sum), plus explicit `(from, to)` routes
//! for anything else. An output channel with no sources is silence
//! (dropping a channel is the complement of duplicating one); an
//! input channel feeding several outputs is duplicated.
//!
//! Maps apply in the f32 planar mixer domain (doc 03 §4.8 e): frame
//! counts never change — only channel shapes do (the sample-count
//! conservation invariant).

use media_types::EngineError;

use crate::error;

/// Input→output channel routing (stereo pair routing in v0).
///
/// Constructed validated: every route's indices are in range for the
/// declared channel counts. Stored as `sources[out] = [in...]` — the
/// apply loop touches each output plane once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelMap {
    in_channels: u32,
    out_channels: u32,
    sources: Vec<Vec<u32>>,
}

impl ChannelMap {
    /// The identity: `channels` in, `channels` out, each channel to
    /// itself.
    ///
    /// # Errors
    /// [`Unsupported`](media_types::ErrorKind::Unsupported) with code
    /// `audio.channel_count_zero`: zero channels is not a shape.
    pub fn identity(channels: u32) -> Result<ChannelMap, EngineError> {
        if channels == 0 {
            return Err(error::channel_range("audio.channel_count_zero"));
        }
        Ok(ChannelMap {
            in_channels: channels,
            out_channels: channels,
            sources: (0..channels).map(|c| vec![c]).collect(),
        })
    }

    /// The stereo swap: 2 in, 2 out, left and right exchanged. A
    /// mis-wired pair is audible in one operation (the corpus's
    /// distinct 440 Hz/660 Hz tone plan exists exactly to catch it).
    pub fn swap_stereo() -> ChannelMap {
        ChannelMap {
            in_channels: 2,
            out_channels: 2,
            sources: vec![vec![1], vec![0]],
        }
    }

    /// Mono upmix: 1 in, 2 out, both outputs fed the single input.
    pub fn mono_to_stereo() -> ChannelMap {
        ChannelMap {
            in_channels: 1,
            out_channels: 2,
            sources: vec![vec![0], vec![0]],
        }
    }

    /// Stereo downmix: 2 in, 1 out, the sum of left and right (the
    /// classic L+R mono fold; a −3 dB pan-law compensation is filter-
    /// chain work, not routing).
    pub fn stereo_to_mono() -> ChannelMap {
        ChannelMap {
            in_channels: 2,
            out_channels: 1,
            sources: vec![vec![0, 1]],
        }
    }

    /// An explicit routing table: each `(from, to)` pair feeds input
    /// channel `from` into output channel `to`. Repeated targets sum;
    /// unreferenced outputs are silence.
    ///
    /// # Errors
    /// [`Unsupported`](media_types::ErrorKind::Unsupported) with code
    /// `audio.channel_count_zero` (zero shape) or
    /// `audio.channel_index_out_of_range` (a route naming a channel
    /// that does not exist).
    pub fn from_routes(
        in_channels: u32,
        out_channels: u32,
        routes: &[(u32, u32)],
    ) -> Result<ChannelMap, EngineError> {
        if in_channels == 0 || out_channels == 0 {
            return Err(error::channel_range("audio.channel_count_zero"));
        }
        let mut sources = vec![Vec::new(); out_channels as usize];
        for &(from, to) in routes {
            if from >= in_channels || to >= out_channels {
                return Err(error::channel_range("audio.channel_index_out_of_range"));
            }
            sources[to as usize].push(from);
        }
        Ok(ChannelMap {
            in_channels,
            out_channels,
            sources,
        })
    }

    /// Input channel count the map expects.
    pub fn in_channels(&self) -> u32 {
        self.in_channels
    }

    /// Output channel count the map produces.
    pub fn out_channels(&self) -> u32 {
        self.out_channels
    }

    /// The sources summed into output channel `to`, in application
    /// order (deterministic: construction order).
    pub fn sources_of(&self, to: u32) -> &[u32] {
        match self.sources.get(to as usize) {
            Some(list) => list,
            None => &[],
        }
    }

    /// Apply to f32 planar planes (in → out). `out` is resized to
    /// `out_channels` planes of the input frame count; its capacity is
    /// reused across calls (no allocation after the first, doc 03
    /// §4.8 c).
    ///
    /// # Errors
    /// [`Corrupt`](media_types::ErrorKind::Corrupt) with code
    /// `audio.planar_plane_mismatch`: `in_planes` must have exactly
    /// `in_channels()` planes of one common length.
    pub fn apply(
        &self,
        in_planes: &[Vec<f32>],
        out: &mut Vec<Vec<f32>>,
    ) -> Result<(), EngineError> {
        if in_planes.len() != self.in_channels as usize {
            return Err(error::plane_mismatch("audio.planar_plane_mismatch"));
        }
        let frames = in_planes.first().map_or(0, Vec::len);
        if in_planes.iter().any(|p| p.len() != frames) {
            return Err(error::plane_mismatch("audio.planar_plane_mismatch"));
        }
        out.resize_with(self.out_channels as usize, Vec::new);
        for plane in out.iter_mut() {
            plane.clear();
            plane.resize(frames, 0.0);
        }
        for (to, sources) in self.sources.iter().enumerate() {
            let out_plane = &mut out[to];
            for &from in sources {
                let in_plane = &in_planes[from as usize];
                // Deterministic order: source-major accumulation into
                // the output plane, left-to-right in route order.
                for (dst, sample) in out_plane.iter_mut().zip(in_plane) {
                    *dst += *sample;
                }
            }
        }
        Ok(())
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
    fn identity_is_bit_exact() {
        let map = ChannelMap::identity(2).must();
        let input = planes(&[&[0.0, 0.25, -0.5], &[0.75, -1.0, 0.125]]);
        let mut out = Vec::new();
        map.apply(&input, &mut out).must();
        assert_eq!(out, input);
        assert_eq!(map.in_channels(), 2);
        assert_eq!(map.out_channels(), 2);
    }

    #[test]
    fn swap_exchanges_the_pair() {
        let map = ChannelMap::swap_stereo();
        let input = planes(&[&[1.0, 2.0], &[3.0, 4.0]]);
        let mut out = Vec::new();
        map.apply(&input, &mut out).must();
        assert_eq!(out, planes(&[&[3.0, 4.0], &[1.0, 2.0]]));
    }

    #[test]
    fn mono_to_stereo_duplicates() {
        let map = ChannelMap::mono_to_stereo();
        let input = planes(&[&[0.5, -0.5]]);
        let mut out = Vec::new();
        map.apply(&input, &mut out).must();
        assert_eq!(out, planes(&[&[0.5, -0.5], &[0.5, -0.5]]));
    }

    #[test]
    fn stereo_to_mono_sums() {
        let map = ChannelMap::stereo_to_mono();
        let input = planes(&[&[0.25, 1.0], &[0.5, -1.0]]);
        let mut out = Vec::new();
        map.apply(&input, &mut out).must();
        assert_eq!(out, planes(&[&[0.75, 0.0]]));
    }

    #[test]
    fn routes_validate_indices_and_sum_duplicates() {
        // 3 in, 2 out: out0 = in0 + in2, out1 = in1.
        let map = ChannelMap::from_routes(3, 2, &[(0, 0), (2, 0), (1, 1)]).must();
        assert_eq!(map.sources_of(0), &[0, 2]);
        assert_eq!(map.sources_of(1), &[1]);
        let input = planes(&[&[1.0], &[2.0], &[4.0]]);
        let mut out = Vec::new();
        map.apply(&input, &mut out).must();
        assert_eq!(out, planes(&[&[5.0], &[2.0]]));
        // Unreferenced output is silence.
        let map = ChannelMap::from_routes(1, 2, &[(0, 1)]).must();
        let mut out = Vec::new();
        map.apply(&planes(&[&[0.5]]), &mut out).must();
        assert_eq!(out, planes(&[&[0.0], &[0.5]]));

        let err = ChannelMap::from_routes(2, 2, &[(2, 0)]).err().must();
        assert_eq!(err.context.code, "audio.channel_index_out_of_range");
        let err = ChannelMap::from_routes(2, 2, &[(0, 5)]).err().must();
        assert_eq!(err.context.code, "audio.channel_index_out_of_range");
        let err = ChannelMap::identity(0).err().must();
        assert_eq!(err.context.code, "audio.channel_count_zero");
    }

    #[test]
    fn apply_rejects_wrong_plane_geometry() {
        let map = ChannelMap::identity(2).must();
        let mut out = Vec::new();
        let err = map.apply(&planes(&[&[0.0]]), &mut out).err().must();
        assert_eq!(err.context.code, "audio.planar_plane_mismatch");
        let err = map
            .apply(&planes(&[&[0.0, 1.0], &[0.0]]), &mut out)
            .err()
            .must();
        assert_eq!(err.context.code, "audio.planar_plane_mismatch");
    }

    #[test]
    fn apply_reuses_output_capacity() {
        // The no-allocation-after-negotiation contract: a second apply
        // with the same geometry must not grow the out planes.
        let map = ChannelMap::stereo_to_mono();
        let input = planes(&[&[1.0; 512], &[2.0; 512]]);
        let mut out = Vec::new();
        map.apply(&input, &mut out).must();
        let capacity: usize = out.iter().map(Vec::capacity).sum();
        map.apply(&input, &mut out).must();
        assert_eq!(out.iter().map(Vec::capacity).sum::<usize>(), capacity);
        assert_eq!(out[0], vec![3.0; 512]);
    }
}
