//! The instrumented sources + probe conformance checks (card 2.A2; the
//! input crate's `contract` pattern, doc 03 §4.1/§4.2).
//!
//! **Test/conformance-only surface (audit F-2):** the whole module is
//! `#[cfg(any(test, feature = "contract"))]` — unit tests compile it
//! always, integration tests and the doc 06 §7 conformance kit enable
//! the `contract` feature, and **release binaries carry none of it**
//! (the executable-contract `assert_*`/`unreachable!` vocabulary is a
//! failure signal for implementations under test, not a runtime
//! condition).
//!
//! [`CountingSource`] is an in-memory [`SourceStream`] and
//! [`CountingStream`] a pass-through adapter over any opened stream
//! (the real access stack) — both record every read request they
//! served: the lengths asked for, the bytes actually handed out per
//! request, the largest single request — plus scriptable faults on the
//! in-memory side (a mid-read failure, non-seekability, short-read
//! chunking). The `assert_*` checks encode the probe-side contract
//! exactly as the engine will consume it, so the same executable proof
//! serves this crate's tests, the future playback-engine wiring tests
//! and the doc 06 §7 conformance kit:
//!
//! - [`assert_probe_depth_bounded`]: the card's "probe-depth bound
//!   tested" AC — after probing, the source never served more than
//!   `max_lookahead` bytes in total, and no single request asked for
//!   more than the budget *remaining at that moment*, where the budget
//!   is spent by the bytes actually **served** (the fill loop's own
//!   arithmetic: it asks for exactly `max_lookahead − window.len()`).
//! - [`assert_position_restored`]: doc 03 §4.2(e) "the probe never
//!   consumes stream bytes" — the stream is back where it started and
//!   still serves the same bytes.
//!
//! The checks are `#[track_caller]` assertion functions: a violation
//! is a contract breach (a bug), reported loudly (MP §8). Workspace
//! lints deny `panic!`/`unwrap`; `assert!`/`unreachable!` are the
//! sanctioned test-side vocabulary (testkit precedent).

use media_types::ids::SourceUri;
use media_types::{EngineError, PipelineStage};
use plugin_api::{SeekOrigin, SourcePosition, SourceStream};

/// The read ledger every instrumented source in this module exposes:
/// the requested lengths and the actually-served lengths of every
/// read, in order (faulted reads are excluded — they served nothing).
///
/// Implemented by the in-memory [`CountingSource`] and the real-stack
/// adapter [`CountingStream`]; consumed by
/// [`assert_probe_depth_bounded`].
pub trait ReadLog {
    /// `(requested, served)` per read, in order — parallel ledgers.
    fn read_log(&self) -> (&[usize], &[usize]);
}

/// An in-memory byte source with read instrumentation and scriptable
/// faults (doc 03 §4.2(f) "instrumented stream asserts this").
#[derive(Debug)]
pub struct CountingSource {
    data: Vec<u8>,
    pos: usize,
    uri: SourceUri,
    seekable: bool,
    /// Serve at most this many bytes per read (usize::MAX = whole
    /// request) — the short-read chunking a real network source shows.
    chunk_limit: usize,
    /// Every read length requested, in order (faulted reads excluded).
    requests: Vec<usize>,
    /// Bytes actually served per request, in order (same length as
    /// `requests`; the two together are the fill loop's budget ledger).
    served: Vec<usize>,
    total_served: usize,
    max_request: usize,
    /// A fault to return from the next read instead of data.
    next_failure: Option<EngineError>,
}

impl CountingSource {
    /// A seekable source over `data` with a neutral file URI.
    ///
    /// # Panics (constructor contract)
    /// Only when the built-in URI literal fails validation — it cannot
    /// (it is a literal; the L0 constructor is exercised by its own
    /// tests). Kept non-`Result` so assertions stay one line.
    #[allow(clippy::missing_panics_doc)]
    pub fn new(data: &[u8]) -> CountingSource {
        let uri = match SourceUri::parse("file:///counting-source.bin") {
            Ok(uri) => uri,
            Err(err) => unreachable!("literal URI is valid: {err}"),
        };
        CountingSource {
            data: data.to_vec(),
            pos: 0,
            uri,
            seekable: true,
            chunk_limit: usize::MAX,
            requests: Vec::new(),
            served: Vec::new(),
            total_served: 0,
            max_request: 0,
            next_failure: None,
        }
    }

    /// A source over `data` whose URI is `uri` (extension-hint tests).
    pub fn with_uri(data: &[u8], uri: &SourceUri) -> CountingSource {
        let mut source = CountingSource::new(data);
        source.uri = uri.clone();
        source
    }

    /// Make the source non-seekable (a live-stream stand-in).
    pub fn set_unseekable(&mut self) {
        self.seekable = false;
    }

    /// Serve at most `limit` bytes per read (short-read chunking).
    pub fn set_chunk_limit(&mut self, limit: usize) {
        self.chunk_limit = limit;
    }

    /// Fail the next read with a scripted mid-read failure.
    pub fn fail_next_read(&mut self) {
        self.next_failure = Some(EngineError::new(
            PipelineStage::Open,
            media_types::ErrorKind::Inaccessible {
                reason: media_types::error::AccessDenied::PermissionDenied,
            },
            media_types::error::ErrorContext::new("probe-test.scripted-read-failure"),
        ));
    }

    /// Every read length requested, in order.
    pub fn requests(&self) -> &[usize] {
        &self.requests
    }

    /// Bytes actually served per request, in order.
    pub fn served(&self) -> &[usize] {
        &self.served
    }

    /// Total bytes handed out across all reads.
    pub fn total_served(&self) -> usize {
        self.total_served
    }

    /// The largest single read length requested.
    pub fn max_request(&self) -> usize {
        self.max_request
    }
}

impl SourceStream for CountingSource {
    fn uri(&self) -> &SourceUri {
        &self.uri
    }

    fn size(&self) -> Option<u64> {
        Some(self.data.len() as u64)
    }

    fn supports_seek(&self) -> bool {
        self.seekable
    }

    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        if let Some(failure) = self.next_failure.take() {
            return Err(failure);
        }
        self.requests.push(len);
        self.max_request = self.max_request.max(len);
        let want = len.min(self.chunk_limit);
        let available = self.data.len().saturating_sub(self.pos);
        let take = want.min(available);
        let out = self.data[self.pos..self.pos + take].to_vec();
        self.pos += take;
        self.served.push(take);
        self.total_served += take;
        Ok(out)
    }

    fn seek(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError> {
        if !self.seekable {
            return Err(EngineError::new(
                PipelineStage::Open,
                media_types::ErrorKind::Unsupported { format: None },
                media_types::error::ErrorContext::new("probe-test.stream-not-seekable"),
            ));
        }
        let target = match origin {
            SeekOrigin::Start(offset) => offset,
            SeekOrigin::Current(delta) => {
                let current = self.pos as i64;
                match current.checked_add(delta) {
                    Some(n) if n >= 0 => n as u64,
                    _ => {
                        return Err(EngineError::new(
                            PipelineStage::Open,
                            media_types::ErrorKind::Inaccessible {
                                reason: media_types::error::AccessDenied::NotFound,
                            },
                            media_types::error::ErrorContext::new("probe-test.seek-before-start"),
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
                media_types::error::ErrorContext::new("probe-test.seek-out-of-range"),
            ));
        }
        self.pos = target as usize;
        Ok(SourcePosition(target))
    }

    fn position(&self) -> SourcePosition {
        SourcePosition(self.pos as u64)
    }
}

impl ReadLog for CountingSource {
    fn read_log(&self) -> (&[usize], &[usize]) {
        (&self.requests, &self.served)
    }
}

/// A pass-through [`SourceStream`] that records every read it
/// forwards — the real-stack counterpart of [`CountingSource`].
///
/// Wrap the stream an [`Access`](plugin_api::Access) opened and the
/// same probe-side assertions apply to the composed stack (the
/// "instrumented stream" of doc 03 §4.2(f), applied over `input`'s
/// buffered `FileStream` rather than a synthetic source). Seeks are
/// position bookkeeping, not data flow, and are forwarded unlogged;
/// the depth bound is a bound on **reads**.
pub struct CountingStream {
    inner: Box<dyn SourceStream>,
    requests: Vec<usize>,
    served: Vec<usize>,
    total_served: usize,
    max_request: usize,
}

impl CountingStream {
    /// Wrap an opened stream.
    pub fn new(inner: Box<dyn SourceStream>) -> CountingStream {
        CountingStream {
            inner,
            requests: Vec::new(),
            served: Vec::new(),
            total_served: 0,
            max_request: 0,
        }
    }

    /// Every read length requested, in order.
    pub fn requests(&self) -> &[usize] {
        &self.requests
    }

    /// Bytes actually served per request, in order.
    pub fn served(&self) -> &[usize] {
        &self.served
    }

    /// Total bytes handed out across all reads.
    pub fn total_served(&self) -> usize {
        self.total_served
    }

    /// The largest single read length requested.
    pub fn max_request(&self) -> usize {
        self.max_request
    }
}

impl SourceStream for CountingStream {
    fn uri(&self) -> &SourceUri {
        self.inner.uri()
    }

    fn size(&self) -> Option<u64> {
        self.inner.size()
    }

    fn supports_seek(&self) -> bool {
        self.inner.supports_seek()
    }

    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        let chunk = self.inner.read(len)?;
        self.requests.push(len);
        self.max_request = self.max_request.max(len);
        self.served.push(chunk.len());
        self.total_served += chunk.len();
        Ok(chunk)
    }

    fn seek(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError> {
        self.inner.seek(origin)
    }

    fn position(&self) -> SourcePosition {
        self.inner.position()
    }
}

impl ReadLog for CountingStream {
    fn read_log(&self) -> (&[usize], &[usize]) {
        (&self.requests, &self.served)
    }
}

/// Assert the probe-depth bound over an instrumented source (the
/// card's AC): total bytes served ≤ `max_lookahead`, and every
/// individual request ≤ the budget remaining at that moment — where
/// the budget is spent by the bytes actually **served** (a short read
/// spends only what it handed out; the fill loop's own arithmetic is
/// `ask = max_lookahead − window.len()`).
#[track_caller]
pub fn assert_probe_depth_bounded(source: &impl ReadLog, max_lookahead: usize) {
    let (requests, served) = source.read_log();
    assert_eq!(
        requests.len(),
        served.len(),
        "the read log's two ledgers must be parallel"
    );
    let mut budget = max_lookahead;
    let mut total = 0usize;
    for (index, (&request, &got)) in requests.iter().zip(served).enumerate() {
        assert!(
            request <= budget,
            "read #{index} asked for {request} with only {budget} of the \
             {max_lookahead}-byte budget left"
        );
        budget -= got.min(budget);
        total += got;
    }
    assert!(
        total <= max_lookahead,
        "probe read {total} bytes total, bound is {max_lookahead}"
    );
}

/// Assert doc 03 §4.2(e)'s non-consumption: the stream sits at
/// `before` again (and the caller can re-read the same bytes — the
/// stronger form is tested where the data is at hand).
#[track_caller]
pub fn assert_position_restored(stream: &dyn SourceStream, before: SourcePosition) {
    let now = stream.position();
    assert_eq!(
        now, before,
        "probe must restore the stream position (doc 03 §4.2(e))"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counting_source_serves_and_instruments() {
        let mut source = CountingSource::new(b"0123456789");
        let first = match source.read(4) {
            Ok(b) => b,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(first, b"0123");
        let second = match source.read(100) {
            Ok(b) => b,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(second, b"456789", "short read to EOS is data");
        let eos = match source.read(8) {
            Ok(b) => b,
            Err(e) => unreachable!("{e}"),
        };
        assert!(eos.is_empty(), "zero only at EOS");
        assert_eq!(source.requests(), &[4, 100, 8]);
        assert_eq!(source.total_served(), 10);
        assert_eq!(source.max_request(), 100);
        // Seek round-trip.
        let pos = match source.seek(SeekOrigin::Start(2)) {
            Ok(p) => p,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(pos, SourcePosition(2));
        let again = match source.read(3) {
            Ok(b) => b,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(again, b"234");
    }

    #[test]
    fn unseekable_and_fault_scripting_work() {
        let mut source = CountingSource::new(b"abcdef");
        source.set_unseekable();
        assert!(!source.supports_seek());
        assert!(source.seek(SeekOrigin::Start(0)).is_err());
        source.fail_next_read();
        let err = match source.read(2) {
            Ok(_) => unreachable!("scripted failure"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "probe-test.scripted-read-failure");
        // The failed read is not counted as a served request beyond
        // the instrumentation of the attempt itself.
        let ok = match source.read(2) {
            Ok(b) => b,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(ok, b"ab");
    }

    #[test]
    fn chunk_limit_simulates_short_reads() {
        let mut source = CountingSource::new(&[0x42; 100]);
        source.set_chunk_limit(7);
        let mut got = Vec::new();
        while got.len() < 30 {
            let chunk = match source.read(30 - got.len()) {
                Ok(b) => b,
                Err(e) => unreachable!("{e}"),
            };
            if chunk.is_empty() {
                break;
            }
            got.extend(chunk);
        }
        assert_eq!(got.len(), 30);
        assert!(source.requests().iter().all(|&r| r <= 30));
        assert_eq!(source.total_served(), 30);
    }

    #[test]
    fn depth_assertions_fail_loudly_on_violations() {
        let mut source = CountingSource::new(b"0123456789");
        let _ = source.read(6);
        // In-bounds: passes.
        assert_probe_depth_bounded(&source, 6);
        assert_probe_depth_bounded(&source, 10);
        // Out of bounds: proven to fail via catch_unwind (the check's
        // own contract, asserted once here like testkit does).
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_probe_depth_bounded(&source, 5)
        }));
        assert!(caught.is_err(), "the bound assertion must fire");
        // A short read spends only what it served: 6 asked, 4 served
        // leaves budget for a later ask — the ledger semantics.
        let mut short = CountingSource::new(b"abcd");
        short.set_chunk_limit(4);
        let _ = short.read(6); // asks 6, serves 4
        let _ = short.read(2); // asks 2, serves 0 (EOS)
        assert_probe_depth_bounded(&short, 6);
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_probe_depth_bounded(&short, 5)
        }));
        assert!(caught.is_err(), "total-served bound must fire at 5");
    }

    #[test]
    fn counting_stream_forwards_and_instruments() {
        // The adapter over the in-memory source (the same delegation
        // it performs over input's FileStream in the integration
        // suite): identical bytes, parallel ledgers.
        let inner = CountingSource::new(b"RIFF....AVI xxxx");
        let mut stream = CountingStream::new(Box::new(inner));
        assert_eq!(stream.uri().scheme().as_str(), "file");
        assert_eq!(stream.size(), Some(16));
        assert!(stream.supports_seek());
        let first = match stream.read(12) {
            Ok(b) => b,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(first, b"RIFF....AVI ");
        let rest = match stream.read(100) {
            Ok(b) => b,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(rest, b"xxxx");
        assert_eq!(stream.requests(), &[12, 100]);
        assert_eq!(stream.served(), &[12, 4]);
        assert_eq!(stream.total_served(), 16);
        assert_eq!(stream.max_request(), 100);
        assert_eq!(stream.position(), SourcePosition(16));
        let pos = match stream.seek(SeekOrigin::Start(0)) {
            Ok(p) => p,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(pos, SourcePosition(0));
        let replay = match stream.read(4) {
            Ok(b) => b,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(replay, b"RIFF");
        // The depth ledger over the adapter: with a bound covering the
        // asks (12 + 100 + 4), every request fits its remaining budget
        // and the 20 served bytes fit the bound.
        assert_probe_depth_bounded(&stream, 112);
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_probe_depth_bounded(&stream, 15)
        }));
        assert!(caught.is_err(), "a 15-byte budget cannot cover these asks");
    }
}
