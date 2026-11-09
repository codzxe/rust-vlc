// fuzz_demux_mpd — the DASH MPD target (cards 6.C1c/6.C1; doc 04
// §8.5 + the fuzz/README registry; the 6.C1 `+FB` acceptance
// criterion's name).
//
// Thin libFuzzer shim: the harness body is the in-crate
// `streaming::dash::fuzz` module (panic-free by construction,
// bounds documented there); one call = one full untrusted parse +
// bounded segment-plan resolution pass over the bytes.
//
// Wiring status (honest): building the campaign needs the Lead's
// nightly wiring (see fuzz/README.md); until then this package
// compiles on stable (`cargo check` in fuzz/) and the harness logic
// is proven by the in-crate deterministic hostile-input sweeps.
//
// Build (once nightly is wired):  cargo +nightly fuzz run fuzz_demux_mpd

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    streaming::dash::fuzz::fuzz_demux_mpd(data);
});
