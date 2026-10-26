//! The ADTS/ADIF container host (card 6.OPW-adts — the registry's
//! fourth wired family, the raw-AAC stream formats of ISO/IEC
//! 13818-7): construct the demuxer, run the host pre-pass `start()`,
//! harvest the header's stream facts into [`TrackFact`]s, validate
//! `--start-time` through the demuxer's pure `seek` query (SlowLinear
//! frame scan for ADTS, format-inherent arithmetic for ADIF), wrap in
//! the shared
//! [`SelectDemuxer`](crate::demux_host::SelectDemuxer).
//!
//! The harvest's decoder-config truth (the doc §4.3.3 rule; the
//! demuxer's [`stream_info`](demux::adts::AdtsDemuxer::stream_info)
//! accessors are the facts source — the OPW-0 doc's own citation):
//!
//! * **The codec descriptor is the header's profile-honest mapping**
//!   (`aac-lc`, `aac-main`, `aac-ssr`, `aac-ltp` — the demuxer's own
//!   `descriptor()`, never a local copy). The AAC-LC decoder matches
//!   `aac`/`aac-lc` only; the other profiles open honestly and the
//!   provider refuses them typed at decode (LC-only by card 5.C2a).
//! * **No `extra_data`**: the whole decoder config rides **in-band**
//!   — every ADTS packet carries its full frame header, and the
//!   decoder's ADTS mode builds its engine from the first packet's
//!   header (the `pipeline_host` `aac`/`aac-lc` arm seeds the blob
//!   only when one exists; seeding a synthetic ASC here would flip
//!   the decoder into raw-block mode and misframe every packet —
//!   `None` is the correct wiring, not an omission). The demuxer
//!   exposes no ASC (raw AAC's config is the per-frame header), so
//!   there is nothing to harvest.
//! * **No PCM pair**: the track is not PCM — `channels`/`rate` live
//!   in the ADTS header the decoder parses itself; the facts carry
//!   only the sample-clock base (one tick = one sample).
//! * **No declared duration**: the track's span end is `Missing` at
//!   open (the demuxer's exact duration exists only after the walk —
//!   ffprobe's own raw-ADTS duration is a bitrate *estimate*). There
//!   is nothing to clamp `--start-time` against; the pure seek query
//!   is the validator (a past-the-end target lands at the last
//!   frame, and the skip watermark then admits no packet — the same
//!   honest shape as a clamped refusal).

use commands::CommandError;
use demux::adts::AdtsDemuxer;
use media_types::Track;
use media_types::time::{Stamp, TimeSpan, Timestamp};
use plugin_api::{Demuxer, SourceStream};

use crate::demux_host::{
    MediaFacts, OpenOptions, OpenedMedia, SelectDemuxer, TrackFact, wire_error,
};

/// Open an ADTS/ADIF source (the doc §4.2 per-container shape): the
/// probe already named `adts` (both the frame format and the `ADIF`
/// magic share the id); this constructs the demuxer, runs the host
/// pre-pass `start()`, harvests the header facts, validates the
/// start position through the seek query and wraps the demuxer in
/// the skip/selection wrapper.
///
/// # Errors
/// Typed [`CommandError`]s (stage `demux`): no usable syncword within
/// the open-scan window, a first frame with a reserved sampling
/// index or a PCE channel configuration, CRC mismatches, ADIF
/// VBR/multi-program refusals, seek-range violations.
pub(crate) fn open(
    stream: Box<dyn SourceStream>,
    opts: &OpenOptions,
) -> Result<OpenedMedia, CommandError> {
    // 1. Demuxer + the host pre-pass (track list + header facts).
    let mut demuxer = AdtsDemuxer::new(stream).map_err(|err| wire_error(&err))?;
    let tracks = demuxer.start().map_err(|err| wire_error(&err))?;
    let facts = harvest_facts(&demuxer, &tracks);
    // 2. The start position (doc 05 §2.4): raw AAC declares no
    //    duration up front (`duration_us` is None — the exact figure
    //    exists only post-walk), so there is no span to clamp
    //    against; the pure seek query validates the target instead.
    let start_us = opts.start_time_us.unwrap_or(0).max(0);
    let mut skip_until_ns = None;
    if start_us > 0 {
        // The pure seek query (SlowLinear for ADTS — a frame-header
        // scan bounded by the file) lands at-or-before the target; a
        // past-the-end target lands at the last frame and the skip
        // watermark below then admits no packet.
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

/// Harvest the track facts from a started ADTS/ADIF demuxer: exactly
/// one audio track; the profile-honest codec descriptor and the
/// sample-clock base are the track's own truth, `stream_info()` the
/// demuxer-side confirmation (format/rate/channels/profile — the
/// fields ffprobe reports). The span end stays `None` (raw AAC's
/// exact duration is walk-derived), and no blob is harvested (the
/// config rides in-band — see the module docs).
fn harvest_facts(demuxer: &AdtsDemuxer, tracks: &[Track]) -> MediaFacts {
    // `stream_info()` (format/rate/channels/profile — the fields
    // ffprobe reports) is the demuxer-side confirmation of the same
    // header the track was minted from: the sample clock's
    // denominator IS the declared rate. Pinned as the harvest's
    // invariant (divergence would mean the track and the accessor
    // disagree about one header — a demuxer bug, never a wiring
    // guess).
    if let Some(info) = demuxer.stream_info()
        && let Some(track) = tracks.first()
    {
        debug_assert_eq!(
            track.timing.base.as_parts().1,
            info.rate,
            "the sample clock is the header's declared rate"
        );
    }
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
        container: "adts".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demux_host::open_source;
    use crate::pipeline_host::HostDecoderProvider;
    use codec_api::{DecoderConfig, FrameSink, ThreadBudget};
    use commands::catalog::types::SourceSpec;
    use demux::adts::SAMPLES_PER_BLOCK;
    use media_types::{EngineError, Frame, GenerationId, MediaKind};
    use playback_engine::DecoderProvider;
    use plugin_api::DemuxEvent;
    use std::path::PathBuf;
    use std::sync::Arc;
    use testkit::Must;

    /// The committed AAC fixture set of the codecs crate (card
    /// 5.C2a, budget-checked): real ffmpeg-authored ADTS bytes, the
    /// corpus-independent lane for this suite (the wav.rs
    /// `generated/valid/` precedent — no ADTS fixture exists under
    /// the root `tests/fixtures`, so the bytes are borrowed from the
    /// codec card's own committed set).
    fn codec_fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/codecs/tests/fixtures")
            .join(name)
    }

    /// The audio sink: counts decoded **sample frames** (the ffmpeg
    /// differential's unit — one `Frame::Audio` carries a whole
    /// buffer's worth; `frames_in` is the honest count) and remembers
    /// the negotiated channel count (the header's channels, confirmed
    /// by the decode).
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

    /// Open a source through the REAL CLI open path (canonicalize →
    /// access → probe → this container host).
    fn open(path: PathBuf, start_us: Option<i64>) -> crate::demux_host::OpenedMedia {
        match open_source(&SourceSpec::File { path }, start_us, false, false) {
            Ok(opened) => opened,
            Err(err) => unreachable!("{err}"),
        }
    }

    /// Decode every packet of an opened media through the REAL
    /// decoder provider (the in-band ADTS config path) and return the
    /// emitted sample-frame count plus the negotiated channel count.
    /// A typed refusal is RETURNED, never panicked — the dirty lane
    /// pins one by name.
    fn decode_frames(
        opened: &mut crate::demux_host::OpenedMedia,
    ) -> Result<(u64, u32), EngineError> {
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

    /// The mono 48 kHz committed fixture opens through the real open
    /// path with the recorded facts: one audio track, the
    /// profile-honest `aac-lc` descriptor, the sample-clock base
    /// (one tick = one sample), **no PCM pair and no blob** (the
    /// whole decoder config rides in-band — see the module docs),
    /// and **no declared extent** (raw AAC's duration is
    /// walk-derived).
    #[test]
    fn fixture_opens_with_expected_facts() {
        let opened = open(codec_fixture("sine48_lc_mono.aac"), None);
        assert_eq!(opened.facts.container, "adts");
        assert_eq!(opened.facts.tracks.len(), 1);
        let track = &opened.facts.tracks[0];
        assert_eq!(track.kind, MediaKind::Audio);
        let fact = &opened.facts.facts[0];
        assert_eq!(fact.codec, "aac-lc");
        assert_eq!(fact.base.as_parts(), (1, 48_000), "the sample clock");
        assert!(fact.pcm.is_none(), "AAC is not PCM — no pair");
        assert!(fact.extra_data.is_none(), "the config rides in-band");
        assert!(
            fact.span_end_ticks.is_none(),
            "raw AAC declares no extent at open"
        );
        assert_eq!(opened.start_position_ns, 0);
    }

    /// Both committed fixtures (mono 48 kHz + stereo 44.1 kHz) demux
    /// AND decode through the real open path: the packet count is
    /// ffprobe's own `show_packets` count and the emitted sample
    /// frames are the exact `packets × 1024` arithmetic (every
    /// AAC-LC frame carries 1024 samples; the 5.C2a differential
    /// pinned ffmpeg decoding exactly this window).
    #[test]
    fn fixtures_decode_the_full_stream() {
        // (file, rate, channels, ffprobe packets)
        for (name, rate, channels, packets) in [
            ("sine48_lc_mono.aac", 48_000u32, 1u16, 142u64),
            ("sine441_lc_stereo.aac", 44_100, 2, 131),
        ] {
            let path = codec_fixture(name);
            assert!(path.is_file(), "fixture missing: {path:?}");
            let mut opened = open(path.clone(), None);
            assert_eq!(opened.facts.container, "adts", "{name}");
            assert_eq!(opened.facts.facts[0].codec, "aac-lc", "{name}");
            assert_eq!(
                opened.facts.tracks[0].timing.base.as_parts(),
                (1, rate),
                "{name}: the sample clock"
            );
            let mut counted = 0u64;
            loop {
                match opened.demuxer.next_event() {
                    Ok(DemuxEvent::Packet(_)) => counted += 1,
                    Ok(DemuxEvent::EndOfStream) => break,
                    Ok(_) => {}
                    Err(err) => unreachable!("{name}: {err}"),
                }
            }
            assert_eq!(counted, packets, "{name}: ffprobe packet count");
            // Pass 2 — the full decode through the real provider on a
            // fresh open (the walk is single-shot).
            let mut opened = open(path, None);
            let (frames, decoded_channels) = decode_frames(&mut opened)
                .unwrap_or_else(|err| unreachable!("{name}: {err}"));
            assert_eq!(
                frames,
                packets * SAMPLES_PER_BLOCK,
                "{name}: decoded sample frames (packets × 1024)"
            );
            assert_eq!(decoded_channels, u32::from(channels), "{name}");
        }
    }

    /// `--start-time` through the pure seek query (SlowLinear): an
    /// in-media target lands every surviving packet at/after it; a
    /// past-the-end target (there is no declared duration to clamp
    /// against) lands at the last frame and admits no packet — the
    /// honest no, never a guess.
    #[test]
    fn start_time_skips_packets() {
        let path = codec_fixture("sine48_lc_mono.aac");
        let mut mid = open(path.clone(), Some(1_000));
        assert_eq!(mid.start_position_ns, 1_000_000);
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

    /// A syncword-intact stream whose first frame declares a reserved
    /// sampling index (13–15, outside the MPEG-4 rate table) refuses
    /// honestly at the demux stage (the probe still names `adts`; the
    /// container host's `start` is the typed refusal).
    #[test]
    fn reserved_sampling_index_refuses_at_the_demux_stage() {
        let bytes = demux::adts::test_support::adts_frame(1, 15, 2, 1, 100, None);
        let path = std::env::temp_dir().join("lumen-cli-adts-bad-index.aac");
        std::fs::write(&path, &bytes).must();
        let err =
            match open_source(&SourceSpec::File { path: path.clone() }, None, false, false) {
                Ok(_) => unreachable!("a reserved sampling index must refuse"),
                Err(err) => err,
            };
        std::fs::remove_file(&path).ok();
        assert_eq!(err.stage, commands::Stage::Demux, "{err}");
        assert_eq!(
            err.details.get("code").map(String::as_str),
            Some("demux.adts.bad-sampling-index"),
            "{err}"
        );
    }

    /// A Main-profile ADTS stream (the demuxer's honest
    /// `aac-main` descriptor) **opens** — the container is fine, the
    /// facts are true — and the decoder provider refuses it typed at
    /// decode: the AAC decoder is LC-only by card 5.C2a, and the
    /// boundary is the provider's, never a silent decode of garbage.
    #[test]
    fn main_profile_opens_but_refuses_the_decoder() {
        let mut stream = Vec::new();
        for _ in 0..3 {
            stream.extend_from_slice(&demux::adts::test_support::adts_frame(
                0, 4, 2, 1, 100, None,
            ));
        }
        let path = std::env::temp_dir().join("lumen-cli-adts-main.aac");
        std::fs::write(&path, &stream).must();
        let opened = open(path.clone(), None);
        std::fs::remove_file(&path).ok();
        assert_eq!(opened.facts.container, "adts");
        assert_eq!(opened.facts.facts[0].codec, "aac-main");
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
        let err = match provider.decoder_for(&track, &config) {
            Ok(_) => unreachable!("aac-main must refuse the LC-only decoder"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "cli.codec_unavailable", "{err}");
    }

    // -----------------------------------------------------------------
    // The corpus differential (the user's standing mandate): every
    // real raw-AAC lane of corpus-v4 opened through the CLI open path
    // and decoded through the real decoder provider, the packet count
    // and emitted sample frames compared against the ffmpeg-measured
    // truth (the same measurements as
    // crates/demux/tests/adts_corpus.rs packets and
    // crates/codecs/tests/aac_corpus_differential.rs samples —
    // ffprobe counts 346/376 packets and ffmpeg decodes exactly
    // `packets × 1024` samples of each lane).
    // -----------------------------------------------------------------

    struct CorpusTruth {
        file: &'static str,
        /// ffprobe `codec_name,profile` → the demuxer's descriptor.
        codec: &'static str,
        /// The track's time base (num, den) — one tick = one sample.
        base: (u32, u32),
        /// ffprobe `-show_packets` packet count.
        packets: u64,
        /// ffmpeg's decoded sample-frame count (packets × 1024).
        truth_frames: u64,
    }

    const TRUTHS: [CorpusTruth; 2] = [
        CorpusTruth {
            file: "0059-audio-only.aac",
            codec: "aac-lc",
            base: (1, 88_200),
            packets: 346,
            truth_frames: 354_304, // 346 × 1024
        },
        CorpusTruth {
            file: "0110-audio-only.aac",
            codec: "aac-lc",
            base: (1, 96_000),
            packets: 376,
            truth_frames: 385_024, // 376 × 1024
        },
    ];

    /// The corpus media dir: the `LUMEN_CORPUS` override, else the
    /// sandbox default. `None` when neither exists (the loud skip —
    /// the crates/demux/tests/adts_corpus.rs convention).
    fn corpus_dir() -> Option<PathBuf> {
        if let Ok(dir) = std::env::var("LUMEN_CORPUS") {
            return Some(PathBuf::from(dir));
        }
        let default = PathBuf::from("/tmp/my-project/corpus-v4/media");
        default.is_dir().then_some(default)
    }

    #[test]
    fn corpus_adts_lanes_decode_through_the_open_path() {
        let Some(dir) = corpus_dir() else {
            eprintln!(
                "corpus(adts-cli): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS \
                 unset — SKIP (the differential table for this instance is in the worklog)"
            );
            return;
        };
        let mut table = Vec::new();
        for truth in &TRUTHS {
            let path = dir.join(truth.file);
            assert!(path.is_file(), "corpus file missing: {path:?}");
            let mut opened = open(path, None);
            assert_eq!(opened.facts.container, "adts", "{}", truth.file);
            assert_eq!(opened.facts.tracks.len(), 1, "{}", truth.file);
            assert_eq!(opened.facts.tracks[0].kind, MediaKind::Audio, "{}", truth.file);
            assert_eq!(opened.facts.facts[0].codec, truth.codec, "{}", truth.file);
            assert_eq!(
                opened.facts.tracks[0].timing.base.as_parts(),
                truth.base,
                "{}: the sample clock",
                truth.file
            );
            assert!(
                opened.facts.facts[0].extra_data.is_none(),
                "{}: in-band config, no blob",
                truth.file
            );
            // The full decode through the real provider: the emitted
            // sample frames equal ffmpeg's decoded output.
            let (frames, _) = decode_frames(&mut opened)
                .unwrap_or_else(|err| unreachable!("{}: {err}", truth.file));
            assert_eq!(
                frames, truth.truth_frames,
                "{}: decoded frames vs ffmpeg truth",
                truth.file
            );
            table.push(format!(
                "{} {} {}Hz -> {} packets, {} frames",
                truth.file, truth.codec, truth.base.1, truth.packets, frames
            ));
        }
        eprintln!("corpus(adts-cli) differential:\n  {}", table.join("\n  "));
    }

    /// The dirty lane through the open path (the recorded divergence,
    /// crates/demux/tests/adts_corpus.rs `corpus_dirty_tail_garbage_is_honest`):
    /// 0059 verbatim plus 39 757 random tail bytes. The demux half:
    /// the 346 real frames plus ffmpeg's four complete resync frames
    /// (350 packets), the garbage recovered loudly, the overrunning
    /// fifth frame refusing as the typed sticky truncation (ffmpeg
    /// emits it as a short 3405-byte packet — 351 — the one recorded
    /// divergence). The decode half: the real audio decodes to the
    /// exact clean-lane count, then the first garbage frame refuses
    /// typed — honest damage, never a panic, never a wrong-rate
    /// decode.
    #[test]
    fn corpus_dirty_tail_lane_is_honest_through_the_open_path() {
        let Some(dir) = corpus_dir() else {
            eprintln!(
                "corpus(adts-cli): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS \
                 unset — SKIP (dirty lane)"
            );
            return;
        };
        let dirty = dir
            .parent()
            .map(|root| root.join("dirty/tail-garbage-003-0059-audio-only.aac"))
            .filter(|p| p.is_file());
        let Some(path) = dirty else {
            eprintln!("corpus(adts-cli): no dirty aac lane this instance — SKIP");
            return;
        };

        // Half 1 — the demux walk through the wrapper: packet count,
        // loud recovery, the typed sticky truncation.
        let mut opened = open(path.clone(), None);
        assert_eq!(opened.facts.container, "adts");
        assert_eq!(opened.facts.facts[0].codec, "aac-lc");
        let mut packets = 0u64;
        let mut recovered = 0usize;
        let mut terminal = String::new();
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(_)) => packets += 1,
                Ok(DemuxEvent::Recovered { .. }) => recovered += 1,
                // The resync walk marks its damage boundary with a
                // Discontinuity and keeps walking (the demux suite's
                // convention) — the recovered garbage frames follow it.
                Ok(DemuxEvent::Discontinuity) => {}
                Ok(DemuxEvent::EndOfStream) => break,
                Err(err) => {
                    terminal = err.context.code.to_owned();
                    break;
                }
            }
        }
        assert_eq!(
            packets,
            TRUTHS[0].packets + 4,
            "346 real + 4 complete garbage frames (ffmpeg: 351 — the short-packet \
             divergence, recorded in the demux card's worklog)"
        );
        assert!(recovered > 0, "the garbage tail is recovered, not silent");
        assert_eq!(
            terminal.as_str(),
            "demux.adts.truncated",
            "the overrunning fifth garbage frame refuses typed"
        );
        // Sticky through the wrapper: the truncation replays.
        let again = opened.demuxer.next_event();
        assert_eq!(
            again.err().map(|e| e.context.code),
            Some("demux.adts.truncated".to_string())
        );

        // Half 2 — the decode walk: the real audio decodes to the
        // clean-lane count, then the garbage refuses typed.
        let mut opened = open(path, None);
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
        let mut refusal = String::new();
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    if let Err(err) = decoder.decode(packet, &mut sink) {
                        refusal = err.context.code.to_owned();
                        break;
                    }
                }
                Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {}
                Ok(DemuxEvent::EndOfStream) => break,
                Err(err) => {
                    refusal = err.context.code.to_owned();
                    break;
                }
            }
        }
        assert_eq!(
            sink.0,
            TRUTHS[0].truth_frames,
            "the real audio decodes to the clean-lane count before the damage"
        );
        assert!(
            !refusal.is_empty(),
            "the garbage tail refuses typed, never decodes silently"
        );
        eprintln!(
            "corpus(adts-cli): PASS dirty/tail-garbage-003 346-exact real frames ({0} sample \
             frames) + 4 recovered garbage frames, typed refusal `{refusal}` (demux terminal: \
             {terminal}; ffprobe: 351 — the short-packet divergence)",
            TRUTHS[0].truth_frames
        );
    }
}
