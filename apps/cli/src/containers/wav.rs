//! The WAV container host (card 6.OPW-wav — the registry's second
//! wired family, the first per-container wiring unit after the seam):
//! construct the demuxer, run the host pre-pass `start()`, harvest
//! the `fmt ` facts into [`TrackFact`]s, clamp `--start-time`
//! against the declared sample-frame span, validate it through the
//! demuxer's pure `seek` query, wrap in the shared
//! [`SelectDemuxer`](crate::demux_host::SelectDemuxer).
//!
//! The harvest's decoder-config truth per payload family (the doc
//! §4.3.3 seeding rule; the demuxer's own accessors are the only
//! source — no local copy of the tag tables):
//!
//! * **PCM** (`pcm_s16le`/`s24le`/`s32le`/`u8`/`f32le`, plain or
//!   extensible-GUID): the facts pair `channels + rate` — the
//!   provider's `with_format` seam, exactly the AVI `strf` precedent.
//! * **ADPCM** (`adpcm_ima_wav`, `adpcm_ms`): the verbatim `fmt `
//!   chunk body as `extra_data` — the WAVEFORMATEX blob the ADPCM
//!   decoder's `configure_wav` parses (tag/channels/rate/block_align,
//!   and the MS coefficient tail).
//! * **FLAC-in-WAV**: the 34-byte STREAMINFO from the parsed extensible
//!   fmt as `extra_data` — the FLAC decoder refuses to guess geometry
//!   without it (`flac.streaminfo.missing`); frames ride the packets.

use commands::CommandError;
use demux::wav::WavDemuxer;
use demux::wav::format::WavCodec;
use media_types::Track;
use media_types::packet::SharedBytes;
use media_types::time::{Stamp, TimeSpan, Timestamp};
use plugin_api::{Demuxer, SourceStream};

use crate::demux_host::{
    MediaFacts, OpenOptions, OpenedMedia, SelectDemuxer, TrackFact, wire_error,
};

/// Open a WAV source (the doc §4.2 per-container shape): the probe
/// already named `wav`; this constructs the demuxer, runs the host
/// pre-pass `start()`, harvests the `fmt ` facts, clamps
/// `--start-time` against the declared duration and wraps the demuxer
/// in the skip/selection wrapper.
///
/// # Errors
/// Typed [`CommandError`]s (stage `demux`): header/fmt parse
/// failures, seek-range violations.
pub(crate) fn open(
    stream: Box<dyn SourceStream>,
    opts: &OpenOptions,
) -> Result<OpenedMedia, CommandError> {
    // 1. Demuxer + the host pre-pass (track list + fmt facts).
    let mut demuxer = WavDemuxer::new(stream).map_err(|err| wire_error(&err))?;
    let tracks = demuxer.start().map_err(|err| wire_error(&err))?;
    let facts = harvest_facts(&demuxer, &tracks);
    let duration_us = facts.duration_us().unwrap_or(i64::MAX / 4);
    // 2. Clamp the start position (doc 05 §2.4: clamping is truth).
    let start_us = opts.start_time_us.unwrap_or(0).clamp(0, duration_us);
    let mut skip_until_ns = None;
    if start_us > 0 {
        // The pure seek query validates the target against the data
        // extent before the session exists (the target is
        // nanosecond-based — the engine clock's base).
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

/// Harvest the track facts from a started WAV demuxer: exactly one
/// audio track whose `fmt ` chunk is the whole decoder-config truth
/// (the PCM pair, the ADPCM WAVEFORMATEX body, or the FLAC
/// STREAMINFO — see the module docs).
fn harvest_facts(demuxer: &WavDemuxer, tracks: &[Track]) -> MediaFacts {
    let mut facts = Vec::with_capacity(tracks.len());
    for track in tracks {
        let (codec, pcm, extra_data) = match demuxer.format() {
            Some(fmt) => match fmt.codec {
                WavCodec::Pcm { .. } => (
                    fmt.codec_descriptor().to_string(),
                    Some((u32::from(fmt.channels), fmt.samples_per_sec)),
                    None,
                ),
                WavCodec::AdpcmImaWav { .. } | WavCodec::AdpcmMs { .. } => (
                    fmt.codec_descriptor().to_string(),
                    None,
                    demuxer
                        .fmt_chunk_bytes()
                        .map(|body| SharedBytes::from_vec(body.to_vec())),
                ),
                WavCodec::Flac { streaminfo } => (
                    fmt.codec_descriptor().to_string(),
                    None,
                    streaminfo.map(|bytes| SharedBytes::from_vec(bytes.to_vec())),
                ),
            },
            None => ("unknown".to_string(), None, None),
        };
        let span_end_ticks = track.timing.span.end.timestamp().ok().map(|ts| ts.ticks);
        facts.push(TrackFact {
            id: track.id,
            kind: track.kind,
            codec,
            base: track.timing.base,
            span_end_ticks,
            pcm,
            extra_data,
            frame_ns: None,
            dimensions: None,
        });
    }
    MediaFacts {
        tracks: tracks.to_vec(),
        facts,
        container: "wav".to_string(),
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
    use plugin_api::DemuxEvent;
    use std::path::PathBuf;
    use std::sync::Arc;
    use testkit::Must;

    fn fixture(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures")
            .join(relative)
    }

    /// The audio sink: counts decoded **sample frames** (the ffmpeg
    /// differential's unit — one `Frame::Audio` carries a whole
    /// buffer's worth; `frames_in` is the honest count).
    struct CountingSink(u64);
    impl FrameSink for CountingSink {
        fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
            if let Frame::Audio(audio) = frame {
                if let Some(frames) = audio.format.frames_in(audio.samples.len()) {
                    self.0 += u64::from(frames);
                }
            }
            Ok(())
        }
    }

    /// Open a source through the REAL CLI open path (canonicalize →
    /// access → probe → this container host).
    fn open(path: PathBuf, start_us: Option<i64>) -> crate::demux_host::OpenedMedia {
        match open_source(
            &SourceSpec::File { path },
            start_us,
            false,
            false,
        ) {
            Ok(opened) => opened,
            Err(err) => unreachable!("{err}"),
        }
    }

    /// Decode every packet of an opened media through the REAL
    /// decoder provider (the extra_data/pcm seeding path) and return
    /// the emitted sample-frame count. A typed decode refusal is
    /// RETURNED, not panicked — the corpus differential pins one
    /// known refusal (flac-in-wav) instead of hiding it.
    fn decode_frames(
        opened: &mut crate::demux_host::OpenedMedia,
    ) -> Result<u64, EngineError> {
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
        let mut sink = CountingSink(0);
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
        Ok(sink.0)
    }

    /// The shared PCM fixture opens end-to-end with the recorded
    /// facts: one audio track, the `pcm_s16le` descriptor, the
    /// channels/rate pair, no extra_data (PCM's truth is the pair).
    #[test]
    fn pcm_fixture_opens_with_expected_facts() {
        let opened = open(fixture("generated/valid/wav_pcm_s16le_44k1_st.wav"), None);
        assert_eq!(opened.facts.container, "wav");
        assert_eq!(opened.facts.tracks.len(), 1);
        let fact = &opened.facts.facts[0];
        assert_eq!(fact.kind, MediaKind::Audio);
        assert_eq!(fact.codec, "pcm_s16le");
        assert_eq!(fact.pcm, Some((2, 44_100)));
        assert!(fact.extra_data.is_none(), "PCM carries no blob");
        // One tick = one sample frame: the span end is the frame count.
        let frames = u64::try_from(fact.span_end_ticks.unwrap_or(0)).unwrap_or(0);
        assert_eq!(frames, opened.facts.tracks[0].timing.span.end.timestamp().unwrap().ticks as u64);
        assert_eq!(opened.start_position_ns, 0);
    }

    /// Every PCM flavor fixture in the tree opens through the host —
    /// including the extensible-GUID form and RF64 (the §5 checklist
    /// AC paths): container `wav`, one audio track, a `pcm_`
    /// descriptor with its facts pair.
    #[test]
    fn all_wav_fixture_flavors_open() {
        for name in [
            "wav_pcm_s16le_44k1_st.wav",
            "wav_pcm_s24le_48k_st.wav",
            "wav_pcm_f32le_48k_st.wav",
            "wav_pcm_u8_44k1_mono.wav",
            "wav_extensible_6ch_48k.wav",
            "wav_rf64_44k1_st.wav",
        ] {
            let path = fixture(&format!("generated/valid/{name}"));
            assert!(path.is_file(), "fixture missing: {path:?}");
            let opened = open(path, None);
            assert_eq!(opened.facts.container, "wav", "{name}");
            assert_eq!(opened.facts.tracks.len(), 1, "{name}");
            let fact = &opened.facts.facts[0];
            assert!(fact.codec.starts_with("pcm_"), "{name}: {}", fact.codec);
            assert!(fact.pcm.is_some(), "{name}: pcm facts pair");
        }
    }

    /// A minimal hand-built IMA ADPCM WAV: the `fmt ` body flows into
    /// `extra_data` and the ADPCM decoder configures from it and
    /// decodes the full block (wSamplesPerBlock frames out).
    #[test]
    fn adpcm_extra_data_flows_into_the_decoder() {
        // fmt body (20 bytes): tag 0x0011, 1 channel, 8000 Hz,
        // block_align 256, 4 bits, cbSize 2, wSamplesPerBlock 505
        // (the IMA formula: ((256 − 4)·2)/1 + 1).
        let mut fmt = Vec::new();
        fmt.extend_from_slice(&0x0011u16.to_le_bytes());
        fmt.extend_from_slice(&1u16.to_le_bytes());
        fmt.extend_from_slice(&8_000u32.to_le_bytes());
        fmt.extend_from_slice(&(8_000u32 * 256 / 505).to_le_bytes());
        fmt.extend_from_slice(&256u16.to_le_bytes());
        fmt.extend_from_slice(&4u16.to_le_bytes());
        fmt.extend_from_slice(&2u16.to_le_bytes());
        fmt.extend_from_slice(&505u16.to_le_bytes());
        // One block: 4-byte preamble (predictor 0, index 0, reserved)
        // + 252 zero nibble bytes.
        let block = vec![0u8; 256];
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        let data_len = u32::try_from(block.len()).must();
        let riff_len =
            u32::try_from(4 + 8 + fmt.len() + 8 + 4 + 8 + block.len()).must();
        wav.extend_from_slice(&riff_len.to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&u32::try_from(fmt.len()).must().to_le_bytes());
        wav.extend_from_slice(&fmt);
        wav.extend_from_slice(b"fact");
        wav.extend_from_slice(&4u32.to_le_bytes());
        wav.extend_from_slice(&505u32.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        wav.extend_from_slice(&block);
        let path = std::env::temp_dir().join("lumen-cli-adpcm-ima.wav");
        std::fs::write(&path, &wav).must();
        let opened = open(path.clone(), None);
        std::fs::remove_file(&path).ok();
        assert_eq!(opened.facts.container, "wav");
        let fact = &opened.facts.facts[0];
        assert_eq!(fact.codec, "adpcm_ima_wav");
        assert!(fact.pcm.is_none(), "ADPCM's truth is the blob, not the pair");
        let extra = fact.extra_data.as_ref().expect("fmt body in extra_data");
        assert_eq!(extra.len(), 20, "the verbatim fmt chunk body");
        assert_eq!(&extra.as_slice()[..2], &0x0011u16.to_le_bytes());
        // Decode through the real provider: 505 sample frames out of
        // one 256-byte block.
        let frames = {
            let mut opened = opened;
            decode_frames(&mut opened).unwrap_or_else(|err| unreachable!("{err}"))
        };
        assert_eq!(frames, 505, "one block = wSamplesPerBlock frames");
    }

    /// `--start-time` clamps to the declared duration and the seek
    /// query validates the target (the doc's Fast seek): a target
    /// past the end admits no packet; an in-media target lands every
    /// surviving packet at/after it.
    #[test]
    fn start_time_clamps_and_skips() {
        let path = fixture("generated/valid/wav_pcm_s16le_44k1_st.wav");
        let mut past = open(path.clone(), Some(i64::MAX / 8));
        assert!(past.start_position_ns > 0, "clamped to the span, not zero");
        let _ = past.demuxer.start().unwrap_or_else(|e| unreachable!("{e}"));
        let mut packets = 0u32;
        loop {
            match past.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    packets += 1;
                    if let Stamp::Present(pts) = packet.pts {
                        let ns = pts
                            .convert(&media_clock::NANO_BASE)
                            .unwrap_or_else(|e| unreachable!("{e:?}"));
                        assert!(
                            ns.ticks >= past.start_position_ns,
                            "packet before the clamped start survived: {}",
                            ns.ticks
                        );
                    }
                }
                Ok(DemuxEvent::EndOfStream) => break,
                Ok(_) => {}
                Err(err) => unreachable!("{err}"),
            }
        }
        assert_eq!(packets, 0, "the whole media predates the clamped end");

        // An in-media target: every surviving packet lands at/after it.
        let mut mid = open(path, Some(1_000));
        assert_eq!(mid.start_position_ns, 1_000_000);
        let _ = mid.demuxer.start().unwrap_or_else(|e| unreachable!("{e}"));
        loop {
            match mid.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
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
    }

    /// A magic-intact WAV whose `fmt ` chunk is destroyed refuses
    /// honestly at the demux stage (the probe still names `wav`; the
    /// container host's construction/start is the typed refusal).
    #[test]
    fn broken_fmt_chunk_refuses_at_the_demux_stage() {
        let bytes = std::fs::read(fixture("generated/valid/wav_pcm_s16le_44k1_st.wav"))
            .unwrap_or_else(|e| unreachable!("fixture unreadable: {e}"));
        let at = bytes
            .windows(4)
            .position(|window| window == b"fmt ")
            .unwrap_or_else(|| unreachable!("no fmt chunk"));
        let mut broken = bytes.clone();
        broken[at..at + 4].copy_from_slice(b"xxxx");
        let path = std::env::temp_dir().join("lumen-cli-wav-broken-fmt.wav");
        std::fs::write(&path, &broken).must();
        let err = match open_source(
            &SourceSpec::File { path: path.clone() },
            None,
            false,
            false,
        ) {
            Ok(_) => unreachable!("a destroyed fmt chunk must refuse"),
            Err(err) => err,
        };
        std::fs::remove_file(&path).ok();
        assert_eq!(err.stage, commands::Stage::Demux, "{err}");
    }

    // -----------------------------------------------------------------
    // The corpus differential (the user's standing mandate): every
    // real WAV lane of corpus-v4 opened through the CLI open path and
    // decoded through the real decoder provider, the emitted sample
    // frames compared against the ffmpeg-measured truth (the same
    // TRUTHS as crates/demux/tests/corpus_wav.rs — ffprobe
    // duration_ts for PCM, ffmpeg's decoded output for the two ADPCM
    // under-declarations, the fact declaration for flac-in-wav).
    // -----------------------------------------------------------------

    struct CorpusTruth {
        file: &'static str,
        codec: &'static str,
        rate: u32,
        channels: u16,
        /// ffmpeg's decoded sample-frame count.
        truth_frames: u64,
    }

    const TRUTHS: [CorpusTruth; 12] = [
        CorpusTruth { file: "0004-audio-only.wav", codec: "pcm_s24le", rate: 8_000, channels: 1, truth_frames: 32_000 },
        CorpusTruth { file: "0006-audio-only.wav", codec: "pcm_s24le", rate: 176_400, channels: 2, truth_frames: 388_433 },
        CorpusTruth { file: "0013-sr-chaos.wav", codec: "pcm_s16le", rate: 11_025, channels: 1, truth_frames: 14_796 },
        CorpusTruth { file: "0016-audio-only.wav", codec: "flac", rate: 96_000, channels: 4, truth_frames: 384_000 },
        CorpusTruth { file: "0026-audio-only.wav", codec: "pcm_s16le", rate: 11_025, channels: 2, truth_frames: 44_100 },
        CorpusTruth { file: "0041-audio-only.wav", codec: "pcm_s16le", rate: 8_000, channels: 1, truth_frames: 32_000 },
        CorpusTruth { file: "0042-audio-only.wav", codec: "adpcm_ima_wav", rate: 44_056, channels: 2, truth_frames: 168_822 },
        CorpusTruth { file: "0095-audio-only.wav", codec: "pcm_s32le", rate: 44_100, channels: 1, truth_frames: 176_400 },
        CorpusTruth { file: "0102-audio-only.wav", codec: "adpcm_ms", rate: 32_000, channels: 2, truth_frames: 128_524 },
        CorpusTruth { file: "0129-sr-chaos.wav", codec: "pcm_s16le", rate: 22_050, channels: 4, truth_frames: 24_255 },
        CorpusTruth { file: "0135-audio-only.wav", codec: "pcm_u8", rate: 22_050, channels: 2, truth_frames: 61_233 },
        CorpusTruth { file: "0138-audio-only.wav", codec: "pcm_s16le", rate: 16_000, channels: 6, truth_frames: 64_000 },
    ];

    /// The corpus media dir: the `LUMEN_CORPUS` override, else the
    /// sandbox default. `None` when neither exists (the loud skip —
    /// the crates/demux/tests/corpus_wav.rs convention).
    fn corpus_dir() -> Option<PathBuf> {
        if let Ok(dir) = std::env::var("LUMEN_CORPUS") {
            return Some(PathBuf::from(dir));
        }
        let default = PathBuf::from("/tmp/my-project/corpus-v4/media");
        default.is_dir().then_some(default)
    }

    #[test]
    fn corpus_wav_lanes_decode_through_the_open_path() {
        let Some(dir) = corpus_dir() else {
            eprintln!(
                "corpus(wav-cli): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS \
                 unset — SKIP (the differential table for the 2026-10-24 instance is in \
                 the worklog)"
            );
            return;
        };
        let mut table = Vec::new();
        for truth in &TRUTHS {
            let path = dir.join(truth.file);
            assert!(path.is_file(), "corpus file missing: {path:?}");
            let mut opened = open(path, None);
            assert_eq!(opened.facts.container, "wav", "{}", truth.file);
            assert_eq!(opened.facts.tracks.len(), 1, "{}", truth.file);
            let fact = opened.facts.facts[0].clone();
            assert_eq!(fact.codec, truth.codec, "{}", truth.file);
            let expected_pair = Some((u32::from(truth.channels), truth.rate));
            match truth.codec {
                "flac" => {
                    let extra = fact
                        .extra_data
                        .as_ref()
                        .unwrap_or_else(|| unreachable!("{}: STREAMINFO blob", truth.file));
                    assert_eq!(extra.len(), 34, "{}: 34-byte STREAMINFO", truth.file);
                }
                codec if codec.starts_with("adpcm_") => {
                    assert!(fact.extra_data.is_some(), "{}: fmt blob", truth.file);
                    assert_eq!(fact.pcm, None, "{}", truth.file);
                }
                _ => assert_eq!(fact.pcm, expected_pair, "{}", truth.file),
            }
            // The full decode through the real provider: the emitted
            // sample frames equal ffmpeg's decoded output. The
            // flac-in-wav lane is the one PINNED gap: the demuxer's
            // byte-quantum packetization has no FLAC frame alignment
            // (the demuxer's own [PROPOSAL] defers the frame-header
            // grammar to the FLAC codec card), so the decoder's typed
            // `flac.frame.structure` refusal is the honest answer —
            // asserted by name, never smoothed over.
            let frames = match decode_frames(&mut opened) {
                Ok(frames) => frames,
                Err(err) if truth.codec == "flac" => {
                    assert!(
                        err.context.code.contains("flac.frame"),
                        "{}: the pinned flac-in-wav gap moved: {err}",
                        truth.file
                    );
                    table.push(format!(
                        "{} flac {}ch {}Hz -> GAP flac-in-wav byte-quantum packets \
                         (frame-alignment deferred to the FLAC codec card): {err}",
                        truth.file, truth.channels, truth.rate
                    ));
                    continue;
                }
                Err(err) => unreachable!("{}: {err}", truth.file),
            };
            table.push(format!(
                "{} {} {}ch {}Hz -> {} frames",
                truth.file, truth.codec, truth.channels, truth.rate, frames
            ));
            assert_eq!(
                frames, truth.truth_frames,
                "{}: decoded frames vs ffmpeg truth",
                truth.file
            );
        }
        eprintln!("corpus(wav-cli) differential:\n  {}", table.join("\n  "));
    }
}
