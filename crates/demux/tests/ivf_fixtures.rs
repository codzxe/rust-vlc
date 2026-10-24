//! The IVF differential suite (card 5.I3b AC): the committed
//! ffmpeg-generated AV1/VP9 fixtures demux **bit-exactly** against the
//! recorded ffmpeg truth — frame counts, pts ladders, dims and the
//! concatenated payload MD5 — and the mutated/truncated derivative
//! families stay inside the typed-error-or-bounded-recovery vocabulary
//! (the `+FB` fuzz-lite contract).
//!
//! # Fixture provenance (recorded for review)
//!
//! The two fixtures under `tests/fixtures/generated/ivf/` are
//! **ffmpeg 7.1.5 products** (Lavf61.7.103 / Lavc61.19.101; signal
//! content is lavfi's `testsrc2` — generated test data, no
//! third-party media, CC0-equivalent):
//!
//! * `av1_64x48_30fps.ivf` — `ffmpeg -f lavfi -i
//!   testsrc2=size=64x48:rate=30 -t 1 -c:v libaom-av1 -cpu-used 8
//!   -deadline realtime -f ivf` (libaom v3.12.1);
//! * `vp9_64x48_25fps.ivf` — `ffmpeg -f lavfi -i
//!   testsrc2=size=64x48:rate=25 -t 1 -c:v libvpx-vp9 -deadline
//!   realtime -cpu-used 8 -f ivf` (libvpx).
//!
//! Committed read-only, ≈ 4.5 KiB total (< 200 KiB fuzz budget). The
//! xtask generator does not own them (the 5.A2/5.B1 arrangement).
//!
//! # The recorded truth (measured, never transcribed by hand)
//!
//! Measured this session from the committed bytes with the installed
//! ffmpeg/ffprobe:
//!
//! * stream parameters via `ffprobe -show_entries
//!   stream=codec_name,codec_tag_string,width,height,r_frame_rate,
//!   time_base,nb_frames`;
//! * packet counts via `ffprobe -count_packets -show_entries
//!   stream=nb_read_packets`;
//! * the pts ladder + sizes via `ffprobe -show_entries
//!   packet=pts,size` (the full ladder is asserted: 0..N−1);
//! * the payload digest via `ffmpeg -i <fixture> -c copy -f md5 -`
//!   — the MD5 of ffmpeg's demuxed packet stream (the raw dump).
//!
//! The differential claim: our demuxer's concatenated packet payloads
//! hash identically to ffmpeg's raw dump for the same file, the pts
//! ladders are the container's own integers on both sides, and the
//! packet counts match `nb_read_packets` exactly.
//!
//! # Honest divergences (measured on this ffmpeg, recorded here)
//!
//! * **flags**: ffprobe marks the first packet `K_` — that bit is
//!   derived by ffmpeg's codec parser (decode domain), not from IVF
//!   container bytes; our packets carry `keyframe = false` (never
//!   invented). Flags are excluded from the compared columns.
//! * **truncated final frame**: ffmpeg emits a *short packet*; we
//!   refuse typed (`demux.ivf.frame-beyond-file`, the ADTS
//!   precedent). Pinned on a crafted derivative below.
//! * **header-only file**: ffmpeg opens it and yields a clean empty
//!   stream; we refuse typed (`demux.ivf.no-frames`, the AVI
//!   no-movi / ADIF no-data house precedent).
//! * **unsupported fourcc** (`VP80`): ffmpeg demuxes VP8-in-IVF
//!   fine; v0 delivery scope is AV1/VP9, so ours is the typed
//!   `demux.ivf.unsupported-fourcc` refusal — scope, not a parse
//!   failure.
//! * **header length ≠ 32**: ffmpeg's walk reads from offset 32
//!   regardless (verified: it rejects a `hlen = 64` file with 32
//!   extra bytes as invalid data); we refuse typed at the header
//!   (`demux.ivf.bad-header`) instead of misparsing.

mod common;

use std::sync::Arc;

use common::{md5_hex, open_via_input};
use demux::ivf::IvfDemuxer;
use demux::ivf::fuzz::fuzz_demux_ivf;
use media_types::time::Stamp;
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;

/// One fixture's recorded ffmpeg truth (all fields measured, see the
/// module docs for the commands).
struct Truth {
    /// Path under the workspace `tests/fixtures/`.
    fixture: &'static str,
    /// ffprobe `codec_name` / `codec_tag_string`.
    codec: &'static str,
    fourcc: &'static [u8; 4],
    /// ffprobe `width`/`height`.
    dims: (u16, u16),
    /// ffprobe `r_frame_rate` (`num/den`) — our rate/scale fields.
    fps: (u32, u32),
    /// ffprobe `nb_read_packets` (== `nb_frames` here).
    packets: u64,
    /// `ffmpeg -c copy -f md5 -` over the demuxed packet stream.
    payload_md5: &'static str,
}

const TRUTHS: &[Truth] = &[
    Truth {
        fixture: "generated/ivf/av1_64x48_30fps.ivf",
        codec: "av1",
        fourcc: b"AV01",
        dims: (64, 48),
        fps: (30, 1),
        packets: 30,
        payload_md5: "0db6161ace241fb5966eaea0e3339aac",
    },
    Truth {
        fixture: "generated/ivf/vp9_64x48_25fps.ivf",
        codec: "vp9",
        fourcc: b"VP90",
        dims: (64, 48),
        fps: (25, 1),
        packets: 25,
        payload_md5: "e62f0aba008315efcb91734112aad24e",
    },
];

/// Demux a fixture through the real access stack (`input`'s
/// FileAccess — the 2.A1+2.B2 composition seam) and return every
/// packet.
fn demux_fixture(truth: &Truth) -> Vec<media_types::Packet> {
    let path = testkit::fixture_path(truth.fixture).must();
    let stream = open_via_input(&path).must();
    let mut demuxer = IvfDemuxer::new(stream).must();
    let tracks = demuxer.start().must();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].kind, media_types::MediaKind::Video);
    assert_eq!(tracks[0].codec.as_str(), truth.codec);
    let mut packets = Vec::new();
    loop {
        match demuxer.next_event().must() {
            DemuxEvent::Packet(p) => packets.push(p),
            DemuxEvent::EndOfStream => break,
            DemuxEvent::Recovered { skipped, reason } => {
                unreachable!("clean fixture recovered {skipped}: {reason}");
            }
            DemuxEvent::Discontinuity => {
                unreachable!("clean fixture is continuous");
            }
        }
    }
    packets
}

#[test]
fn stream_parameters_match_ffprobe() {
    for truth in TRUTHS {
        let path = testkit::fixture_path(truth.fixture).must();
        let stream = open_via_input(&path).must();
        let mut demuxer = IvfDemuxer::new(stream).must();
        demuxer.start().must();
        let info = demuxer.stream_info().must();
        assert_eq!(info.format, "ivf", "{}", truth.fixture);
        assert_eq!(info.codec, truth.codec, "{}", truth.fixture);
        assert_eq!(&info.fourcc, truth.fourcc, "{}", truth.fixture);
        assert_eq!((info.width, info.height), truth.dims, "{}", truth.fixture);
        assert_eq!((info.rate, info.scale), truth.fps, "{}", truth.fixture);
        // The declared frame count agrees with ffprobe's nb_frames.
        assert_eq!(
            u64::from(info.declared_frames),
            truth.packets,
            "{}",
            truth.fixture
        );
        let header = demuxer.header().must();
        // The track's tick base is the container's own ratio
        // (ffprobe's time_base).
        let base = header.tick_base().must();
        assert_eq!(
            (base.num, base.den),
            (truth.fps.1, truth.fps.0),
            "{}",
            truth.fixture
        );
    }
}

#[test]
fn frame_counts_pts_ladders_and_payload_md5_match_ffmpeg() {
    for truth in TRUTHS {
        let packets = demux_fixture(truth);
        // Frame count == ffprobe's nb_read_packets.
        assert_eq!(packets.len() as u64, truth.packets, "{}", truth.fixture);
        // The pts ladder: the container's own integers, 0..N−1
        // (ffprobe's full `packet=pts` dump, compared in order).
        let mut payload = Vec::new();
        for (i, packet) in packets.iter().enumerate() {
            let Stamp::Present(ts) = packet.pts else {
                unreachable!("pts is present");
            };
            assert_eq!(ts.ticks, i as i64, "{}[{}]", truth.fixture, i);
            assert_eq!(ts.base.den, truth.fps.0);
            // Every packet carries the declared one-tick duration
            // (ffprobe's `duration = 1` convention).
            assert!(packet.duration.is_some(), "{}[{}]", truth.fixture, i);
            assert!(!packet.keyframe, "{}[{}]: never invented", truth.fixture, i);
            payload.extend_from_slice(&packet.data[..]);
        }
        // The payload MD5 == ffmpeg's `-c copy -f md5 -` raw dump.
        assert_eq!(
            md5_hex(&payload),
            truth.payload_md5,
            "{}: payload differential",
            truth.fixture
        );
    }
}

#[test]
fn truncated_derivatives_are_typed_never_panics() {
    for truth in TRUTHS {
        let bytes: Arc<[u8]> = testkit::fixture_bytes(truth.fixture).must().into();
        // Every truncation of the container (the empty cut is the
        // access seam's own typed refusal, pinned in the unit suite):
        // typed outcome or a bounded clean end, never a panic (the
        // +FB contract over the real ffmpeg bytes).
        for cut in 1..bytes.len() {
            let view = &bytes[..cut];
            let stream = demux::ivf::test_support::mem_stream(view.to_vec()).must();
            let mut demuxer = IvfDemuxer::new(stream).must();
            match demuxer.start() {
                Err(err) => {
                    assert!(
                        matches!(
                            err.context.code.as_str(),
                            "demux.ivf.truncated"
                                | "demux.ivf.no-frames"
                                | "demux.ivf.bad-header"
                                | "demux.ivf.not-ivf"
                        ),
                        "{}@{cut}: unexpected {}",
                        truth.fixture,
                        err.context.code
                    );
                }
                Ok(_) => loop {
                    match demuxer.next_event() {
                        Ok(DemuxEvent::EndOfStream) => break,
                        Ok(_) => {}
                        Err(err) => {
                            assert_eq!(
                                err.context.code, "demux.ivf.frame-beyond-file",
                                "{}@{cut}",
                                truth.fixture
                            );
                            break;
                        }
                    }
                },
            }
        }
    }
}

#[test]
fn crafted_frame_beyond_file_refuses_where_ffmpeg_shortens() {
    // ffmpeg's measured behavior on this shape: it emits the 10
    // available bytes as a short final packet (ffprobe: one packet,
    // size 10). Ours is the typed refusal — the recorded divergence.
    let mut bytes = testkit::fixture_bytes(TRUTHS[0].fixture).must();
    // Truncate inside the last frame's declared payload.
    bytes.truncate(bytes.len() - 1);
    let stream = demux::ivf::test_support::mem_stream(bytes).must();
    let mut demuxer = IvfDemuxer::new(stream).must();
    demuxer.start().must();
    let mut packets = 0u64;
    let terminal = loop {
        match demuxer.next_event() {
            Ok(DemuxEvent::Packet(_)) => packets += 1,
            Ok(DemuxEvent::EndOfStream) => break None,
            Ok(_) => {}
            Err(err) => break Some(err.context.code.clone()),
        }
    };
    // The 29 whole frames stream out; the 30th refuses typed.
    assert_eq!(packets, TRUTHS[0].packets - 1);
    assert_eq!(terminal, Some("demux.ivf.frame-beyond-file".to_string()));
}

#[test]
fn fuzz_lite_over_the_real_ffmpeg_bytes() {
    // Deterministic bit-flip families over both committed fixtures
    // (≈ 4.5 KiB of real container bytes — under the 200 KiB budget),
    // each driving the full fuzz pass: open, drain, seek probes.
    // Absence-of-panic is the contract; typed outcomes are expected.
    for truth in TRUTHS {
        let bytes = testkit::fixture_bytes(truth.fixture).must();
        let mut lcg = 0x05EE_D1F1_u64;
        for round in 0..512usize {
            let mut mutant = bytes.clone();
            let flips = 1 + round % 8;
            for _ in 0..flips {
                lcg = lcg
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let at = (lcg >> 33) as usize % mutant.len();
                mutant[at] ^= 1u8 << (lcg % 8) as u8;
            }
            fuzz_demux_ivf(&mutant);
        }
        // Truncation sweep through the fuzz harness as well.
        for cut in (0..bytes.len()).step_by(7) {
            fuzz_demux_ivf(&bytes[..cut]);
        }
    }
}
