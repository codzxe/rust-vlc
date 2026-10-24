// fuzz_playlist_m3u — the M3U/M3U8/HLS playlist target (card 5.Q1; doc 04 §8.5 + the fuzz/README registry).
//
// Thin libFuzzer shim: the harness body is the in-crate `playlists::fuzz`
// module (panic-free by construction, bounds documented there); one
// call = one full untrusted pass over the bytes.
//
// Wiring status (honest): building the campaign needs the Lead's
// nightly wiring (see fuzz/README.md); until then this package
// compiles on stable (`cargo check` in fuzz/) and the harness logic
// is proven by the in-crate deterministic hostile-input sweeps.
//
// Build (once nightly is wired):  cargo +nightly fuzz run fuzz_playlist_m3u

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    playlists::fuzz::fuzz_playlist_m3u(data);
});
