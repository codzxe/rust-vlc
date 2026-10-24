//! Host for the rebrand acceptance suite (card 4.H2).
//!
//! `tests/rebrand/rebrand_acceptance.rs` is the real test file (repo-root
//! tests/ tree, next to its fixture, per doc 07 §8.3); it is compiled
//! here because the runtime plane it proves needs this crate — the typed
//! product model + the `runtime` identity resolvers — and the other
//! natural host (`tools/xtask`) is std-only-zero-dep by C5. Same
//! `#[path]` convention as the 1.E4 skeleton's wiring into
//! tools/xtask/tests/cli.rs.
//!
//! Run: `cargo test -p product-config --test rebrand_host`

#[path = "../../../tests/rebrand/rebrand_acceptance.rs"]
mod rebrand_acceptance;
