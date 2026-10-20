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
//! The card offered `jpeg-decoder` or `zune-jpeg`; the decision is
//! **in-house**, following D-018's zero-new-dependency logic for the
//! first vertical: the repo already owns the encoder-side ground
//! truth (the 2.A3 fixture writer's fixed-point DCT basis, zigzag,
//! canonical Huffman tables and byte stuffing are the exact
//! primitives this decoder inverts), and an in-house decoder gives
//! full control of the output format — codec-native planar
//! [`PixelFormat::I420`](media_types::PixelFormat::I420) for 4:2:0
//! streams (doc 01 §3.3: "I420 — the codec-native default"), RGBA
//! for the samplings that have no planar vocabulary yet. Zero new
//! deps to vet; the external crates remain future fallback options,
//! each owed a full Lead-ratified audit (license, `unsafe` surface,
//! transitive graph) before any adoption `[VERIFY: not audited in
//! this card — the in-house path made it moot]`.
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
