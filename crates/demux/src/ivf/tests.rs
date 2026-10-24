//! The IVF unit suite (card 5.I3b): header round-trips, the frame
//! walk, every typed error path, seek/reanchor, and the `+FB`
//! fuzz-lite sweeps.

use std::sync::Arc;

use media_types::Duration;
use media_types::time::Stamp;
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};
use testkit::Must;

use super::test_support::{demo_stream, ivf_frame, ivf_header, mem_stream};
use super::{FRAME_CEILING_BYTES, HEADER_BYTES, IvfDemuxer, IvfHeader, probe};

/// Drain to end, collecting (packets, recovered, discontinuities,
/// terminal error code).
struct Drain {
    packets: Vec<media_types::Packet>,
    recovered: Vec<(u64, &'static str)>,
    discontinuities: usize,
    terminal: Option<String>,
}

fn drain(demuxer: &mut IvfDemuxer) -> Drain {
    let mut out = Drain {
        packets: Vec::new(),
        recovered: Vec::new(),
        discontinuities: 0,
        terminal: None,
    };
    loop {
        match demuxer.next_event() {
            Ok(DemuxEvent::Packet(p)) => out.packets.push(p),
            Ok(DemuxEvent::Recovered { skipped, reason }) => out.recovered.push((skipped, reason)),
            Ok(DemuxEvent::Discontinuity) => out.discontinuities += 1,
            Ok(DemuxEvent::EndOfStream) => break,
            Err(err) => {
                out.terminal = Some(err.context.code.clone());
                break;
            }
        }
    }
    out
}

fn open(bytes: impl Into<Arc<[u8]>>) -> IvfDemuxer {
    let mut demuxer = IvfDemuxer::new(mem_stream(bytes).must()).must();
    demuxer.start().must();
    demuxer
}

// -- header parsing --------------------------------------------------------

#[test]
fn header_parse_roundtrips_the_field_set() {
    // The exact header ffmpeg 7.1.5 writes for a 64×48 30 fps AV1
    // delivery (verified by hexdump on the committed fixture).
    let bytes = ivf_header(b"AV01", 64, 48, 30, 1, 30);
    let head = IvfHeader::parse(&bytes).must();
    assert_eq!(head.version, 0);
    assert_eq!(head.header_len, 32);
    assert_eq!(head.fourcc, *b"AV01");
    assert_eq!((head.width, head.height), (64, 48));
    assert_eq!((head.rate, head.scale), (30, 1));
    assert_eq!(head.frame_count, 30);
    assert_eq!(head.codec(), Some("av1"));
    assert_eq!(head.descriptor().must().as_str(), "av1");
    // One tick = scale/rate = 1/30 s.
    let base = head.tick_base().must();
    assert_eq!((base.num, base.den), (1, 30));
    assert_eq!(head.tick_nanos(), Some(1_000_000_000 / 30));

    let vp9 = IvfHeader::parse(&ivf_header(b"VP90", 64, 48, 25, 1, 25)).must();
    assert_eq!(vp9.codec(), Some("vp9"));
    assert_eq!(vp9.tick_base().must().den, 25);

    // An unsupported fourcc parses (the refusal is the demuxer's
    // policy, so the error can name the tag).
    let vp8 = IvfHeader::parse(&ivf_header(b"VP80", 64, 48, 30, 1, 1)).must();
    assert_eq!(vp8.codec(), None);
    assert!(vp8.descriptor().is_none());
}

#[test]
fn probe_detects_dkif_and_rejects_the_rest() {
    assert!(probe(b"DKIF\x00\x00\x20\x00"));
    assert!(!probe(b"DKI\x00"));
    assert!(!probe(b"RIFF"));
    assert!(!probe(b""));
}

#[test]
fn header_refuses_short_buffers_typed() {
    for len in [0usize, 4, 16, 31] {
        let err = IvfHeader::parse(&vec![0u8; len]).err().must();
        assert_eq!(err.context.code, "demux.ivf.truncated", "len {len}");
    }
}

#[test]
fn header_refuses_bad_magic_typed() {
    let mut bytes = ivf_header(b"AV01", 64, 48, 30, 1, 1);
    bytes[0] = b'X';
    let err = IvfHeader::parse(&bytes).err().must();
    assert_eq!(err.context.code, "demux.ivf.not-ivf");
}

#[test]
fn header_refuses_structural_lies_typed() {
    // Header length other than 32.
    let mut bytes = ivf_header(b"AV01", 64, 48, 30, 1, 1);
    bytes[6] = 64;
    let err = IvfHeader::parse(&bytes).err().must();
    assert_eq!(err.context.code, "demux.ivf.bad-header");
    // Zero dims.
    for at in [12usize, 14] {
        let mut bytes = ivf_header(b"AV01", 64, 48, 30, 1, 1);
        bytes[at..at + 2].copy_from_slice(&0u16.to_le_bytes());
        let err = IvfHeader::parse(&bytes).err().must();
        assert_eq!(err.context.code, "demux.ivf.bad-header", "dim at {at}");
    }
    // Zero rate / scale (ffmpeg's "Invalid frame rate" refusal).
    for at in [16usize, 20] {
        let mut bytes = ivf_header(b"AV01", 64, 48, 30, 1, 1);
        bytes[at..at + 4].copy_from_slice(&0u32.to_le_bytes());
        let err = IvfHeader::parse(&bytes).err().must();
        assert_eq!(err.context.code, "demux.ivf.bad-header", "rate at {at}");
    }
}

// -- the clean walk ---------------------------------------------------------

#[test]
fn clean_walk_passes_the_container_truth() {
    let mut bytes = ivf_header(b"AV01", 64, 48, 30, 1, 3);
    for (pts, len) in [(0u64, 8usize), (1, 12), (2, 4)] {
        bytes.extend_from_slice(&ivf_frame(pts, len));
    }
    let mut demuxer = open(bytes);
    let info = demuxer.stream_info().must();
    assert_eq!(
        (
            info.format,
            info.codec,
            info.width,
            info.height,
            info.rate,
            info.scale
        ),
        ("ivf", "av1", 64, 48, 30, 1)
    );
    assert_eq!(info.fourcc, *b"AV01");
    let out = drain(&mut demuxer);
    assert_eq!(out.packets.len(), 3);
    assert!(out.terminal.is_none());
    assert!(out.recovered.is_empty());
    // The container's own pts, unrenumbered; one declared tick each.
    let expected_pts = [0i64, 1, 2];
    let expected_first = [0u8, 1, 2, 3, 4, 5, 6, 7];
    for (i, packet) in out.packets.iter().enumerate() {
        let Stamp::Present(ts) = packet.pts else {
            unreachable!("pts is present");
        };
        assert_eq!(ts.ticks, expected_pts[i]);
        assert_eq!((ts.base.num, ts.base.den), (1, 30));
        assert_eq!(
            packet.duration,
            Some(Duration::Known(media_types::Nanoseconds(
                1_000_000_000 / 30
            )))
        );
        // The honesty pair: IVF declares neither.
        assert!(!packet.keyframe);
        assert_eq!(packet.dts, Stamp::Missing);
        if i == 0 {
            assert_eq!(&packet.data[..], &expected_first);
        }
        assert_eq!(packet.data.len(), [8usize, 12, 4][i]);
    }
    assert_eq!(demuxer.frames_demuxed(), 3);
    assert_eq!(demuxer.damage_total(), 0);
    // Duration = (max pts + 1) × tick.
    let Duration::Known(ns) = demuxer.track_duration().must() else {
        unreachable!("duration is known");
    };
    assert_eq!(ns.0, 3 * (1_000_000_000 / 30));
}

#[test]
fn walk_passes_vfr_gaps_and_zero_size_frames_through() {
    let mut bytes = ivf_header(b"VP90", 32, 16, 1000, 1, 4);
    bytes.extend_from_slice(&ivf_frame(0, 4));
    bytes.extend_from_slice(&ivf_frame(0, 0)); // zero-size frame: emitted
    bytes.extend_from_slice(&ivf_frame(5, 4)); // VFR gap: passed through
    let mut demuxer = open(bytes);
    let out = drain(&mut demuxer);
    assert_eq!(out.packets.len(), 3);
    let ticks: Vec<i64> = out
        .packets
        .iter()
        .map(|p| match p.pts {
            Stamp::Present(t) => t.ticks,
            Stamp::Missing => unreachable!("pts present"),
        })
        .collect();
    assert_eq!(ticks, vec![0, 0, 5]);
    assert_eq!(out.packets[1].data.len(), 0);
    // Tick = 1/1000 s here (ffmpeg's ms-resolution IVF shape).
    assert_eq!(
        out.packets[0].duration,
        Some(Duration::Known(media_types::Nanoseconds(1_000_000)))
    );
}

#[test]
fn declared_frame_count_is_advisory_the_walk_is_truth() {
    // Declares 99 frames, carries 2 (some writers leave the field
    // stale; the demuxer neither trusts nor needs it).
    let mut bytes = ivf_header(b"AV01", 8, 8, 30, 1, 99);
    bytes.extend_from_slice(&ivf_frame(0, 4));
    bytes.extend_from_slice(&ivf_frame(1, 4));
    let mut demuxer = open(bytes);
    assert_eq!(demuxer.stream_info().must().declared_frames, 99);
    let out = drain(&mut demuxer);
    assert_eq!(out.packets.len(), 2);
    assert_eq!(demuxer.frames_demuxed(), 2);
}

#[test]
fn trailing_fragment_is_recovered_then_clean_end() {
    // ffmpeg's own walk ends cleanly on a sub-header tail; ours notes
    // the bounded skip and ends cleanly too (no short packet).
    let mut bytes = ivf_header(b"AV01", 8, 8, 30, 1, 2);
    bytes.extend_from_slice(&ivf_frame(0, 4));
    bytes.extend_from_slice(&[0xEE; 5]);
    let mut demuxer = open(bytes);
    let out = drain(&mut demuxer);
    assert_eq!(out.packets.len(), 1);
    assert!(out.terminal.is_none());
    assert_eq!(out.recovered, vec![(5, "ivf: trailing fragment")]);
    assert_eq!(demuxer.damage_total(), 5);
}

// -- typed error paths ------------------------------------------------------

#[test]
fn start_refuses_every_malformed_shape_typed() {
    let cases: Vec<(Vec<u8>, &str)> = vec![
        (
            ivf_header(b"AV01", 8, 8, 30, 1, 1)[..24].to_vec(),
            "demux.ivf.truncated",
        ), // cut header
        (ivf_header(b"AV01", 8, 8, 30, 1, 0), "demux.ivf.no-frames"), // header only
        (
            ivf_header(b"VP80", 8, 8, 30, 1, 1),
            "demux.ivf.unsupported-fourcc",
        ),
        (
            ivf_header(b"H264", 8, 8, 30, 1, 1),
            "demux.ivf.unsupported-fourcc",
        ),
        (
            ivf_header(b"\x00\x00\x00\x00", 8, 8, 30, 1, 1),
            "demux.ivf.unsupported-fourcc",
        ),
    ];
    for (bytes, code) in cases {
        let mut demuxer = IvfDemuxer::new(mem_stream(bytes).must()).must();
        let err = demuxer.start().err().must();
        assert_eq!(err.context.code, code);
        // Sticky: the failure replays, the stream is never re-read.
        let again = demuxer.start().err().must();
        assert_eq!(again.context.code, code);
        assert_eq!(demuxer.next_event().err().must().context.code, code);
    }
}

#[test]
fn walk_refuses_frame_beyond_file_sticky() {
    let mut bytes = ivf_header(b"AV01", 8, 8, 30, 1, 2);
    bytes.extend_from_slice(&ivf_frame(0, 4));
    // Declares 500 payload bytes, carries 10 (ffmpeg emits a short
    // packet here — the recorded divergence; we refuse typed).
    let mut frame = Vec::new();
    frame.extend_from_slice(&500u32.to_le_bytes());
    frame.extend_from_slice(&1u64.to_le_bytes());
    frame.extend_from_slice(&[0x41; 10]);
    bytes.extend_from_slice(&frame);
    let mut demuxer = open(bytes);
    let out = drain(&mut demuxer);
    assert_eq!(out.packets.len(), 1);
    assert_eq!(
        out.terminal,
        Some("demux.ivf.frame-beyond-file".to_string())
    );
    // Sticky replay.
    assert_eq!(
        demuxer.next_event().err().must().context.code,
        "demux.ivf.frame-beyond-file"
    );
}

#[test]
fn walk_refuses_ceiling_sized_frames_before_allocating() {
    let over = u32::try_from(FRAME_CEILING_BYTES + 1).must();
    let mut bytes = ivf_header(b"AV01", 8, 8, 30, 1, 1);
    bytes.extend_from_slice(&ivf_frame(0, 4));
    let mut frame = Vec::new();
    frame.extend_from_slice(&over.to_le_bytes());
    frame.extend_from_slice(&1u64.to_le_bytes());
    bytes.extend_from_slice(&frame);
    let mut demuxer = open(bytes);
    let out = drain(&mut demuxer);
    assert_eq!(out.packets.len(), 1);
    assert_eq!(out.terminal, Some("demux.ivf.frame-too-large".to_string()));
    // The pool never grew toward the claim (MP §12: refused, not read).
    assert_eq!(demuxer.pool_bytes(), 4);
}

#[test]
fn walk_refuses_i64_impossible_pts() {
    let mut bytes = ivf_header(b"AV01", 8, 8, 30, 1, 1);
    let mut frame = Vec::new();
    frame.extend_from_slice(&4u32.to_le_bytes());
    frame.extend_from_slice(&u64::MAX.to_le_bytes()); // > i64::MAX
    frame.extend_from_slice(&[0x41; 4]);
    bytes.extend_from_slice(&frame);
    let mut demuxer = open(bytes);
    let out = drain(&mut demuxer);
    assert_eq!(out.packets.len(), 0);
    assert_eq!(out.terminal, Some("demux.ivf.pts-overflow".to_string()));
}

#[test]
fn trait_calls_before_start_refuse_typed() {
    let mut demuxer = IvfDemuxer::new(mem_stream(demo_stream()).must()).must();
    assert_eq!(
        demuxer.next_event().err().must().context.code,
        "demux.ivf.not-started"
    );
    assert_eq!(
        demuxer
            .seek(media_types::time::TimeSpan::UNKNOWN)
            .err()
            .must()
            .context
            .code,
        "demux.ivf.not-started"
    );
    assert_eq!(demuxer.seekability(), SeekSupport::None);
    // Empty sources never reach the container layer: the shared
    // access seam refuses them at open (the AVI/WAV/Ogg/ADTS
    // precedent — stage `Open`, not `Demux`).
    let err = IvfDemuxer::new(mem_stream(Vec::new()).must()).err().must();
    assert_eq!(err.context.code, "demux.source.empty");
}

// -- seek -------------------------------------------------------------------

#[test]
fn seek_lands_on_the_last_frame_at_or_before_the_target() {
    let mut bytes = ivf_header(b"AV01", 8, 8, 30, 1, 5);
    for pts in 0..5u64 {
        bytes.extend_from_slice(&ivf_frame(pts, 4));
    }
    let mut demuxer = open(bytes.clone());
    assert_eq!(demuxer.seekability(), SeekSupport::SlowLinear);
    let base = demuxer.header().must().tick_base().must();
    for (target, want) in [(0i64, 0i64), (2, 2), (3, 3), (4, 4)] {
        let span = demuxer
            .seek(media_types::time::TimeSpan {
                start: Stamp::Present(media_types::time::Timestamp::new(target, base)),
                end: Stamp::Missing,
            })
            .must();
        let Stamp::Present(ts) = span.start else {
            unreachable!("landing start is present");
        };
        assert_eq!(ts.ticks, want);
    }
    // A pure query: the walk position never moved (still frame 0).
    let out = drain(&mut demuxer);
    assert_eq!(out.packets.len(), 5);
    // Past the end: clamps to the last frame (ffmpeg's demuxer
    // semantics — nothing is invented past the container's truth).
    let span = demuxer
        .seek(media_types::time::TimeSpan {
            start: Stamp::Present(media_types::time::Timestamp::new(5, base)),
            end: Stamp::Missing,
        })
        .must();
    let Stamp::Present(ts) = span.start else {
        unreachable!("landing start is present");
    };
    assert_eq!(ts.ticks, 4);
    // Before the stream start: clamps to the first frame.
    let span = demuxer
        .seek(media_types::time::TimeSpan {
            start: Stamp::Present(media_types::time::Timestamp::new(-5, base)),
            end: Stamp::Missing,
        })
        .must();
    let Stamp::Present(ts) = span.start else {
        unreachable!("landing start is present");
    };
    assert_eq!(ts.ticks, 0);
    let _ = bytes;
}

#[test]
fn reanchor_resumes_the_walk_from_the_landing() {
    let mut bytes = ivf_header(b"VP90", 8, 8, 25, 1, 6);
    for pts in 0..6u64 {
        bytes.extend_from_slice(&ivf_frame(pts, 4));
    }
    let mut demuxer = open(bytes);
    let base = demuxer.header().must().tick_base().must();
    let span = demuxer
        .reanchor(media_types::time::TimeSpan {
            start: Stamp::Present(media_types::time::Timestamp::new(3, base)),
            end: Stamp::Missing,
        })
        .must();
    let Stamp::Present(ts) = span.start else {
        unreachable!("landing start is present");
    };
    assert_eq!(ts.ticks, 3);
    let out = drain(&mut demuxer);
    assert_eq!(out.discontinuities, 1);
    assert_eq!(out.packets.len(), 3); // frames 3, 4, 5
    let ticks: Vec<i64> = out
        .packets
        .iter()
        .map(|p| match p.pts {
            Stamp::Present(t) => t.ticks,
            Stamp::Missing => unreachable!("pts present"),
        })
        .collect();
    assert_eq!(ticks, vec![3, 4, 5]);
    // Reanchor to a pre-roll target (negative) clamps to frame 0.
    demuxer
        .reanchor(media_types::time::TimeSpan {
            start: Stamp::Present(media_types::time::Timestamp::new(-5, base)),
            end: Stamp::Missing,
        })
        .must();
    let out = drain(&mut demuxer);
    assert_eq!(out.packets.len(), 6);
}

#[test]
fn seek_out_of_range_only_when_no_frame_exists() {
    // A degenerate container: header + a sub-header tail (start
    // succeeds — there is a trailing fragment worth a Recovered), but
    // no complete frame exists to land on.
    let mut bytes = ivf_header(b"AV01", 8, 8, 30, 1, 1);
    bytes.extend_from_slice(&[0xEE; 5]);
    let mut demuxer = open(bytes);
    let base = demuxer.header().must().tick_base().must();
    let err = demuxer
        .seek(media_types::time::TimeSpan {
            start: Stamp::Present(media_types::time::Timestamp::new(0, base)),
            end: Stamp::Missing,
        })
        .err()
        .must();
    assert_eq!(err.context.code, "demux.ivf.seek.out-of-range");
}

// -- +FB fuzz-lite ----------------------------------------------------------

/// A tiny deterministic LCG (the crate's fuzz sweeps are seeded, so
/// failures replay bit-for-bit).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound.max(1)
    }
}

#[test]
fn fuzz_lite_bit_flips_never_panic_or_hang() {
    let seed = demo_stream();
    let mut lcg = Lcg(0x5EED_5EED);
    for round in 0..512usize {
        let mut bytes = (*seed).to_vec();
        let flips = 1 + lcg.below(8) as usize;
        for _ in 0..flips {
            let at = lcg.below(bytes.len() as u64) as usize;
            bytes[at] ^= 1u8 << lcg.below(8);
        }
        super::fuzz::fuzz_demux_ivf(&bytes);
        // The demuxer is deterministic: two runs of the same bytes
        // agree on the terminal code (no hidden state leaks).
        if round % 64 == 0 {
            let mut a = IvfDemuxer::new(mem_stream(bytes.clone()).must()).must();
            let mut b = IvfDemuxer::new(mem_stream(bytes.clone()).must()).must();
            let code_a = match (a.start(), a.next_event()) {
                (Err(e), _) => e.context.code.clone(),
                (Ok(_), Err(e)) => e.context.code.clone(),
                (Ok(_), Ok(_)) => String::new(),
            };
            let code_b = match (b.start(), b.next_event()) {
                (Err(e), _) => e.context.code.clone(),
                (Ok(_), Err(e)) => e.context.code.clone(),
                (Ok(_), Ok(_)) => String::new(),
            };
            assert_eq!(code_a, code_b);
        }
    }
}

#[test]
fn fuzz_lite_truncations_never_panic_or_hang() {
    let seed = demo_stream();
    for cut in 0..seed.len() {
        super::fuzz::fuzz_demux_ivf(&seed[..cut]);
    }
    // Wild edges.
    super::fuzz::fuzz_demux_ivf(&[]);
    super::fuzz::fuzz_demux_ivf(b"DKIF");
    super::fuzz::fuzz_demux_ivf(&[0x00; HEADER_BYTES as usize]);
    super::fuzz::fuzz_demux_ivf(&[0xFF; 128]);
}

#[test]
fn fuzz_lite_wild_declared_sizes_are_refused_not_allocated() {
    let mut seed = demo_stream().to_vec();
    for round in 0..256usize {
        seed.truncate(32 + 12 + 8); // header + one frame + ramp
        let at = 32 + (round % 4);
        seed[at] = 0xFF;
        if round % 2 == 0 {
            seed[36..44].copy_from_slice(&u64::MAX.to_le_bytes()); // wild pts
        }
        let mut demuxer = IvfDemuxer::new(mem_stream(seed.clone()).must()).must();
        if demuxer.start().is_ok() {
            let _ = drain(&mut demuxer);
        }
        // Whatever the outcome, no allocation grew toward the 4 GiB
        // claim (pool is bounded by the ceiling refusal).
        assert!(demuxer.pool_bytes() <= 2 * super::MAX_FRAME_BYTES);
    }
}
