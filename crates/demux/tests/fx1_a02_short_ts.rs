//! FX1-A02 (FIX-PLAN §2 F0.4a) — the demux-side twin of the TS-188
//! probe boundary: `ts::probe` indexed `header[188]` when the
//! `len < PACKET_BYTES` guard failed at exactly 188, so a
//! one-packet 0x47-led header panicked (Wave-4's standalone rustc
//! repro — the same off-by-one the live media-probe twin carried).
//!
//! The fixed contract (the function's own comment): a whole
//! single-packet header is accepted — the demuxer's walk judges it.
//! A crafted truncated one-packet TS (188 bytes, no PAT anywhere)
//! then refuses through the module's existing typed error path
//! (`demux.ts.no-pat`, stage Demux — the audit's executed 187-byte
//! CLI control, exit 65), never a panic and never exit 101.

use demux::ts::test_support::mem_stream;
use demux::ts::{PACKET_BYTES, TsDemuxer, probe};
use plugin_api::Demuxer;
use testkit::Must;

/// A 0x47-led window of `len` bytes (sync byte + zeros — the audit's
/// crafted `ts188.bin` shape).
fn ts_window(len: usize) -> Vec<u8> {
    let mut window = vec![0_u8; len];
    if !window.is_empty() {
        window[0] = 0x47;
    }
    window
}

/// The probe boundary sweep: exactly one packet (the executed crash
/// shape), sub-packet, and one-past on both sides of the stride
/// check — no length may panic.
#[test]
fn fx1_a02_probe_accepts_an_exactly_one_packet_header() {
    // Exactly 188 bytes, 0x47-led: must not index header[188] on a
    // 188-byte slice — accepted, the walk will tell.
    assert!(probe(&ts_window(PACKET_BYTES)));
    // Sub-packet control (pre-existing behavior): accepted.
    assert!(probe(&ts_window(PACKET_BYTES - 1)));
    // One-past controls: the second-sync check itself, both sides.
    assert!(!probe(&ts_window(PACKET_BYTES + 1)));
    let mut with_second = ts_window(PACKET_BYTES + 1);
    with_second[PACKET_BYTES] = 0x47;
    assert!(probe(&with_second));
}

/// The truncated-stream refusal: a crafted 188-byte one-packet TS
/// with no PAT answers the module's existing typed `demux.ts.no-pat`
/// error (sticky across retries) — the error path the CLI already
/// maps to exit 65, not a panic.
#[test]
fn fx1_a02_truncated_one_packet_ts_refuses_typed() {
    let bytes = ts_window(PACKET_BYTES);
    let mut demuxer = match TsDemuxer::new(mem_stream(bytes).must()) {
        Ok(demuxer) => demuxer,
        Err(err) => unreachable!("construct failed: {err:?}"),
    };
    match demuxer.start() {
        Err(err) => assert_eq!(err.context.code, "demux.ts.no-pat"),
        Ok(_) => unreachable!("a PAT-less single-packet stream must refuse"),
    }
    // Sticky: the refusal replays (the module's existing shape).
    match demuxer.start() {
        Err(err) => assert_eq!(err.context.code, "demux.ts.no-pat"),
        Ok(_) => unreachable!("sticky refusal expected"),
    }
}
