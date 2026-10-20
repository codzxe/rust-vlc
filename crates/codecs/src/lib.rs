//! Lumen builtin software codecs (doc 01 §2.5; cards 2.B4 + 2.B5):
//! pure-Rust decoder implementations behind the [`codec_api`]
//! contract.
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
//! # The 2.B5 dependency decision (recorded in
//! [`audio::pcm`]'s docs, evidence in worklog 2.B5)
//!
//! The PCM card offered `symphonia-codec-pcm` or a self-build; the
//! decision is likewise **in-house** on the same D-018 logic — see
//! [`audio::pcm`] for the full rationale (ground truth in-repo,
//! permutation-only byte engine, zero-copy native path).
//!
//! # Module map
//!
//! - [`video`] — video decoders; [`video::mjpeg`] is the Motion-JPEG /
//!   baseline-JPEG decoder (card 2.B4, doc 04 §4 row "Motion-JPEG").
//! - [`audio`] — audio decoders; [`audio::pcm`] is the unpacked-PCM
//!   decoder (card 2.B5, doc 04 §3.1 row "PCM") — its module docs
//!   record the 2.B5 dependency decision (in-house, D-018 logic).
//!
//! # Conventions (crate-wide)
//!
//! - No `unsafe`, no panics on any input: every decode path is
//!   bounds-checked and overflow-checked (the fuzz targets
//!   `fuzz_codec_mjpeg` / `fuzz_codec_pcm`, doc 04 §8.5, hammer
//!   exactly this guarantee).
//! - Corruption is either concealed-and-reported
//!   ([`DecodeOutcome::Recovered`](codec_api::DecodeOutcome::Recovered))
//!   or a typed [`EngineError`] — never a silent partial frame, never a
//!   crash (doc 03 §4.5(d)).

pub mod audio;
pub mod video;
