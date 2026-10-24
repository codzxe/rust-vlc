//! Vorbis comment header (card 5.B2's metadata exposure): the
//! `vendor` string + the `key=value` user-comment list (spec §5.2,
//! "field name" = the key up to the first '=').
//!
//! This is OUR parser, not the engine's — symphonia-codec-vorbis does
//! not expose the comment header at all (it is skipped in
//! `read_extradata`), which is exactly why the header chain stays in
//! our hands ([`super::headers`]). FLAC's VORBIS_COMMENT shares this
//! wire form minus the `\x03vorbis` prefix; the body parser here
//! (`[`parse_comment_body`]`) is written field-generic so a future
//! FLAC reuse is a two-line composition, not a fork.
//!
//! # Field conventions surfaced
//!
//! - Keys are ASCII case-insensitive per the spec ("field names are
//!   not case sensitive"); lookups are case-folded ASCII.
//! - Values are UTF-8 (typed rejection on invalid bytes — the spec's
//!   own recommendation for strict consumers).
//! - Cover art rides `METADATA_BLOCK_PICTURE` (base64 FLAC picture
//!   block); it is exposed as the raw string value here — decoding it
//!   is the metadata-layer's job (5.R3 registry), not the decoder's.
//!
//! Nothing panics; every length is bounds-checked before use (the
//! `u32` length fields cannot buy an over-read, only a typed
//! truncation).

use super::error::{VorbisError, corrupt};
use media_types::EngineError;

/// The comment header's packet-type byte.
const TYPE_COMMENT: u8 = 0x03;
/// The common header signature.
const MAGIC: &[u8] = b"vorbis";
/// A sanity ceiling for one comment's length (a comment body beyond
/// 16 MiB is damage, not metadata `[PROPOSAL: mirrors demux::ogg's
/// MAX_HEADER_BYTES scale]`).
const MAX_ONE_COMMENT: usize = 16 * 1024 * 1024;

/// The parsed comment header.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VorbisComments {
    /// The encoder's vendor string (e.g. `libvorbis I 2023…`).
    pub vendor: String,
    /// The user comments, in stream order (`key` case-preserved,
    /// compared case-insensitively).
    pub comments: Vec<(String, String)>,
}

impl VorbisComments {
    /// First value whose key matches `field` (ASCII case-insensitive),
    /// `None` when absent.
    #[must_use]
    pub fn get(&self, field: &str) -> Option<&str> {
        self.comments
            .iter()
            .find(|(key, _)| eq_ignore_ascii_case(key, field))
            .map(|(_, value)| value.as_str())
    }

    /// All values whose key matches `field` (repeatable tags — e.g.
    /// `ARTIST` — keep their multiplicity, spec §5.2.2).
    #[must_use]
    pub fn get_all(&self, field: &str) -> Vec<&str> {
        self.comments
            .iter()
            .filter(|(key, _)| eq_ignore_ascii_case(key, field))
            .map(|(_, value)| value.as_str())
            .collect()
    }

    /// The `TITLE` tag, when present.
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.get("TITLE")
    }

    /// The `ARTIST` tag(s).
    #[must_use]
    pub fn artists(&self) -> Vec<&str> {
        self.get_all("ARTIST")
    }

    /// The `ALBUM` tag, when present.
    #[must_use]
    pub fn album(&self) -> Option<&str> {
        self.get("ALBUM")
    }

    /// The `DATE` tag, when present.
    #[must_use]
    pub fn date(&self) -> Option<&str> {
        self.get("DATE")
    }
}

/// ASCII case-insensitive equality (the spec's field-name rule:
/// same length, then byte-wise `u8::eq_ignore_ascii_case`).
fn eq_ignore_ascii_case(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .all(|(x, y)| x.eq_ignore_ascii_case(&y))
}

/// Measure one comment header's full byte length (type + magic +
/// body), for the chain split in [`super::headers`].
///
/// # Errors
/// `vorbis.comment.truncated` when the length fields run past the
/// buffer, `vorbis.comment.magic` when the prefix is wrong.
pub(super) fn header_len(data: &[u8]) -> Result<usize, VorbisError> {
    if data.len() < 7 || data[0] != TYPE_COMMENT || &data[1..7] != MAGIC {
        return Err(corrupt(
            "vorbis.comment.magic",
            media_types::error::CorruptDetail::InvalidStructure { offset: Some(0) },
            "the second header is not a Vorbis comment header (\\x03vorbis)",
        ));
    }
    let mut pos = 0usize;
    let vendor_len = read_len(&data[7..], &mut pos, "vendor")?;
    read_slice(&data[7..], &mut pos, vendor_len, "vendor")?;
    let count = read_len(&data[7..], &mut pos, "count")?;
    let count = count.min(data[7..].len().saturating_sub(pos) / 4 + 1);
    for _ in 0..count {
        let len = read_len(&data[7..], &mut pos, "comment length")?;
        read_slice(&data[7..], &mut pos, len, "comment")?;
    }
    // The framing byte terminates the header.
    if data[7..].get(pos).is_none() {
        return Err(corrupt(
            "vorbis.comment.truncated",
            media_types::error::CorruptDetail::Truncated {
                expected: u64::try_from(7 + pos + 1).unwrap_or(u64::MAX),
                found: data.len() as u64,
            },
            "comment header ends before its framing byte",
        ));
    }
    Ok(7 + pos + 1)
}

/// Parse one full comment header packet (`\x03vorbis` + body).
///
/// # Errors
/// `vorbis.comment.magic`, `vorbis.comment.truncated`,
/// `vorbis.comment.framing` (framing bit clear),
/// `vorbis.comment.utf8` (a non-UTF-8 value),
/// `vorbis.comment.field` (a comment without `=`).
pub fn parse(data: &[u8]) -> Result<VorbisComments, EngineError> {
    if data.len() < 7 || data[0] != TYPE_COMMENT || &data[1..7] != MAGIC {
        return Err(corrupt(
            "vorbis.comment.magic",
            media_types::error::CorruptDetail::InvalidStructure { offset: Some(0) },
            "the packet is not a Vorbis comment header (\\x03vorbis)",
        )
        .into_engine_error());
    }
    parse_comment_body(&data[7..]).map_err(VorbisError::into_engine_error)
}

/// Parse the comment body (everything after `\x03vorbis`): vendor
/// length + vendor, count, then `length + bytes` pairs, then the
/// framing byte.
///
/// # Errors
/// See [`parse`].
pub fn parse_comment_body(data: &[u8]) -> Result<VorbisComments, VorbisError> {
    let mut pos = 0usize;
    let vendor_len = read_len(data, &mut pos, "vendor")?;
    let vendor_bytes = read_slice(data, &mut pos, vendor_len, "vendor")?;
    let vendor = decode_utf8(vendor_bytes, "vorbis.comment.utf8", "vendor string")?;
    let count = read_len(data, &mut pos, "count")?;
    // The comment list's total is bounded by the buffer itself (each
    // read slices `data`), but a lying count must not buy an
    // unbounded loop: `count` is clamped to the buffer's remaining
    // bytes (one comment costs ≥ 4 bytes).
    let count = count.min(data.len().saturating_sub(pos) / 4 + 1);
    let mut comments = Vec::new();
    for _ in 0..count {
        let len = read_len(data, &mut pos, "comment length")?;
        let bytes = read_slice(data, &mut pos, len, "comment")?;
        let text = decode_utf8(bytes, "vorbis.comment.utf8", "comment value")?;
        match text.split_once('=') {
            Some((key, value)) => comments.push((key.to_string(), value.to_string())),
            None => {
                return Err(corrupt(
                    "vorbis.comment.field",
                    media_types::error::CorruptDetail::InvalidStructure { offset: None },
                    "a user comment carries no '=' field separator",
                ));
            }
        }
    }
    let framing = data.get(pos).copied().unwrap_or(0);
    if framing & 0x01 == 0 {
        return Err(corrupt(
            "vorbis.comment.framing",
            media_types::error::CorruptDetail::InvalidStructure {
                offset: Some(pos as u64),
            },
            "comment header framing bit is not set",
        ));
    }
    Ok(VorbisComments { vendor, comments })
}

/// Read one little-endian `u32` length at `*pos`, advancing it.
fn read_len(data: &[u8], pos: &mut usize, what: &str) -> Result<usize, VorbisError> {
    let bytes = read_slice(data, pos, 4, what)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize)
}

/// Read `len` bytes at `*pos`, advancing it (typed truncation on any
/// overrun; the `MAX_ONE_COMMENT` ceiling guards absurd claims).
fn read_slice<'a>(
    data: &'a [u8],
    pos: &mut usize,
    len: usize,
    what: &str,
) -> Result<&'a [u8], VorbisError> {
    if len > MAX_ONE_COMMENT {
        return Err(corrupt(
            "vorbis.comment.truncated",
            media_types::error::CorruptDetail::Truncated {
                expected: MAX_ONE_COMMENT as u64,
                found: len as u64,
            },
            format!("{what} length {len} exceeds the {MAX_ONE_COMMENT}-byte ceiling"),
        ));
    }
    match data.get(*pos..*pos + len) {
        Some(slice) => {
            *pos += len;
            Ok(slice)
        }
        None => Err(corrupt(
            "vorbis.comment.truncated",
            media_types::error::CorruptDetail::Truncated {
                expected: u64::try_from(*pos + len).unwrap_or(u64::MAX),
                found: data.len() as u64,
            },
            format!("{what} runs past the end of the comment header"),
        )),
    }
}

/// UTF-8 validation with the module's typed error shape.
fn decode_utf8(bytes: &[u8], code: &'static str, what: &str) -> Result<String, VorbisError> {
    match std::str::from_utf8(bytes) {
        Ok(text) => Ok(text.to_string()),
        Err(err) => Err(corrupt(
            code,
            media_types::error::CorruptDetail::InvalidStructure { offset: None },
            format!("{what} is not valid UTF-8: {err}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A hand-built comment header: vendor + two comments + framing.
    fn comment_header() -> Vec<u8> {
        let mut out = Vec::new();
        out.push(TYPE_COMMENT);
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&(5u32).to_le_bytes());
        out.extend_from_slice(b"lumen");
        out.extend_from_slice(&(2u32).to_le_bytes());
        out.extend_from_slice(&(11u32).to_le_bytes());
        out.extend_from_slice(b"TiTle=hello");
        out.extend_from_slice(&(9u32).to_le_bytes());
        out.extend_from_slice(b"artist=me");
        out.push(0x01);
        out
    }

    #[test]
    fn comments_roundtrip_with_case_insensitive_lookup() {
        let parsed = match parse(&comment_header()) {
            Ok(parsed) => parsed,
            Err(err) => unreachable!("{err:?}"),
        };
        assert_eq!(parsed.vendor, "lumen");
        assert_eq!(parsed.title(), Some("hello"));
        assert_eq!(parsed.artists(), vec!["me"]);
        assert_eq!(parsed.album(), None);
        // The stored key is case-preserved; lookup is folded.
        assert_eq!(parsed.comments[0].0, "TiTle");
        assert_eq!(parsed.get("title"), Some("hello"));
    }

    #[test]
    fn repeatable_tags_keep_multiplicity() {
        let mut out = Vec::new();
        out.push(TYPE_COMMENT);
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&0u32.to_le_bytes()); // empty vendor
        out.extend_from_slice(&2u32.to_le_bytes());
        for pair in ["artist=a", "ARTIST=b"] {
            out.extend_from_slice(&(pair.len() as u32).to_le_bytes());
            out.extend_from_slice(pair.as_bytes());
        }
        out.push(0x01);
        let parsed = match parse(&out) {
            Ok(parsed) => parsed,
            Err(err) => unreachable!("{err:?}"),
        };
        assert_eq!(parsed.artists(), vec!["a", "b"]);
        assert_eq!(parsed.get_all("artist").len(), 2);
    }

    #[test]
    fn damage_is_typed_never_panicking() {
        // Truncated mid-vendor.
        let full = comment_header();
        let err = match parse(&full[..12]) {
            Err(err) => err,
            Ok(parsed) => unreachable!("must refuse: {parsed:?}"),
        };
        assert_eq!(err.context.code, "vorbis.comment.truncated");
        // Framing bit clear.
        let mut unframed = full.clone();
        *unframed.last_mut().unwrap_or(&mut 0) = 0;
        let err = match parse(&unframed) {
            Err(err) => err,
            Ok(parsed) => unreachable!("must refuse: {parsed:?}"),
        };
        assert_eq!(err.context.code, "vorbis.comment.framing");
        // Wrong magic.
        let mut bad = full;
        bad[0] = 0x01;
        let err = match parse(&bad) {
            Err(err) => err,
            Ok(parsed) => unreachable!("must refuse: {parsed:?}"),
        };
        assert_eq!(err.context.code, "vorbis.comment.magic");
        // A comment without '='.
        let mut noeq = Vec::new();
        noeq.push(TYPE_COMMENT);
        noeq.extend_from_slice(MAGIC);
        noeq.extend_from_slice(&0u32.to_le_bytes());
        noeq.extend_from_slice(&1u32.to_le_bytes());
        noeq.extend_from_slice(&(3u32).to_le_bytes());
        noeq.extend_from_slice(b"abc");
        noeq.push(0x01);
        let err = match parse(&noeq) {
            Err(err) => err,
            Ok(parsed) => unreachable!("must refuse: {parsed:?}"),
        };
        assert_eq!(err.context.code, "vorbis.comment.field");
        // A lying count (claims 1000 comments) must not over-read or
        // hang — the clamp keeps it a typed truncation.
        let mut lying = Vec::new();
        lying.push(TYPE_COMMENT);
        lying.extend_from_slice(MAGIC);
        lying.extend_from_slice(&0u32.to_le_bytes());
        lying.extend_from_slice(&1000u32.to_le_bytes());
        lying.extend_from_slice(&(3u32).to_le_bytes());
        lying.extend_from_slice(b"a=b");
        lying.push(0x01);
        let err = match parse(&lying) {
            Err(err) => err,
            Ok(parsed) => unreachable!("must refuse: {parsed:?}"),
        };
        assert_eq!(err.context.code, "vorbis.comment.truncated");
    }

    #[test]
    fn body_length_measurement_feeds_the_chain_split() {
        let full = comment_header();
        let len = match header_len(&full) {
            Ok(len) => len,
            Err(err) => unreachable!("{err:?}"),
        };
        assert_eq!(len, full.len());
    }
}
