// fuzz_codec_h264 — the H.264 parser target (parser self-build) (card 5.Q1; doc 04 §8.5 + the fuzz/README registry).
//
// Thin libFuzzer shim: the harness body is the in-crate `codecs::video::h264::fuzz`
// module (panic-free by construction, bounds documented there); one
// call = one full untrusted pass over the bytes.
//
// Wiring status (honest): building the campaign needs the Lead's
// nightly wiring (see fuzz/README.md); until then this package
// compiles on stable (`cargo check` in fuzz/) and the harness logic
// is proven by the in-crate deterministic hostile-input sweeps.
//
// Build (once nightly is wired):  cargo +nightly fuzz run fuzz_codec_h264

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    codecs::video::h264::fuzz::fuzz_codec_h264(data);
});
