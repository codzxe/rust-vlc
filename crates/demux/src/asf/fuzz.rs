//! The ASF fuzz harness (card 6.ASF-a; doc 04 §8.5 registry name:
//! `fuzz_demux_asf`).
//!
//! One call = one full untrusted pass: construct → `start` (the
//! header walk) → the trait-order refusals, plus the [`probe`]
//! window gate — the whole surface this unit owns. The harness
//! contract is absence-of-panic/hang/unbounded-allocation, nothing
//! else: damage is *expected* to surface as typed
//! `demux.asf.*` errors, all swallowed here; the unit tests pin the
//! specific outcomes.
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz target lives at
//! `fuzz/fuzz_targets/fuzz_demux_asf.rs` (Lead-owned nightly wiring,
//! see `fuzz/README.md`); the harness logic is meanwhile proven on
//! stable by the deterministic sweeps below.
//!
//! # Bounds (MP §12)
//!
//! The walk's own ceilings apply (header memory ceiling, object
//! count guards, field caps — see `header.rs`); this harness adds a
//! start-attempt budget only.

use plugin_api::Demuxer;

use super::{AsfDemuxer, probe};

/// Drive the full untrusted pass over `data`. Never panics; see the
/// module docs.
pub fn fuzz_demux_asf(data: &[u8]) {
    // The probe window gate first (16 bytes minimum — the same
    // prefix the media-probe signature scores).
    let _ = probe(data);

    let stream = match crate::asf::test_support::mem_stream(data.to_vec()) {
        Ok(stream) => stream,
        Err(_) => return,
    };
    let Ok(mut demuxer) = AsfDemuxer::new(stream) else {
        return;
    };
    let _ = demuxer.seekability();
    if demuxer.start().is_err() {
        return;
    }
    // The ASF-b/c not-yet refusals must be as panic-free as the
    // walk itself.
    let _ = demuxer.next_event();
    let _ = demuxer.seek(media_types::time::TimeSpan::UNKNOWN);
}

/// One deterministic hostile sweep (the in-crate stable-loop
/// survival test — the es/ts convention).
///
/// # Panics
/// Never: the harness is panic-free by construction (the assertions
/// live in the unit tests, not here).
pub fn stable_loop_survival() {
    let templates: Vec<Vec<u8>> = vec![
        // A valid header prefix the mutations corrupt.
        {
            let mut t = Vec::new();
            t.extend_from_slice(super::guid::HEADER.as_bytes());
            t.extend_from_slice(&757u64.to_le_bytes());
            t.extend_from_slice(&[0u8; 48]);
            t
        },
        // The broadcast prefix shape.
        {
            let mut t = vec![0x24, 0x48, 0x2f, 0x03, 0, 0, 0, 0, 0, 0x0c, 0x2f, 0x03];
            t.extend_from_slice(super::guid::HEADER.as_bytes());
            t.extend_from_slice(&[0u8; 48]);
            t
        },
    ];
    let mut budget = 8_192usize;
    for template in &templates {
        for pos in 0..template.len() {
            for value in [0x00u8, 0xFF, 0x7F] {
                if budget == 0 {
                    return;
                }
                budget -= 1;
                let mut window = template.clone();
                window[pos] = value;
                fuzz_demux_asf(&window);
            }
        }
        for cut in 0..=template.len() {
            if budget == 0 {
                return;
            }
            budget -= 1;
            fuzz_demux_asf(&template[..cut]);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn hostile_sweeps_survive_without_panics() {
        super::stable_loop_survival();
    }
}
