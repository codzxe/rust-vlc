//! The raw elementary-stream fuzz harnesses (card 5.Q1; doc 04 §8.5
//! registry names: `fuzz_demux_es` for the headerless family and
//! `fuzz_demux_mp3stream`, table §2.1 row "MP3 raw stream").
//!
//! One call = one full untrusted pass per lane:
//!
//! - [`fuzz_demux_es`] drives all three `open`-constructed lanes —
//!   H.264 Annex-B, HEVC Annex-B and raw AAC (= the ADTS demuxer) —
//!   through `start` + a budgeted event drain, plus each lane's
//!   `probe` over the leading window;
//! - [`fuzz_demux_mp3stream`] drives the raw MPEG-audio lane
//!   (`RawMp3Demuxer`: ID3 skip, header walk, framing arithmetic)
//!   through the same construct/start/drain contract, plus
//!   [`parse_head`](super::mp3::parse_head) probes at input-derived
//!   offsets.
//!
//! **The harness contract is absence-of-panic/hang/unbounded-
//! allocation, nothing else** — damage is *expected* to surface as
//! typed errors (`demux.es.*`) or concealed/truncated events, all
//! swallowed here; `es_corpus` pins the specific outcomes.
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz targets live at `fuzz/fuzz_targets/fuzz_demux_es.rs`
//! and `fuzz/fuzz_targets/fuzz_demux_mp3stream.rs` (Lead-owned nightly
//! wiring, see `fuzz/README.md`); the harness logic is meanwhile
//! proven on stable by the deterministic sweeps below and the crate's
//! ES corpus run.
//!
//! # Bounds (MP §12)
//!
//! Event budget: hard cap below, per lane; the walks are monotone.
//! Memory: the lanes' own ceilings (AU cap, damage-log cap) apply —
//! these harnesses add none.

use plugin_api::{DemuxEvent, Demuxer};

/// Maximum events drained per lane per pass `[PROPOSAL: 200_000]`.
pub const FUZZ_EVENT_BUDGET: usize = 200_000;

/// Construct/start/drain one ES lane (the shared body of the annexb
/// and raw-AAC lanes). Never panics; see the module docs.
fn drain_lane(
    opened: Result<impl Demuxer, media_types::EngineError>,
) {
    let Ok(mut demuxer) = opened else {
        return;
    };
    if demuxer.start().is_err() {
        return;
    }
    let mut drained = 0usize;
    loop {
        if drained >= FUZZ_EVENT_BUDGET {
            return;
        }
        drained += 1;
        match demuxer.next_event() {
            Ok(DemuxEvent::Packet(_))
            | Ok(DemuxEvent::Recovered { .. })
            | Ok(DemuxEvent::Discontinuity) => continue,
            Ok(DemuxEvent::EndOfStream) | Err(_) => break,
        }
    }
}

/// Run one fuzz pass over `data` across the annexb + raw-AAC lanes
/// (see the module docs). Never panics, never hangs, never allocates
/// without a ceiling.
pub fn fuzz_demux_es(data: &[u8]) {
    // The probes (cheap head-window classification).
    let head = &data[..data.len().min(16)];
    let _ = super::h264::probe(head);
    let _ = super::hevc::probe(head);
    let _ = super::aac::probe(head);
    // The three open-constructed lanes, each a full drain (one
    // shared stream clone per lane — the mem stream is seekable and
    // length-known, exactly the lanes' contract).
    if let Ok(stream) = crate::avi::test_support::mem_stream(data.to_vec()) {
        drain_lane(super::h264::open(stream));
    }
    if let Ok(stream) = crate::avi::test_support::mem_stream(data.to_vec()) {
        drain_lane(super::hevc::open(stream));
    }
    if let Ok(stream) = crate::avi::test_support::mem_stream(data.to_vec()) {
        drain_lane(super::aac::open(stream));
    }
}

/// Run one fuzz pass over `data` through the raw MPEG-audio lane
/// (see the module docs). Same contract.
pub fn fuzz_demux_mp3stream(data: &[u8]) {
    // Header-parse probes at input-derived offsets (the frame
    // header's 4-byte window under hostile bytes).
    if data.len() >= 4 {
        for i in 0..8u32 {
            let at = (usize::try_from(i * 97).unwrap_or(0)).min(data.len() - 4);
            let _ = super::mp3::parse_head(&data[at..at + 4]);
        }
    }
    if let Ok(stream) = crate::avi::test_support::mem_stream(data.to_vec()) {
        drain_lane(super::mp3::RawMp3Demuxer::new(stream));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_marker_inputs_survive() {
        fuzz_demux_es(&[]);
        fuzz_demux_es(&[0x00, 0x00, 0x00, 0x01]);
        fuzz_demux_mp3stream(&[]);
        fuzz_demux_mp3stream(&[0xFF, 0xFB]);
    }

    #[test]
    fn hostile_wide_inputs_survive() {
        // All-0x00: start-code floods / zero-length fields; all-0xFF:
        // maximal sync false positives (both families).
        for hostile in [&vec![0x00u8; 4096][..], &vec![0xffu8; 4096][..]] {
            fuzz_demux_es(hostile);
            fuzz_demux_mp3stream(hostile);
        }
        for length in 1..=64usize {
            let data: Vec<u8> = (0..length).map(|i| (i * 41 % 256) as u8).collect();
            fuzz_demux_es(&data);
            fuzz_demux_mp3stream(&data);
        }
    }
}
