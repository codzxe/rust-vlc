//! Audio decoders (doc 01 §2.5: one module per codec).
//!
//! v0's first vertical carried [`pcm`] — the unpacked-PCM decoder
//! behind [`codec_api::Decoder`] (card 2.B5, doc 03 §7). [`adpcm`]
//! joins with card 5.J2b's breadth flavors (Yamaha, OKI/Dialogic,
//! SWF, Apple IMA4, Westwood IMA — in-house, bit-exact against the
//! ffmpeg 7.1.5 reference). Per-codec Cargo features arrive with the
//! next codec family (doc 01 §2.5(a)).

pub mod adpcm;
pub mod pcm;
