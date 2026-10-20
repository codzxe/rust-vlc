// fuzz_codec_pcm — the PCM codec fuzz target (card 2.B5 `+FB`).
//
// Registry: doc 04 §8.5 (table §3.1 row "PCM", fuzz target
// `fuzz_codec_pcm`) — "Directory fuzz/, cargo-fuzz, one target per
// parser named exactly as in the tables above".
//
// WIRING STATUS (honest, 2026-10-20): the harness LOGIC lives in
// `codecs::audio::pcm::fuzz::fuzz_codec_pcm` (std-only,
// panic-free by construction, memory bounded by the packet's own
// bytes — every PCM conversion is a width- and order-preserving
// permutation of the input, see that module's docs) and is proven
// today by the in-crate deterministic smoke tests plus the
// committed-corpus survival run in
// `crates/codecs/tests/pcm_fuzz_survival.rs` (both class-A parents'
// full audio tracks, systematic truncations and bit flips at sample
// and sub-sample boundaries, adversarial format configurations).
// Building THIS file needs the cargo-fuzz scaffolding
// (`fuzz/Cargo.toml` + the `libfuzzer-sys` dev-dependency), which is
// a Lead-owned dependency decision under AGENTS.md §3.2 / C5 — until
// it is approved and wired, this target is registered by name and
// shape but not yet compiled in CI (same status as `fuzz_demux_avi`
// / `fuzz_codec_mjpeg`, worklogs 2.B2/2.B4). Seed corpus when that
// happens: the 60 committed PCM chunks from both class-A parents
// (CC0, tests/fixtures/CORPUS.md) plus truncations at every
// sample-frame boundary and the class-B derivative files.
//
// Build (once wired):  cargo +nightly fuzz run fuzz_codec_pcm

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    codecs::audio::pcm::fuzz::fuzz_codec_pcm(data);
});
