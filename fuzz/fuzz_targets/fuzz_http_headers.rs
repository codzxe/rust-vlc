// fuzz_http_headers — the HTTP response-head target (issue #14 M2,
// card 6.F3): hostile bytes from the wire into the transport's
// response-head grammar (status line + `Name: value` lines).
//
// Thin libFuzzer shim, the same convention as the other rows: the
// harness body is the in-crate
// `streaming::mockhttp::fuzz_response_head` — the pure parsing half
// of the socket path's `read_response_head`, split out additively
// for exactly this target. Bounds: input capped at the transport's
// own 64 KiB head budget (over-limit answers the typed
// `NetworkError::ProtocolViolation`, never an unbounded parse);
// hostile bytes decode lossy (U+FFFD), they never panic the grammar.
//
// The request-side companion passes (`Range:` header-value
// formatting under hostile numbers) live in the `#[path]`-included
// `crates/streaming/tests/netfuzz/fuzz.rs` twin below — the same
// module the stable sweep runs, so the campaign and the sweep share
// one body. (Naming note: the target is `fuzz_http_headers`, plural,
// per issue #14's target list; the mockhttp doc comment that banked
// `fuzz_response_head` spells the file singular — same harness.)
//
// Wiring status (honest, same as the other 33): building the
// campaign needs the Lead's nightly wiring (fuzz/README.md); until
// then this package compiles on stable (`cargo check` in fuzz/) and
// the harness logic is proven by the deterministic hostile-input
// sweep `crates/streaming/tests/netfuzz_sweep.rs`.
//
// Seeds: fuzz/corpus/fuzz_http_headers/ — hand-authored response
// heads (CC0, ours) plus one cross-format playlist seed (format
// confusion is fair fuzz material).
//
// Build (once nightly is wired):  cargo +nightly fuzz run fuzz_http_headers

#![no_main]
// The registry gate's shim check reads the bare in-crate harness
// call below (`streaming::mockhttp::fuzz_response_head(data);`), so
// it stays a bare statement — the discarded `Result` is the point
// (survival, not the verdict), and the lint would rather have a
// `let _ =` the gate's line grammar cannot see through.
#![allow(unused_must_use)]

use libfuzzer_sys::fuzz_target;

#[path = "../../crates/streaming/tests/netfuzz/fuzz.rs"]
mod netfuzz;

fuzz_target!(|data: &[u8]| {
    streaming::mockhttp::fuzz_response_head(data);
    netfuzz::fuzz_http_headers(data);
});
