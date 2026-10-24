//! Video decoders (doc 01 §2.5: one module per codec).
//!
//! v0 carried a single codec: [`mjpeg`] — the Motion-JPEG /
//! baseline JPEG decoder behind [`codec_api::Decoder`] (card 2.B4,
//! the first vertical's video path, doc 03 §7). Wave 4 added the
//! [`h264`] bitstream parser; wave 7 adds [`av1`] — the rav1d-backed
//! software decoder, the flagship of doc 04 §4.5's honest v1 video
//! story. Per-codec Cargo features arrive with the registration
//! card (doc 01 §2.5(a)).

pub mod av1;
pub mod h264;
pub mod hevc;
pub mod mjpeg;
