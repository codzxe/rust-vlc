// fuzz_hls_playlist — the HLS network-input target (issue #14 M2,
// card 6.F3): hostile bytes from the wire into the full HLS
// interpretation stack.
//
// One call = TWO passes (both panic-free by construction, bounds
// documented where each lives):
//
// 1. `playlists::fuzz::fuzz_playlist_m3u` — the in-crate generic
//    M3U/M3U8 parser harness (BOM/encoding sniff + the line walk).
//    This is the registry-gate-visible in-crate harness call, the
//    same convention as the other rows.
// 2. `netfuzz::fuzz_hls_playlist` — the HLS-specific consumers
//    layered on top (media/master models, the ABR variant pick, the
//    X-TIMESTAMP-MAP wall-clock mapping), `#[path]`-included from
//    `crates/streaming/tests/netfuzz/fuzz.rs` — the wave agent's
//    ownership boundary (no `src/` edits, per the M1 netfault
//    precedent).
//
// Wiring status (honest, same as the other 33): building the
// campaign needs the Lead's nightly wiring (fuzz/README.md); until
// then this package compiles on stable (`cargo check` in fuzz/) and
// the harness logic is proven by the deterministic hostile-input
// sweep `crates/streaming/tests/netfuzz_sweep.rs`.
//
// Seeds: fuzz/corpus/fuzz_hls_playlist/ — the committed real HLS
// playlists from crates/playlists/tests/fixtures/hls/ (master,
// ts_vod, fmp4).
//
// Build (once nightly is wired):  cargo +nightly fuzz run fuzz_hls_playlist

#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "../../crates/streaming/tests/netfuzz/fuzz.rs"]
mod netfuzz;

fuzz_target!(|data: &[u8]| {
    playlists::fuzz::fuzz_playlist_m3u(data);
    netfuzz::fuzz_hls_playlist(data);
});
