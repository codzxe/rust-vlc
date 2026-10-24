//! Audio decoders (doc 01 §2.5: one module per codec).
//!
//! [`pcm`] is the unpacked-PCM decoder behind [`codec_api::Decoder`]
//! (card 2.B5, the first vertical's audio path, doc 03 §7). [`flac`]
//! is the FLAC decoder through the purity-pinned `claxon` engine
//! (card 5.A1, the Phase-5 lossless core, doc 04 §3.1 row "FLAC").
//! Per-codec Cargo features arrive with the registration card
//! (doc 01 §2.5(a)); today both modules ship unconditionally.

pub mod flac;
pub mod pcm;
