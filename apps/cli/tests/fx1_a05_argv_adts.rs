//! FX1-A05 regression tests (wave FX1, FIX-PLAN F0.4e+f): the two
//! hostile-input S1 guards this fix lands, driven in-process through
//! the container/parser public API — no process spawning.
//!
//! - **argv boundary** (W13-A05 F-001): `std::env::args()` panicked
//!   with exit 101 on any non-UTF-8 argv element before a single
//!   player line ran (`./lumen $'\xff\xfe'`, executed by the audit).
//!   The OS-native boundary ([`lumen_cli::parse::argv_string`]) is
//!   unit-tested with raw byte tokens — the audit's executed 0xFF
//!   0xFE shape on Unix, an unpaired surrogate on Windows.
//! - **ADTS AU-split walk** (W13-A08 F-001): a crafted zero-length
//!   ADTS frame inside one TS PES wedged the walk forever (the
//!   header re-parsed at the same offset minted empty packets
//!   without end — hang + unbounded queue). The lane here is the
//!   REAL open path (probe → container registry → the PES bridge),
//!   built with the demux crate's public `test_support` builders;
//!   no committed .ts fixture exists in the tree, so the crafted
//!   bytes are written here (the ts.rs suite's recorded convention).

use std::ffi::OsStr;
use std::path::PathBuf;

use commands::catalog::types::SourceSpec;
use demux::ts::test_support::{
    pat_body, pmt_body, psi_packets, psi_section, sdt_body, stream_of, ts_packet,
};
use lumen_cli::demux_host::{open_source, OpenedMedia};
use lumen_cli::parse::{argv_string, USAGE};
use media_types::time::Stamp;
use plugin_api::DemuxEvent;
use testkit::Must;

// ---------------------------------------------------------------------------
// argv boundary
// ---------------------------------------------------------------------------

/// Valid UTF-8 tokens pass through verbatim — the boundary must not
/// become a new refusal point for legal argv (ASCII or not).
#[test]
fn fx1_a05_argv_string_passes_valid_tokens_through() {
    for token in ["open", "--json", "clip.mp4", "h\u{e9}llo"] {
        match argv_string(OsStr::new(token)) {
            Ok(text) => assert_eq!(text, token, "{token}"),
            Err(failure) => unreachable!("valid token refused: {}", failure.message),
        }
    }
}

/// The audit's executed shape on Linux: `./lumen $'\xff\xfe'` → the
/// `env::args()` panic. The refusal must be typed: the message names
/// the class (never echoes the raw bytes — no lossy mojibake), the
/// usage hint rides along.
#[test]
#[cfg(unix)]
fn fx1_a05_argv_string_refuses_non_utf8_typed() {
    use std::os::unix::ffi::OsStrExt;
    match argv_string(OsStr::from_bytes(&[0xFF, 0xFE])) {
        Err(failure) => {
            assert_eq!(failure.hint, USAGE, "{}", failure.message);
            assert!(
                failure.message.contains("not valid UTF-8"),
                "{}",
                failure.message
            );
            assert!(
                !failure.message.contains('\u{FFFD}'),
                "no lossy echo: {}",
                failure.message
            );
        }
        Ok(token) => unreachable!("non-UTF-8 argv must refuse typed, got {token:?}"),
    }
}

/// The Windows twin: an unpaired surrogate is legal `OsString`
/// content but not UTF-8 — the same typed refusal.
#[test]
#[cfg(windows)]
fn fx1_a05_argv_string_refuses_non_utf8_typed() {
    use std::os::windows::ffi::OsStringExt;
    match argv_string(&std::ffi::OsString::from_wide(&[0xD800])) {
        Err(failure) => {
            assert_eq!(failure.hint, USAGE, "{}", failure.message);
            assert!(
                failure.message.contains("not valid UTF-8"),
                "{}",
                failure.message
            );
        }
        Ok(token) => unreachable!("non-UTF-8 argv must refuse typed, got {token:?}"),
    }
}

// ---------------------------------------------------------------------------
// ADTS AU-split walk
// ---------------------------------------------------------------------------

/// A temp dir that removes itself (the ts.rs suite's convention).
struct TempDir(PathBuf);
impl TempDir {
    fn new(tag: &str) -> TempDir {
        let dir = std::env::temp_dir().join(format!("lumen-fx1-a05-{tag}-{}", std::process::id())); // lumen-allow-literal: cfg(test) scratch filename — test-only, never a shipped surface
        std::fs::create_dir_all(&dir).must();
        TempDir(dir)
    }
    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).must();
        path
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Open a crafted source through the REAL CLI open path (the ts.rs
/// suite's in-module convention, driven from outside the crate).
fn open(path: PathBuf) -> OpenedMedia {
    match open_source(&SourceSpec::File { path }, None, false, false) {
        Ok(opened) => opened,
        Err(err) => unreachable!("open: {err}"),
    }
}

/// Encode a 33-bit stamp into the 5 guarded PES bytes (mirrors the
/// ts.rs suite's private helper — the inverse of the assembler's
/// `decode_ts5`, with the marker bits the decoder requires).
fn encode_ts5(value: u64, prefix: u8) -> [u8; 5] {
    let v = value & 0x1_FFFF_FFFF;
    [
        ((u64::from(prefix) << 4) | ((v >> 30) & 0x07) << 1 | 1) as u8,
        ((v >> 22) & 0xFF) as u8,
        ((((v >> 15) & 0x7F) << 1) | 1) as u8,
        ((v >> 7) & 0xFF) as u8,
        (((v & 0x7F) << 1) | 1) as u8,
    ]
}

/// One bounded PTS-only PES packet (mirrors the ts.rs suite's private
/// helper; the demux crate's test_support owns the PSI half).
fn pes_bytes(stream_id: u8, pts: u64, payload: &[u8]) -> Vec<u8> {
    let mut out = vec![0x00, 0x00, 0x01, stream_id];
    // The length field counts everything after these 6 bytes:
    // flags1 + flags2 + data_length + the 5-byte PTS + payload.
    let after = 8 + payload.len();
    out.extend_from_slice(&u16::try_from(after).unwrap_or(u16::MAX).to_be_bytes());
    out.push(0x80); // flags1 (marker bits)
    out.push(0x80); // flags2: PTS only (0b10)
    out.push(5); // data_length
    out.extend_from_slice(&encode_ts5(pts, 0b0010));
    out.extend_from_slice(payload);
    out
}

/// One ADTS-shaped frame (85 bytes: 7-byte LC/44.1 kHz header + body,
/// 1024 samples) — framing only, nothing decodes it (mirrors the
/// ts.rs suite's `adts_frame_85`).
fn adts_frame_85(fill: u8) -> [u8; 85] {
    let mut frame = [fill; 85];
    frame[0] = 0xFF;
    frame[1] = 0xF1; // MPEG-4, layer 0, no CRC
    frame[2] = (1 << 6) | (4 << 2); // profile LC, sf_idx 4 (44100)
    frame[3] = 0x80; // channel_config high bits (2) + length hi
    frame[4] = (85 >> 3) as u8;
    frame[5] = ((85 & 0x07) << 5) | 0x1F; // length lo + VBR fullness
    frame[6] = 0xFC; // fullness lo + one raw block
    frame
}

/// The audit's crafted zero-length frame (executed shape): a 7-byte
/// header with the syncword, LC profile, sampling index 4 (a
/// resolvable 44 100 Hz rate, so the AU-split walk engages) — and
/// all 13 `frame_length` bits zero: a length smaller than the
/// header's own 7 bytes. `AdtsHeader::parse` accepts it; the length
/// mints nothing and advances nothing.
fn adts_frame_zero_length() -> [u8; 7] {
    [0xFF, 0xF1, 0x50, 0x00, 0x00, 0x00, 0x00]
}

/// A minimal single-program TS whose audio PES (pid 0x101,
/// stream_type 0x0F — the AU-split lane) carries the given
/// elementary payload, plus one video PES on 0x100: the ts.rs
/// suite's `synthetic_ts` shape with the audio payload replaced by
/// the crafted hostile one. Every PES fits its transport packet's
/// payload region exactly (the ffmpeg muxing convention the
/// assembler's bounded-length checks assume).
fn synthetic_ts_with_audio(payload: &[u8]) -> Vec<u8> {
    let pat = psi_section(0x00, 1, 0, &pat_body(&[(1, 0x1000)]));
    let pmt = psi_section(
        0x02,
        1,
        0,
        &pmt_body(0x100, &[], &[(0x1B, 0x100, &[]), (0x0F, 0x101, &[])]),
    );
    let sdt = psi_section(0x42, 1, 0, &sdt_body(1, &[(1, "FFmpeg", "Service01")]));
    let mut packets = Vec::new();
    packets.extend(psi_packets(0x0011, 0, &[sdt]));
    packets.extend(psi_packets(0x0000, 0, &[pat]));
    packets.extend(psi_packets(0x1000, 0, &[pmt]));
    packets.push(ts_packet(
        0x100,
        true,
        0,
        None,
        &pes_bytes(0xE0, 126_000, &[0xAA; 170]),
    ));
    packets.push(ts_packet(
        0x101,
        true,
        0,
        None,
        &pes_bytes(0xC0, 128_000, payload),
    ));
    stream_of(&packets)
}

/// The pure hostile shape (the audit's crafted PES): the whole audio
/// payload is one zero-length ADTS frame. Pre-fix the AU-split walk
/// re-parsed it forever (an unbounded queue of empty packets — this
/// test did not terminate); post-fix the walk stops at the damage,
/// mints nothing for the audio track, counts it typed, and the
/// stream's lanes all still reach end-of-stream. The second
/// `pes.length-overrun` record is the assembler's own honest
/// accounting for the transport pad after the short PES (the
/// crafted PES does not fill its packet's payload region — the
/// audit's exact 7-byte shape).
#[test]
fn fx1_a05_zero_length_adts_frame_does_not_wedge_the_au_split_walk() {
    let dir = TempDir::new("adts-zero");
    let path = dir.write(
        "adts-zero.ts",
        &synthetic_ts_with_audio(&adts_frame_zero_length()),
    );
    let mut opened = open(path);
    assert_eq!(opened.facts.container, "mpegts");
    assert_eq!(opened.facts.tracks.len(), 2, "the PMT declares two ES");
    let audio = opened.facts.tracks[1].id;
    let mut packets = [0u64, 0u64]; // (other tracks, audio)
    let mut recovered = Vec::new();
    loop {
        match opened.demuxer.next_event() {
            Ok(DemuxEvent::Packet(packet)) => {
                packets[if packet.track == audio { 1 } else { 0 }] += 1;
            }
            Ok(DemuxEvent::EndOfStream) => break,
            Ok(DemuxEvent::Recovered { skipped, reason }) => recovered.push((skipped, reason)),
            Ok(DemuxEvent::Discontinuity) => {}
            Err(err) => unreachable!("demux: {err}"),
        }
    }
    assert_eq!(
        packets,
        [1, 0],
        "the video PES survives; the malformed audio PES mints no packet"
    );
    assert_eq!(
        recovered,
        vec![
            (7u64, "ts.pes.adts-short-frame"),
            (163u64, "pes.length-overrun")
        ],
        "the zero-length frame is counted typed, never minted, never re-parsed; \
         the overrun is the pad after the short PES"
    );
}

/// Damage after a good AU: the good frame still mints with its
/// anchor stamp, the walk stops AT the zero-length frame (bounded
/// advance — it neither skips past the damage nor re-parses it), and
/// the damage is counted.
#[test]
fn fx1_a05_zero_length_adts_frame_stops_the_walk_after_good_aus() {
    let mut payload = adts_frame_85(0xBB).to_vec();
    payload.extend_from_slice(&adts_frame_zero_length());
    let dir = TempDir::new("adts-zero-tail");
    let path = dir.write("adts-zero-tail.ts", &synthetic_ts_with_audio(&payload));
    let mut opened = open(path);
    let audio = opened.facts.tracks[1].id;
    let mut packets = [0u64, 0u64]; // (other tracks, audio)
    let mut audio_pts = Vec::new();
    let mut recovered = Vec::new();
    loop {
        match opened.demuxer.next_event() {
            Ok(DemuxEvent::Packet(packet)) => {
                if packet.track == audio {
                    packets[1] += 1;
                    match packet.pts {
                        Stamp::Present(stamp) => audio_pts.push(stamp.ticks),
                        Stamp::Missing => {}
                    }
                } else {
                    packets[0] += 1;
                }
            }
            Ok(DemuxEvent::EndOfStream) => break,
            Ok(DemuxEvent::Recovered { skipped, reason }) => recovered.push((skipped, reason)),
            Ok(DemuxEvent::Discontinuity) => {}
            Err(err) => unreachable!("demux: {err}"),
        }
    }
    assert_eq!(packets, [1, 1], "the video PES + the one good AU");
    assert_eq!(audio_pts, vec![128_000], "the good AU rides the PES anchor");
    assert_eq!(
        recovered,
        vec![
            (7u64, "ts.pes.adts-short-frame"),
            (78u64, "pes.length-overrun")
        ],
        "the walk stops at the damage and counts it — never a wedge; \
         the overrun is the pad after the short PES"
    );
}
