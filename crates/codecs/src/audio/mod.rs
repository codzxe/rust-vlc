//! Audio decoders (doc 01 §2.5: one module per codec).
//!
//! [`pcm`] is the unpacked-PCM decoder behind [`codec_api::Decoder`]
//! (card 2.B5, the first vertical's audio path, doc 03 §7). [`flac`]
//! is the FLAC decoder through the purity-pinned `claxon` engine
//! (card 5.A1, the Phase-5 lossless core, doc 04 §3.1 row "FLAC").
//! [`mp3`] is the MP3/MP2 decoder through the purity-pinned
//! `symphonia-bundle-mp3` engine with our own ID3v2/Xing/VBRI/
//! gapless/seek surface (card 5.A3, doc 04 §3.1 rows "MP3"/"MP2").
//! [`vorbis`] is the Vorbis decoder through the purity-pinned
//! `symphonia-codec-vorbis` engine with our own header-chain,
//! Vorbis-comment and granule-arithmetic surface (card 5.B2, doc 04
//! §3.1 row "Vorbis"). Per-codec Cargo features arrive with the
//! registration card (doc 01 §2.5(a)); today the modules ship
//! unconditionally.

pub mod flac;
pub mod mp3;
pub mod pcm;
pub mod vorbis;
