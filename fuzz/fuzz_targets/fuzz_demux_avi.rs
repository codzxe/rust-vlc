// fuzz_demux_avi — the AVI 1.0 container fuzz target (card 2.B2 `+FB`).
//
// Registry: doc 04 §8.5 (row "AVI", fuzz target `fuzz_demux_avi`) —
// "Directory fuzz/, cargo-fuzz, one target per parser named exactly as
// in the tables above".
//
// WIRING STATUS (honest, 2026-10-20): the harness LOGIC lives in
// `demux::avi::fuzz::fuzz_demux_avi` (std-only, panic-free by
// construction, bounded budgets — see that module's docs) and is
// proven today by the in-crate deterministic smoke tests plus the
// committed-corpus run in `crates/demux/tests/fixtures_class_b.rs`.
// Building THIS file needs the cargo-fuzz scaffolding
// (`fuzz/Cargo.toml` + the `libfuzzer-sys` dev-dependency), which is a
// Lead-owned dependency decision under AGENTS.md §3.2 / C5 — until it
// is approved and wired, this target is registered by name and shape
// but not yet compiled in CI. Seed corpus when that happens: the
// committed class-B AVI derivatives (CC0, tests/fixtures/CORPUS.md)
// plus truncations of the parent at every 256-byte boundary.
//
// Build (once wired):  cargo +nightly fuzz run fuzz_demux_avi

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    demux::avi::fuzz::fuzz_demux_avi(data);
});
