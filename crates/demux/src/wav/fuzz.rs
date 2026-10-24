//! The WAV fuzz harness (card 5.A2 `+FB`; doc 04 §8.5 registry name
//! `fuzz_demux_wav`).
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
//! The cargo-fuzz target file (`fuzz/fuzz_targets/fuzz_demux_wav.rs`)
//! is a Lead-owned dependency decision (`libfuzzer-sys`), exactly as
//! the AVI card recorded; until it lands the harness logic is proven
//! by the deterministic in-crate sweeps here and by the integration
//! corpus run over the mutated fixture family.
//!
//! # Bounds (MP §12)
//!
//! - Event budget: every loop has a hard cap; the data walk is
//!   monotone (positions only advance), so the budget is a second
//!   belt over a structurally-bounded first one.
//! - Payload memory: the demuxer's own ceilings
//!   ([`MAX_PACKET_BYTES`](crate::wav::MAX_PACKET_BYTES),
//!   [`format::MAX_FMT_BYTES`](crate::wav::format::MAX_FMT_BYTES),
//!   [`MAX_INFO_BYTES`](crate::wav::MAX_INFO_BYTES)) apply — this
//!   harness adds none.

use media_types::time::TimeSpan;
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};

use super::test_support;
use crate::wav::WavDemuxer;

/// Maximum events drained per pass `[PROPOSAL: 200_000 — two orders
/// of magnitude above the largest sane WAV's packet count at the
/// 64 KiB packet budget; the walk's monotonicity makes hitting this a
/// bug, not media]`.
pub const FUZZ_EVENT_BUDGET: usize = 200_000;

/// Events drained after each seek probe `[PROPOSAL: 64 — enough to
/// exercise post-landing paths without dominating the run]`.
pub const FUZZ_POST_SEEK_DRAIN: usize = 64;

/// Number of data-derived seek probes per pass.
pub const FUZZ_SEEK_PROBES: usize = 4;

/// Run one fuzz pass over `data` (see the module docs). Never panics,
/// never hangs, never allocates without a ceiling — by construction
/// of the demuxer and the budgets above.
pub fn fuzz_demux_wav(data: &[u8]) {
    let Ok(mut demuxer) = WavDemuxer::new(match test_support::mem_stream(data.to_vec()) {
        Ok(stream) => stream,
        Err(_) => return,
    }) else {
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

    // Seek probes at data-derived targets: zero, a quarter of the
    // observed pts span, its middle and a past-the-end stamp. Only
    // meaningful when the demuxer claims a seekability.
    if demuxer.seekability() == SeekSupport::None {
        return;
    }
    let Some(base_rate) = demuxer.format().map(|fmt| fmt.samples_per_sec) else {
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
        if let Ok(landing) = demuxer.seek(target) {
            // A landing that maps commits through reanchor, then
            // drains a bounded window from the new position.
            if demuxer.reanchor(target).is_ok() {
                let _ = landing;
                for _ in 0..FUZZ_POST_SEEK_DRAIN {
                    match demuxer.next_event() {
                        Ok(DemuxEvent::EndOfStream) | Err(_) => break,
                        Ok(_) => continue,
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wav::test_support::minimal_pcm_wav;

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
        fuzz_demux_wav(&[]);
        fuzz_demux_wav(b"RI");
        fuzz_demux_wav(b"RIFF\x00\x00\x00\x00WAVE");
        fuzz_demux_wav(&[0xff; 4096]);
    }

    #[test]
    fn scrambled_structured_files_survive() {
        // A plausible WAV (fmt + LIST INFO + data), scrambled at
        // increasing severity — every mutation must stay inside the
        // typed skip/truncate vocabulary, never a panic (D-3).
        let base = minimal_pcm_wav(1, 2, 44_100, 16, &vec![0x33u8; 1_024]);
        for flips in 0..64u32 {
            let mut data = base.to_vec();
            let mut rng = Scrambler(0xD1B5_4A32_E743_1C59 ^ u64::from(flips));
            for _ in 0..flips {
                rng.flip_byte(&mut data);
            }
            fuzz_demux_wav(&data);
        }
    }

    #[test]
    fn scrambled_extensible_headers_survive() {
        // The 40-byte WAVEFORMATEXTENSIBLE ffmpeg writes (the corpus
        // 0004/0006/0095/0129/0138 shape), scrambled — every
        // mutation of the tag/GUID/width fields must stay inside the
        // typed refusal vocabulary (D-3 on the extensible parse
        // surface).
        let mut fmt = Vec::new();
        fmt.extend_from_slice(&0xFFFEu16.to_le_bytes());
        fmt.extend_from_slice(&2u16.to_le_bytes());
        fmt.extend_from_slice(&48_000u32.to_le_bytes());
        fmt.extend_from_slice(&288_000u32.to_le_bytes());
        fmt.extend_from_slice(&6u16.to_le_bytes());
        fmt.extend_from_slice(&24u16.to_le_bytes());
        fmt.extend_from_slice(&22u16.to_le_bytes());
        fmt.extend_from_slice(&24u16.to_le_bytes());
        fmt.extend_from_slice(&0x3u32.to_le_bytes());
        fmt.extend_from_slice(&crate::wav::format::SUBFORMAT_PCM_GUID);
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(b"WAVE");
        b.extend_from_slice(b"fmt ");
        b.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
        b.extend_from_slice(&fmt);
        b.extend_from_slice(b"data");
        b.extend_from_slice(&512u32.to_le_bytes());
        b.extend_from_slice(&[0x55u8; 512]);
        let riff = b.len() - 8;
        b[4..8].copy_from_slice(&(riff as u32).to_le_bytes());
        for flips in 0..64u32 {
            let mut data = b.clone();
            let mut rng = Scrambler(0x1234_5678_9ABC_DEF0 ^ u64::from(flips));
            for _ in 0..flips {
                rng.flip_byte(&mut data);
            }
            fuzz_demux_wav(&data);
        }
        // Truncations inside the fmt region: every cut is a typed
        // truncation or refusal, never a panic.
        for cut in [12usize, 16, 20, 30, 40, 50, 52, 60, 64] {
            let mut data = b.clone();
            data.truncate(cut);
            fuzz_demux_wav(&data);
        }
    }

    #[test]
    fn adversarial_size_fields_survive() {
        // Extreme declared sizes at the interesting sites: the RIFF
        // size, the fmt size, the data size, the sentinel.
        let base = minimal_pcm_wav(1, 1, 44_100, 16, &[0x11, 0x22]);
        for patch in [
            (4u32, u32::MAX),
            (4, 0),
            (16, u32::MAX), // fmt size (at 12 + 4)
            (16, 0),
            (16, 1),
            (28, u32::MAX), // data size (at 24 + 4)
            (28, 0),
            (28, 1),
        ] {
            let mut data = base.to_vec();
            let at = patch.0 as usize;
            if at + 4 <= data.len() {
                data[at..at + 4].copy_from_slice(&patch.1.to_le_bytes());
                fuzz_demux_wav(&data);
            }
        }
        fuzz_demux_wav(&base);
    }

    #[test]
    fn truncations_at_every_header_site_survive() {
        let base = minimal_pcm_wav(1, 2, 48_000, 16, &vec![0x5Au8; 512]);
        for cut in [0usize, 1, 8, 11, 12, 16, 20, 24, 28, 32, 44, 100, 300] {
            let mut data = base.to_vec();
            data.truncate(cut);
            fuzz_demux_wav(&data);
        }
    }
}
