//! The Opus decoder (card 6.M-opus; doc 01 §2.5, doc 04 §3.1 row
//! "Opus"): the `ruopus` engine behind our own OpusHead validation,
//! pre-skip/end-trim granule arithmetic, RFC 7845 → house
//! channel-order mapping and typed error surface.
//!
//! # Module map (the Vorbis 5.B2 shape)
//!
//! - [`head`]: the OpusHead grammar over the Ogg host's
//!   `OpusHead ‖ OpusTags` extra_data blob — the **pre-constructor
//!   validation** that closes the ruopus constructors' documented
//!   panic windows (channels/rate) before any engine exists (the
//!   admission record's binding condition 2, first half).
//! - [`order`]: the RFC 7845 Vorbis → house (WAV) channel-order
//!   permutation applied at the output seam (binding condition 2,
//!   second half).
//! - [`decoder`]: the [`codec_api::Decoder`] implementation —
//!   pre-skip head-trim, granulepos end-trim, packet-scoped damage
//!   isolation, the pooled output rotation.
//! - [`error`]: the typed stage-tagged taxonomy mapping onto
//!   [`media_types::EngineError`].
//!
//! # The engine (the admission record's condition 1)
//!
//! `ruopus = { version = "=0.1.2", default-features = false,
//! features = ["std"] }` — the zero-dependency shape. Default
//! features (spectrograms → ndarray/rustfft/regex-automata graph,
//! python → pyo3) are OFF, non-negotiably; the pin is exact (the
//! record's honest gap 1: a young release line, re-verified on
//! bump).

pub mod decoder;
mod error;
mod head;
mod order;

pub use decoder::OpusDecoder;
