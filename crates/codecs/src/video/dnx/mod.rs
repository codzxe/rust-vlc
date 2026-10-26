//! The DNxHD / DNxHR decoder (card 6.M-dnx; doc 01 §2.5, doc 04 §4
//! row "DNxHD"): SMPTE ST 2019-1 (VC-3) through the admitted
//! `filmcraft-dnx` engine, behind our own CBR truncation gate,
//! lossless output-format mapping and typed error surface.
//!
//! # Module map (the 6.M-opus shape)
//!
//! - [`decoder`]: the [`codec_api::Decoder`] implementation —
//!   one-packet-one-frame decode, the pooled packed-output
//!   rotation, the full-depth [`decoder::DnxPicture`] seam, and the
//!   CBR concealment gate answering the admission record's §H
//!   truncation flag.
//! - [`error`]: the typed stage-tagged taxonomy mapping onto
//!   [`media_types::EngineError`] (dotted `codecs.dnx.*` codes).
//!
//! # The engine (the admission record's pin)
//!
//! `filmcraft-dnx = { version = "=0.9.0", default-features = false
//! }` — the `threads` feature (rayon) is OFF, non-negotiably: the
//! admission shape keeps the lockfile rayon-free and the measured
//! decode path single-threaded. The exact pin re-triggers
//! `docs/dependencies/records/filmcraft-dnx-0.9.0.md` on any bump
//! or feature change (doc 11 §3.7).
//!
//! # The ratified tolerance (6.VTA-DNX-RATIFY)
//!
//! The engine's IDCT is the exact orthonormal f32 evaluation of
//! SMPTE eq. 8.3 where ffmpeg uses an integer approximation, so
//! bit-exactness vs ffmpeg is unreachable **in principle**: the
//! differential class is ±2 LSB (bulk ±1, balanced signs), Lead-
//! ratified for VC-3. The differential suite
//! (`tests/dnx_differential.rs`) pins exactly that gate on the
//! record's five lanes.
//!
//! # Scope (decoder wiring only)
//!
//! This card is the codecs-crate decode surface. Container /
//! elementary-stream wiring (AVdn/AVdh sample entries, MKF, the
//! CBR ES splitter the raw-`.dnxhd` lanes assume) is the Lead's
//! reconciliation; the packet contract here is "one packet = one
//! complete VC-3 frame".

pub mod decoder;
mod error;

pub use decoder::{DnxChroma, DnxColorVolume, DnxDecoder, DnxPicture};
