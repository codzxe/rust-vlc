//! Card 3.B3's read-chain zero-allocation proofs (directives-audit M4,
//! design E3(b)): the movi-walk-shaped sequence over the borrow seams
//! — header peeks, prefix reads, the payload slot pool — allocates
//! **nothing** in the steady state, measured with the testkit counting
//! allocator (runtime evidence, D-6 — not syntactic green).
//!
//! One `#[test]` per proof, whole-binary serialized (the allocator is
//! process-global; sibling test threads would be counted — see
//! `testkit::alloc_count`). The `lumen` fixtures' payload sizes are
//! KBs, so the walk shape here is: for each chunk, peek its 8-byte
//! header at the cursor, read the payload into a pooled slot, hand the
//! packet its clone — exactly the sequence `step_movi_inner` performs
//! once W2-07 wires it onto these seams (the walk itself is W2-07's
//! file; this binary proves the seam's cost).

use std::num::NonZeroUsize;

use media_types::ids::SourceUri;
use media_types::packet::SharedBytes;
use media_types::{EngineError, PipelineStage};
use plugin_api::{SeekOrigin, SourcePosition, SourceStream};
use testkit::alloc_count::{CountingGlobalAlloc, serialize_tests};
use testkit::Must;

#[global_allocator]
static ALLOC: CountingGlobalAlloc = CountingGlobalAlloc::new();

/// An in-memory seekable source whose `read_into` is the honest local
/// override (memcpy from the backing buffer — zero allocation), the
/// shape `FileStream`/`SyntheticStream` provide in production.
struct BorrowMemSource {
    uri: SourceUri,
    data: std::sync::Arc<[u8]>,
    pos: usize,
}

impl BorrowMemSource {
    fn new(data: std::sync::Arc<[u8]>) -> BorrowMemSource {
        BorrowMemSource {
            uri: SourceUri::parse("memory://zero-alloc-proof").must(),
            data,
            pos: 0,
        }
    }
}

impl SourceStream for BorrowMemSource {
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

    fn read_into(&mut self, out: &mut [u8]) -> Result<usize, EngineError> {
        let end = (self.pos + out.len()).min(self.data.len());
        out[..end - self.pos].copy_from_slice(&self.data[self.pos..end]);
        let got = end - self.pos;
        self.pos = end;
        Ok(got)
    }

    fn seek(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError> {
        let target: u64 = match origin {
            SeekOrigin::Start(to) => to,
            SeekOrigin::Current(delta) => {
                let cur = self.pos as i64;
                match cur.checked_add(delta) {
                    Some(n) if n >= 0 => n as u64,
                    _ => {
                        return Err(EngineError::new(
                            PipelineStage::Open,
                            media_types::ErrorKind::Inaccessible {
                                reason: media_types::error::AccessDenied::NotFound,
                            },
                            media_types::error::ErrorContext::new("test.mem.seek-out-of-range"),
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
                media_types::error::ErrorContext::new("test.mem.seek-out-of-range"),
            ));
        }
        self.pos = target as usize;
        Ok(SourcePosition(target))
    }

    fn position(&self) -> SourcePosition {
        SourcePosition(self.pos as u64)
    }
}

/// Build a movi-shaped byte layout: N chunks of `payload` bytes, each
/// behind an 8-byte `fourcc + u32le size` header, WORD-padded.
fn movi_layout(chunks: usize, payload: usize) -> std::sync::Arc<[u8]> {
    let mut out = Vec::with_capacity(chunks * (8 + payload + 1));
    for index in 0..chunks {
        out.extend_from_slice(b"00dc");
        out.extend_from_slice(&(payload as u32).to_le_bytes());
        let base = (index * 31 + 7) as u8;
        out.extend(std::iter::repeat(base).take(payload));
        if payload % 2 == 1 {
            out.push(0);
        }
    }
    out.into()
}

/// The header path (M4's first kill): peeking N chunk headers through
/// the borrow seam allocates nothing once the reader is warm — the
/// old path spent two Vecs (8 B each) per chunk.
#[test]
fn header_peek_loop_allocates_nothing_when_warm() {
    let _serial = serialize_tests();
    let data = movi_layout(1024, 300);
    let payload = 300usize;
    let stride = 8 + payload + (payload & 1);
    let mut reader =
        demux::avi::riff::SourceReader::new(Box::new(BorrowMemSource::new(std::sync::Arc::clone(
            &data,
        ))))
        .must();

    // Warm-up: first peeks size nothing but the reader's internals
    // settle (the source's Arc, the position queries).
    let mut hdr = [0u8; 8];
    let mut cursor = 0u64;
    for _ in 0..16 {
        reader.peek_chunk_header_into(cursor, &mut hdr).must();
        cursor += stride as u64;
    }

    let guard = ALLOC.state().guard();
    let mut parsed = 0usize;
    cursor = 0;
    for _ in 0..1000 {
        reader.peek_chunk_header_into(cursor, &mut hdr).must();
        let header = demux::avi::riff::ChunkHeader::from_le_bytes(&hdr);
        assert_eq!(header.fourcc, demux::avi::riff::FourCC::tag(b"00dc"));
        assert_eq!(header.size as usize, payload);
        cursor += stride as u64;
        parsed += 1;
    }
    assert_eq!(parsed, 1000);
    guard.assert_zero("1000 warm header peeks (the M4 header-alloc kill)");
}

/// The walk shape end to end (M4's payload kill for uniform streams):
/// per chunk, peek the header at the cursor, fill the payload through
/// the pool, drop the packet's handle — after the pool warms to its
/// two slots, a full 1000-chunk walk allocates nothing.
#[test]
fn pooled_movi_walk_allocates_nothing_when_warm() {
    let _serial = serialize_tests();
    let data = movi_layout(1050, 512);
    let payload = 512usize;
    let stride = 8 + payload; // even payload: no pad byte
    let mut reader =
        demux::avi::riff::SourceReader::new(Box::new(BorrowMemSource::new(std::sync::Arc::clone(
            &data,
        ))))
        .must();
    let mut pool = demux::avi::riff::PayloadSlots::default();

    // Warm-up: hold two handles live so the pool grows to its bound,
    // then drop both — the slots go idle for the measured loop.
    let mut cursor = 0u64;
    let mut hdr = [0u8; 8];
    let mut held = Vec::new();
    for _ in 0..2 {
        reader.peek_chunk_header_into(cursor, &mut hdr).must();
        let at = cursor + 8;
        let handle = pool
            .fill(payload, |out| reader.read_prefix_into(out, at))
            .must();
        assert_eq!(handle.len(), payload);
        held.push(handle);
        cursor += stride as u64;
    }
    drop(held);
    assert_eq!(pool.resident_slots(), pool.max_slots(), "the pool is warm");

    let guard = ALLOC.state().guard();
    for _ in 0..1000 {
        reader.peek_chunk_header_into(cursor, &mut hdr).must();
        let header = demux::avi::riff::ChunkHeader::from_le_bytes(&hdr);
        let at = cursor + 8;
        let handle = pool
            .fill(header.size as usize, |out| {
                reader.read_prefix_into(out, at)
            })
            .must();
        // The bytes are the chunk's own (the layout's per-chunk base).
        let base = ((cursor / stride as u64) * 31 + 7) as u8;
        assert_eq!(handle.len(), payload);
        assert!(handle.as_slice().iter().all(|&b| b == base));
        drop(handle);
        cursor += stride as u64;
    }
    guard.assert_zero("1000 warm pooled chunks (header + payload, zero allocs)");
}

/// The borrow twins on their own: exact and prefix reads into caller
/// memory allocate nothing (the caller owns the buffer — including
/// reads far larger than any internal window).
#[test]
fn borrow_reads_allocate_nothing() {
    let _serial = serialize_tests();
    let data = movi_layout(64, 2048);
    let mut reader =
        demux::avi::riff::SourceReader::new(Box::new(BorrowMemSource::new(std::sync::Arc::clone(
            &data,
        ))))
        .must();

    let mut scratch = vec![0u8; 2048];
    // Warm-up.
    reader.read_exact_into(&mut scratch[..64]).must();
    let _ = reader.read_prefix_into(&mut scratch, 8);

    let guard = ALLOC.state().guard();
    let mut hdr = [0u8; 8];
    for _ in 0..500 {
        reader.read_exact_into(&mut hdr).must();
        let got = reader.read_prefix_into(&mut scratch, 0).must();
        assert_eq!(got, 2048);
    }
    guard.assert_zero("500 borrow reads (exact + prefix), zero allocs");
}

/// The Vec-returning twins (still used by the not-yet-wired callers —
/// `index.rs`'s per-entry reads, the walk's payload paths until W2-07
/// swaps them) now cost **exactly one** allocation per call — the
/// borrow seam fills the result directly. The pre-3.B3 chain spent two
/// per call (the reader's accumulator plus the source-read result).
#[test]
fn vec_twins_cost_exactly_one_allocation_per_call() {
    let _serial = serialize_tests();
    let data = movi_layout(64, 2048);
    let mut reader =
        demux::avi::riff::SourceReader::new(Box::new(BorrowMemSource::new(std::sync::Arc::clone(
            &data,
        ))))
        .must();

    // Warm-up.
    let _ = reader.read_exact_or_end(16).must();
    let _ = reader.read_prefix_at(16, 8).must();

    const CALLS: u64 = 256;
    let guard = ALLOC.state().guard();
    for index in 0..CALLS {
        let at = (index % 32) * 64;
        match reader.read_exact_or_end(16).must() {
            demux::avi::riff::ReadExact::Complete(bytes) => assert_eq!(bytes.len(), 16),
            other => unreachable!("complete, got {other:?}"),
        }
        let prefix = reader.read_prefix_at(16, at).must();
        assert_eq!(prefix.len(), 16);
    }
    let delta = guard.delta();
    assert_eq!(
        delta.alloc_calls, CALLS * 2,
        "exactly one allocation per Vec twin call (idx1's per-entry cost until \
         the walk/index swap lands): {} calls, {} allocs",
        CALLS * 2,
        delta.alloc_calls
    );
}

/// The pool's spill path for varying sizes is bounded and honest: a
/// mixed-size walk (the JPEG shape) pays one exact-size allocation per
/// payload — never growth, never waits.
#[test]
fn varying_size_spill_is_exactly_one_alloc_per_payload() {
    let _serial = serialize_tests();
    let data = movi_layout(64, 2048);
    let mut reader =
        demux::avi::riff::SourceReader::new(Box::new(BorrowMemSource::new(std::sync::Arc::clone(
            &data,
        ))))
        .must();
    let mut pool = demux::avi::riff::PayloadSlots::default();

    // Warm-up with one size so the pool is populated.
    let _ = pool
        .fill(256, |out| reader.read_prefix_into(out, 0))
        .must();
    let guard = ALLOC.state().guard();
    const PAYLOADS: u64 = 128;
    for index in 0..PAYLOADS {
        // A distinct size per payload: every one spills (the varying
        // shape), each into one exact-size allocation.
        let want = 64 + (index as usize % 512);
        let at = (index % 32) * 64;
        let handle = pool
            .fill(want, |out| reader.read_prefix_into(out, at))
            .must();
        assert_eq!(handle.len(), want.min(2048));
        drop(handle);
    }
    let delta = guard.delta();
    // The spill allocation plus the Arc conversion inside
    // SharedBytes::from_vec — the same two-step every payload paid
    // pre-3.B3, minus the reader's accumulator.
    assert_eq!(
        delta.alloc_calls, PAYLOADS * 2,
        "one spill + one Arc conversion per varying payload: {} payloads, {} allocs",
        PAYLOADS, delta.alloc_calls
    );
}
