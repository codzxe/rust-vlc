//! FX1-A02 (FIX-PLAN §2 F0.4a) — the TS-188 probe boundary, pinned at
//! every window length the signature can see: a probe window of
//! EXACTLY one packet must never index past its end. The Wave-4
//! executed crash: `TsSignature::probe` indexed `prefix[188]` under a
//! `len >= 188` guard, so any 188-byte 0x47-led file — a whole valid
//! single-packet TS capture, or any 64 KiB probe window truncated to
//! file size by `fill_window` — panicked the shipped CLI open path:
//! "index out of bounds: the len is 188 but the index is 188", exit
//! 101.
//!
//! The fixed contract (the demuxer twin's own documented shape): a
//! window of at most one packet stays the anchored `mpegts` hit (one
//! byte consumed — "a whole single-packet file is the demuxer's own
//! walk to judge"); only a window STRICTLY longer than one packet
//! consults the second sync byte at the 188-byte stride. Honest
//! negatives keep the typed `probe.no-demuxer` refusal (stage Probe,
//! CLI exit 65 via the existing exit table) — never a panic, never a
//! new exit code.

use media_probe::signature::{ProbePlugin, TsSignature};
use media_probe::{ProbeConfig, ProbeRegistry, SourceMeta};
use media_types::PipelineStage;
use testkit::Must;

/// A 0x47-led probe window of `len` bytes (sync byte + zeros — the
/// audit's crafted `ts188.bin` shape).
fn ts_window(len: usize) -> Vec<u8> {
    let mut window = vec![0_u8; len];
    if !window.is_empty() {
        window[0] = 0x47;
    }
    window
}

/// The signature-level boundary sweep: exactly one packet (the
/// executed crash shape), sub-packet, and one-past on both sides of
/// the stride check — no length may panic.
#[test]
fn fx1_a02_one_packet_ts_window_never_panics() {
    let signature = TsSignature;
    let meta = SourceMeta::new(None, Some("ts"));
    // Exactly 188 bytes, 0x47-led: the guard must not read
    // prefix[188] on a 188-byte window — the walk-will-tell hit.
    match signature.probe(&ts_window(188), &meta) {
        Some(hit) => {
            assert_eq!(hit.format.as_str(), "mpegts");
            assert_eq!(hit.prefix_consumed, 1);
        }
        None => unreachable!("a one-packet window must stay anchored"),
    }
    // Sub-packet control (pre-existing behavior): still anchored.
    let short = signature.probe(&ts_window(187), &meta);
    assert_eq!(short.map(|hit| hit.prefix_consumed), Some(1), "187 B");
    // One-past controls: the second-sync check itself, both sides.
    let no_second = ts_window(189);
    assert!(signature.probe(&no_second, &meta).is_none(), "no second sync");
    let mut with_second = ts_window(189);
    with_second[188] = 0x47;
    match signature.probe(&with_second, &meta) {
        Some(hit) => assert_eq!(hit.prefix_consumed, 189),
        None => unreachable!("a second sync at the stride anchors"),
    }
}

/// The registry-level shape the CLI open path runs (`probe_stream`
/// feeds `registry.probe` the window `fill_window` truncated to file
/// size): a one-packet window routes to the TS container host as
/// `mpegts`, and a 188-byte window with no sync byte at all answers
/// the existing typed probe-stage refusal — exit 65, not 101.
#[test]
fn fx1_a02_registry_one_packet_ts_window_is_typed() {
    let registry = ProbeRegistry::with_builtins();
    let config = ProbeConfig::new(4096, 60).must();
    let meta = SourceMeta::new(None, Some("bin"));
    // The 188-byte 0x47-led window anchors mpegts (score 70 ≥ 60);
    // the TS walk downstream owns the deeper typed refusal.
    let report = registry.probe(&ts_window(188), &meta, &config).must();
    assert_eq!(report.format.as_str(), "mpegts");
    assert_eq!(report.bytes_examined, 188);
    // A 188-byte window with no sync byte at all: the honest
    // no-demuxer refusal, stage Probe — the existing exit-65 lane.
    match registry.probe(&vec![0_u8; 188], &meta, &config) {
        Err(err) => {
            assert_eq!(err.context.code, "probe.no-demuxer");
            assert_eq!(err.stage, PipelineStage::Probe);
        }
        Ok(report) => unreachable!("garbage must stay unprobed: {report:?}"),
    }
}
