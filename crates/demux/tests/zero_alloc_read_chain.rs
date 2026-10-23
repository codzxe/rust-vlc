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
//!
//! Foreign-noise discipline (W3-05R): every proof runs through
//! [`clean_replay`] — a settle pause plus a one-replay backstop for
//! harness thread-spawn noise; see its docs for the measurement and
//! the D-5 argument (it hardens against the environment, never
//! weakens the proof against the code).

use media_types::ids::SourceUri;
use media_types::{EngineError, PipelineStage};
use plugin_api::{SeekOrigin, SourcePosition, SourceStream};
use testkit::Must;
use testkit::alloc_count::{AllocDelta, CountingGlobalAlloc, serialize_tests};

#[global_allocator]
static ALLOC: CountingGlobalAlloc = CountingGlobalAlloc::new();

/// Let an in-flight sibling-thread spawn finish before a measured
/// window opens.
///
/// The counting allocator is process-global and counts EVERY
/// allocation in the process — including the libtest harness's own
/// thread machinery. `serialize_tests` serializes test *bodies*, but
/// the harness starts the next test's thread while the current one
/// still measures: a spawn in flight at the lock handoff lands inside
/// the window (observed here: 2–10 events / 55 B–10 KiB per handoff
/// with sleeping tests; one real-suite flake, `real_movi_walk` at
/// 2 events / 99 B). The 30 ms settle lets the in-flight spawn finish
/// first (the same diagnostic: 40/40 windows clean vs nearly all
/// dirty without the pause).
fn settle_foreign_spawns() {
    std::thread::sleep(std::time::Duration::from_millis(30));
}

/// Run a deterministic measured `section` under the binary's
/// foreign-noise discipline and return a delta `accepts` signs off
/// on (D-5: this hardens the proof against the environment, never
/// weakens it against the code).
///
/// Every section in this binary is deterministic — in-memory bytes,
/// no clocks, one thread — so a REAL allocation defect reproduces on
/// every replay and fails the both-replays-dirty assert below with
/// both deltas printed. Transient harness noise (a sibling spawn
/// that survived the settle) fails at most the first window; the
/// single replay then runs the whole section again on fresh state
/// and must come back clean. A deterministic defect cannot pass one
/// clean replay, so no false pass exists; a second dirty replay is
/// always a real defect, so no false tolerance exists either.
fn clean_replay<R, F, A>(mut section: F, accepts: A) -> R
where
    R: Copy + std::fmt::Debug,
    F: FnMut() -> R,
    A: Fn(&R) -> bool,
{
    settle_foreign_spawns();
    let first = section();
    if accepts(&first) {
        return first;
    }
    settle_foreign_spawns();
    let second = section();
    assert!(
        accepts(&second),
        "measured window dirty on BOTH deterministic replays — a real \
         allocation defect, not harness noise (first {first:?}, second {second:?})"
    );
    second
}

/// `accepts` for the zero-allocation proofs: no events, no bytes
/// (the same predicate `CountingGuard::assert_zero` enforces).
fn is_zero(delta: &AllocDelta) -> bool {
    delta.alloc_calls == 0 && delta.total_bytes == 0
}

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
        out.extend(std::iter::repeat_n(base, payload));
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
    let delta = clean_replay(
        || {
            let data = movi_layout(1024, 300);
            let payload = 300usize;
            let stride = 8 + payload + (payload & 1);
            let mut reader = demux::avi::riff::SourceReader::new(Box::new(BorrowMemSource::new(
                std::sync::Arc::clone(&data),
            )))
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
            guard.delta()
        },
        is_zero,
    );
    assert_eq!(
        (delta.alloc_calls, delta.total_bytes),
        (0, 0),
        "1000 warm header peeks (the M4 header-alloc kill)"
    );
}

/// The walk shape end to end (M4's payload kill for uniform streams):
/// per chunk, peek the header at the cursor, fill the payload through
/// the pool, drop the packet's handle — after the pool warms to its
/// two slots, a full 1000-chunk walk allocates nothing.
#[test]
fn pooled_movi_walk_allocates_nothing_when_warm() {
    let _serial = serialize_tests();
    let delta = clean_replay(
        || {
            let data = movi_layout(1050, 512);
            let payload = 512usize;
            let stride = 8 + payload; // even payload: no pad byte
            let mut reader = demux::avi::riff::SourceReader::new(Box::new(BorrowMemSource::new(
                std::sync::Arc::clone(&data),
            )))
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
                    .fill(header.size as usize, |out| reader.read_prefix_into(out, at))
                    .must();
                // The bytes are the chunk's own (the layout's per-chunk base).
                let base = ((cursor / stride as u64) * 31 + 7) as u8;
                assert_eq!(handle.len(), payload);
                assert!(handle.as_slice().iter().all(|&b| b == base));
                drop(handle);
                cursor += stride as u64;
            }
            guard.delta()
        },
        is_zero,
    );
    assert_eq!(
        (delta.alloc_calls, delta.total_bytes),
        (0, 0),
        "1000 warm pooled chunks (header + payload, zero allocs)"
    );
}

/// The borrow twins on their own: exact and prefix reads into caller
/// memory allocate nothing (the caller owns the buffer — including
/// reads far larger than any internal window).
#[test]
fn borrow_reads_allocate_nothing() {
    let _serial = serialize_tests();
    let delta = clean_replay(
        || {
            let data = movi_layout(64, 2048);
            let mut reader = demux::avi::riff::SourceReader::new(Box::new(BorrowMemSource::new(
                std::sync::Arc::clone(&data),
            )))
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
            guard.delta()
        },
        is_zero,
    );
    assert_eq!(
        (delta.alloc_calls, delta.total_bytes),
        (0, 0),
        "500 borrow reads (exact + prefix), zero allocs"
    );
}

/// The Vec-returning twins (still used by the not-yet-wired callers —
/// `index.rs`'s per-entry reads, the walk's payload paths until W2-07
/// swaps them) now cost **exactly one** allocation per call — the
/// borrow seam fills the result directly. The pre-3.B3 chain spent two
/// per call (the reader's accumulator plus the source-read result).
#[test]
fn vec_twins_cost_exactly_one_allocation_per_call() {
    let _serial = serialize_tests();
    const CALLS: u64 = 256;
    let delta = clean_replay(
        || {
            let data = movi_layout(64, 2048);
            let mut reader = demux::avi::riff::SourceReader::new(Box::new(BorrowMemSource::new(
                std::sync::Arc::clone(&data),
            )))
            .must();

            // Warm-up.
            let _ = reader.read_exact_or_end(16).must();
            let _ = reader.read_prefix_at(16, 8).must();

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
            guard.delta()
        },
        |delta: &AllocDelta| delta.alloc_calls == CALLS * 2,
    );
    assert_eq!(
        delta.alloc_calls,
        CALLS * 2,
        "exactly one allocation per Vec twin call (idx1's per-entry cost until \
         the walk/index swap lands): {} calls, {} allocs",
        CALLS * 2,
        delta.alloc_calls
    );
}

/// The pool's varying-size path (7-c, V-1): a mixed-size walk — the
/// JPEG shape, a distinct size per payload — pays **zero** allocations
/// once the arena is warm. Pre-7-c the exact-size slot pool never
/// matched a varying size, so every payload spilled (one exact-size
/// allocation + one Arc conversion = 2 events/payload — the pin this
/// test replaced, `PAYLOADS * 2`); the arena serves any size ≤ its
/// capacity, packing small payloads at the cursor and recycling the
/// drained generation, so the per-payload count went 2 → 0 (the
/// counting-allocator evidence for the memory-audit V-1 fix).
#[test]
fn varying_size_payloads_pack_into_the_arena_zero_alloc() {
    let _serial = serialize_tests();
    const PAYLOADS: u64 = 128;
    let delta = clean_replay(
        || {
            let data = movi_layout(64, 2048);
            let mut reader = demux::avi::riff::SourceReader::new(Box::new(BorrowMemSource::new(
                std::sync::Arc::clone(&data),
            )))
            .must();
            let mut pool = demux::avi::riff::PayloadSlots::default();

            // Warm-up: one fill spills a 256-byte arena into the pool.
            let _ = pool.fill(256, |out| reader.read_prefix_into(out, 0)).must();
            let guard = ALLOC.state().guard();
            for index in 0..PAYLOADS {
                // A distinct size per payload (the varying shape): each
                // packs at the cursor or recycles the drained arena —
                // never an allocation.
                let want = 64 + (index as usize % 512);
                let at = (index % 32) * 64;
                let handle = pool
                    .fill(want, |out| reader.read_prefix_into(out, at))
                    .must();
                assert_eq!(handle.len(), want.min(2048));
                drop(handle);
            }
            guard.delta()
        },
        is_zero,
    );
    assert_eq!(
        delta.alloc_calls,
        0,
        "128 varying payloads over a warm arena: 0 allocs (pre-7-c pin: {} — \
         one spill + one Arc conversion per payload)",
        PAYLOADS * 2
    );
}

/// The true spill path stays bounded and honest (7-c): payloads over
/// the arena ceiling — or arriving while every arena is pinned — pay
/// exactly one exact-size allocation plus the constant-size Arc
/// header (2 events/payload, the same two-step the pre-7-c varying
/// walk paid for *every* payload), and oversized spills never enter
/// the pool.
#[test]
fn over_ceiling_spill_is_exactly_two_allocs_per_payload() {
    let _serial = serialize_tests();
    const PAYLOADS: u64 = 32;
    const CEILING: usize = 1024;
    let delta = clean_replay(
        || {
            let data = movi_layout(64, 4096);
            let mut reader = demux::avi::riff::SourceReader::new(Box::new(BorrowMemSource::new(
                std::sync::Arc::clone(&data),
            )))
            .must();
            // A tight ceiling: every ask in the loop is over it, so
            // each payload is a true spill that never pools.
            let mut pool = demux::avi::riff::PayloadSlots::new(2, CEILING);

            // Warm-up: one under-ceiling fill populates the pool.
            let _ = pool.fill(256, |out| reader.read_prefix_into(out, 0)).must();
            let guard = ALLOC.state().guard();
            for index in 0..PAYLOADS {
                let want = CEILING + 64 + (index as usize % 256);
                let handle = pool
                    .fill(want, |out| reader.read_prefix_into(out, 0))
                    .must();
                assert_eq!(handle.len(), want);
                drop(handle);
            }
            assert_eq!(pool.resident_slots(), 1, "oversized spills never pool");
            guard.delta()
        },
        |delta: &AllocDelta| delta.alloc_calls == PAYLOADS * 2,
    );
    assert_eq!(
        delta.alloc_calls,
        PAYLOADS * 2,
        "one spill buffer + one Arc header per over-ceiling payload: {} payloads, {} allocs",
        PAYLOADS,
        delta.alloc_calls
    );
}

// ---------------------------------------------------------------------------
// The end-to-end proofs (design E3(b)/(c)): the REAL `AviDemuxer` over
// the real walk — the M4 kill demonstrated at the public-contract
// boundary, not just at the seam.
// ---------------------------------------------------------------------------

use demux::avi::header::{DamageLog, StreamDesc, StreamFormat};
use demux::avi::index::{AVIIF_KEYFRAME, parse_idx1};
use demux::avi::riff::FourCC;
use media_types::MediaKind;
use plugin_api::Demuxer;

/// Build a one-video-stream AVI (the doctest's builder shape) whose
/// `movi` carries `chunks` uniform `payload`-byte `00dc` chunks, plus
/// — when `with_idx1` — an `idx1` entry per chunk.
fn avi_bytes(chunks: usize, payload: usize, with_idx1: bool) -> std::sync::Arc<[u8]> {
    let mut b = Vec::with_capacity(chunks * (8 + payload) + 1024);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(b"AVI ");
    // hdrl: avih + one strl (video MJPG).
    let hdrl_start = b.len();
    b.extend_from_slice(b"LIST");
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(b"hdrl");
    let mut avih = Vec::new();
    avih.extend_from_slice(&33_333u32.to_le_bytes());
    avih.extend_from_slice(&0u32.to_le_bytes());
    avih.extend_from_slice(&0u32.to_le_bytes());
    avih.extend_from_slice(&0x110u32.to_le_bytes());
    avih.extend_from_slice(&1u32.to_le_bytes());
    avih.extend_from_slice(&0u32.to_le_bytes());
    avih.extend_from_slice(&2u32.to_le_bytes());
    avih.extend_from_slice(&4u32.to_le_bytes());
    avih.extend_from_slice(&320u32.to_le_bytes());
    avih.extend_from_slice(&240u32.to_le_bytes());
    avih.extend(&[0u8; 16]);
    b.extend_from_slice(b"avih");
    b.extend_from_slice(&(avih.len() as u32).to_le_bytes());
    b.extend(&avih);
    // strl: strh (video) + strf (BITMAPINFOHEADER).
    b.extend_from_slice(b"LIST");
    let strl_size_pos = b.len();
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(b"strl");
    let mut strh = Vec::new();
    strh.extend_from_slice(b"vids");
    strh.extend_from_slice(b"MJPG");
    strh.extend(&[0u8; 12]);
    strh.extend_from_slice(&1u32.to_le_bytes()); // scale
    strh.extend_from_slice(&30u32.to_le_bytes()); // rate
    strh.extend_from_slice(&0u32.to_le_bytes());
    strh.extend_from_slice(&(chunks as u32).to_le_bytes()); // length
    strh.extend_from_slice(&4u32.to_le_bytes());
    strh.extend_from_slice(&u32::MAX.to_le_bytes());
    strh.extend_from_slice(&0u32.to_le_bytes()); // sample size
    strh.extend(&[0u8; 8]);
    b.extend_from_slice(b"strh");
    b.extend_from_slice(&(strh.len() as u32).to_le_bytes());
    b.extend(&strh);
    let mut strf = Vec::new();
    strf.extend_from_slice(&40u32.to_le_bytes());
    strf.extend_from_slice(&320u32.to_le_bytes());
    strf.extend_from_slice(&240u32.to_le_bytes());
    strf.extend_from_slice(&1u16.to_le_bytes());
    strf.extend_from_slice(&24u16.to_le_bytes());
    strf.extend_from_slice(b"MJPG");
    strf.extend(&vec![0u8; 20]);
    b.extend_from_slice(b"strf");
    b.extend_from_slice(&(strf.len() as u32).to_le_bytes());
    b.extend(&strf);
    let strl_size = b.len() - strl_size_pos - 4;
    b[strl_size_pos..strl_size_pos + 4].copy_from_slice(&(strl_size as u32).to_le_bytes());
    let hdrl_size = b.len() - hdrl_start - 8;
    b[hdrl_start + 4..hdrl_start + 8].copy_from_slice(&(hdrl_size as u32).to_le_bytes());
    // movi: `chunks` uniform video chunks.
    let movi_list_pos = b.len();
    b.extend_from_slice(b"LIST");
    let movi_size_pos = b.len();
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(b"movi");
    let movi_form_pos = (b.len() - 4) as u64;
    let mut chunk_positions: Vec<u64> = Vec::with_capacity(chunks);
    for index in 0..chunks {
        chunk_positions.push(b.len() as u64);
        b.extend_from_slice(b"00dc");
        b.extend_from_slice(&(payload as u32).to_le_bytes());
        let base = (index % 251) as u8;
        b.extend(std::iter::repeat_n(base, payload));
        if payload % 2 == 1 {
            b.push(0);
        }
    }
    let movi_size = b.len() - movi_size_pos - 4;
    b[movi_size_pos..movi_size_pos + 4].copy_from_slice(&(movi_size as u32).to_le_bytes());
    let _ = movi_list_pos;
    // idx1: one entry per chunk (all keyframes, movi-relative offsets).
    if with_idx1 {
        b.extend_from_slice(b"idx1");
        b.extend_from_slice(&((chunks * 16) as u32).to_le_bytes());
        for &chunk_pos in &chunk_positions {
            b.extend_from_slice(b"00dc");
            b.extend_from_slice(&AVIIF_KEYFRAME.to_le_bytes());
            b.extend_from_slice(&((chunk_pos - movi_form_pos) as u32).to_le_bytes());
            b.extend_from_slice(&(payload as u32).to_le_bytes());
        }
    }
    let riff_size = b.len() - 8;
    b[4..8].copy_from_slice(&(riff_size as u32).to_le_bytes());
    b.into()
}

/// The M4 kill, end to end (design E3(b)): a real `AviDemuxer` over an
/// in-memory AVI with 1050 uniform chunks, walked through the public
/// `Demuxer` contract. After the warm-up (pool growth, pending-queue
/// seeding, sticky machinery), **1000 consecutive packet events
/// allocate nothing** — header peek, payload read, packet hand-off
/// all zero (the pre-3.B3 chain spent ~3 allocs + 2 copies per
/// packet).
#[test]
fn real_movi_walk_allocates_nothing_per_packet() {
    let _serial = serialize_tests();
    const CHUNKS: usize = 1050;
    const PAYLOAD: usize = 512;
    let delta = clean_replay(
        || {
            let bytes = avi_bytes(CHUNKS, PAYLOAD, false);
            let mut avi = demux::avi::AviDemuxer::new(
                demux::avi::test_support::mem_stream(std::sync::Arc::clone(&bytes)).must(),
            )
            .must();
            let tracks = avi.start().must();
            assert_eq!(tracks.len(), 1, "one video track");

            // Warm-up: the first events grow the pool to its bound, seed the
            // pending queue, and settle the one-time machinery.
            let mut seen = 0usize;
            while seen < 50 {
                match avi.next_event().must() {
                    plugin_api::DemuxEvent::Packet(packet) => {
                        assert_eq!(packet.data.len(), PAYLOAD);
                        drop(packet);
                        seen += 1;
                    }
                    plugin_api::DemuxEvent::EndOfStream => {
                        unreachable!("warm-up hit EOS at {seen}")
                    }
                    _ => {}
                }
            }

            let guard = ALLOC.state().guard();
            let mut walked = 0usize;
            while walked < 1000 {
                match avi.next_event().must() {
                    plugin_api::DemuxEvent::Packet(packet) => {
                        assert_eq!(packet.data.len(), PAYLOAD, "packet {walked}");
                        // Content sanity: the chunk's uniform fill byte.
                        let base = ((seen + walked) % 251) as u8;
                        assert!(
                            packet.data.as_slice().iter().all(|&byte| byte == base),
                            "packet {walked}: payload bytes are the chunk's own"
                        );
                        drop(packet);
                        walked += 1;
                    }
                    plugin_api::DemuxEvent::EndOfStream => {
                        unreachable!("EOS at {walked} of 1000 with {} chunks", CHUNKS - seen)
                    }
                    _ => {}
                }
            }
            let delta = guard.delta();
            // And the stream still ends cleanly afterwards (outside the
            // measured window).
            let mut ended = false;
            while !ended {
                ended = matches!(avi.next_event().must(), plugin_api::DemuxEvent::EndOfStream);
            }
            delta
        },
        is_zero,
    );
    assert_eq!(
        (delta.alloc_calls, delta.total_bytes),
        (0, 0),
        "1000 packets through the real walk (M4 end-to-end kill)"
    );
}

/// Build a two-stream AVI (video MJPG + audio PCM — the W5-14 lane
/// shape): `pairs` × (`00dc` video of `video_payload` uniform bytes,
/// `01wb` audio of `audio_payload` uniform bytes), one idx1 entry per
/// chunk (so the walk's keyframe cursor runs too).
fn avi_bytes_av(pairs: usize, video_payload: usize, audio_payload: usize) -> std::sync::Arc<[u8]> {
    let mut b = Vec::with_capacity(pairs * (16 + video_payload + audio_payload) + 1024);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(b"AVI ");
    let hdrl_start = b.len();
    b.extend_from_slice(b"LIST");
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(b"hdrl");
    let mut avih = Vec::new();
    for value in [33_333u32, 0, 0, 0x110, (pairs as u32), 0, 2, 0, 320, 240] {
        avih.extend_from_slice(&value.to_le_bytes());
    }
    avih.extend(&[0u8; 16]);
    b.extend_from_slice(b"avih");
    b.extend_from_slice(&(avih.len() as u32).to_le_bytes());
    b.extend(&avih);
    // strl 0: video MJPG.
    b.extend_from_slice(b"LIST");
    let v_strl_pos = b.len();
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(b"strl");
    let mut strh = Vec::new();
    strh.extend_from_slice(b"vids");
    strh.extend_from_slice(b"MJPG");
    strh.extend(&[0u8; 12]);
    strh.extend_from_slice(&1u32.to_le_bytes()); // scale
    strh.extend_from_slice(&30u32.to_le_bytes()); // rate
    strh.extend_from_slice(&0u32.to_le_bytes());
    strh.extend_from_slice(&(pairs as u32).to_le_bytes());
    strh.extend_from_slice(&4u32.to_le_bytes());
    strh.extend_from_slice(&u32::MAX.to_le_bytes());
    strh.extend_from_slice(&0u32.to_le_bytes()); // sample size (video)
    strh.extend(&[0u8; 8]);
    b.extend_from_slice(b"strh");
    b.extend_from_slice(&(strh.len() as u32).to_le_bytes());
    b.extend(&strh);
    let mut strf = Vec::new();
    strf.extend_from_slice(&40u32.to_le_bytes());
    strf.extend_from_slice(&320u32.to_le_bytes());
    strf.extend_from_slice(&240u32.to_le_bytes());
    strf.extend_from_slice(&1u16.to_le_bytes());
    strf.extend_from_slice(&24u16.to_le_bytes());
    strf.extend_from_slice(b"MJPG");
    strf.extend(&vec![0u8; 20]);
    b.extend_from_slice(b"strf");
    b.extend_from_slice(&(strf.len() as u32).to_le_bytes());
    b.extend(&strf);
    let size = b.len() - v_strl_pos - 4;
    b[v_strl_pos..v_strl_pos + 4].copy_from_slice(&(size as u32).to_le_bytes());
    // strl 1: audio PCM (stereo 16-bit 48 kHz, block align 4).
    b.extend_from_slice(b"LIST");
    let a_strl_pos = b.len();
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(b"strl");
    let mut strh = Vec::new();
    strh.extend_from_slice(b"auds");
    strh.extend(&[0u8; 4]); // no handler
    strh.extend(&[0u8; 12]);
    strh.extend_from_slice(&1u32.to_le_bytes()); // scale
    strh.extend_from_slice(&48_000u32.to_le_bytes()); // rate
    strh.extend_from_slice(&0u32.to_le_bytes());
    strh.extend_from_slice(&((pairs * audio_payload / 4) as u32).to_le_bytes());
    strh.extend_from_slice(&4u32.to_le_bytes());
    strh.extend_from_slice(&u32::MAX.to_le_bytes());
    strh.extend_from_slice(&4u32.to_le_bytes()); // sample size = block align
    strh.extend(&[0u8; 8]);
    b.extend_from_slice(b"strh");
    b.extend_from_slice(&(strh.len() as u32).to_le_bytes());
    b.extend(&strh);
    let mut strf = Vec::new();
    strf.extend_from_slice(&1u16.to_le_bytes()); // WAVE_FORMAT_PCM
    strf.extend_from_slice(&2u16.to_le_bytes()); // channels
    strf.extend_from_slice(&48_000u32.to_le_bytes()); // rate
    strf.extend_from_slice(&192_000u32.to_le_bytes()); // avg bytes/sec
    strf.extend_from_slice(&4u16.to_le_bytes()); // block align
    strf.extend_from_slice(&16u16.to_le_bytes()); // bits
    b.extend_from_slice(b"strf");
    b.extend_from_slice(&(strf.len() as u32).to_le_bytes());
    b.extend(&strf);
    let size = b.len() - a_strl_pos - 4;
    b[a_strl_pos..a_strl_pos + 4].copy_from_slice(&(size as u32).to_le_bytes());
    let hdrl_size = b.len() - hdrl_start - 8;
    b[hdrl_start + 4..hdrl_start + 8].copy_from_slice(&(hdrl_size as u32).to_le_bytes());
    // movi: interleaved uniform chunks.
    let movi_size_pos = b.len() + 4;
    b.extend_from_slice(b"LIST");
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(b"movi");
    let movi_form_pos = (b.len() - 4) as u64;
    let mut chunk_positions: Vec<u64> = Vec::with_capacity(pairs * 2);
    for index in 0..pairs {
        chunk_positions.push(b.len() as u64);
        b.extend_from_slice(b"00dc");
        b.extend_from_slice(&(video_payload as u32).to_le_bytes());
        b.extend(std::iter::repeat_n(
            0xa0 | (index % 16) as u8,
            video_payload,
        ));
        chunk_positions.push(b.len() as u64);
        b.extend_from_slice(b"01wb");
        b.extend_from_slice(&(audio_payload as u32).to_le_bytes());
        b.extend(std::iter::repeat_n(
            0x50 | (index % 16) as u8,
            audio_payload,
        ));
    }
    let movi_size = b.len() - movi_size_pos - 4;
    b[movi_size_pos..movi_size_pos + 4].copy_from_slice(&(movi_size as u32).to_le_bytes());
    // idx1: one keyframe entry per chunk, movi-relative.
    b.extend_from_slice(b"idx1");
    b.extend_from_slice(&((chunk_positions.len() * 16) as u32).to_le_bytes());
    for (index, &chunk_pos) in chunk_positions.iter().enumerate() {
        let tag: &[u8; 4] = if index % 2 == 0 { b"00dc" } else { b"01wb" };
        let size = if index % 2 == 0 {
            video_payload
        } else {
            audio_payload
        };
        b.extend_from_slice(tag);
        b.extend_from_slice(&AVIIF_KEYFRAME.to_le_bytes());
        b.extend_from_slice(&((chunk_pos - movi_form_pos) as u32).to_le_bytes());
        b.extend_from_slice(&(size as u32).to_le_bytes());
    }
    let riff_size = b.len() - 8;
    b[4..8].copy_from_slice(&(riff_size as u32).to_le_bytes());
    b.into()
}

/// The two-stream lane shape (W5-14): a real `AviDemuxer` over an
/// in-memory video+audio AVI — uniform payloads per stream, so the
/// pool's two slots serve both sizes in the steady state — walked
/// through the public contract with the idx1 keyframe cursor live.
/// After warm-up, **1000 consecutive packet events allocate nothing**:
/// this pins the lanes restructure (per-ordinal lane lookups, the
/// cached per-stream `TimeBase`) as allocation-neutral on the hot path.
#[test]
fn two_stream_interleaved_walk_allocates_nothing_per_packet() {
    let _serial = serialize_tests();
    const PAIRS: usize = 550;
    const VIDEO: usize = 512;
    const AUDIO: usize = 256;
    let delta = clean_replay(
        || {
            let bytes = avi_bytes_av(PAIRS, VIDEO, AUDIO);
            let mut avi = demux::avi::AviDemuxer::new(
                demux::avi::test_support::mem_stream(std::sync::Arc::clone(&bytes)).must(),
            )
            .must();
            let tracks = avi.start().must();
            assert_eq!(tracks.len(), 2, "video + audio tracks");

            // Warm-up: pool growth to its bound (both sizes), queue
            // seeding, one-time machinery.
            let mut seen = 0usize;
            while seen < 50 {
                match avi.next_event().must() {
                    plugin_api::DemuxEvent::Packet(packet) => {
                        drop(packet);
                        seen += 1;
                    }
                    plugin_api::DemuxEvent::EndOfStream => {
                        unreachable!("warm-up hit EOS at {seen}")
                    }
                    _ => {}
                }
            }

            let guard = ALLOC.state().guard();
            let mut walked = 0usize;
            while walked < 1000 {
                match avi.next_event().must() {
                    plugin_api::DemuxEvent::Packet(packet) => {
                        // The measured window starts after the warm-up's
                        // `seen` packets — the chunk index is global.
                        let global = seen + walked;
                        let (want_len, want_track, fill) = if global.is_multiple_of(2) {
                            (VIDEO, 0u32, 0xa0 | ((global / 2) % 16) as u8)
                        } else {
                            (AUDIO, 1u32, 0x50 | ((global / 2) % 16) as u8)
                        };
                        assert_eq!(packet.data.len(), want_len, "packet {walked}");
                        assert_eq!(packet.track.handle(), want_track, "packet {walked}");
                        assert!(
                            packet.data.as_slice().iter().all(|&byte| byte == fill),
                            "packet {walked}: payload bytes are the chunk's own"
                        );
                        drop(packet);
                        walked += 1;
                    }
                    plugin_api::DemuxEvent::EndOfStream => {
                        unreachable!("EOS at {walked} of 1000 with {PAIRS} pairs")
                    }
                    _ => {}
                }
            }
            let delta = guard.delta();
            // The stream still ends cleanly afterwards (outside the
            // measured window).
            let mut ended = false;
            while !ended {
                ended = matches!(avi.next_event().must(), plugin_api::DemuxEvent::EndOfStream);
            }
            delta
        },
        is_zero,
    );
    assert_eq!(
        (delta.alloc_calls, delta.total_bytes),
        (0, 0),
        "1000 packets through the two-stream lane walk — zero allocs"
    );
}

/// The idx1 batch proof (design E3(c)): parsing a 1024-entry index
/// through the block reader allocates a **constant-plus-log** count —
/// the 4 KiB block buffer (1), the raw-entry Vec's doubling ladder,
/// the final entries Vec — NOT one allocation per entry (the
/// pre-3.B3 shape: 1024 sixteen-byte reads, 1024 allocations). The
/// 256-vs-1024 comparison pins the growth as the Vec ladder alone.
#[test]
fn idx1_parse_allocates_per_block_not_per_entry() {
    let _serial = serialize_tests();
    let (small, large) = clean_replay(
        || {
            let parse = |entries: usize| -> u64 {
                let bytes = avi_bytes(entries, 512, true);
                let mut reader = demux::avi::riff::SourceReader::new(
                    demux::avi::test_support::mem_stream(std::sync::Arc::clone(&bytes)).must(),
                )
                .must();
                // The idx1 chunk sits after the movi list: locate it by
                // scanning for the fourcc (the file is ours; a direct find is
                // the honest test-side locator).
                let hay = bytes.as_ref();
                let idx1_at = hay.windows(4).position(|window| window == b"idx1").must();
                let idx1_pos = idx1_at as u64;
                let declared = (entries * 16) as u32;
                // movi geometry (the builder's own arithmetic).
                let movi_form = (hay.windows(4).position(|w| w == b"movi").must()) as u64;
                let movi_data_start = movi_form + 4;
                // The movi list's data ends exactly where the idx1 chunk
                // begins (the builder appends idx1 directly after movi).
                let movi_data_end = idx1_pos;
                let mut damage = DamageLog::default();
                // One video stream (the builder's own strl): every entry is
                // timeable, so the ladder pass runs too — the measured window
                // is the WHOLE parse, not just the read loop.
                let streams = [StreamDesc {
                    ordinal: 0,
                    kind: MediaKind::Video,
                    handler: FourCC::tag(b"MJPG"),
                    scale: 1,
                    rate: 30,
                    start: 0,
                    length: entries as u32,
                    sample_size: 0,
                    format: StreamFormat::MotionJpeg {
                        width: 320,
                        height: 240,
                    },
                    title: None,
                    strh_offset: 0,
                }];
                let guard = ALLOC.state().guard();
                let table = parse_idx1(
                    &mut reader,
                    idx1_pos,
                    declared,
                    movi_form,
                    movi_data_start,
                    movi_data_end,
                    bytes.len() as u64,
                    &streams,
                    &[], // no declared-but-filtered ordinals in this fixture
                    &mut damage,
                )
                .must();
                let delta = guard.delta();
                // Every entry is a valid in-movi keyframe chunk of a known
                // stream: the table times all of them and no damage is noted —
                // the parse is fully exercised inside the measured window.
                let index = table.must();
                assert_eq!(index.entries.len(), entries);
                assert_eq!(damage.total, 0, "a clean index notes no damage");
                delta.alloc_calls
            };
            (parse(256), parse(1024))
        },
        |(small, large): &(u64, u64)| *large <= *small + 4 && *large < 64,
    );
    // The growth from 256 to 1024 entries is the raw Vec's doubling
    // ladder (2 extra doublings) — everything else is constant.
    assert!(
        large <= small + 4,
        "256 entries: {small} allocs; 1024 entries: {large} allocs — \
         growth must be the Vec ladder, not per-entry reads"
    );
    // And the absolute shape: per-block reads (1024 entries = 4 KiB
    // blocks = 4 reads) plus the ladder lands far below the old
    // per-entry 1024.
    assert!(
        large < 64,
        "1024 entries parsed with {large} allocs — the pre-3.B3 chain spent 1024 (one per entry)"
    );
}

// ---------------------------------------------------------------------------
// Card 7-c (V-1 + the input read path): the per-payload source-call
// count. `READ_STEP_MAX` used to chop every payload read into 64 KiB
// source calls — a multi-hundred-KB MJPEG frame paid ceil(want / 64 KiB)
// read_into calls through the seam (and, on the real file path, one
// buffered window refill + one window→slot copy per call). The borrow
// seam's destination is caller-owned memory (the payload pool's arena
// slot), so the step loop bounds nothing the caller has not already
// bounded; the payload ask is now one read_into of the full `want`.
// ---------------------------------------------------------------------------

/// The seam-call census instrument (shared counters — the source moves
/// into the demuxer's `Box`, the test keeps the handles): counts
/// `read_into` calls, bytes served, the largest ask received, and the
/// payload-class asks (asks over the `BIG_ASK` threshold — the open
/// phase's header/format reads are all ≤ 56 B). This is the
/// syscall-count proxy for the in-memory walk; the real-file syscall
/// count is the input crate's `large_read_bypasses_the_window`
/// proof.
const BIG_ASK: usize = 1024;

#[derive(Default)]
struct SeamStats {
    calls: std::sync::atomic::AtomicU64,
    bytes: std::sync::atomic::AtomicU64,
    max_ask: std::sync::atomic::AtomicU64,
    big_asks: std::sync::atomic::AtomicU64,
}

impl SeamStats {
    fn record(&self, ask: usize, got: u64) {
        use std::sync::atomic::Ordering::Relaxed;
        self.calls.fetch_add(1, Relaxed);
        self.bytes.fetch_add(got, Relaxed);
        self.max_ask.fetch_max(ask as u64, Relaxed);
        if ask > BIG_ASK {
            self.big_asks.fetch_add(1, Relaxed);
        }
    }
}

/// An in-memory source that counts every `read_into` it serves.
struct CountingSource {
    inner: BorrowMemSource,
    stats: std::sync::Arc<SeamStats>,
}

impl SourceStream for CountingSource {
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
        self.inner.read(len)
    }
    fn read_into(&mut self, out: &mut [u8]) -> Result<usize, EngineError> {
        let got = self.inner.read_into(out)?;
        self.stats.record(out.len(), got as u64);
        Ok(got)
    }
    fn seek(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError> {
        self.inner.seek(origin)
    }
    fn position(&self) -> SourcePosition {
        self.inner.position()
    }
}

/// The payload-class ask is one source call per frame (7-c): a real
/// `AviDemuxer` walk over 6 uniform 1.5 MiB `00dc` chunks issues
/// exactly one `read_into` per payload — of the FULL frame size — plus
/// the 8-byte header read per chunk. Before 7-c the payload read was
/// chopped into `ceil(want / 64 KiB)` = 24 seam calls per frame: red on
/// main @ 3f04786 the census read **164 calls** (14 open-phase reads +
/// 150 walk calls = 6 × (24 chopped payload steps + 1 header)); after
/// 7-c it reads **26** (the same 14 open-phase reads + 6 full payload
/// asks + 6 header asks).
///
/// The original pin (5bc51ef) asserted a total of 12, assuming the
/// open phase's reads (RIFF head, `hdrl` peeks, `avih`/`strh`/`strf`
/// payloads, `movi` discovery — every one of them routes through the
/// card-3.B3 borrow seam, all asks ≤ 56 B) were Vec reads. They are
/// not, and making them Vec reads would be a regression (an allocation
/// per open read). The amended census pins the same contract where it
/// matters, more precisely: **exactly one payload-class ask per
/// frame**, of the full frame size, with the open phase's fixed 14
/// small reads accounted by name.
#[test]
fn big_payload_reads_are_one_source_call_per_frame() {
    let _serial = serialize_tests();
    const CHUNKS: usize = 6;
    const PAYLOAD: usize = 1536 * 1024; // 1.5 MiB — the frame class
    /// The open-phase `read_into` calls for this fixture (measured:
    /// RIFF head 12 B, `LIST hdrl` peek + form, `avih` peek + 56 B,
    /// `LIST strl` peek + form, `strh` peek + 56 B, `strf` peek +
    /// 40 B, `LIST movi` peek + form — 14 small asks, none over
    /// 56 B, none per-payload).
    const OPEN_READS: u64 = 14;
    let bytes = avi_bytes(CHUNKS, PAYLOAD, false);
    let stats = std::sync::Arc::new(SeamStats::default());
    let source = CountingSource {
        inner: BorrowMemSource::new(std::sync::Arc::clone(&bytes)),
        stats: std::sync::Arc::clone(&stats),
    };
    let mut avi = demux::avi::AviDemuxer::new(Box::new(source)).must();
    let tracks = avi.start().must();
    assert_eq!(tracks.len(), 1);

    let mut packets = 0usize;
    loop {
        match avi.next_event().must() {
            plugin_api::DemuxEvent::Packet(packet) => {
                assert_eq!(packet.data.len(), PAYLOAD, "packet {packets}");
                packets += 1;
            }
            plugin_api::DemuxEvent::EndOfStream => break,
            _ => {}
        }
    }
    assert_eq!(packets, CHUNKS);
    use std::sync::atomic::Ordering::Relaxed;
    // The payload census: exactly one payload-class ask per packet —
    // the contract the pin exists for. A chopped payload (the 64 KiB
    // step loop) shows up here immediately as 24 asks per frame.
    assert_eq!(
        stats.big_asks.load(Relaxed),
        CHUNKS as u64,
        "one payload-class ask per frame — every extra big ask is a chopped \
         payload (main @ 3f04786: 6 x 24 = 144)"
    );
    // The full census, open phase accounted by name: OPEN_READS small
    // open asks + one header ask and one full payload ask per chunk.
    assert_eq!(
        stats.calls.load(Relaxed),
        OPEN_READS + (CHUNKS * 2) as u64,
        "14 open-phase reads (≤ 56 B, fixed) + 6 payload reads (one per \
         frame, full ask) + 6 header reads (8 B each) — every extra call \
         is a chopped payload"
    );
    assert_eq!(
        stats.max_ask.load(Relaxed),
        PAYLOAD as u64,
        "the payload ask is the full frame size, not a 64 KiB step"
    );
}
