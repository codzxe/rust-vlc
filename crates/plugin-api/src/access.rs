//! Access + SourceStream: bytes in (doc 06 §1.2 kinds 1–2).
//!
//! `Access` impls live in `input` (file, disc image — ADR-008) and
//! `streaming` (HTTP(S), HLS, DASH, RTSP, with buffering/reconnect
//! policy). It is the first pipeline stage: it feeds the probe and the
//! demuxer. Instance scope: one per open source (§1.3); teardown rides
//! the pipeline generation (doc 03 §5).

use std::fmt;

use media_types::ids::SourceUri;
use media_types::{EngineError, PipelineStage};

/// A byte source opened by [`Access`] (doc 06 §1.2 rows 1–2).
///
/// Split from the factory trait so the engine can hold one stream per
/// open source while asking the registry for *candidates* per scheme.
/// The stream is **pull-driven** and **positional**: the demuxer asks
/// for ranges; live sources simply report
/// [`supports_seek`](SourceStream::supports_seek) = false.
///
/// # Calling context
///
/// Called from the input worker thread (doc 01 §4.1); every method
/// must return (bounded work per call — a read of `len` bytes may
/// return fewer, never block forever; network impls use the engine's
/// timeout + reconnect budget and surface
/// [`NetworkCause`](media_types::error::NetworkCause) errors).
pub trait SourceStream: Send {
    /// The URI actually opened (post-redirect for network sources).
    fn uri(&self) -> &SourceUri;

    /// Total size when known — live streams report `None`
    /// (a value and its absence are different types, MP §8).
    fn size(&self) -> Option<u64>;

    /// Whether positional seeks are served (§2.2 access capability
    /// `supports_seek` — must agree with the manifest claim; the
    /// conformance kit checks this honesty, doc 06 §7.2).
    fn supports_seek(&self) -> bool;

    /// Read up to `len` bytes at the current position.
    ///
    /// Returns `Ok(0)` **only** at end of stream; fewer than `len`
    /// bytes is normal (short reads are data, not errors).
    ///
    /// # Errors
    /// [`EngineError`] — network causes for streams, `AccessDenied`
    /// for permission failures, always with stage
    /// [`PipelineStage::Open`].
    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError>;

    /// Read up to `out.len()` bytes into caller memory at the current
    /// position — the borrow twin of [`read`](SourceStream::read)
    /// (card 3.B3, directives-audit M4's root-cause fix).
    ///
    /// Same contract as `read` (short reads are data; `Ok(0)` only at
    /// end of stream; a zero-length buffer is a zero-length success
    /// without I/O) minus the per-call [`Vec`] allocation: the bytes
    /// land in memory the caller already owns. Local sources override
    /// this with a direct copy out of their buffer; the provided
    /// default allocates once and copies, so every existing
    /// implementation keeps working unchanged.
    ///
    /// # Errors
    /// [`EngineError`] — the source's own read errors (stage
    /// [`PipelineStage::Open`]); errors leave the position unchanged
    /// where the source can honor that.
    fn read_into(&mut self, out: &mut [u8]) -> Result<usize, EngineError> {
        let chunk = self.read(out.len())?;
        out[..chunk.len()].copy_from_slice(&chunk);
        Ok(chunk.len())
    }

    /// Reposition when [`supports_seek`](SourceStream::supports_seek).
    ///
    /// # Errors
    /// [`EngineError`] with stage [`PipelineStage::Open`] when the
    /// source cannot seek or the offset is out of range.
    fn seek(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError>;

    /// Current position (by definition known — sources are positional;
    /// live sources still advance through the byte stream).
    fn position(&self) -> SourcePosition;
}

/// Seek reference point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeekOrigin {
    /// From the start of the stream.
    Start(u64),
    /// From the current position (may be negative).
    Current(i64),
}

/// An absolute byte position in a source (newtype, MP §8: no bare
/// numbers crossing interfaces).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourcePosition(pub u64);

impl fmt::Display for SourcePosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} bytes", self.0)
    }
}

/// The access factory (doc 06 §1.2 rows 1–2): open a source for a
/// URI whose scheme the manifest claims.
///
/// The engine drives a **probe → candidates → selection** pass per doc
/// 06 §3.2 — this trait is what a selected candidate provides.
///
/// # Calling context
///
/// `open` runs on a worker thread and may perform bounded I/O
/// (connect, TLS, redirect budget); it must return. Reconnect policy
/// is the engine's loop around fresh `open` calls, not an internal
/// forever-loop.
pub trait Access: Send {
    /// Whether this access impl serves `uri`'s scheme (cross-checked
    /// against the manifest's `schemes` capability claim — doc 06
    /// §7.2 capability honesty).
    fn accepts(&self, uri: &SourceUri) -> bool;

    /// Open the source.
    ///
    /// # Errors
    /// [`EngineError`] — `AccessDenied` for permission failures,
    /// network causes for streams; stage [`PipelineStage::Open`].
    fn open(&self, uri: &SourceUri) -> Result<Box<dyn SourceStream>, EngineError>;
}

/// Typed open-stage error for access impls: scheme not served.
pub fn scheme_not_served(uri: &SourceUri) -> EngineError {
    EngineError::new(
        PipelineStage::Open,
        media_types::ErrorKind::Unsupported {
            format: Some(media_types::error::FormatInfo {
                name: Some(uri.scheme().as_str().to_owned()),
                sniffed: Vec::new(),
            }),
        },
        media_types::error::ErrorContext::new("plugin.access.scheme-not-served"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal in-memory source stream (the fixture every access
    /// test below builds on).
    struct MemSource {
        uri: SourceUri,
        data: &'static [u8],
        pos: usize,
    }

    impl SourceStream for MemSource {
        fn uri(&self) -> &SourceUri {
            &self.uri
        }
        fn size(&self) -> Option<u64> {
            Some(self.data.len() as u64)
        }
        fn supports_seek(&self) -> bool {
            true
        }
        fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
            let end = (self.pos + len).min(self.data.len());
            let out = self.data[self.pos..end].to_vec();
            self.pos = end;
            Ok(out)
        }
        fn seek(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError> {
            let target: u64 = match origin {
                SeekOrigin::Start(o) => o,
                SeekOrigin::Current(d) => {
                    let cur = self.pos as i64;
                    match cur.checked_add(d) {
                        Some(n) if n >= 0 => n as u64,
                        _ => {
                            return Err(EngineError::new(
                                PipelineStage::Open,
                                media_types::ErrorKind::Inaccessible {
                                    reason: media_types::error::AccessDenied::PermissionDenied,
                                },
                                media_types::error::ErrorContext::new(
                                    "plugin.access.seek-before-start",
                                ),
                            ));
                        }
                    }
                }
            };
            if target > self.data.len() as u64 {
                return Err(EngineError::new(
                    PipelineStage::Open,
                    media_types::ErrorKind::Inaccessible {
                        reason: media_types::error::AccessDenied::NotFound,
                    },
                    media_types::error::ErrorContext::new("plugin.access.seek-out-of-range"),
                ));
            }
            self.pos = target as usize;
            Ok(SourcePosition(target))
        }
        fn position(&self) -> SourcePosition {
            SourcePosition(self.pos as u64)
        }
    }

    struct MemAccess;

    impl Access for MemAccess {
        fn accepts(&self, uri: &SourceUri) -> bool {
            uri.scheme().as_str() == "file"
        }
        fn open(&self, uri: &SourceUri) -> Result<Box<dyn SourceStream>, EngineError> {
            if !self.accepts(uri) {
                return Err(scheme_not_served(uri));
            }
            Ok(Box::new(MemSource {
                uri: uri.clone(),
                data: b"matroska-bytes",
                pos: 0,
            }))
        }
    }

    #[test]
    fn mem_source_reads_seeks_and_errors() {
        // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
        let uri = match SourceUri::parse("file:///tmp/a.mkv") {
            Ok(u) => u,
            Err(e) => unreachable!("{e}"),
        };
        let access = MemAccess;
        assert!(access.accepts(&uri));
        let mut stream = match access.open(&uri) {
            Ok(s) => s,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(stream.size(), Some(14));
        let first = match stream.read(4) {
            Ok(b) => b,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(first, b"matr");
        // Seek back to start and re-read.
        let pos = match stream.seek(SeekOrigin::Start(0)) {
            Ok(p) => p,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(pos, SourcePosition(0));
        // Relative seek forward.
        let pos = match stream.seek(SeekOrigin::Current(4)) {
            Ok(p) => p,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(pos, SourcePosition(4));
        let tail = match stream.read(100) {
            Ok(b) => b,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(tail, b"oska-bytes", "short read to EOS is fine");
        let eos = match stream.read(8) {
            Ok(b) => b,
            Err(e) => unreachable!("{e}"),
        };
        assert!(eos.is_empty(), "zero only at EOS");
        // Out of range.
        assert!(stream.seek(SeekOrigin::Start(99)).is_err());
        // Before start.
        assert!(stream.seek(SeekOrigin::Current(-99)).is_err());
    }

    #[test]
    fn scheme_mismatch_is_a_typed_error() {
        // lumen-allow-literal: URL parse/redaction test vector — data under test
        let uri = match SourceUri::parse("https://example.com/a") {
            Ok(u) => u,
            Err(e) => unreachable!("{e}"),
        };
        let err = match MemAccess.open(&uri) {
            Ok(_) => unreachable!("file access must refuse https"),
            Err(e) => e,
        };
        assert_eq!(err.stage, PipelineStage::Open);
        assert!(matches!(
            err.kind,
            media_types::ErrorKind::Unsupported { .. }
        ));
    }

    /// The provided `read_into` default (the alloc-copy over `read`)
    /// honors the trait contract for implementations that do not
    /// override it: varied buffer sizes reconstruct the data, the
    /// position tracks consumption, `Ok(0)` only at end of stream,
    /// and a zero-length buffer is a zero-length success.
    #[test]
    fn default_read_into_reconstructs_and_tracks_position() {
        // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
        let uri = match SourceUri::parse("file:///tmp/a.mkv") {
            Ok(u) => u,
            Err(e) => unreachable!("{e}"),
        };
        let mut stream = match MemAccess.open(&uri) {
            Ok(s) => s,
            Err(e) => unreachable!("{e}"),
        };
        let expected: &[u8] = b"matroska-bytes";

        let mut got = Vec::new();
        let mut scratch = [0u8; 3];
        loop {
            let n = match stream.read_into(&mut scratch) {
                Ok(n) => n,
                Err(e) => unreachable!("{e}"),
            };
            if n == 0 {
                break;
            }
            assert!(n <= scratch.len());
            got.extend_from_slice(&scratch[..n]);
            assert_eq!(
                stream.position().0 as usize,
                got.len(),
                "position tracks read_into consumption"
            );
        }
        assert_eq!(got, expected, "varied-size read_into reconstructs");

        // Zero-length buffer: zero-length success, no position move.
        let n = match stream.read_into(&mut []) {
            Ok(n) => n,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(n, 0);
        assert_eq!(stream.position().0 as usize, expected.len());

        // Seek back and read the whole payload in one caller buffer.
        match stream.seek(SeekOrigin::Start(0)) {
            Ok(_) => {}
            Err(e) => unreachable!("{e}"),
        }
        let mut whole = [0u8; 64];
        let n = match stream.read_into(&mut whole) {
            Ok(n) => n,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(&whole[..n], expected, "one big borrow read");
    }
}
