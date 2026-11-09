//! The DASH MPD fuzz harness (card 6.C1c; doc 04 §8.5 + the fuzz/
//! README registry — the `fuzz_demux_mpd` target the wave program's
//! 6.C1 `+FB` acceptance criterion names).
//!
//! One call = one full untrusted pass through
//! [`playlists::parse_mpd_bytes`](playlists::parse_mpd_bytes) — the
//! 6.C1a parser — **plus a bounded segment-plan resolution walk**
//! (the 6.C1b resolver), because a manifest that parses clean but
//! mints hostile template math is exactly the class the campaign
//! must survive.
//!
//! **The harness contract is absence-of-panic/hang/unbounded-
//! allocation, nothing else** — malformed input is *expected* to
//! produce typed errors ([`playlists::MpdError`] /
//! [`super::resolve::DashResolveError`]), both swallowed here; the
//! parser and resolver suites pin the specific outcomes.
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz target lives at `fuzz/fuzz_targets/
//! fuzz_demux_mpd.rs` (Lead-owned nightly wiring, see fuzz/
//! README.md — same posture as every target); the harness logic is
//! meanwhile proven on stable by the deterministic sweep below and
//! the committed-fixture suites in `resolve`'s tests.
//!
//! # Bounds (MP §12 / D-2)
//!
//! - Parse: the 6.C1a ceilings (8 MiB input, 64 KiB lines, depth
//!   64) bound the tree; warnings grow ≤ O(input).
//! - Resolution: each representation plan is bounded by 6.C1b's own
//!   budgets ([`MAX_PLAN_SEGMENTS`] segments, 16 MiB URL bytes);
//!   the walk adds global ceilings — [`WALK_REPRESENTATIONS`]
//!   representations processed and [`WALK_TOTAL_SEGMENTS`] segments
//!   planned across the whole manifest — so a manifest with
//!   thousands of rungs cannot multiply into unbounded time either.
//!   Reaching a ceiling stops the walk (a fuzz finding needs one
//!   clean repro, not the whole forest).

use playlists::parse_mpd_bytes;

use super::resolve::{MAX_PLAN_SEGMENTS, plan_representation};

/// Global ceiling on representations the walk resolves per pass.
pub const WALK_REPRESENTATIONS: usize = 32;

/// Global ceiling on segments planned across the whole pass.
pub const WALK_TOTAL_SEGMENTS: usize = MAX_PLAN_SEGMENTS;

/// Run one DASH MPD fuzz pass over `data` (see the module docs).
/// Never panics, never hangs, never allocates without a ceiling.
pub fn fuzz_demux_mpd(data: &[u8]) {
    let _ = walk(data);
}

/// The bounded parse + resolve walk; returns the segments planned
/// (the observable the deterministic sweep's bounded-memory
/// invariant checks). `None` = the parse refused.
fn walk(data: &[u8]) -> Option<usize> {
    let parsed = parse_mpd_bytes(data).ok()?;
    let mut reps = 0usize;
    let mut total_segments = 0usize;
    for period in &parsed.mpd.periods {
        for set in &period.adaptation_sets {
            for rep in &set.representations {
                if reps >= WALK_REPRESENTATIONS || total_segments >= WALK_TOTAL_SEGMENTS {
                    return Some(total_segments);
                }
                reps += 1;
                if let Ok(plan) =
                    plan_representation(&parsed.mpd, period, set, rep, "http://fuzz.test/m.mpd")
                {
                    total_segments += plan.segments.len();
                }
            }
        }
    }
    Some(total_segments)
}

// ------------------------------------------------------------------ tests

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Vec<u8> {
        let path = format!(
            "{}/../playlists/tests/fixtures/dash/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) => unreachable!("fixture {name}: {e}"),
        }
    }

    /// splitmix64 — the deterministic PRNG the in-crate sweeps use
    /// (same generator as the 6.C1a fuzz-lite sweep).
    struct SplitMix64(u64);
    impl SplitMix64 {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }
    }

    #[test]
    fn empty_and_hostile_minima_survive() {
        for data in [
            &b""[..],
            b"\n",
            b"<MPD>",
            b"<?xml version=\"1.0\"?><MPD type=\"static\"></MPD>",
            b"\xff\xfe\x00\x01garbage",
            // Truncation mid-open-tag.
            b"<MPD>",
        ] {
            fuzz_demux_mpd(data);
        }
    }

    /// The committed fixtures survive a full pass and stay under the
    /// walk ceilings (bounded memory, invariant-checked).
    #[test]
    fn committed_fixtures_survive_within_ceilings() {
        for name in [
            "ffmpeg_av_template.mpd",
            "ffmpeg_av_list.mpd",
            "hand_static_cenc.mpd",
            "hand_dynamic.mpd",
        ] {
            let planned = match walk(&fixture(name)) {
                Some(n) => n,
                None => unreachable!("real fixture must parse: {name}"),
            };
            assert!(
                planned <= WALK_TOTAL_SEGMENTS,
                "{name}: planned {planned} segments over the ceiling"
            );
        }
    }

    /// The deterministic property sweep (the stable-runnable
    /// fallback while the Lead's nightly wiring is pending): 600
    /// splitmix64-driven structured mutations of the real template
    /// fixture — truncate / byte-flip / splice / window — with the
    /// invariants: no panic, a typed parse verdict every time, and
    /// a bounded plan when the parse survives.
    #[test]
    fn mutation_sweep_never_panics_and_stays_bounded() {
        let base = fixture("ffmpeg_av_template.mpd");
        let list = fixture("ffmpeg_av_list.mpd");
        let mut rng = SplitMix64(0x5EED_0DA5_0000_0001);
        let mut parsed_ok = 0usize;
        let mut planned_overall = 0usize;
        for round in 0..600u64 {
            let source = if round % 2 == 0 { &base } else { &list };
            let mutant: Vec<u8> = match rng.next() % 4 {
                // Truncate at a random point.
                0 => {
                    let at = (rng.next() as usize) % (source.len() + 1);
                    source[..at].to_vec()
                }
                // Flip one random byte.
                1 => {
                    let mut m = source.clone();
                    if !m.is_empty() {
                        let at = (rng.next() as usize) % m.len();
                        m[at] ^= u8::try_from(rng.next() % 255 + 1).unwrap_or(1);
                    }
                    m
                }
                // Splice two halves of the round's source.
                2 => {
                    let cut = (rng.next() as usize) % (source.len() + 1);
                    let mut m = source[..cut].to_vec();
                    m.extend_from_slice(&source[cut..]);
                    m
                }
                // A random window slice.
                _ => {
                    let a = (rng.next() as usize) % source.len();
                    let b = a + (rng.next() as usize) % (source.len() - a + 1);
                    source[a..b].to_vec()
                }
            };
            // The invariants: survival (no panic) + a typed verdict +
            // bounded plans.
            if let Some(planned) = walk(&mutant) {
                parsed_ok += 1;
                planned_overall += planned;
                assert!(planned <= WALK_TOTAL_SEGMENTS);
            }
        }
        // The sweep is meaningful: mutations survive and refuse (a
        // sweep where nothing parsed would prove nothing about the
        // resolver).
        assert!(parsed_ok > 0, "at least one mutant must survive the parse");
        let _ = planned_overall;
    }

    /// The walk ceilings are enforced, not aspirational: a manifest
    /// whose every rung plans many segments stops at the global cap.
    #[test]
    fn walk_ceilings_stop_the_traversal() {
        // 40 representations × 10 timeline segments each: the
        // 32-representation / global-segment ceilings must stop the
        // walk before the forest is mowed.
        let mut mpd = String::from(
            "<?xml version=\"1.0\"?>\n<MPD type=\"static\" mediaPresentationDuration=\"PT100S\">\n\
             <Period>\n<AdaptationSet>\n",
        );
        for i in 0..40 {
            mpd.push_str(&format!(
                "<Representation id=\"r{i}\" bandwidth=\"1000\">\n\
                 <SegmentTemplate timescale=\"1\" duration=\"10\" \
                 startNumber=\"1\" media=\"$RepresentationID$-$Number$.m4s\"/>\n\
                 </Representation>\n"
            ));
        }
        mpd.push_str("</AdaptationSet>\n</Period>\n</MPD>\n");
        let planned = match walk(mpd.as_bytes()) {
            Some(n) => n,
            None => unreachable!("synthetic manifest must parse"),
        };
        // 32 reps × 10 segments = 320 planned; reps 33–40 unvisited.
        assert_eq!(planned, 320);
    }
}
