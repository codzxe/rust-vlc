//! Audio decoders (doc 01 §2.5: one module per codec).
//!
//! v0 carries a single codec: [`pcm`] — the unpacked-PCM decoder
//! behind [`codec_api::Decoder`] (card 2.B5, the first vertical's
//! audio path, doc 03 §7). Per-codec Cargo features arrive with the
//! next codec family (doc 01 §2.5(a)); today the crate ships one
//! audio module and one video module unconditionally.

pub mod pcm;
