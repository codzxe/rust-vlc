//! FX1-A09 regression guard — the PS `payload_len` u64 underflow
//! (W05-A03 F-001, S2; FIX-PLAN F0.4m). The MPEG-1 PES lane of the
//! demuxer's `parse_pes_full` subtracted `header_len` from the
//! declared packet length with no guard (the MPEG-2 lane checks its
//! header extent first), so a legal-prefix MPEG-1 PES with
//! `declared_len = 2` and a 5-byte PTS header computed `2 - 5`:
//! a debug-build panic, and in release a wrapped ≈2^64 `payload_len`
//! that killed the whole session as `demux.ps.payload-too-large`.
//!
//! The guard is a checked subtraction returning the module's typed
//! `demux.ps.bad-pes-header` refusal. Both lanes drive crafted bytes
//! through the REAL CLI open path (probe → containers::open →
//! PsDemuxer::start's pre-scan), in-process (the fx1_a05_argv_adts
//! convention; no binary spawning): pre-fix the underflow shape
//! panicked this test; post-fix it answers the typed refusal.

use std::path::PathBuf;

use commands::catalog::types::SourceSpec;
use demux::ps::test_support::{mpeg1_pack, program_end, stamp_bytes};
use lumen_cli::demux_host::open_source;
use testkit::Must;

/// A temp dir that removes itself (the ts.rs suite's convention).
struct TempDir(PathBuf);
impl TempDir {
    fn new(tag: &str) -> TempDir {
        let dir = std::env::temp_dir().join(format!("lumen-fx1-a09-{tag}-{}", std::process::id())); // lumen-allow-literal: cfg(test) scratch filename — test-only, never a shipped surface
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

/// The audit's crafted MPEG-1 PES underflow shape: an MPEG-1 pack, one
/// video PES whose declared length (2) is over-run by its own 5-byte
/// PTS header (pre-fix `2 - 5`), then the program end code.
fn crafted_ps() -> Vec<u8> {
    let mut stream = Vec::new();
    stream.extend_from_slice(&mpeg1_pack(0, 250_000));
    // PES start code + stream id 0xE0, declared_len = 2.
    stream.extend_from_slice(&[0x00, 0x00, 0x01, 0xE0, 0x00, 0x02]);
    // The MPEG-1 header: a 5-byte PTS stamp (prefix 0x21) — five bytes
    // of header for a two-byte declared packet.
    stream.extend_from_slice(&stamp_bytes(0b0010, 9_000));
    stream.extend_from_slice(&program_end());
    stream
}

/// The underflow shape refuses typed at open — the PES header that
/// overruns its own declared packet is `demux.ps.bad-pes-header`,
/// never the arithmetic panic (debug) or the release lane's
/// `payload-too-large` session death.
#[test]
fn fx1_a09_ps_mpeg1_header_overruns_declared_len_is_typed_not_a_panic() {
    let dir = TempDir::new("ps-underflow");
    let path = dir.write("underflow.mpg", &crafted_ps());
    match open_source(&SourceSpec::File { path }, None, false, false) {
        Err(err) => {
            assert_eq!(
                err.details.get("code").map(String::as_str),
                Some("demux.ps.bad-pes-header"),
                "the typed PES-header-vs-declared-length refusal: {err}"
            );
        }
        Ok(_) => unreachable!("the over-run PES must refuse, got an open session"),
    }
}

/// The sibling lane: the same PES shape with a declared length that
/// FITS its header opens cleanly — the guard refuses only the
/// over-run, not the MPEG-1 stamped lane (one real mp2 AU, the ps.rs
/// synthetic-lane shape).
#[test]
fn fx1_a09_ps_mpeg1_lane_still_opens_after_the_guard() {
    // One 208-byte Layer II frame SHAPE (MPEG-1, 64 kbps, 44.1 kHz —
    // the ps.rs synthetic-lane convention; framing only).
    let mut frame = vec![0x11u8; 208];
    frame[0] = 0xFF;
    frame[1] = 0xFD; // MPEG-1, Layer II, no CRC
    frame[2] = 0x40; // bitrate idx 4 (64 kbps), rate idx 0 (44100)
    frame[3] = 0xC4; // mono, no flags
    let mut pes = vec![0x00, 0x00, 0x01, 0xC0];
    // declared_len = 5-byte PTS header + 208-byte payload = 213.
    pes.extend_from_slice(&213u16.to_be_bytes());
    pes.extend_from_slice(&stamp_bytes(0b0010, 9_000));
    pes.extend_from_slice(&frame);
    let mut stream = Vec::new();
    stream.extend_from_slice(&mpeg1_pack(0, 250_000));
    stream.extend_from_slice(&pes);
    stream.extend_from_slice(&program_end());
    let dir = TempDir::new("ps-clean");
    let path = dir.write("clean.mpg", &stream);
    match open_source(&SourceSpec::File { path }, None, false, false) {
        Ok(opened) => {
            assert_eq!(opened.facts.container, "mpegps");
            assert_eq!(opened.facts.tracks.len(), 1, "one mp2 audio track");
            assert_eq!(opened.facts.tracks[0].codec.as_str(), "mp2");
        }
        Err(err) => unreachable!("the fitting MPEG-1 PES still opens: {err}"),
    }
}
