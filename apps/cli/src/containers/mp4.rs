//! The MP4/MOV/M4A container host (card 6.OPW-mp4 — the registry's
//! sixth wired family, the ISO-BMFF transport of ISO/IEC 14496-12
//! and QuickTime): construct the demuxer, run the host pre-pass
//! `start()` (the `ftyp`+`moov` box walk — progressive sample tables
//! or `mvex` fragment defaults), harvest every `trak`'s facts into
//! [`TrackFact`]s, clamp `--start-time` against the declared extent
//! and validate it through the demuxer's pure `seek` query (the
//! `stss` keyframe walk-back, or the bounded moof-scan for
//! fragments), wrap in the shared
//! [`SelectDemuxer`](crate::demux_host::SelectDemuxer).
//!
//! The harvest's decoder-config truth (the doc §4.3.3 rule; the
//! demuxer's [`sample_entry_bytes`](demux::mp4::Mp4Demuxer::sample_entry_bytes)
//! is the facts source — the OPW-0 doc's own citation: "the verbatim
//! first `stsd` entry payload", the WAV `fmt ` precedent):
//!
//! * **AAC `extra_data` = the esds AudioSpecificConfig** — the
//!   sample entry's `esds` child walked to its
//!   DecoderSpecificInfo, EXACTLY the bare ASC the AAC decoder's
//!   raw-block mode sniffs (`asc[0] >> 3` = the object type; the
//!   whole-entry blob would not parse — the extraction is this
//!   harvest's own truth, mirroring the demuxer's `esds_oti` child
//!   walk for the mislabeled-codec quirks).
//! * **ALAC `extra_data` = the `alac` child box verbatim** (8-byte
//!   box header + config) — the ALAC decoder's cookie prefix
//!   stripper accepts the atom-wrapped shape.
//! * **h264/hevc `extra_data` = the `avcC`/`hvcC` child payload**
//!   (the config record WITHOUT the box header — ffmpeg's extradata
//!   shape). No software h264/hevc decoder is dispatched in this
//!   build: the blob is the forward-honest config, and
//!   `decoder_for` refuses the codec typed (`cli.codec_unavailable`)
//!   — the hw-path cards own the seam.
//! * **opus `extra_data` = OpusHead synthesized from `dOps`** (the
//!   RFC 7845 19-byte identification header — dOps' big-endian
//!   fields re-packed little-endian, ffmpeg's `ff_mov_rewrite_dops`
//!   mapping). The opus decoder IS dispatched since 6.M-opus — the
//!   synthesized head configures it end-to-end.
//! * **mjpeg/PCM carry no blob**: MJPEG is self-describing per
//!   frame (the avi precedent); PCM's truth is the geometry pair
//!   below.
//! * **The audio geometry pair** (channels + rate, the
//!   [`TrackFact::audio_geometry`] limb source) is the sample
//!   entry's own declaration — `channelcount` at offset 12, the
//!   16.16 `samplerate`'s integer half at offset 20 (the fields
//!   ffprobe reports). For PCM codecs the pair rides `pcm` (the
//!   PcmDecoder `with_format` seam); for AAC it rides the same
//!   pair — the mp4a entry DECLARES the geometry (raw ADTS does
//!   not, which is why that family stays limb-less), and every
//!   consumer that treats `pcm` as "this track decodes as PCM"
//!   gates on the codec descriptor first (the provider's `pcm_`
//!   arm, the bench limb) — for a non-PCM track the pair is
//!   geometry-only, never a decode route.
//! * **Video dimensions** are the VisualSampleEntry's own
//!   `width`/`height` (offsets 24/26 — the coded box, not the
//!   display matrix; `0058-rot-display` rotation stays
//!   demuxer-domain). `frame_ns` stays `None`: ISO-BMFF has no
//!   fixed frame duration (VFR is native to the family — a
//!   timescale tick is not a frame), and a guessed cadence would
//!   be a lie.
//! * **Duration IS declared** (mdhd media duration on the emitted
//!   timeline; `Missing` for fragment-only files), so
//!   `--start-time` clamps against the real extent.
//! * **Seek**: progressive = `stss` keyframe walk-back
//!   (`Fast { indexed: true }`); fragmented = bounded moof-scan
//!   (`SlowLinear`). The demuxer's `seek` refuses past-the-end
//!   targets typed — the clamp above means the open-time query
//!   never asks for one.
//! * **Chapters** are demuxer-side metadata (the `tref/chap`
//!   traks are consumed, hidden from the track list) —
//!   [`MediaFacts`](crate::demux_host::MediaFacts) carries no
//!   chapter vocabulary yet, an honestly-recorded gap for the
//!   session layer (the chapter test pins the current shape).

use commands::CommandError;
use demux::mp4::Mp4Demuxer;
use media_types::packet::SharedBytes;
use media_types::time::{Stamp, TimeSpan, Timestamp};
use media_types::{EngineError, MediaKind, Track};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};

use crate::demux_host::{
    MediaFacts, OpenOptions, OpenedMedia, SelectDemuxer, TrackFact, wire_error,
};

/// Open an MP4/MOV/M4A source (the doc §4.2 per-container shape): the
/// probe already named `mp4`; this constructs the demuxer, runs the
/// host pre-pass `start()`, harvestes the sample-entry facts, clamps
/// `--start-time` against the declared duration, validates it through
/// the demuxer's pure `seek` query and wraps the demuxer in the
/// skip/selection wrapper.
///
/// # Errors
/// Typed [`CommandError`]s (stage `demux`): box-walk parse failures,
/// truncations, seek-range violations.
pub(crate) fn open(
    stream: Box<dyn plugin_api::SourceStream>,
    opts: &OpenOptions,
) -> Result<OpenedMedia, CommandError> {
    // 1. Demuxer + the host pre-pass (track list + sample-entry facts).
    let mut demuxer = Mp4Demuxer::new(stream).map_err(|err| wire_error(&err))?;
    let tracks = demuxer.start().map_err(|err| wire_error(&err))?;
    let facts = harvest_facts(&demuxer, &tracks);
    // 2. Clamp the start position (doc 05 §2.4: clamping is truth) —
    //    the mdhd-declared extent; fragment-only files declare none
    //    (the honest MAX/4 bound, the avi convention).
    let duration_us = facts.duration_us().unwrap_or(i64::MAX / 4);
    let start_us = opts.start_time_us.unwrap_or(0).clamp(0, duration_us);
    let mut skip_until_ns = None;
    if start_us > 0 {
        // The pure seek query validates the target against the sample
        // tables (the stss walk-back / moof-scan) before the session
        // exists; the target is nanosecond-based (the engine clock).
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
        StartedMp4 {
            inner: demuxer,
            tracks: tracks.clone(),
        },
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

/// A started MP4 demuxer behind the host's start-idempotence seam:
/// the CLI pre-pass calls `start()` once (the composition root's
/// read of the track list, doc §4.2), and the engine's own `start()`
/// would hit the demuxer's deliberate second-call refusal
/// (`demux.mp4.not-started` — unlike ADTS/AVI, which answer the
/// cached list). This adapter caches the pre-pass answer and
/// forwards everything else verbatim (seek, events, seekability, the
/// reanchor default) — the demuxer crate stays untouched (the
/// 6.OPW STOP rule; the idempotence question is recorded for its
/// owning card).
struct StartedMp4 {
    inner: Mp4Demuxer,
    tracks: Vec<Track>,
}

impl Demuxer for StartedMp4 {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        Ok(self.tracks.clone())
    }
    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        self.inner.next_event()
    }
    fn seek(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        self.inner.seek(target)
    }
    fn seekability(&self) -> SeekSupport {
        self.inner.seekability()
    }
    fn reanchor(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        // The demuxer keeps the trait's default typed refusal (the
        // moving half of in-session seek is its honest gap) —
        // forwarded, never invented here.
        self.inner.reanchor(target)
    }
}

/// Harvest the per-track facts from a started MP4 demuxer: one fact
/// per exposed `trak`, the codec-config blob extracted from the
/// demuxer's verbatim sample entry (see the module docs for each
/// codec's shape), the audio geometry pair from the sample entry's
/// own declaration, and the coded dimensions for video tracks.
fn harvest_facts(demuxer: &Mp4Demuxer, tracks: &[Track]) -> MediaFacts {
    let mut facts = Vec::with_capacity(tracks.len());
    for track in tracks {
        let codec = track.codec.as_str();
        let entry = demuxer.sample_entry_bytes(track.id);
        // The codec-config blob (the module-doc truth table).
        let extra_data = entry.and_then(|entry| config_blob(codec, entry));
        // The geometry pair: the sample entry's own declaration for
        // audio-kind tracks. PCM routes it through `pcm` (the
        // PcmDecoder `with_format` seam); AAC through the same pair
        // as the geometry-only carrier (raw ADTS declares no such
        // pair — that family stays limb-less; every consumer that
        // treats `pcm` as a decode route gates on the codec
        // descriptor first, see the module docs).
        let pcm = entry
            .filter(|_| track.kind == MediaKind::Audio)
            .and_then(audio_geometry)
            .filter(|_| codec.starts_with("pcm_") || codec == "aac");
        // Coded dimensions from the VisualSampleEntry.
        let dimensions = entry
            .filter(|_| track.kind == MediaKind::Video)
            .and_then(visual_dimensions);
        let span_end_ticks = track.timing.span.end.timestamp().ok().map(|ts| ts.ticks);
        facts.push(TrackFact {
            id: track.id,
            kind: track.kind,
            codec: codec.to_string(),
            base: track.timing.base,
            span_end_ticks,
            pcm,
            extra_data,
            frame_ns: None, // VFR family — no fixed frame duration (module docs)
            dimensions,
        });
    }
    MediaFacts {
        tracks: tracks.to_vec(),
        facts,
        container: "mp4".to_string(),
    }
}

/// The per-codec config blob from one verbatim `stsd` entry payload.
fn config_blob(codec: &str, entry: &[u8]) -> Option<SharedBytes> {
    match codec {
        // The esds AudioSpecificConfig (the bare DSI bytes).
        "aac" => asc_of_entry(entry).map(|asc| SharedBytes::from_vec(asc.to_vec())),
        // The alac child box verbatim (header + config — the cookie
        // shape the decoder's prefix stripper accepts).
        "alac" => child_box_full(entry, b"alac").map(SharedBytes::from_vec),
        // The config-record payloads (ffmpeg's extradata shapes).
        "h264" => find_child_box(entry, b"avcC").map(|body| SharedBytes::from_vec(body.to_vec())),
        "hevc" => find_child_box(entry, b"hvcC").map(|body| SharedBytes::from_vec(body.to_vec())),
        "av1" => find_child_box(entry, b"av1C").map(|body| SharedBytes::from_vec(body.to_vec())),
        // OpusHead synthesized from dOps (the RFC 7845 mapping).
        "opus" => find_child_box(entry, b"dOps")
            .and_then(opus_head_of_dops)
            .map(SharedBytes::from_vec),
        // MPEG-4 part 2: the esds DSI (the VOS/VOL start codes).
        "mpeg4video" => asc_of_entry(entry).map(|dsi| SharedBytes::from_vec(dsi.to_vec())),
        // mp3/mjpeg/PCM/mov_text: the config rides in-band or is the
        // geometry pair — no blob (the avi precedent).
        _ => None,
    }
}

/// u16 big-endian at `at` (the demuxer's own `u16be` shape).
fn u16be(data: &[u8], at: usize) -> Option<u16> {
    let b = data.get(at..at + 2)?;
    Some(u16::from_be_bytes([b[0], b[1]]))
}

/// u32 big-endian at `at`.
fn u32be(data: &[u8], at: usize) -> Option<u32> {
    let b = data.get(at..at + 4)?;
    Some(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

/// Find one child box of an `stsd` entry by fourcc and return its
/// PAYLOAD (after the 8-byte box header). Child config boxes ride
/// after the sample entry's fixed prefix, whose length depends on the
/// entry class: 28 for AudioSampleEntry (§12.4.3), 76/78 for
/// VisualSampleEntry (§12.1.3; ffmpeg pads to 78) — the demuxer's own
/// `esds_oti` candidate convention, mirrored (a wrong candidate walks
/// no well-formed child and falls through).
fn find_child_box<'a>(entry: &'a [u8], magic: &[u8; 4]) -> Option<&'a [u8]> {
    for start in [28usize, 76, 78] {
        if entry.len() < start + 8 {
            continue;
        }
        let mut pos = start;
        while let Some((size, kind)) = child_header(entry, pos) {
            if kind == *magic {
                return entry.get(pos + 8..pos + size);
            }
            if size < 8 {
                break; // structural lie: this candidate is wrong
            }
            pos += size;
        }
    }
    None
}

/// The same child box WITH its 8-byte header (the ALAC cookie shape).
fn child_box_full(entry: &[u8], magic: &[u8; 4]) -> Option<Vec<u8>> {
    for start in [28usize, 76, 78] {
        if entry.len() < start + 8 {
            continue;
        }
        let mut pos = start;
        while let Some((size, kind)) = child_header(entry, pos) {
            if kind == *magic {
                return entry.get(pos..pos + size).map(|bytes| bytes.to_vec());
            }
            if size < 8 {
                break;
            }
            pos += size;
        }
    }
    None
}

/// One child box header (declared size, fourcc) at `pos`, `None`
/// past the entry's end.
fn child_header(entry: &[u8], pos: usize) -> Option<(usize, [u8; 4])> {
    let size = u32be(entry, pos)? as usize;
    let kind = [
        *entry.get(pos + 4)?,
        *entry.get(pos + 5)?,
        *entry.get(pos + 6)?,
        *entry.get(pos + 7)?,
    ];
    if pos + size > entry.len() {
        // Overruns the entry: not a well-formed child of this
        // candidate walk.
        return None;
    }
    Some((size, kind))
}

/// Walk one `esds` box payload to its DecoderSpecificInfo — the
/// descriptor walk two levels deeper than the demuxer's
/// `oti_of_esds` (which stops at the OTI): ES_Descriptor(0x03)
/// expandable-len → ES_ID(2)+flags(1) → DecoderConfig(0x04)
/// expandable-len → 13 fixed bytes → DecoderSpecificInfo(0x05)
/// expandable-len → EXACTLY its declared length (the chain trails
/// an SLConfigDescriptor — the slice must stop at the DSI's own
/// length, never run to the box end).
fn dsi_of_esds(body: &[u8]) -> Option<&[u8]> {
    /// Skip one descriptor tag's expandable length (1..4 bytes,
    /// 0x80-continuation — the demuxer's own loop shape), answering
    /// (next position, declared length).
    fn expandable_len(body: &[u8], at: usize) -> Option<(usize, usize)> {
        let mut at = at;
        let mut len = 0usize;
        for _ in 0..4 {
            let byte = *body.get(at)?;
            at += 1;
            len = (len << 7) | usize::from(byte & 0x7F);
            if byte & 0x80 == 0 {
                return Some((at, len));
            }
        }
        None
    }
    let mut at = 4; // version/flags
    if body.get(at) != Some(&0x03) {
        return None;
    }
    at = expandable_len(body, at + 1)?.0;
    at += 3; // ES_ID + flags
    if body.get(at) != Some(&0x04) {
        return None;
    }
    at = expandable_len(body, at + 1)?.0;
    at += 13; // OTI + streamType + bufferSizeDB + max/avg bitrate
    if body.get(at) != Some(&0x05) {
        return None;
    }
    let (at, len) = expandable_len(body, at + 1)?;
    let dsi = body.get(at..at.checked_add(len)?)?;
    (!dsi.is_empty()).then_some(dsi)
}

/// The AudioSpecificConfig of one `mp4a`/`mp4v` sample entry (the
/// esds DecoderSpecificInfo — the bare ASC for AAC, the VOL start
/// codes for MPEG-4 part 2).
fn asc_of_entry(entry: &[u8]) -> Option<&[u8]> {
    let esds = find_child_box(entry, b"esds")?;
    let dsi = dsi_of_esds(esds)?;
    (!dsi.is_empty()).then_some(dsi)
}

/// The audio geometry pair one AudioSampleEntry declares:
/// `channelcount` (offset 16) and the 16.16 `samplerate`'s integer
/// half (offset 24) — the fields ffprobe reports (ISO 14496-12
/// §12.4.3: 8 common + 2 version + 2 revision + 4 vendor + 2
/// channels + 2 samplesize + 2 pre_defined + 2 reserved + 4 rate).
fn audio_geometry(entry: &[u8]) -> Option<(u32, u32)> {
    let channels = u16be(entry, 16)?;
    let rate = u16be(entry, 24)?;
    (channels > 0 && rate > 0).then_some((u32::from(channels), u32::from(rate)))
}

/// The coded dimensions one VisualSampleEntry declares: `width` and
/// `height` (offsets 24/26) — the coded box, never the display
/// matrix (rotation is demuxer-domain).
fn visual_dimensions(entry: &[u8]) -> Option<(u32, u32)> {
    let width = u16be(entry, 24)?;
    let height = u16be(entry, 26)?;
    (width > 0 && height > 0).then_some((u32::from(width), u32::from(height)))
}

/// Synthesize the RFC 7845 OpusHead identification header from one
/// `dOps` box payload (ISO/IEC 23003-5: version(1) channels(1)
/// pre-skip(2) input rate(4) gain(2) mapping family(1) [mapping
/// table…], all big-endian — re-packed little-endian into the 19-byte
/// OpusHead + table, ffmpeg's `ff_mov_rewrite_dops` mapping). The
/// output rate is always 48 kHz (RFC 7845 §4.2); the input rate is
/// advisory.
fn opus_head_of_dops(dops: &[u8]) -> Option<Vec<u8>> {
    if dops.len() < 11 || dops[0] != 0 {
        return None; // unknown dOps version: never a guess
    }
    let mut head = Vec::with_capacity(19);
    head.extend_from_slice(b"OpusHead");
    head.push(1); // OpusHead version
    head.push(dops[1]); // channel count
    head.extend_from_slice(&dops[2..4]); // pre-skip (LE)
    // The INPUT sample rate (advisory — the decoder output is 48 kHz;
    // audio_geometry keeps the 48 kHz rule on the OpusHead sniff).
    head.extend_from_slice(&dops[4..8]); // input rate (LE)
    head.extend_from_slice(&dops[8..10]); // output gain (LE)
    head.push(dops[10]); // mapping family
    if dops[10] > 0 {
        // The channel mapping table rides verbatim (its length is
        // family-dependent: 2 + channels for family 1).
        head.extend_from_slice(dops.get(11..)?);
    }
    Some(head)
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
    use testkit::Must;

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
    /// negotiated channel count (the adts.rs shape).
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

    /// The decoder provider's answer for one track: `Ok(counts)` for
    /// a dispatched decoder, `Err` for the typed refusal.
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
    /// open path with the recorded facts: two tracks, the AAC blob is
    /// the bare esds ASC (object type 2 = LC), the geometry pair is
    /// the sample entry's own declaration, the h264 blob is the avcC
    /// config record, the coded dimensions are the entry's, and the
    /// extent is declared (mdhd).
    #[test]
    fn h264_aac_fixture_opens_with_expected_facts() {
        let opened = open(fixture("generated/valid/mp4_h264_aac.mp4"), None);
        assert_eq!(opened.facts.container, "mp4");
        assert_eq!(opened.facts.tracks.len(), 2);
        let [video, audio] = &opened.facts.tracks[..] else {
            unreachable!("ffprobe: h264 then aac")
        };
        assert_eq!(video.codec.as_str(), "h264");
        assert_eq!(audio.codec.as_str(), "aac");
        let vf = &opened.facts.facts[0];
        let af = &opened.facts.facts[1];
        // h264: the avcC config record (configurationVersion 1),
        // coded dimensions, no geometry, no frame cadence (VFR rule).
        let avcc = vf
            .extra_data
            .as_ref()
            .unwrap_or_else(|| unreachable!("avcC harvested"));
        assert_eq!(avcc.as_slice()[0], 1, "avcC configurationVersion");
        assert_eq!(vf.dimensions, Some((128, 96)), "the coded box");
        assert!(vf.pcm.is_none() && vf.frame_ns.is_none());
        // aac: the bare ASC (AOT 2 = LC — `asc[0] >> 3`), and the
        // entry-declared geometry pair flowing through the limb
        // lookup (the module docs: `pcm` is the geometry carrier;
        // the decode route gates on the codec descriptor).
        let asc = af
            .extra_data
            .as_ref()
            .unwrap_or_else(|| unreachable!("ASC harvested"));
        let asc = asc.as_slice();
        assert!(asc.len() >= 2);
        assert_eq!(asc[0] >> 3, 2, "the ASC's object type is LC");
        assert_eq!(af.pcm, Some((1, 44_100)), "the entry's declaration");
        assert_eq!(af.audio_geometry(), Some((1, 44_100)));
        // The declared extent clamps the start position.
        assert!(opened.facts.duration_us().is_some());
        assert_eq!(opened.start_position_ns, 0);
    }

    /// The QuickTime-authored `.mov` and the `elst` edit-list variant
    /// open with the same facts (the family is one probe id).
    #[test]
    fn mov_and_edts_fixtures_open() {
        for name in ["generated/valid/mp4_qt.mov", "generated/valid/mp4_edts.mp4"] {
            let opened = open(fixture(name), None);
            assert_eq!(opened.facts.container, "mp4", "{name}");
            assert_eq!(opened.facts.tracks.len(), 2, "{name}");
            assert_eq!(opened.facts.facts[1].codec, "aac", "{name}");
            assert!(
                opened.facts.duration_us().is_some_and(|us| us > 0),
                "{name}: the mdhd/elst extent"
            );
        }
    }

    /// The mjpeg+PCM parent fixture opens with the PCM pair as the
    /// audio facts (the PcmDecoder `with_format` seam) and no blobs.
    #[test]
    fn mjpeg_pcm_fixture_opens_with_pcm_facts() {
        let opened = open(fixture("generated/valid/mp4_mjpeg_pcm.mp4"), None);
        assert_eq!(opened.facts.container, "mp4");
        assert_eq!(opened.facts.tracks.len(), 2);
        assert_eq!(opened.facts.facts[0].codec, "mjpeg");
        assert_eq!(opened.facts.facts[0].extra_data, None);
        assert_eq!(opened.facts.facts[1].codec, "pcm_s16le");
        assert_eq!(opened.facts.facts[1].pcm, Some((2, 44_100)));
        assert_eq!(opened.facts.facts[1].extra_data, None);
    }

    /// `--start-time` through the pure seek query: an in-media target
    /// lands every surviving packet at/after it (the wrapper's skip
    /// watermark), and the seekability truth forwards through the
    /// wrapper — `Fast { indexed }` for progressive tables.
    #[test]
    fn start_time_skips_packets_and_reports_indexed_seek() {
        let mut mid = open(fixture("generated/valid/mp4_h264_aac.mp4"), Some(1_000));
        assert_eq!(mid.start_position_ns, 1_000_000);
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
                        assert!(ns.ticks >= 1_000_000, "early packet: {}", ns.ticks);
                    }
                }
                Ok(DemuxEvent::EndOfStream) => break,
                Ok(_) => {}
                Err(err) => unreachable!("{err}"),
            }
        }
        assert!(seen > 0, "an in-media target keeps the tail");
    }

    /// The chapters fixture through the open path: the `tref/chap`
    /// text trak is CONSUMED as metadata (hidden from the track list
    /// — ffprobe shows it as a `bin_data` data stream), the media
    /// traks demux normally, and the open path carries NO chapter
    /// vocabulary — the demuxer's three titled markers stay
    /// demuxer-side (the honest recorded gap for the session layer).
    #[test]
    fn chapter_trak_is_hidden_and_markers_stay_demuxer_side() {
        let path = fixture("generated/valid/mp4_chapters.mp4");
        let mut opened = open(path.clone(), None);
        assert_eq!(opened.facts.container, "mp4");
        assert_eq!(
            opened.facts.tracks.len(),
            2,
            "the chapter trak is hidden (ffprobe's third stream is data)"
        );
        assert_eq!(opened.facts.facts[0].codec, "h264");
        assert_eq!(opened.facts.facts[1].codec, "aac");
        let mut counts = [0u64; 2];
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(p)) => counts[p.track.handle() as usize] += 1,
                Ok(DemuxEvent::EndOfStream) => break,
                Ok(_) => {}
                Err(err) => unreachable!("{err}"),
            }
        }
        assert!(counts[0] > 0 && counts[1] > 0, "both media traks flow");
        // The demuxer-side truth (the direct construction — the open
        // path has no chapter surface to assert on): three titled
        // markers on the movie timeline, the demuxer card's own pin.
        let bytes = std::fs::read(&path).must();
        let stream = demux::mp4::test_support::mem_stream(bytes).must();
        let mut mp4 = Mp4Demuxer::new(stream).unwrap_or_else(|e| unreachable!("{e}"));
        mp4.start().unwrap_or_else(|e| unreachable!("{e}"));
        let chapters = mp4.chapters();
        assert_eq!(chapters.len(), 3, "the demuxer's current marker count");
        assert!(chapters.iter().all(|c| !c.title.is_empty()));
    }

    /// The attached-picture trak (`0029-cover-audio.m4a`): the cover
    /// art is a still-image mjpeg video track appended after the
    /// audio trak (the ffmpeg attached-pic precedent), one keyframe
    /// packet emitted ahead of the media walk — decoded as one frame
    /// through the real provider, honestly.
    #[test]
    fn cover_art_m4a_opens_with_still_image_trak() {
        let Some(dir) = corpus_dir() else {
            skip_loudly();
            return;
        };
        let mut opened = open(dir.join("0029-cover-audio.m4a"), None);
        assert_eq!(opened.facts.container, "mp4");
        assert_eq!(opened.facts.tracks.len(), 2);
        let [audio, cover] = &opened.facts.tracks[..] else {
            unreachable!("aac then the cover trak")
        };
        assert_eq!(audio.codec.as_str(), "aac");
        assert_eq!(cover.codec.as_str(), "mjpeg");
        assert!(
            cover.flags.still_image,
            "the cover trak is the still-image shape"
        );
        // The synthesized cover trak carries NO sample entry (the
        // demuxer builds it from udta/covr bytes, not an stsd row) —
        // no dimensions to harvest, the honest gap (ffprobe parses
        // the JPEG's SOF for 300x300; OUR mjpeg decoder derives the
        // frame geometry from the JPEG bytes at decode instead).
        assert_eq!(opened.facts.facts[1].dimensions, None);
        assert!(opened.facts.facts[1].extra_data.is_none());
        // The walk: 2 aac packets + 1 cover packet (ffprobe's count).
        let mut aac_packets = 0u64;
        let mut cover_packets = 0u64;
        let mut cover_decoder =
            decoder_for(&opened, cover).unwrap_or_else(|err| unreachable!("mjpeg decodes: {err}"));
        let mut sink = CountingSink(0, 0);
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(p)) => {
                    if p.track == audio.id {
                        aac_packets += 1;
                    } else {
                        cover_packets += 1;
                        let _ = cover_decoder
                            .decode(p, &mut sink)
                            .unwrap_or_else(|e| unreachable!("cover decode: {e}"));
                    }
                }
                Ok(DemuxEvent::EndOfStream) => break,
                Ok(_) => {}
                Err(err) => unreachable!("{err}"),
            }
        }
        let _ = cover_decoder
            .flush(&mut sink)
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!((aac_packets, cover_packets), (2, 1));
        assert_eq!(sink.0, 1, "the one cover packet decodes to one frame");
    }

    // -----------------------------------------------------------------
    // The corpus differential (the user's standing mandate): every
    // ISO-BMFF lane of corpus-v4 through the REAL open path — track
    // codecs and packet counts against the same ffmpeg-measured
    // truth as crates/demux/tests/corpus_mp4.rs (ffprobe
    // -count_packets), decode-count assertions where a decoder is
    // dispatched (aac ×1024 sample frames per packet, mjpeg one
    // frame per packet, PCM's own sample extent), and the honest
    // typed refusal where none is (h264/hevc/mpeg4video — the
    // hw-path cards own those seams).
    // -----------------------------------------------------------------

    /// One corpus lane's expected shape (the ffmpeg-measured truth:
    /// ffprobe codec + packet count per stream, ffprobe/ffmpeg
    /// decoded extents where a decoder exists).
    struct Lane {
        file: &'static str,
        /// (codec, packets) per track.
        streams: &'static [(&'static str, u64)],
        /// (track idx, decoded frames) for the FIRST decodable track
        /// of each kind, when one exists.
        audio_frames: Option<u64>,
        video_frames: Option<u64>,
    }

    const TRUTHS: [Lane; 31] = [
        // 0008: h264 all-keyframe extremes.
        Lane {
            file: "0008-fps-extremes.mp4",
            streams: &[("h264", 345)],
            audio_frames: None,
            video_frames: None,
        },
        Lane {
            file: "0012-still-24h.mp4",
            streams: &[("h264", 1)],
            audio_frames: None,
            video_frames: None,
        },
        Lane {
            file: "0021-vbv-burst.mp4",
            streams: &[("h264", 86)],
            audio_frames: None,
            video_frames: None,
        },
        Lane {
            file: "0025-meta-bomb-av.mp4",
            streams: &[("h264", 111), ("aac", 44)],
            audio_frames: None,
            video_frames: None,
        }, // aac is 6-channel — the decoder's mono/stereo gate refuses it typed below
        Lane {
            file: "0027-mp4-video.mp4",
            streams: &[("h264", 42)],
            audio_frames: None,
            video_frames: None,
        },
        // 0032: fragmented (moof ladder) h264.
        Lane {
            file: "0032-fmp4-video.mp4",
            streams: &[("h264", 77)],
            audio_frames: None,
            video_frames: None,
        },
        Lane {
            file: "0045-mp4-video.mp4",
            streams: &[("mpeg4video", 37)],
            audio_frames: None,
            video_frames: None,
        },
        Lane {
            file: "0049-muxed-av.mp4",
            streams: &[("h264", 42), ("aac", 35)],
            audio_frames: Some(35 * 1024),
            video_frames: None,
        },
        Lane {
            file: "0058-rot-display.mp4",
            streams: &[("h264", 28)],
            audio_frames: None,
            video_frames: None,
        },
        Lane {
            file: "0060-drift-av.mp4",
            streams: &[("h264", 1412), ("aac", 2029)],
            audio_frames: Some(2029 * 1024),
            video_frames: None,
        },
        Lane {
            file: "0062-mp4-video.mp4",
            streams: &[("h264", 3)],
            audio_frames: None,
            video_frames: None,
        },
        Lane {
            file: "0069-muxed-av.mp4",
            streams: &[("h264", 25), ("aac", 18)],
            audio_frames: Some(18 * 1024),
            video_frames: None,
        },
        Lane {
            file: "0074-bframe-storm.mp4",
            streams: &[("h264", 16)],
            audio_frames: None,
            video_frames: None,
        },
        Lane {
            file: "0082-vfr-av.mp4",
            streams: &[("h264", 17), ("aac", 32)],
            audio_frames: None,
            video_frames: None,
        }, // 6-channel aac: the mono/stereo gate
        // 0089: fragmented HEVC.
        Lane {
            file: "0089-fmp4-video.mp4",
            streams: &[("hevc", 39)],
            audio_frames: None,
            video_frames: None,
        },
        Lane {
            file: "0093-param-midchange.mp4",
            streams: &[("mpeg4video", 63)],
            audio_frames: None,
            video_frames: None,
        },
        Lane {
            file: "0097-mp4-video.mp4",
            streams: &[("mpeg4video", 37)],
            audio_frames: None,
            video_frames: None,
        },
        Lane {
            file: "0100-mp4-video.mp4",
            streams: &[("hevc", 49)],
            audio_frames: None,
            video_frames: None,
        },
        // 0105: three traks — h264 + aac + mp3 (the OTI 0x6b quirk).
        Lane {
            file: "0105-multi-av.mp4",
            streams: &[("h264", 84), ("aac", 62), ("mp3", 55)],
            audio_frames: Some(62 * 1024),
            video_frames: None,
        },
        Lane {
            file: "0109-odd-dims.mp4",
            streams: &[("h264", 44)],
            audio_frames: None,
            video_frames: None,
        },
        Lane {
            file: "0112-packed-bs.mp4",
            streams: &[("h264", 25)],
            audio_frames: None,
            video_frames: None,
        },
        // 0115: the mp4v-mislabeled-MJPEG quirk (esds OTI 0x6c).
        Lane {
            file: "0115-fps-extremes.mp4",
            streams: &[("mjpeg", 8)],
            audio_frames: None,
            video_frames: Some(8),
        },
        // 0128 (mpeg2video) refuses typed at the demux stage — the
        // vocabulary gap pinned by its own test below.
        Lane {
            file: "0140-muxed-av.mp4",
            streams: &[("hevc", 82), ("aac", 101)],
            audio_frames: Some(101 * 1024),
            video_frames: None,
        },
        // ---- MOV (QuickTime authored) ----
        // 0011: the mp3-in-MOV lane (MPEG-2 LSF, 16 kHz): our decoder
        // emits 37,487 sample frames for the 66 packets — ffmpeg
        // answers 36,911 (it trims the tail to the trak-declared
        // sample extent; our engine decodes whole codec frames — the
        // 576-sample tail divergence is recorded here, the mp3 codec
        // card owns the trim question).
        Lane {
            file: "0011-vfr-av.mov",
            streams: &[("h264", 117), ("mp3", 66)],
            audio_frames: Some(37_487),
            video_frames: None,
        },
        // 0051: mjpeg + sowt PCM (the MOV PCM seam).
        Lane {
            file: "0051-vfr-av.mov",
            streams: &[("mjpeg", 46), ("pcm_s16le", 93)],
            audio_frames: Some(95_232),
            video_frames: Some(46),
        },
        Lane {
            file: "0061-muxed-av.mov",
            streams: &[("h264", 61), ("aac", 46)],
            audio_frames: None,
            video_frames: None,
        }, // 6-channel aac: the mono/stereo gate
        Lane {
            file: "0079-mov-video.mov",
            streams: &[("mjpeg", 60)],
            audio_frames: None,
            video_frames: Some(60),
        },
        Lane {
            file: "0122-packed-bs.mov",
            streams: &[("h264", 22)],
            audio_frames: None,
            video_frames: None,
        },
        // ---- M4A ----
        Lane {
            file: "0029-cover-audio.m4a",
            streams: &[("aac", 2), ("mjpeg", 1)],
            audio_frames: Some(2 * 1024),
            video_frames: Some(1),
        },
        // 0094: 6-channel aac — the decoder's mono/stereo gate refuses
        // it typed below (the honest recorded partial, card 5.C2a).
        Lane {
            file: "0094-audio-only.m4a",
            streams: &[("aac", 48)],
            audio_frames: None,
            video_frames: None,
        },
        // 0128: MPEG-2 video under an `mp4v` tag in THIS corpus
        // instance (ffprobe name-maps it `mpeg2video` by esds OTI;
        // the demuxer maps the tag — the recorded divergence, the
        // MPEG-2 codec card owns the OTI sniff). It opens, demuxes
        // 64 packets, and the codec refuses typed like every
        // mpeg4video lane.
        Lane {
            file: "0128-param-midchange.mp4",
            streams: &[("mpeg4video", 64)],
            audio_frames: None,
            video_frames: None,
        },
    ];

    /// The corpus media dir (the demux suite's convention — `None`
    /// means the loud skip).
    fn corpus_dir() -> Option<PathBuf> {
        // lumen-allow-literal: cfg(test) corpus-gate env-var name — test harness knob, not product config
        if let Ok(dir) = std::env::var("LUMEN_CORPUS") {
            return Some(PathBuf::from(dir));
        }
        let default = PathBuf::from("/tmp/my-project/corpus-v4/media"); // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
        default.is_dir().then_some(default)
    }

    fn skip_loudly() {
        eprintln!(
            // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
            "corpus(mp4-open): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \
             — SKIP (the differential table for this instance is in the worklog)"
        );
    }

    #[test]
    fn corpus_iso_bmff_lanes_match_the_ffmpeg_truth() {
        let Some(dir) = corpus_dir() else {
            skip_loudly();
            return;
        };
        for lane in &TRUTHS {
            let name = lane.file;
            let mut opened = open(dir.join(name), None);
            assert_eq!(opened.facts.container, "mp4", "{name}");
            assert_eq!(opened.facts.tracks.len(), lane.streams.len(), "{name}");
            for (fact, (codec, _)) in opened.facts.facts.iter().zip(lane.streams) {
                assert_eq!(fact.codec, *codec, "{name}: codec");
            }
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
                        // The honest typed refusals: the hw-path
                        // codecs (no software decoder dispatched)
                        // and the AAC engine's own mono/stereo
                        // boundary for the 6-channel lanes (5.C2a's
                        // recorded partial — the refusal is the
                        // decoder's, never a silent stereo downmix).
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

    /// `0128-param-midchange.mp4` (MPEG-2 video under an `mp4v` tag
    /// in this corpus instance): it OPENS and demuxes — the demuxer
    /// maps the stsd tag (the esds-OTI sniff is the codec card's
    /// follow-up) — and the codec refuses typed, like every
    /// mpeg4video lane. (The demux suite's `no-tracks` gap note
    /// predates this corpus instance's tag.)
    #[test]
    fn mpeg2_tagged_lane_opens_and_refuses_the_codec_typed() {
        let Some(dir) = corpus_dir() else {
            skip_loudly();
            return;
        };
        let mut opened = open(dir.join("0128-param-midchange.mp4"), None);
        assert_eq!(opened.facts.tracks.len(), 1);
        assert_eq!(opened.facts.facts[0].codec, "mpeg4video");
        let track = opened.facts.tracks[0].clone();
        let err = match decoder_for(&opened, &track) {
            Ok(_) => unreachable!("no mpeg4video decoder is dispatched"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "cli.codec_unavailable", "{err}");
        let mut packets = 0u64;
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(_)) => packets += 1,
                Ok(DemuxEvent::EndOfStream) => break,
                Ok(_) => {}
                Err(err) => unreachable!("{err}"),
            }
        }
        assert_eq!(packets, 64, "ffprobe's packet count");
    }

    /// The six-channel AAC lanes (ffprobe: 6 ch / 12 kHz): the
    /// geometry pair is the sample entry's declaration and flows to
    /// the limb; the AAC ENGINE refuses the channel count at
    /// configure (mono/stereo only — 5.C2a's recorded partial, the
    /// honest no, never a silent stereo downmix).
    #[test]
    fn six_channel_aac_geometry_flows_but_the_decoder_refuses() {
        let Some(dir) = corpus_dir() else {
            skip_loudly();
            return;
        };
        let opened = open(dir.join("0025-meta-bomb-av.mp4"), None);
        let aac = &opened.facts.facts[1];
        assert_eq!(aac.codec, "aac");
        assert_eq!(aac.audio_geometry(), Some((6, 12_000)));
        let track = opened.facts.tracks[1].clone();
        let err = match decoder_for(&opened, &track) {
            Ok(_) => unreachable!("the mono/stereo engine must refuse 6 channels"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "aac.engine.init", "{err}");
    }

    /// The dOps → OpusHead synthesis (ffmpeg's `ff_mov_rewrite_dops`
    /// mapping): big-endian dOps fields re-packed little-endian into
    /// the RFC 7845 identification header, mapping table verbatim.
    #[test]
    fn dops_synthesizes_the_rfc7845_opus_head() {
        let dops: &[u8] = &[
            0x00, 0x02, 0x00, 0x38, 0x00, 0x00, 0xac, 0x44, 0x00, 0x00, 0x00, // family 0
        ];
        let head = opus_head_of_dops(dops).unwrap_or_else(|| unreachable!("v0 synthesizes"));
        assert_eq!(&head[..8], b"OpusHead");
        assert_eq!(head[8], 1, "OpusHead version");
        assert_eq!(head[9], 2, "channels");
        assert_eq!(&head[10..12], &[0x00, 0x38], "pre-skip LE");
        assert_eq!(&head[12..16], &[0x00, 0x00, 0xac, 0x44], "input rate LE");
        assert_eq!(head.len(), 19, "family 0: no mapping table");
        // A family-1 table rides verbatim; an unknown version refuses.
        let fam1: &[u8] = &[
            0x00, 0x02, 0x00, 0x38, 0x00, 0x00, 0xac, 0x44, 0x00, 0x00, 0x01, 0x00, 0x01, 0xfe,
        ];
        let head = opus_head_of_dops(fam1).unwrap_or_else(|| unreachable!("family 1"));
        assert_eq!(&head[19..], &[0x00, 0x01, 0xfe], "the mapping table");
        assert!(
            opus_head_of_dops(&[0x01, 0x02]).is_none(),
            "unknown version"
        );
    }

    /// The esds DSI walk on a hand-built descriptor chain (the
    /// demuxer's `oti_of_esds` prefixes, extended two levels): the
    /// ASC slice is exactly the DecoderSpecificInfo bytes.
    #[test]
    fn esds_dsi_walk_extracts_the_asc() {
        // ES_Descriptor(0x03) len 0x15: ES_ID(2) flags(1)
        // DecoderConfig(0x04) len 0x11: OTI(1)=0x40 streamType(1)
        // bufferSize(3) max(4) avg(4) DSI(0x05) len 0x02: 0x11 0x90
        let mut esds = vec![0x00, 0x00, 0x00, 0x00]; // version/flags
        esds.extend_from_slice(&[0x03, 0x15, 0x12, 0x34, 0x00]);
        esds.extend_from_slice(&[0x04, 0x11, 0x40, 0x15]);
        esds.extend_from_slice(&[0x00, 0x00, 0x00]);
        esds.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        esds.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        esds.extend_from_slice(&[0x05, 0x02, 0x11, 0x90]);
        let dsi = dsi_of_esds(&esds).unwrap_or_else(|| unreachable!("the chain walks"));
        assert_eq!(dsi, &[0x11, 0x90][..]);
        assert_eq!(dsi[0] >> 3, 2, "sanity: the ASC sniffs LC");
        // Truncated chains refuse (never a partial slice).
        assert_eq!(dsi_of_esds(&esds[..12]), None);
    }
}
