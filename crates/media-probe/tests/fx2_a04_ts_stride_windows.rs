//! FX2-A04 — the no-panic sweep's promise, pinned at the registry
//! lane the CLI open path runs: every probe-window length across the
//! MPEG-TS 188-byte stride boundary answers typed, never a panic.
//!
//! The audit's false-promise note (Wave-4's executed crash context,
//! recorded in FX1-A02's risk section): the in-module sweep
//! `builtin_signatures_never_panic_on_arbitrary_windows` swept a
//! 64-byte RIFF template only and never reached a 188-byte 0x47
//! window — the exact shape that panicked the shipped probe before
//! FX1-A02's guard — so its "arbitrary windows" name overpromised.
//! That sweep now carries the stride template itself; this suite pins
//! the registry-level contract at every length around the stride,
//! both verdicts typed: below it the 0x47 sync anchors `mpegts`
//! (the walk-will-tell hit), past it without a second sync the typed
//! `probe.no-demuxer` refusal (stage Probe, CLI exit 65 via the
//! existing exit table).

use media_probe::{ProbeConfig, ProbeRegistry, SourceMeta};
use media_types::PipelineStage;
use testkit::Must;

/// A 0x47-led probe window of `len` bytes (sync byte + zeros — the
/// audit's crafted shape).
fn ts_window(len: usize) -> Vec<u8> {
    let mut window = vec![0_u8; len];
    if !window.is_empty() {
        window[0] = 0x47;
    }
    window
}

/// Every window length from empty to one-past the 188-byte stride:
/// each answers typed — `mpegts` anchored below the stride (the
/// single-packet walk-will-tell hit), the typed no-demuxer refusal
/// at and past it without the second sync — and no length panics
/// (the suite completing IS that proof).
#[test]
fn fx2_a04_ts_stride_boundary_lengths_are_typed() {
    let registry = ProbeRegistry::with_builtins();
    let config = ProbeConfig::new(4096, 60).must();
    let meta = SourceMeta::new(None, Some("bin"));
    for len in 0..=189usize {
        match registry.probe(&ts_window(len), &meta, &config) {
            Ok(report) => {
                assert_eq!(report.format.as_str(), "mpegts", "len {len}");
            }
            Err(err) => {
                assert_eq!(err.context.code, "probe.no-demuxer", "len {len}");
                assert_eq!(err.stage, PipelineStage::Probe, "len {len}");
            }
        }
    }
}

/// The stride check's yes side: 189 bytes with the second sync at
/// [188] — `mpegts` anchored through the stride, the whole window
/// examined.
#[test]
fn fx2_a04_ts_stride_second_sync_anchors_mpegts() {
    let registry = ProbeRegistry::with_builtins();
    let config = ProbeConfig::new(4096, 60).must();
    let meta = SourceMeta::new(None, Some("bin"));
    let mut window = ts_window(189);
    window[188] = 0x47;
    let report = registry.probe(&window, &meta, &config).must();
    assert_eq!(report.format.as_str(), "mpegts");
    assert_eq!(report.bytes_examined, 189);
}
