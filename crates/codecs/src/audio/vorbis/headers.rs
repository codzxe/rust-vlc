//! The Vorbis header chain (card 5.B2): identification-header parse,
//! the three-header split, and the codec-api `extra_data` seam
//! definition.
//!
//! # The seam ([PROPOSAL], the FLAC STREAMINFO precedent)
//!
//! [`DecoderConfig::extra_data`](codec_api::DecoderConfig) carries the
//! stream's three Vorbis header packets **concatenated in order** —
//! identification ‖ comment ‖ setup — exactly as the Ogg demuxer's
//! [`header_packets`](demux::ogg::OggDemuxer::header_packets) hands
//! them over (BOS run). The chain is self-delimiting: the
//! identification header is exactly [`IDENT_LEN`] = 30 bytes, the
//! comment header is length-prefixed to its last byte, and the setup
//! header is the remainder. [`VorbisHeaders::from_extra_data`]
//! enforces the split; [`VorbisHeaders::from_packets`] accepts the
//! packet list directly (the ogg seam's native shape, 1 = ident,
//! 2 = comment, 3 = setup — extra header packets beyond three are a
//! typed rejection, the spec has exactly three).
//!
//! The engine is fed identification ‖ setup (its non-laced
//! `read_extradata` branch — symphonia-common's Xiph lacing is a
//! Matroska concern we don't need); the comment header stays ours
//! (metadata exposure, [`super::comments`]).
//!
//! Nothing here trusts the engine for header *validation*: magic,
//! version, geometry and framing bits are checked before the engine
//! ever sees the bytes, so a damaged chain dies as OUR typed error
//! with a stable code.

use super::comments::VorbisComments;
use super::error::{corrupt, unsupported};
use media_types::EngineError;

/// The identification header's exact byte length (spec §4.2.2 field
/// order: type + "vorbis" + version + channels + rate + 3×bitrate +
/// blocksize + framing).
pub const IDENT_LEN: usize = 30;

/// The common header packet signature (spec §4.2.1).
const MAGIC: &[u8] = b"vorbis";
/// Packet-type byte of the identification header.
const TYPE_IDENT: u8 = 0x01;
/// Packet-type byte of the setup header.
const TYPE_SETUP: u8 = 0x05;
/// The only Vorbis I version (spec §4.2.2: "currently 0").
const VERSION: u32 = 0;
/// Minimum block size (64 samples) as a power-of-two exponent.
const BS_MIN: u8 = 6;
/// Maximum block size (8192 samples) as a power-of-two exponent.
const BS_MAX: u8 = 13;

/// The parsed identification header (the stream's geometry).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Identification {
    /// Audio channel count (1–255; symphonia maps up to 8 positions —
    /// above that configure is a typed refusal, not a guess).
    pub channels: u8,
    /// Sample rate in Hz (validated > 0).
    pub sample_rate: u32,
    /// Short-window block size, samples (2^`bs0_exp`).
    pub blocksize_short: u32,
    /// Long-window block size, samples (2^`bs1_exp`).
    pub blocksize_long: u32,
    /// Optional maximum bitrate hint (bits/s), `None` when 0.
    pub bitrate_max: Option<i32>,
    /// Nominal bitrate hint (bits/s), `None` when 0.
    pub bitrate_nominal: Option<i32>,
    /// Optional minimum bitrate hint (bits/s), `None` when 0.
    pub bitrate_min: Option<i32>,
}

/// Does `data` start with a Vorbis identification header? (The probe
/// surface: magic + type only — the cheap sniff; full validation is
/// [`parse_identification`].)
#[must_use]
pub fn is_vorbis_ident(data: &[u8]) -> bool {
    data.len() >= 7 && data[0] == TYPE_IDENT && &data[1..7] == MAGIC
}

/// Parse and validate one identification-header packet (spec §4.2.2).
///
/// # Errors
/// Typed [`VorbisError`]s: `vorbis.ident.magic` (wrong type/signature),
/// `vorbis.ident.truncated` (shorter than 30 bytes),
/// `vorbis.ident.version` (not Vorbis I),
/// `vorbis.ident.channels` (zero),
/// `vorbis.ident.rate` (zero),
/// `vorbis.ident.blocksize` (out of 2^6..2^13 or inverted),
/// `vorbis.ident.framing` (the framing bit is clear).
pub fn parse_identification(data: &[u8]) -> Result<Identification, EngineError> {
    if data.len() < IDENT_LEN {
        return Err(corrupt(
            "vorbis.ident.truncated",
            media_types::error::CorruptDetail::Truncated {
                expected: IDENT_LEN as u64,
                found: data.len() as u64,
            },
            "identification header shorter than 30 bytes",
        )
        .into_engine_error());
    }
    if data[0] != TYPE_IDENT || &data[1..7] != MAGIC {
        return Err(unsupported(
            "vorbis.ident.magic",
            "packet is not a Vorbis identification header (\\x01vorbis)",
        )
        .into_engine_error());
    }
    let version = u32::from_le_bytes([data[7], data[8], data[9], data[10]]);
    if version != VERSION {
        return Err(unsupported(
            "vorbis.ident.version",
            format!("Vorbis version {version} is not the supported I ({VERSION})"),
        )
        .into_engine_error());
    }
    let channels = data[11];
    if channels == 0 {
        return Err(corrupt(
            "vorbis.ident.channels",
            media_types::error::CorruptDetail::InvalidStructure { offset: Some(11) },
            "identification header declares zero channels",
        )
        .into_engine_error());
    }
    let sample_rate = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
    if sample_rate == 0 {
        return Err(corrupt(
            "vorbis.ident.rate",
            media_types::error::CorruptDetail::InvalidStructure { offset: Some(12) },
            "identification header declares a zero sample rate",
        )
        .into_engine_error());
    }
    let bitrate_max = i32::from_le_bytes([data[16], data[17], data[18], data[19]]);
    let bitrate_nominal = i32::from_le_bytes([data[20], data[21], data[22], data[23]]);
    let bitrate_min = i32::from_le_bytes([data[24], data[25], data[26], data[27]]);
    let blocksize = data[28];
    let bs0 = blocksize & 0x0f;
    let bs1 = blocksize >> 4;
    if !(BS_MIN..=BS_MAX).contains(&bs0) || !(BS_MIN..=BS_MAX).contains(&bs1) || bs0 > bs1 {
        return Err(corrupt(
            "vorbis.ident.blocksize",
            media_types::error::CorruptDetail::InvalidStructure { offset: Some(28) },
            format!("blocksize exponents {bs0}/{bs1} outside 2^{BS_MIN}..2^{BS_MAX} or inverted"),
        )
        .into_engine_error());
    }
    if data[29] & 0x01 == 0 {
        return Err(corrupt(
            "vorbis.ident.framing",
            media_types::error::CorruptDetail::InvalidStructure { offset: Some(29) },
            "identification header framing bit is not set",
        )
        .into_engine_error());
    }
    Ok(Identification {
        channels,
        sample_rate,
        blocksize_short: 1 << bs0,
        blocksize_long: 1 << bs1,
        bitrate_max: (bitrate_max != 0).then_some(bitrate_max),
        bitrate_nominal: (bitrate_nominal != 0).then_some(bitrate_nominal),
        bitrate_min: (bitrate_min != 0).then_some(bitrate_min),
    })
}

/// The validated three-header chain.
#[derive(Debug, Clone)]
pub struct VorbisHeaders {
    /// The identification header's parsed geometry.
    pub identification: Identification,
    /// The comment header's parsed metadata.
    pub comments: VorbisComments,
    /// The verbatim identification packet (engine extradata part 1).
    ident_packet: Vec<u8>,
    /// The verbatim setup packet (engine extradata part 2).
    setup_packet: Vec<u8>,
}

impl VorbisHeaders {
    /// Accept the demuxer's header-packet list (the ogg seam's native
    /// shape: `[ident, comment, setup]` — see [`OggDemuxer::
    /// header_packets`](demux::ogg::OggDemuxer::header_packets)).
    ///
    /// # Errors
    /// `vorbis.headers.count` for any packet count other than 3, then
    /// the per-header errors of the chain split.
    pub fn from_packets<'a, I>(packets: I) -> Result<VorbisHeaders, EngineError>
    where
        I: IntoIterator<Item = &'a [u8]>,
    {
        let collected: Vec<&[u8]> = packets.into_iter().collect();
        if collected.len() != 3 {
            return Err(unsupported(
                "vorbis.headers.count",
                format!(
                    "a Vorbis stream carries exactly 3 header packets (identification, \
                     comment, setup); got {}",
                    collected.len()
                ),
            )
            .into_engine_error());
        }
        let mut joined =
            Vec::with_capacity(collected[0].len() + collected[1].len() + collected[2].len());
        for packet in &collected {
            joined.extend_from_slice(packet);
        }
        VorbisHeaders::from_extra_data(&joined)
    }

    /// Split + validate the concatenated header chain (the
    /// `extra_data` seam; see the module docs).
    ///
    /// # Errors
    /// The identification header's errors first, then the comment
    /// header's, then `vorbis.setup.magic` when the remainder is not a
    /// setup header, `vorbis.setup.truncated` when it is absent.
    pub fn from_extra_data(data: &[u8]) -> Result<VorbisHeaders, EngineError> {
        let identification = parse_identification(data)?;
        let comment_len = super::comments::header_len(&data[IDENT_LEN..])?;
        let comment_end = IDENT_LEN + comment_len;
        let comments = super::comments::parse(&data[IDENT_LEN..comment_end])?;
        let setup = data.get(comment_end..).unwrap_or(&[]);
        if setup.is_empty() {
            return Err(corrupt(
                "vorbis.setup.truncated",
                media_types::error::CorruptDetail::Truncated {
                    expected: 7,
                    found: 0,
                },
                "the setup header is absent from the header chain",
            )
            .into_engine_error());
        }
        if setup[0] != TYPE_SETUP || &setup[1..7] != MAGIC {
            return Err(corrupt(
                "vorbis.setup.magic",
                media_types::error::CorruptDetail::InvalidStructure {
                    offset: Some(comment_end as u64),
                },
                "the remainder after the comment header is not a Vorbis setup header",
            )
            .into_engine_error());
        }
        Ok(VorbisHeaders {
            identification,
            comments,
            ident_packet: data[..IDENT_LEN].to_vec(),
            setup_packet: setup.to_vec(),
        })
    }

    /// The engine's extradata: identification ‖ setup (the engine's
    /// non-laced `read_extradata` branch; the comment header is not
    /// engine food — it is ours).
    #[must_use]
    pub fn engine_extra_data(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.ident_packet.len() + self.setup_packet.len());
        out.extend_from_slice(&self.ident_packet);
        out.extend_from_slice(&self.setup_packet);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal legal identification header (mono, 48 kHz, 256/2048
    /// blocks) — the hand-built fixture every parse test starts from.
    fn ident(channels: u8, rate: u32, bs0: u8, bs1: u8) -> Vec<u8> {
        let mut out = Vec::with_capacity(IDENT_LEN);
        out.push(TYPE_IDENT);
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.push(channels);
        out.extend_from_slice(&rate.to_le_bytes());
        out.extend_from_slice(&0i32.to_le_bytes()); // bitrate max
        out.extend_from_slice(&112_000i32.to_le_bytes()); // nominal
        out.extend_from_slice(&0i32.to_le_bytes()); // bitrate min
        out.push(bs0 | (bs1 << 4));
        out.push(0x01); // framing
        out
    }

    #[test]
    fn identification_roundtrips_every_field() {
        let bytes = ident(2, 44_100, 8, 11);
        let parsed = match parse_identification(&bytes) {
            Ok(parsed) => parsed,
            Err(err) => unreachable!("{err:?}"),
        };
        assert_eq!(parsed.channels, 2);
        assert_eq!(parsed.sample_rate, 44_100);
        assert_eq!(parsed.blocksize_short, 256);
        assert_eq!(parsed.blocksize_long, 2048);
        assert_eq!(parsed.bitrate_nominal, Some(112_000));
        assert_eq!(parsed.bitrate_max, None);
        assert_eq!(parsed.bitrate_min, None);
        assert!(is_vorbis_ident(&bytes));
    }

    #[test]
    fn bad_magic_is_the_typed_unsupported_refusal() {
        let mut bytes = ident(2, 44_100, 8, 11);
        bytes[2] = b'V'; // \x01Vorbis
        let err = match parse_identification(&bytes) {
            Err(err) => err,
            Ok(parsed) => unreachable!("must refuse: {parsed:?}"),
        };
        assert_eq!(err.context.code, "vorbis.ident.magic");
        assert!(!is_vorbis_ident(&bytes));
    }

    #[test]
    fn truncation_version_geometry_and_framing_are_all_typed() {
        // Truncation.
        let short = ident(1, 8_000, 8, 8);
        let err = match parse_identification(&short[..29]) {
            Err(err) => err,
            Ok(parsed) => unreachable!("must refuse: {parsed:?}"),
        };
        assert_eq!(err.context.code, "vorbis.ident.truncated");
        // Version.
        let mut v1 = ident(1, 8_000, 8, 8);
        v1[7] = 1;
        let err = match parse_identification(&v1) {
            Err(err) => err,
            Ok(parsed) => unreachable!("must refuse: {parsed:?}"),
        };
        assert_eq!(err.context.code, "vorbis.ident.version");
        // Zero rate.
        let err = match parse_identification(&ident(1, 0, 8, 8)) {
            Err(err) => err,
            Ok(parsed) => unreachable!("must refuse: {parsed:?}"),
        };
        assert_eq!(err.context.code, "vorbis.ident.rate");
        // Inverted block sizes.
        let err = match parse_identification(&ident(1, 8_000, 11, 8)) {
            Err(err) => err,
            Ok(parsed) => unreachable!("must refuse: {parsed:?}"),
        };
        assert_eq!(err.context.code, "vorbis.ident.blocksize");
        // Clear framing bit.
        let mut unframed = ident(1, 8_000, 8, 8);
        unframed[29] = 0;
        let err = match parse_identification(&unframed) {
            Err(err) => err,
            Ok(parsed) => unreachable!("must refuse: {parsed:?}"),
        };
        assert_eq!(err.context.code, "vorbis.ident.framing");
    }

    /// A minimal legal comment header (empty vendor, no comments).
    fn comment() -> Vec<u8> {
        let mut out = Vec::new();
        out.push(0x03);
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&0u32.to_le_bytes()); // vendor length
        out.extend_from_slice(&0u32.to_le_bytes()); // comment count
        out.push(0x01); // framing
        out
    }

    #[test]
    fn header_chain_split_rejects_short_and_foreign_remainders() {
        // Identification only: the comment header is absent.
        let chain = ident(1, 8_000, 8, 8);
        let err = match VorbisHeaders::from_extra_data(&chain) {
            Err(err) => err,
            Ok(headers) => unreachable!("must refuse: {headers:?}"),
        };
        assert_eq!(err.context.code, "vorbis.comment.magic");
        // A complete ident+comment chain with no setup header.
        let mut no_setup = chain.clone();
        no_setup.extend_from_slice(&comment());
        let err = match VorbisHeaders::from_extra_data(&no_setup) {
            Err(err) => err,
            Ok(headers) => unreachable!("must refuse: {headers:?}"),
        };
        assert_eq!(err.context.code, "vorbis.setup.truncated");
        // A "setup" remainder with the wrong type byte.
        let mut with_setup = no_setup.clone();
        with_setup.push(TYPE_SETUP + 2);
        with_setup.extend_from_slice(MAGIC);
        let err = match VorbisHeaders::from_extra_data(&with_setup) {
            Err(err) => err,
            Ok(headers) => unreachable!("must refuse: {headers:?}"),
        };
        assert_eq!(err.context.code, "vorbis.setup.magic");
        // Packet-count gate.
        let err = match VorbisHeaders::from_packets([&chain[..], &chain[..]]) {
            Err(err) => err,
            Ok(headers) => unreachable!("must refuse: {headers:?}"),
        };
        assert_eq!(err.context.code, "vorbis.headers.count");
    }
}
