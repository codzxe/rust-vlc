//! FX1-A04 (FIX-PLAN §2 F0.4c+d) — the two MKV regression pins: the
//! lacing count semantics and the window-slide livelock exit.
//!
//! # Lacing (F0.4c)
//!
//! The Block lacing count byte is **frames − 1** (Matroska §6.3.6 —
//! `laces = *data++ + 1` in ffmpeg, this repo's own designated
//! oracle). The audit executed the inversion: the byte was read as
//! the literal frame count, so ffmpeg-legal laces misparsed silently
//! (2 packets instead of 3, damage 0 — frame-boundary corruption on
//! every laced block; mkvmerge laces audio by default). The pin below
//! drives all three codings through [`parse_block`] plus the
//! degenerate one-frame lace (count byte 0), on which the codings
//! must agree.
//!
//! # Window slide (F0.4d)
//!
//! `next_block_packets` slides its bounded window two ways, and both
//! could re-issue an identical read forever: a block whose span is
//! wider than [`MAX_CLUSTER_WINDOW`] but inside the 32 MiB block gate
//! (the audit's executed livelock — watchdog kill, exit 124), and a
//! stream that ends inside the cluster's declared extent (the stall).
//! Both must terminate with the module's typed `demux.mkv.truncated`.
//! The walks run on a watchdog thread (the packet-queue suites'
//! pattern): a regression hangs the walk inside one `next_event`
//! call, and the timeout fails the test crisply instead of stalling
//! the whole run.

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use demux::avi::test_support::BytesStream;
use demux::avi::test_support::mem_stream;
use demux::mkv::cluster::{Lacing, parse_block};
use demux::mkv::ebml;
use demux::mkv::{MAX_CLUSTER_WINDOW, MkvDemuxer};
use media_types::EngineError;
use media_types::ids::SourceUri;
use plugin_api::{DemuxEvent, Demuxer, SeekOrigin, SourcePosition, SourceStream};
use testkit::Must;

/// The watchdog budget: a legal walk of these ≤ 5 MiB files takes
/// milliseconds; the livelock takes forever.
const WATCHDOG: Duration = Duration::from_secs(30);

// ---------------------------------------------------------------------------
// The lacing pin (F0.4c)
// ---------------------------------------------------------------------------

/// A SimpleBlock payload: track vint (≤ 126), i16 relative timecode,
/// flags byte, then the laced body (the `cluster.rs` unit-test
/// convention).
fn block(track: u8, rel: i16, flags: u8, body: &[u8]) -> Vec<u8> {
    let mut v = vec![0x80 | track];
    v.extend_from_slice(&rel.to_be_bytes());
    v.push(flags);
    v.extend_from_slice(body);
    v
}

/// The count byte is frames − 1 (spec §6.3.6): 2 → 3 frames for all
/// three codings, and 0 → the degenerate one-frame lace on which the
/// codings agree. Under the pre-fix inversion the same bodies yielded
/// one frame fewer (or were refused), mis-splitting every real laced
/// block.
#[test]
fn fx1_a04_lacing_count_byte_is_frames_minus_one() {
    // Xiph: count byte 2 → THREE frames — two coded sizes (5 and 300,
    // the latter in 0xFF,0x2D continuation form), the last frame is
    // the payload remainder.
    let mut xiph = vec![2u8, 5, 0xFF, 0x2D];
    xiph.extend_from_slice(&[0xA1; 5]);
    xiph.extend_from_slice(&[0xB1; 300]);
    xiph.extend_from_slice(&[0xC1; 160]);
    let data = block(1, 0, 0x02, &xiph); // 0b10 >> 1 = Xiph
    let parsed = parse_block(&data, 0, true).must();
    assert_eq!(parsed.lacing, Lacing::Xiph);
    assert_eq!(parsed.frames.len(), 3, "count byte 2 = frames - 1");
    assert_eq!(
        (
            parsed.frames[0].len,
            parsed.frames[1].len,
            parsed.frames[2].len
        ),
        (5, 300, 160)
    );
    // Two coded sizes consumed after the count byte: frame 0 starts at
    // 1 (count) + 1 (size 5) + 2 (continuation) = 4 (one fewer size
    // read under the inversion — start 2).
    assert_eq!(
        (
            parsed.frames[0].start,
            parsed.frames[1].start,
            parsed.frames[2].start
        ),
        (4, 9, 309)
    );
    let mid = &data[4 + parsed.frames[1].start..4 + parsed.frames[1].start + 300];
    assert!(
        mid.iter().all(|&b| b == 0xB1),
        "frame boundaries byte-exact"
    );

    // Fixed: count byte 3 → FOUR equal 2-byte frames over 8 payload
    // bytes (the pre-fix code read 3 literally and refused the body:
    // 8 % 3 != 0).
    let fixed = [3u8, 1, 2, 3, 4, 5, 6, 7, 8];
    let data = block(2, 10, 0x04, &fixed); // 0b100 >> 1 = Fixed
    let parsed = parse_block(&data, 0, true).must();
    assert_eq!(parsed.lacing, Lacing::Fixed);
    assert_eq!(parsed.frames.len(), 4, "count byte 3 = frames - 1");
    assert!(parsed.frames.iter().all(|f| f.len == 2));

    // EBML: count byte 2 → THREE frames — first size vint 0x82 (value
    // 2 → size 3), the equal-size delta 0 encodes as value 63 = bias
    // 2^6−1 → 0xBF, the last frame is the remainder.
    let mut ebml = vec![2u8, 0x82, 0xBF];
    ebml.extend_from_slice(&[0xAA; 3]);
    ebml.extend_from_slice(&[0xBB; 3]);
    ebml.extend_from_slice(&[0xCC; 2]);
    let data = block(1, 0, 0x06, &ebml); // 0b110 >> 1 = EBML
    let parsed = parse_block(&data, 0, true).must();
    assert_eq!(parsed.lacing, Lacing::Ebml);
    assert_eq!(parsed.frames.len(), 3, "count byte 2 = frames - 1");
    assert_eq!(
        (
            parsed.frames[0].len,
            parsed.frames[1].len,
            parsed.frames[2].len
        ),
        (3, 3, 2)
    );

    // The degenerate one-frame lace (count byte 0): all three codings
    // agree — one frame, the whole body past the count byte.
    for flags in [0x02u8, 0x04, 0x06] {
        let data = block(1, 0, flags, &[0x00, 0xA7, 0xB9]);
        let parsed = parse_block(&data, 0, true).must();
        assert_eq!(
            parsed.frames.len(),
            1,
            "flags {flags:#04x}: count byte 0 = the one-frame lace"
        );
        assert_eq!(parsed.frames[0].start, 1);
        assert_eq!(parsed.frames[0].len, 2);
    }
}

// ---------------------------------------------------------------------------
// EBML byte-builder helpers (the mkv_extras_wiring.rs vocabulary)
// ---------------------------------------------------------------------------

/// The minimal big-endian bytes of an element id (marker form).
fn id_bytes(id: u32) -> Vec<u8> {
    let be = id.to_be_bytes();
    let first = be.iter().position(|&b| b != 0).unwrap_or(3);
    be[first..].to_vec()
}

/// The EBML size vint for `n` (shortest encoding, marker bit set).
fn size_vint(n: u64) -> Vec<u8> {
    for len in 1usize..=8 {
        let max_val = (1u64 << (7 * len)) - 1;
        if n <= max_val {
            let mut out = n.to_be_bytes()[8 - len..].to_vec();
            out[0] |= 1u8 << (8 - len);
            return out;
        }
    }
    unreachable!("u64 fits the 8-octet vint")
}

/// One element: id + size + payload.
fn el(id: u32, payload: &[u8]) -> Vec<u8> {
    let mut out = id_bytes(id);
    out.extend(size_vint(payload.len() as u64));
    out.extend_from_slice(payload);
    out
}

/// One element whose declared size is `declared` — more than the
/// payload carried (the truncated-cluster shape: the extent claims
/// bytes the stream never delivers).
fn el_declared(id: u32, payload: &[u8], declared: u64) -> Vec<u8> {
    let mut out = id_bytes(id);
    out.extend(size_vint(declared));
    out.extend_from_slice(payload);
    out
}

/// A uint element (minimal-width big-endian payload).
fn el_uint(id: u32, v: u64) -> Vec<u8> {
    let be = v.to_be_bytes();
    let first = be.iter().position(|&b| b != 0).unwrap_or(7);
    el(id, &be[first..])
}

/// Concatenate element buffers.
fn cat(parts: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    for p in parts {
        out.extend_from_slice(p);
    }
    out
}

/// A minimal `EBML` header for DocType `matroska`.
fn ebml_header() -> Vec<u8> {
    cat(&[
        el(ebml::ID_EBML_VERSION, &[1]),
        el(ebml::ID_EBML_READ_VERSION, &[1]),
        el(ebml::ID_EBML_MAX_ID_LENGTH, &[4]),
        el(ebml::ID_EBML_MAX_SIZE_LENGTH, &[8]),
        el(ebml::ID_DOCTYPE, b"matroska"),
        el(ebml::ID_DOCTYPE_VERSION, &[2]),
        el(ebml::ID_DOCTYPE_READ_VERSION, &[2]),
    ])
}

/// A minimal `Info` (scale 1 ms).
fn info() -> Vec<u8> {
    el(ebml::ID_INFO, &el_uint(ebml::ID_TIMESTAMP_SCALE, 1_000_000))
}

/// One audio `TrackEntry` (number, type, CodecID).
fn track_entry(number: u64) -> Vec<u8> {
    let mut entry = el_uint(ebml::ID_TRACK_NUMBER, number);
    entry.extend(el_uint(ebml::ID_TRACK_TYPE, 2));
    entry.extend(el(ebml::ID_CODEC_ID, b"A_OPUS"));
    el(ebml::ID_TRACK_ENTRY, &entry)
}

/// One `SimpleBlock` element: track vint, i16 relative timecode,
/// flags, payload (single unlaced frame).
fn simple_block(track: u8, rel_tc: i16, flags: u8, payload: &[u8]) -> Vec<u8> {
    let mut body = vec![0x80 | track];
    body.extend_from_slice(&rel_tc.to_be_bytes());
    body.push(flags);
    body.extend_from_slice(payload);
    el(ebml::ID_SIMPLE_BLOCK, &body)
}

/// A full hand-built file: EBML header + exact-sized `Segment` over
/// `children`.
fn mkv_file(children: &[Vec<u8>]) -> Vec<u8> {
    let payload = cat(children);
    let mut out = el(ebml::ID_EBML_HEADER, &ebml_header());
    out.extend(id_bytes(ebml::ID_SEGMENT));
    out.extend(size_vint(payload.len() as u64));
    out.extend(payload);
    out
}

// ---------------------------------------------------------------------------
// The window-slide pins (F0.4d)
// ---------------------------------------------------------------------------

/// A seekable source whose reported size is larger than the bytes it
/// carries — the truncated-after-open shape: the container's declared
/// extents run past the bytes that exist, and reads past the real
/// bytes are a clean EOF (`Ok(0)`).
struct DeclaredLongerStream {
    inner: BytesStream,
    declared_len: u64,
}

impl SourceStream for DeclaredLongerStream {
    fn uri(&self) -> &SourceUri {
        self.inner.uri()
    }

    fn size(&self) -> Option<u64> {
        Some(self.declared_len)
    }

    fn supports_seek(&self) -> bool {
        self.inner.supports_seek()
    }

    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        self.inner.read(len)
    }

    fn read_into(&mut self, out: &mut [u8]) -> Result<usize, EngineError> {
        self.inner.read_into(out)
    }

    fn seek(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError> {
        self.inner.seek(origin)
    }

    fn position(&self) -> SourcePosition {
        self.inner.position()
    }
}

/// The bounded walk's terminal state (the watchdog thread's report).
#[derive(Debug)]
enum WalkEnd {
    /// The stream ended cleanly.
    Eos,
    /// A typed refusal surfaced to the caller (the error code).
    Refused(String),
}

/// Drain a started demuxer until it refuses or ends, collecting the
/// packet payloads — bounded by the caller's watchdog, not by this
/// loop (the livelock never returns).
fn walk(demuxer: &mut MkvDemuxer) -> (Vec<Vec<u8>>, WalkEnd) {
    let mut payloads = Vec::new();
    loop {
        match demuxer.next_event() {
            Ok(DemuxEvent::Packet(p)) => payloads.push(p.data.to_vec()),
            Ok(DemuxEvent::EndOfStream) => return (payloads, WalkEnd::Eos),
            Ok(_) => {}
            Err(e) => {
                return (
                    payloads,
                    WalkEnd::Refused(e.context.code.as_str().to_string()),
                );
            }
        }
    }
}

/// The audit's executed livelock shape: a SimpleBlock whose span is
/// wider than the 4 MiB walk window (well inside the 32 MiB block
/// gate) in an otherwise complete file — the slide re-read the same
/// window forever, one seek + 4 MiB read per iteration (the probe was
/// killed by the watchdog, exit 124). Fixed: the slide that cannot
/// cover the block terminates with the module's typed truncation.
#[test]
fn fx1_a04_window_slide_oversize_block_terminates_typed() {
    // The block payload: one window + 64 KiB (the span exceeds any
    // 4 MiB slide; the element itself stays far under MAX_BLOCK_BYTES).
    let span = MAX_CLUSTER_WINDOW as usize + 64 * 1024;
    let big_block = simple_block(1, 0, 0x80, &vec![0x57u8; span]);
    let bytes = mkv_file(&[
        info(),
        el(ebml::ID_TRACKS, &track_entry(1)),
        el(
            ebml::ID_CLUSTER,
            &cat(&[el_uint(ebml::ID_TIMECODE, 0), big_block]),
        ),
    ]);
    let mut demuxer = MkvDemuxer::new(mem_stream(bytes).must()).must();
    demuxer.start().must();
    let (tx, rx) = mpsc::channel();
    let walker = thread::Builder::new()
        .name("fx1-a04-oversize-walk".into())
        .spawn(move || {
            let _ = tx.send(walk(&mut demuxer));
        })
        .must();
    let reported = rx.recv_timeout(WATCHDOG);
    assert!(
        reported.is_ok(),
        "window-slide livelock: the walk never returned (watchdog {} s)",
        WATCHDOG.as_secs()
    );
    let (payloads, end) = reported.must();
    assert!(
        payloads.is_empty(),
        "the oversize block never demuxes — the refusal precedes it"
    );
    match end {
        WalkEnd::Refused(code) => assert_eq!(code, "demux.mkv.truncated"),
        other => unreachable!("expected the typed truncation, got {other:?}"),
    }
    walker
        .join()
        .unwrap_or_else(|_| unreachable!("walk thread panicked"));
}

/// The stalled-stream shape: the cluster (and segment) declare a
/// ghost tail the stream never delivers — the real blocks demux, the
/// cursor-advance slide at the real end re-issued a zero-byte read
/// forever. Fixed: the real packet first, then the typed truncation
/// (never a spin).
#[test]
fn fx1_a04_window_slide_truncated_cluster_terminates_typed() {
    let ghost = 1024u64;
    // The cluster content ends exactly at the last block byte; the
    // declared extents claim `ghost` bytes more.
    let cluster_body = cat(&[
        el_uint(ebml::ID_TIMECODE, 0),
        simple_block(1, 0, 0x80, b"stall"),
    ]);
    let cluster = el_declared(
        ebml::ID_CLUSTER,
        &cluster_body,
        cluster_body.len() as u64 + ghost,
    );
    let seg_children = cat(&[info(), el(ebml::ID_TRACKS, &track_entry(1)), cluster]);
    let full = cat(&[
        el(ebml::ID_EBML_HEADER, &ebml_header()),
        el_declared(
            ebml::ID_SEGMENT,
            &seg_children,
            seg_children.len() as u64 + ghost,
        ),
    ]);
    // The ghost is never materialized: the real bytes end at the last
    // block; the stream reports the declared (larger) length.
    let declared_len = full.len() as u64 + ghost;
    let real: Arc<[u8]> = full.into();
    let stream: Box<dyn SourceStream> = Box::new(DeclaredLongerStream {
        inner: BytesStream::new(real).must(),
        declared_len,
    });
    let mut demuxer = MkvDemuxer::new(stream).must();
    demuxer.start().must();
    let (tx, rx) = mpsc::channel();
    let walker = thread::Builder::new()
        .name("fx1-a04-truncated-walk".into())
        .spawn(move || {
            let _ = tx.send(walk(&mut demuxer));
        })
        .must();
    let reported = rx.recv_timeout(WATCHDOG);
    assert!(
        reported.is_ok(),
        "window-slide livelock: the walk never returned (watchdog {} s)",
        WATCHDOG.as_secs()
    );
    let (payloads, end) = reported.must();
    assert_eq!(
        payloads,
        vec![b"stall".to_vec()],
        "the real block demuxes before the stall"
    );
    match end {
        WalkEnd::Refused(code) => assert_eq!(code, "demux.mkv.truncated"),
        other => unreachable!("expected the typed truncation, got {other:?}"),
    }
    walker
        .join()
        .unwrap_or_else(|_| unreachable!("walk thread panicked"));
}
