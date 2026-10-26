//! The OpusHead identification-header parser (card 6.M-opus; the
//! Vorbis `headers.rs` analogue): the RFC 7845 §4.2 grammar over the
//! **verbatim header-chain blob** the Ogg open-path host stages as
//! `DecoderConfig::extra_data` = `OpusHead ‖ OpusTags` (the 6.OPW-ogg
//! seam — no length prefixes, no lacing; the OpusHead packet sits at
//! the front because it is always the stream's first header).
//!
//! # The pre-constructor validation contract (binding condition 2)
//!
//! The ruopus constructors **panic** on channel counts outside {1, 2}
//! (the mono/stereo `OpusDecoder`) and rates outside {8, 12, 16, 24,
//! 48} kHz — the admission record's R6. This parser therefore closes
//! the WHOLE geometry from OpusHead *before* any engine is built:
//!
//! * version must be exactly 1 (RFC 7845 §4: decoders may reject
//!   other versions; we refuse typed, never guess),
//! * family 0 ⇒ channels ∈ {1, 2} (the mono/stereo unmapped case),
//! * family ≥ 1 ⇒ the multistream tail must be present and close
//!   arithmetically: `streams ≥ 1`, `coupled ≤ streams`, total
//!   decoded slots `streams + coupled` = the declared channel count,
//!   and every mapping-table entry indexes a decoded slot
//!   (`< streams + coupled`),
//! * the decode rate is ALWAYS 48 kHz (RFC 7845 §4.2: the
//!   `input sample rate` field is the ORIGINAL encoder input rate —
//!   advisory only; Ogg granules and our decode both count 48 kHz
//!   samples), so the rate side of the panic contract is a constant
//!   on our side of the seam.
//!
//! Only then does [`super::decoder`] construct the engine — the
//! constructor's panic sites are unreachable through this wrapper.

use super::error::{unsupported, OpusError};

/// The exact decode rate the wrapper uses (RFC 7845 §4.2).
pub(crate) const DECODE_RATE: u32 = 48_000;

/// The minimum OpusHead length (the family-0 head, magic included).
const HEAD_MIN: usize = 19;

/// A parsed, fully-validated OpusHead (the configure-time facts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpusHead {
    /// Channel count (1–255; family 0 pins it to 1–2 at parse time).
    pub channels: u8,
    /// Pre-skip: samples to discard from the decode head (RFC 7845
    /// §4.2 — the encoder's look-ahead; granules count it).
    pub pre_skip: u16,
    /// The ORIGINAL encoder input rate (advisory; decode is 48 kHz).
    pub input_rate: u32,
    /// Output gain, Q7.8 dB-scaled linear gain (RFC 7845 §4.2): the
    /// decoder applies `sample × gain / 256`. Every corpus lane
    /// measures 0 (unity) — applied anyway, honestly.
    pub output_gain: i16,
    /// The channel-mapping family (0 = mono/stereo, 1 = Vorbis
    /// order surround, 255 = discrete).
    pub mapping_family: u8,
    /// Multistream geometry (family ≥ 1): stream count.
    pub streams: u8,
    /// Multistream geometry (family ≥ 1): coupled (stereo) stream
    /// count.
    pub coupled: u8,
    /// The per-output-channel mapping into decoded slots (family ≥
    /// 1; empty for family 0).
    pub mapping: Vec<u8>,
}

/// Parse the OpusHead from the front of the joined header-chain blob
/// (`OpusHead ‖ OpusTags`, verbatim — the Ogg host's staging). The
/// OpusTags tail is NOT validated here (it is comment metadata, not
/// decode geometry); the head grammar alone gates the engine.
pub(crate) fn parse_head(blob: &[u8]) -> Result<OpusHead, OpusError> {
    if blob.len() < HEAD_MIN {
        return Err(unsupported(
            "opus.head.truncated",
            format!(
                "header chain is {} bytes; the OpusHead alone is {HEAD_MIN}",
                blob.len()
            ),
        ));
    }
    if &blob[..8] != b"OpusHead" {
        return Err(unsupported(
            "opus.head.magic",
            "the extra_data blob does not start with the OpusHead magic",
        ));
    }
    let version = blob[8];
    if version != 1 {
        return Err(unsupported(
            "opus.head.version",
            format!("OpusHead version {version}; only version 1 is defined (RFC 7845 §4)"),
        ));
    }
    let channels = blob[9];
    if channels == 0 {
        return Err(unsupported(
            "opus.head.channels.zero",
            "OpusHead declares zero channels",
        ));
    }
    let pre_skip = u16::from_le_bytes([blob[10], blob[11]]);
    let input_rate = u32::from_le_bytes([blob[12], blob[13], blob[14], blob[15]]);
    let output_gain = i16::from_le_bytes([blob[16], blob[17]]);
    let mapping_family = blob[18];
    if mapping_family == 0 {
        // The unmapped family: mono or stereo only — this is exactly
        // the `OpusDecoder::new` panic window, closed before the call.
        if channels > 2 {
            return Err(unsupported(
                "opus.head.family0_channels",
                format!(
                    "mapping family 0 carries {channels} channels; family 0 is mono/stereo \
                     (RFC 7845 §3.1.1) and the mono engine accepts at most 2"
                ),
            ));
        }
        return Ok(OpusHead {
            channels,
            pre_skip,
            input_rate,
            output_gain,
            mapping_family,
            streams: 1,
            coupled: u8::from(channels == 2),
            mapping: Vec::new(),
        });
    }
    // Multistream family (1 = Vorbis-ordered surround, 255 = discrete;
    // the arithmetic is family-agnostic on our side — the engine
    // consumes streams/coupled/mapping the same way).
    if blob.len() < 21 + usize::from(channels) {
        return Err(unsupported(
            "opus.head.multistream_truncated",
            format!(
                "family {mapping_family} needs {} bytes of multistream tail (streams + coupled \
                 + {channels}-entry mapping); the chain carries {}",
                2 + usize::from(channels),
                blob.len() - HEAD_MIN
            ),
        ));
    }
    let streams = blob[19];
    let coupled = blob[20];
    let mapping = blob[21..21 + usize::from(channels)].to_vec();
    if streams == 0 {
        return Err(unsupported(
            "opus.head.streams.zero",
            "multistream OpusHead declares zero streams",
        ));
    }
    if coupled > streams {
        return Err(unsupported(
            "opus.head.coupled_over_streams",
            format!(
                "coupled stream count {coupled} exceeds the stream count {streams}"
            ),
        ));
    }
    // Total decoded slots: each uncoupled stream is mono, each coupled
    // stream stereo — must equal the declared channel count, and the
    // mapping must stay inside the slot range (both are constructor
    // panic windows in the multistream engine).
    let slots = u16::from(streams) + u16::from(coupled);
    if slots != u16::from(channels) {
        return Err(unsupported(
            "opus.head.slot_arithmetic",
            format!(
                "streams {streams} + coupled {coupled} = {slots} decoded slots, but OpusHead \
                 declares {channels} channels"
            ),
        ));
    }
    if let Some((index, slot)) = mapping
        .iter()
        .enumerate()
        .find(|(_, s)| **s != 255 && u16::from(**s) >= slots)
    {
        return Err(unsupported(
            "opus.head.mapping_range",
            format!(
                "mapping entry {index} targets decoded slot {slot}; valid slots are 0..{slots} \
                 (255 = silent)"
            ),
        ));
    }
    Ok(OpusHead {
        channels,
        pre_skip,
        input_rate,
        output_gain,
        mapping_family,
        streams,
        coupled,
        mapping,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal family-0 mono head (version 1, 1 ch, pre-skip 312,
    /// 48 kHz input, gain 0).
    fn head_v0(channels: u8, preskip: u16) -> Vec<u8> {
        let mut v = b"OpusHead".to_vec();
        v.push(1);
        v.push(channels);
        v.extend_from_slice(&preskip.to_le_bytes());
        v.extend_from_slice(&48_000u32.to_le_bytes());
        v.extend_from_slice(&0i16.to_le_bytes());
        v.push(0);
        v
    }

    /// A family-1 5.1 head (streams 4, coupled 2, Vorbis-order
    /// mapping 0..6 — the 0077 lane's shape).
    fn head_v1_51() -> Vec<u8> {
        let mut v = head_v0(6, 312);
        v[18] = 1;
        v.push(4); // streams
        v.push(2); // coupled
        v.extend_from_slice(&[0, 1, 2, 3, 4, 5]); // identity mapping
        v
    }

    #[test]
    fn parses_the_family0_geometry() {
        let head = parse_head(&head_v0(2, 312)).unwrap_or_else(|e| unreachable!("{e:?}"));
        assert_eq!(head.channels, 2);
        assert_eq!(head.pre_skip, 312);
        assert_eq!(head.input_rate, 48_000);
        assert_eq!(head.mapping_family, 0);
        assert_eq!(head.coupled, 1);
    }

    #[test]
    fn parses_the_51_multistream_geometry() {
        let head = parse_head(&head_v1_51()).unwrap_or_else(|e| unreachable!("{e:?}"));
        assert_eq!(head.channels, 6);
        assert_eq!(head.streams, 4);
        assert_eq!(head.coupled, 2);
        assert_eq!(head.mapping, vec![0, 1, 2, 3, 4, 5]);
    }

    /// The dotted code of a parse failure (test-side shorthand).
    fn code_of(err: OpusError) -> &'static str {
        match err {
            OpusError::Unsupported { code, .. } => code,
            OpusError::Corrupt { code, .. } => code,
            OpusError::BadState => "opus.not_configured",
        }
    }

    #[test]
    fn rejects_the_constructor_panic_windows_typed() {
        // Family 0 with 3 channels: the `OpusDecoder::new` panic
        // window — refused HERE, before any engine exists.
        assert_eq!(
            code_of(parse_head(&head_v0(3, 0)).unwrap_err()),
            "opus.head.family0_channels"
        );
        // Zero channels.
        assert_eq!(
            code_of(parse_head(&head_v0(0, 0)).unwrap_err()),
            "opus.head.channels.zero"
        );
        // Bad magic.
        let mut bad = head_v0(1, 0);
        bad[..8].copy_from_slice(b"OpusTail");
        assert_eq!(code_of(parse_head(&bad).unwrap_err()), "opus.head.magic");
        // Short chain.
        assert_eq!(
            code_of(parse_head(b"OpusH").unwrap_err()),
            "opus.head.truncated"
        );
        // Version 2.
        let mut v2 = head_v0(1, 0);
        v2[8] = 2;
        assert_eq!(code_of(parse_head(&v2).unwrap_err()), "opus.head.version");
    }

    #[test]
    fn rejects_broken_multistream_arithmetic() {
        let mut v = head_v1_51();
        v[19] = 3; // streams 3 + coupled 2 = 5 ≠ 6 channels
        assert_eq!(
            code_of(parse_head(&v).unwrap_err()),
            "opus.head.slot_arithmetic"
        );
        let mut v = head_v1_51();
        v[20] = 5; // coupled > streams
        assert_eq!(
            code_of(parse_head(&v).unwrap_err()),
            "opus.head.coupled_over_streams"
        );
        let mut v = head_v1_51();
        v[21] = 6; // mapping entry 6 targets a nonexistent slot
        assert_eq!(
            code_of(parse_head(&v).unwrap_err()),
            "opus.head.mapping_range"
        );
        let mut v = head_v0(6, 0);
        v[18] = 1; // family 1, but the multistream tail is missing
        assert_eq!(
            code_of(parse_head(&v).unwrap_err()),
            "opus.head.multistream_truncated"
        );
    }
}
