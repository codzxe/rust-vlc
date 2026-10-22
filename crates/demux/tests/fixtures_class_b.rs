//! The class-B derivative behavior matrix (card 2.B2 AC): all ten
//! generated malformed AVIs behave per their `expectation/1` files —
//! truncations produce typed errors at the right boundaries, flips and
//! garbage produce typed parse errors or bounded recovery, and the
//! `noidx0` derivative demuxes fully through the movi-walk fallback.
//!
//! Per-file flavors are PINNED here (the expectations record the union
//! contract `typed-error-or-bounded-recovery`; this suite is where
//! 2.B2 fixes the specific flavor, as the 2.A3 worklog promised).

mod common;

use common::{open_via_input, parse_expectation, sha256_hex};
use demux::avi::AviDemuxer;
use media_types::error::CorruptDetail;
use media_types::time::{Stamp, TimeBase, TimeSpan, Timestamp};
use media_types::{EngineError, ErrorKind};
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;

const PARENT: &str = "generated/valid/avi_mjpeg_pcm.avi";
const PARENT_EXPECTATION: &str = "expectations/valid/avi_mjpeg_pcm.avi.toml";

fn derivative(name: &str) -> AviDemuxer {
    let relative = format!("generated/malformed/avi_mjpeg_pcm__{name}.avi");
    let path = testkit::fixture_path(&relative).must();
    let stream = open_via_input(&path).must();
    match AviDemuxer::new(stream) {
        Ok(demuxer) => demuxer,
        Err(err) => unreachable!("{name} opens through the access stack: {err}"),
    }
}

fn derivative_expectation(name: &str) -> common::Expectation {
    parse_expectation(&format!(
        "expectations/malformed/avi_mjpeg_pcm__{name}.avi.toml"
    ))
}

/// The typed truncation shape, asserted at the exact boundary.
fn assert_truncated(err: &EngineError, expected: u64, found: u64, context: &str) {
    assert_eq!(
        err.kind,
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated { expected, found }
        },
        "{context}: wrong error shape"
    );
    assert_eq!(err.context.code, "demux.avi.truncated", "{context}");
}

/// Drain until error or EOS; returns (packets, recovered-count, terminal).
type DrainOutcome = (Vec<(u32, Vec<u8>)>, u32, Result<(), EngineError>);
fn drain(avi: &mut AviDemuxer) -> DrainOutcome {
    let mut packets = Vec::new();
    let mut recovered = 0u32;
    let terminal = loop {
        match avi.next_event() {
            Ok(DemuxEvent::Packet(packet)) => {
                packets.push((packet.track.handle(), packet.data.as_slice().to_vec()));
            }
            Ok(DemuxEvent::Recovered { .. }) => recovered += 1,
            Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break Ok(()),
            Err(err) => break Err(err),
        }
    };
    (packets, recovered, terminal)
}

/// The reasons-capturing drain outcome (the ED-FIX-3 containment flavors
/// are pinned by name, not just by count).
type DrainWithReasons = (
    Vec<(u32, Vec<u8>)>,
    Vec<&'static str>,
    Result<(), EngineError>,
);

/// Drain that also captures the recovery reasons in order (the ED-FIX-3
/// containment flavors are pinned by name, not just by count).
fn drain_with_reasons(avi: &mut AviDemuxer) -> DrainWithReasons {
    let mut packets = Vec::new();
    let mut reasons = Vec::new();
    let terminal = loop {
        match avi.next_event() {
            Ok(DemuxEvent::Packet(packet)) => {
                packets.push((packet.track.handle(), packet.data.as_slice().to_vec()));
            }
            Ok(DemuxEvent::Recovered { reason, .. }) => reasons.push(reason),
            Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break Ok(()),
            Err(err) => break Err(err),
        }
    };
    (packets, reasons, terminal)
}

fn assert_video_hashes_match_parent(
    packets: &[(u32, Vec<u8>)],
    parent: &common::Expectation,
    frames: std::ops::Range<usize>,
) {
    let video: Vec<&Vec<u8>> = packets
        .iter()
        .filter(|(track, _)| *track == 0)
        .map(|(_, data)| data)
        .collect();
    assert_eq!(video.len(), frames.len(), "video packet count");
    for (offset, data) in video.iter().enumerate() {
        assert_eq!(
            sha256_hex(data),
            parent.video_sample_sha256[frames.start + offset],
            "frame {} hash",
            frames.start + offset
        );
    }
}

/// The prefix variant for containment walks: the FIRST `frames.len()`
/// video packets must hash-match the parent; extra trailing video
/// packets (a delivered clamped partial) are legal and excluded.
fn assert_video_hashes_prefix_match_parent(
    packets: &[(u32, Vec<u8>)],
    parent: &common::Expectation,
    frames: std::ops::Range<usize>,
) {
    let video: Vec<&Vec<u8>> = packets
        .iter()
        .filter(|(track, _)| *track == 0)
        .map(|(_, data)| data)
        .collect();
    assert!(
        video.len() >= frames.len(),
        "at least the complete frames must be video packets"
    );
    for (offset, data) in video[..frames.len()].iter().enumerate() {
        assert_eq!(
            sha256_hex(data),
            parent.video_sample_sha256[frames.start + offset],
            "frame {} hash",
            frames.start + offset
        );
    }
}

// ---------------------------------------------------------------------------
// trunc0..trunc4 — typed errors at the recorded boundaries
// ---------------------------------------------------------------------------

#[test]
fn trunc0_after_magic_is_typed_truncation() {
    let expect = derivative_expectation("trunc0");
    assert_eq!(
        expect.cut_offset,
        Some(12),
        "cut after RIFF size+form magic"
    );
    let mut avi = derivative("trunc0");
    let err = match avi.start() {
        Ok(_) => unreachable!("a 12-byte prefix cannot be a container"),
        Err(err) => err,
    };
    // The RIFF declared the full 425_516-byte extent; 12 arrived.
    assert_truncated(&err, 425_516, 12, "trunc0");
    // Sticky replay.
    let replay = match avi.start() {
        Ok(_) => unreachable!("sticky"),
        Err(err) => err,
    };
    assert_eq!(replay.context.code, err.context.code);
}

#[test]
fn trunc1_mid_avih_is_typed_truncation() {
    let expect = derivative_expectation("trunc1");
    assert_eq!(expect.cut_offset, Some(52), "cut mid-avih");
    let mut avi = derivative("trunc1");
    let err = match avi.start() {
        Ok(_) => unreachable!("a header cut cannot start"),
        Err(err) => err,
    };
    // The avih chunk needed bytes 32..88; 52 arrived.
    assert_truncated(&err, 88, 52, "trunc1");
}

#[test]
fn trunc2_at_first_movi_chunk_starts_but_errors_at_the_boundary() {
    let expect = derivative_expectation("trunc2");
    assert_eq!(
        expect.cut_offset,
        Some(324),
        "cut at first movi chunk header"
    );
    let mut avi = derivative("trunc2");
    let tracks = match avi.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("the header is intact: {err}"),
    };
    assert_eq!(tracks.len(), 2, "track discovery survives the data cut");
    let (packets, _recovered, terminal) = drain(&mut avi);
    assert!(packets.is_empty(), "no complete chunk exists");
    match terminal {
        Err(err) => {
            // The movi declared its list end at 423_588; 324 arrived.
            assert_truncated(&err, 423_588, 324, "trunc2");
        }
        Ok(()) => unreachable!("clamped movi must be a typed truncation"),
    }
}

#[test]
fn trunc3_mid_payload_delivers_prefix_with_counted_damage() {
    // ED-FIX-3 semantics change (Lead-verified 2026-10-22, D-3/D-9): a
    // mid-payload cut is bounded recovery, not a fatal truncation — the
    // readable prefix of the cut chunk is DELIVERED as the final packet,
    // every lying extent is damage-counted, and the walk ends in EOS.
    // The 2.B2-era contract (60 packets + typed `demux.avi.truncated`)
    // is superseded by this stronger observability contract: the loss
    // is accounted, not fatal.
    let expect = derivative_expectation("trunc3");
    let parent = parse_expectation(PARENT_EXPECTATION);
    assert_eq!(
        expect.cut_offset,
        Some(212_512),
        "cut mid-payload of frame-30 00dc"
    );
    let mut avi = derivative("trunc3");
    let tracks = match avi.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("the header is intact: {err}"),
    };
    assert_eq!(tracks.len(), 2);
    let (packets, reasons, terminal) = drain_with_reasons(&mut avi);
    // 30 complete video frames + their 30 audio ticks survive, plus the
    // clamped prefix of the cut frame-30 video chunk.
    assert_eq!(packets.len(), 61);
    assert_video_hashes_prefix_match_parent(&packets, &parent, 0..30);
    let audio_bytes: usize = packets
        .iter()
        .filter(|(track, _)| *track == 1)
        .map(|(_, data)| data.len())
        .sum();
    assert_eq!(audio_bytes, 30 * 5_880, "the surviving PCM ticks");
    let (last_track, last_len) = match packets.last() {
        Some((track, data)) => (*track, data.len()),
        None => unreachable!("61 packets were just counted"),
    };
    assert_eq!(last_track, 0, "the partial is the video frame-30 prefix");
    assert!(last_len > 0, "a non-empty readable prefix was delivered");
    // Three counted repairs, in walk order: the RIFF extent lied past
    // the stream, the movi list extended past the (clamped) RIFF, and
    // the chunk payload crossed the end of the file.
    assert_eq!(
        reasons,
        [
            "riff-extends-past-stream",
            "movi-extends-past-riff",
            "chunk-payload-past-eof",
        ],
        "the containment flavors, by name"
    );
    assert!(
        terminal.is_ok(),
        "terminal containment: EOS after the delivered prefix"
    );
    assert!(avi.damage_total() > 0, "the loss is damage-accounted");
}

#[test]
fn trunc4_last_byte_demuxes_fully_with_index_salvage() {
    let expect = derivative_expectation("trunc4");
    let parent = parse_expectation(PARENT_EXPECTATION);
    assert_eq!(expect.cut_offset, Some(425_515), "cut the final byte");
    let mut avi = derivative("trunc4");
    let tracks = match avi.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("header intact: {err}"),
    };
    assert_eq!(tracks.len(), 2);
    let (packets, reasons, terminal) = drain_with_reasons(&mut avi);
    match terminal {
        Ok(()) => {}
        Err(err) => unreachable!("losing the final byte is bounded recovery: {err}"),
    }
    assert_eq!(packets.len(), 120, "every chunk demuxes");
    assert_video_hashes_match_parent(&packets, &parent, 0..60);
    let pcm: Vec<u8> = packets
        .iter()
        .filter(|(track, _)| *track == 1)
        .flat_map(|(_, data)| data.iter().copied())
        .collect();
    assert_eq!(sha256_hex(&pcm), parent.audio_stream_sha256);
    // Three counted repairs (ED-FIX-3 idx1 hardening): the RIFF declared
    // one byte more than the stream carries (1 byte), the idx1 chunk's
    // declared size crossed the available bytes (1 byte), and the idx1
    // lost its final entry mid-record (15 of 16 bytes).
    assert_eq!(
        reasons,
        [
            "riff-extends-past-stream",
            "idx1-size-past-available",
            "idx1-partial-entry",
        ],
        "the salvage flavors, by name"
    );
    assert_eq!(
        avi.damage_total(),
        17,
        "1 missing stream byte + 1 idx1 size byte + 15 partial-entry bytes"
    );
}

// ---------------------------------------------------------------------------
// garbage0 / flip1 / flip4 / flip16 — bounded recovery or typed parse
// ---------------------------------------------------------------------------

#[test]
fn garbage0_appended_past_extent_is_invisible() {
    let expect = derivative_expectation("garbage0");
    let parent = parse_expectation(PARENT_EXPECTATION);
    assert_eq!(expect.size_bytes, 425_516 + 4_096);
    let mut avi = derivative("garbage0");
    let tracks = match avi.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("garbage past the RIFF extent is not read: {err}"),
    };
    assert_eq!(tracks.len(), 2);
    let (packets, recovered, terminal) = drain(&mut avi);
    match terminal {
        Ok(()) => {}
        Err(err) => unreachable!("bounded recovery ends at EOS: {err}"),
    }
    assert_eq!(packets.len(), 120);
    assert_video_hashes_match_parent(&packets, &parent, 0..60);
    assert_eq!(recovered, 0, "the walk never touches past-extent bytes");
}

#[test]
fn flip1_advisory_field_flip_demuxes_identically() {
    // flip1 flips bit 6 of byte 58 — inside avih's advisory dwStreams.
    // Advisory fields are never control flow: full demux, zero damage.
    let parent = parse_expectation(PARENT_EXPECTATION);
    let mut avi = derivative("flip1");
    let tracks = match avi.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("advisory flip: {err}"),
    };
    assert_eq!(tracks.len(), 2);
    let (packets, recovered, terminal) = drain(&mut avi);
    match terminal {
        Ok(()) => {}
        Err(err) => unreachable!("bounded recovery ends at EOS: {err}"),
    }
    assert_eq!(packets.len(), 120);
    assert_video_hashes_match_parent(&packets, &parent, 0..60);
    assert_eq!(recovered, 0, "nothing was repaired");
}

#[test]
fn flip4_lying_sizes_recover_with_counted_damage() {
    // flip4 hits the RIFF size, the hdrl LIST size and two advisory
    // avih bytes. Both sizes lie LARGE: the RIFF extent clamps at the
    // stream, and hdrl recognition stops at the movi it tried to
    // swallow — full demux with the repairs counted.
    let parent = parse_expectation(PARENT_EXPECTATION);
    let mut avi = derivative("flip4");
    let tracks = match avi.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("lying sizes recover: {err}"),
    };
    assert_eq!(tracks.len(), 2);
    let (packets, recovered, terminal) = drain(&mut avi);
    match terminal {
        Ok(()) => {}
        Err(err) => unreachable!("bounded recovery ends at EOS: {err}"),
    }
    assert_eq!(packets.len(), 120);
    assert_video_hashes_match_parent(&packets, &parent, 0..60);
    let pcm: Vec<u8> = packets
        .iter()
        .filter(|(track, _)| *track == 1)
        .flat_map(|(_, data)| data.iter().copied())
        .collect();
    assert_eq!(sha256_hex(&pcm), parent.audio_stream_sha256);
    assert!(
        recovered >= 2,
        "riff-extends-past-stream + hdrl-size lies are counted (got {recovered})"
    );
}

#[test]
fn flip16_broken_magic_is_a_typed_parse_error() {
    // flip16's first flip lands in the RIFF magic itself; the demuxer
    // rejects the file as not-an-AVI with the sniffed prefix.
    let mut avi = derivative("flip16");
    let err = match avi.start() {
        Ok(_) => unreachable!("corrupted magic must be rejected"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "demux.avi.not-avi");
    match err.kind {
        ErrorKind::Unsupported { format } => {
            assert_eq!(format.and_then(|f| f.name).as_deref(), Some("avi"));
        }
        other => unreachable!("unsupported flavor, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// noidx0 — full demux via the movi walk (the card's explicit AC)
// ---------------------------------------------------------------------------

#[test]
fn noidx0_demuxes_fully_via_the_movi_walk() {
    let parent = parse_expectation(PARENT_EXPECTATION);
    let mut avi = derivative("noidx0");
    let tracks = match avi.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("no idx1 is not a demux error: {err}"),
    };
    assert_eq!(tracks.len(), 2);
    let (packets, recovered, terminal) = drain(&mut avi);
    match terminal {
        Ok(()) => {}
        Err(err) => unreachable!("bounded recovery ends at EOS: {err}"),
    }
    assert_eq!(
        packets.len(),
        120,
        "the movi walk is the packet-extraction truth"
    );
    assert_video_hashes_match_parent(&packets, &parent, 0..60);
    let pcm: Vec<u8> = packets
        .iter()
        .filter(|(track, _)| *track == 1)
        .flat_map(|(_, data)| data.iter().copied())
        .collect();
    assert_eq!(pcm.len() as u64, parent.audio_stream_bytes);
    assert_eq!(sha256_hex(&pcm), parent.audio_stream_sha256);
    assert_eq!(recovered, 0, "nothing to repair — the index is optional");

    // Seek still works: the position-restoring linear scan answers the
    // parent's target grid within tolerance.
    let nanos = TimeBase::new(1, 1_000_000_000).must();
    for target_us in &parent.seek_targets_us {
        let target = TimeSpan {
            start: Stamp::Present(Timestamp::new(*target_us as i64 * 1_000, nanos)),
            end: Stamp::Missing,
        };
        let landing = match avi.seek(target) {
            Ok(span) => span,
            Err(err) => unreachable!("linear seek {target_us}µs: {err}"),
        };
        let landed_us = match landing.start.timestamp() {
            Ok(ts) => match ts.as_nanos() {
                Ok(ns) => ns.0 / 1_000,
                Err(err) => unreachable!("{err}"),
            },
            Err(_) => unreachable!("landing start present"),
        };
        let delta = landed_us.abs_diff(*target_us);
        assert!(
            delta <= parent.seek_tolerance_us,
            "noidx0 seek {target_us}µs landed at {landed_us}µs"
        );
        assert!(landed_us <= *target_us);
    }
}

// ---------------------------------------------------------------------------
// The whole class-B corpus through the fuzz harness (no panic, no hang)
// ---------------------------------------------------------------------------

#[test]
fn every_avi_fixture_survives_the_fuzz_harness() {
    let mut relatives = vec![PARENT.to_string()];
    for name in [
        "trunc0", "trunc1", "trunc2", "trunc3", "trunc4", "garbage0", "flip1", "flip4", "flip16",
        "noidx0",
    ] {
        relatives.push(format!("generated/malformed/avi_mjpeg_pcm__{name}.avi"));
    }
    for relative in relatives {
        let bytes = testkit::fixture_bytes(&relative).must();
        demux::avi::fuzz::fuzz_demux_avi(&bytes);
    }
}
