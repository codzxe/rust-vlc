//! Lumen builtin software codecs (doc 01 §2.5; card 2.B4): pure-Rust
//! decoder implementations behind the [`codec_api`] contract.
//!
//! Layer L3 (doc 01 §2 registry row: `codec-api`, `plugin-api`,
//! `media-types` — this card's decoder surface needs the first and last;
//! `plugin-api` joins with the registry/`register()` surface of doc 01
//! §2.5(b), a later card). Std-only by C5: **zero external dependencies**.
//!
//! # The 2.B4 dependency decision (recorded here, evidence in worklog 2.B4)
//!
//! The card offered `jpeg-decoder` or `zune-jpeg`; both were P0-verified
//! by tarball inspection (doc 11 §4.1 method):
//!
//! - `jpeg-decoder` 0.3.2 — pure Rust (no native sources, `build = false`,
//!   no `links`, no `extern "C"`); sha256 matches the registry record;
//!   23 scoped `#[allow(unsafe_code)]` blocks exist **only** for
//!   `core::arch` SIMD dispatch and are compiled out by
//!   `features = ["platform_independent"]`, which sets
//!   `forbid(unsafe_code)`; with `default-features = false` it has zero
//!   registry dependencies. A compliant option — but it outputs packed
//!   RGB/RGBA only (no planar YCbCr), and adopting it needs a
//!   Lead-ratified allowlist row + deny.toml change.
//! - `zune-jpeg` 0.5.0 — pure Rust, but 65 `unsafe` blocks across 11 core
//!   files (bitstream, upsampler, decoder) with **no feature that removes
//!   them**, a mandatory `zune-core` transitive dependency, and a third
//!   license class (Zlib) that deny.toml would have to admit. Weakest
//!   policy fit.
//!
//! Decision: **in-house baseline decoder**, following D-018's
//! zero-new-dependency logic for the first vertical and the in-repo 2.A3
//! JPEG *encoder* (fixed-point DCT basis, zigzag, canonical Huffman
//! tables, byte stuffing — the exact inverse primitives), with full
//! control of the output format: codec-native planar
//! [`PixelFormat::I420`](media_types::PixelFormat::I420) for 4:2:0
//! streams (doc 01 §3.3: "I420 — the codec-native default"), RGBA for the
//! samplings that have no planar vocabulary yet.
//!
//! # Module map
//!
//! - [`video`] — video decoders; [`video::mjpeg`] is the Motion-JPEG /
//!   baseline-JPEG decoder (card 2.B4, doc 04 §4 row "Motion-JPEG").
//!
//! # Conventions (crate-wide)
//!
//! - No `unsafe`, no panics on any input: every decode path is
//!   bounds-checked and overflow-checked (the fuzz target
//!   `fuzz_codec_mjpeg`, doc 04 §8.5, hammers exactly this guarantee).
//! - Corruption is either concealed-and-reported
//!   ([`DecodeOutcome::Recovered`](codec_api::DecodeOutcome::Recovered))
//!   or a typed [`EngineError`] — never a silent partial frame, never a
//!   crash (doc 03 §4.5(d)).

pub mod video;
