//! The AIFF container host (card 6.OPW-aiff — the registry's third
//! wired family, the WAV twin's big-endian sibling): construct the
//! demuxer, run the host pre-pass `start()`, harvest the COMM facts
//! into [`TrackFact`]s, clamp `--start-time` against the declared
//! sample-frame span, validate it through the demuxer's pure `seek`
//! query (Fast, non-indexed — the SSND extent arithmetic), wrap in the
//! shared [`SelectDemuxer`](crate::demux_host::SelectDemuxer).
//!
//! The harvest's decoder-config truth per payload family (the doc
//! §4.3.3 seeding rule; the demuxer's own accessors are the only
//! source — no local copy of the compression tables):
//!
//! * **Every form the demuxer exposes is a PCM descriptor**: classic
//!   AIFF / AIFC `NONE` (big-endian `pcm_s16be`/`s24be`/`s32be`),
//!   AIFC `sowt` (little-endian `pcm_s16le`/`s24le`/`s32le`),
//!   `fl32`/`fl64` (big-endian float) and `raw ` (unsigned 8-bit,
//!   the ffmpeg AIFC mapping). The facts pair `channels + rate` is
//!   the whole decoder-config truth for every one of them — the
//!   provider's `with_format` seam, exactly the WAV PCM precedent;
//!   no `extra_data` is harvested (the demuxer's
//!   [`comm_chunk_bytes`](demux::aiff::AiffDemuxer::comm_chunk_bytes)
//!   seam stays available for the first non-PCM form that lands).
//! * **Everything else refuses before the harvest exists**: the
//!   COMM parse itself rejects compression types outside that
//!   vocabulary (`ima4`, `ulaw`, `alaw`, … and classic AIFF's signed
//!   8-bit) with the typed `demux.aiff.unsupported-compression`, and
//!   truncated/malformed structure with its own typed codes — the
//!   container host surfaces the demuxer's refusal; nothing is
//!   guessed and no lane is silently dropped.

use commands::CommandError;
use demux::aiff::AiffDemuxer;
use media_types::Track;
use media_types::time::{Stamp, TimeSpan, Timestamp};
use plugin_api::{Demuxer, SourceStream};

use crate::demux_host::{
    MediaFacts, OpenOptions, OpenedMedia, SelectDemuxer, TrackFact, wire_error,
};

/// Open an AIFF/AIFC source (the doc §4.2 per-container shape): the
/// probe already named `aiff` (both forms — classic `AIFF` and `AIFC`
/// share the id); this constructs the demuxer, runs the host pre-pass
/// `start()`, harvests the COMM facts, clamps `--start-time` against
/// the declared duration and wraps the demuxer in the skip/selection
/// wrapper.
///
/// # Errors
/// Typed [`CommandError`]s (stage `demux`): FORM/COMM/SSND parse
/// failures, unsupported compression types, seek-range violations.
pub(crate) fn open(
    stream: Box<dyn SourceStream>,
    opts: &OpenOptions,
) -> Result<OpenedMedia, CommandError> {
    // 1. Demuxer + the host pre-pass (track list + COMM facts).
    let mut demuxer = AiffDemuxer::new(stream).map_err(|err| wire_error(&err))?;
    let tracks = demuxer.start().map_err(|err| wire_error(&err))?;
    let facts = harvest_facts(&demuxer, &tracks);
    let duration_us = facts.duration_us().unwrap_or(i64::MAX / 4);
    // 2. Clamp the start position (doc 05 §2.4: clamping is truth).
    let start_us = opts.start_time_us.unwrap_or(0).clamp(0, duration_us);
    let mut skip_until_ns = None;
    if start_us > 0 {
        // The pure seek query validates the target against the sound
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

/// Harvest the track facts from a started AIFF demuxer: exactly one
/// audio track whose COMM is the whole decoder-config truth — and
/// since every codec in the demuxer's vocabulary is a `pcm_`
/// descriptor, that truth is the channels/rate pair (see the module
/// docs; the unsupported forms never reach here — `start` refuses
/// them typed).
fn harvest_facts(demuxer: &AiffDemuxer, tracks: &[Track]) -> MediaFacts {
    let comm = demuxer.comm();
    let mut facts = Vec::with_capacity(tracks.len());
    for track in tracks {
        // The pair the provider's `with_format` seam consumes; the
        // codec descriptor is the demuxer's own track vocabulary
        // (`pcm_s16be`, `pcm_s16le`, `pcm_u8`, `pcm_f32be`, …).
        let pcm = comm.map(|comm| (u32::from(comm.channels), comm.sample_rate));
        let span_end_ticks = track.timing.span.end.timestamp().ok().map(|ts| ts.ticks);
        facts.push(TrackFact {
            id: track.id,
            kind: track.kind,
            codec: track.codec.as_str().to_string(),
            base: track.timing.base,
            span_end_ticks,
            pcm,
            extra_data: None,
            frame_ns: None,
            dimensions: None,
        });
    }
    MediaFacts {
        tracks: tracks.to_vec(),
        facts,
        container: "aiff".to_string(),
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
            if let Frame::Audio(audio) = frame
                && let Some(frames) = audio.format.frames_in(audio.samples.len())
            {
                self.0 += frames;
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
    /// decoder provider (the pcm-pair seeding path) and return the
    /// emitted sample-frame count.
    fn decode_frames(opened: &mut crate::demux_host::OpenedMedia) -> Result<u64, EngineError> {
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

    /// The shared big-endian PCM fixture opens end-to-end with the
    /// recorded facts: one audio track, the `pcm_s16be` descriptor,
    /// the channels/rate pair, no extra_data (PCM's truth is the
    /// pair), and the declared 8 820-frame extent (one tick = one
    /// sample frame at 44.1 kHz = 200 ms).
    #[test]
    fn pcm_be_fixture_opens_with_expected_facts() {
        let opened = open(fixture("generated/valid/aiff_pcm_s16be_44k1_st.aiff"), None);
        assert_eq!(opened.facts.container, "aiff");
        assert_eq!(opened.facts.tracks.len(), 1);
        let fact = &opened.facts.facts[0];
        assert_eq!(fact.kind, MediaKind::Audio);
        assert_eq!(fact.codec, "pcm_s16be");
        assert_eq!(fact.pcm, Some((2, 44_100)));
        assert!(fact.extra_data.is_none(), "PCM carries no blob");
        let span_end = opened.facts.tracks[0].timing.span.end.timestamp();
        assert!(
            span_end.is_ok_and(|ts| ts.ticks == 8_820),
            "declared extent: {span_end:?}"
        );
        assert_eq!(opened.start_position_ns, 0);
    }

    /// Every AIFF/AIFC flavor fixture in the tree opens AND decodes
    /// through the real open path + decoder provider — the emitted
    /// sample frames equal the ffmpeg-measured truth (the same
    /// TRUTHS as crates/demux/tests/wav_aiff_fixtures.rs): classic
    /// big-endian s16/s24, AIFC `sowt` little-endian, AIFC `raw `
    /// unsigned 8-bit. No corpus lanes exist for this family (Lead
    /// census: zero .aiff in corpus-v4) — these fixtures ARE the
    /// differential.
    #[test]
    fn all_aiff_fixture_flavors_open_and_decode() {
        struct AiffTruth {
            file: &'static str,
            codec: &'static str,
            channels: u32,
            rate: u32,
            /// ffmpeg's decoded sample-frame count.
            truth_frames: u64,
        }
        let truths = [
            AiffTruth {
                file: "aiff_pcm_s16be_44k1_st.aiff",
                codec: "pcm_s16be",
                channels: 2,
                rate: 44_100,
                truth_frames: 8_820,
            },
            AiffTruth {
                file: "aiff_pcm_s24be_48k_st.aiff",
                codec: "pcm_s24be",
                channels: 2,
                rate: 48_000,
                truth_frames: 9_600,
            },
            AiffTruth {
                file: "aifc_sowt_s16le_44k1_st.aifc",
                codec: "pcm_s16le",
                channels: 2,
                rate: 44_100,
                truth_frames: 8_820,
            },
            AiffTruth {
                file: "aifc_raw_u8_44k1_mono.aifc",
                codec: "pcm_u8",
                channels: 1,
                rate: 44_100,
                truth_frames: 8_820,
            },
        ];
        let mut table = Vec::new();
        for truth in &truths {
            let path = fixture(&format!("generated/valid/{}", truth.file));
            assert!(path.is_file(), "fixture missing: {path:?}");
            let mut opened = open(path, None);
            assert_eq!(opened.facts.container, "aiff", "{}", truth.file);
            assert_eq!(opened.facts.tracks.len(), 1, "{}", truth.file);
            let fact = &opened.facts.facts[0];
            assert_eq!(fact.codec, truth.codec, "{}", truth.file);
            assert_eq!(fact.pcm, Some((truth.channels, truth.rate)), "{}", truth.file);
            assert!(fact.extra_data.is_none(), "{}: PCM carries no blob", truth.file);
            let frames = decode_frames(&mut opened)
                .unwrap_or_else(|err| unreachable!("{}: {err}", truth.file));
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
        eprintln!("aiff fixture differential:\n  {}", table.join("\n  "));
    }

    /// AIFC compressed forms outside the demuxer's PCM vocabulary
    /// (here `ima4`, the classic AIFF-C codec) refuse honestly at
    /// the demux stage — the probe still names `aiff` (FORM/AIFC
    /// magic intact), the container host's `start()` pre-pass is the
    /// typed refusal, mirroring the demuxer's own
    /// `unsupported-compression` behavior. Classic AIFF's signed
    /// 8-bit width refuses the same way (no registry descriptor).
    #[test]
    fn unsupported_aifc_compression_refuses_typed() {
        // Hand-built AIFC: COMM(18) + `ima4` + empty pstring, SSND
        // with 4 whole frames of sound — structurally valid except
        // the compression type.
        let mut comm = Vec::new();
        comm.extend_from_slice(&2u16.to_be_bytes()); // channels
        comm.extend_from_slice(&4u32.to_be_bytes()); // numSampleFrames
        comm.extend_from_slice(&16u16.to_be_bytes()); // sampleSize
        comm.extend_from_slice(
            &match demux::aiff::test_support::extended80(44_100.0) {
                Ok(ext) => ext,
                Err(err) => unreachable!("rate: {err}"),
            },
        );
        comm.extend_from_slice(b"ima4"); // compressionType
        comm.push(0); // pstring: zero-length name
        let mut aifc = Vec::new();
        aifc.extend_from_slice(b"FORM");
        aifc.extend_from_slice(&68u32.to_be_bytes());
        aifc.extend_from_slice(b"AIFC");
        aifc.extend_from_slice(b"COMM");
        aifc.extend_from_slice(&u32::try_from(comm.len()).must().to_be_bytes());
        aifc.extend_from_slice(&comm);
        aifc.push(0); // odd chunk → pad to even
        aifc.extend_from_slice(b"SSND");
        aifc.extend_from_slice(&24u32.to_be_bytes()); // 8 + 16 sound bytes
        aifc.extend_from_slice(&0u32.to_be_bytes()); // offset
        aifc.extend_from_slice(&0u32.to_be_bytes()); // blockSize
        aifc.extend_from_slice(&[0u8; 16]);
        let path = std::env::temp_dir().join("lumen-cli-aifc-ima4.aifc");
        std::fs::write(&path, &aifc).must();
        let err = match open_source(&SourceSpec::File { path: path.clone() }, None, false, false) {
            Ok(_) => unreachable!("ima4 must refuse"),
            Err(err) => err,
        };
        std::fs::remove_file(&path).ok();
        assert_eq!(err.stage, commands::Stage::Demux, "{err}");
        assert_eq!(
            err.details.get("code").map(String::as_str),
            Some("demux.aiff.unsupported-compression"),
            "{err}"
        );

        // Classic AIFF, 8-bit signed width: the same typed refusal
        // (built through the demuxer's own test-support builder).
        let bytes = demux::aiff::test_support::minimal_aiff(1, 44_100, 8, &[0x12, 0x34]);
        let path = std::env::temp_dir().join("lumen-cli-aiff-s8.aiff");
        std::fs::write(&path, &*bytes).must();
        let err = match open_source(&SourceSpec::File { path: path.clone() }, None, false, false) {
            Ok(_) => unreachable!("classic 8-bit must refuse"),
            Err(err) => err,
        };
        std::fs::remove_file(&path).ok();
        assert_eq!(err.stage, commands::Stage::Demux, "{err}");
        assert_eq!(
            err.details.get("code").map(String::as_str),
            Some("demux.aiff.unsupported-compression"),
            "{err}"
        );
    }

    /// A magic-intact AIFF cut mid-COMM (the committed malformed
    /// derivative `trunc1`) refuses honestly at the demux stage with
    /// the demuxer's own typed truncation code.
    #[test]
    fn truncated_aiff_refuses_at_the_demux_stage() {
        let path = fixture("generated/malformed/aiff_pcm_s16be_44k1_st__trunc1");
        assert!(path.is_file(), "malformed fixture missing: {path:?}");
        let err = match open_source(&SourceSpec::File { path }, None, false, false) {
            Ok(_) => unreachable!("a truncated COMM must refuse"),
            Err(err) => err,
        };
        assert_eq!(err.stage, commands::Stage::Demux, "{err}");
        assert_eq!(
            err.details.get("code").map(String::as_str),
            Some("demux.aiff.truncated"),
            "{err}"
        );
    }

    /// `--start-time` clamps to the declared duration and the seek
    /// query validates the target (the demuxer's Fast seek): a
    /// target past the end admits no packet; an in-media target
    /// lands every surviving packet at/after it.
    #[test]
    fn start_time_clamps_and_skips() {
        let path = fixture("generated/valid/aiff_pcm_s16be_44k1_st.aiff");
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
}
