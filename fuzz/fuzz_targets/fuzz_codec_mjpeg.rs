// fuzz_codec_mjpeg — the Motion-JPEG / baseline-JPEG codec fuzz
// target (card 2.B4 `+FB`).
//
// Registry: doc 04 §8.5 (table §4 row "Motion-JPEG", fuzz target
// `fuzz_codec_mjpeg`) — "Directory fuzz/, cargo-fuzz, one target per
// parser named exactly as in the tables above".
//
// WIRING STATUS (updated 2026-10-22, card W1-12): the cargo-fuzz
// scaffolding now EXISTS — `fuzz/Cargo.toml` (package `lumen-fuzz`,
// `libfuzzer-sys` 0.4.13, this target declared) + `fuzz/README.md`.
// What is still owed is the *nightly wiring* (a Lead dependency
// decision per AGENTS.md §3.2 / C5 — the repository toolchain is
// stable-pinned, cargo-fuzz needs a nightly for `-Zsanitizer`); the
// exact commands live in the README. Until that lands, this target
// is compile-ready but not built by any gate. The harness logic
// (std-only, panic-free by construction, memory bounded by the
// decoder's own MAX_PIXELS/per-plane ceilings — see that module's
// docs) is meanwhile proven on stable by the in-crate deterministic
// smoke tests, the committed-corpus survival run in
// `crates/codecs/tests/fuzz_survival.rs` (all 22 fixture files, all
// 120 extracted frames, systematic truncations and single-byte flips
// of a real frame, adversarial near-cap geometries), and the seeded
// survival loop in `crates/codecs/tests/fuzz_loop_stable.rs` (card
// W1-12). Seed corpus when the campaign lights up: the 60 committed
// JPEG frames from both class-A parents (CC0,
// tests/fixtures/CORPUS.md) plus truncations of frame 0 at every
// 128-byte boundary and the class-B derivative files.
//
// Build (once nightly is wired):  cargo +nightly fuzz run fuzz_codec_mjpeg

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    codecs::video::mjpeg::fuzz::fuzz_codec_mjpeg(data);
});
