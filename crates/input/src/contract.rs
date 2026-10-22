//! The scripted synthetic source + access conformance checks (card
//! 2.A1; the codec-api `contract` pattern).
//!
//! [`ScriptSource`] is an in-memory byte source with scriptable faults —
//! shrink (a file truncated mid-read), an injected read failure — plus
//! instrumentation of the largest chunk any reader asked it to read and
//! a count of the reads it served. [`SyntheticStream`] wraps it in a
//! [`BufferedReader`](crate::BufferedReader) behind the real
//! [`SourceStream`] trait object shape, so the `assert_*` checks encode
//! the plugin-api contract exactly as the engine will consume it.
//!
//! Every access implementation — this crate, `streaming` (Phase 6), the
//! conformance kit (doc 06 §7.2) — runs the same executable contract
//! from its own tests:
//!
//! - [`assert_stream_contract`]: reads in varied chunk sizes reconstruct
//!   the data exactly; `Ok(0)` only at end of stream; positions always
//!   consistent with bytes consumed; seeks round-trip; seek-to-length
//!   is a clean EOF; seek past length and seek before start are typed
//!   errors.
//! - [`assert_read_into_contract`] (card 3.B3): the borrow twin of the
//!   above over [`SourceStream::read_into`] — varied caller buffer
//!   sizes reconstruct the data exactly, positions track consumption,
//!   `Ok(0)` only at end of stream, a zero-length buffer is a
//!   zero-length success without I/O, and the bytes match a `read`
//!   pass over the same range byte-for-byte (the two APIs never
//!   disagree).
//! - [`assert_peek_into_contract`] (card 3.B3): window-bound peek
//!   honesty for readers that expose it ([`SyntheticStream::peek_into`])
//!   — peeks never advance, never do I/O, and never return more than
//!   what is buffered.
//! - [`assert_bounded_read`]: the card's "no unbounded buffering" proof —
//!   a read many times the buffer capacity returns byte-exact data
//!   while the underlying source never sees a read larger than the
//!   capacity and the internal buffer never exceeds it.
//! - [`assert_truncated`]: a mid-read shrink surfaces the typed
//!   `Corrupt::Truncated` error, never a silent EOF.
//!
//! The checks are `#[track_caller]` assertion functions: a violation is
//! a contract breach (a bug), reported loudly (MP §8). Workspace lints
//! deny `panic!`/`unwrap`; `assert!`/`unreachable!` are the sanctioned
//! test-side vocabulary (testkit precedent).

use media_types::error::CorruptDetail;
use media_types::ids::SourceUri;
use media_types::{EngineError, ErrorKind};
use plugin_api::{SeekOrigin, SourcePosition, SourceStream};

use crate::buffered::BufferedReader;
use crate::error;
use crate::seekable::{RawRead, Seekable};

/// An in-memory byte source with scriptable faults and read
/// instrumentation (doc 03 §4.1(a) "test synthetic sources").
#[derive(Debug)]
pub struct ScriptSource {
    data: Vec<u8>,
    /// The currently visible length: `shrink_to` reduces it to simulate
    /// a file truncated underneath the reader.
    visible_len: u64,
    pos: u64,
    /// The largest `out.len()` any reader has passed to [`RawRead::read`].
    max_read_seen: usize,
    /// How many reads have been served (buffer-hit seeks serve zero).
    reads_served: u64,
    /// A fault to return from the next read instead of data.
    next_failure: Option<EngineError>,
}

impl ScriptSource {
    /// A source over `data`, positioned at zero.
    pub fn new(data: &[u8]) -> ScriptSource {
        ScriptSource {
            data: data.to_vec(),
            visible_len: data.len() as u64,
            pos: 0,
            max_read_seen: 0,
            reads_served: 0,
            next_failure: None,
        }
    }

    /// Simulate the file being truncated to `new_len` bytes: the
    /// visible length drops immediately; a reader positioned past the
    /// new end (or reaching it) observes end-of-stream while its
    /// open-time expectation was larger.
    pub fn shrink_to(&mut self, new_len: u64) {
        self.visible_len = self.visible_len.min(new_len);
    }

    /// Fail the next read with a scripted mid-read I/O failure.
    pub fn fail_next_read(&mut self) {
        self.next_failure = Some(error::read_io(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "scripted read failure",
        )));
    }

    /// The largest read chunk any reader has requested so far.
    pub fn max_read_seen(&self) -> usize {
        self.max_read_seen
    }

    /// How many reads have been served (an instrumented no-I/O proof
    /// for buffer-hit seeks).
    pub fn reads_served(&self) -> u64 {
        self.reads_served
    }
}

impl Seekable for ScriptSource {
    fn position(&self) -> u64 {
        self.pos
    }

    fn length(&self) -> Option<u64> {
        Some(self.visible_len)
    }

    fn seek(&mut self, to: u64) -> Result<u64, EngineError> {
        if to > self.visible_len {
            return Err(error::seek_out_of_range());
        }
        self.pos = to;
        Ok(to)
    }
}

impl RawRead for ScriptSource {
    fn read(&mut self, out: &mut [u8]) -> Result<usize, EngineError> {
        if let Some(failure) = self.next_failure.take() {
            return Err(failure);
        }
        self.max_read_seen = self.max_read_seen.max(out.len());
        self.reads_served += 1;
        let available = self.visible_len.saturating_sub(self.pos);
        let n = (out.len() as u64).min(available) as usize;
        let start = self.pos as usize;
        let end = start + n;
        if end <= self.data.len() {
            out[..n].copy_from_slice(&self.data[start..end]);
        }
        self.pos += n as u64;
        Ok(n)
    }
}

/// A [`ScriptSource`] behind a [`BufferedReader`] behind the real
/// [`SourceStream`] trait — the conformance-kit shape.
pub struct SyntheticStream {
    reader: BufferedReader<ScriptSource>,
    uri: SourceUri,
}

impl SyntheticStream {
    /// Build a synthetic stream over `data` with a `capacity`-sized
    /// bounded reader.
    ///
    /// # Errors
    /// [`EngineError`] when the capacity is rejected (zero / over the
    /// ceiling) or the internal synthetic URI fails validation.
    pub fn new(data: &[u8], capacity: usize) -> Result<SyntheticStream, EngineError> {
        let reader = BufferedReader::new(ScriptSource::new(data), capacity)?;
        let uri = SourceUri::parse("file:///lumen-synthetic-source")?;
        Ok(SyntheticStream { reader, uri })
    }

    /// The scripted source, for fault scripting and instrumentation.
    pub fn source(&self) -> &ScriptSource {
        self.reader.source()
    }

    /// Script a truncation fault (see [`ScriptSource::shrink_to`]).
    pub fn shrink_to(&mut self, new_len: u64) {
        self.reader.source_mut().shrink_to(new_len);
    }
}

impl SourceStream for SyntheticStream {
    fn uri(&self) -> &SourceUri {
        &self.uri
    }

    fn size(&self) -> Option<u64> {
        self.reader.source_length()
    }

    fn supports_seek(&self) -> bool {
        true
    }

    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        self.reader.read(len)
    }

    fn read_into(&mut self, out: &mut [u8]) -> Result<usize, EngineError> {
        // The local-source override (card 3.B3): a direct window→out
        // copy through the bounded reader, no per-call allocation.
        self.reader.read_into(out)
    }

    fn seek(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError> {
        let target = resolve_origin(self.reader.position(), origin)?;
        let to = self.reader.seek(target)?;
        Ok(SourcePosition(to))
    }

    fn position(&self) -> SourcePosition {
        SourcePosition(self.reader.position())
    }
}

impl SyntheticStream {
    /// Peek up to `out.len()` buffered bytes without advancing (card
    /// 3.B3 — the movi walk's header look-ahead): delegates to the
    /// bounded reader's pure window math. Answers 0 when the window
    /// is empty; never more than what is buffered.
    pub fn peek_into(&mut self, out: &mut [u8]) -> Result<usize, EngineError> {
        self.reader.peek_into(out)
    }

    /// The bounded reader's exact-read twin, surfaced for the demux
    /// chain's contract checks.
    pub fn read_exact_into(&mut self, out: &mut [u8]) -> Result<(), EngineError> {
        self.reader.read_exact_into(out)
    }
}

/// Resolve a [`SeekOrigin`] against `from` into an absolute target,
/// rejecting before-start and overflowing positions with typed errors.
pub(crate) fn resolve_origin(from: u64, origin: SeekOrigin) -> Result<u64, EngineError> {
    match origin {
        SeekOrigin::Start(to) => Ok(to),
        SeekOrigin::Current(delta) => {
            let target = i128::from(from) + i128::from(delta);
            if target < 0 {
                Err(error::seek_before_start())
            } else if target > i128::from(u64::MAX) {
                Err(error::seek_overflow())
            } else {
                Ok(target as u64)
            }
        }
    }
}

/// Match a typed truncation error and return `(expected, found)`.
#[track_caller]
pub fn assert_truncated(err: &EngineError) -> (u64, u64) {
    match &err.kind {
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated { expected, found },
        } => (*expected, *found),
        other => unreachable!("expected Corrupt::Truncated, got {other:?} ({err})"),
    }
}

/// The full read/seek/EOF contract over a seekable
/// [`SourceStream`] whose bytes are `expected` (varied chunk sizes,
/// position consistency, clean EOF, typed seek errors).
#[track_caller]
pub fn assert_stream_contract(stream: &mut dyn SourceStream, expected: &[u8]) {
    let len = expected.len() as u64;
    assert_eq!(stream.size(), Some(len), "size is the open-time truth");
    assert!(
        stream.supports_seek(),
        "this contract describes seekable sources"
    );

    // Pass 1: varied chunk sizes; position tracks consumption exactly.
    let mut got = Vec::new();
    let mut pos = 0u64;
    for chunk in [1usize, 2, 3, 7, 64, 1000] {
        loop {
            let piece = stream
                .read(chunk)
                .unwrap_or_else(|e| unreachable!("read: {e}"));
            if piece.is_empty() {
                break;
            }
            got.extend_from_slice(&piece);
            pos += piece.len() as u64;
            assert_eq!(
                stream.position(),
                SourcePosition(pos),
                "position tracks reads"
            );
            if piece.len() < chunk {
                break;
            }
        }
    }
    assert_eq!(got, expected, "varied-size reads reconstruct the data");

    // End of stream is clean and sticky: reads keep returning empty.
    for _ in 0..2 {
        let tail = stream
            .read(16)
            .unwrap_or_else(|e| unreachable!("eof read: {e}"));
        assert!(tail.is_empty(), "Ok(0) only at end of stream");
    }
    assert_eq!(stream.position(), SourcePosition(len));

    // Pass 2: seek to start, one big read returns the same bytes.
    let back = stream
        .seek(SeekOrigin::Start(0))
        .unwrap_or_else(|e| unreachable!("seek 0: {e}"));
    assert_eq!(back, SourcePosition(0));
    let all = stream
        .read(expected.len() + 100)
        .unwrap_or_else(|e| unreachable!("big read: {e}"));
    assert_eq!(
        all, expected,
        "single large read streams through the buffer"
    );

    // Seek to exactly the length: legal, next read is a clean EOF.
    let end = stream
        .seek(SeekOrigin::Start(len))
        .unwrap_or_else(|e| unreachable!("seek len: {e}"));
    assert_eq!(end, SourcePosition(len));
    let eof = stream
        .read(8)
        .unwrap_or_else(|e| unreachable!("eof read: {e}"));
    assert!(eof.is_empty(), "seek-to-length is a clean EOF");

    // Relative seeks round-trip.
    stream
        .seek(SeekOrigin::Start(4))
        .unwrap_or_else(|e| unreachable!("seek 4: {e}"));
    let fwd = stream
        .seek(SeekOrigin::Current(3))
        .unwrap_or_else(|e| unreachable!("cur +3: {e}"));
    assert_eq!(fwd, SourcePosition(7));
    let back = stream
        .seek(SeekOrigin::Current(-7))
        .unwrap_or_else(|e| unreachable!("cur -7: {e}"));
    assert_eq!(back, SourcePosition(0));

    // Seek past the length: typed error, position unchanged.
    let pos_before = stream.position();
    let err = match stream.seek(SeekOrigin::Start(len + 1)) {
        Err(e) => e,
        Ok(_) => unreachable!("a seek past the length must fail typed"),
    };
    assert_eq!(
        err.context.code, "input.seek.out-of-range",
        "past-end seek: {err}"
    );
    assert_eq!(
        stream.position(),
        pos_before,
        "a failed seek never moves the cursor"
    );

    // Seek before the start: typed error.
    let err = match stream.seek(SeekOrigin::Current(-1)) {
        Err(e) => e,
        Ok(_) => unreachable!("a seek before the start must fail typed"),
    };
    assert_eq!(
        err.context.code, "input.seek.before-start",
        "before-start seek: {err}"
    );
}

/// The bounded-memory proof (card 2.A1 AC): reading `want` bytes — many
/// times the buffer capacity — returns byte-exact data while the
/// underlying source never receives a read larger than the capacity and
/// the reader's internal buffer stays within it.
#[track_caller]
pub fn assert_bounded_read(stream: &mut SyntheticStream, want: usize, expected: &[u8]) {
    let capacity = stream.reader.buffer_capacity();
    assert!(capacity > 0, "a zero capacity cannot be constructed");
    assert!(
        want > capacity,
        "the proof needs a request beyond the buffer"
    );
    let from = stream.reader.position() as usize;
    let remaining = &expected[from.min(expected.len())..];
    let data = stream
        .read(want)
        .unwrap_or_else(|e| unreachable!("bounded read: {e}"));
    assert_eq!(
        data.len(),
        want.min(remaining.len()),
        "short only at end of stream"
    );
    assert_eq!(
        data,
        &remaining[..data.len()],
        "byte-exact through the fixed buffer"
    );
    assert!(
        stream.reader.buffered_bytes() <= capacity,
        "internal buffer within capacity ({} > {})",
        stream.reader.buffered_bytes(),
        capacity
    );
    let seen = stream.source().max_read_seen();
    assert!(
        seen <= capacity,
        "underlying reads are chunked to the capacity (saw {seen}, capacity {capacity})"
    );
}

/// The borrow-twin contract (card 3.B3): [`SourceStream::read_into`]
/// over a seekable stream whose bytes are `expected`. Varied caller
/// buffer sizes reconstruct the data exactly; the position tracks
/// consumption; `Ok(0)` only at end of stream; a zero-length buffer is
/// a zero-length success; and a parallel `read` pass over the same
/// bytes agrees byte-for-byte (the two APIs can never disagree).
#[track_caller]
pub fn assert_read_into_contract(stream: &mut dyn SourceStream, expected: &[u8]) {
    let len = expected.len() as u64;
    assert_eq!(stream.size(), Some(len), "size is the open-time truth");

    // Pass 1: varied caller buffer sizes, position tracking.
    let mut got = Vec::new();
    let mut pos = 0u64;
    for size in [1usize, 2, 3, 7, 64, 1000] {
        let mut buf = vec![0u8; size];
        loop {
            let n = stream
                .read_into(&mut buf)
                .unwrap_or_else(|e| unreachable!("read_into: {e}"));
            if n == 0 {
                break;
            }
            assert!(n <= size, "read_into never exceeds the caller buffer");
            got.extend_from_slice(&buf[..n]);
            pos += n as u64;
            assert_eq!(
                stream.position(),
                SourcePosition(pos),
                "position tracks read_into consumption"
            );
            if n < size {
                break;
            }
        }
    }
    assert_eq!(got, expected, "varied-size read_into reconstructs the data");

    // End of stream is clean and sticky through the borrow path too.
    for _ in 0..2 {
        let mut tail = [0u8; 16];
        let n = stream
            .read_into(&mut tail)
            .unwrap_or_else(|e| unreachable!("eof read_into: {e}"));
        assert_eq!(n, 0, "Ok(0) only at end of stream");
    }
    // A zero-length buffer is a zero-length success.
    let n = stream
        .read_into(&mut [])
        .unwrap_or_else(|e| unreachable!("zero-length read_into: {e}"));
    assert_eq!(n, 0);
    assert_eq!(stream.position(), SourcePosition(len));

    // Pass 2: seek to start; one big borrow read returns the same
    // bytes a `read` of the same range returns (the APIs agree).
    stream
        .seek(SeekOrigin::Start(0))
        .unwrap_or_else(|e| unreachable!("seek 0: {e}"));
    let mut whole = vec![0u8; expected.len() + 100];
    let n = stream
        .read_into(&mut whole)
        .unwrap_or_else(|e| unreachable!("big read_into: {e}"));
    assert_eq!(n, expected.len(), "short only at end of stream");
    assert_eq!(&whole[..n], expected, "big borrow read reconstructs");
    stream
        .seek(SeekOrigin::Start(0))
        .unwrap_or_else(|e| unreachable!("seek 0 again: {e}"));
    let via_read = stream
        .read(expected.len() + 100)
        .unwrap_or_else(|e| unreachable!("parallel read: {e}"));
    assert_eq!(via_read, expected, "read still reconstructs");
}

/// The peek contract (card 3.B3): [`SyntheticStream::peek_into`] is
/// window-bound honesty — it never advances the position, never does
/// I/O, and never returns more than what is currently buffered.
#[track_caller]
pub fn assert_peek_into_contract(stream: &mut SyntheticStream, expected: &[u8]) {
    // Prime the window with a small read.
    let mut take = [0u8; 8];
    let n = stream
        .read_into(&mut take)
        .unwrap_or_else(|e| unreachable!("priming read: {e}"));
    assert!(n > 0, "the contract needs a non-empty source");
    let served = stream.source().reads_served();
    let pos = stream.position();

    let mut probe = [0u8; 16];
    for _ in 0..3 {
        let got = stream
            .peek_into(&mut probe)
            .unwrap_or_else(|e| unreachable!("peek_into: {e}"));
        let window = stream.reader.buffered_bytes();
        assert!(
            got <= window,
            "a peek returned {got} bytes but only {window} are buffered — fabrication"
        );
        assert!(
            got <= probe.len(),
            "a peek returned more than the caller buffer"
        );
        // What was peekable must equal the data at the position.
        let at = pos.0 as usize;
        assert_eq!(&probe[..got], &expected[at..at + got], "peeked bytes");
    }
    assert_eq!(stream.position(), pos, "peeks never advance");
    assert_eq!(stream.source().reads_served(), served, "peeks do no I/O");

    // A peek larger than the window returns exactly the window.
    let mut big = [0u8; 8192];
    let got = stream
        .peek_into(&mut big)
        .unwrap_or_else(|e| unreachable!("big peek: {e}"));
    let window = stream.reader.buffered_bytes();
    assert_eq!(got, window, "never more than what is buffered");
    assert_eq!(stream.position(), pos, "still no advancement");
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    fn pattern(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i % 251) as u8).collect()
    }

    #[test]
    fn synthetic_stream_passes_its_own_contract() {
        let data = pattern(5000);
        let mut stream = SyntheticStream::new(&data, 64).must();
        assert_stream_contract(&mut stream, &data);
    }

    /// The synthetic stream passes the borrow-twin contract through
    /// the real trait-object shape (the override path the demuxer
    /// consumes, card 3.B3).
    #[test]
    fn synthetic_stream_passes_the_read_into_contract() {
        let data = pattern(5000);
        let mut stream = SyntheticStream::new(&data, 64).must();
        assert_read_into_contract(&mut stream, &data);
    }

    /// Window-bound peek honesty on the synthetic stream (card 3.B3).
    #[test]
    fn synthetic_stream_passes_the_peek_into_contract() {
        let data = pattern(5000);
        let mut stream = SyntheticStream::new(&data, 64).must();
        assert_peek_into_contract(&mut stream, &data);
    }

    #[test]
    fn bounded_read_proof_on_synthetic_source() {
        let data = pattern(2000);
        let mut stream = SyntheticStream::new(&data, 64).must();
        assert_bounded_read(&mut stream, 1000, &data);
        // The rest of the file also streams through in bounded chunks.
        assert_bounded_read(&mut stream, 1000, &data);
        // ...and a read beyond the data short-reads at end of stream.
        let tail = stream.read(50).unwrap_or_else(|e| unreachable!("{e}"));
        assert!(tail.is_empty());
    }

    /// The card's truncation AC: shrink mid-read surfaces the typed
    /// Corrupt::Truncated error with (expected, found) = (open-time,
    /// shrunk) — never a silent EOF, never a panic.
    #[test]
    fn shrink_mid_read_is_a_typed_truncation_error() {
        let data = pattern(1000);
        let mut stream = SyntheticStream::new(&data, 32).must();
        let first = stream.read(400).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(first.len(), 400);
        stream.shrink_to(400);
        let err = testkit::assert_err(stream.read(400), "truncated read");
        let (expected, found) = assert_truncated(&err);
        assert_eq!(expected, 1000, "expected = the open-time length");
        assert_eq!(found, 400, "found = the shrunk length");
        assert_eq!(err.context.code, "input.file.truncated");
    }

    /// A scripted mid-read I/O failure propagates typed.
    #[test]
    fn scripted_read_failure_propagates_typed() {
        let data = pattern(100);
        let mut stream = SyntheticStream::new(&data, 16).must();
        stream.reader.source_mut().fail_next_read();
        let err = testkit::assert_err(stream.read(10), "scripted read failure");
        assert_eq!(err.context.code, "input.file.read-io");
    }

    /// Seeks inside the *unconsumed* window do no I/O; seeks to
    /// already-consumed positions drop the window and refill.
    #[test]
    fn seek_within_buffer_serves_no_reads() {
        let data = pattern(600);
        let mut stream = SyntheticStream::new(&data, 256).must();
        let _ = stream.read(100).unwrap_or_else(|e| unreachable!("{e}"));
        let served = stream.source().reads_served();
        assert_eq!(served, 1, "one 256-byte refill served the 100-byte read");

        // Forward, inside the unconsumed window [100, 256): index math.
        let pos = stream
            .seek(SeekOrigin::Start(150))
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(pos, SourcePosition(150));
        assert_eq!(
            stream.source().reads_served(),
            served,
            "buffer-hit seek: no I/O"
        );
        let piece = stream.read(10).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(&piece, &data[150..160]);
        assert_eq!(
            stream.source().reads_served(),
            served,
            "served from read-ahead"
        );

        // Back below the window start: the bytes are consumed; the
        // window drops and the next read refills.
        stream
            .seek(SeekOrigin::Start(10))
            .unwrap_or_else(|e| unreachable!("{e}"));
        let piece = stream.read(10).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(&piece, &data[10..20]);
        assert_eq!(
            stream.source().reads_served(),
            served + 1,
            "a refill after the window dropped"
        );
    }

    /// Relative-origin resolution: negatives, overflow, exact bounds.
    #[test]
    fn resolve_origin_math_is_checked() {
        assert_eq!(resolve_origin(10, SeekOrigin::Start(3)).must(), 3);
        assert_eq!(resolve_origin(10, SeekOrigin::Current(5)).must(), 15);
        assert_eq!(resolve_origin(10, SeekOrigin::Current(-10)).must(), 0);
        let before = testkit::assert_err(resolve_origin(0, SeekOrigin::Current(-1)), "before");
        assert_eq!(before.context.code, "input.seek.before-start");
        let over =
            testkit::assert_err(resolve_origin(u64::MAX, SeekOrigin::Current(2)), "overflow");
        assert_eq!(over.context.code, "input.seek.overflow");
        // The u64::MAX boundary itself resolves.
        assert_eq!(
            resolve_origin(u64::MAX - 1, SeekOrigin::Current(1)).must(),
            u64::MAX
        );
    }
}
