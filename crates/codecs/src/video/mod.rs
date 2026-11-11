//! Video decoders (doc 01 §2.5: one module per codec).
//!
//! v0 carried a single codec: [`mjpeg`] — the Motion-JPEG /
//! baseline JPEG decoder behind [`codec_api::Decoder`] (card 2.B4,
//! the first vertical's video path, doc 03 §7). Wave 4 added the
//! [`h264`] bitstream parser; wave 7 adds [`av1`] — the rav1d-backed
//! software decoder, the flagship of doc 04 §4.5's honest v1 video
//! story. Wave 6's M-cards add the admitted engines behind the
//! contract: [`dnx`] (filmcraft-dnx, DNxHD/DNxHR VC-3). Per-codec
//! Cargo feature history: the EXTERNAL-engine modules ([`av1`],
//! [`dnx`], [`vp9`]) ride their `codec-*` Cargo features since card
//! 7.K1 (doc 09 §2.4's tree-shake lane; `default` enables the full
//! set, so the unconditional pre-7.K1 graph is unchanged); the
//! in-house modules ([`h264`], [`hevc`], [`mjpeg`], [`mpeg12`]) stay
//! unconditional until the doc 01 §2.5(a) registration card owns
//! their gates.

/// The rav1d-backed AV1 decoder (card 5.E1). `codec-av1` — rav1d
/// does not compile for wasm32-unknown-unknown (39 errors, measured
/// 2026-11-10; docs/web/size-report.md), so wasm subsets never carry
/// this module.
#[cfg(feature = "codec-av1")]
pub mod av1;
/// The filmcraft-dnx DNxHD/DNxHR decoder (card 6.M-dnx).
#[cfg(feature = "codec-dnx")]
pub mod dnx;
/// In-house bitstream parser (wave 4): unconditional until the
/// registration card's per-codec gates (doc 01 §2.5(a)).
pub mod h264;
/// In-house bitstream parser: unconditional until the registration
/// card's per-codec gates (doc 01 §2.5(a)).
pub mod hevc;
/// In-house (card 2.B4): unconditional until the registration card's
/// per-codec gates (doc 01 §2.5(a)).
pub mod mjpeg;
/// In-house sequence-layer parser + typed refusal (card 5.H1):
/// unconditional until the registration card's per-codec gates
/// (doc 01 §2.5(a)).
pub mod mpeg12;
/// The rusty_vp9 VP9 decoder (card 6.M-vp9).
#[cfg(feature = "codec-vp9")]
pub mod vp9;
