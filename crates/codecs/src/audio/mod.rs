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
//! §3.1 row "Vorbis"). [`aac`] is the AAC-LC decoder through the
//! purity-pinned `symphonia-codec-aac` engine with our own ADTS
//! header parse and profile/HE gates (card 5.C2a — the decoder half
//! of 5.C2; HE-AAC honestly partial, see its module docs). [`alac`]
//! is the ALAC decoder through the purity-pinned
//! `symphonia-codec-alac` engine with our own magic-cookie parse and
//! error taxonomy (card 5.J3a — the ALAC half of 5.J3, lossless).
//! Per-codec Cargo features arrive with the registration card
//! (doc 01 §2.5(a)); today the modules ship unconditionally.

pub mod aac;
/// [`adpcm`] is the in-house ADPCM family (cards 5.J2a + 5.J2b, doc
/// 04 §3.1 rows "ADPCM MS"/"ADPCM IMA" and the ADPCM breadth rows):
/// the WAV-carried MS/IMA-WAV pair plus Yamaha, OKI/Dialogic, SWF,
/// Apple IMA4 and Westwood IMA — one `AdpcmDecoder`, seven flavors,
/// bit-exact against the ffmpeg 7.1.5 reference. Per-codec Cargo
/// features arrive with the registration card (doc 01 §2.5(a)).
pub mod adpcm;
pub mod alac;
pub mod flac;
/// [`mp2`] is the dedicated MP2 (MPEG audio Layer II) decoder: the
/// family engine (`symphonia-bundle-mp3`, no new dependency) behind
/// our own Layer II gate, CBR geometry/seek arithmetic and typed
/// error surface (card 5.J3 — the J3b half; doc 04 §3.1 row "MP2").
pub mod mp2;
pub mod mp3;
/// [`opus`] is the Opus decoder through the admitted `ruopus` engine
/// (card 6.M-opus, doc 04 §3.1 row "Opus") with our own OpusHead
/// validation (the pre-constructor panic gate), pre-skip/end-trim
/// granule arithmetic and RFC 7845 → house channel-order mapping
/// (the admission record's two binding conditions).
pub mod opus;
pub mod pcm;
pub mod vorbis;
