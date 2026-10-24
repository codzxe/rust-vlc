//! ID3v2 metadata (card 5.A3): the 2.2/2.3/2.4 tag skip + parse.
//!
//! An MP3 stream may start (and, with tags-after-media, end) with an
//! ID3v2 tag. The demuxer primarily needs its **total length** to
//! skip to the first audio frame; the player wants the text frames
//! (title/artist/…) and the cover-art blobs. This module parses the
//! full tag structure:
//!
//! - the 10-byte header (magic `ID3`, version, flags, **syncsafe**
//!   28-bit size — 4×7 bits, big-endian, per spec §3.1),
//! - v2.2's 6-byte frames (`TT2`, `PIC`, …, 3-byte sizes),
//! - v2.3's 10-byte frames (plain 32-bit sizes, tag-global
//!   unsynchronisation, optional extended header),
//! - v2.4's 10-byte frames (**syncsafe** sizes, per-frame
//!   unsynchronisation flag, optional extended header + footer),
//! - the `unsynchronisation` scheme (`FF 00` → `FF`) applied
//!   tag-globally in ≤2.3 and per-frame in 2.4,
//! - the 10-byte footer (`3DI` mirror) counted into the skip length.
//!
//! Error paths (each typed, per the card): a tag claiming more bytes
//! than the buffer holds, a frame running past the tag body, a bad
//! text encoding byte, a truncated syncsafe size field. `None` (not
//! an error) when there is no `ID3` magic at offset 0 — probing is
//! [`id3v2_len`].

use super::error::{corrupt, unsupported};
use media_types::EngineError;
use media_types::error::CorruptDetail;

/// One parsed ID3v2 frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Id3Frame {
    /// A text frame (TIT2/TALB/… or v2.2 TT2/TAL…), decoded to UTF-8.
    Text {
        /// The frame id (`TIT2` in v2.3/2.4, `TT2` in v2.2).
        id: String,
        /// The decoded value.
        value: String,
    },
    /// A picture frame (APIC/PIC): kept raw (the embedded image bytes
        /// are the picture pipeline's input, not ours).
    Picture {
        /// MIME type (`JPG`/`PNG` in v2.2, `image/jpeg`… later).
        mime: String,
        /// The picture type byte (03 = front cover, …).
        picture_type: u8,
        /// The raw image bytes (after the description terminator).
        data: Vec<u8>,
    },
    /// Any other frame, id + raw payload (kept for completeness of
        /// the walk; unknown frames are skipped, not fatal).
    Other {
        /// The frame id.
        id: String,
        /// Payload bytes after the frame header.
        data: Vec<u8>,
    },
}

/// A parsed ID3v2 tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Id3Tag {
    /// The version tuple (major, minor): (2,2), (2,3) or (2,4).
    pub version: (u8, u8),
    /// True when the tag carried the unsynchronisation flag.
    pub unsynchronised: bool,
    /// True when a 10-byte footer was present (v2.4).
    pub has_footer: bool,
    /// The parsed frames (text/picture/other, in file order).
    pub frames: Vec<Id3Frame>,
    /// Total tag length in bytes INCLUDING header and footer — the
    /// number a demuxer skips to reach audio.
    pub total_len: usize,
}

impl Id3Tag {
    /// The first text frame's value for an id (`TIT2` in v2.3/2.4
    /// spelling; v2.2 callers pass `TT2`).
    #[must_use]
    pub fn text(&self, id: &str) -> Option<&str> {
        self.frames.iter().find_map(|f| match f {
            Id3Frame::Text { id: fid, value } if fid == id => Some(value.as_str()),
            _ => None,
        })
    }
}

/// Decode a syncsafe 4×7-bit big-endian integer (the ID3v2 size
/// fields). `None` when fewer than 4 bytes.
fn syncsafe32(b: &[u8]) -> Option<u32> {
    let q: [u8; 4] = b.get(..4)?.try_into().ok()?;
    // A set high bit in any byte is a spec violation; tolerate it by
    // masking (never a panic, damage noted by the caller when sizes
    // disagree — the honest behavior for damaged tags).
    Some(
        (u32::from(q[0] & 0x7F) << 21)
            | (u32::from(q[1] & 0x7F) << 14)
            | (u32::from(q[2] & 0x7F) << 7)
            | u32::from(q[3] & 0x7F),
    )
}

/// Reverse the unsynchronisation scheme in place: every `FF 00`
/// becomes `FF`. Returns the de-unsynchronised buffer.
fn deunsynchronise(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        out.push(data[i]);
        if data[i] == 0xFF && data.get(i + 1) == Some(&0x00) {
            i += 2;
        } else {
            i += 1;
        }
    }
    out
}

/// Decode an ID3 text payload (the encoding byte + data) to UTF-8.
/// Returns a typed error for an unknown encoding byte; empty values
/// are legal ("" returned).
fn decode_text(payload: &[u8]) -> Result<String, EngineError> {
    let Some((&enc, rest)) = payload.split_first() else {
        return Ok(String::new());
    };
    match enc {
        0 => {
            // ISO-8859-1: map 1:1 to Unicode code points.
            let s: String = rest
                .iter()
                .map(|&b| char::from_u32(u32::from(b)).unwrap_or('\u{FFFD}'))
                .collect();
            Ok(s.trim_end_matches('\0').to_string())
        }
        3 => Ok(String::from_utf8_lossy(rest)
            .trim_end_matches('\0')
            .to_string()),
        1 => {
            // UTF-16 with BOM.
            let (le, body) = match rest.get(..2) {
                Some([0xFF, 0xFE]) => (true, &rest[2..]),
                Some([0xFE, 0xFF]) => (false, &rest[2..]),
                _ => (true, rest),
            };
            let units: Vec<u16> = body
                .chunks_exact(2)
                .take_while(|c| !(c[0] == 0 && c[1] == 0))
                .map(|c| {
                    if le {
                        u16::from_le_bytes([c[0], c[1]])
                    } else {
                        u16::from_be_bytes([c[0], c[1]])
                    }
                })
                .collect();
            Ok(String::from_utf16_lossy(&units))
        }
        2 => {
            // UTF-16BE without BOM.
            let units: Vec<u16> = body_utf16be(rest);
            Ok(String::from_utf16_lossy(&units))
        }
        other => Err(corrupt(
            "mp3.id3.text_encoding",
            CorruptDetail::InvalidStructure { offset: None },
            format!("unknown text encoding byte {other}"),
        )
        .into_engine_error()),
    }
}

fn body_utf16be(rest: &[u8]) -> Vec<u16> {
    rest.chunks_exact(2)
        .take_while(|c| !(c[0] == 0 && c[1] == 0))
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect()
}

/// The total byte length of the ID3v2 tag at the front of `data`
/// (header + body + footer), or `None` when there is no tag.
///
/// `Err` when the header is truncated or the declared body runs past
/// the end of `data` — the card's "ID3 tag claiming more than file"
/// path.
pub fn id3v2_len(data: &[u8]) -> Result<Option<usize>, EngineError> {
    if data.get(..3) != Some(b"ID3") {
        return Ok(None);
    }
    let head = match data.get(..10) {
        Some(h) => h,
        None => {
            return Err(corrupt(
                "mp3.id3.header_truncated",
                CorruptDetail::Truncated {
                    expected: 10,
                    found: data.len() as u64,
                },
                "ID3v2 header needs 10 bytes",
            )
            .into_engine_error());
        }
    };
    let version = (head[3], head[4]);
    if head[3] < 2 || head[3] > 4 {
        return Err(unsupported(
            "mp3.id3.version",
            format!("ID3v2.{} is not a 2.2/2.3/2.4 tag", head[3]),
        )
        .into_engine_error());
    }
    let Some(body) = syncsafe32(&head[6..10]) else {
        return Err(corrupt(
            "mp3.id3.size_field",
            CorruptDetail::Truncated {
                expected: 10,
                found: data.len() as u64,
            },
            "syncsafe size field missing",
        )
        .into_engine_error());
    };
    let footer = version.0 >= 4 && head[5] & 0x10 != 0;
    let total = 10 + body as usize + usize::from(footer) * 10;
    if total > data.len() {
        return Err(corrupt(
            "mp3.id3.past_end",
            CorruptDetail::Truncated {
                expected: total as u64,
                found: data.len() as u64,
            },
            format!(
                "ID3v2.{} tag claims {total} bytes but only {} follow the header",
                version.0,
                data.len() - 10
            ),
        )
        .into_engine_error());
    }
    Ok(Some(total))
}

/// Parse the ID3v2 tag at the front of `data` (assumes the caller
/// checked `data` starts with `ID3`; returns the tag AND consumes
/// exactly `tag.total_len`).
///
/// # Errors
/// Typed [`EngineError`]s: truncated header/body, a frame running
/// past the tag, an unknown text encoding. Unknown frame *ids* are
/// not errors (kept as [`Id3Frame::Other`]).
pub fn parse_id3v2(data: &[u8]) -> Result<Id3Tag, EngineError> {
    let Some(total) = id3v2_len(data)? else {
        return Err(corrupt(
            "mp3.id3.magic",
            CorruptDetail::InvalidStructure { offset: Some(0) },
            "parse_id3v2 called on data without an ID3 magic",
        )
        .into_engine_error());
    };
    let head = &data[..10];
    let version = (head[3], head[4]);
    let flags = head[5];
    let unsynchronised = flags & 0x80 != 0;
    let extended = flags & 0x40 != 0;
    let footer = version.0 >= 4 && flags & 0x10 != 0;
    let body_end = total - usize::from(footer) * 10;
    // De-unsynchronise the body (≤2.3: tag-global; 2.4: per frame).
    let body: Vec<u8> = if unsynchronised && version.0 < 4 {
        deunsynchronise(&data[10..body_end])
    } else {
        data[10..body_end].to_vec()
    };
    let mut pos = 0usize;
    // Extended header: skip its declared size.
    if extended {
        let skipped = match version.0 {
            3 => {
                // v2.3: 4-byte plain BE size (excludes itself) + 2 flag
                // bytes + 4-byte padding size; the size counts the rest
                // of the extended header.
                let sz = u32::from_be_bytes([
                    *body.get(pos).unwrap_or(&0),
                    *body.get(pos + 1).unwrap_or(&0),
                    *body.get(pos + 2).unwrap_or(&0),
                    *body.get(pos + 3).unwrap_or(&0),
                ]) as usize;
                4 + sz
            }
            _ => {
                // v2.4: syncsafe size INCLUDING itself.
                syncsafe32(&body[pos..]).unwrap_or(0) as usize
            }
        };
        pos = (pos + skipped).min(body.len());
    }
    let mut frames = Vec::new();
    let id_len = if version.0 == 2 { 3 } else { 4 };
    let head_len = if version.0 == 2 { 6 } else { 10 };
    while pos + head_len <= body.len() {
        let id_bytes = &body[pos..pos + id_len];
        // A zero frame id = padding to the tag's end.
        if id_bytes.iter().all(|&b| b == 0) {
            break;
        }
        let id = String::from_utf8_lossy(id_bytes).to_string();
        let (size, frame_flags) = if version.0 == 2 {
            let s = (u32::from(body[pos + 3]) << 16)
                | (u32::from(body[pos + 4]) << 8)
                | u32::from(body[pos + 5]);
            (s as usize, 0u8)
        } else {
            let raw = [
                body[pos + 4],
                body[pos + 5],
                body[pos + 6],
                body[pos + 7],
            ];
            let s = if version.0 >= 4 {
                syncsafe32(&raw).unwrap_or(0) as usize
            } else {
                u32::from_be_bytes(raw) as usize
            };
            (s, body[pos + 8 + 1])
        };
        let start = pos + head_len;
        let end = start.saturating_add(size);
        if end > body.len() {
            return Err(corrupt(
                "mp3.id3.frame_past_end",
                CorruptDetail::Truncated {
                    expected: end as u64,
                    found: body.len() as u64,
                },
                format!("frame `{id}` runs past the tag body"),
            )
            .into_engine_error());
        }
        let mut payload = &body[start..end];
        // v2.4 per-frame unsynchronisation.
        let deunsynced;
        if version.0 >= 4 && frame_flags & 0x02 != 0 {
            deunsynced = deunsynchronise(payload);
            payload = &deunsynced;
        }
        // v2.3/2.4 compression and encryption are refused, not guessed.
        let compressed_23 = version.0 == 3 && frame_flags & 0x0080 != 0;
        let compressed_24 = version.0 >= 4 && frame_flags & 0x08 != 0;
        let encrypted = version.0 >= 3 && frame_flags & 0x04 != 0;
        if compressed_23 || compressed_24 || encrypted {
            return Err(unsupported(
                "mp3.id3.frame_transform",
                format!("frame `{id}` is compressed or encrypted"),
            )
            .into_engine_error());
        }
        if id.starts_with('T') {
            let value = decode_text(payload)?;
            frames.push(Id3Frame::Text { id, value });
        } else if id == "APIC" || id == "PIC" {
            frames.push(parse_picture(&id, payload));
        } else {
            frames.push(Id3Frame::Other {
                id,
                data: payload.to_vec(),
            });
        }
        pos = end;
    }
    Ok(Id3Tag {
        version,
        unsynchronised,
        has_footer: footer,
        frames,
        total_len: total,
    })
}

/// Parse an APIC (v2.3/2.4) or PIC (v2.2) payload.
fn parse_picture(id: &str, payload: &[u8]) -> Id3Frame {
    // APIC: encoding, mime\0, type, description\0, data.
    // PIC: encoding, 3-byte image format, type, description\0, data.
    let mut i = 0usize;
    let _enc = payload.first().copied().unwrap_or(0);
    i += 1;
    let mime = if id == "PIC" {
        i += 3;
        String::from_utf8_lossy(payload.get(1..4.min(payload.len())).unwrap_or(b"JPG")).to_string()
    } else {
        let end = payload[i..]
            .iter()
            .position(|&b| b == 0)
            .map(|p| i + p)
            .unwrap_or(payload.len());
        let m = String::from_utf8_lossy(payload.get(i..end).unwrap_or(b"")).to_string();
        i = (end + 1).min(payload.len());
        m
    };
    let picture_type = payload.get(i).copied().unwrap_or(0);
    i += 1;
    // Skip the description (encoding-aware terminator).
    let wide = matches!(_enc, 1 | 2);
    let data_start = if wide {
        let mut j = i;
        while j + 1 < payload.len() {
            if payload[j] == 0 && payload[j + 1] == 0 {
                break;
            }
            j += 2;
        }
        (j + 2).min(payload.len())
    } else {
        let end = payload[i..]
            .iter()
            .position(|&b| b == 0)
            .map(|p| i + p)
            .unwrap_or(payload.len());
        (end + 1).min(payload.len())
    };
    Id3Frame::Picture {
        mime,
        picture_type,
        data: payload.get(data_start..).unwrap_or(&[]).to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal v2.3 tag with one TIT2 frame.
    fn tag_v23(title: &str) -> Vec<u8> {
        let text = format!("\0{title}");
        let body_len = 10 + text.len();
        let mut v = Vec::new();
        v.extend_from_slice(b"ID3");
        v.push(3);
        v.push(0);
        v.push(0); // flags
        v.push(((body_len >> 21) & 0x7F) as u8);
        v.push(((body_len >> 14) & 0x7F) as u8);
        v.push(((body_len >> 7) & 0x7F) as u8);
        v.push((body_len & 0x7F) as u8);
        v.extend_from_slice(b"TIT2");
        v.extend_from_slice(&(text.len() as u32).to_be_bytes());
        v.push(0);
        v.push(0);
        v.extend_from_slice(text.as_bytes());
        v
    }

    #[test]
    fn skips_and_parses_a_v23_tag() {
        let tag = tag_v23("Interstellar");
        let len = match id3v2_len(&tag) {
            Ok(Some(l)) => l,
            other => unreachable!("{other:?}"),
        };
        assert_eq!(len, tag.len());
        let parsed = match parse_id3v2(&tag) {
            Ok(t) => t,
            Err(e) => unreachable!("{e:?}"),
        };
        assert_eq!(parsed.version, (3, 0));
        assert_eq!(parsed.text("TIT2"), Some("Interstellar"));
        assert_eq!(parsed.total_len, tag.len());
    }

    #[test]
    fn a_tag_claiming_more_than_the_file_is_typed_truncation() {
        let mut tag = tag_v23("x");
        // Declare 1000 bytes of body; supply ~20.
        tag[6] = 0;
        tag[7] = 0;
        tag[8] = 0x07;
        tag[9] = 0x68; // 1000
        let e = match id3v2_len(&tag) {
            Err(e) => e,
            Ok(_) => unreachable!("oversized claim must fail"),
        };
        assert_eq!(e.context.code, "mp3.id3.past_end");
    }

    #[test]
    fn v24_footer_and_syncsafe_frame_sizes() {
        // Frame: TALB + syncsafe size 7 + flags(2) + "\0album!".
        let payload = b"\0album!";
        let mut body = Vec::new();
        body.extend_from_slice(b"TALB");
        body.extend_from_slice(&[0, 0, 0, payload.len() as u8]); // syncsafe 7
        body.extend_from_slice(&[0, 0]); // format flags
        body.extend_from_slice(payload);
        let total_body = body.len();
        let mut v = Vec::new();
        v.extend_from_slice(b"ID3");
        v.extend_from_slice(&[4, 0, 0x10]); // v2.4 + footer flag
        v.push(((total_body >> 21) & 0x7F) as u8);
        v.push(((total_body >> 14) & 0x7F) as u8);
        v.push(((total_body >> 7) & 0x7F) as u8);
        v.push((total_body & 0x7F) as u8);
        v.extend_from_slice(&body);
        v.extend_from_slice(&[0; 10]); // footer mirror
        let len = match id3v2_len(&v) {
            Ok(Some(l)) => l,
            other => unreachable!("{other:?}"),
        };
        assert_eq!(len, v.len(), "footer counted");
        let parsed = match parse_id3v2(&v) {
            Ok(t) => t,
            Err(e) => unreachable!("{e:?}"),
        };
        assert!(parsed.has_footer);
        assert_eq!(parsed.text("TALB"), Some("album!"));
    }

    #[test]
    fn unsynchronisation_is_reversed() {
        // One TIT2 frame whose latin-1 logical payload is [0, 0xFF, 0x41]
        // ("\u{FF}A"). On disk the unsync scheme inserts a 0x00 after
        // the FF; v2.3 sizes are declared against the LOGICAL content.
        let logical = [0x00u8, 0xFF, 0x41];
        let on_disk_payload = [0x00u8, 0xFF, 0x00, 0x41];
        let mut body = Vec::new();
        body.extend_from_slice(b"TIT2");
        body.extend_from_slice(&(logical.len() as u32).to_be_bytes());
        body.extend_from_slice(&[0, 0]);
        body.extend_from_slice(&on_disk_payload);
        let unsync_body_len = body.len();
        let mut v = Vec::new();
        v.extend_from_slice(b"ID3");
        v.extend_from_slice(&[3, 0, 0x80]); // unsync flag
        v.push(((unsync_body_len >> 21) & 0x7F) as u8);
        v.push(((unsync_body_len >> 14) & 0x7F) as u8);
        v.push(((unsync_body_len >> 7) & 0x7F) as u8);
        v.push((unsync_body_len & 0x7F) as u8);
        v.extend_from_slice(&body);
        let parsed = match parse_id3v2(&v) {
            Ok(t) => t,
            Err(e) => unreachable!("{e:?}"),
        };
        assert!(parsed.unsynchronised);
        assert_eq!(parsed.text("TIT2"), Some("\u{FF}A"));
    }

    #[test]
    fn v22_three_char_ids_and_picture_frames() {
        // v2.2: TT2 frame, 3-byte size.
        let text = [0u8, b'H', b'i'];
        let mut body = Vec::new();
        body.extend_from_slice(b"TT2");
        body.push(0);
        body.push(0);
        body.push(text.len() as u8);
        body.extend_from_slice(&text);
        let mut v = Vec::new();
        v.extend_from_slice(b"ID3");
        v.extend_from_slice(&[2, 0, 0]);
        v.push(((body.len() >> 21) & 0x7F) as u8);
        v.push(((body.len() >> 14) & 0x7F) as u8);
        v.push(((body.len() >> 7) & 0x7F) as u8);
        v.push((body.len() & 0x7F) as u8);
        v.extend_from_slice(&body);
        let parsed = match parse_id3v2(&v) {
            Ok(t) => t,
            Err(e) => unreachable!("{e:?}"),
        };
        assert_eq!(parsed.version, (2, 0));
        assert_eq!(parsed.text("TT2"), Some("Hi"));
        // Picture parse unit: APIC payload shape.
        let mut pic = Vec::new();
        pic.push(0); // latin-1
        pic.extend_from_slice(b"image/png\0");
        pic.push(3); // front cover
        pic.extend_from_slice(b"cover\0");
        pic.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47]);
        let frame = parse_picture("APIC", &pic);
        match frame {
            Id3Frame::Picture {
                mime,
                picture_type,
                data,
            } => {
                assert_eq!(mime, "image/png");
                assert_eq!(picture_type, 3);
                assert_eq!(data, [0x89, 0x50, 0x4E, 0x47]);
            }
            other => unreachable!("{other:?}"),
        }
    }

    #[test]
    fn bad_text_encoding_is_typed() {
        let e = match decode_text(&[9, b'x']) {
            Err(e) => e,
            Ok(_) => unreachable!("encoding 9 must fail"),
        };
        assert_eq!(e.context.code, "mp3.id3.text_encoding");
    }

    #[test]
    fn no_magic_is_none_not_error() {
        assert!(matches!(id3v2_len(&[0xFF, 0xFB, 0x90, 0x00]), Ok(None)));
        assert!(matches!(id3v2_len(&[]), Ok(None)));
        // Truncated magic-bearing header is the typed path.
        let e = match id3v2_len(b"ID3\x03\x00") {
            Err(e) => e,
            Ok(_) => unreachable!("short header must fail"),
        };
        assert_eq!(e.context.code, "mp3.id3.header_truncated");
    }

    #[test]
    fn a_frame_running_past_the_tag_is_typed() {
        let mut tag = tag_v23("ok");
        // Corrupt the TIT2 size to run past the body.
        let body_start = 10;
        tag[body_start + 4] = 0x7F;
        let e = match parse_id3v2(&tag) {
            Err(e) => e,
            Ok(_) => unreachable!("oversized frame must fail"),
        };
        assert_eq!(e.context.code, "mp3.id3.frame_past_end");
    }
}
