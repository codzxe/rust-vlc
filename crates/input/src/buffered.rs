//! The bounded buffered reader (card 2.A1, doc 03 §4.1 + MP §12).
//!
//! [`BufferedReader`] wraps any [`RawRead`] + [`Seekable`](crate::Seekable)
//! source with a **fixed-capacity** buffer allocated once at
//! construction and never grown. Small reads (below the capacity) are
//! served from the window with read-ahead locality; requests **at or
//! above the capacity** bypass the window entirely — leftover
//! read-ahead is drained first, then the source is asked directly for
//! the remainder in caller-visible memory (one transfer instead of
//! `want / capacity` window refills — the 7-c part-2 payload-class
//! path). A single read request above [`MAX_READ_BYTES`] is refused
//! with a typed `ResourceLimit` error instead of an allocation. A
//! source that shrinks below its open-time length surfaces
//! `Corrupt::Truncated`, never a silent EOF.
//!
//! # Memory accounting (the card's "no unbounded buffering" proof)
//!
//! Steady-state memory = the fixed buffer (≤ [`MAX_BUFFER_BYTES`]) + the
//! caller-visible result of one bounded read (≤ [`MAX_READ_BYTES`]).
//! The bypass reads straight into memory the caller already owns (the
//! borrow path) or the result `Vec` it would have allocated anyway —
//! the reader never asks the source for more than the caller's own
//! request, and the internal window never grows. The
//! [`contract`](crate::contract) checks instrument the underlying
//! source so tests can prove both bounds.
//!
//! # Position invariant
//!
//! `source.position() == buf_pos + buffered_bytes()` always holds: the
//! source cursor sits just past the buffered window, `buf_pos` is the
//! absolute offset of the window's first valid byte, and `position()`
//! reads `buf_pos` without touching the source (a `&self` query, like
//! `plugin_api::SourceStream::position`).

use media_types::EngineError;

use crate::error;
use crate::seekable::{RawRead, Seekable};

/// Default read-ahead: the 64 KiB internal-chunk figure of doc 03
/// §4.1(e). `[PROPOSAL]` — the cancellation-bound chunk size the spec
/// sketch named; also a comfortable probe window.
pub const DEFAULT_BUFFER_BYTES: usize = 64 * 1024;

/// Hard ceiling on a single reader's buffer (16 MiB, the probe-depth
/// scale of doc 03 §4.2). `[PROPOSAL]` — bounded memory is a contract
/// (MP §12); the ceiling keeps a misconfigured caller from turning
/// "bounded" into "one huge allocation".
pub const MAX_BUFFER_BYTES: usize = 16 * 1024 * 1024;

/// Hard ceiling on a single read request (64 MiB). `[PROPOSAL]` — a
/// demuxer reads packets and a probe reads a window; nothing legitimate
/// asks for more in one call, and `usize::MAX`-shaped requests must
/// fail typed, not abort the allocator.
pub const MAX_READ_BYTES: usize = 64 * 1024 * 1024;

/// A fixed-capacity buffered reader over a positional byte source.
///
/// The buffer is allocated once ([`Vec`] of exactly `capacity` bytes)
/// and never reallocated: reads refill it only when it is completely
/// empty, so leftover bytes survive as read-ahead for the next call and
/// seeks inside the buffered window cost no I/O.
pub struct BufferedReader<S> {
    source: S,
    /// The fixed window: `buf[start..end]` is valid read-ahead.
    buf: Vec<u8>,
    start: usize,
    end: usize,
    /// Absolute position of `buf[start]` (the reader's position).
    buf_pos: u64,
    /// The source length observed at construction — the truncation
    /// reference (card 2.A1 AC).
    declared_len: Option<u64>,
}

impl<S: RawRead + Seekable> BufferedReader<S> {
    /// Build a reader with a `capacity`-byte fixed buffer.
    ///
    /// # Errors
    /// [`EngineError`] (kind `Unsupported`, `input.buffer.capacity-zero`)
    /// when `capacity` is zero, or (kind `ResourceLimit`,
    /// `input.buffer.capacity-out-of-range`) when it exceeds
    /// [`MAX_BUFFER_BYTES`].
    pub fn new(source: S, capacity: usize) -> Result<BufferedReader<S>, EngineError> {
        if capacity == 0 {
            return Err(error::capacity_zero());
        }
        if capacity > MAX_BUFFER_BYTES {
            return Err(error::capacity_over_budget(capacity, MAX_BUFFER_BYTES));
        }
        let declared_len = source.length();
        let buf_pos = source.position();
        Ok(BufferedReader {
            source,
            buf: vec![0u8; capacity],
            start: 0,
            end: 0,
            buf_pos,
            declared_len,
        })
    }

    /// Build a reader with [`DEFAULT_BUFFER_BYTES`].
    ///
    /// # Errors
    /// As [`BufferedReader::new`] (the default capacity is in range by
    /// construction; the `Result` keeps failure non-panicking).
    pub fn with_default_capacity(source: S) -> Result<BufferedReader<S>, EngineError> {
        BufferedReader::new(source, DEFAULT_BUFFER_BYTES)
    }

    /// The absolute position of the next byte a [`read`](BufferedReader::read)
    /// would return — an `&self` query; the source is never touched.
    pub fn position(&self) -> u64 {
        self.buf_pos
    }

    /// The fixed buffer's capacity (never changes after construction).
    pub fn buffer_capacity(&self) -> usize {
        self.buf.len()
    }

    /// Bytes currently buffered (read-ahead) — always ≤ capacity.
    pub fn buffered_bytes(&self) -> usize {
        self.end - self.start
    }

    /// The source's current length, queried live (growing files report
    /// their new truth; `None` when unobservable).
    pub fn source_length(&self) -> Option<u64> {
        self.source.length()
    }

    /// The wrapped source (fault scripting, instrumentation).
    pub fn source(&self) -> &S {
        &self.source
    }

    /// The wrapped source, mutable (fault scripting).
    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }

    /// Reposition to the absolute offset `to`.
    ///
    /// A seek inside the buffered window (or exactly at its end)
    /// adjusts indices only — no I/O. Anything else drops the window
    /// and delegates to the source's own typed seek.
    ///
    /// # Errors
    /// The source's seek errors propagate unchanged (typed
    /// `input.seek.out-of-range` for targets beyond a bounded source).
    pub fn seek(&mut self, to: u64) -> Result<u64, EngineError> {
        let window_end = self.buf_pos + self.buffered_bytes() as u64;
        if to >= self.buf_pos && to < window_end {
            self.start += (to - self.buf_pos) as usize;
            self.buf_pos = to;
            return Ok(to);
        }
        if to == window_end {
            // The source cursor is already there (position invariant);
            // dropping the window is enough.
            self.start = 0;
            self.end = 0;
            self.buf_pos = to;
            return Ok(to);
        }
        let at = self.source.seek(to)?;
        self.start = 0;
        self.end = 0;
        self.buf_pos = at;
        Ok(at)
    }

    /// Read up to `want` bytes at the current position.
    ///
    /// Two regimes (7-c part 2):
    /// - `want <` the window capacity: served from the fixed window
    ///   with read-ahead locality (small asks, header probes).
    /// - `want ≥` the window capacity — the payload class the demux
    ///   arena asks for: leftover read-ahead is drained, then the
    ///   source is asked **directly** for the remainder into the
    ///   result — one full-size transfer for the bulk instead of
    ///   `want / capacity` window refills, and no window→result copy
    ///   for the transferred bytes (the source writes through), plus
    ///   one capacity-bounded read-ahead refill so the following
    ///   header-sized ask stays a window hit.
    ///
    /// Returns fewer than `want` bytes only at end of stream (short
    /// reads are data, not errors); `Ok(empty)` only at end of stream.
    ///
    /// # Errors
    /// [`EngineError`] — kind `ResourceLimit`
    /// (`input.read.request-over-limit`) when `want` exceeds
    /// [`MAX_READ_BYTES`]; kind `Corrupt` (`input.file.truncated`) when
    /// the source ends before its open-time length; the source's own
    /// read folds otherwise.
    pub fn read(&mut self, want: usize) -> Result<Vec<u8>, EngineError> {
        if want == 0 {
            return Ok(Vec::new());
        }
        if want > MAX_READ_BYTES {
            return Err(error::read_over_budget(want, MAX_READ_BYTES));
        }
        if want >= self.buf.len() {
            return self.read_bypassing_window(want);
        }
        let mut out = Vec::with_capacity(want);
        self.drain_into(&mut out);
        while out.len() < want {
            if self.start == self.end {
                // Window empty: refill the whole fixed capacity in one
                // chunk — this is the bounded-transfer step.
                self.start = 0;
                self.end = 0;
                let filled = self.source.read(&mut self.buf)?;
                if filled == 0 {
                    self.confirm_eof()?;
                    break;
                }
                self.end = filled;
            }
            self.drain_into(&mut out);
        }
        Ok(out)
    }

    /// The payload-class path (7-c part 2): a request at or above the
    /// window capacity. Drains leftover read-ahead, then asks the
    /// source directly for the remainder — the bytes land in the
    /// caller-visible result without passing through the window. A
    /// short-but-nonzero source read is re-asked for the remainder;
    /// `Ok(0)` is end of stream (typed truncation check applies). The
    /// zero-filled `Vec` is the price of the safe (no-`unsafe`) API:
    /// `RawRead` needs initialized memory, and one `want`-sized memset
    /// replaces `want / capacity` syscall-sized copies.
    fn read_bypassing_window(&mut self, want: usize) -> Result<Vec<u8>, EngineError> {
        let mut out = vec![0u8; want];
        let mut filled = self.drain_leftover_into(&mut out);
        while filled < want {
            let got = self.source.read(&mut out[filled..])?;
            if got == 0 {
                self.confirm_eof()?;
                break;
            }
            self.buf_pos += got as u64;
            filled += got;
        }
        if filled == want {
            // One capacity-bounded read-ahead refill: keeps the
            // header-after-payload pattern a window hit and preserves
            // the truncation snapshot semantics (≤ one capacity of
            // pre-fetched bytes). At exact end of stream it returns 0
            // and leaves the window empty — the truncation check
            // belongs to reads that still need data.
            self.refill_read_ahead()?;
        }
        out.truncate(filled);
        Ok(out)
    }

    /// Refill the (empty) window from the source — one capacity-sized
    /// read-ahead step, bounded by construction. Only called with an
    /// empty window, so the position invariant
    /// (`source.position() == buf_pos + buffered_bytes()`) is preserved.
    fn refill_read_ahead(&mut self) -> Result<(), EngineError> {
        self.start = 0;
        self.end = 0;
        let filled = self.source.read(&mut self.buf)?;
        self.end = filled;
        Ok(())
    }

    /// Move buffered bytes into `out`'s spare capacity, advancing the
    /// window. Never reallocates: the take is bounded by the room left.
    fn drain_into(&mut self, out: &mut Vec<u8>) {
        let room = out.capacity() - out.len();
        let take = room.min(self.end - self.start);
        if take > 0 {
            out.extend_from_slice(&self.buf[self.start..self.start + take]);
            self.start += take;
            self.buf_pos += take as u64;
        }
    }

    /// Read up to `out.len()` bytes at the current position into
    /// caller memory, advancing (card 3.B3 — the borrow twin of
    /// [`read`](BufferedReader::read)).
    ///
    /// Leftover read-ahead is served first; then the request splits by
    /// size (7-c part 2): an `out` **below** the window capacity goes
    /// through the fixed window (read-ahead locality for header-sized
    /// asks); an `out` **at or above** the capacity — the demux arena's
    /// full-frame payload class — is filled by asking the source
    /// **directly** for the remainder: zero allocation, zero
    /// intermediate copies, one full-size bulk transfer instead of
    /// `out.len() / capacity` window refills (plus one capacity-bounded
    /// read-ahead refill so the next header-sized ask stays a window
    /// hit). Short results are data
    /// (end of stream or a smaller ask than window); `Ok(0)` only at
    /// end of stream (a zero-length `out` is a zero-length success
    /// without I/O). Unlike [`read`] there is no [`MAX_READ_BYTES`]
    /// request ceiling: the caller owns the destination memory, so no
    /// allocation can spike.
    ///
    /// # Errors
    /// [`EngineError`] — kind `Corrupt` (`input.file.truncated`) when
    /// the source ends before its open-time length; the source's own
    /// read errors fold otherwise.
    pub fn read_into(&mut self, out: &mut [u8]) -> Result<usize, EngineError> {
        if out.is_empty() {
            return Ok(0);
        }
        let mut filled = self.drain_leftover_into(out);
        if out.len() >= self.buf.len() {
            // The bypass (7-c part 2): the destination is caller-owned
            // and already window-sized or larger — the source writes
            // straight into it.
            while filled < out.len() {
                let got = self.source.read(&mut out[filled..])?;
                if got == 0 {
                    self.confirm_eof()?;
                    break;
                }
                self.buf_pos += got as u64;
                filled += got;
            }
            if filled == out.len() {
                // The same capacity-bounded read-ahead refill as the
                // `Vec` twin: the next header-sized ask stays a window
                // hit instead of a fresh syscall.
                self.refill_read_ahead()?;
            }
            return Ok(filled);
        }
        while filled < out.len() {
            if self.start == self.end {
                self.start = 0;
                self.end = 0;
                let got = self.source.read(&mut self.buf)?;
                if got == 0 {
                    self.confirm_eof()?;
                    break;
                }
                self.end = got;
            }
            let take = (out.len() - filled).min(self.end - self.start);
            out[filled..filled + take].copy_from_slice(&self.buf[self.start..self.start + take]);
            self.start += take;
            self.buf_pos += take as u64;
            filled += take;
        }
        Ok(filled)
    }

    /// Move the front of the buffered window into `out` (at most
    /// `out.len()` bytes, at most what is buffered), advancing the
    /// window — the shared first step of both read paths. Returns the
    /// number of bytes taken.
    fn drain_leftover_into(&mut self, out: &mut [u8]) -> usize {
        let take = out.len().min(self.end - self.start);
        if take > 0 {
            out[..take].copy_from_slice(&self.buf[self.start..self.start + take]);
            self.start += take;
            self.buf_pos += take as u64;
        }
        take
    }

    /// Fill `out` completely or fail typed (card 3.B3): the exact-read
    /// discipline with the truncation check of [`BufferedReader::read`]
    /// — [`Ok(())`], or `Corrupt::Truncated` when the stream ends
    /// before `out.len()` bytes (expected = the position a complete
    /// read would reach, found = where the stream actually ends).
    ///
    /// # Errors
    /// [`EngineError`] — kind `Corrupt` (`input.file.truncated`) on a
    /// short stream; source read errors fold as in
    /// [`read_into`](BufferedReader::read_into).
    pub fn read_exact_into(&mut self, out: &mut [u8]) -> Result<(), EngineError> {
        let start = self.buf_pos;
        let mut filled = 0;
        while filled < out.len() {
            let got = self.read_into(&mut out[filled..])?;
            if got == 0 {
                return Err(error::truncated(start + out.len() as u64, self.buf_pos));
            }
            filled += got;
        }
        Ok(())
    }

    /// Copy up to `min(out.len(), buffered bytes)` from the front of
    /// the window into `out` **without advancing** (card 3.B3): pure
    /// window math — no I/O, no seek, no allocation. Answers 0 when
    /// the window is empty (the caller refills via
    /// [`read_into`](BufferedReader::read_into) first). Peeks never
    /// fabricate: they return exactly what is buffered, never more.
    pub fn peek_into(&mut self, out: &mut [u8]) -> Result<usize, EngineError> {
        let take = out.len().min(self.end - self.start);
        out[..take].copy_from_slice(&self.buf[self.start..self.start + take]);
        Ok(take)
    }

    /// The underlying source reported end-of-stream: either that is the
    /// truth (clean EOF) or the source shrank below the length this
    /// reader opened with — a typed truncation error, never a silent
    /// EOF (card 2.A1 AC).
    fn confirm_eof(&mut self) -> Result<(), EngineError> {
        if let (Some(declared), Some(live)) = (self.declared_len, self.source.length())
            && live < declared
        {
            return Err(error::truncated(declared, live));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::ScriptSource;
    use media_types::ErrorKind;
    use media_types::error::LimitKind;
    use testkit::Must;

    fn pattern(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i % 251) as u8).collect()
    }

    fn reader(data: &[u8], capacity: usize) -> BufferedReader<ScriptSource> {
        BufferedReader::new(ScriptSource::new(data), capacity).must()
    }

    /// Capacity validation: zero and over-ceiling are typed errors.
    #[test]
    fn capacity_bounds_are_typed_errors() {
        let data = pattern(10);
        let zero = testkit::assert_err(
            BufferedReader::new(ScriptSource::new(&data), 0),
            "zero capacity",
        );
        assert_eq!(zero.context.code, "input.buffer.capacity-zero");
        assert!(matches!(zero.kind, ErrorKind::Unsupported { .. }));

        let huge = testkit::assert_err(
            BufferedReader::new(ScriptSource::new(&data), MAX_BUFFER_BYTES + 1),
            "over-ceiling capacity",
        );
        assert_eq!(huge.context.code, "input.buffer.capacity-out-of-range");
        match huge.kind {
            ErrorKind::ResourceLimit {
                limit,
                budget,
                attempted,
            } => {
                assert_eq!(limit, LimitKind::Memory);
                assert_eq!(budget, MAX_BUFFER_BYTES as u64);
                assert_eq!(attempted, (MAX_BUFFER_BYTES + 1) as u64);
            }
            other => unreachable!("wrong kind: {other:?}"),
        }
    }

    /// A read request over the ceiling is a typed limit error; the
    /// ceiling itself is served.
    #[test]
    fn read_requests_over_the_ceiling_are_typed_limit_errors() {
        let mut r = reader(&pattern(8), 8);
        let err = testkit::assert_err(r.read(MAX_READ_BYTES + 1), "over-ceiling read");
        assert_eq!(err.context.code, "input.read.request-over-limit");
        match err.kind {
            ErrorKind::ResourceLimit {
                limit,
                budget,
                attempted,
            } => {
                assert_eq!(limit, LimitKind::Memory);
                assert_eq!(budget, MAX_READ_BYTES as u64);
                assert_eq!(attempted, (MAX_READ_BYTES + 1) as u64);
            }
            other => unreachable!("wrong kind: {other:?}"),
        }
        // At the ceiling the read proceeds (the source short-reads to
        // its 8 bytes, which is the contract's "fewer is normal").
        let data = pattern(8);
        let served = testkit::Must::must(r.read(MAX_READ_BYTES));
        assert_eq!(served, data);
    }

    /// Position tracking across reads, buffer-hit seeks and refills;
    /// leftover read-ahead survives small reads.
    #[test]
    fn positions_and_readahead_are_exact() {
        let data = pattern(3000);
        let mut r = reader(&data, 64);
        assert_eq!(r.position(), 0);

        let first = r.read(10).must();
        assert_eq!(&first, &data[..10]);
        assert_eq!(r.position(), 10);
        // One 64-byte refill serves the 10-byte request; the remaining
        // 54 bytes are read-ahead.
        assert_eq!(r.source().reads_served(), 1, "one refill");
        assert_eq!(r.source().max_read_seen(), 64, "chunks are capacity-sized");
        assert_eq!(r.buffered_bytes(), 54);

        // A small read inside the window: no new I/O.
        let second = r.read(10).must();
        assert_eq!(&second, &data[10..20]);
        assert_eq!(r.source().reads_served(), 1, "served from read-ahead");
        assert_eq!(r.position(), 20);

        // Seek forward, inside the unconsumed window [20, 64): no I/O.
        r.seek(30).must();
        assert_eq!(r.position(), 30);
        assert_eq!(r.source().reads_served(), 1, "buffer-hit seek does no I/O");
        let again = r.read(10).must();
        assert_eq!(&again, &data[30..40]);
        assert_eq!(r.source().reads_served(), 1, "served from read-ahead");

        // A zero-length read is an empty success and does no I/O.
        let empty = r.read(0).must();
        assert!(empty.is_empty());
        assert_eq!(r.source().reads_served(), 1);
    }

    /// Large reads take the 7-c part-2 **bypass**: one full-size
    /// source ask for the bulk (plus one capacity-bounded read-ahead
    /// refill) instead of capacity-sized window refills, byte-exact,
    /// and the fixed buffer neither grows nor is copied through.
    ///
    /// Supersedes the card-2.A1 pin "no chunk above capacity": V-1 made
    /// the demux arena ask for whole frames, and chunking a
    /// multi-hundred-KB ask through a 64 KiB window costs
    /// `want / capacity` syscalls plus a window copy per chunk. The
    /// bounded-memory proof moves with it — the reader never asks the
    /// source for more than the caller's own request, and the window
    /// stays fixed (both asserted below).
    #[test]
    fn large_reads_bypass_the_window_in_one_source_call() {
        let data = pattern(1000);
        let mut r = reader(&data, 32);
        let got = r.read(1000).must();
        assert_eq!(got, data);
        assert_eq!(
            r.source().reads_served(),
            2,
            "one full-size bulk ask + one capacity-bounded read-ahead refill — not 32"
        );
        assert_eq!(
            r.source().max_read_seen(),
            1000,
            "the bulk ask is the caller's request — never a speculative over-read"
        );
        assert!(r.buffered_bytes() <= 32);
        assert_eq!(r.buffer_capacity(), 32, "the buffer never grows");
        assert_eq!(r.position(), 1000);
    }

    /// The bypass serves leftover read-ahead first: bytes already in
    /// the window are never re-fetched from the source, and the direct
    /// ask is only for the remainder.
    #[test]
    fn bypass_reads_serve_leftover_read_ahead_first() {
        let data = pattern(200);
        let mut r = reader(&data, 64);
        // Prime the window: one 64-byte refill, 54 bytes of leftover.
        let head = r.read(10).must();
        assert_eq!(&head, &data[..10]);
        assert_eq!(r.source().reads_served(), 1);

        // A payload-class ask: the 54 leftover bytes drain from the
        // window, then ONE direct ask covers the remaining 136 (the
        // trailing read-ahead refill hits exact EOF and returns 0).
        let payload = r.read(190).must();
        assert_eq!(&payload, &data[10..200]);
        assert_eq!(
            r.source().reads_served(),
            3,
            "prime + bulk + read-ahead refill (EOF)"
        );
        assert_eq!(r.source().max_read_seen(), 136, "the remainder, exactly");
        assert_eq!(r.position(), 200);
    }

    /// A source that serves short-but-nonzero reads is re-asked for
    /// the remainder — the bypass loop makes strict progress and stays
    /// byte-exact (the multi-syscall honest path when the kernel or a
    /// remote source dribbles).
    #[test]
    fn bypass_reads_survive_short_source_reads() {
        /// A source that never serves more than 40 bytes per call —
        /// the dribble shape of a slow pipe, pinned at unit level.
        struct Dribble {
            data: Vec<u8>,
            pos: usize,
            calls: usize,
            max_ask: usize,
        }
        impl Seekable for Dribble {
            fn position(&self) -> u64 {
                self.pos as u64
            }
            fn length(&self) -> Option<u64> {
                Some(self.data.len() as u64)
            }
            fn seek(&mut self, to: u64) -> Result<u64, EngineError> {
                self.pos = to as usize;
                Ok(to)
            }
        }
        impl RawRead for Dribble {
            fn read(&mut self, out: &mut [u8]) -> Result<usize, EngineError> {
                self.calls += 1;
                self.max_ask = self.max_ask.max(out.len());
                let n = out.len().min(40).min(self.data.len() - self.pos);
                out[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
                self.pos += n;
                Ok(n)
            }
        }

        let data = pattern(300);
        let src = Dribble {
            data: data.clone(),
            pos: 0,
            calls: 0,
            max_ask: 0,
        };
        let mut r = BufferedReader::new(src, 16).must();
        let mut out = vec![0u8; 300];
        let got = r.read_into(&mut out).must();
        assert_eq!(got, 300);
        assert_eq!(&out, &data, "byte-exact across dribbled refills");
        assert_eq!(r.source().calls, 9, "8 dribbled bulk calls + 1 refill");
        assert_eq!(r.source().max_ask, 300, "asks stay caller-sized");
    }

    /// A payload-class read past the end of a short stream is a clean
    /// short result (end of stream is data), and the window stays put.
    #[test]
    fn bypass_read_past_end_short_reads_cleanly() {
        let data = pattern(100);
        let mut r = reader(&data, 16);
        let got = r.read(1000).must();
        assert_eq!(got, data, "the whole file, short of the ask");
        assert_eq!(r.position(), 100);
        assert!(r.read(64).must().is_empty(), "sticky EOF");
    }

    /// End-of-stream at the open-time length is clean and repeatable.
    #[test]
    fn clean_eof_at_declared_length() {
        let data = pattern(100);
        let mut r = reader(&data, 16);
        let all = r.read(100).must();
        assert_eq!(all, data);
        for _ in 0..3 {
            let tail = r.read(8).must();
            assert!(tail.is_empty(), "sticky clean EOF");
        }
        assert_eq!(r.position(), 100);

        // Seek to exactly the length: legal, clean EOF again.
        r.seek(100).must();
        assert!(r.read(4).must().is_empty());

        // One past: typed out-of-range.
        let err = testkit::assert_err(r.seek(101), "seek past end");
        assert_eq!(err.context.code, "input.seek.out-of-range");
        assert_eq!(r.position(), 100, "failed seek never moves");
    }

    /// A shrink underneath the reader is the typed truncation error
    /// with (expected, found) = (open-time, shrunk).
    #[test]
    fn shrink_mid_read_surfaces_typed_truncation() {
        let data = pattern(600);
        let mut r = reader(&data, 16);
        let head = r.read(200).must();
        assert_eq!(&head, &data[..200]);
        r.source_mut().shrink_to(300);
        let err = testkit::assert_err(r.read(400), "truncated read");
        assert_eq!(err.context.code, "input.file.truncated");
        match err.kind {
            ErrorKind::Corrupt {
                detail: media_types::error::CorruptDetail::Truncated { expected, found },
            } => {
                assert_eq!(expected, 600);
                assert_eq!(found, 300);
            }
            other => unreachable!("wrong kind: {other:?}"),
        }
    }

    /// Sources opened at a non-zero position keep the invariant.
    #[test]
    fn non_zero_start_positions_are_respected() {
        let data = pattern(100);
        let mut source = ScriptSource::new(&data);
        source.seek(40).must();
        let mut r = BufferedReader::new(source, 16).must();
        assert_eq!(r.position(), 40);
        let got = r.read(10).must();
        assert_eq!(&got, &data[40..50]);
    }

    /// The borrow twin reconstructs byte-exactly, and a payload-class
    /// borrow (the demux arena's full-frame ask, 7-c part 2) is ONE
    /// full-size source call — no window refills, no intermediate
    /// copies — while the fixed window never grows.
    #[test]
    fn read_into_reconstructs_and_bypasses_for_payload_asks() {
        let data = pattern(1000);
        let mut r = reader(&data, 32);
        let mut out = vec![0u8; 1000];
        let got = r.read_into(&mut out).must();
        assert_eq!(got, 1000);
        assert_eq!(&out, &data);
        assert_eq!(r.position(), 1000);
        assert_eq!(
            r.source().reads_served(),
            2,
            "one bulk call + one read-ahead refill"
        );
        assert_eq!(r.source().max_read_seen(), 1000, "caller-sized asks");
        assert!(r.buffered_bytes() <= 32);
        assert_eq!(r.buffer_capacity(), 32, "the buffer never grows");

        // End of stream is a clean zero, repeatable.
        for _ in 0..2 {
            let n = r.read_into(&mut out).must();
            assert_eq!(n, 0, "Ok(0) only at end of stream");
        }
        // A zero-length ask is a zero-length success without I/O.
        let served = r.source().reads_served();
        assert_eq!(r.read_into(&mut []).must(), 0);
        assert_eq!(r.source().reads_served(), served);
    }

    /// Partial borrows: a small `out` takes from the read-ahead and
    /// leaves the rest for the next call (the same locality the `Vec`
    /// path gives, minus the per-call allocation).
    #[test]
    fn read_into_serves_small_asks_from_read_ahead() {
        let data = pattern(300);
        let mut r = reader(&data, 64);
        let mut chunk = [0u8; 10];
        assert_eq!(r.read_into(&mut chunk).must(), 10);
        assert_eq!(&chunk, &data[..10]);
        assert_eq!(r.source().reads_served(), 1, "one refill");
        assert_eq!(r.read_into(&mut chunk).must(), 10);
        assert_eq!(&chunk, &data[10..20]);
        assert_eq!(
            r.source().reads_served(),
            1,
            "the second ask is served from read-ahead"
        );
        assert_eq!(r.position(), 20);
    }

    /// The exact-read twin: complete fills are silent successes; a
    /// stream that ends early is the typed truncation with the ask's
    /// arithmetic; a shrink mid-read keeps the typed (open-time,
    /// shrunk) pair.
    #[test]
    fn read_exact_into_fills_or_fails_typed() {
        let data = pattern(200);
        let mut r = reader(&data, 16);
        let mut out = [0u8; 50];
        r.read_exact_into(&mut out).must();
        assert_eq!(&out, &data[..50]);
        assert_eq!(r.position(), 50);

        // Runs off the end: typed truncation, expected = 50 + 200,
        // found = the stream end (200).
        let mut rest = [0u8; 200];
        let err = testkit::assert_err(r.read_exact_into(&mut rest), "past-the-end exact read");
        assert_eq!(err.context.code, "input.file.truncated");
        let (expected, found) = match err.kind {
            ErrorKind::Corrupt {
                detail: media_types::error::CorruptDetail::Truncated { expected, found },
            } => (expected, found),
            other => unreachable!("wrong kind: {other:?}"),
        };
        assert_eq!(expected, 250);
        assert_eq!(found, 200);

        // A shrink mid-read keeps the (open-time, shrunk) pair.
        let mut r = reader(&data, 16);
        let mut head = [0u8; 40];
        r.read_exact_into(&mut head).must();
        r.source_mut().shrink_to(100);
        let mut rest = [0u8; 100];
        let err = testkit::assert_err(r.read_exact_into(&mut rest), "truncated exact read");
        assert_eq!(err.context.code, "input.file.truncated");
    }

    /// The peek twin: window-bound honesty, no advancement, no I/O.
    #[test]
    fn peek_into_never_advances_and_never_fabricates() {
        let data = pattern(500);
        let mut r = reader(&data, 128);
        let mut probe = [0u8; 16];
        assert_eq!(r.peek_into(&mut probe).must(), 0, "empty window peeks 0");

        // Fill the window via a small read, then peek repeatedly.
        let mut take = [0u8; 8];
        r.read_into(&mut take).must();
        let served = r.source().reads_served();
        for _ in 0..3 {
            let n = r.peek_into(&mut probe).must();
            assert_eq!(n, 16, "the window holds 120 bytes — 16 are peekable");
            assert_eq!(&probe[..n], &data[8..24]);
        }
        assert_eq!(r.position(), 8, "peeks never advance");
        assert_eq!(r.source().reads_served(), served, "peeks do no I/O");

        // A peek larger than the window returns exactly the window.
        let mut big = [0u8; 4096];
        let n = r.peek_into(&mut big).must();
        assert_eq!(n, 120, "never more than what is buffered");
        assert_eq!(&big[..n], &data[8..128]);
        assert_eq!(r.position(), 8, "still no advancement");
    }
}
