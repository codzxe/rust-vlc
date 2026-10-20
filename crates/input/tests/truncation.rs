//! Truncation acceptance tests (card 2.A1 AC): a file that shrinks
//! below its open-time length mid-read surfaces the **typed**
//! `Corrupt::Truncated` error — never a panic, never a silent EOF.
//!
//! The complement lives in `file_stream.rs`: a file that *grows* past
//! its open-time length keeps reading (no false truncation). Here the
//! fault is real: `set_len` on the live file underneath an open stream,
//! at several positions relative to the reader cursor and the buffer
//! window.

mod common;

use common::{TempFile, pattern};
use input::FileAccess;
use input::contract::assert_truncated;
use media_types::ids::SourceUri;
use media_types::{EngineError, ErrorKind};
use plugin_api::{Access, SourceStream};
use testkit::Must;

/// Open a stream over `temp` with a `capacity`-byte read-ahead window
/// (a small window forces multiple refills, so truncations land mid-
/// transfer, not only on the first chunk).
fn open_with_capacity(temp: &TempFile, capacity: usize) -> Box<dyn SourceStream> {
    let access = FileAccess::with_buffer_capacity(capacity).must();
    let uri = SourceUri::parse(&temp.file_uri()).must();
    access.open(&uri).must()
}

/// Assert an error is the typed truncation error with (expected, found)
/// = (open-time length, shrunk length) and the input code.
#[track_caller]
fn assert_truncation(err: &EngineError) {
    assert_eq!(err.context.code, "input.file.truncated", "{err}");
    let (expected, found) = assert_truncated(err);
    assert_eq!((expected, found), (1000, 300), "{err}");
    assert_eq!(
        err.to_string(),
        "[open] truncated data: expected 1000 bytes, found 300 (input.file.truncated)",
        "structured Display, no secrets"
    );
}

/// The core AC: file truncated mid-read (partially consumed, more than
/// one buffer window from the end) → typed `Corrupt::Truncated`.
#[test]
fn file_truncated_mid_read_surfaces_the_typed_error() {
    let data = pattern(1000);
    let temp = TempFile::create("trunc-mid", &data).must();
    let mut stream = open_with_capacity(&temp, 16);

    let head = stream.read(200).must();
    assert_eq!(&head, &data[..200]);

    temp.truncate_to(300).must();
    let err = testkit::assert_err(stream.read(400), "read across the truncation");
    assert_truncation(&err);
    // `size` reports the live truth; the open-time reference lives in
    // the error, not in a stale size.
    assert_eq!(stream.size(), Some(300));
}

/// Truncated to *below* the reader's cursor: the bounded read-ahead
/// already in the window is served first (snapshot semantics, like an
/// OS page cache — at most one capacity of pre-truncation bytes, and
/// byte-identical to what was read before the shrink), and the moment
/// the window empties the typed truncation error fires — never a
/// clean EOF, never a panic.
#[test]
fn truncation_below_the_cursor_drains_the_snapshot_then_errors_typed() {
    let data = pattern(1000);
    let temp = TempFile::create("trunc-below", &data).must();
    let mut stream = open_with_capacity(&temp, 64);

    let head = stream.read(600).must();
    assert_eq!(&head, &data[..600]);

    temp.truncate_to(300).must();

    // Drain the stale snapshot: every read serves pre-truncation bytes
    // until the window empties, then the typed error surfaces.
    let mut stale = Vec::new();
    let err = loop {
        match stream.read(8) {
            Ok(piece) => {
                // A clean empty here would be the silent-EOF failure
                // mode the card forbids: fail loudly instead.
                assert!(!piece.is_empty(), "a shrunken source never EOFs cleanly");
                stale.extend_from_slice(&piece);
            }
            Err(err) => break err,
        }
    };
    assert!(
        !stale.is_empty() && stale.len() <= 64,
        "the stale snapshot is bounded by one window: {}",
        stale.len()
    );
    assert_eq!(&stale, &data[600..600 + stale.len()], "snapshot bytes");
    assert_eq!(stream.position().0, (600 + stale.len()) as u64);

    assert_eq!(err.context.code, "input.file.truncated");
    let (expected, found) = assert_truncated(&err);
    assert_eq!((expected, found), (1000, 300));
    // The error is sticky: the source is still short of its open-time
    // length, so a retry errors again rather than flipping to EOF.
    let retry = testkit::assert_err(stream.read(8), "retry after the truncation");
    assert_eq!(retry.context.code, "input.file.truncated");
    assert_eq!(assert_truncated(&retry), (1000, 300));
}

/// Truncated to zero: the degenerate shrink is still typed (a live
/// length of `Some(0)` is a truth, not an absence).
#[test]
fn truncation_to_zero_is_typed() {
    let data = pattern(500);
    let temp = TempFile::create("trunc-zero", &data).must();
    let mut stream = open_with_capacity(&temp, 32);

    let head = stream.read(100).must();
    assert_eq!(&head, &data[..100]);

    temp.truncate_to(0).must();
    let err = testkit::assert_err(stream.read(64), "read after truncation to zero");
    assert_eq!(err.context.code, "input.file.truncated");
    let (expected, found) = assert_truncated(&err);
    assert_eq!((expected, found), (500, 0));
}

/// The fault lands mid-refill: buffered bytes drain, the next refill
/// comes up short, and the EOF check fires the typed error (with the
/// truncation happening *after* one full window was already consumed).
#[test]
fn truncation_lands_mid_refill() {
    let data = pattern(1000);
    let temp = TempFile::create("trunc-refill", &data).must();
    let mut stream = open_with_capacity(&temp, 64);

    // Consume exactly 100: one full window plus part of a second, so
    // read-ahead remains in the buffer when the fault lands.
    let head = stream.read(100).must();
    assert_eq!(&head, &data[..100]);

    temp.truncate_to(150).must();
    let err = testkit::assert_err(stream.read(800), "multi-refill read across the fault");
    assert_eq!(err.context.code, "input.file.truncated");
    let (expected, found) = assert_truncated(&err);
    assert_eq!((expected, found), (1000, 150));
}

/// After a truncation, the surviving region is still readable: seeks
/// within the live length serve real bytes; only crossing the end
/// against the open-time expectation errors. The error is about the
/// missing tail, not about the shrink itself.
#[test]
fn surviving_region_reads_then_crossing_errors_typed() {
    let data = pattern(1000);
    let temp = TempFile::create("trunc-survive", &data).must();
    let mut stream = open_with_capacity(&temp, 64);

    let head = stream.read(100).must();
    assert_eq!(&head, &data[..100]);
    temp.truncate_to(300).must();

    // Seek within the live length: served with the surviving bytes.
    let at = stream.seek(plugin_api::SeekOrigin::Start(250)).must();
    assert_eq!(at.0, 250);
    let piece = stream.read(40).must();
    assert_eq!(&piece, &data[250..290]);

    // A read that would cross the end (290 + 20 > 300): the typed
    // truncation error — the reader cannot hand back 20 honest bytes.
    let err = testkit::assert_err(stream.read(20), "read crossing the truncated end");
    assert_eq!(err.context.code, "input.file.truncated");
    let (expected, found) = assert_truncated(&err);
    assert_eq!((expected, found), (1000, 300));

    // A read stopping exactly at the live end is still good data.
    let at = stream.seek(plugin_api::SeekOrigin::Start(290)).must();
    assert_eq!(at.0, 290);
    let last = stream.read(10).must();
    assert_eq!(&last, &data[290..300]);
}

/// A seek beyond the *live* (shrunk) length is the typed out-of-range
/// error — the truncation reference does not loosen seek bounds.
#[test]
fn seek_beyond_the_shrunk_length_is_typed_out_of_range() {
    let data = pattern(1000);
    let temp = TempFile::create("trunc-seek", &data).must();
    let mut stream = open_with_capacity(&temp, 64);

    let head = stream.read(50).must();
    assert_eq!(&head, &data[..50]);
    temp.truncate_to(300).must();

    let before = stream.position();
    let err = testkit::assert_err(
        stream.seek(plugin_api::SeekOrigin::Start(301)),
        "seek past the shrunk end",
    );
    assert_eq!(err.context.code, "input.seek.out-of-range");
    assert!(matches!(err.kind, ErrorKind::Inaccessible { .. }), "{err}");
    assert_eq!(stream.position(), before, "a failed seek never moves");
}
