//! Bounded-buffering acceptance tests (card 2.A1 AC): reads stay
//! byte-exact through a fixed, capacity-bounded window over a **real**
//! file — requests far larger than the window, read sizes straddling
//! the window boundary, and the degenerate capacity of one byte.
//!
//! The reader's memory is the fixed window plus one bounded result: the
//! window is allocated once and never grows (asserted below on the real
//! file), and the OS-facing chunk bound — the source never sees a read
//! larger than the capacity — is proven with instrumentation on the
//! synthetic source in the unit suite and `contract::assert_bounded_read`.

mod common;

use common::{TempFile, pattern};
use input::{BufferedReader, FileAccess, FileSource, MAX_BUFFER_BYTES};
use media_types::ids::SourceUri;
use media_types::{ErrorKind, PipelineStage};
use plugin_api::Access;
use testkit::Must;

/// Capacity validation at the `Access` surface: zero and over-ceiling
/// are typed errors; legal values (including the degenerate 1 and the
/// ceiling itself) construct and report their window.
#[test]
fn access_capacity_bounds_are_typed() {
    let zero = testkit::assert_err(FileAccess::with_buffer_capacity(0), "zero capacity");
    assert_eq!(zero.context.code, "input.buffer.capacity-zero");
    assert!(matches!(zero.kind, ErrorKind::Unsupported { .. }), "{zero}");
    assert_eq!(zero.stage, PipelineStage::Open);

    let over = testkit::assert_err(
        FileAccess::with_buffer_capacity(MAX_BUFFER_BYTES + 1),
        "over-ceiling capacity",
    );
    assert_eq!(over.context.code, "input.buffer.capacity-out-of-range");
    match over.kind {
        ErrorKind::ResourceLimit {
            limit,
            budget,
            attempted,
        } => {
            assert_eq!(limit, media_types::error::LimitKind::Memory);
            assert_eq!(budget, MAX_BUFFER_BYTES as u64);
            assert_eq!(attempted, (MAX_BUFFER_BYTES + 1) as u64);
        }
        other => unreachable!("wrong kind: {other:?}"),
    }

    for legal in [1usize, 64, MAX_BUFFER_BYTES] {
        let access = FileAccess::with_buffer_capacity(legal).must();
        assert_eq!(access.buffer_capacity(), legal);
    }
    assert_eq!(
        FileAccess::new().buffer_capacity(),
        input::DEFAULT_BUFFER_BYTES,
        "the default is the 64 KiB doc-03 window"
    );
}

/// Read sizes straddling the window boundary (capacity 64): below, at,
/// above, and far above — the reconstruction stays byte-exact and the
/// position tracks consumption at every step.
#[test]
fn reads_straddle_the_window_boundary_byte_exactly() {
    let data = pattern(5000);
    let temp = TempFile::create("bounds-matrix", &data).must();
    let access = FileAccess::with_buffer_capacity(64).must();
    let uri = SourceUri::parse(&temp.file_uri()).must();
    let mut stream = access.open(&uri).must();

    let mut got = Vec::new();
    'outer: for chunk in [1usize, 31, 32, 63, 64, 65, 127, 128, 129, 1000] {
        loop {
            let piece = stream.read(chunk).must();
            if piece.is_empty() {
                continue 'outer;
            }
            assert_eq!(stream.position().0, (got.len() + piece.len()) as u64);
            got.extend_from_slice(&piece);
            if piece.len() < chunk {
                continue 'outer;
            }
        }
    }
    assert_eq!(got, data, "byte-exact across every boundary straddle");
    // A past-end read is a clean empty (nothing was truncated).
    assert!(stream.read(4096).must().is_empty());
}

/// The degenerate capacity of one byte: every byte crosses the window
/// boundary; the data still reconstructs exactly.
#[test]
fn capacity_one_reads_byte_exactly() {
    let data = pattern(300);
    let temp = TempFile::create("bounds-cap1", &data).must();
    let access = FileAccess::with_buffer_capacity(1).must();
    let uri = SourceUri::parse(&temp.file_uri()).must();
    let mut stream = access.open(&uri).must();

    let mut got = Vec::new();
    loop {
        let piece = stream.read(100).must();
        if piece.is_empty() {
            break;
        }
        got.extend_from_slice(&piece);
    }
    assert_eq!(got, data);
    assert_eq!(stream.position().0, 300);
}

/// Over the real `FileSource`, through `BufferedReader` directly: a
/// read many times the window returns byte-exact data while the window
/// never grows past its construction size.
#[test]
fn the_fixed_window_never_grows_over_a_real_file() {
    let data = pattern(2000);
    let temp = TempFile::create("bounds-window", &data).must();
    let source = FileSource::open(&temp.path).must();
    let mut reader = BufferedReader::new(source, 32).must();
    assert_eq!(reader.buffer_capacity(), 32);
    assert_eq!(reader.source_length(), Some(2000));

    let got = reader.read(2000).must();
    assert_eq!(got, data, "a 2000-byte request through a 32-byte window");
    assert_eq!(reader.buffer_capacity(), 32, "the buffer never grows");
    assert!(reader.buffered_bytes() <= 32);
    assert_eq!(reader.position(), 2000);
}

/// Seek bounds and positions over the real file through the reader:
/// window-hit seeks, the exact window end, backward seeks that drop the
/// window, seek-to-length as clean EOF, and the typed past-end error
/// that never moves the cursor.
#[test]
fn seek_positions_and_bounds_over_a_real_file() {
    let data = pattern(500);
    let temp = TempFile::create("bounds-seek", &data).must();
    let source = FileSource::open(&temp.path).must();
    let mut reader = BufferedReader::new(source, 64).must();

    // Consume 100 of a 64-byte window's refill chain: read-ahead for
    // [100, 128) remains buffered.
    let head = reader.read(100).must();
    assert_eq!(&head, &data[..100]);

    // Backward into the consumed region: the window drops, data flows.
    reader.seek(10).must();
    assert_eq!(reader.position(), 10);
    let back = reader.read(20).must();
    assert_eq!(&back, &data[10..30]);

    // Forward into the unconsumed window: index math only.
    reader.seek(120).must();
    assert_eq!(reader.position(), 120);
    let fwd = reader.read(8).must();
    assert_eq!(&fwd, &data[120..128]);

    // Seek-to-length is legal and reads a clean EOF.
    reader.seek(500).must();
    assert_eq!(reader.position(), 500);
    assert!(reader.read(4).must().is_empty());

    // One past the length: typed error, cursor unmoved.
    let err = testkit::assert_err(reader.seek(501), "seek past the end");
    assert_eq!(err.context.code, "input.seek.out-of-range");
    assert_eq!(reader.position(), 500, "a failed seek never moves");

    // A legal seek still works afterwards.
    reader.seek(0).must();
    let first = reader.read(16).must();
    assert_eq!(&first, &data[..16]);
}
