// fuzz_demux_avi — the AVI 1.0 container fuzz target (card 2.B2 `+FB`).
//
// Registry: doc 04 §8.5 (row "AVI", fuzz target `fuzz_demux_avi`) —
// "Directory fuzz/, cargo-fuzz, one target per parser named exactly as
// in the tables above".
//
// WIRING STATUS (updated 2026-10-22, card W1-12): the cargo-fuzz
// scaffolding now EXISTS — `fuzz/Cargo.toml` (package `lumen-fuzz`,
// `libfuzzer-sys` 0.4.13, this target declared) + `fuzz/README.md`.
// What is still owed is the *nightly wiring* (a Lead dependency
// decision per AGENTS.md §3.2 / C5 — the repository toolchain is
// stable-pinned, cargo-fuzz needs a nightly for `-Zsanitizer`); the
// exact commands live in the README. Until that lands, this target
// is compile-ready but not built by any gate. The harness logic
// (std-only, panic-free by construction, bounded budgets — see that
// module's docs) is meanwhile proven on stable by the in-crate
// deterministic smoke tests, the committed-corpus run in
// `crates/demux/tests/fixtures_class_b.rs`, and the seeded survival
// loop in `crates/demux/tests/fuzz_loop_stable.rs` (card W1-12).
// Seed corpus when the campaign lights up: the committed class-B AVI
// derivatives (CC0, tests/fixtures/CORPUS.md) plus truncations of
// the parent at every 256-byte boundary.
//
// Build (once nightly is wired):  cargo +nightly fuzz run fuzz_demux_avi

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    demux::avi::fuzz::fuzz_demux_avi(data);
});
