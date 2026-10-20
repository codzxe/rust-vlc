// fuzz_codec_mjpeg — the Motion-JPEG / baseline-JPEG codec fuzz
// target (card 2.B4 `+FB`).
//
// Registry: doc 04 §8.5 (table §4 row "Motion-JPEG", fuzz target
// `fuzz_codec_mjpeg`) — "Directory fuzz/, cargo-fuzz, one target per
// parser named exactly as in the tables above".
//
// WIRING STATUS (honest, 2026-10-20): the harness LOGIC lives in
// `codecs::video::mjpeg::fuzz::fuzz_codec_mjpeg` (std-only,
// panic-free by construction, memory bounded by the decoder's own
// MAX_PIXELS/per-plane ceilings — see that module's docs) and is
// proven today by the in-crate deterministic smoke tests plus the
// committed-corpus survival run in
// `crates/codecs/tests/fuzz_survival.rs` (all 22 fixture files, all
// 120 extracted frames, systematic truncations and single-byte flips
// of a real frame, adversarial near-cap geometries). Building THIS
// file needs the cargo-fuzz scaffolding (`fuzz/Cargo.toml` + the
// `libfuzzer-sys` dev-dependency), which is a Lead-owned dependency
// decision under AGENTS.md §3.2 / C5 — until it is approved and
// wired, this target is registered by name and shape but not yet
// compiled in CI (same status as `fuzz_demux_avi`, worklog 2.B2).
// Seed corpus when that happens: the 60 committed JPEG frames from
// both class-A parents (CC0, tests/fixtures/CORPUS.md) plus
// truncations of frame 0 at every 128-byte boundary and the class-B
// derivative files.
//
// Build (once wired):  cargo +nightly fuzz run fuzz_codec_mjpeg

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    codecs::video::mjpeg::fuzz::fuzz_codec_mjpeg(data);
});
