//! The MATROSKA container host (card 6.OPW-mkv — the registry's
//! seventh wired family, the EBML transport of the Matroska spec:
//! mkv, mka and webm under the one `matroska` probe id): construct
//! the demuxer, run the host pre-pass `start()` (the EBML header →
//! Segment → Info/Tracks/Cues walk; idempotent — the second call
//! answers the cached list, so no start-adapter is needed), harvest
//! every `TrackEntry`'s facts into [`TrackFact`]s, clamp
//! `--start-time` against the declared extent and validate it
//! through the demuxer's pure `seek` query (the Cues landing), wrap
//! in the shared [`SelectDemuxer`](crate::demux_host::SelectDemuxer).
//!
//! The harvest's decoder-config truth (the doc §4.3.3 rule; the
//! demuxer's [`track_entries`](demux::mkv::MkvDemuxer::track_entries)
//! → [`TrackPrivate`](demux::mkv::header::TrackPrivate) is the facts
//! source — the CodecPrivate bytes plus the Audio/Video masters'
//! own declarations, every shape below MEASURED on this corpus
//! instance this session):
//!
//! * **Vorbis `extra_data` = the three-header chain joined
//!   VERBATIM** after stripping the Matroska Xiph lacing — the
//!   private is `[length-byte count][0xff-padded raw lengths]
//!   [headers…]` (measured on `0003`: `02 1e 5d` + 30 + 93 + 3832
//!   bytes), and the OGG host's join format is what the decoder's
//!   [`VorbisHeaders::from_extra_data`] parses. The split is gated
//!   by the headers' own magic (`\x01`/`\x03`/`\x05` `vorbis`): a
//!   lacing that does not land all three magics yields no blob and
//!   the decoder refuses at `configure` — never a corrupt feed.
//! * **FLAC `extra_data` = the 34-byte STREAMINFO** — the private is
//!   `fLaC` + the 4-byte metadata block header (last-flag, type 0,
//!   length 34) + the info (measured on `0127` and the committed
//!   `mkv_h264_flac` fixture, 42 bytes); the block header is parsed
//!   and the type/length are verified before the slice is taken.
//! * **AAC `extra_data` = the bare AudioSpecificConfig, verbatim**
//!   — the card's named honesty check, RESOLVED POSITIVE by
//!   measurement: the A_AAC CodecPrivate in this corpus is the
//!   5-byte ASC itself (measured `12 08 56 e5 00` on `0007`:
//!   AOT 2 = LC — the same bare-ASC shape the MP4/esds harvest
//!   extracts), so the AAC decoder's raw-block ASC mode consumes it
//!   end-to-end. No esds walk exists here — Matroska stores the ASC
//!   directly.
//! * **ALAC `extra_data` = the bare 24-byte ALACSpecificConfig,
//!   verbatim** (measured on `0098`: the private starts with
//!   frameLength `00 00 10 00` = 4096, no atom wrapper — ffmpeg's
//!   matroska muxer writes the bare config where its own mov muxer
//!   writes the atom; the decoder's cookie parser accepts the bare
//!   shape, and re-wraps atom-shaped privates itself if one ever
//!   appears).
//! * **Opus `extra_data` = OpusHead(‖ mapping) verbatim** (measured
//!   on `0078`: the 19-byte RFC 7845 identification header — the
//!   same shape the MP4 host synthesizes from `dOps`); the mkv
//!   private carries no OpusTags and the decoder needs none.
//! * **h264/hevc `extra_data` = the avcC/hvcC config record
//!   verbatim** (measured on `0007`/`0035`: the private starts with
//!   avcC `configurationVersion` 1 — ffmpeg's extradata shape). No
//!   software h264/hevc decoder is dispatched in this build: the
//!   blob is the forward-honest config, and `decoder_for` refuses
//!   the codec typed (`cli.codec_unavailable`) — the hw-path cards
//!   own the seam.
//! * **mpeg4 `extra_data` = the VOS/VOL start-code chain verbatim**
//!   (measured on `0020`: `00 00 01 b0 …`) — the forward-honest
//!   twin of the MP4 host's esds-DSI seeding; the `mpeg4`
//!   descriptor has no decoder row today (typed refusal).
//! * **av1 carries NO blob**: the V_AV1 private is the
//!   sequence-header OBU, but the landed `Av1Decoder` configures
//!   from the in-band OBUs alone (its `configure` opens the engine,
//!   `extra_data` unconsumed) — measured: `0034` decodes all 101
//!   frames through the plain dispatch with zero seeding.
//! * **vp8/vp9/ffv1 carry no blob** (vp8/vp9 privates are empty in
//!   this corpus; ffv1's is the VFW BITMAPINFOHEADER) — vp9 refuses
//!   typed now and the parallel M-vp9 unit owns the decode wiring;
//!   ffv1's raw fourcc descriptor refuses typed (no decoder).
//! * **PCM**: the geometry pair below is the decode route; note the
//!   honest vocabulary gap — the demuxer maps `A_PCM/INT/LIT` to
//!   `pcm_s16le` ONLY when the Audio master declares `BitDepth`, and
//!   ffmpeg's matroska muxer omits it (measured on a generated
//!   pcm_s16le lane: descriptor passes through raw and the provider
//!   refuses typed). mkvmerge-authored lanes that declare BitDepth
//!   map and decode; this corpus instance carries none.
//! * **The audio geometry pair** (channels + rate, the
//!   [`TrackFact::audio_geometry`] limb source) is the Audio
//!   master's own declaration — `Channels` + `SamplingFrequency`
//!   (f64, rounded; a non-finite or implausible value yields no
//!   pair, never a guess). It rides `pcm` for the PCM decode route
//!   and for `aac` (the MP4 host's geometry-only-carrier precedent:
//!   every consumer that treats `pcm` as a decode route gates on
//!   the codec descriptor first). Vorbis/Opus answer through their
//!   blobs' identification headers (the generic sniff); flac/alac/
//!   mp3 lanes open limb-less on the wall clock — the honest
//!   recorded shape until the sniff grows FLAC/ALAC/MPEG-audio
//!   parsers.
//! * **Video dimensions** are the Video master's `PixelWidth`/
//!   `PixelHeight` (the coded box, never display rotation —
//!   `0023-rot-display` stays demuxer-domain). `frame_ns` IS
//!   declared for video tracks: the `DefaultDuration` element (the
//!   AVI track-base precedent — absent for VFR lanes, `None`
//!   honestly).
//! * **Duration is NOT per-track**: the demuxer stamps every track
//!   span end `Missing` at open (Matroska declares no per-track
//!   length — the segment-level `Duration` stays demuxer-side
//!   container truth), so `--start-time` clamps against the honest
//!   MAX/4 bound and the demuxer's pure `seek` query is the real
//!   range gate (out-of-range targets refuse typed — the ADTS
//!   family shape).
//! * **Seek**: Cues → `Fast { indexed: true }` (measured: every
//!   corpus lane but `0083-zeroframe-stream`, which carries no
//!   Cues element and answers `SlowLinear` — asserted per lane).
//! * **Subtitles deselect at open**: the demuxer mints S_TEXT tracks
//!   as `MediaKind::Subtitle` and the shared wrapper's `start`
//!   filters non-A/V kinds (§3 of the wiring doc) — subtitle tracks
//!   never block playback (pinned on the multitrack fixture).
//! * **Chapters/Editions/ordered-chapters**: demuxer-side metadata
//!   only — `MediaFacts` carries no chapter vocabulary (the MP4
//!   host's recorded gap); ordered-chapter navigation is v0 out of
//!   scope. **Attachments** are skipped by the demuxer (no track
//!   minted, no packet). **CodecDelay** (declared on AAC/Opus
//!   lanes) is muxer metadata this host does not consume — the
//!   decoders own their priming/delay accounting.

use commands::CommandError;
use demux::mkv::MkvDemuxer;
use demux::mkv::header::TrackPrivate;
use media_types::packet::SharedBytes;
use media_types::time::{Stamp, TimeSpan, Timestamp};
use media_types::{MediaKind, Track};
use plugin_api::Demuxer;

use crate::demux_host::{
    MediaFacts, OpenOptions, OpenedMedia, SelectDemuxer, TrackFact, wire_error,
};

/// Open an MKV/MKA/WEBM source (the doc §4.2 per-container shape):
/// the probe already named `matroska`; this constructs the demuxer,
/// runs the host pre-pass `start()`, harvests the TrackEntry facts,
/// clamps `--start-time` against the declared duration, validates it
/// through the demuxer's pure `seek` query and wraps the demuxer in
/// the skip/selection wrapper.
///
/// # Errors
/// Typed [`CommandError`]s (stage `demux`): EBML parse failures,
/// truncations, seek-range violations.
pub(crate) fn open(
    stream: Box<dyn plugin_api::SourceStream>,
    opts: &OpenOptions,
) -> Result<OpenedMedia, CommandError> {
    // 1. Demuxer + the host pre-pass (track list + entries).
    let mut demuxer = MkvDemuxer::new(stream).map_err(|err| wire_error(&err))?;
    let tracks = demuxer.start().map_err(|err| wire_error(&err))?;
    let facts = harvest_facts(&demuxer, &tracks);
    // 2. Clamp the start position (doc 05 §2.4: clamping is truth) —
    //    the declared track extent; a segment declaring none uses the
    //    honest MAX/4 bound (the avi convention).
    let duration_us = facts.duration_us().unwrap_or(i64::MAX / 4);
    let start_us = opts.start_time_us.unwrap_or(0).clamp(0, duration_us);
    let mut skip_until_ns = None;
    if start_us > 0 {
        // The pure seek query validates the target against the Cues
        // index before the session exists; the target is
        // nanosecond-based (the engine clock).
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

/// Harvest the per-track facts from a started MKV demuxer: one fact
/// per minted track, the entry found by its ordinal (the L0 `TrackId`
/// the demuxer mints — `TrackEntry::to_track` stamps `TrackId::new
/// (ordinal)`), the codec-config blob per the module-doc truth table,
/// the Audio master's geometry pair, the Video master's coded
/// dimensions and `DefaultDuration` cadence.
fn harvest_facts(demuxer: &MkvDemuxer, tracks: &[Track]) -> MediaFacts {
    let mut facts = Vec::with_capacity(tracks.len());
    for track in tracks {
        let entry = demuxer
            .track_entries()
            .iter()
            .find(|entry| entry.ordinal == track.id.handle());
        let codec = track.codec.as_str();
        let private = entry.map(|entry| &entry.private);
        let extra_data = private.and_then(|private| config_blob(codec, private));
        let pcm = private
            .filter(|_| track.kind == MediaKind::Audio)
            .and_then(audio_pair)
            .filter(|_| codec.starts_with("pcm_") || codec == "aac");
        let dimensions = private
            .filter(|_| track.kind == MediaKind::Video)
            .and_then(dimensions_of);
        let frame_ns = private
            .filter(|_| track.kind == MediaKind::Video)
            .and_then(|private| private.default_duration_ns);
        let span_end_ticks = track.timing.span.end.timestamp().ok().map(|ts| ts.ticks);
        facts.push(TrackFact {
            id: track.id,
            kind: track.kind,
            codec: codec.to_string(),
            base: track.timing.base,
            span_end_ticks,
            pcm,
            extra_data,
            frame_ns,
            dimensions,
        });
    }
    MediaFacts {
        tracks: tracks.to_vec(),
        facts,
        container: "matroska".to_string(),
    }
}

/// The per-codec config blob from one TrackPrivate (the module-doc
/// truth table). An absent CodecPrivate yields no blob for every
/// codec — the decoder's own `configure` refusal stays the honest
/// authority.
fn config_blob(codec: &str, private: &TrackPrivate) -> Option<SharedBytes> {
    let bytes = private.codec_private.as_slice();
    if bytes.is_empty() {
        return None;
    }
    match codec {
        // The Xiph lacing strip + verbatim three-header join.
        "vorbis" => vorbis_chain(bytes),
        // The 34-byte STREAMINFO out of the fLaC-prefixed private.
        "flac" => flac_streaminfo(bytes),
        // The bare ASC / bare cookie / OpusHead / config records:
        // the private IS the ffmpeg-extradata shape for these.
        "aac" | "alac" | "opus" | "h264" | "hevc" | "mpeg4" => {
            Some(SharedBytes::from_vec(bytes.to_vec()))
        }
        // vp8/vp9/ffv1/av1/pcm/mjpeg/subtitles: no blob (module docs).
        _ => None,
    }
}

/// Strip the Matroska Xiph lacing and join the Vorbis headers
/// verbatim. Measured packing (this corpus, ffmpeg 7.1.5): the
/// private is `[length-byte count][length bytes][headers…]` where
/// each length is raw (a 30-byte identification header rides as one
/// `0x1e` byte — NOT the minus-one Ogg page convention) and 0xFF
/// bytes accumulate. The split is gated by the headers' own magic:
/// identification `\x01vorbis`, comment `\x03vorbis`, setup
/// `\x05vorbis` — exactly three headers, or no blob (the decoder
/// refuses at `configure`, never a corrupt feed).
fn vorbis_chain(private: &[u8]) -> Option<SharedBytes> {
    const MAGIC: [&[u8]; 3] = [
        &[0x01, b'v', b'o', b'r', b'b', b'i', b's'],
        &[0x03, b'v', b'o', b'r', b'b', b'i', b's'],
        &[0x05, b'v', b'o', b'r', b'b', b'i', b's'],
    ];
    let count = *private.first()? as usize;
    if count == 0 || count > 8 {
        return None; // a count outside any sane packing
    }
    // The 0xff-padded raw lengths (each group ends at a byte < 0xFF).
    let mut lens: Vec<usize> = Vec::new();
    let mut at = 1usize;
    let mut acc = 0usize;
    let mut read = 0usize;
    while read < count {
        let byte = usize::from(*private.get(at)?);
        at += 1;
        read += 1;
        acc = acc.saturating_add(byte);
        if byte < 0xFF {
            lens.push(acc);
            acc = 0;
        }
    }
    if acc != 0 || lens.len() + 1 != MAGIC.len() {
        return None; // unterminated lacing, or not the 3-header chain
    }
    let mut blob = Vec::with_capacity(private.len());
    let mut boundary = 0usize;
    for len in &lens {
        let header = private.get(at..at.checked_add(*len)?)?;
        if !header.starts_with(MAGIC[boundary]) {
            return None; // the split missed the magic: refuse the blob
        }
        boundary += 1;
        blob.extend_from_slice(header);
        at += *len;
    }
    let setup = private.get(at..)?;
    if !setup.starts_with(MAGIC[boundary]) || setup.len() < MAGIC[boundary].len() {
        return None; // the setup header must carry its own magic
    }
    blob.extend_from_slice(setup);
    Some(SharedBytes::from_vec(blob))
}

/// The 34-byte FLAC STREAMINFO out of the CodecPrivate. Measured
/// packing: `fLaC` + the metadata block header (last-flag|type at
/// byte 4, 24-bit length at 5..8) + the 34-byte info — the type and
/// the declared length are verified before the slice is taken; the
/// bare 34-byte shape (no magic) is the demuxer-agnostic fallback.
fn flac_streaminfo(private: &[u8]) -> Option<SharedBytes> {
    if private.len() == 34 {
        return Some(SharedBytes::from_vec(private.to_vec()));
    }
    if !private.starts_with(b"fLaC") || private.len() < 42 {
        return None;
    }
    let block_type = private[4] & 0x7F;
    let declared =
        (usize::from(private[5]) << 16) | (usize::from(private[6]) << 8) | usize::from(private[7]);
    if block_type != 0 || declared != 34 {
        return None; // not a STREAMINFO block: never a guess
    }
    Some(SharedBytes::from_vec(private[8..42].to_vec()))
}

/// The Audio master's geometry pair: `Channels` + `SamplingFrequency`
/// (f64, rounded). A missing master, a zero/implausible channel
/// count or a non-finite/negative rate yields no pair — the session
/// opens limb-less rather than on a guess.
fn audio_pair(private: &TrackPrivate) -> Option<(u32, u32)> {
    let channels = private.channels?;
    let rate = private.sampling_frequency?;
    if channels == 0 || channels > 64 || !rate.is_finite() || rate <= 0.0 || rate > 4_096_000.0 {
        return None;
    }
    Some((channels as u32, rate.round() as u32))
}

/// The Video master's coded dimensions (`PixelWidth`/`PixelHeight`).
fn dimensions_of(private: &TrackPrivate) -> Option<(u32, u32)> {
    let width = private.pixel_width?;
    let height = private.pixel_height?;
    if width == 0 || height == 0 || width > u64::from(u32::MAX) || height > u64::from(u32::MAX) {
        return None;
    }
    Some((width as u32, height as u32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demux_host::open_source;
    use crate::pipeline_host::HostDecoderProvider;
    use codec_api::{DecoderConfig, FrameSink, ThreadBudget};
    use commands::catalog::types::SourceSpec;
    use media_types::{EngineError, Frame, GenerationId};
    use playback_engine::DecoderProvider;
    use plugin_api::{DemuxEvent, SeekSupport};
    use std::path::PathBuf;
    use std::sync::Arc;

    /// A committed fixture under the root `tests/fixtures`.
    fn fixture(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures")
            .join(relative)
    }

    /// Open a source through the REAL CLI open path.
    fn open(path: PathBuf, start_us: Option<i64>) -> crate::demux_host::OpenedMedia {
        match open_source(&SourceSpec::File { path }, start_us, false, false) {
            Ok(opened) => opened,
            Err(err) => unreachable!("{err}"),
        }
    }

    /// The audio sink: counts decoded sample frames and remembers the
    /// negotiated channel count (the adts.rs shape); video frames
    /// count one each (the mp4.rs shape).
    struct CountingSink(u64, u32);
    impl FrameSink for CountingSink {
        fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
            match frame {
                Frame::Audio(audio) => {
                    if let Some(frames) = audio.format.frames_in(audio.samples.len()) {
                        self.0 += frames;
                        self.1 = audio.format.channels;
                    }
                }
                Frame::Video(_) => {
                    self.0 += 1;
                }
                Frame::Subtitle(_) => {}
            }
            Ok(())
        }
    }

    /// The provider's answer for one track: a configured decoder, or
    /// the typed refusal (pinned by code — never a panic; the
    /// hw-path and 6-channel lanes own the refusal paths).
    fn decoder_for(
        opened: &crate::demux_host::OpenedMedia,
        track: &media_types::Track,
    ) -> Result<Box<dyn codec_api::Decoder>, EngineError> {
        let facts = Arc::new(opened.facts.clone());
        let provider = HostDecoderProvider::new(Arc::clone(&facts));
        let config = DecoderConfig::new(
            track.codec.clone(),
            GenerationId::FIRST,
            ThreadBudget::default(),
            None,
            None,
        );
        provider.decoder_for(track, &config)
    }

    /// The committed h264+aac parent fixture opens through the real
    /// open path with the recorded facts: two tracks, the h264 blob
    /// is the bare avcC config record, the aac blob is the bare
    /// 5-byte ASC (AOT 2 = LC — the honesty check's positive
    /// verdict), the geometry pair is the Audio master's own
    /// declaration, the coded dimensions and the DefaultDuration
    /// cadence are the Video master's, and the extent is declared.
    #[test]
    fn h264_aac_fixture_opens_with_expected_facts() {
        let opened = open(fixture("generated/valid/mkv_h264_aac.mkv"), None);
        assert_eq!(opened.facts.container, "matroska");
        assert_eq!(opened.facts.tracks.len(), 2);
        let [video, audio] = &opened.facts.tracks[..] else {
            unreachable!("ffprobe: h264 then aac");
        };
        assert_eq!(video.codec.as_str(), "h264");
        assert_eq!(audio.codec.as_str(), "aac");
        let vf = &opened.facts.facts[0];
        let af = &opened.facts.facts[1];
        // h264: the avcC config record (configurationVersion 1),
        // coded dimensions, the declared 10 fps cadence.
        let avcc = vf
            .extra_data
            .as_ref()
            .unwrap_or_else(|| unreachable!("avcC harvested"));
        assert_eq!(avcc.as_slice()[0], 1, "avcC configurationVersion");
        assert_eq!(vf.dimensions, Some((160, 120)), "the Video master");
        assert_eq!(vf.frame_ns, Some(100_000_000), "DefaultDuration 10 fps");
        assert!(vf.pcm.is_none());
        // aac: the bare ASC (`asc[0] >> 3` = 2 = LC — the MP4/esds
        // shape, stored directly by Matroska), the Audio master's
        // pair flowing through the limb lookup.
        let asc = af
            .extra_data
            .as_ref()
            .unwrap_or_else(|| unreachable!("ASC harvested"));
        let asc = asc.as_slice();
        assert_eq!(asc.len(), 5, "the measured 5-byte private");
        assert_eq!(asc[0] >> 3, 2, "the ASC's object type is LC");
        assert_eq!(af.pcm, Some((1, 44_100)), "the Audio master's pair");
        assert_eq!(af.audio_geometry(), Some((1, 44_100)));
        // Matroska declares no per-track length at open (the span end
        // is `Missing` by demuxer design — the segment Duration is
        // container-level truth); the start clamp falls to the honest
        // MAX/4 bound and the seek query is the range gate.
        assert_eq!(opened.facts.duration_us(), None);
        assert_eq!(opened.start_position_ns, 0);
    }

    /// The h264+FLAC parent fixture: the flac blob is the 34-byte
    /// STREAMINFO out of the fLaC-prefixed private, the track is
    /// limb-less (the geometry sniff has no FLAC parser — the honest
    /// recorded shape), and the full stream decodes through the real
    /// provider: 20 packets → 88 200 sample frames (2 s at 44.1 kHz,
    /// ffmpeg's own count for the fixture).
    #[test]
    fn flac_fixture_opens_and_decodes_the_full_stream() {
        let mut opened = open(fixture("generated/valid/mkv_h264_flac.mkv"), None);
        assert_eq!(opened.facts.container, "matroska");
        assert_eq!(opened.facts.facts[1].codec, "flac");
        let streaminfo = opened.facts.facts[1]
            .extra_data
            .as_ref()
            .unwrap_or_else(|| unreachable!("STREAMINFO harvested"));
        assert_eq!(streaminfo.as_slice().len(), 34, "the bare STREAMINFO");
        assert!(opened.facts.facts[1].pcm.is_none());
        assert_eq!(
            opened.facts.facts[1].audio_geometry(),
            None,
            "no FLAC sniff: the limb-less honest shape"
        );
        let track = opened.facts.tracks[1].clone();
        let mut decoder =
            decoder_for(&opened, &track).unwrap_or_else(|err| unreachable!("flac decodes: {err}"));
        let mut sink = CountingSink(0, 0);
        let mut packets = 0u64;
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    if packet.track == track.id {
                        packets += 1;
                        let _ = decoder
                            .decode(packet, &mut sink)
                            .unwrap_or_else(|err| unreachable!("decode: {err}"));
                    }
                }
                Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {}
                Ok(DemuxEvent::EndOfStream) => break,
                Err(err) => unreachable!("demux: {err}"),
            }
        }
        let _ = decoder
            .flush(&mut sink)
            .unwrap_or_else(|err| unreachable!("flush: {err}"));
        assert_eq!(packets, 20, "ffprobe's packet count");
        assert_eq!(sink.0, 88_200, "2 s at 44.1 kHz — ffmpeg's own extent");
        assert_eq!(sink.1, 1, "the negotiated channel count");
    }

    /// The multitrack fixture (h264 + aac + flac + subrip): the
    /// subtitle track is minted as a track (facts carry all four,
    /// the demuxer emits its 2 packets) but DESELECTS at open — the
    /// wrapper's `start` answers the A/V subset only (§3 of the
    /// wiring doc: subtitle tracks never block playback).
    #[test]
    fn multitrack_fixture_deselects_subtitles_at_open() {
        let mut opened = open(fixture("generated/valid/mkv_multitrack.mkv"), None);
        assert_eq!(opened.facts.tracks.len(), 4, "ffprobe: v + a + a + subrip");
        assert_eq!(opened.facts.facts[3].codec, "subrip");
        let engine_tracks = opened
            .demuxer
            .start()
            .unwrap_or_else(|err| unreachable!("wrapper start: {err}"));
        assert_eq!(engine_tracks.len(), 3, "the subtitle kind deselects");
        assert!(
            engine_tracks
                .iter()
                .all(|track| track.kind != MediaKind::Subtitle)
        );
        // The walk: 20 h264 + 88 aac + 20 flac + 2 subrip packets
        // (ffprobe's counts — the subtitle packets flow through the
        // demuxer; the ENGINE would not wire a limb for them).
        let mut counts = [0u64; 4];
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => counts[packet.track.handle() as usize] += 1,
                Ok(DemuxEvent::EndOfStream) => break,
                Ok(_) => {}
                Err(err) => unreachable!("demux: {err}"),
            }
        }
        assert_eq!(counts, [20, 88, 20, 2]);
    }

    /// The WebM branch (the same probe id): vp9 refuses typed at the
    /// provider (the parallel M-vp9 unit owns the decode), the opus
    /// track decodes the full stream through the private's OpusHead
    /// (101 packets → 96 648 sample frames at 48 kHz mono, ffmpeg's
    /// own extent for the fixture).
    #[test]
    fn webm_vp9_refuses_and_opus_decodes_from_the_private_head() {
        let mut opened = open(fixture("generated/valid/webm_vp9_opus.webm"), None);
        assert_eq!(opened.facts.container, "matroska");
        let [video, audio] = &opened.facts.tracks[..] else {
            unreachable!("ffprobe: vp9 then opus");
        };
        assert_eq!(video.codec.as_str(), "vp9");
        assert_eq!(audio.codec.as_str(), "opus");
        let err = match decoder_for(&opened, video) {
            Ok(_) => unreachable!("no vp9 decoder is dispatched in this build"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "cli.codec_unavailable", "{err}");
        // The head: the 19-byte RFC 7845 identification header the
        // private carries verbatim, and the geometry it answers.
        let head = opened.facts.facts[1]
            .extra_data
            .as_ref()
            .unwrap_or_else(|| unreachable!("OpusHead harvested"));
        assert!(head.as_slice().starts_with(b"OpusHead"));
        assert_eq!(opened.facts.facts[1].audio_geometry(), Some((1, 48_000)));
        let mut decoder =
            decoder_for(&opened, audio).unwrap_or_else(|err| unreachable!("opus decodes: {err}"));
        let mut sink = CountingSink(0, 0);
        let mut packets = 0u64;
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    if packet.track == audio.id {
                        packets += 1;
                        let _ = decoder
                            .decode(packet, &mut sink)
                            .unwrap_or_else(|err| unreachable!("decode: {err}"));
                    }
                }
                Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {}
                Ok(DemuxEvent::EndOfStream) => break,
                Err(err) => unreachable!("demux: {err}"),
            }
        }
        let _ = decoder
            .flush(&mut sink)
            .unwrap_or_else(|err| unreachable!("flush: {err}"));
        assert_eq!(packets, 101, "ffprobe's packet count");
        assert_eq!(sink.0, 96_648, "the ffmpeg-matched extent");
        assert_eq!(sink.1, 1, "the negotiated channel count");
    }

    /// `--start-time` through the pure seek query: an in-media target
    /// lands every surviving packet at/after it (the wrapper's skip
    /// watermark), and the Cues-backed seekability truth forwards
    /// through the wrapper — `Fast { indexed: true }`.
    #[test]
    fn start_time_skips_packets_and_reports_indexed_seek() {
        let Some(dir) = corpus_dir() else {
            skip_loudly();
            return;
        };
        let mut mid = open(dir.join("0007-chapter-bomb.mkv"), Some(500_000));
        assert_eq!(mid.start_position_ns, 500_000_000);
        assert_eq!(
            mid.demuxer.seekability(),
            SeekSupport::Fast { indexed: true }
        );
        let mut seen = 0u32;
        loop {
            match mid.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    seen += 1;
                    if let Stamp::Present(pts) = packet.pts {
                        let ns = pts
                            .convert(&media_clock::NANO_BASE)
                            .unwrap_or_else(|e| unreachable!("{e:?}"));
                        assert!(ns.ticks >= 500_000_000, "early packet: {}", ns.ticks);
                    }
                }
                Ok(DemuxEvent::EndOfStream) => break,
                Ok(_) => {}
                Err(err) => unreachable!("demux: {err}"),
            }
        }
        assert!(seen > 0, "an in-media target keeps the tail");
    }

    /// The six-channel AAC lane (`0017-meta-bomb-av`): the geometry
    /// pair is the Audio master's declaration and flows to the limb;
    /// the AAC ENGINE refuses the channel count at configure
    /// (mono/stereo only — 5.C2a's recorded partial, the honest no,
    /// never a silent stereo downmix).
    #[test]
    fn six_channel_aac_geometry_flows_but_the_decoder_refuses() {
        let Some(dir) = corpus_dir() else {
            skip_loudly();
            return;
        };
        let opened = open(dir.join("0017-meta-bomb-av.mkv"), None);
        let aac = &opened.facts.facts[1];
        assert_eq!(aac.codec, "aac");
        assert_eq!(aac.audio_geometry(), Some((6, 32_000)));
        let track = opened.facts.tracks[1].clone();
        let err = match decoder_for(&opened, &track) {
            Ok(_) => unreachable!("the mono/stereo engine must refuse 6 channels"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "aac.engine.init", "{err}");
    }

    /// The Vorbis lacing strip on hand-built privates (the measured
    /// packing): the three headers join verbatim, the magics gate
    /// the split, and a lacing that misses them yields no blob.
    #[test]
    fn vorbis_lacing_strip_joins_the_measured_packing() {
        // count=2, lengths 8 and 8 (raw), then the three headers
        // (each a full 7-byte magic + one payload byte).
        let id = [0x01, b'v', b'o', b'r', b'b', b'i', b's', 0x00];
        let comment = [0x03, b'v', b'o', b'r', b'b', b'i', b's', 0x00];
        let setup = [0x05, b'v', b'o', b'r', b'b', b'i', b's', 0xAA];
        let mut private = vec![0x02, 0x08, 0x08];
        private.extend_from_slice(&id);
        private.extend_from_slice(&comment);
        private.extend_from_slice(&setup);
        let chain = vorbis_chain(&private).unwrap_or_else(|| unreachable!("the packing walks"));
        let bytes = chain.as_slice();
        assert_eq!(&bytes[..8], &id);
        assert_eq!(&bytes[8..16], &comment);
        assert_eq!(&bytes[16..], &setup);
        assert_eq!(bytes.len(), 24, "the verbatim join");
        // A wrong length (comment declared 9) misses the setup magic:
        // no blob, the decoder's configure refusal stays the answer.
        let mut wrong = vec![0x02, 0x08, 0x09];
        wrong.extend_from_slice(&private[3..]);
        assert!(vorbis_chain(&wrong).is_none(), "the magic gate refuses");
        // 0xff padding accumulates: count=3 length bytes (0xFF, 0x01,
        // 0x08) = a 256-byte id header + an 8-byte comment.
        let mut padded = vec![0x03, 0xFF, 0x01, 0x08];
        padded.extend_from_slice(&id);
        padded.extend(std::iter::repeat_n(0x00, 248));
        padded.extend_from_slice(&comment);
        padded.extend_from_slice(&setup);
        let chain = vorbis_chain(&padded).unwrap_or_else(|| unreachable!("0xff accumulates"));
        assert_eq!(chain.as_slice().len(), 256 + 8 + 8);
        // An unterminated lacing (count 2, one 0xff, no terminator).
        assert!(vorbis_chain(&[0x02, 0xFF]).is_none());
    }

    /// The STREAMINFO extraction on hand-built privates: the
    /// fLaC-prefixed shape with a verified block header, the bare
    /// 34-byte fallback, and a wrong block type/length refused.
    #[test]
    fn flac_streaminfo_extracts_the_verified_block() {
        let mut private = Vec::new();
        private.extend_from_slice(b"fLaC");
        private.extend_from_slice(&[0x80, 0x00, 0x00, 0x22]); // last|type 0, len 34
        private.extend(std::iter::repeat_n(0x11, 34));
        let info = flac_streaminfo(&private).unwrap_or_else(|| unreachable!("the shape walks"));
        assert_eq!(info.as_slice().len(), 34);
        assert!(info.as_slice().iter().all(|b| *b == 0x11));
        // A PADDING block (type 4) or a lying length: no blob.
        let mut wrong = Vec::new();
        wrong.extend_from_slice(b"fLaC");
        wrong.extend_from_slice(&[0x80, 0x04, 0x00, 0x22]);
        wrong.extend(std::iter::repeat_n(0, 34));
        assert!(flac_streaminfo(&wrong).is_none(), "wrong block type");
        let mut lying = Vec::new();
        lying.extend_from_slice(b"fLaC");
        lying.extend_from_slice(&[0x80, 0x00, 0x00, 0x20]); // declares 32
        lying.extend(std::iter::repeat_n(0, 34));
        assert!(flac_streaminfo(&lying).is_none(), "lying length");
        // The bare 34-byte shape rides verbatim.
        let bare = flac_streaminfo(&[0x7Fu8; 34]).unwrap_or_else(|| unreachable!("bare"));
        assert_eq!(bare.as_slice().len(), 34);
    }

    // -----------------------------------------------------------------
    // The corpus differential (the user's standing mandate): every
    // Matroska lane of corpus-v4 through the REAL open path — track
    // codecs and packet counts against the same ffmpeg-measured
    // truth as crates/demux/tests/mkv_corpus.rs (ffprobe
    // -count_packets, this instance, measured this session), decode
    // counts where a decoder is dispatched (aac ×1024 sample frames
    // per packet; the mp3/vorbis/opus/alac/flac engines' own
    // emission windows), and the honest typed refusal where none is
    // (h264/hevc/mpeg4/vp8/vp9/ffv1/ac3 — the hw-path and unlanded
    // codec cards own those seams; the 6-channel aac lanes hit the
    // AAC engine's own mono/stereo boundary, 5.C2a's recorded
    // partial).
    // -----------------------------------------------------------------

    /// One corpus lane's expected shape (the ffmpeg-measured truth:
    /// ffprobe codec + packet count per stream, decoded extents
    /// where a decoder exists).
    struct Lane {
        file: &'static str,
        /// (codec, packets) per track, in track order (subtitles
        /// included — they are facts, deselected at open).
        streams: &'static [(&'static str, u64)],
        /// Decoded sample frames for the FIRST decodable audio
        /// track, when one exists.
        audio_frames: Option<u64>,
        /// Decoded frames for the FIRST decodable video track, when
        /// one exists.
        video_frames: Option<u64>,
        /// The seekability truth (Cues → Fast; `0083` carries no
        /// Cues element and answers SlowLinear).
        seek_fast: bool,
    }

    const TRUTHS: [Lane; 34] = [
        // 0002: ac3-only — every track refuses typed (no ac3 decoder).
        Lane {
            file: "0002-audio-only.mkv",
            streams: &[("ac3", 84)],
            audio_frames: None,
            video_frames: None,
            seek_fast: true,
        },
        // 0003: h264 + mp3 + vorbis + 2 ass. The first decodable
        // audio is the mp3 track (26 543 frames; ffmpeg's trimmed
        // window is 27 072 — the whole-frame emission divergence the
        // mp3 card records). The vorbis lane has its own test below.
        Lane {
            file: "0003-multi-av.mkv",
            streams: &[
                ("h264", 6),
                ("mp3", 47),
                ("vorbis", 38),
                ("ass", 0),
                ("ass", 0),
            ],
            audio_frames: Some(26_543),
            video_frames: None,
            seek_fast: true,
        },
        // 0007: the chapter bomb — 50 chapters, demuxer-side only
        // (no host vocabulary, the recorded gap); aac decodes exact.
        Lane {
            file: "0007-chapter-bomb.mkv",
            streams: &[("h264", 60), ("aac", 153)],
            audio_frames: Some(153 * 1024),
            video_frames: None,
            seek_fast: true,
        },
        // 0009: the parameter midchange (h264 SPS switch mid-file) —
        // demuxes whole, decode refuses typed.
        Lane {
            file: "0009-param-midchange.mkv",
            streams: &[("h264", 51)],
            audio_frames: None,
            video_frames: None,
            seek_fast: true,
        },
        Lane {
            file: "0010-multi-av.mkv",
            streams: &[
                ("h264", 91),
                ("ac3", 34),
                ("aac", 27),
                ("mp3", 47),
                ("subrip", 1),
                ("ass", 0),
            ],
            audio_frames: Some(27 * 1024),
            video_frames: None,
            seek_fast: true,
        },
        Lane {
            file: "0014-muxed-av.mkv",
            streams: &[("h264", 185), ("mp3", 70)],
            audio_frames: Some(39_791),
            video_frames: None,
            seek_fast: true,
        },
        // 0017: the metadata bomb — 6-channel aac (the mono/stereo
        // engine gate refuses it typed; the geometry test above).
        Lane {
            file: "0017-meta-bomb-av.mkv",
            streams: &[("h264", 505), ("aac", 67)],
            audio_frames: None,
            video_frames: None,
            seek_fast: true,
        },
        Lane {
            file: "0020-drift-av.mkv",
            streams: &[("mpeg4", 2324), ("aac", 3638)],
            audio_frames: Some(3638 * 1024),
            video_frames: None,
            seek_fast: true,
        },
        Lane {
            file: "0023-rot-display.mkv",
            streams: &[("h264", 33)],
            audio_frames: None,
            video_frames: None,
            seek_fast: true,
        },
        // 0024: three tracks — h264 + vp9 (both refuse typed; vp9
        // decode arrives with the parallel M-vp9 unit) + 6-channel
        // aac (the engine gate).
        Lane {
            file: "0024-multivideo-av.mkv",
            streams: &[("h264", 95), ("vp9", 119), ("aac", 126)],
            audio_frames: None,
            video_frames: None,
            seek_fast: true,
        },
        // 0034: av1 — decodes through the landed in-band-OBUs decoder
        // (101 frames; no blob needed, the module-doc measurement).
        Lane {
            file: "0034-mkv-video.mkv",
            streams: &[("av1", 101)],
            audio_frames: None,
            video_frames: Some(101),
            seek_fast: true,
        },
        Lane {
            file: "0035-mkv-video.mkv",
            streams: &[("hevc", 169)],
            audio_frames: None,
            video_frames: None,
            seek_fast: true,
        },
        Lane {
            file: "0043-drift-av.mkv",
            streams: &[("h264", 2930), ("aac", 4208)],
            audio_frames: Some(4208 * 1024),
            video_frames: None,
            seek_fast: true,
        },
        Lane {
            file: "0053-mkv-video.mkv",
            streams: &[("h264", 61)],
            audio_frames: None,
            video_frames: None,
            seek_fast: true,
        },
        Lane {
            file: "0057-mkv-video.mkv",
            streams: &[("h264", 200)],
            audio_frames: None,
            video_frames: None,
            seek_fast: true,
        },
        Lane {
            file: "0063-multi-av.mkv",
            streams: &[("h264", 48), ("aac", 35), ("mp3", 47), ("ass", 0)],
            audio_frames: Some(35 * 1024),
            video_frames: None,
            seek_fast: true,
        },
        Lane {
            file: "0066-odd-dims.mkv",
            streams: &[("mpeg4", 27)],
            audio_frames: None,
            video_frames: None,
            seek_fast: true,
        },
        Lane {
            file: "0070-drift-av.mkv",
            streams: &[("mpeg4", 1705), ("aac", 3332)],
            audio_frames: Some(3332 * 1024),
            video_frames: None,
            seek_fast: true,
        },
        // 0075: vp9-only (the M-vp9 unit's lane).
        Lane {
            file: "0075-mkv-video.mkv",
            streams: &[("vp9", 76)],
            audio_frames: None,
            video_frames: None,
            seek_fast: true,
        },
        // 0078: vfr vp9 + opus — the opus decodes from the private
        // head (173 448 frames; ffmpeg's trimmed window is 172 800
        // — the 648-sample granule tail the ogg card records).
        Lane {
            file: "0078-vfr-av.mkv",
            streams: &[("vp9", 19), ("opus", 181)],
            audio_frames: Some(173_448),
            video_frames: None,
            seek_fast: true,
        },
        // 0083: the zero-frame video stream (0 packets — ffprobe's
        // N/A) + aac; the segment carries NO Cues element: the
        // seekability truth is SlowLinear (asserted here, honestly).
        Lane {
            file: "0083-zeroframe-stream.mkv",
            streams: &[("h264", 0), ("aac", 89)],
            audio_frames: Some(89 * 1024),
            video_frames: None,
            seek_fast: false,
        },
        Lane {
            file: "0088-fps-extremes.mkv",
            streams: &[("mpeg4", 8)],
            audio_frames: None,
            video_frames: None,
            seek_fast: true,
        },
        Lane {
            file: "0096-mkv-video.mkv",
            streams: &[("h264", 96)],
            audio_frames: None,
            video_frames: None,
            seek_fast: true,
        },
        // 0098: A_ALAC — the bare 24-byte cookie decodes exact
        // (88 200 = 4 s at 22.05 kHz × 3 channels; ffmpeg's own).
        Lane {
            file: "0098-audio-only.mka",
            streams: &[("alac", 22)],
            audio_frames: Some(88_200),
            video_frames: None,
            seek_fast: true,
        },
        // 0103: the ffv1 fourcc lane (odd 11×3 dims) — raw
        // descriptor, typed refusal.
        Lane {
            file: "0103-odd-dims.mkv",
            streams: &[("ffv1", 15)],
            audio_frames: None,
            video_frames: None,
            seek_fast: true,
        },
        Lane {
            file: "0120-vbv-burst.mkv",
            streams: &[("h264", 73)],
            audio_frames: None,
            video_frames: None,
            seek_fast: true,
        },
        // 0121: 6-channel aac + ac3 — both refuse (the engine gate
        // and the unlanded ac3 decoder).
        Lane {
            file: "0121-multi-av.mkv",
            streams: &[("h264", 8), ("aac", 45), ("ac3", 29)],
            audio_frames: None,
            video_frames: None,
            seek_fast: true,
        },
        // 0126: the WebM vp8 lane (no vp8 decoder — typed refusal).
        Lane {
            file: "0126-webm-video.webm",
            streams: &[("vp8", 41)],
            audio_frames: None,
            video_frames: None,
            seek_fast: true,
        },
        // 0127: A_FLAC — the STREAMINFO decode, exact vs ffmpeg
        // (28 489 frames; the sr-chaos rate switches are the
        // decoder's own domain).
        Lane {
            file: "0127-sr-chaos.mka",
            streams: &[("flac", 7)],
            audio_frames: Some(28_489),
            video_frames: None,
            seek_fast: true,
        },
        Lane {
            file: "0133-dynres-video.mkv",
            streams: &[("h264", 298)],
            audio_frames: None,
            video_frames: None,
            seek_fast: true,
        },
        Lane {
            file: "0136-mkv-video.mkv",
            streams: &[("hevc", 192)],
            audio_frames: None,
            video_frames: None,
            seek_fast: true,
        },
        Lane {
            file: "0141-mkv-video.mkv",
            streams: &[("vp9", 52)],
            audio_frames: None,
            video_frames: None,
            seek_fast: true,
        },
        Lane {
            file: "0147-mkv-video.mkv",
            streams: &[("ffv1", 669)],
            audio_frames: None,
            video_frames: None,
            seek_fast: true,
        },
        Lane {
            file: "0149-dynres-video.mkv",
            streams: &[("h264", 136)],
            audio_frames: None,
            video_frames: None,
            seek_fast: true,
        },
    ];

    /// The corpus media dir (the demux suite's convention — `None`
    /// means the loud skip).
    fn corpus_dir() -> Option<PathBuf> {
        if let Ok(dir) = std::env::var("LUMEN_CORPUS") {
            return Some(PathBuf::from(dir));
        }
        let default = PathBuf::from("/tmp/my-project/corpus-v4/media");
        default.is_dir().then_some(default)
    }

    fn skip_loudly() {
        eprintln!(
            "corpus(mkv-open): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \
             — SKIP (the differential table for this instance is in the worklog)"
        );
    }

    #[test]
    fn corpus_matroska_lanes_match_the_ffmpeg_truth() {
        let Some(dir) = corpus_dir() else {
            skip_loudly();
            return;
        };
        for lane in &TRUTHS {
            let name = lane.file;
            let mut opened = open(dir.join(name), None);
            assert_eq!(opened.facts.container, "matroska", "{name}");
            assert_eq!(opened.facts.tracks.len(), lane.streams.len(), "{name}");
            for (fact, (codec, _)) in opened.facts.facts.iter().zip(lane.streams) {
                assert_eq!(fact.codec, *codec, "{name}: codec");
            }
            // The seekability truth forwards through the wrapper.
            let seek = if lane.seek_fast {
                SeekSupport::Fast { indexed: true }
            } else {
                SeekSupport::SlowLinear
            };
            assert_eq!(opened.demuxer.seekability(), seek, "{name}");
            // Pass 1 — the packet walk (per-track counts).
            let mut counts = vec![0u64; lane.streams.len()];
            loop {
                match opened.demuxer.next_event() {
                    Ok(DemuxEvent::Packet(p)) => counts[p.track.handle() as usize] += 1,
                    Ok(DemuxEvent::EndOfStream) => break,
                    Ok(_) => {}
                    Err(err) => unreachable!("{name}: {err}"),
                }
            }
            for (i, (_, want)) in lane.streams.iter().enumerate() {
                assert_eq!(counts[i], *want, "{name}: track {i} packets");
            }
            // Pass 2 — the decodes (a fresh open: the walk is
            // single-shot) on the FIRST decodable track of each kind.
            let mut opened = open(dir.join(name), None);
            let mut audio: Option<(usize, Box<dyn codec_api::Decoder>)> = None;
            let mut video: Option<(usize, Box<dyn codec_api::Decoder>)> = None;
            for (i, track) in opened.facts.tracks.iter().enumerate() {
                match decoder_for(&opened, track) {
                    Ok(decoder) => match track.kind {
                        MediaKind::Audio if audio.is_none() => audio = Some((i, decoder)),
                        MediaKind::Video if video.is_none() => video = Some((i, decoder)),
                        _ => {}
                    },
                    Err(err) => {
                        // The honest typed refusals: the hw-path and
                        // unlanded codecs (h264/hevc/mpeg4/vp8/vp9/
                        // ffv1/ac3, the subtitle vocabulary) and the
                        // AAC engine's mono/stereo boundary for the
                        // 6-channel lanes (5.C2a's recorded partial).
                        assert!(
                            matches!(
                                err.context.code.as_str(),
                                "cli.codec_unavailable" | "aac.engine.init"
                            ),
                            "{name}: track {i} ({}) refusal shape: {}",
                            track.codec,
                            err.context.code
                        );
                    }
                }
            }
            // Separate sinks: audio counts sample frames, video one
            // per decoded frame (a lane with both never mixes them).
            let mut audio_sink = CountingSink(0, 0);
            let mut video_sink = CountingSink(0, 0);
            loop {
                match opened.demuxer.next_event() {
                    Ok(DemuxEvent::Packet(p)) => {
                        let idx = p.track.handle() as usize;
                        if let Some((_, decoder)) = audio.as_mut().filter(|(i, _)| *i == idx) {
                            let _ = decoder
                                .decode(p, &mut audio_sink)
                                .unwrap_or_else(|e| unreachable!("{name}: audio decode: {e}"));
                        } else if let Some((_, decoder)) = video.as_mut().filter(|(i, _)| *i == idx)
                        {
                            let _ = decoder
                                .decode(p, &mut video_sink)
                                .unwrap_or_else(|e| unreachable!("{name}: video decode: {e}"));
                        }
                    }
                    Ok(DemuxEvent::EndOfStream) => break,
                    Ok(_) => {}
                    Err(err) => unreachable!("{name}: {err}"),
                }
            }
            if let Some((_, ref mut decoder)) = audio {
                let _ = decoder
                    .flush(&mut audio_sink)
                    .unwrap_or_else(|e| unreachable!("{name}: audio flush: {e}"));
            }
            if let Some((_, ref mut decoder)) = video {
                let _ = decoder
                    .flush(&mut video_sink)
                    .unwrap_or_else(|e| unreachable!("{name}: video flush: {e}"));
            }
            if let Some(want) = lane.audio_frames {
                assert_eq!(audio_sink.0, want, "{name}: decoded audio sample frames");
            }
            if let Some(want) = lane.video_frames {
                assert_eq!(video_sink.0, want, "{name}: decoded video frames");
            }
            // Every lane WITHOUT a decodable track of a kind must have
            // refused it typed above — the cross-check that the
            // Option fields and the refusal assertions agree.
            if lane.audio_frames.is_none() {
                assert!(audio.is_none(), "{name}: no audio decoder expected");
            }
            if lane.video_frames.is_none() {
                assert!(video.is_none(), "{name}: no video decoder expected");
            }
        }
    }

    /// The Vorbis lane of `0003` (the corpus's only A_VORBIS): the
    /// lacing strip seeds the chain (the blob's own magics —
    /// identification, comment, setup — and the geometry the generic
    /// sniff answers), and the full stream decodes through the real
    /// provider: 38 packets → 37 568 sample frames (ffmpeg's trimmed
    /// window is 37 440 — the +128 raw-emission window the ogg card
    /// records for the same engine).
    #[test]
    fn vorbis_lane_decodes_the_full_stream_from_the_laced_private() {
        let Some(dir) = corpus_dir() else {
            skip_loudly();
            return;
        };
        let mut opened = open(dir.join("0003-multi-av.mkv"), None);
        let fact = &opened.facts.facts[2];
        assert_eq!(fact.codec, "vorbis");
        let chain = fact
            .extra_data
            .as_ref()
            .unwrap_or_else(|| unreachable!("the chain harvested"));
        let chain = chain.as_slice();
        assert!(chain.starts_with(&[0x01, b'v', b'o', b'r', b'b', b'i', b's']));
        assert!(
            chain
                .windows(7)
                .any(|w| w == [0x03, b'v', b'o', b'r', b'b', b'i', b's']),
            "the comment header is in the chain"
        );
        assert!(
            chain
                .windows(7)
                .any(|w| w == [0x05, b'v', b'o', b'r', b'b', b'i', b's']),
            "the setup header is in the chain"
        );
        assert!(chain.len() > 3_000, "the codebooks ride the setup header");
        assert_eq!(
            fact.audio_geometry(),
            Some((2, 32_000)),
            "the id-header sniff"
        );
        let track = opened.facts.tracks[2].clone();
        let mut decoder = decoder_for(&opened, &track)
            .unwrap_or_else(|err| unreachable!("vorbis decodes: {err}"));
        let mut sink = CountingSink(0, 0);
        let mut packets = 0u64;
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(p)) => {
                    if p.track == track.id {
                        packets += 1;
                        let _ = decoder
                            .decode(p, &mut sink)
                            .unwrap_or_else(|err| unreachable!("decode: {err}"));
                    }
                }
                Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {}
                Ok(DemuxEvent::EndOfStream) => break,
                Err(err) => unreachable!("demux: {err}"),
            }
        }
        let _ = decoder
            .flush(&mut sink)
            .unwrap_or_else(|err| unreachable!("flush: {err}"));
        assert_eq!(packets, 38, "ffprobe's packet count");
        assert_eq!(sink.0, 37_568, "the raw-emission window");
        assert_eq!(sink.1, 2, "the negotiated channel count");
    }
}
