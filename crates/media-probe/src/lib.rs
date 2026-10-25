//! # media-probe — bounded format detection (card 2.A2; doc 03 §4.2)
//!
//! Identifies the container format from a **bounded lookahead window**
//! with confidence scores, and hands the engine the
//! [`ProbeReport`] it uses to select a demuxer (doc 01 §2 step 3).
//! Never probes unbounded data (MP §12): the window is
//! [`ProbeConfig`]-bounded and hard-ceiling-capped, and there is no
//! code path that reads past it.
//!
//! # Module map
//!
//! - [`config`] — [`ProbeConfig`] and the doc 03 §4.2 constants
//!   (64 KiB default window, 1 MiB hard cap, minimum score 60) with
//!   typed validation.
//! - [`report`] — the outcome vocabulary: [`FormatId`],
//!   [`Confidence`], [`TrackHint`], [`ProbeHit`], [`ProbeReport`],
//!   [`SourceMeta`].
//! - [`signature`] — the [`ProbePlugin`](signature::ProbePlugin) seam
//!   (doc 03 §4.2's trait) and the
//!   [`ProbeRegistry`](signature::ProbeRegistry) that scores
//!   signatures and applies the selection rules; the builtin AVI and
//!   MP4 signatures with their documented scoring ladders, plus the
//!   Phase-5 R3 consolidation rows (card 5.R3-REG): WAV, AIFF, Ogg,
//!   Matroska/WebM, ADTS/ADIF, MPEG-TS, MPEG-PS and IVF — every
//!   landed container, magic-anchored, mirror-pinned to its
//!   demuxer's own probe gate by the round-trip suite.
//! - [`contract`] — the instrumented [`CountingSource`] and the
//!   executable probe-side contract (depth bound, position restore)
//!   for this crate's tests, the engine wiring tests and the doc 06
//!   §7 conformance kit. **Test/conformance-only** (audit F-2):
//!   compiled under `#[cfg(any(test, feature = "contract"))]` —
//!   release binaries carry none of its panicky conformance
//!   vocabulary.
//! - [`error`] — typed error constructors at stage
//!   [`PipelineStage::Probe`](media_types::PipelineStage::Probe); the
//!   taxonomy is media-types' and is never duplicated here.
//!
//! # Design decisions (recorded for review)
//!
//! - **Scoring is a pure function of `(prefix, meta)`** (doc 03
//!   §4.2(e)): [`ProbeRegistry::probe`] never touches a stream. The
//!   stream-facing [`probe_stream`] is a thin orchestration — fill the
//!   window, score it, restore the position.
//! - **The probe never consumes stream bytes** (doc 03 §4.2(e)): the
//!   window is read from the stream's *current* position and the
//!   position is restored afterwards (for the `input` crate's
//!   buffered `FileStream` the bytes stay in the fixed read-ahead
//!   window, so the selected demuxer receives the prefix as
//!   already-read data — the composition the 2.A1+2.A2 integration
//!   suite proves). v0 requires seekable sources for this guarantee;
//!   live-source probing is the `streaming` crate's Phase-6
//!   peek-buffer design, not a silent half-consumption here.
//! - **One-shot probing in v1** (doc 03 §4.2(e) `[PROPOSAL]`): a
//!   format that only becomes identifiable deep in the stream is
//!   reported unsupported — there is no re-probe loop. The window is
//!   the *leading* `max_lookahead` bytes, by construction.
//! - **The extension is a hint, not truth** (doc 03 §4.2(f)): content
//!   wins; a valid AVI named `.mp4` probes as AVI (tested), and the
//!   extension surfaces only in the no-hit error payload.
//! - **Hints are sniffed, never assumed**: a track hint exists only
//!   when the signature's anchored scan actually saw the bytes (an
//!   AVI `strl` pair, an MP4 `hdlr`+`stsd` pair). Damage inside the
//!   window degrades hints honestly — the class-B flip fixtures pin
//!   this; the demuxer's counted-damage recovery is its own, later
//!   stage.
//! - **The codec names hints carry are the demuxers' names**
//!   (`mjpeg`, `pcm_s16le` — the L0 descriptor strings the
//!   expectation TOMLs record); unknown fourccs stay unhinted rather
//!   than invented (rows are added when their decoder cards land).
//!
//! # House rules
//!
//! Std-only, zero external dependencies (C5); no `unsafe`; no
//! `unwrap`/`expect`/`panic!`/`todo!`/`dbg!` in non-test code
//! (`unreachable!` only on provably-dead literal-contract arms, the
//! testkit `must` precedent — confined to the feature-gated
//! `contract` module, which never compiles into release builds);
//! bounded memory: the window is the only allocation and it is
//! ceiling-capped.
//!
//! # Example
//!
//! Probe an in-memory window (the pure path), then a stream:
//!
//! ```
//! use media_probe::{ProbeConfig, ProbeRegistry, SourceMeta, probe_stream};
//! use media_types::ids::SourceUri;
//! use media_types::EngineError;
//! use plugin_api::{SeekOrigin, SourcePosition, SourceStream};
//!
//! # fn main() -> Result<(), EngineError> {
//! // A minimal RIFF/AVI header window.
//! let mut avi = b"RIFF\x24\x7e\x06\x00AVI ".to_vec();
//! avi.extend_from_slice(b"LIST\xec\x00\x00\x00hdrlavih\x38\x00\x00\x00");
//!
//! // Pure scoring: registry + window + meta + config.
//! let registry = ProbeRegistry::with_builtins();
//! let meta = SourceMeta::new(Some(avi.len() as u64), Some("avi"));
//! let config = ProbeConfig::default();
//! let report = registry.probe(&avi, &meta, &config)?;
//! assert_eq!(report.format.as_str(), "avi");
//! assert_eq!(report.confidence.as_u8(), 90); // magic + hdrl + avih
//!
//! // Stream probing: the window is filled (bounded), scored, and the
//! // position restored — the source serves the same bytes again. The
//! // source here is a minimal in-memory `SourceStream` (a doc stand-in
//! // for a real access stream; the crate's own instrumented sources
//! // live in the test-only `contract` module).
//! struct Window {
//!     data: Vec<u8>,
//!     pos: usize,
//!     uri: SourceUri,
//! }
//! impl SourceStream for Window {
//!     fn uri(&self) -> &SourceUri {
//!         &self.uri
//!     }
//!     fn size(&self) -> Option<u64> {
//!         Some(self.data.len() as u64)
//!     }
//!     fn supports_seek(&self) -> bool {
//!         true
//!     }
//!     fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
//!         let take = len.min(self.data.len() - self.pos);
//!         let out = self.data[self.pos..self.pos + take].to_vec();
//!         self.pos += take;
//!         Ok(out)
//!     }
//!     fn seek(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError> {
//!         let to = match origin {
//!             SeekOrigin::Start(to) => to.min(self.data.len() as u64) as usize,
//!             SeekOrigin::Current(delta) => {
//!                 (self.pos as i64 + delta).clamp(0, self.data.len() as i64) as usize
//!             }
//!         };
//!         self.pos = to;
//!         Ok(SourcePosition(to as u64))
//!     }
//!     fn position(&self) -> SourcePosition {
//!         SourcePosition(self.pos as u64)
//!     }
//! }
//! let uri = SourceUri::parse("file:///probe-example.avi")?;
//! let mut source = Window {
//!     data: avi.clone(),
//!     pos: 0,
//!     uri,
//! };
//! let report = probe_stream(&registry, &mut source, &config)?;
//! assert_eq!(report.format.as_str(), "avi");
//! assert_eq!(source.position().0, 0, "position restored");
//! let replay = source.read(4)?;
//! assert_eq!(&replay, b"RIFF", "the prefix was not consumed");
//! # Ok(())
//! # }
//! ```

pub mod config;
#[cfg(any(test, feature = "contract"))]
pub mod contract;
pub mod error;
pub mod report;
pub mod signature;

pub use config::{DEFAULT_MAX_LOOKAHEAD, DEFAULT_MIN_SCORE, MAX_LOOKAHEAD_CEILING, ProbeConfig};
pub use report::{Confidence, FormatId, ProbeHit, ProbeReport, SourceMeta, TrackHint};
pub use signature::{ProbePlugin, ProbeRegistry};

use media_types::EngineError;
use plugin_api::{SeekOrigin, SourceStream};

/// Probe an opened stream: fill the bounded window from the stream's
/// current position, score it against the registry, and restore the
/// position (doc 03 §4.2 — the open-path step between access and
/// demux).
///
/// The window is the next `config.max_lookahead` bytes (the leading
/// bytes when called at the open path's position 0 — the doc's
/// one-shot probing; there is no deeper second pass). Reads stop at
/// end of stream; a source serving more than asked is defensively
/// clamped so the bound holds regardless.
///
/// # Errors
/// - [`EngineError`] (stage `Probe`, `Unsupported`) when the stream
///   cannot seek — v0's non-consumption guarantee requires the
///   restore (see the crate docs; live sources are the `streaming`
///   crate's Phase-6 concern).
/// - Whatever the stream's `read`/`seek` surface (stage `Open` per the
///   plugin-api contract) — propagated unchanged.
/// - [`EngineError`] (stage `Probe`, `Unsupported`) when no signature
///   reaches `config.min_score` (doc 03 §4.2(d)).
pub fn probe_stream(
    registry: &ProbeRegistry,
    stream: &mut dyn SourceStream,
    config: &ProbeConfig,
) -> Result<ProbeReport, EngineError> {
    if !stream.supports_seek() {
        return Err(error::stream_not_rewindable(stream.uri().scheme().as_str()));
    }
    let meta = SourceMeta::from_stream(stream);
    let start = stream.position();
    let window = fill_window(stream, config.max_lookahead)?;
    // Doc 03 §4.2(e): the probe never consumes stream bytes. Restore
    // the pre-probe position (for input's buffered streams this is a
    // read-ahead hit, not fresh I/O).
    if stream.position() != start {
        stream.seek(SeekOrigin::Start(start.0))?;
    }
    registry.probe(&window, &meta, config)
}

/// Read at most `max` bytes into the probe window, stopping at end of
/// stream. The single bounded-read path: every request asks for
/// exactly the remaining budget, never more.
fn fill_window(stream: &mut dyn SourceStream, max: usize) -> Result<Vec<u8>, EngineError> {
    // Allocation hint: the known size when it is smaller than the
    // bound; saturating conversion — a hint only, never a limit.
    let hint = match stream.size() {
        Some(size) => usize::try_from(size).unwrap_or(max).min(max),
        None => max,
    };
    let mut window = Vec::with_capacity(hint);
    while window.len() < max {
        let want = max - window.len();
        let chunk = stream.read(want)?;
        if chunk.is_empty() {
            break; // end of stream — one shot, no retry
        }
        // Defensive clamp: the plugin-api contract says "up to len";
        // a violating source cannot grow the window past the bound.
        let take = chunk.len().min(want);
        window.extend_from_slice(&chunk[..take]);
    }
    Ok(window)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{CountingSource, assert_position_restored, assert_probe_depth_bounded};

    /// The doctest's inline in-memory source, against the crate's own
    /// instrumented source: both must probe identically (audit F-2's
    /// doctest rewrite keeps the example honest — this pin keeps the
    /// crate honest about it).
    #[test]
    fn doctest_shape_probes_like_the_instrumented_source() {
        let mut data = b"RIFF\x24\x7e\x06\x00AVI ".to_vec();
        data.extend_from_slice(b"LIST\xec\x00\x00\x00hdrlavih\x38\x00\x00\x00");
        let registry = ProbeRegistry::with_builtins();
        let mut source = CountingSource::new(&data);
        let config = match ProbeConfig::new(4096, 60) {
            Ok(config) => config,
            Err(e) => unreachable!("{e}"),
        };
        let report = match probe_stream(&registry, &mut source, &config) {
            Ok(report) => report,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(report.format.as_str(), "avi");
        assert_eq!(report.confidence.as_u8(), 90);
        assert_eq!(source.position().0, 0, "position restored");
        assert_position_restored(&source, plugin_api::SourcePosition(0));
        assert_probe_depth_bounded(&source, config.max_lookahead);
    }

    fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => unreachable!("{e}"),
        }
    }

    /// A synthetic full AVI header (magic + hdrl + avih; no tracks).
    fn avi_header() -> Vec<u8> {
        let mut out = b"RIFF\x24\x7e\x06\x00AVI ".to_vec();
        out.extend_from_slice(b"LIST\xec\x00\x00\x00hdrlavih\x38\x00\x00\x00");
        out
    }

    #[test]
    fn probe_stream_scores_and_restores() {
        let data = avi_header();
        let mut source = CountingSource::new(&data);
        let registry = ProbeRegistry::with_builtins();
        let config = ok(ProbeConfig::new(4096, 60));
        let report = ok(probe_stream(&registry, &mut source, &config));
        assert_eq!(report.format.as_str(), "avi");
        assert_eq!(report.confidence.as_u8(), 90);
        assert_eq!(report.bytes_examined, data.len());
        assert_position_restored(&source, plugin_api::SourcePosition(0));
        assert_probe_depth_bounded(&source, config.max_lookahead);
        // The stronger non-consumption proof: the same bytes again.
        let replay = ok(source.read(4));
        assert_eq!(replay, b"RIFF".to_vec());
    }

    #[test]
    fn window_bound_is_exact() {
        // A 36-byte header with a 36-byte window: everything read.
        let data = avi_header();
        let mut source = CountingSource::new(&data);
        let registry = ProbeRegistry::with_builtins();
        let config = ok(ProbeConfig::new(data.len(), 60));
        let report = ok(probe_stream(&registry, &mut source, &config));
        assert_eq!(report.bytes_examined, data.len());
        // One byte short of the avih chunk header: the rung drops.
        let mut source = CountingSource::new(&data);
        let config = ok(ProbeConfig::new(data.len() - 1, 60));
        let report = ok(probe_stream(&registry, &mut source, &config));
        assert_eq!(report.bytes_examined, data.len() - 1);
        assert_eq!(report.confidence.as_u8(), 80);
        // The magic boundary: 12 bytes in, still AVI; 11, unknown.
        let mut source = CountingSource::new(&data);
        let config = ok(ProbeConfig::new(12, 60));
        let report = ok(probe_stream(&registry, &mut source, &config));
        assert_eq!(report.confidence.as_u8(), 70);
        let mut source = CountingSource::new(&data);
        let config = ok(ProbeConfig::new(11, 60));
        let err = match probe_stream(&registry, &mut source, &config) {
            Ok(_) => unreachable!("11 bytes cannot carry the AVI form"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "probe.no-demuxer");
    }

    #[test]
    fn depth_bound_holds_for_every_window_size() {
        // The card's AC at unit level: for any configured bound, the
        // source serves at most that many bytes, one request at a time
        // within the remaining budget. Windows too small to carry a
        // magic are an honest typed unknown, not a guess — the same
        // run of the bound and restore assertions applies to them.
        let mut data = vec![0x11u8; 300_000];
        data[..12].copy_from_slice(b"RIFF....AVI ");
        let registry = ProbeRegistry::with_builtins();
        for bound in [1usize, 11, 12, 64, 4096, 65_536, 262_144, 300_000] {
            let mut source = CountingSource::new(&data);
            let config = ok(ProbeConfig::new(bound, 60));
            match probe_stream(&registry, &mut source, &config) {
                Ok(report) => assert!(report.bytes_examined <= bound),
                Err(e) => {
                    // 1 and 11 bytes cannot carry the RIFF+form magic:
                    // no signature anchors, and the verdict is the
                    // typed Unsupported of doc 03 §4.2(d).
                    assert_eq!(e.context.code, "probe.no-demuxer");
                    assert_eq!(e.stage, media_types::PipelineStage::Probe);
                }
            }
            assert_probe_depth_bounded(&source, bound);
            assert_position_restored(&source, plugin_api::SourcePosition(0));
        }
        // EOS stop: a window larger than the data reads exactly the
        // data, no retry spin (one EOS read, then done).
        let mut source = CountingSource::new(&data[..50]);
        let config = ok(ProbeConfig::new(65_536, 60));
        let report = ok(probe_stream(&registry, &mut source, &config));
        assert_eq!(report.bytes_examined, 50);
        assert_eq!(source.requests().len(), 2, "fill + EOS probe only");
    }

    #[test]
    fn short_reads_are_robust() {
        // A source serving 7 bytes per read fills the same window.
        let data = avi_header();
        let mut source = CountingSource::new(&data);
        source.set_chunk_limit(7);
        let registry = ProbeRegistry::with_builtins();
        let config = ok(ProbeConfig::new(4096, 60));
        let report = ok(probe_stream(&registry, &mut source, &config));
        assert_eq!(report.confidence.as_u8(), 90);
        assert!(source.requests().iter().all(|&r| r <= 4096));
        assert_probe_depth_bounded(&source, 4096);
    }

    #[test]
    fn non_seekable_streams_are_a_typed_refusal() {
        let mut source = CountingSource::new(&avi_header());
        source.set_unseekable();
        let registry = ProbeRegistry::with_builtins();
        let err = match probe_stream(&registry, &mut source, &ok(ProbeConfig::new(4096, 60))) {
            Ok(_) => unreachable!("must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "probe.stream-not-rewindable");
        assert_eq!(err.stage, media_types::PipelineStage::Probe);
        // And nothing was read before the refusal.
        assert_eq!(source.total_served(), 0);
    }

    #[test]
    fn read_failures_propagate_from_the_stream() {
        let mut source = CountingSource::new(&avi_header());
        source.fail_next_read();
        let registry = ProbeRegistry::with_builtins();
        let err = match probe_stream(&registry, &mut source, &ok(ProbeConfig::new(4096, 60))) {
            Ok(_) => unreachable!("must fail"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "probe-test.scripted-read-failure");
        assert_eq!(err.stage, media_types::PipelineStage::Open);
    }

    #[test]
    fn garbage_windows_are_low_confidence_rejections() {
        let registry = ProbeRegistry::with_builtins();
        let config = ProbeConfig::default();
        let meta = SourceMeta::new(Some(4096), Some("bin"));
        for window in [
            &vec![0u8; 4096][..],
            &b"just some text, not a container"[..],
            &b"\x1a\x45\xdf\xa4\x93B\x86\x81\x01"[..], // EBML-ish, magic off by one
            &b"RIFX....WAVEfmt "[..],                  // RIFF family, wrong magic
            &b"\0\0\0\x18ft9p\x00\x00\x00\x00"[..],    // broken ftyp type
        ] {
            let err = match registry.probe(window, &meta, &config) {
                Ok(_) => unreachable!("garbage must not probe: {window:?}"),
                Err(e) => e,
            };
            assert_eq!(err.context.code, "probe.no-demuxer");
            assert_eq!(err.stage, media_types::PipelineStage::Probe);
        }
    }

    #[test]
    fn probing_from_a_nonzero_position_scores_that_window() {
        // The orchestrator is position-honest: it probes from wherever
        // the stream sits (the open path's 0 in practice) and restores
        // that same position — no hidden seek-to-start policy.
        let mut data = vec![0u8; 64];
        data[40..52].copy_from_slice(b"RIFF....AVI ");
        let mut source = CountingSource::new(&data);
        ok(source.seek(plugin_api::SeekOrigin::Start(40)));
        let registry = ProbeRegistry::with_builtins();
        let report = ok(probe_stream(
            &registry,
            &mut source,
            &ok(ProbeConfig::new(64, 60)),
        ));
        assert_eq!(report.format.as_str(), "avi");
        assert_eq!(report.bytes_examined, 24);
        assert_position_restored(&source, plugin_api::SourcePosition(40));
    }
}
