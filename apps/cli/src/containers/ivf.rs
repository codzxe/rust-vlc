//! The IVF container host (card 6.OPW-ivf — the registry's fifth
//! wired family, the AV1/VP9 delivery container of the libvpx/AOM
//! lineage): construct the demuxer, run the host pre-pass `start()`
//! (the 32-byte `DKIF` header walk — fourcc gate, coded dims, the
//! rational tick rate), harvest the header's stream facts into
//! [`TrackFact`]s, validate `--start-time` through the demuxer's pure
//! `seek` query (the bounded header-only frame scan — `SlowLinear`,
//! no index exists in this format), wrap in the shared
//! [`SelectDemuxer`](crate::demux_host::SelectDemuxer).
//!
//! The harvest's decoder-config truth (the doc §4.3.3 rule; the
//! demuxer's [`stream_info`](demux::ivf::IvfDemuxer::stream_info)
//! accessors are the facts source — the OPW-0 doc's own citation):
//!
//! * **The codec descriptor is the fourcc's honest mapping** (`av1`
//!   for `AV01`, `vp9` for `VP90` — the demuxer's own gate, never a
//!   local copy; every other fourcc already refused typed at
//!   `start`). AV1 **decodes** (the rav1d-backed `Av1Decoder`, the
//!   provider's existing `av1`/`av01` arm — mp4's av1C blob is NOT
//!   needed here, see below). VP9 **decodes** too (the rusty_vp9-
//!   backed `Vp9Decoder`, card 6.M-vp9 landed and the Lead
//!   reconciled this host's vp9 lane — the ogg/opus inversion
//!   precedent; in-band uncompressed frame headers, no seeding).
//! * **No `extra_data`**: IVF separates no config from the stream —
//!   the AV1 sequence header is the first in-band OBU of the first
//!   keyframe and VP9's frame header is the per-frame uncompressed
//!   sync bytes; the container's payload pass-through is byte-for-
//!   byte (the demuxer's own no-parse contract). Seeding anything
//!   here would be an invention (`None` is the wiring, not an
//!   omission — the AV1 decoder's configure path for this family is
//!   the in-band one the codec differential pins).
//! * **No PCM pair**: the single track is video.
//! * **`frame_ns` = the header's declared one-tick frame duration**
//!   (`tick_nanos` — one tick = `scale`/`rate` seconds, the value
//!   every packet carries as its declared duration, ffmpeg's own
//!   `duration = 1` convention). The avi host computes the same
//!   figure from its track base; a VFR file whose pts ladder skips
//!   still *declares* this cadence — the gaps are the container's
//!   data, passed through by the demuxer, never smoothed here.
//! * **Dimensions are the header's own declaration** (the coded
//!   `width`/`height` at offsets 12/14 — the fields ffprobe reports;
//!   no crop/display vocabulary exists in this format).
//! * **No declared duration**: the track's span end is `Missing` at
//!   open (the exact extent exists only after the walk — the demuxer
//!   refines it via `track_duration`). There is nothing to clamp
//!   `--start-time` against; the pure seek query is the validator
//!   (a past-the-end target lands on the last frame, and the skip
//!   watermark then admits no packet — the honest no, the ADTS
//!   shape).
//! * **Seek** is `SlowLinear` (no index — the landing needs the
//!   bounded header-only scan; the OPW-0 doc's §5 shorthand said
//!   "frame-index Fast", its own §6 unverified note — this wiring
//!   asserts the value the demuxer actually returns).

use commands::CommandError;
use demux::ivf::IvfDemuxer;
use media_types::Track;
use media_types::time::{Stamp, TimeSpan, Timestamp};
use plugin_api::{Demuxer, SourceStream};

use crate::demux_host::{
    MediaFacts, OpenOptions, OpenedMedia, SelectDemuxer, TrackFact, wire_error,
};

/// Open an IVF source (the doc §4.2 per-container shape): the probe
/// already named `ivf` (the `DKIF` magic, both AV1 and VP9 fourccs);
/// this constructs the demuxer, runs the host pre-pass `start()`,
/// harvests the header facts, validates the start position through
/// the seek query and wraps the demuxer in the skip/selection
/// wrapper.
///
/// # Errors
/// Typed [`CommandError`]s (stage `demux`): a non-`DKIF` or
/// structurally invalid header, a fourcc outside the AV1/VP9 delivery
/// scope, a header-only file (no frames to demux), seek-scan
/// violations.
pub(crate) fn open(
    stream: Box<dyn SourceStream>,
    opts: &OpenOptions,
) -> Result<OpenedMedia, CommandError> {
    // 1. Demuxer + the host pre-pass (track list + header facts).
    let mut demuxer = IvfDemuxer::new(stream).map_err(|err| wire_error(&err))?;
    let tracks = demuxer.start().map_err(|err| wire_error(&err))?;
    let facts = harvest_facts(&demuxer, &tracks);
    // 2. The start position (doc 05 §2.4): IVF declares no duration
    //    up front (`duration_us` is None — the exact figure exists
    //    only post-walk), so there is no span to clamp against; the
    //    pure seek query validates the target instead.
    let start_us = opts.start_time_us.unwrap_or(0).max(0);
    let mut skip_until_ns = None;
    if start_us > 0 {
        // The pure seek query (SlowLinear — the bounded header-only
        // frame scan) lands at-or-before the target; a past-the-end
        // target lands on the last frame and the skip watermark
        // below then admits no packet.
        let target = TimeSpan {
            start: Stamp::Present(Timestamp::new(
                start_us.saturating_mul(1000),
                media_clock::NANO_BASE,
            )),
            end: Stamp::Missing,
        };
        demuxer.seek(target).map_err(|err| wire_error(&err))?;
        skip_until_ns = Some(start_us.saturating_mul(1000));
    }
    let wrapper = SelectDemuxer::new(
        demuxer,
        skip_until_ns,
        facts
            .facts
            .iter()
            .map(|fact| (fact.id, fact.base))
            .collect(),
        opts.no_video,
        opts.no_audio,
    );
    Ok(OpenedMedia {
        demuxer: Box::new(wrapper),
        facts,
        start_position_ns: start_us.saturating_mul(1000),
        uri: opts.uri.clone(),
    })
}

/// Harvest the track facts from a started IVF demuxer: exactly one
/// video track; the fourcc-honest codec descriptor, the container's
/// own tick base (one tick = `scale`/`rate` seconds) and the coded
/// dimensions are the track's own truth, `stream_info()` the
/// demuxer-side confirmation (codec/dims/rate/scale — the fields
/// ffprobe reports). The span end stays `None` (IVF's exact duration
/// is walk-derived), and no blob is harvested (the config rides
/// in-band — see the module docs).
fn harvest_facts(demuxer: &IvfDemuxer, tracks: &[Track]) -> MediaFacts {
    // `stream_info()` (codec/dims/rate/scale — the fields ffprobe
    // reports) is the demuxer-side confirmation of the same header
    // the track was minted from: the tick base IS the header's
    // scale/rate ratio, and the descriptor IS the fourcc's mapping.
    // Pinned as the harvest's invariant (divergence would mean the
    // track and the accessor disagree about one header — a demuxer
    // bug, never a wiring guess).
    let info = demuxer.stream_info();
    if let Some(info) = info
        && let Some(track) = tracks.first()
    {
        debug_assert_eq!(
            track.timing.base.as_parts(),
            (info.scale, info.rate),
            "the tick base is the header's scale/rate ratio"
        );
        debug_assert_eq!(
            track.codec.as_str(),
            info.codec,
            "the descriptor is the fourcc's mapping"
        );
    }
    // The declared one-tick frame duration (every packet carries it
    // as its declared duration — see the module docs).
    let frame_ns = demuxer.header().and_then(|head| head.tick_nanos());
    let dimensions = info.map(|info| (u32::from(info.width), u32::from(info.height)));
    let mut facts = Vec::with_capacity(tracks.len());
    for track in tracks {
        let span_end_ticks = track.timing.span.end.timestamp().ok().map(|ts| ts.ticks);
        facts.push(TrackFact {
            id: track.id,
            kind: track.kind,
            codec: track.codec.as_str().to_string(),
            base: track.timing.base,
            span_end_ticks,
            pcm: None,
            extra_data: None,
            frame_ns,
            dimensions,
        });
    }
    MediaFacts {
        tracks: tracks.to_vec(),
        facts,
        container: "ivf".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demux_host::open_source;
    use crate::pipeline_host::HostDecoderProvider;
    use codec_api::{DecoderConfig, FrameSink, ThreadBudget};
    use commands::catalog::types::SourceSpec;
    use media_types::{EngineError, Frame, GenerationId, MediaKind};
    use playback_engine::DecoderProvider;
    use plugin_api::{DemuxEvent, SeekSupport};
    use std::path::PathBuf;
    use std::sync::Arc;
    use testkit::Must;

    /// The committed AV1 fixture set of the codecs crate (card 5.F2,
    /// ffmpeg-authored): the decode lane for this family — no `.ivf`
    /// file exists in the corpus (verified), so the codec card's own
    /// committed bytes are the real-file differential.
    fn av1_fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/codecs/tests/fixtures/av1")
            .join(name)
    }

    /// A fixture of the root workspace set (the demux card's
    /// committed ffmpeg products under `tests/fixtures/generated/ivf`
    /// — the av1 64×48 lane and the vp9 lane).
    fn generated_fixture(name: &str) -> PathBuf {
        testkit::fixture_path(&format!("generated/ivf/{name}"))
            .unwrap_or_else(|err| unreachable!("committed fixture {name}: {err}"))
    }

    /// The video sink: counts decoded **frames** (one `Frame::Video`
    /// per accepted frame — the ffmpeg differential's unit) and
    /// remembers the last frame's coded size.
    struct CountingSink(u64, (u32, u32));
    impl FrameSink for CountingSink {
        fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
            if let Frame::Video(video) = frame {
                self.0 += 1;
                self.1 = video.size;
            }
            Ok(())
        }
    }

    /// Open a source through the REAL CLI open path (canonicalize →
    /// access → probe → this container host).
    fn open(path: PathBuf, start_us: Option<i64>) -> crate::demux_host::OpenedMedia {
        match open_source(&SourceSpec::File { path }, start_us, false, false) {
            Ok(opened) => opened,
            Err(err) => unreachable!("{err}"),
        }
    }

    /// One lane's recorded truth (ffprobe, measured this session:
    /// `codec_name,width,height,r_frame_rate,time_base,nb_frames` —
    /// the same commands as the demux suite's fixture table).
    struct Truth {
        /// The fixture to open.
        path: PathBuf,
        /// ffprobe `codec_name` (the fourcc's mapping).
        codec: &'static str,
        /// The tick base (scale, rate) — ffprobe's `time_base`.
        base: (u32, u32),
        /// ffprobe `width`/`height`.
        dims: (u32, u32),
        /// ffprobe `nb_frames` == `nb_read_packets`.
        frames: u64,
    }

    /// The committed lane table (all numbers measured this session,
    /// never transcribed): five AV1 fixtures of the codecs crate plus
    /// the demux crate's generated AV1/VP9 pair. No corpus `.ivf`
    /// lane exists (verified) — these committed bytes are the
    /// family's real-file differential.
    fn lanes() -> Vec<Truth> {
        [
            ("rgb_192x108_6f.ivf", "av1", (1, 6), (192, 108), 6),
            ("smpte_160x128_18f.ivf", "av1", (1, 12), (160, 128), 18),
            ("tenbit_128x96_5f.ivf", "av1", (1, 5), (128, 96), 5),
            ("testsrc2_320x240_60f.ivf", "av1", (1, 30), (320, 240), 60),
            (
                "testsrc2_g6_192x108_18f.ivf",
                "av1",
                (1, 30),
                (192, 108),
                18,
            ),
        ]
        .into_iter()
        .map(|(name, codec, base, dims, frames)| Truth {
            path: av1_fixture(name),
            codec,
            base,
            dims,
            frames,
        })
        .chain(
            [
                ("av1_64x48_30fps.ivf", "av1", (1, 30), (64, 48), 30),
                ("vp9_64x48_25fps.ivf", "vp9", (1, 25), (64, 48), 25),
            ]
            .into_iter()
            .map(|(name, codec, base, dims, frames)| Truth {
                path: generated_fixture(name),
                codec,
                base,
                dims,
                frames,
            }),
        )
        .collect()
    }

    /// Every committed lane opens through the real open path with
    /// the recorded facts: one video track, the fourcc-honest codec
    /// descriptor, the container's own tick base (ffprobe's
    /// `time_base` — the rate/scale handling this card owns), the
    /// coded dimensions, the declared one-tick frame cadence,
    /// **no PCM pair and no blob** (the config rides in-band — see
    /// the module docs), and **no declared extent** (IVF's duration
    /// is walk-derived). The wrapper reports the demuxer's honest
    /// `SlowLinear` seekability (no index — the OPW-0 §5 shorthand
    /// said "Fast"; its §6 unverified note is resolved here by
    /// asserting what the code returns).
    #[test]
    fn lanes_open_with_expected_facts() {
        for truth in lanes() {
            let name = truth
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            assert!(truth.path.is_file(), "fixture missing: {:?}", truth.path);
            let opened = open(truth.path, None);
            assert_eq!(opened.facts.container, "ivf", "{name}");
            assert_eq!(opened.facts.tracks.len(), 1, "{name}");
            let track = &opened.facts.tracks[0];
            assert_eq!(track.kind, MediaKind::Video, "{name}");
            let fact = &opened.facts.facts[0];
            assert_eq!(fact.codec, truth.codec, "{name}");
            assert_eq!(
                fact.base.as_parts(),
                truth.base,
                "{name}: the tick base (ffprobe time_base)"
            );
            assert_eq!(fact.dimensions, Some(truth.dims), "{name}");
            assert!(fact.pcm.is_none(), "{name}: video family — no pair");
            assert!(fact.extra_data.is_none(), "{name}: in-band config");
            assert!(
                fact.span_end_ticks.is_none(),
                "{name}: IVF declares no extent at open"
            );
            let (scale, rate) = truth.base;
            let tick_ns = u128::from(scale) * 1_000_000_000 / u128::from(rate);
            assert_eq!(
                fact.frame_ns,
                u64::try_from(tick_ns).ok(),
                "{name}: the declared one-tick frame cadence"
            );
            assert_eq!(
                opened.demuxer.seekability(),
                SeekSupport::SlowLinear,
                "{name}: no index — the header-only scan"
            );
            assert_eq!(opened.start_position_ns, 0, "{name}");
        }
    }

    /// The packet flow (every lane): the demux walk through the
    /// wrapper emits exactly ffprobe's `nb_read_packets` count, the
    /// pts ladder is the container's own integers 0..N−1 (ffprobe's
    /// full `packet=pts` dump — the demux suite's convention), and
    /// every packet carries the declared one-tick duration.
    #[test]
    fn lanes_demux_the_full_frame_count() {
        for truth in lanes() {
            let name = truth
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            let mut opened = open(truth.path, None);
            let mut packets = 0u64;
            loop {
                match opened.demuxer.next_event() {
                    Ok(DemuxEvent::Packet(packet)) => {
                        let Stamp::Present(pts) = packet.pts else {
                            unreachable!("{name}[{packets}]: pts is present");
                        };
                        assert_eq!(
                            pts.ticks, packets as i64,
                            "{name}: the container's own pts ladder"
                        );
                        assert_eq!(pts.base.as_parts(), truth.base, "{name}");
                        assert!(
                            packet.duration.is_some(),
                            "{name}[{packets}]: the declared one-tick duration"
                        );
                        assert!(!packet.keyframe, "{name}: never invented");
                        packets += 1;
                    }
                    Ok(DemuxEvent::Recovered { skipped, reason }) => {
                        unreachable!("{name}: clean fixture recovered {skipped}: {reason}");
                    }
                    Ok(DemuxEvent::Discontinuity) => {
                        unreachable!("{name}: clean fixture is continuous");
                    }
                    Ok(DemuxEvent::EndOfStream) => break,
                    Err(err) => unreachable!("{name}: {err}"),
                }
            }
            assert_eq!(packets, truth.frames, "{name}: ffprobe frame count");
        }
    }

    /// The AV1 decode lanes through the REAL decoder provider (the
    /// in-band sequence-header path — no blob, module docs): the
    /// emitted frame count equals the packet count (the 1:1
    /// frame-per-TU shape the codec differential pins), every frame
    /// at the declared coded size. The **10-bit lane is the honest
    /// exception**: every TU refuses typed `av1.bitdepth` (the v0
    /// 8-bit boundary — the codec card's own recorded refusal, never
    /// a misdecode).
    #[test]
    fn av1_lanes_decode_the_full_stream() {
        // (name, dims, decoded frames) — the 8-bit lanes decode 1:1;
        // tenbit refuses (pinned below).
        for (name, dims, decoded) in [
            ("rgb_192x108_6f.ivf", (192, 108), 6u64),
            ("smpte_160x128_18f.ivf", (160, 128), 18),
            ("testsrc2_320x240_60f.ivf", (320, 240), 60),
            ("testsrc2_g6_192x108_18f.ivf", (192, 108), 18),
        ] {
            let mut opened = open(av1_fixture(name), None);
            assert_eq!(opened.facts.facts[0].codec, "av1", "{name}");
            let (frames, size) =
                decode_all(&mut opened).unwrap_or_else(|err| unreachable!("{name}: {err}"));
            assert_eq!(frames, decoded, "{name}: decoded frames (1:1 per TU)");
            assert_eq!(size, dims, "{name}: the declared coded size");
        }

        // The 10-bit lane: 5 packets demux, every TU refuses typed —
        // no fabricated frames, never a wrong-bit-depth decode.
        let mut opened = open(av1_fixture("tenbit_128x96_5f.ivf"), None);
        let facts = Arc::new(opened.facts.clone());
        let provider = HostDecoderProvider::new(Arc::clone(&facts));
        let track = facts.tracks[0].clone();
        let config = DecoderConfig::new(
            track.codec.clone(),
            GenerationId::FIRST,
            ThreadBudget::default(),
            None,
            None,
        );
        let mut decoder = provider
            .decoder_for(&track, &config)
            .unwrap_or_else(|err| unreachable!("tenbit: decoder_for: {err}"));
        let mut sink = CountingSink(0, (0, 0));
        let mut refused = 0usize;
        let mut first_code = String::new();
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    if let Err(err) = decoder.decode(packet, &mut sink) {
                        if first_code.is_empty() {
                            first_code = err.context.code.clone();
                        }
                        refused += 1;
                    }
                }
                Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {}
                Ok(DemuxEvent::EndOfStream) => break,
                Err(err) => unreachable!("tenbit: {err}"),
            }
        }
        assert_eq!(refused, 5, "tenbit: every 10-bit TU refuses");
        assert_eq!(first_code, "av1.bitdepth", "tenbit: the v0 boundary site");
        assert_eq!(sink.0, 0, "tenbit: no fabricated frames");
    }

    /// The generated 64×48 AV1 lane (the demux crate's committed
    /// ffmpeg product) decodes through the real open path too — the
    /// lane the ivf_fixtures demux suite pins byte-exactly, now
    /// carried through the CLI wiring end-to-end.
    #[test]
    fn generated_av1_lane_decodes_the_full_stream() {
        let mut opened = open(generated_fixture("av1_64x48_30fps.ivf"), None);
        assert_eq!(opened.facts.facts[0].codec, "av1");
        let (frames, size) =
            decode_all(&mut opened).unwrap_or_else(|err| unreachable!("av1_64x48: {err}"));
        assert_eq!(frames, 30, "av1_64x48: 30 packets → 30 frames");
        assert_eq!(size, (64, 48), "av1_64x48: the declared coded size");
    }

    /// The VP9 lane RECONCILED (card 6.M-vp9 landed — the Lead
    /// inversion per the ogg/opus precedent): the file opens, the
    /// facts are true (fourcc-honest `vp9` descriptor, dims, tick
    /// base), the packets flow — and rusty_vp9 decodes the full
    /// stream 1:1 through the real provider (no seeding: in-band
    /// uncompressed frame headers).
    #[test]
    fn vp9_lane_decodes_the_full_stream() {
        let path = generated_fixture("vp9_64x48_25fps.ivf");
        let mut opened = open(path, None);
        assert_eq!(opened.facts.container, "ivf");
        assert_eq!(opened.facts.facts[0].codec, "vp9");
        assert_eq!(opened.facts.facts[0].dimensions, Some((64, 48)));
        // The packets still flow (the demux half is container truth).
        let mut packets = 0u64;
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(_)) => packets += 1,
                Ok(DemuxEvent::EndOfStream) => break,
                Ok(_) => {}
                Err(err) => unreachable!("vp9: {err}"),
            }
        }
        assert_eq!(packets, 25, "vp9: ffprobe frame count");

        // The decode half (the inverted assertion): the full stream
        // through the real provider — 25 packets → 25 frames at the
        // declared coded size.
        let mut opened = open(generated_fixture("vp9_64x48_25fps.ivf"), None);
        let (frames, size) =
            decode_all(&mut opened).unwrap_or_else(|err| unreachable!("vp9 decode: {err}"));
        assert_eq!(frames, 25, "vp9: 25 packets → 25 frames");
        assert_eq!(size, (64, 48), "vp9: the declared coded size");
    }

    /// `--start-time` through the pure seek query (SlowLinear): an
    /// in-media target lands every surviving packet at/after it; a
    /// past-the-end target (there is no declared duration to clamp
    /// against) lands on the last frame and admits no packet — the
    /// honest no, never a guess.
    #[test]
    fn start_time_skips_packets() {
        // testsrc2_320x240_60f: 60 frames at 30 fps — the 1 s target
        // keeps exactly the second half (µs in, ns at the watermark:
        // 1_000_000 µs = 1e9 ns).
        let path = av1_fixture("testsrc2_320x240_60f.ivf");
        let mut mid = open(path.clone(), Some(1_000_000));
        assert_eq!(mid.start_position_ns, 1_000_000_000);
        let mut seen = 0u32;
        loop {
            match mid.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    seen += 1;
                    let Stamp::Present(pts) = packet.pts else {
                        unreachable!("pts is present");
                    };
                    let ns = pts
                        .convert(&media_clock::NANO_BASE)
                        .unwrap_or_else(|e| unreachable!("{e:?}"));
                    assert!(ns.ticks >= 1_000_000_000, "early packet: {} ns", ns.ticks);
                }
                Ok(DemuxEvent::EndOfStream) => break,
                Ok(_) => {}
                Err(err) => unreachable!("{err}"),
            }
        }
        assert_eq!(seen, 30, "the 1 s target keeps the second half");

        let mut past = open(path, Some(i64::MAX / 8));
        assert!(
            past.start_position_ns > 0,
            "unclamped (no declared duration) but honest"
        );
        let mut packets = 0u32;
        loop {
            match past.demuxer.next_event() {
                Ok(DemuxEvent::Packet(_)) => packets += 1,
                Ok(DemuxEvent::EndOfStream) => break,
                Ok(_) => {}
                Err(err) => unreachable!("{err}"),
            }
        }
        assert_eq!(packets, 0, "the whole media predates the requested start");
    }

    /// The crafted refusal shapes through the open path (the probe
    /// still names `ivf` — the `DKIF` magic is intact; the demuxer's
    /// typed refusals are the container host's `start` answers): a
    /// header-only file refuses `demux.ivf.no-frames` (ffmpeg opens
    /// it and yields an empty stream — the recorded divergence), and
    /// a `VP80` fourcc refuses `demux.ivf.unsupported-fourcc` (the
    /// v0 AV1/VP9 delivery scope — ffmpeg demuxes VP8 fine; scope,
    /// not a parse failure).
    #[test]
    fn crafted_shapes_refuse_at_the_demux_stage() {
        // Header only: no frames to demux.
        let header_only = demux::ivf::test_support::ivf_header(b"AV01", 64, 48, 30, 1, 0);
        let path = std::env::temp_dir().join("lumen-cli-ivf-header-only.ivf");
        std::fs::write(&path, &header_only).must();
        let err = match open_source(&SourceSpec::File { path: path.clone() }, None, false, false) {
            Ok(_) => unreachable!("a header-only file must refuse"),
            Err(err) => err,
        };
        std::fs::remove_file(&path).ok();
        assert_eq!(err.stage, commands::Stage::Demux, "{err}");
        assert_eq!(
            err.details.get("code").map(String::as_str),
            Some("demux.ivf.no-frames"),
            "{err}"
        );

        // VP80: outside the v0 delivery scope.
        let mut vp8 = demux::ivf::test_support::ivf_header(b"VP80", 64, 48, 30, 1, 1);
        vp8.extend_from_slice(&demux::ivf::test_support::ivf_frame(0, 8));
        let path = std::env::temp_dir().join("lumen-cli-ivf-vp8.ivf");
        std::fs::write(&path, &vp8).must();
        let err = match open_source(&SourceSpec::File { path: path.clone() }, None, false, false) {
            Ok(_) => unreachable!("a VP80 fourcc must refuse (v0 scope)"),
            Err(err) => err,
        };
        std::fs::remove_file(&path).ok();
        assert_eq!(err.stage, commands::Stage::Demux, "{err}");
        assert_eq!(
            err.details.get("code").map(String::as_str),
            Some("demux.ivf.unsupported-fourcc"),
            "{err}"
        );
    }

    /// A non-unit `scale` (the NTSC-shaped 30000/1001 pair no committed
    /// fixture carries — every real lane is scale 1): the host passes
    /// the container's own ratio through as the tick base, unscaled
    /// and unrenumbered (ffprobe reads this header back as
    /// `time_base = 1001/30000`), and the declared cadence is the
    /// tick's nanoseconds. The ramp payload is not decodable AV1 —
    /// this lane pins the FACTS only, never a decode.
    #[test]
    fn non_unit_scale_timebase_is_the_containers_own_ratio() {
        let mut bytes = demux::ivf::test_support::ivf_header(b"AV01", 64, 48, 30_000, 1001, 1);
        bytes.extend_from_slice(&demux::ivf::test_support::ivf_frame(0, 8));
        let path = std::env::temp_dir().join("lumen-cli-ivf-ntsc.ivf");
        std::fs::write(&path, &bytes).must();
        let mut opened = open(path.clone(), None);
        std::fs::remove_file(&path).ok();
        assert_eq!(opened.facts.container, "ivf");
        let fact = &opened.facts.facts[0];
        assert_eq!(fact.codec, "av1");
        assert_eq!(
            fact.base.as_parts(),
            (1001, 30_000),
            "the header's scale/rate pair, verbatim"
        );
        assert_eq!(fact.dimensions, Some((64, 48)));
        assert_eq!(
            fact.frame_ns,
            Some(33_366_666),
            "one tick in ns (1001e9/30000, floored)"
        );
        // The packet's pts is the container's own integer in that base.
        match opened.demuxer.next_event() {
            Ok(DemuxEvent::Packet(packet)) => {
                let Stamp::Present(pts) = packet.pts else {
                    unreachable!("pts is present");
                };
                assert_eq!(pts.ticks, 0);
                assert_eq!(pts.base.as_parts(), (1001, 30_000));
            }
            Ok(DemuxEvent::EndOfStream) => unreachable!("the crafted frame must emit"),
            Ok(_) => unreachable!("the crafted lane is packet-then-end"),
            Err(err) => unreachable!("{err}"),
        }
    }

    /// Decode every packet of an opened media through the REAL
    /// decoder provider and return the emitted frame count plus the
    /// last frame's coded size. A typed refusal is RETURNED, never
    /// panicked — the tenbit lane pins one by name.
    fn decode_all(
        opened: &mut crate::demux_host::OpenedMedia,
    ) -> Result<(u64, (u32, u32)), EngineError> {
        let facts = Arc::new(opened.facts.clone());
        let provider = HostDecoderProvider::new(Arc::clone(&facts));
        let track = facts.tracks[0].clone();
        let config = DecoderConfig::new(
            track.codec.clone(),
            GenerationId::FIRST,
            ThreadBudget::default(),
            None,
            None,
        );
        let mut decoder = provider
            .decoder_for(&track, &config)
            .unwrap_or_else(|err| unreachable!("decoder_for: {err}"));
        let mut sink = CountingSink(0, (0, 0));
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    let _ = decoder.decode(packet, &mut sink)?;
                }
                Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {}
                Ok(DemuxEvent::EndOfStream) => break,
                Err(err) => unreachable!("demux: {err}"),
            }
        }
        let _ = decoder.flush(&mut sink)?;
        Ok((sink.0, sink.1))
    }
}
