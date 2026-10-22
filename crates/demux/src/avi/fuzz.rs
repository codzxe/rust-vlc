//! The AVI fuzz harness (card 2.B2 `+FB`; doc 04 §8.5 registry name
//! `fuzz_demux_avi`).
//!
//! One call = one full untrusted-container pass: construct, `start`,
//! drain every event to end-of-stream (bounded), then probe seeks at
//! data-derived targets and drain a little after each. **The harness
//! contract is absence-of-panic/hang/unbounded-allocation, nothing
//! else** — malformed input is *expected* to produce typed errors,
//! which are swallowed here; the conformance suite (integration tests)
//! pins the specific error flavors.
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz target file lives at
//! `fuzz/fuzz_targets/fuzz_demux_avi.rs` exactly as doc 04 §8.5
//! registers it. Building it needs `libfuzzer-sys` (an external
//! dev-dependency) plus the `fuzz/Cargo.toml` scaffolding — both are
//! Lead-owned dependency decisions (AGENTS.md §3.2), so until they
//! land the harness logic is proven by the in-crate deterministic
//! smoke tests here and by the integration corpus run; the libFuzzer
//! campaign itself is owed CI wiring.
//!
//! # Bounds (MP §12)
//!
//! - Event budget: every loop in here has a hard cap; the `movi` walk
//!   itself is monotone (positions only advance), so the budget is a
//!   second belt over a structurally-bounded first one.
//! - Payload memory: the demuxer's own ceilings (`MAX_CHUNK_PAYLOAD`,
//!   `MAX_IDX1_ENTRIES`) apply — this harness adds none.

use media_types::time::{Stamp, TimeBase, TimeSpan, Timestamp};
use plugin_api::{DemuxEvent, Demuxer};

use super::test_support;
use crate::avi::AviDemuxer;

/// Maximum events drained per pass `[PROPOSAL: 200_000 — two orders
/// of magnitude above the largest sane AVI's chunk count; the walk's
/// monotonicity makes hitting this a bug, not media]`.
pub const FUZZ_EVENT_BUDGET: usize = 200_000;

/// Events drained after each seek probe `[PROPOSAL: 64 — enough to
/// exercise post-landing damage paths without dominating the run]`.
pub const FUZZ_POST_SEEK_DRAIN: usize = 64;

/// Number of data-derived seek probes per pass.
pub const FUZZ_SEEK_PROBES: usize = 4;

/// Run one fuzz pass over `data` (see the module docs). Never panics,
/// never hangs, never allocates without a ceiling — by construction of
/// the demuxer and the budgets above.
pub fn fuzz_demux_avi(data: &[u8]) {
    let Ok(mut demuxer) = AviDemuxer::new(match test_support::mem_stream(data.to_vec()) {
        Ok(stream) => stream,
        Err(_) => return,
    }) else {
        return;
    };
    if demuxer.start().is_err() {
        return;
    }
    let mut events = 0usize;
    loop {
        match demuxer.next_event() {
            Ok(DemuxEvent::EndOfStream) | Err(_) => break,
            Ok(_) => {
                events += 1;
                if events >= FUZZ_EVENT_BUDGET {
                    break;
                }
            }
        }
    }

    // Seek probes: targets derived from the input itself (leading
    // bytes), expressed on a nanosecond base the demuxer converts per
    // stream. Out-of-range answers are as valid as landings — the
    // harness only asserts survivability and that a landing state
    // still drains. A `start` that succeeded implies ≥ 12 bytes, but
    // the harness stays panic-free by construction, not by inference:
    // the seed read is guarded independently.
    let Ok(nanos) = TimeBase::new(1, 1_000_000_000) else {
        return;
    };
    let seed = if data.len() >= 8 {
        u64::from_be_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ])
    } else {
        0
    };
    // The probe targets are non-negative and saturate within i64.
    let targets: [i64; FUZZ_SEEK_PROBES] = [
        0,
        (seed >> 1) as i64,
        (seed >> 2) as i64,
        (seed >> 3) as i64,
    ];
    for ticks in targets {
        let target = TimeSpan {
            start: Stamp::Present(Timestamp::new(ticks, nanos)),
            end: Stamp::Missing,
        };
        if demuxer.seek(target).is_ok() {
            for _ in 0..FUZZ_POST_SEEK_DRAIN {
                match demuxer.next_event() {
                    Ok(DemuxEvent::EndOfStream) | Err(_) => break,
                    Ok(_) => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-rolled deterministic scrambler (xorshift64*) — same shape
    /// the fixture generator uses; no external rand dependency.
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
        fuzz_demux_avi(&[]);
        fuzz_demux_avi(b"RI");
        fuzz_demux_avi(b"RIFF\x00\x00\x00\x00AVI ");
        fuzz_demux_avi(&[0xff; 4096]);
    }

    #[test]
    fn scrambled_structured_prefixes_survive() {
        // A plausible AVI prefix, scrambled deterministically at
        // increasing severity — the harness must swallow every flavor.
        let prefix: Vec<u8> = {
            let mut b = Vec::new();
            b.extend_from_slice(b"RIFF");
            b.extend_from_slice(&4_000u32.to_le_bytes());
            b.extend_from_slice(b"AVI ");
            b.extend_from_slice(b"LIST");
            b.extend_from_slice(&3_000u32.to_le_bytes());
            b.extend_from_slice(b"hdrl");
            b.extend_from_slice(b"avih");
            b.extend_from_slice(&56u32.to_le_bytes());
            b.extend_from_slice(&[0u8; 56]);
            b.extend_from_slice(b"LIST");
            b.extend_from_slice(&100u32.to_le_bytes());
            b.extend_from_slice(b"strl");
            b.extend_from_slice(b"strh");
            b.extend_from_slice(&56u32.to_le_bytes());
            b.extend_from_slice(&[0u8; 56]);
            b.extend_from_slice(b"LIST");
            b.extend_from_slice(&500u32.to_le_bytes());
            b.extend_from_slice(b"movi");
            b.extend_from_slice(b"00dc");
            b.extend_from_slice(&10u32.to_le_bytes());
            b.extend_from_slice(&[0x42; 10]);
            b.extend_from_slice(b"idx1");
            b.extend_from_slice(&16u32.to_le_bytes());
            b.extend_from_slice(&[0u8; 16]);
            b
        };
        for flips in 0..64u32 {
            let mut data = prefix.clone();
            let mut rng = Scrambler(0x9E37_79B9_7F4A_7C15 ^ u64::from(flips));
            for _ in 0..flips {
                rng.flip_byte(&mut data);
            }
            fuzz_demux_avi(&data);
        }
    }

    #[test]
    fn adversarial_size_fields_survive() {
        // Extreme declared sizes at every interesting site: max u32,
        // zero, one, odd-with-pad, and sizes that overflow the parent
        // extents. Built by patching a small well-formed core.
        let mut core: Vec<u8> = {
            let mut b = Vec::new();
            b.extend_from_slice(b"RIFF");
            b.extend_from_slice(&0u32.to_le_bytes());
            b.extend_from_slice(b"AVI ");
            b.extend_from_slice(b"LIST");
            b.extend_from_slice(&4u32.to_le_bytes());
            b.extend_from_slice(b"hdrl");
            b.extend_from_slice(b"LIST");
            b.extend_from_slice(&4u32.to_le_bytes());
            b.extend_from_slice(b"movi");
            b.extend_from_slice(b"00dc");
            b.extend_from_slice(&2u32.to_le_bytes());
            b.extend_from_slice(&[0xAA, 0xBB]);
            b
        };
        let end = core.len() as u32;
        core[4..8].copy_from_slice(&end.to_le_bytes());
        for patch in [
            (4u32, u32::MAX),
            (12, u32::MAX),
            (16, 0),
            (16, 1),
            (20, 0),
            (24, u32::MAX),
            (28, u32::MAX),
            (28, 1),
        ] {
            let mut data = core.clone();
            data[patch.0 as usize..patch.0 as usize + 4].copy_from_slice(&patch.1.to_le_bytes());
            fuzz_demux_avi(&data);
        }
        // And the unpatched core itself.
        fuzz_demux_avi(&core);
    }

    #[test]
    fn scrambled_extensible_audio_headers_survive() {
        // A well-formed AVI whose audio strf is the 40-byte
        // WAVEFORMATEXTENSIBLE ffmpeg writes (the pcm stream-recognition
        // follow-up's new header path), scrambled at increasing
        // severity — every mutation of the tag/GUID/width fields must
        // stay inside the typed skip/truncate vocabulary, never a
        // panic (D-3 dirty-world on the new parse surface).
        let strf: Vec<u8> = {
            let mut v = Vec::new();
            v.extend_from_slice(&0xFFFEu16.to_le_bytes()); // extensible
            v.extend_from_slice(&8u16.to_le_bytes()); // channels
            v.extend_from_slice(&11_025u32.to_le_bytes()); // rate
            v.extend_from_slice(&264_600u32.to_le_bytes()); // avg bytes
            v.extend_from_slice(&24u16.to_le_bytes()); // block align
            v.extend_from_slice(&24u16.to_le_bytes()); // bits
            v.extend_from_slice(&22u16.to_le_bytes()); // cbSize
            v.extend_from_slice(&24u16.to_le_bytes()); // valid bits
            v.extend_from_slice(&0x63fu32.to_le_bytes()); // channel mask
            v.extend_from_slice(&[
                0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38,
                0x9b, 0x71,
            ]);
            v
        };
        let prefix: Vec<u8> = {
            let mut b = Vec::new();
            b.extend_from_slice(b"RIFF");
            b.extend_from_slice(&0u32.to_le_bytes());
            b.extend_from_slice(b"AVI ");
            b.extend_from_slice(b"LIST");
            let hdrl_size_pos = b.len();
            b.extend_from_slice(&0u32.to_le_bytes());
            b.extend_from_slice(b"hdrl");
            b.extend_from_slice(b"avih");
            b.extend_from_slice(&56u32.to_le_bytes());
            b.extend_from_slice(&[0u8; 56]);
            b.extend_from_slice(b"LIST");
            let strl_size_pos = b.len();
            b.extend_from_slice(&0u32.to_le_bytes());
            b.extend_from_slice(b"strl");
            b.extend_from_slice(b"strh");
            b.extend_from_slice(&56u32.to_le_bytes());
            let mut strh = vec![0u8; 56];
            strh[0..4].copy_from_slice(b"auds");
            strh[20..24].copy_from_slice(&1u32.to_le_bytes()); // scale
            strh[24..28].copy_from_slice(&11_025u32.to_le_bytes()); // rate
            strh[32..36].copy_from_slice(&200u32.to_le_bytes()); // length
            strh[44..48].copy_from_slice(&24u32.to_le_bytes()); // sample size
            b.extend_from_slice(&strh);
            b.extend_from_slice(b"strf");
            b.extend_from_slice(&(strf.len() as u32).to_le_bytes());
            b.extend_from_slice(&strf);
            let strl_size = b.len() - strl_size_pos - 4;
            b[strl_size_pos..strl_size_pos + 4].copy_from_slice(&(strl_size as u32).to_le_bytes());
            let hdrl_size = b.len() - hdrl_size_pos - 4;
            b[hdrl_size_pos..hdrl_size_pos + 4].copy_from_slice(&(hdrl_size as u32).to_le_bytes());
            b.extend_from_slice(b"LIST");
            b.extend_from_slice(&4u32.to_le_bytes());
            b.extend_from_slice(b"movi");
            b.extend_from_slice(b"00wb");
            b.extend_from_slice(&48u32.to_le_bytes());
            b.extend_from_slice(&[0x55; 48]);
            let riff = b.len() - 8;
            b[4..8].copy_from_slice(&(riff as u32).to_le_bytes());
            b
        };
        // The clean file first: it must fully walk (sanity for the
        // fixture itself, exercised in detail by the unit suite).
        fuzz_demux_avi(&prefix);
        for flips in 0..64u32 {
            let mut data = prefix.clone();
            let mut rng = Scrambler(0xD1B5_4A32_E743_1C59 ^ u64::from(flips));
            for _ in 0..flips {
                rng.flip_byte(&mut data);
            }
            fuzz_demux_avi(&data);
        }
        // Truncations inside the strf region (its payload spans
        // [172, 212) in this layout: cuts after 10 bytes, inside the
        // 16-byte prefix, inside the GUID, and past the whole strf):
        // every cut is a typed truncation or a contained skip, never
        // a panic.
        for cut_at in [160usize, 178, 182, 190, 198, 202, 206, 214] {
            let mut data = prefix.clone();
            data.truncate(cut_at);
            fuzz_demux_avi(&data);
        }
    }
}
