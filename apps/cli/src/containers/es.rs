//! The raw elementary-stream container hosts (card 6.OPW-es — the
//! registry's seventh wired family): the headerless bitstreams the
//! `demux::es` card landed, routed by the probe signatures this unit
//! added (`media-probe/src/signature/es.rs` — before them, every
//! raw-ES file probe-missed).
//!
//! # What this module wires
//!
//! * **`mp3` / `mp2`** → [`RawMp3Demuxer`](demux::es::mp3::
//!   RawMp3Demuxer) (one demuxer, both layers — the probe's
//!   layer-bit discrimination named the lane, the demuxer's own
//!   track descriptor carries it decode-side: L3 → `mp3`,
//!   L2 → `mp2`). These lanes **decode end-to-end** — the MP3/MP2
//!   decoders are wired (`pipeline_host` `mp3`/`mp2` arms).
//! * **`h264` / `hevc`** → [`AnnexbVideoDemuxer`](demux::es::annexb::
//!   AnnexbVideoDemuxer) with the codec the probe's NAL-set
//!   discrimination named. These lanes **open and demux, then
//!   refuse decode honestly** — H.264/HEVC have no software decoder
//!   in this tree (`codecs/src/video/h264/mod.rs`: parsers +
//!   hardware-path seam only), so the provider answers the typed
//!   `cli.codec_unavailable` and an all-video session lands in the
//!   engine's `NoDecodableTrack` naming the container. That honest
//!   refusal IS these lanes' value: packet-exact demux of the
//!   hw-path codecs with no silent wrong decode.
//! * **`flac`** → the honest typed refusal: the `fLaC` probe id is
//!   landed, but **no raw-FLAC demuxer exists** — `crates/demux/
//!   src/es` covers h264/hevc/mp3/aac only (card 5.I3a's slice).
//!   The refusal names exactly that gap; the demuxer card that
//!   lands later (its harvest contract: the 34-byte STREAMINFO body
//!   as `extra_data`, per the FLAC decoder's configure gate) flips
//!   this one arm. The `annexb` family id never reaches the
//!   registry — the probe mints it sub-threshold (ambiguous
//!   NAL shapes), so the probe stage itself refuses with the
//!   family named.
//!
//! # Harvest truth (the §4.3.3 rules, per lane)
//!
//! * **mpeg audio**: codec = the demuxer's layer-honest track
//!   descriptor (never a local copy); base = the track's exact
//!   sample clock (one tick = one sample — the demuxer's own
//!   arithmetic); `pcm` = the channels/rate pair from the first
//!   validated frame header, as the **geometry-only carrier**
//!   (the 6.OPW-mp4 aac convention: the pair sizes the output
//!   limb through `audio_geometry()`; every decode-route consumer
//!   gates on the codec descriptor first — MP3/MP2 are not PCM);
//!   `extra_data` = None (the whole decoder config rides in-band
//!   in every frame header, the ADTS posture); span = None (raw
//!   MPEG audio declares no extent at open — the exact duration
//!   exists only post-walk).
//! * **annexb**: codec = the construction codec's descriptor
//!   (`h264`/`hevc`); base = the demuxer's ffmpeg-documented
//!   1/25 fallback mint (raw Annex-B carries no timing at all);
//!   no pcm, no blob, no dimensions (SPS/VPS geometry is
//!   decode-domain — the landed codecs-side parsers own it), no
//!   extent (stamps are `Missing` by the demuxer's own honesty).
//!
//! # `--start-time` (the honest shapes)
//!
//! Neither ES demuxer implements the `seek` API (both refuse
//! typed — no index exists to map a target onto), so this host
//! sets the skip watermark **without** a seek query: the MPEG-audio
//! watermark is exact (every packet carries a present
//! sample-clock pts; the wrapper drops precisely the frames that
//! predate the target), while the Annex-B watermark judges nothing
//! (pts are `Missing`; the never-drop-on-a-guess rule passes every
//! packet — a raw video stream has no positions to skip to, and
//! the open still succeeds with the honest no-op). In-session
//! native seeks refuse typed through the demuxers'
//! `SeekSupport::None`.

use commands::{CommandError, ErrorCode, Stage};
use demux::es::annexb::{AnnexbCodec, AnnexbVideoDemuxer};
use demux::es::mp3::RawMp3Demuxer;
use media_types::Track;
use plugin_api::{Demuxer, SourceStream};

use crate::demux_host::{
    MediaFacts, OpenOptions, OpenedMedia, SelectDemuxer, TrackFact, wire_error,
};

/// Open a raw MPEG-audio source (the `mp3`/`mp2` probe ids share
/// this host — the layer named the lane, the demuxer is one).
///
/// # Errors
/// Typed [`CommandError`]s (stage `demux`): no usable frame header
/// within the open-scan window, a truncated head ID3v2 tag, or the
/// bounded recovery-budget refusal.
pub(crate) fn open_mpeg_audio(
    stream: Box<dyn SourceStream>,
    opts: &OpenOptions,
    format: &str,
) -> Result<OpenedMedia, CommandError> {
    // 1. Demuxer + the host pre-pass (track list + first-frame facts).
    let mut demuxer = RawMp3Demuxer::new(stream).map_err(|err| wire_error(&err))?;
    let tracks = demuxer.start().map_err(|err| wire_error(&err))?;
    let facts = harvest_mpeg_audio(&demuxer, &tracks, format);
    // 2. The start position: no declared extent to clamp against
    //    (raw MPEG audio's exact duration is walk-derived), and no
    //    seek query — the demuxer's seek API refuses typed (no
    //    index); the watermark below is the exact filter instead
    //    (every packet's pts is a present sample-clock stamp).
    let start_us = opts.start_time_us.unwrap_or(0).max(0);
    let skip_until_ns = (start_us > 0).then(|| start_us.saturating_mul(1000));
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

/// Harvest the track facts from a started MPEG-audio demuxer:
/// exactly one audio track; the codec descriptor and the sample
/// clock are the track's own truth, `head()` the demuxer-side
/// confirmation (the geometry pair rides `pcm` as the
/// geometry-only carrier — see the module docs).
fn harvest_mpeg_audio(demuxer: &RawMp3Demuxer, tracks: &[Track], format: &str) -> MediaFacts {
    // `head()` (the first validated frame header) is the
    // demuxer-side confirmation of the same header the track was
    // minted from: the sample clock's denominator IS the declared
    // rate. Pinned as the harvest's invariant (divergence would
    // mean the track and the accessor disagree about one header —
    // a demuxer bug, never a wiring guess).
    if let Some(head) = demuxer.head()
        && let Some(track) = tracks.first()
    {
        debug_assert_eq!(
            track.timing.base.as_parts().1,
            head.sample_rate,
            "the sample clock is the first frame's declared rate"
        );
    }
    let mut facts = Vec::with_capacity(tracks.len());
    for track in tracks {
        let span_end_ticks = track.timing.span.end.timestamp().ok().map(|ts| ts.ticks);
        // The geometry pair: channels + rate from the first frame
        // header (u16 channels → the pair's u32). NOT a PCM claim —
        // the mp4-aac convention; the descriptor gates decode.
        let geometry = demuxer
            .head()
            .map(|head| (u32::from(head.channels), head.sample_rate));
        facts.push(TrackFact {
            id: track.id,
            kind: track.kind,
            codec: track.codec.as_str().to_string(),
            base: track.timing.base,
            span_end_ticks,
            pcm: geometry,
            extra_data: None,
            frame_ns: None,
            dimensions: None,
        });
    }
    MediaFacts {
        tracks: tracks.to_vec(),
        facts,
        container: format.to_string(),
    }
}

/// Open a raw Annex-B video source (the `h264`/`hevc` probe ids —
/// the probe's NAL-set discrimination already named the codec this
/// demuxer is constructed with; the demuxer's own `AnnexbCodec`
/// parameter is the construction truth).
///
/// # Errors
/// Typed [`CommandError`]s (stage `demux`): no Annex-B sync
/// structure within the open-scan window, an access unit over the
/// memory cap, or the unknown-length/unseekable source refusals.
pub(crate) fn open_annexb(
    codec: AnnexbCodec,
    stream: Box<dyn SourceStream>,
    opts: &OpenOptions,
) -> Result<OpenedMedia, CommandError> {
    let mut demuxer = AnnexbVideoDemuxer::new(codec, stream).map_err(|err| wire_error(&err))?;
    let tracks = demuxer.start().map_err(|err| wire_error(&err))?;
    let facts = harvest_annexb(&tracks, codec);
    // --start-time: the watermark is set for shape-parity with the
    // audio lanes, but every Annex-B packet carries `Missing` pts —
    // the wrapper's never-drop-on-a-guess rule passes them all (the
    // module docs' honest no-op).
    let start_us = opts.start_time_us.unwrap_or(0).max(0);
    let skip_until_ns = (start_us > 0).then(|| start_us.saturating_mul(1000));
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

/// Harvest the track facts from a started Annex-B demuxer: exactly
/// one video track; the codec is the construction codec's own
/// descriptor (mirrored here — the demuxer's `descriptor()` stays
/// crate-private; the two names are the probe ids by construction),
/// the base the demuxer's documented 1/25 fallback mint (raw
/// Annex-B declares no timing — the module never derives a stamp
/// from it), and nothing else exists to harvest at demux level
/// (geometry is the SPS/VPS parser's domain).
fn harvest_annexb(tracks: &[Track], codec: AnnexbCodec) -> MediaFacts {
    let container = match codec {
        AnnexbCodec::H264 => "h264",
        AnnexbCodec::Hevc => "hevc",
    };
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
            frame_ns: None,
            dimensions: None,
        });
    }
    MediaFacts {
        tracks: tracks.to_vec(),
        facts,
        container: container.to_string(),
    }
}

/// The raw-FLAC open: the honest typed refusal (the module docs'
/// gap — the `fLaC` probe id landed with this unit, the demuxer has
/// not; refusing with the gap named beats a probe-miss that says
/// nothing).
///
/// # Errors
/// Always: [`CommandError`] (stage `probe`, code `unsupported`,
/// `format=flac` detail) naming the missing demuxer and the harvest
/// contract the landing card owes.
pub(crate) fn open_flac(
    _stream: Box<dyn SourceStream>,
    _opts: &OpenOptions,
) -> Result<OpenedMedia, CommandError> {
    Err(CommandError::new(
        Stage::Probe,
        ErrorCode::Unsupported,
        "format `flac` probes (the fLaC marker) but the raw-FLAC elementary-stream demuxer \
         has not landed: crates/demux/src/es covers h264/hevc/mp3/aac (card 5.I3a's slice) — \
         the landing card harvests the 34-byte STREAMINFO body as extra_data for the \
         already-wired FLAC decoder and flips this one registry arm",
    )
    .with_detail("format", "flac"))
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
    use plugin_api::DemuxEvent;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    /// The committed codec-crate fixture home (the adts.rs
    /// precedent — real ffmpeg-authored bytes, the
    /// corpus-independent lane).
    fn codec_fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/codecs/tests/fixtures")
            .join(name)
    }

    /// The live corpus media dir (the wav/adts differential
    /// convention: the session default, no `LUMEN_CORPUS`).
    fn corpus_dir() -> Option<PathBuf> {
        if let Ok(dir) = std::env::var("LUMEN_CORPUS") {
            return Some(PathBuf::from(dir));
        }
        let default = PathBuf::from("/tmp/my-project/corpus-v4/media");
        default.is_dir().then_some(default)
    }

    fn corpus_file(dir: &Path, name: &str) -> PathBuf {
        dir.join(name)
    }

    /// Open a source through the REAL CLI open path (canonicalize →
    /// access → probe → this container host).
    fn open(path: PathBuf, start_us: Option<i64>) -> crate::demux_host::OpenedMedia {
        match open_source(&SourceSpec::File { path }, start_us, false, false) {
            Ok(opened) => opened,
            Err(err) => unreachable!("{err}"),
        }
    }

    /// The refused-open twin: returns the typed error instead.
    fn open_refused(path: PathBuf) -> CommandError {
        match open_source(&SourceSpec::File { path }, None, false, false) {
            Ok(_) => unreachable!("must refuse"),
            Err(err) => err,
        }
    }

    /// Drain the demuxer: packet count + keyframe count + a
    /// terminal-error probe.
    fn walk(opened: &mut crate::demux_host::OpenedMedia) -> (u64, Vec<media_types::Packet>) {
        let mut packets = Vec::new();
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(p)) => packets.push(p),
                Ok(DemuxEvent::EndOfStream) => break,
                Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {}
                Err(err) => unreachable!("clean lane demux: {err}"),
            }
        }
        (packets.len() as u64, packets)
    }

    /// The audio sink: counts decoded sample frames and remembers
    /// the negotiated channel count (the adts.rs shape).
    struct CountingSink(u64, u32);
    impl FrameSink for CountingSink {
        fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
            if let Frame::Audio(audio) = frame
                && let Some(frames) = audio.format.frames_in(audio.samples.len())
            {
                self.0 += frames;
                self.1 = audio.format.channels;
            }
            Ok(())
        }
    }

    /// The committed MPEG-audio fixtures open through the real open
    /// path with the recorded facts: one audio track, the demuxer's
    /// layer-honest codec descriptor, the exact sample-clock base,
    /// the geometry pair riding `pcm` (the carrier convention), no
    /// blob (in-band config), no declared extent.
    #[test]
    fn mpeg_audio_fixture_opens_with_expected_facts() {
        for (name, format, codec, rate, channels) in [
            ("mono48.mp3", "mp3", "mp3", 48_000u32, 1u32),
            ("mpeg2_24k.mp3", "mp3", "mp3", 24_000, 2),
            ("mp2_48k.mp2", "mp2", "mp2", 48_000, 2),
            ("mp2_lsf_22k.mp2", "mp2", "mp2", 22_050, 2),
        ] {
            let path = codec_fixture(name);
            assert!(path.is_file(), "fixture missing: {path:?}");
            let opened = open(path, None);
            assert_eq!(opened.facts.container, format, "{name}");
            assert_eq!(opened.facts.tracks.len(), 1, "{name}");
            let track = &opened.facts.tracks[0];
            assert_eq!(track.kind, MediaKind::Audio, "{name}");
            let fact = &opened.facts.facts[0];
            assert_eq!(fact.codec, codec, "{name}");
            assert_eq!(fact.base.as_parts(), (1, rate), "{name}: the sample clock");
            assert_eq!(fact.pcm, Some((channels, rate)), "{name}: geometry pair");
            assert!(fact.extra_data.is_none(), "{name}: in-band config");
            assert!(fact.span_end_ticks.is_none(), "{name}: no extent at open");
            assert_eq!(opened.start_position_ns, 0);
        }
    }

    /// The mpeg-audio fixtures demux AND decode the full stream
    /// through the real open path + provider: packet counts are
    /// ffprobe's own `show_packets` counts. The mp3 decoded counts
    /// are the MEASURED open-path window — whole-frame arithmetic
    /// MINUS the LAME start delay (529 samples, honored from the
    /// Xing/LAME tag); the end padding is not trimmed through the
    /// container lane yet (the codec crate's ratified full-trim
    /// reference for mono48 is 72,000 == ffmpeg; the trim
    /// unification is the mp3 gapless card's job — the OPW-mp4
    /// recorded precedent). The mp2 counts are sample-exact against
    /// ffmpeg's decoded bytes: mp2_48k 145,152 / mp2_lsf 71,424.
    #[test]
    fn mpeg_audio_fixtures_decode_the_full_stream() {
        // (file, format, packets, channels, decoded_frames)
        for (name, format, packets, channels, decoded) in [
            ("mono48.mp3", "mp3", 64u64, 1u32, 73_199u64),
            ("mp2_48k.mp2", "mp2", 63, 2, 72_576),
            ("mp2_lsf_22k.mp2", "mp2", 31, 2, 35_712),
        ] {
            let mut opened = open(codec_fixture(name), None);
            assert_eq!(opened.facts.container, format, "{name}");
            let (count, _) = walk(&mut opened);
            assert_eq!(count, packets, "{name}: ffprobe packet count");
            // The demux walk is single-shot (the adts convention):
            // a fresh open for the decode pass.
            let mut opened = open(codec_fixture(name), None);
            let (frames, negotiated) =
                decode_sink(&mut opened).unwrap_or_else(|err| unreachable!("{name}: {err}"));
            assert_eq!(frames, decoded, "{name}: packets × spf");
            assert_eq!(negotiated, channels, "{name}: negotiated channels");
        }
    }

    /// The h264 corpus lane: opens through the real path, demuxes
    /// ffprobe's exact 26 packets with `Missing` stamps (raw
    /// Annex-B carries no timing), and the provider refuses the
    /// track typed (`cli.codec_unavailable` — no software H.264
    /// decoder; the honest hw-path posture).
    #[test]
    fn h264_corpus_lane_opens_demuxes_and_refuses_decode() {
        let Some(dir) = corpus_dir() else {
            eprintln!("SKIP es corpus: corpus dir absent");
            return;
        };
        let path = corpus_file(&dir, "0019-raw-video.h264");
        if !path.is_file() {
            eprintln!("SKIP es corpus: 0019-raw-video.h264 absent");
            return;
        }
        let mut opened = open(path, None);
        assert_eq!(opened.facts.container, "h264");
        assert_eq!(opened.facts.tracks.len(), 1);
        let track = &opened.facts.tracks[0];
        assert_eq!(track.kind, MediaKind::Video);
        assert_eq!(opened.facts.facts[0].codec, "h264");
        assert_eq!(
            opened.facts.facts[0].base.as_parts(),
            (1, 25),
            "the demuxer's documented fallback mint"
        );
        let (count, packets) = walk(&mut opened);
        assert_eq!(count, 26, "ffprobe -show_packets: 26 packets");
        assert!(
            packets
                .iter()
                .all(|p| p.pts == media_types::time::Stamp::Missing)
        );
        assert_eq!(packets[1].data.len(), 196, "packet 2 size vs ffprobe");
        // The typed decode refusal (the lane's honest value).
        let err = decode_refusal(&opened);
        assert_eq!(err.context.code, "cli.codec_unavailable", "{err}");
    }

    /// The hevc corpus lane: the same shape — 56 packets (ffprobe),
    /// `Missing` stamps, typed codec refusal.
    #[test]
    fn hevc_corpus_lane_opens_demuxes_and_refuses_decode() {
        let Some(dir) = corpus_dir() else {
            eprintln!("SKIP es corpus: corpus dir absent");
            return;
        };
        let path = corpus_file(&dir, "0044-raw-video.265");
        if !path.is_file() {
            eprintln!("SKIP es corpus: 0044-raw-video.265 absent");
            return;
        }
        let file_len = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let mut opened = open(path, None);
        assert_eq!(opened.facts.container, "hevc");
        assert_eq!(opened.facts.facts[0].codec, "hevc");
        let (count, packets) = walk(&mut opened);
        assert_eq!(count, 56, "ffprobe -show_packets: 56 packets");
        assert!(
            packets
                .iter()
                .all(|p| p.pts == media_types::time::Stamp::Missing)
        );
        let total: u64 = packets.iter().map(|p| p.data.len() as u64).sum();
        assert_eq!(total, file_len, "contiguous coverage of the whole file");
        let err = decode_refusal(&opened);
        assert_eq!(err.context.code, "cli.codec_unavailable", "{err}");
    }

    /// The flac lanes: the probe names `flac` (the fLaC marker) and
    /// the registry refuses with the demuxer gap named — an honest,
    /// informative no, never a silent miss.
    #[test]
    fn flac_lanes_refuse_with_the_demuxer_gap_named() {
        let Some(dir) = corpus_dir() else {
            eprintln!("SKIP es corpus: corpus dir absent");
            return;
        };
        for name in ["0046-audio-only.flac", "0106-cover-audio.flac"] {
            let path = corpus_file(&dir, name);
            if !path.is_file() {
                eprintln!("SKIP es corpus: {name} absent");
                continue;
            }
            let err = open_refused(path);
            assert_eq!(err.stage, Stage::Probe, "{name}: {err}");
            assert_eq!(err.code, ErrorCode::Unsupported, "{name}: {err}");
            assert!(
                err.message.contains("format `flac` probes"),
                "{name}: {err}"
            );
            assert!(err.message.contains("raw-FLAC"), "{name}: {err}");
            assert!(
                err.details.get("format").map(String::as_str) == Some("flac"),
                "{name}: {err}"
            );
        }
    }

    /// The extension-lies lane (ffprobe-verified): corpus
    /// `0071-cover-audio.flac` is MP3 audio + an mjpeg cover behind
    /// a 99,682-byte ID3v2 tag in a `.flac` name — the tag exceeds
    /// the 64 KiB probe window, so no frame is visible and the
    /// honest verdict is the typed probe-miss (the demuxer would
    /// play it; the probe cannot name it — recorded gap with the
    /// measured number).
    #[test]
    fn oversized_id3_lane_is_a_typed_probe_miss() {
        let Some(dir) = corpus_dir() else {
            eprintln!("SKIP es corpus: corpus dir absent");
            return;
        };
        let path = corpus_file(&dir, "0071-cover-audio.flac");
        if !path.is_file() {
            eprintln!("SKIP es corpus: 0071-cover-audio.flac absent");
            return;
        }
        let err = open_refused(path);
        assert_eq!(err.stage, Stage::Probe, "{err}");
        assert_eq!(err.code, ErrorCode::Unsupported, "{err}");
        assert!(err.message.contains("probe.no-demuxer"), "{err}");
    }

    /// The corpus mpeg-audio differential (LIVE, this instance):
    /// packet counts == ffprobe; decoded sample frames ==
    /// `packets × spf` (whole-frame decode — the mp3 lanes' ffmpeg
    /// gapless-trim divergence recorded per lane: 0139 44,100 /
    /// 0145 2,304 trimmed windows vs our whole-frame 45,504 /
    /// 3,456; the mp2 lanes are sample-exact: 64,512 / 177,408 ==
    /// ffmpeg's decoded sample counts).
    #[test]
    fn corpus_mpeg_audio_differential() {
        let Some(dir) = corpus_dir() else {
            eprintln!("SKIP es corpus: corpus dir absent");
            return;
        };
        // (name, format, packets, channels, decoded, ffmpeg_frames)
        // The mp3 `decoded` numbers are the MEASURED open-path window:
        // whole-frame arithmetic MINUS the LAME start delay (529
        // samples, honored from the Xing/LAME tag) — the end padding
        // is not yet trimmed through the container lane (the codec
        // crate's ratified full-trim references are smaller: mono48
        // 72,000 == ffmpeg; 0139 44,100). The trim unification is the
        // mp3 gapless card's job — the OPW-mp4 recorded precedent
        // (mp3-in-mov 576-tail divergence), same divergence class.
        for (name, format, packets, channels, decoded, ffmpeg_frames) in [
            (
                "0139-audio-only.mp3",
                "mp3",
                79u64,
                2u32,
                44_975u64,
                44_100u64,
            ),
            ("0145-cover-audio.mp3", "mp3", 6, 2, 2_927, 2_304),
            ("0005-audio-only.mp2", "mp2", 56, 1, 64_512, 64_512),
            ("0119-audio-only.mp2", "mp2", 154, 2, 177_408, 177_408),
        ] {
            let path = corpus_file(&dir, name);
            if !path.is_file() {
                eprintln!("SKIP es corpus: {name} absent");
                continue;
            }
            let mut opened = open(path, None);
            assert_eq!(opened.facts.container, format, "{name}");
            let (count, _) = walk(&mut opened);
            assert_eq!(count, packets, "{name}: ffprobe packet count");
            // Fresh open for the decode pass (the walk is
            // single-shot — the adts convention).
            let mut opened = open(corpus_file(&dir, name), None);
            let (frames, negotiated) =
                decode_sink(&mut opened).unwrap_or_else(|err| unreachable!("{name}: {err}"));
            assert_eq!(frames, decoded, "{name}: whole-frame decode arithmetic");
            assert_eq!(negotiated, channels, "{name}: negotiated channels");
            // The ffmpeg window rides the assertion only where the
            // two windows are the same arithmetic (mp2 lanes);
            // the mp3 lanes' trim divergence is the recorded
            // comment above, not a false equality.
            if name.ends_with(".mp2") {
                assert_eq!(frames, ffmpeg_frames, "{name}: mp2 is sample-exact");
            }
        }
    }

    /// `--start-time` on the mp3 lane: the watermark is exact —
    /// frames whose sample-clock pts predates 1.000 s are dropped
    /// (19 frames of 576 samples @ 11025 Hz: k×576 < 11025 ⇒
    /// k ≤ 19), the walk resumes at the first frame whose pts is
    /// at-or-after the target.
    #[test]
    fn mp3_start_time_watermark_is_exact() {
        let Some(dir) = corpus_dir() else {
            eprintln!("SKIP es corpus: corpus dir absent");
            return;
        };
        let path = corpus_file(&dir, "0139-audio-only.mp3");
        if !path.is_file() {
            eprintln!("SKIP es corpus: 0139-audio-only.mp3 absent");
            return;
        }
        let mut opened = open(path, Some(1_000_000));
        let (count, packets) = walk(&mut opened);
        assert_eq!(count, 59, "79 − 20 pre-target frames (k=0..=19)");
        assert_eq!(opened.start_position_ns, 1_000_000_000);
        // Every surviving packet's pts is at-or-after the target.
        for p in &packets {
            if let media_types::time::Stamp::Present(pts) = p.pts {
                let ns = pts.convert(&media_clock::NANO_BASE);
                assert!(ns.map(|ns| ns.ticks >= 1_000_000_000).unwrap_or(true));
            }
        }
    }

    /// The h264 lane's `--start-time` is the honest no-op: `Missing`
    /// stamps are never dropped on a guess (the module docs).
    #[test]
    fn h264_start_time_is_the_honest_noop() {
        let Some(dir) = corpus_dir() else {
            eprintln!("SKIP es corpus: corpus dir absent");
            return;
        };
        let path = corpus_file(&dir, "0019-raw-video.h264");
        if !path.is_file() {
            eprintln!("SKIP es corpus: 0019-raw-video.h264 absent");
            return;
        }
        let mut opened = open(path, Some(500_000));
        let (count, _) = walk(&mut opened);
        assert_eq!(count, 26, "no stamp is judgeable — nothing dropped");
    }

    // -- helpers -------------------------------------------------------

    /// The typed decode refusal for an all-hw-codec lane.
    fn decode_refusal(opened: &crate::demux_host::OpenedMedia) -> EngineError {
        // A read-only facts clone drives the provider lookup without
        // touching the walked demuxer.
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
        match provider.decoder_for(&track, &config) {
            Ok(_) => unreachable!("h264/hevc refuse decode"),
            Err(err) => err,
        }
    }

    /// The shared decode driver: provider lookup + full-walk decode
    /// through the REAL dispatch (errors returned, never panicked).
    fn decode_sink(opened: &mut crate::demux_host::OpenedMedia) -> Result<(u64, u32), EngineError> {
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
        let mut decoder = provider.decoder_for(&track, &config)?;
        let mut sink = CountingSink(0, 0);
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    let _ = decoder.decode(packet, &mut sink)?;
                }
                Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {}
                Ok(DemuxEvent::EndOfStream) => break,
                Err(err) => return Err(err),
            }
        }
        let _ = decoder.flush(&mut sink)?;
        Ok((sink.0, sink.1))
    }
}
