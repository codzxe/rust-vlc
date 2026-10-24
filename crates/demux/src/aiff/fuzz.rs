//! The AIFF fuzz harness (card 5.A2 `+FB`; doc 04 §8.5 registry name
//! `fuzz_demux_aiff`).
//!
//! One call = one full untrusted-container pass: construct, `start`,
//! drain every event to end-of-stream (bounded), then probe seeks at
//! data-derived targets and drain a little after each. **The harness
//! contract is absence-of-panic/hang/unbounded-allocation, nothing
//! else** — malformed input is *expected* to produce typed errors,
//! which are swallowed here; the conformance suite pins the specific
//! error flavors.
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz target file (`fuzz/fuzz_targets/fuzz_demux_aiff.rs`)
//! is a Lead-owned dependency decision (`libfuzzer-sys`), exactly as
//! the AVI/WAV cards recorded; until it lands the harness logic is
//! proven by the deterministic in-crate sweeps here and by the
//! integration corpus run over the mutated fixture family.
//!
//! # Bounds (MP §12)
//!
//! - Event budget: every loop has a hard cap; the sound walk is
//!   monotone (positions only advance).
//! - Payload memory: the demuxer's own ceilings
//!   ([`MAX_PACKET_BYTES`](crate::aiff::MAX_PACKET_BYTES),
//!   [`chunks::MAX_COMM_BYTES`](crate::aiff::chunks::MAX_COMM_BYTES))
//!   apply — this harness adds none.

use media_types::time::TimeSpan;
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};

use crate::aiff::AiffDemuxer;

/// Maximum events drained per pass `[PROPOSAL: 200_000 — the WAV
/// twin's budget]`.
pub const FUZZ_EVENT_BUDGET: usize = 200_000;

/// Events drained after each seek probe `[PROPOSAL: 64]`.
pub const FUZZ_POST_SEEK_DRAIN: usize = 64;

/// Number of data-derived seek probes per pass.
pub const FUZZ_SEEK_PROBES: usize = 4;

/// Run one fuzz pass over `data` (see the module docs). Never panics,
/// never hangs, never allocates without a ceiling — by construction
/// of the demuxer and the budgets above.
pub fn fuzz_demux_aiff(data: &[u8]) {
    let Ok(mut demuxer) =
        AiffDemuxer::new(match crate::avi::test_support::mem_stream(data.to_vec()) {
            Ok(stream) => stream,
            Err(_) => return,
        })
    else {
        return;
    };
    if demuxer.start().is_err() {
        return;
    }
    let mut drained = 0usize;
    let mut last_pts_ticks = 0i64;
    loop {
        if drained >= FUZZ_EVENT_BUDGET {
            return;
        }
        drained += 1;
        match demuxer.next_event() {
            Ok(DemuxEvent::Packet(packet)) => {
                if let media_types::time::Stamp::Present(ts) = packet.pts {
                    last_pts_ticks = ts.ticks;
                }
            }
            Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => continue,
            Ok(DemuxEvent::EndOfStream) | Err(_) => break,
        }
    }

    // Seek probes at data-derived targets (the WAV twin).
    if demuxer.seekability() == SeekSupport::None {
        return;
    }
    let Some(base_rate) = demuxer.comm().map(|comm| comm.sample_rate) else {
        return;
    };
    let Ok(base) = media_types::time::TimeBase::new(1, base_rate) else {
        return;
    };
    let span = last_pts_ticks.saturating_mul(4);
    for quarter in 0..=FUZZ_SEEK_PROBES {
        let ticks = span / FUZZ_SEEK_PROBES as i64 * quarter as i64;
        let ts = media_types::time::Timestamp::new(ticks, base);
        let target = TimeSpan {
            start: media_types::time::Stamp::Present(ts),
            end: media_types::time::Stamp::Missing,
        };
        if demuxer.seek(target).is_ok() && demuxer.reanchor(target).is_ok() {
            for _ in 0..FUZZ_POST_SEEK_DRAIN {
                match demuxer.next_event() {
                    Ok(DemuxEvent::EndOfStream) | Err(_) => break,
                    Ok(_) => continue,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiff::test_support::minimal_aiff;

    /// Hand-rolled deterministic scrambler (xorshift64*) — the AVI
    /// harness's shape; no external rand dependency.
    struct Scrambler(u64);

    impl Scrambler {
        fn next_u64(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }

        fn flip_byte(&mut self, data: &mut [u8]) {
            if data.is_empty() {
                return;
            }
            let index = (self.next_u64() % data.len() as u64) as usize;
            let bit = (self.next_u64() % 8) as u8;
            data[index] ^= 1 << bit;
        }
    }

    #[test]
    fn empty_and_tiny_inputs_survive() {
        fuzz_demux_aiff(&[]);
        fuzz_demux_aiff(b"FO");
        fuzz_demux_aiff(b"FORM\x00\x00\x00\x00AIFF");
        fuzz_demux_aiff(&[0xff; 4096]);
    }

    #[test]
    fn scrambled_structured_files_survive() {
        // A plausible AIFF (COMM + MARK + INST + SSND), scrambled at
        // increasing severity — every mutation must stay inside the
        // typed skip/truncate vocabulary, never a panic (D-3).
        let base = minimal_aiff(2, 44_100, 16, &vec![0x33u8; 1_024]);
        for flips in 0..64u32 {
            let mut data = base.to_vec();
            let mut rng = Scrambler(0x9E37_79B9_7F4A_7C15 ^ u64::from(flips));
            for _ in 0..flips {
                rng.flip_byte(&mut data);
            }
            fuzz_demux_aiff(&data);
        }
    }

    #[test]
    fn adversarial_size_fields_survive() {
        // Extreme declared sizes at the interesting sites: the FORM
        // size, the COMM size, the SSND size.
        let base = minimal_aiff(1, 44_100, 16, &[0x11, 0x22]);
        for patch in [
            (4u32, u32::MAX), // FORM size
            (16, u32::MAX),   // COMM size (at 12 + 4)
            (34, 0),          // SSND size (COMM is 8 + 18 = 26 → SSND header at 30, size at 34)
            (34, u32::MAX),
        ] {
            let mut data = base.to_vec();
            let at = patch.0 as usize;
            if at + 4 <= data.len() {
                data[at..at + 4].copy_from_slice(&patch.1.to_be_bytes());
                fuzz_demux_aiff(&data);
            }
        }
        fuzz_demux_aiff(&base);
    }

    #[test]
    fn truncations_at_every_header_site_survive() {
        let base = minimal_aiff(2, 48_000, 16, &vec![0x5Au8; 512]);
        for cut in [0usize, 1, 8, 11, 12, 16, 20, 26, 30, 34, 38, 42, 100] {
            let mut data = base.to_vec();
            data.truncate(cut);
            fuzz_demux_aiff(&data);
        }
    }
}
