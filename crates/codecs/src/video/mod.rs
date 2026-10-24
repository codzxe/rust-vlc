//! Video decoders (doc 01 §2.5: one module per codec).
//!
//! v0 carries a single codec: [`mjpeg`] — the Motion-JPEG / baseline
//! JPEG decoder behind [`codec_api::Decoder`] (card 2.B4, the first
//! vertical's video path, doc 03 §7). Per-codec Cargo features arrive
//! with the second codec (doc 01 §2.5(a)).

pub mod h264;
pub mod mjpeg;
