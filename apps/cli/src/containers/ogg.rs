//! The OGG container host (card 6.OPW-ogg — the registry's fifth
//! wired family, the Xiph Ogg transport of RFC 3533): construct the
//! demuxer, run the host pre-pass `start()` (the BOS header walk +
//! tail-granule duration probe), harvest every logical stream's
//! facts into [`TrackFact`]s, validate `--start-time` through the
//! demuxer's pure `seek` query (the granule-based `SlowLinear`
//! page-header scan), wrap in the shared
//! [`SelectDemuxer`](crate::demux_host::SelectDemuxer).
//!
//! This is the first wired family carrying NON-PCM audio (Vorbis,
//! Opus) and video (Theora) — the audio-limb seam fix rides along
//! (see [`TrackFact::audio_geometry`]).
//!
//! The harvest's decoder-config truth (the doc §4.3.3 rule; the
//! demuxer's [`header_packets`](demux::ogg::OggDemuxer::header_packets)
//! accessors are the facts source — OPW-0's own citation):
//!
//! * **Vorbis `extra_data` = the three-header chain joined
//!   VERBATIM** (identification ‖ comment ‖ setup, no length
//!   prefixes, no Xiph lacing): the decoder's
//!   [`VorbisHeaders::from_extra_data`](codecs::audio::vorbis::VorbisHeaders::from_extra_data)
//!   splits the blob by *parsing* the identification header then the
//!   comment header's length — the packing this host chose is
//!   exactly what that parser consumes (the Matroska Xiph-laced
//!   form is a *different* container's seam; Ogg's header packets
//!   are already frame-delimited, so the join loses nothing).
//! * **Opus `extra_data` = OpusHead ‖ OpusTags** (RFC 7845's
//!   identification + comment headers, joined the same way). No
//!   Opus decoder exists in this build yet (the admission M-wave
//!   has not wired one) — the blob is the forward-honest config,
//!   and decode refuses typed `cli.codec_unavailable` today.
//! * **Theora `extra_data` = the three header packets joined**
//!   (`\x80`info ‖ `\x81`comment ‖ `\x82`setup) — same shape, same
//!   typed refusal (no Theora decoder; the blocked-registry
//!   honesty).
//! * **FLAC-in-Ogg `extra_data` = the SECOND header packet** (the
//!   native STREAMINFO block the Xiph FLAC mapping carries after
//!   its one-page mapping header) — the FLAC decoder's 34-byte
//!   STREAMINFO requirement; the mapping header is not part of it.
//! * **No `pcm` pair**: Ogg's vocabulary has no PCM codec — every
//!   audio geometry is codec-domain (Vorbis ID header, OpusHead),
//!   carried by the blob above and surfaced through
//!   [`TrackFact::audio_geometry`] (the generic audio-limb lookup).
//! * **Theora dimensions are NOT harvested**: Ogg carries no
//!   container-level width/height and the demuxer deliberately
//!   treats the info header's geometry as codec-domain (the corpus
//!   differential records the same split) — `dimensions` stays
//!   `None`, never a guess.
//! * **Duration IS declared**: the demuxer's tail-granule probe
//!   stamps every track's span end at `start()`, so
//!   `--start-time` clamps against the real extent (unlike ADTS).
//! * **Seek**: Ogg has no index — landing needs a page-header scan
//!   (`SeekSupport::SlowLinear`), mapped through each stream's
//!   granule clock. The demuxer's `seek` is a pure query (position
//!   restored), so the open-time validation moves nothing.

use std::collections::VecDeque;

use commands::CommandError;
use demux::ogg::OggDemuxer;
use media_types::packet::SharedBytes;
use media_types::time::{Stamp, TimeSpan, Timestamp};
use media_types::{MediaKind, Track};
use plugin_api::Demuxer;

use crate::demux_host::{
    MediaFacts, OpenOptions, OpenedMedia, SelectDemuxer, TrackFact, wire_error,
};

/// Open an OGG/OGV/OPUS source: demuxer + pre-pass + harvest +
/// start-time clamp/validation + the selection wrapper.
///
/// # Errors
/// Typed [`CommandError`]s (stage `demux`): page/CRC parse
/// failures, truncations, seek-range violations.
pub(crate) fn open(
    stream: Box<dyn plugin_api::SourceStream>,
    opts: &OpenOptions,
) -> Result<OpenedMedia, CommandError> {
    // 1. Demuxer + the host pre-pass (track list + header stash).
    let mut demuxer = OggDemuxer::new(stream).map_err(|err| wire_error(&err))?;
    let tracks = demuxer.start().map_err(|err| wire_error(&err))?;
    let facts = harvest_facts(&demuxer, &tracks);
    // 2. Clamp the start position against the declared extent (the
    // tail-granule probe — Ogg DOES declare duration at open).
    let duration_us = facts.duration_us().unwrap_or(i64::MAX / 4);
    let start_us = opts.start_time_us.unwrap_or(0).clamp(0, duration_us);
    let mut skip_until_ns = None;
    if start_us > 0 {
        // The pure seek query validates the target (the granule
        // clock's SlowLinear page scan; nothing observable moves).
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

/// Classify one logical stream by its first stashed header packet's
/// magic — read-only mirror of the demuxer's own `identify()`
/// vocabulary (the descriptor names, never a second grammar: the
/// magic prefixes are the demuxer's published constants in prose).
/// `None` for skeleton streams (they stash headers but mint no
/// track) and for opaque streams (no headers stashed at all).
fn classify(first: &[u8]) -> Option<&'static str> {
    if first.starts_with(&[0x01, b'v', b'o', b'r', b'b', b'i', b's']) {
        Some("vorbis")
    } else if first.starts_with(b"OpusHead") {
        Some("opus")
    } else if first.starts_with(&[0x80, b't', b'h', b'e', b'o', b'r', b'a']) {
        Some("theora")
    } else if first.starts_with(&[0x7F, b'F', b'L', b'A', b'C']) {
        Some("flac")
    } else if first.starts_with(b"Speex   ") {
        Some("speex")
    } else {
        None
    }
}

/// Harvest the track facts from a started Ogg demuxer: one fact per
/// minted track, the header chain per logical stream (mapped
/// track→serial through the arrival-order correspondence — tracks
/// are minted in BOS arrival order minus skeleton streams, and
/// `serial_numbers()` is arrival order, so per-codec queues line
/// up), the tail-probe span end, and the theora frame cadence.
fn harvest_facts(demuxer: &OggDemuxer, tracks: &[Track]) -> MediaFacts {
    // Per-codec serial queues in BOS arrival order (the mapping the
    // demuxer's own mint order defines; two streams of one codec
    // stay ordered by arrival on both sides).
    let mut vorbis_q: VecDeque<u32> = VecDeque::new();
    let mut theora_q: VecDeque<u32> = VecDeque::new();
    let mut flac_q: VecDeque<u32> = VecDeque::new();
    let mut speex_q: VecDeque<u32> = VecDeque::new();
    for serial in demuxer.serial_numbers() {
        let Some(first) = demuxer.header_packets(serial).first() else {
            continue; // opaque stream: nothing stashed, nothing to map
        };
        match classify(first.as_slice()) {
            Some("vorbis") => vorbis_q.push_back(serial),
            Some("opus") => {} // every ogg opus stream shares the descriptor;
            // the queue is unnecessary: the ONLY opus truth this host
            // harvests (the joined OpusHead/OpusTags chain) is
            // identical in shape for any opus stream, and the corpus
            // lanes are single-stream. Multi-opus-stream mapping
            // waits on a demuxer-side track→serial accessor.
            Some("theora") => theora_q.push_back(serial),
            Some("flac") => flac_q.push_back(serial),
            Some("speex") => speex_q.push_back(serial),
            _ => {} // skeleton (magic `fishead\0`): no track minted
        }
    }
    let mut facts = Vec::with_capacity(tracks.len());
    for track in tracks {
        let codec = track.codec.as_str();
        let serial = match codec {
            "vorbis" => vorbis_q.pop_front(),
            "theora" => theora_q.pop_front(),
            "flac" => flac_q.pop_front(),
            "speex" => speex_q.pop_front(),
            "opus" => demuxer.serial_numbers().into_iter().find(|s| {
                demuxer
                    .header_packets(*s)
                    .first()
                    .is_some_and(|h| h.as_slice().starts_with(b"OpusHead"))
            }),
            _ => None, // ogg-opaque: no headers stashed, no blob
        };
        let extra_data = serial.map(|serial| {
            let headers = demuxer.header_packets(serial);
            if codec == "flac" {
                // The native STREAMINFO block (the mapping header's
                // successor) — the FLAC decoder's 34-byte truth.
                match headers.get(1) {
                    Some(h) => SharedBytes::from_vec(h.as_slice().to_vec()),
                    None => SharedBytes::from_vec(Vec::new()),
                }
            } else {
                // The verbatim header-chain join (see module docs).
                let mut blob = Vec::new();
                for header in headers {
                    blob.extend_from_slice(header.as_slice());
                }
                SharedBytes::from_vec(blob)
            }
        });
        let span_end_ticks = track.timing.span.end.timestamp().ok().map(|ts| ts.ticks);
        // Theora's frame cadence from the track base (one tick =
        // one frame at the info header's rate — the avi convention).
        let frame_ns = (track.kind == MediaKind::Video).then(|| {
            u64::from(track.timing.base.num)
                .saturating_mul(1_000_000_000)
                .saturating_div(u64::from(track.timing.base.den))
        });
        facts.push(TrackFact {
            id: track.id,
            kind: track.kind,
            codec: codec.to_string(),
            base: track.timing.base,
            span_end_ticks,
            pcm: None, // Ogg's vocabulary has no PCM codec
            extra_data,
            frame_ns,
            dimensions: None, // theora geometry is codec-domain (module docs)
        });
    }
    MediaFacts {
        tracks: tracks.to_vec(),
        facts,
        container: "ogg".to_string(),
    }
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
    use plugin_api::DemuxEvent;
    use std::path::PathBuf;
    use std::sync::Arc;

    /// The committed Vorbis/Opus fixtures of the codecs crate (cards
    /// 5.B2's own set — real ffmpeg-authored Ogg bytes, the
    /// corpus-independent lane; the adts.rs precedent for borrowing
    /// codec-card fixtures).
    fn codec_fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/codecs/tests/fixtures")
            .join(name)
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

    /// Open a source through the REAL CLI open path.
    fn open(path: PathBuf, start_us: Option<i64>) -> crate::demux_host::OpenedMedia {
        match open_source(&SourceSpec::File { path }, start_us, false, false) {
            Ok(opened) => opened,
            Err(err) => unreachable!("{err}"),
        }
    }

    /// The provider's answer for the first track: a configured
    /// decoder, or the typed refusal (pinned by code — never a
    /// panic; the opus/theora lanes own the refusal paths).
    fn decoder_for(
        opened: &crate::demux_host::OpenedMedia,
    ) -> Result<Box<dyn codec_api::Decoder>, EngineError> {
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
        provider.decoder_for(&track, &config)
    }

    /// Demux every packet of an opened media and return the count.
    fn count_packets(opened: &mut crate::demux_host::OpenedMedia) -> u64 {
        let mut counted = 0u64;
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(_)) => counted += 1,
                Ok(DemuxEvent::EndOfStream) => break,
                Ok(_) => {}
                Err(err) => unreachable!("demux: {err}"),
            }
        }
        counted
    }

    /// The stereo 44.1 kHz Vorbis fixture opens through the real
    /// open path with the recorded facts: one audio track, the
    /// `vorbis` descriptor, the sample-clock base, NO pcm pair, the
    /// three-header chain as the blob (the join format the decoder
    /// parses), the tail-probe extent — and the GENERIC audio
    /// geometry the limb seam consumes (the 6.OPW-ogg fix).
    #[test]
    fn vorbis_fixture_opens_with_expected_facts() {
        let opened = open(codec_fixture("vorbis_sine_44100_stereo.ogg"), None);
        assert_eq!(opened.facts.container, "ogg");
        assert_eq!(opened.facts.tracks.len(), 1);
        let track = &opened.facts.tracks[0];
        assert_eq!(track.kind, MediaKind::Audio);
        let fact = &opened.facts.facts[0];
        assert_eq!(fact.codec, "vorbis");
        assert_eq!(fact.base.as_parts(), (1, 44_100), "the sample clock");
        assert!(fact.pcm.is_none(), "Ogg has no PCM vocabulary");
        // The blob: identification ‖ comment ‖ setup, verbatim join —
        // the reference truth's raw chain (3 headers; the setup
        // header carries the codebooks, hence the size).
        let bytes = fact
            .extra_data
            .as_ref()
            .map(|blob| blob.as_slice())
            .unwrap_or(&[]);
        assert!(bytes.starts_with(&[0x01, b'v', b'o', b'r', b'b', b'i', b's']));
        assert!(
            bytes
                .windows(7)
                .any(|w| w == [0x05, b'v', b'o', b'r', b'b', b'i', b's']),
            "the setup header is in the chain"
        );
        assert!(bytes.len() > 3_000, "codebooks ride the setup header");
        assert_eq!(
            fact.span_end_ticks,
            Some(66_150),
            "the tail-granule extent (1.5 s at 44.1 kHz)"
        );
        // THE SEAM: non-PCM audio answers the generic geometry.
        assert_eq!(fact.audio_geometry(), Some((2, 44_100)));
        assert_eq!(opened.start_position_ns, 0);
    }

    /// The Vorbis fixture demuxes AND decodes the full stream
    /// through the real open path + provider: the packet count is
    /// ffprobe's own, the emitted sample frames are the codec
    /// card's pinned reference (`raw_samples` — the encoder-delayed
    /// raw emission; ffmpeg's trimmed window is 66 150).
    #[test]
    fn vorbis_fixture_decodes_the_full_stream() {
        // The demux walk is single-shot: one fresh open per pass
        // (the adts suite's convention).
        let mut counted = open(codec_fixture("vorbis_sine_44100_stereo.ogg"), None);
        assert_eq!(count_packets(&mut counted), 67, "ffprobe nb_read_packets");
        let mut opened = open(codec_fixture("vorbis_sine_44100_stereo.ogg"), None);
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
        let mut sink = CountingSink(0, 0);
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    let _ = decoder
                        .decode(packet, &mut sink)
                        .unwrap_or_else(|err| unreachable!("decode: {err}"));
                }
                Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {}
                Ok(DemuxEvent::EndOfStream) => break,
                Err(err) => unreachable!("demux: {err}"),
            }
        }
        let _ = decoder
            .flush(&mut sink)
            .unwrap_or_else(|err| unreachable!("flush: {err}"));
        // The granule tail clamp's pin (6.FIX-ogg-reanchor): the
        // delivered count is the 128-frame head emission excess +
        // the 66,150-sample valid timeline — the encoder's 986
        // padding samples past the final granule are trimmed, never
        // played (the old pin, 67,264, counted them).
        assert_eq!(sink.0, 66_278, "the reference raw_samples pin");
        assert_eq!(sink.1, 2, "the negotiated channel count");
    }

    /// `--start-time` mid-stream — the honest Ogg watermark shape:
    /// the demuxer stamps only each page's last-completing packet
    /// (RFC 3533 §6; apportioning one granule among laced packets
    /// is decode-domain), so the open-time watermark drops the
    /// STAMPED pre-target packets while `Missing` stamps pass
    /// unjudged (the unjudgeable-pass rule — never dropped on a
    /// guess). The exact mid-stream seek is the ENGINE's reanchor
    /// path; this is the open-time contract, documented as measured.
    #[test]
    fn start_time_skips_the_stamped_head() {
        let mut opened = open(
            codec_fixture("vorbis_sine_44100_stereo.ogg"),
            Some(1_100_000),
        );
        assert_eq!(opened.start_position_ns, 1_100_000_000);
        let mut seen = 0u64;
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    seen += 1;
                    if let Stamp::Present(pts) = packet.pts {
                        let ns = pts
                            .convert(&media_clock::NANO_BASE)
                            .unwrap_or_else(|_| unreachable!("same-base convert"));
                        assert!(
                            ns.ticks >= 1_100_000_000,
                            "stamped packet {seen} at {} ns predates the 1.1 s target",
                            ns.ticks
                        );
                    }
                }
                Ok(DemuxEvent::EndOfStream) => break,
                Ok(_) => {}
                Err(err) => unreachable!("{err}"),
            }
        }
        // Measured: the fixture's two page-terminal stamps are
        // 1.0115 s and 1.5 s — the 1.1 s target drops the first
        // (66 survivors of 67; the 65 unstamped packets pass
        // unjudged by the watermark's never-drop-on-a-guess rule).
        assert_eq!(seen, 66, "the stamped pre-target packet is dropped");
    }

    /// A past-the-end `--start-time` clamps to the declared extent
    /// (the tail-granule probe) and the watermark keeps every
    /// Present stamp at/after the extent.
    #[test]
    fn start_time_past_end_clamps_to_the_extent() {
        let mut opened = open(
            codec_fixture("vorbis_sine_44100_stereo.ogg"),
            Some(600_000_000),
        );
        assert_eq!(
            opened.start_position_ns, 1_500_000_000,
            "clamped to the 1.5 s extent"
        );
        let mut seen = 0u64;
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    seen += 1;
                    if let Stamp::Present(pts) = packet.pts {
                        let ns = pts
                            .convert(&media_clock::NANO_BASE)
                            .unwrap_or_else(|_| unreachable!("same-base convert"));
                        assert!(
                            ns.ticks >= 1_500_000_000,
                            "stamped packet {seen} at {} ns is inside the media",
                            ns.ticks
                        );
                    }
                }
                Ok(DemuxEvent::EndOfStream) => break,
                Ok(_) => {}
                Err(err) => unreachable!("{err}"),
            }
        }
        // Measured: the clamp drops the 1.0115 s page-terminal
        // stamp; the final stamp sits exactly ON the extent (not
        // before it) and the unstamped packets pass unjudged.
        assert_eq!(seen, 66);
    }

    /// The Opus fixture opens + demuxes + DECODES (6.M-opus landed —
    /// the refusal shape inverted when the decoder arrived; the
    /// blob/geometry harvest feeds it end-to-end).
    #[test]
    fn opus_fixture_opens_and_decodes() {
        let opus_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/generated/valid/ogg_opus_48k_mono.opus");
        let mut opened = open(opus_path.clone(), None);
        assert_eq!(opened.facts.container, "ogg");
        let fact = &opened.facts.facts[0];
        assert_eq!(fact.codec, "opus");
        assert_eq!(
            fact.base.as_parts(),
            (1, 48_000),
            "RFC 7845's granule clock"
        );
        assert_eq!(
            fact.audio_geometry(),
            Some((1, 48_000)),
            "OpusHead geometry; the output rate is always 48 kHz"
        );
        let blob = fact.extra_data.as_ref().unwrap_or_else(|| {
            unreachable!("the ogg harvest always stashes the opus header chain")
        });
        assert!(blob.as_slice().starts_with(b"OpusHead"));
        assert_eq!(count_packets(&mut opened), 21, "ffprobe nb_read_packets");
        // 6.M-opus landed: the opus decoder now EXISTS — the fixture
        // must decode, not refuse. Fresh open: the walk above consumed
        // the single-shot demuxer.
        let mut opened = open(opus_path, None);
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
            .unwrap_or_else(|err| unreachable!("opus decoder_for: {err}"));
        let mut sink = CountingSink(0, 0);
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    let _ = decoder
                        .decode(packet, &mut sink)
                        .unwrap_or_else(|err| unreachable!("opus fixture decode: {err}"));
                }
                Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {}
                Ok(DemuxEvent::EndOfStream) => break,
                Err(err) => unreachable!("demux: {err}"),
            }
        }
        let _ = decoder
            .flush(&mut sink)
            .unwrap_or_else(|err| unreachable!("opus fixture flush: {err}"));
        assert!(
            sink.0 > 0,
            "the opus fixture must decode PCM (got {})",
            sink.0
        );
        assert_eq!(sink.1, 1, "mono output");
    }

    /// The corpus differential (the user's standing mandate): every
    /// Ogg-family lane of corpus-v4 through the REAL open path —
    /// the vorbis lane decodes the full stream (packet count =
    /// ffprobe; sample frames = the codec card's pinned reference),
    /// the opus/theora lanes demux every ffprobe packet and refuse
    /// decode typed at the codec stage. Skips loudly when the
    /// corpus is absent (the corpus_ogg.rs convention; no
    /// `LUMEN_CORPUS` override is set by this suite).
    #[test]
    fn corpus_ogg_lanes_through_the_open_path() {
        let Some(dir) = corpus_dir() else {
            eprintln!(
                "corpus(ogg-cli): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \
                 — SKIP (the differential table for the 2026-10-24 instance is in the worklog)"
            );
            return;
        };
        // 0085 vorbis stereo 8 kHz: 126 packets, 32 256 raw sample
        // frames (the reference `raw_samples`; ffmpeg's granule-
        // trimmed window is 32 000).
        let mut counted = open(dir.join("0085-audio-only.ogg"), None);
        assert_eq!(counted.facts.container, "ogg");
        assert_eq!(counted.facts.facts[0].codec, "vorbis");
        assert_eq!(counted.facts.facts[0].audio_geometry(), Some((2, 8_000)));
        assert_eq!(count_packets(&mut counted), 126, "ffprobe nb_read_packets");
        // Fresh open for the decode pass (single-shot walk).
        let mut lane = open(dir.join("0085-audio-only.ogg"), None);
        let facts = Arc::new(lane.facts.clone());
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
        let mut sink = CountingSink(0, 0);
        loop {
            match lane.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    let _ = decoder
                        .decode(packet, &mut sink)
                        .unwrap_or_else(|err| unreachable!("0085 decode: {err}"));
                }
                Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {}
                Ok(DemuxEvent::EndOfStream) => break,
                Err(err) => unreachable!("demux: {err}"),
            }
        }
        let _ = decoder
            .flush(&mut sink)
            .unwrap_or_else(|err| unreachable!("0085 flush: {err}"));
        assert_eq!(sink.0, 32_256, "the 0085 reference raw_samples pin");
        assert_eq!(sink.1, 2);
        drop(decoder);
        // 0047 opus mono: 201 packets; 6.M-opus landed — full decode
        // (granule 192,312 − pre-skip 312 = 192,000 valid samples).
        // Fresh open for the decode pass (single-shot walk).
        let mut lane = open(dir.join("0047-audio-only.ogg"), None);
        assert_eq!(lane.facts.facts[0].codec, "opus");
        assert_eq!(lane.facts.facts[0].audio_geometry(), Some((1, 48_000)));
        assert_eq!(count_packets(&mut lane), 201, "ffprobe nb_read_packets");
        // Fresh open for the decode pass (single-shot walk).
        let mut lane = open(dir.join("0047-audio-only.ogg"), None);
        let facts = Arc::new(lane.facts.clone());
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
            .unwrap_or_else(|err| unreachable!("0047 opus decoder_for: {err}"));
        let mut sink = CountingSink(0, 0);
        loop {
            match lane.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    let _ = decoder
                        .decode(packet, &mut sink)
                        .unwrap_or_else(|err| unreachable!("0047 opus decode: {err}"));
                }
                Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {}
                Ok(DemuxEvent::EndOfStream) => break,
                Err(err) => unreachable!("demux: {err}"),
            }
        }
        let _ = decoder
            .flush(&mut sink)
            .unwrap_or_else(|err| unreachable!("0047 opus flush: {err}"));
        assert_eq!(
            sink.0, 192_000,
            "the 0047 valid-sample pin (granule − pre-skip)"
        );
        assert_eq!(sink.1, 1, "mono output");
        // 0077 opus 5.1: 201 packets, the 6-channel geometry.
        let mut lane = open(dir.join("0077-audio-only.ogg"), None);
        assert_eq!(lane.facts.facts[0].codec, "opus");
        assert_eq!(lane.facts.facts[0].audio_geometry(), Some((6, 48_000)));
        assert_eq!(count_packets(&mut lane), 201, "ffprobe nb_read_packets");
        // 0090 theora .ogv: the video lane — 2 packets at 1/2 fps,
        // the frame cadence harvested from the track base, decode
        // refused typed (no theora decoder).
        let mut lane = open(dir.join("0090-ogv-video.ogv"), None);
        assert_eq!(lane.facts.container, "ogg", "the probe names ogg for .ogv");
        let fact = &lane.facts.facts[0];
        assert_eq!(fact.kind, MediaKind::Video);
        assert_eq!(fact.codec, "theora");
        assert_eq!(
            fact.base.as_parts(),
            (2, 1),
            "one tick = one frame at 1/2 fps"
        );
        assert_eq!(fact.frame_ns, Some(2_000_000_000));
        assert_eq!(
            fact.audio_geometry(),
            None,
            "video tracks answer no geometry"
        );
        assert_eq!(count_packets(&mut lane), 2, "ffprobe nb_read_packets");
        match decoder_for(&lane) {
            Err(err) => assert_eq!(err.context.code.as_str(), "cli.codec_unavailable", "{err}"),
            Ok(_) => unreachable!("no theora decoder exists in this build"),
        }
    }

    /// The corpus media dir (the corpus_ogg.rs convention).
    fn corpus_dir() -> Option<PathBuf> {
        if let Ok(dir) = std::env::var("LUMEN_CORPUS") {
            return Some(PathBuf::from(dir));
        }
        let default = PathBuf::from("/tmp/my-project/corpus-v4/media");
        default.is_dir().then_some(default)
    }
}
