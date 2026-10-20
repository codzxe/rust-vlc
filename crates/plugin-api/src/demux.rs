//! The demuxer contract (doc 06 §1.2 kind 3): container parse →
//! timestamped packets + seek tables. Impl crate: `demux` (L2).
//!
//! Instance scope: one per playback session (§1.3); teardown rides the
//! pipeline generation. The demuxer pulls from an
//! [`Access`](crate::access::Access) stream the engine opens and hands
//! it, and pushes demuxed packets into the engine's
//! [`PacketSink`]. It owns no queues, no pacing, no track selection —
//! the engine decides what to decode (doc 01 §4).

use media_types::ids::TrackId;
use media_types::time::TimeSpan;
use media_types::{Duration, EngineError, Packet, Track};

/// A demuxer (doc 06 §1.2 row 3).
///
/// # Calling context
///
/// Runs on the demux worker thread (doc 01 §4.1): [`Send`], bounded
/// work per call — **one container chunk per
/// [`next_event`](Demuxer::next_event)**, never an internal
/// read-the-whole-file loop. Cancellation is generation staleness:
/// the engine bumps the session [`GenerationId`], drops the demuxer,
/// and constructs a fresh one for the new position; packets of the old
/// generation are dropped at the earliest checkpoint (MP §8).
///
/// # Dataflow
///
/// - [`start`](Demuxer::start) runs once per instance: container
///   header parse + track enumeration. All tracks it reports ride the
///   engine's track registry (`Track` values from L0).
/// - [`next_event`](Demuxer::next_event) yields one
///   [`DemuxEvent`] at a time until
///   [`EndOfStream`](DemuxEvent::EndOfStream). The demuxer reads
///   through its access stream as needed (bounded per call).
/// - [`seek`](Demuxer::seek) asks the container for its nearest
///   keyframe-safe position ≤ the target; the engine owns the
///   generation bump and the decoder flush around it.
///
/// # Errors
///
/// Failures are [`EngineError`] values at stage
/// [`PipelineStage::Demux`]: corrupt containers mint
/// `ErrorKind::Corrupt { detail }`, truncated files
/// `Corrupt { detail: Truncated }`, unknown containers
/// `ErrorKind::Unsupported`. Panics are contract violations.
pub trait Demuxer: Send {
    /// Parse the container header and enumerate tracks.
    ///
    /// # Errors
    /// [`EngineError`] (stage [`PipelineStage::Demux`]) when the
    /// container is unrecognized or the header is corrupt.
    fn start(&mut self) -> Result<Vec<Track>, EngineError>;

    /// Produce the next demux event (one packet, or a container-level
    /// signal).
    ///
    /// # Errors
    /// [`EngineError`] when the container fails unrecoverably;
    /// recoverable damage is reported as
    /// [`DemuxEvent::Discontinuity`] and counted by the engine
    /// (MP §8 bounded recovery).
    fn next_event(&mut self) -> Result<DemuxEvent, EngineError>;

    /// Map a wall-time target onto the container's keyframe-safe
    /// position ≤ it, and report where the stream will resume.
    ///
    /// Does **not** move the read position: the engine decides
    /// whether to act (it owns the generation bump and the decoder
    /// flush, doc 03 §5).
    ///
    /// # Errors
    /// [`EngineError`] when the container carries no seek table or the
    /// target is out of range.
    fn seek(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError>;
}

/// One demux outcome (doc 01 §3.3 packet + container signals).
#[derive(Debug, Clone, PartialEq)]
pub enum DemuxEvent {
    /// A demuxed packet, generation-tagged, headed for decode.
    Packet(Packet),
    /// The demuxer overcame container damage (resynchronized past it);
    /// the engine counts it against the demux recovery budget (MP §8).
    Recovered {
        /// Bytes or blocks skipped by the resynchronization.
        skipped: u64,
        /// Why (for observability).
        reason: &'static str,
    },
    /// A marked discontinuity (splice, chapter boundary) — timestamps
    /// after it may jump; arithmetic across it without a clock reset
    /// is a `TimeError::Discontinuity` (doc 01 §3.2).
    Discontinuity,
    /// Nothing more will come out of this container (the flush
    /// endpoint).
    EndOfStream,
}

/// Where demuxed packets go: the engine's queue, created and consumed
/// on the demux worker thread (no `Send` bound — the codec-api
/// `FrameSink` precedent).
pub trait PacketSink {
    /// Take one demuxed packet.
    ///
    /// Returning `Err` aborts the in-flight
    /// [`next_event`](Demuxer::next_event) call: the demuxer
    /// propagates or chains the error and remains usable.
    ///
    /// # Errors
    /// The sink's own typed error (queue full, engine stopping),
    /// expressed as an [`EngineError`].
    fn accept(&mut self, packet: Packet) -> Result<(), EngineError>;
}

/// Typed demux error: duration unknown is data, not error — but a
/// track whose timing the container declares and the demuxer cannot
/// honor is an error this helper mints.
pub fn track_timing_conflict(track: &TrackId) -> EngineError {
    EngineError::new(
        media_types::PipelineStage::Demux,
        media_types::ErrorKind::Corrupt {
            detail: media_types::error::CorruptDetail::Timing,
        },
        media_types::error::ErrorContext::new("plugin.demux.track-timing-conflict"),
    )
    .with_source(media_types::time::TimeError::Missing)
    .with_source(DemuxSourceInfo(*track))
}

/// Tiny source-chaining helper (the engine's error taxonomy keeps
/// `source` for the chain root; this gives the demux error a
/// track-bearing leaf).
#[derive(Debug)]
struct DemuxSourceInfo(TrackId);

impl std::fmt::Display for DemuxSourceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "demux track #{}", self.0.handle())
    }
}
impl std::error::Error for DemuxSourceInfo {}

/// Duration of a container, as the demuxer reports it to the engine
/// (a value and its absence are different types, MP §8).
pub fn container_duration(span: TimeSpan) -> Option<Duration> {
    match (span.start.timestamp(), span.end.timestamp()) {
        (Ok(a), Ok(b)) => {
            // End − start in the end stamp's base, expressed in
            // nanoseconds via the L0 checked conversion.
            let converted = a.convert(&b.base).ok()?;
            let ticks = b.ticks.checked_sub(converted.ticks)?;
            let delta = media_types::time::Timestamp::new(ticks, b.base);
            match delta.as_nanos() {
                Ok(ns) => Some(Duration::Known(ns)),
                Err(_) => None,
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::ids::{GenerationId, StreamId, TrackFlags};
    use media_types::packet::SharedBytes;
    use media_types::time::{Stamp, TimeBase, Timestamp, TrackTiming};

    /// One in-memory packet list standing in for a demuxed container.
    struct FakeDemux {
        packets: Vec<Packet>,
        emitted: usize,
        tracks: Vec<Track>,
    }

    impl Demuxer for FakeDemux {
        fn start(&mut self) -> Result<Vec<Track>, EngineError> {
            Ok(self.tracks.clone())
        }
        fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
            match self.packets.get(self.emitted) {
                Some(p) => {
                    self.emitted += 1;
                    Ok(DemuxEvent::Packet(p.clone()))
                }
                None => Ok(DemuxEvent::EndOfStream),
            }
        }
        fn seek(&mut self, _target: TimeSpan) -> Result<TimeSpan, EngineError> {
            Ok(TimeSpan::UNKNOWN)
        }
    }

    fn packet(generation: GenerationId, ticks: i64) -> Packet {
        let ts = match Timestamp::from_parts(ticks, 1, 1000) {
            Ok(ts) => ts,
            Err(e) => unreachable!("{e}"),
        };
        let dur = match Timestamp::from_parts(40, 1, 1000) {
            Ok(ts) => ts,
            Err(e) => unreachable!("{e}"),
        };
        Packet {
            track: TrackId::new(0),
            pts: Stamp::Present(ts),
            dts: Stamp::Present(ts),
            duration: Some(Duration::Known(media_types::Nanoseconds(
                dur.as_nanos().map(|n| n.0).unwrap_or(0),
            ))),
            data: SharedBytes::from_static(&[0x1a, 0x45, 0xdf, 0xa3]),
            keyframe: true,
            discontinuity: false,
            generation,
        }
    }

    #[test]
    fn events_flow_packets_then_eos() {
        let base = TimeBase::new(1, 1000).unwrap_or_else(|e| unreachable!("{e}"));
        let timing = TrackTiming {
            span: TimeSpan::UNKNOWN,
            base,
            gaps: Vec::new(),
        };
        let tracks = vec![Track {
            id: TrackId::new(0),
            stream: StreamId::new(0),
            kind: media_types::MediaKind::Video,
            language: None,
            codec: match media_types::ids::CodecDescriptor::new("av1") {
                Ok(c) => c,
                Err(e) => unreachable!("{e}"),
            },
            title: None,
            flags: TrackFlags::NONE,
            timing: timing.clone(),
        }];
        let mut d = FakeDemux {
            packets: vec![
                packet(GenerationId::FIRST, 0),
                packet(GenerationId::FIRST, 40),
            ],
            emitted: 0,
            tracks,
        };
        let tracks = match d.start() {
            Ok(t) => t,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(tracks.len(), 1);
        let e1 = match d.next_event() {
            Ok(e) => e,
            Err(e) => unreachable!("{e}"),
        };
        assert!(matches!(e1, DemuxEvent::Packet(_)));
        let e2 = match d.next_event() {
            Ok(e) => e,
            Err(e) => unreachable!("{e}"),
        };
        assert!(matches!(e2, DemuxEvent::Packet(_)));
        let e3 = match d.next_event() {
            Ok(e) => e,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(e3, DemuxEvent::EndOfStream);
        // Idempotent EOS.
        let e4 = match d.next_event() {
            Ok(e) => e,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(e4, DemuxEvent::EndOfStream);
    }

    #[test]
    fn typed_error_shapes() {
        let err = track_timing_conflict(&TrackId::new(7));
        assert_eq!(err.stage, media_types::PipelineStage::Demux);
        assert!(matches!(err.kind, media_types::ErrorKind::Corrupt { .. }));
        // Display renders structured data only; the detail is on the
        // source chain (L0 no-secrets rule).
        assert!(std::error::Error::source(&err).is_some());
        assert_eq!(err.context.code, "plugin.demux.track-timing-conflict");
        // container_duration: known span ⇒ Some, unknown ⇒ None.
        let a = match Timestamp::from_parts(0, 1, 1000) {
            Ok(ts) => ts,
            Err(e) => unreachable!("{e}"),
        };
        let b = match Timestamp::from_parts(90_000, 1, 1000) {
            Ok(ts) => ts,
            Err(e) => unreachable!("{e}"),
        };
        let span = TimeSpan {
            start: Stamp::Present(a),
            end: Stamp::Present(b),
        };
        assert!(container_duration(span).is_some());
        assert!(container_duration(TimeSpan::UNKNOWN).is_none());
    }
}
