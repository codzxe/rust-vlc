//! The positional-source surface (card 2.A1): position, length, typed
//! seeks, and the raw chunk primitive the bounded reader is generic
//! over.
//!
//! [`Seekable`] is the card's seek contract: a value and its absence are
//! different types (`length()` is `Option`, MP §8), positions are
//! absolute, and seek failures are typed
//! [`EngineError`](media_types::EngineError)s — never sentinel returns.
//! [`RawRead`] is the deliberately small read primitive: chunked reads
//! into caller memory, `Ok(0)` only at the underlying end of stream.
//!
//! # Seek semantics (shared by every implementation)
//!
//! - Seeking to exactly `length()` is legal — the EOF position; the
//!   next read returns a clean end-of-stream.
//! - Seeking **beyond** the current length is a typed error
//!   (`input.seek.out-of-range`): a demuxer that asks for a byte that
//!   does not exist is a caller bug, reported truthfully (MP §8), not a
//!   POSIX-style silent past-end cursor.
//! - Sources whose length is unknown (`None`) accept any target; the
//!   underlying store is the judge.
//!
//! # Threading
//!
//! Called from the source worker thread (doc 01 §4.1); every method
//! performs bounded work per call. The traits are data-owned: one
//! stream per open source, moved between threads wholesale (`Send`
//! comes from the concrete source, not a supertrait — mirrors
//! `plugin_api::SourceStream`).

use media_types::EngineError;
use plugin_api::SeekOrigin;

/// A byte source whose position and length are observable and whose
/// seeks fail with typed errors (card 2.A1: position/length; seek
/// semantics with typed errors).
pub trait Seekable {
    /// The absolute byte offset of the next byte a read would return.
    fn position(&self) -> u64;

    /// The total length when knowable; `None` for sources with no
    /// observable end (a value and its absence are different types,
    /// MP §8).
    fn length(&self) -> Option<u64>;

    /// Reposition to the absolute offset `to`, returning the position
    /// actually set.
    ///
    /// # Errors
    /// [`EngineError`] (stage `Open`) — `input.seek.out-of-range` when
    /// `to` exceeds the current length of a bounded source, or the
    /// source's own I/O fold otherwise.
    fn seek(&mut self, to: u64) -> Result<u64, EngineError>;
}

/// The raw chunked-read primitive the [`BufferedReader`](crate::BufferedReader)
/// wraps: read up to `out.len()` bytes at the current position.
///
/// # Contract
///
/// - `Ok(n)` with `0 < n <= out.len()`: `n` bytes were stored and the
///   position advanced by `n`. Short reads are data, not errors.
/// - `Ok(0)`: the underlying source is at its end **for now**.
/// - `Err`: a typed error; the position is unchanged.
pub trait RawRead {
    /// Read up to `out.len()` bytes into `out` at the current position.
    ///
    /// # Errors
    /// [`EngineError`] — the source's I/O fold (see
    /// [`crate::error`]).
    fn read(&mut self, out: &mut [u8]) -> Result<usize, EngineError>;
}

/// Resolve a [`SeekOrigin`] against `from` into an absolute target,
/// rejecting before-start and overflowing positions with typed errors
/// (the module's shared seek semantics, in the
/// `SourceStream::seek` shape — used by the file access stack and
/// the synthetic conformance source alike; audit F-2 moved it here
/// from the test-side contract module so the module can be
/// feature-gated without touching production).
///
/// # Errors
/// [`EngineError`] (stage `Open`) — `input.seek.before-start` when
/// the resolved target is negative, `input.seek.overflow` when it
/// exceeds `u64::MAX`.
pub(crate) fn resolve_origin(from: u64, origin: SeekOrigin) -> Result<u64, EngineError> {
    match origin {
        SeekOrigin::Start(to) => Ok(to),
        SeekOrigin::Current(delta) => {
            let target = i128::from(from) + i128::from(delta);
            if target < 0 {
                Err(crate::error::seek_before_start())
            } else if target > i128::from(u64::MAX) {
                Err(crate::error::seek_overflow())
            } else {
                Ok(target as u64)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    /// Relative-origin resolution: negatives, overflow, exact bounds
    /// (pinned with the helper in its new home, audit F-2 — the test
    /// ran in the contract module before the module was gated).
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
