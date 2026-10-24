//! Typed parse failure for the MPEG-1/2 sequence-layer surface
//! (card 5.H1: no panics, no damage smoothing — every failure names
//! its rule; the H.264 [`H264Error`](crate::video::h264::parser::
//! H264Error) shape verbatim).
//!
//! The one engine-facing conversion is the **decode-refusal gate**:
//! [`Mpeg12Error::decode_refusal`] builds the typed
//! [`ErrorKind::Unsupported`](media_types::ErrorKind::Unsupported)
//! that the codec-api registry receives for `mpeg1video` /
//! `mpeg2video` descriptors while software decode is `blocked` —
//! an honest, observable refusal, never a silent no-op.

use media_types::error::{EngineError, ErrorContext, ErrorKind, FormatInfo, PipelineStage};

/// Typed parse failure for the sequence layer (headers, extensions,
/// slice addressing).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mpeg12Error {
    /// The header ended before the syntax element being read
    /// (`needed`/`have` in bits).
    Truncated {
        /// Bits the syntax required.
        needed: u32,
        /// Bits actually present.
        have: u32,
    },
    /// A `marker_bit` read as 0 — the bitstream is corrupt at a
    /// structural seam (ISO/IEC 13818-2 §6.2.2.1 and friends: marker
    /// bits are always 1 by construction).
    Marker {
        /// Stable dotted rule code, e.g. `mpeg12.seq.marker`.
        code: &'static str,
    },
    /// A syntax value outside its spec-declared range (forbidden
    /// `frame_rate_code`, reserved `aspect_ratio_information`,
    /// forbidden `picture_coding_type`, …).
    OutOfRange {
        /// Field name, e.g. `frame_rate_code`.
        field: &'static str,
        /// The parsed value.
        value: u64,
        /// Stable dotted rule code naming the table violated.
        code: &'static str,
    },
    /// Structural violation with a stable rule code (bad extension
    /// id, a start code this parser does not model, …).
    Invalid {
        /// Stable dotted rule code.
        code: &'static str,
    },
}

impl core::fmt::Display for Mpeg12Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Mpeg12Error::Truncated { needed, have } => {
                write!(f, "truncated: needed {needed} bits, have {have}")
            }
            Mpeg12Error::Marker { code } => write!(f, "marker bit violation: {code}"),
            Mpeg12Error::OutOfRange { field, value, code } => {
                write!(f, "{field} {value} out of range ({code})")
            }
            Mpeg12Error::Invalid { code } => write!(f, "invalid: {code}"),
        }
    }
}

impl std::error::Error for Mpeg12Error {}

/// The codec descriptors this module recognizes (the demuxers'
/// registry vocabulary: TS stream_type 0x01/0x02, PS/AVI/MP4/MKV
/// mappings all normalize to these two strings).
pub const DESCRIPTORS: [&str; 2] = ["mpeg1video", "mpeg2video"];

/// True for the two MPEG-1/2 video descriptors.
#[must_use]
pub fn accepts(descriptor: &str) -> bool {
    DESCRIPTORS.contains(&descriptor)
}

/// The typed decode refusal the codec-api registry surfaces while
/// MPEG-1/2 **software decode is blocked** (card 5.H1's honest
/// record, doc 04 §4 row "MPEG-1/2 video": no validated pure-Rust
/// engine exists; self-build is an L-sized VLC/ffmpeg-class project,
/// queued Phase 6+). The sequence layer under this module is what
/// probe/track-info consume; decode itself refuses loudly.
///
/// Refuses any descriptor that is not `mpeg1video`/`mpeg2video` with
/// the same gate shape (a foreign descriptor through this gate is a
/// registry wiring bug, not a stream property).
pub fn decode_refusal(descriptor: &str) -> EngineError {
    let detail = if accepts(descriptor) {
        format!(
            "MPEG-1/2 software decode is blocked (no validated pure-Rust engine; \
             self-build is L-sized, doc 04 §4): descriptor `{descriptor}` carries \
             sequence-layer facts only — probe/track-info, not pixels"
        )
    } else {
        format!("codec descriptor `{descriptor}` is neither `mpeg1video` nor `mpeg2video`")
    };
    EngineError::new(
        PipelineStage::Decode,
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some(descriptor.to_string()),
                sniffed: Vec::new(),
            }),
        },
        ErrorContext::new("mpeg12.decode.blocked"),
    )
    .with_source(Detail(detail))
}

/// Chain-root carrier for the human-readable reason (the 2.B4
/// `FeatureDetail` pattern: the reason rides in `source`, never in
/// `Display`'s no-secrets surface).
#[derive(Debug, Clone)]
struct Detail(String);

impl core::fmt::Display for Detail {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Detail {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

    #[test]
    fn descriptor_gate_accepts_exactly_the_two_ids() {
        assert!(accepts("mpeg1video"));
        assert!(accepts("mpeg2video"));
        for foreign in ["mpeg4", "h264", "mjpeg", "", "MPEG2VIDEO"] {
            assert!(!accepts(foreign), "{foreign} must not gate in");
        }
    }

    #[test]
    fn decode_refusal_is_typed_unsupported_for_both_descriptors() {
        for id in DESCRIPTORS {
            let err = decode_refusal(id);
            assert!(
                matches!(err.kind, ErrorKind::Unsupported { .. }),
                "{id}: {}",
                err
            );
            assert_eq!(err.context.code, "mpeg12.decode.blocked");
        }
    }

    #[test]
    fn decode_refusal_names_foreign_descriptors() {
        let err = decode_refusal("h264");
        assert!(matches!(err.kind, ErrorKind::Unsupported { .. }));
        let reason = err
            .source()
            .and_then(|s| s.downcast_ref::<Detail>())
            .map(|d| d.0.as_str())
            .unwrap_or_default();
        assert!(reason.contains("neither"), "{reason}");
    }

    #[test]
    fn parse_errors_render_their_rule() {
        assert_eq!(
            Mpeg12Error::Truncated {
                needed: 12,
                have: 3
            }
            .to_string(),
            "truncated: needed 12 bits, have 3"
        );
        assert_eq!(
            Mpeg12Error::Marker {
                code: "mpeg12.seq.marker"
            }
            .to_string(),
            "marker bit violation: mpeg12.seq.marker"
        );
    }
}
