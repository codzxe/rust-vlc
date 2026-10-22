//! Zero-allocation proof for the desktop frame path (audit row C7,
//! ED-FIX-2 completion — W2-03): after one warm-up cycle, the
//! per-frame transport path allocates **zero bytes**, measured by the
//! counting global allocator (the `zero_alloc_null_sink.rs` /
//! `zero_alloc_quantum.rs` discipline).
//!
//! # What is measured
//!
//! Two windows, one per decode entry point (the allocator is
//! process-global; whole-binary serialization via
//! [`testkit::alloc_count::serialize_tests`]):
//!
//! 1. **The mailbox hand-off** — the production shape: bridge
//!    `present` → serialize into the reused scratch buffer →
//!    `MailboxTransport::send` → `WireDecoder::decode_record` (ONE
//!    decoder for the transport's whole life, slice-window header
//!    parse, one copy into a pooled payload buffer) → mailbox offer →
//!    consumer draw → drop (buffer returns to the pool). The old path
//!    built a fresh `WireDecoder` per `send` and paid a full-frame
//!    `to_vec` + an O(n) `drain` per record — 2 allocs + 2 copies on
//!    the frame path.
//! 2. **The streaming decoder** — `feed` + `next_frame` over N
//!    records with the consumer dropping each frame: the cursor ring
//!    resets in place and the pooled payload cycles, so the steady
//!    state allocates nothing.
//!
//! Frame construction, transport construction and the warm-up cycle
//! happen **before** the guards — the steady-state frame path is the
//! claim, and the steady-state frame path is what is measured.

#[global_allocator]
static ALLOC: testkit::alloc_count::CountingGlobalAlloc =
    testkit::alloc_count::CountingGlobalAlloc::new();

mod common;

use common::marked_request;
use desktop::mailbox::MailboxConfig;
use desktop::transport::MailboxTransport;
use desktop::wire::WireDecoder;
use desktop::{CanvasBridge, EmitClock};
use renderer_wgpu::params::Extent2D;
use testkit::Must;
use testkit::alloc_count::serialize_tests;
use video_engine::Presenter;

/// The suite's frame extent (64x48, the pipeline suite's shape).
const EXTENT: Extent2D = Extent2D {
    width: 64,
    height: 48,
};

/// 30 fps spacing (varies the records so nothing can "pass" by
/// memoizing identical inputs).
const FRAME_30: i64 = 33_333_334;

const CYCLES: usize = 256;

/// A fixed virtual emit clock (deterministic stamps, no clocks inside
/// the measured window).
struct FixedClock(u64);

impl EmitClock for FixedClock {
    fn emit_ns(&self) -> u64 {
        self.0
    }
}

#[test]
fn mailbox_frame_path_is_zero_allocation_after_warm_up() {
    let _serial = serialize_tests();

    // Setup (allocates freely, outside every measured window).
    let transport = MailboxTransport::new(MailboxConfig::default()).must();
    let mut bridge = CanvasBridge::with_clock(transport, Box::new(FixedClock(1_000)));
    let mut request = marked_request(EXTENT, 0, 42);

    // Warm-up: one full present→draw cycle sizes the bridge's encode
    // scratch, the decoder's payload pool and the mailbox's queue.
    bridge.present(&request).must();
    drop(bridge.transport_mut().mailbox_mut().draw_next().must());

    // The measured window: the production per-frame path, N cycles.
    let guard = ALLOC.state().guard();
    for i in 0..CYCLES as i64 {
        let pts = i * FRAME_30;
        request.when = media_types::time::Timestamp::new(pts, video_engine::NANO_BASE);
        bridge.present(&request).must();
        let frame = bridge.transport_mut().mailbox_mut().draw_next().must();
        // Content identity through the whole path: the record that
        // arrived is the record that was shipped.
        assert_eq!(frame.header.pts_ns, pts);
        assert_eq!(frame.header.frame_id, i as u64 + 1);
        // The consumer drops the frame: the payload buffer returns
        // to the pool for the next cycle.
        drop(frame);
    }
    guard.assert_zero(
        "desktop mailbox frame path: present -> serialize -> send -> \
         decode_record -> offer -> draw over 256 cycles",
    );
}

#[test]
fn streaming_decoder_path_is_zero_allocation_after_warm_up() {
    let _serial = serialize_tests();

    // Setup: one encoded record (the encode path allocates by design
    // — `encode` is the allocating convenience wrapper; it runs here,
    // outside the window).
    let header = desktop::wire::FrameHeader {
        format: desktop::wire::FormatCode::Rgba8,
        flags: desktop::wire::FrameFlags::default(),
        width: EXTENT.width,
        height: EXTENT.height,
        payload_len: EXTENT.width * EXTENT.height * 4,
        frame_id: 7,
        pts_ns: 1_000_000,
        emitted_ns: 1_700_000_000_000_000_000,
    };
    let payload = common::marked_bytes(EXTENT, 7);
    let record = desktop::wire::encode(&header, &payload).must();

    let mut decoder = WireDecoder::new();
    // Warm-up: one feed→decode→drop cycle sizes the pending buffer
    // and the payload pool.
    decoder.feed(&record);
    drop(decoder.next_frame().must().must());

    // The measured window: the chunked-consumer shape (what a real
    // byte-transport host does per message), N records.
    let guard = ALLOC.state().guard();
    for _ in 0..CYCLES {
        decoder.feed(&record);
        let frame = decoder.next_frame().must().must();
        assert_eq!(frame.header.frame_id, 7);
        assert_eq!(frame.payload, payload);
        drop(frame);
    }
    guard.assert_zero("wire streaming decode: feed + next_frame over 256 records");
}
