//! The AVI container host — the registry's first wired family
//! (D-018's first vertical, moved verbatim from the demux-host gate
//! by 6.OPW-PREP; behavior identical): construct the demuxer, run the
//! host pre-pass `start()`, harvest the `strf` facts, resolve the
//! start-time through the pure seek query, wrap in the shared
//! [`SelectDemuxer`].
//!
//! The AVI-specific remainder is exactly this file's two functions:
//! the constructor call + `stream_format` facts harvest. Everything
//! else (skip wrapper, engine wiring, codec dispatch) is shared code.

use commands::CommandError;
use demux::avi::AviDemuxer;
use demux::avi::header::StreamFormat;
use media_types::time::{Stamp, TimeSpan, Timestamp};
use media_types::Track;
use plugin_api::{Demuxer, SourceStream};

use crate::demux_host::{
    MediaFacts, OpenedMedia, OpenOptions, SelectDemuxer, TrackFact, wire_error,
};

/// Open an AVI source (the doc §4.2 per-container shape): the probe
/// already named `avi`; this constructs the demuxer, runs the host
/// pre-pass `start()` (the CLI is the composition root — it reads the
/// track list and the `strf` stream formats BEFORE the demuxer moves
/// into the engine; the engine's own `start()` then returns the
/// cached tracks), clamps `--start-time` against the declared
/// duration, validates it through the demuxer's pure `seek` query and
/// wraps the demuxer in the skip/selection wrapper.
///
/// # Errors
/// Typed [`CommandError`]s (stage `demux`): header parse failures,
/// seek-range violations.
pub(crate) fn open(
    stream: Box<dyn SourceStream>,
    opts: &OpenOptions,
) -> Result<OpenedMedia, CommandError> {
    // 1. Demuxer + the host pre-pass (track list + strf facts).
    let mut demuxer = AviDemuxer::new(stream).map_err(|err| wire_error(&err))?;
    let tracks = demuxer.start().map_err(|err| wire_error(&err))?;
    let facts = harvest_facts(&demuxer, &tracks);
    let duration_us = facts.duration_us().unwrap_or(i64::MAX / 4);
    // 2. Clamp the start position (doc 05 §2.4: clamping is truth).
    let start_us = opts.start_time_us.unwrap_or(0).clamp(0, duration_us);
    let mut skip_until_ns = None;
    if start_us > 0 {
        // The pure seek query validates the target against the seek
        // table (and the no-index scan) before the session exists.
        // The target is nanosecond-based (the engine clock's base).
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

/// Harvest the per-track facts from a started AVI demuxer: the `strf`
/// stream formats are the decoder-config seam (MJPEG dimensions, PCM
/// channels/rate through the demuxer's own descriptor mapping — not a
/// local copy of the tag/bits table).
fn harvest_facts(demuxer: &AviDemuxer, tracks: &[Track]) -> MediaFacts {
    let mut facts = Vec::with_capacity(tracks.len());
    for track in tracks {
        let (codec, pcm, frame_ns, dimensions) = match demuxer.stream_format(track.id) {
            Some(StreamFormat::MotionJpeg { width, height }) => {
                // One tick of the video base is one frame (AVI scale=1).
                let frame_ns = (u64::from(track.timing.base.num)).saturating_mul(1_000_000_000)
                    / u64::from(track.timing.base.den);
                (
                    "mjpeg".to_string(),
                    None,
                    Some(frame_ns),
                    Some((width, height)),
                )
            }
            Some(
                stream_format @ StreamFormat::Pcm {
                    channels,
                    samples_per_sec,
                    ..
                },
            ) => {
                // The registry descriptor the strf declares — the
                // demuxer's own single mapping (header.rs), not a
                // local copy of the tag/bits table.
                let codec = stream_format.codec_descriptor().to_string();
                (
                    codec,
                    Some((u32::from(channels), samples_per_sec)),
                    None,
                    None,
                )
            }
            None => ("unknown".to_string(), None, None, None),
        };
        let span_end_ticks = track.timing.span.end.timestamp().ok().map(|ts| ts.ticks);
        facts.push(TrackFact {
            id: track.id,
            kind: track.kind,
            codec,
            base: track.timing.base,
            span_end_ticks,
            pcm,
            // AVI carries no out-of-band codec blob: its PCM truth is
            // the `strf` pair above and MJPEG is self-describing per
            // frame (the esds ASC / ALAC cookie / Vorbis header chain
            // / WAVEFORMATEX bodies are the other containers'
            // harvests).
            extra_data: None,
            frame_ns,
            dimensions,
        });
    }
    MediaFacts {
        tracks: tracks.to_vec(),
        facts,
        container: "avi".to_string(),
    }
}
