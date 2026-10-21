//! Wire-protocol integration suite (card 2.D3): the protocol across
//! module boundaries — the renderer's composited output as a
//! streamable record, the bridge's stream over varied sizes, stream
//! diagnostics — everything the canvas-side reference implementation
//! (`prototype/lumen-frames.mjs`) mirrors, proven here on the Rust
//! side first.

mod common;

use common::{marked_request, payload_marker};
use desktop::wire::{FormatCode, FrameHeader, WireDecoder, encode};
use desktop::{CanvasBridge, CaptureTransport};
use media_types::frame::FrameStorage;
use renderer_wgpu::Renderer;
use renderer_wgpu::color::SurfaceFormat;
use renderer_wgpu::params::Extent2D;
use renderer_wgpu::surface::{PresentMode, SurfaceConfig};
use testkit::Must;
use video_engine::Presenter;

/// The bridge's stream over varied sizes decodes back to the exact
/// input bytes, in order, with the repeat flags where the wiring put
/// them (60 frames — a 2-second 30 fps burst).
#[test]
fn bridge_stream_round_trips_sixty_frames_varied_sizes() {
    let extents = [
        Extent2D {
            width: 32,
            height: 24,
        },
        Extent2D {
            width: 64,
            height: 48,
        },
        Extent2D {
            width: 320,
            height: 180,
        },
    ];
    let transport = CaptureTransport::new();
    let mut bridge = CanvasBridge::new(transport);

    let mut sources = Vec::new();
    for i in 0..60u64 {
        let extent = extents[i as usize % extents.len()];
        let pts_ns = (i as i64) * 33_333_334;
        let request = marked_request(extent, pts_ns, (i % 256) as u8);
        let bytes = match &request.frame.data {
            FrameStorage::Cpu(bytes) => bytes.as_slice().to_vec(),
            FrameStorage::OpaqueHandle(_) => Vec::new(),
        };
        sources.push((extent, i, bytes, pts_ns));
        // Every 7th delivery is the wiring's RepeatLast path.
        if i > 0 && i % 7 == 0 {
            bridge.present_repeat(&request).must();
        } else {
            bridge.present(&request).must();
        }
    }

    let records = bridge.transport().records().to_vec();
    assert_eq!(records.len(), 60);
    let mut decoder = WireDecoder::new();
    for record in &records {
        decoder.feed(record);
    }
    for (expected_index, (extent, i, bytes, pts_ns)) in sources.iter().enumerate() {
        let frame = decoder.next_frame().must().must();
        assert_eq!(frame.header.frame_id, *i, "monotonic ids");
        assert_eq!(frame.header.pts_ns, *pts_ns);
        assert_eq!(
            (frame.header.width, frame.header.height),
            (extent.width, extent.height)
        );
        assert_eq!(frame.payload, *bytes, "byte-exact payload for frame {i}");
        // Content identity: the marker pixel names the frame.
        assert_eq!(payload_marker(*extent, &frame.payload)[0], (*i % 256) as u8);
        assert_eq!(
            frame.header.flags.repeat,
            expected_index > 0 && expected_index % 7 == 0,
            "repeat flag exactly where the wiring put it"
        );
        let _ = FormatCode::from_code(frame.header.format.code()).must();
    }
    assert!(decoder.next_frame().must().is_none());
    decoder.finish().must();
    // Accounting: every byte shipped is accounted.
    let stats = bridge.stats();
    assert_eq!(stats.frames_shipped, 60);
    assert_eq!(stats.repeats, 8); // i = 7, 14, ..., 56
    assert_eq!(
        stats.bytes_shipped,
        records.iter().map(Vec::len).sum::<usize>() as u64
    );
}

/// Variant B's composited shape: what the 2.D1 renderer put on the
/// surface is streamable and recovers byte-exact — the engine-side
/// half of "stream the presented pixels" with the REAL renderer in
/// the loop (letterboxed placement included).
#[test]
fn renderer_composited_output_streams_byte_exact() {
    let surface_extent = Extent2D {
        width: 64,
        height: 48,
    };
    let mut renderer = Renderer::new_headless(
        surface_extent,
        SurfaceConfig {
            format: SurfaceFormat::Bgra8Srgb,
            present_mode: PresentMode::Fifo,
        },
    )
    .must();
    // A 32x24 synthetic frame letterboxes into the 64x48 surface.
    let request = marked_request(
        Extent2D {
            width: 32,
            height: 24,
        },
        1_000_000,
        42,
    );
    renderer.present(&request).must();
    let composited = renderer.readback_rgba().must();
    assert_eq!(composited.len(), 64 * 48 * 4);

    // The composited surface becomes ONE record on the wire.
    let header = FrameHeader {
        format: FormatCode::Rgba8,
        flags: desktop::wire::FrameFlags::default(),
        width: surface_extent.width,
        height: surface_extent.height,
        payload_len: (composited.len() as u64).try_into().must(),
        frame_id: 0,
        pts_ns: 1_000_000,
        emitted_ns: 0,
    };
    let record = encode(&header, &composited).must();
    assert_eq!(record.len(), 48 + 64 * 48 * 4);
    let mut decoder = WireDecoder::new();
    decoder.feed(&record);
    let frame = decoder.next_frame().must().must();
    assert_eq!(
        frame.payload, composited,
        "composited pixels recover byte-exact"
    );
    decoder.finish().must();
}

/// A 1080p frame — the doc 08 §7.2 measurement size class — round
/// trips with exact accounting (the header is 48 of 8 294 448
/// bytes).
#[test]
fn full_hd_frame_round_trips() {
    let extent = Extent2D {
        width: 1920,
        height: 1080,
    };
    let request = marked_request(extent, 0, 7);
    let bytes = match &request.frame.data {
        FrameStorage::Cpu(bytes) => bytes.as_slice().to_vec(),
        FrameStorage::OpaqueHandle(_) => Vec::new(),
    };
    assert_eq!(bytes.len(), 1920 * 1080 * 4);
    let transport = CaptureTransport::new();
    let mut bridge = CanvasBridge::new(transport);
    bridge.present(&request).must();
    let records = bridge.transport().records().to_vec();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].len(), 48 + 1920 * 1080 * 4);
    let mut decoder = WireDecoder::new();
    decoder.feed(&records[0]);
    let frame = decoder.next_frame().must().must();
    assert_eq!(frame.payload, bytes);
    assert_eq!(bridge.stats().bytes_shipped, (48 + 1920 * 1080 * 4) as u64);
}

/// Stream diagnostics: a corrupt record's offset is in fed-stream
/// coordinates (after N consumed records), not per-chunk.
#[test]
fn bad_magic_reports_stream_offset_after_consumed_records() {
    let small = marked_request(
        Extent2D {
            width: 64,
            height: 48,
        },
        0,
        0,
    );
    let transport = CaptureTransport::new();
    let mut bridge = CanvasBridge::new(transport);
    // Two distinct presents (frame ids 0 and 1), then a corrupt one.
    bridge.present(&small).must();
    let second = marked_request(
        Extent2D {
            width: 64,
            height: 48,
        },
        33_333_334,
        1,
    );
    bridge.present(&second).must();
    let records = bridge.transport().records().to_vec();
    assert_eq!(records.len(), 2);
    let mut stream = records[0].clone();
    stream.extend(records[1].clone());
    // 48+ bytes of garbage: enough for the decoder to attempt a
    // header parse and refuse the magic (shorter garbage would just
    // be "need more input").
    stream.extend(b"XXXX rest is garbage padded out past the 48-byte header window");
    let mut decoder = WireDecoder::new();
    decoder.feed(&stream);
    let a = decoder.next_frame().must().must();
    let b = decoder.next_frame().must().must();
    assert_eq!(a.header.frame_id, 0);
    assert_eq!(b.header.frame_id, 1);
    let err = decoder.next_frame().err().must();
    assert!(
        matches!(&err, desktop::WireError::BadMagic { offset, .. } if *offset == (records[0].len() + records[1].len()) as u64),
        "offset must be in stream coordinates: {err:?}"
    );
}
