//! ASF GUID constants and the little-endian reader primitives
//! (card 6.ASF-a).
//!
//! ASF objects are addressed by 16-byte GUIDs. Storage layout
//! (byte-verified on the §1.2 fixtures, plan §2.1): `Data1` is a
//! little-endian `DWORD`, `Data2`/`Data3` little-endian `WORD`s, and
//! `Data4` is 8 bytes verbatim — the Header GUID appears on disk as
//! `30 26 b2 75 8e 66 cf 11 a6 d9 00 aa 00 62 ce 6c` and renders as
//! `75B22630-668E-11CF-A6D9-00AA0062CE6C`.
//!
//! The constants table below splits honestly: the GUIDs marked
//! *observed* were byte-verified against ffmpeg 7.1.5 output this
//! session (the plan's §1.2 fixture family); the ones marked
//! `[FORMAT KNOWLEDGE]` are recalled identifiers for objects ffmpeg
//! never writes — they are carried for observability names only and
//! never gate parsing (every unknown GUID is skipped by size, the
//! AVI junk-list discipline).

use std::fmt;

/// A 16-byte ASF object GUID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Guid([u8; 16]);

impl Guid {
    /// Wrap raw GUID bytes (no conversion — the wire form is the
    /// storage form).
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Guid {
        Guid(bytes)
    }

    /// Read the GUID at `offset` (caller bounds-checks; a short
    /// buffer reads as zeros).
    #[must_use]
    pub fn read(data: &[u8], offset: usize) -> Guid {
        let mut bytes = [0u8; 16];
        let end = offset.saturating_add(16).min(data.len());
        if offset < data.len() {
            let got = &data[offset..end];
            let mut i = 0;
            while i < got.len() {
                bytes[i] = got[i];
                i += 1;
            }
        }
        Guid(bytes)
    }

    /// The raw wire bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// The canonical hyphenated uppercase rendering (Data1/Data2/Data3
    /// little-endian, Data4 verbatim) — observability only.
    #[must_use]
    pub fn render(&self) -> String {
        let d1 = u32::from_le_bytes([self.0[0], self.0[1], self.0[2], self.0[3]]);
        let d2 = u16::from_le_bytes([self.0[4], self.0[5]]);
        let d3 = u16::from_le_bytes([self.0[6], self.0[7]]);
        let r = &self.0[8..16];
        format!(
            "{d1:08X}-{d2:04X}-{d3:04X}-{}-{}",
            r[..2].iter().map(|b| format!("{b:02X}")).collect::<String>(),
            r[2..].iter().map(|b| format!("{b:02X}")).collect::<String>()
        )
    }

    /// A human name when the GUID is in the known table, else `None`
    /// (the caller reports it raw — never a guessed name).
    #[must_use]
    pub fn known_name(&self) -> Option<&'static str> {
        match *self {
            HEADER => Some("header"),
            DATA => Some("data"),
            SIMPLE_INDEX => Some("simple-index"),
            FILE_PROPERTIES => Some("file-properties"),
            HEADER_EXTENSION => Some("header-extension"),
            EXTENDED_CONTENT_DESCRIPTION => Some("extended-content-description"),
            CONTENT_DESCRIPTION => Some("content-description"),
            STREAM_PROPERTIES => Some("stream-properties"),
            CODEC_LIST => Some("codec-list"),
            EXTENDED_STREAM_PROPERTIES => Some("extended-stream-properties"),
            STREAM_TYPE_AUDIO => Some("stream-type-audio"),
            STREAM_TYPE_VIDEO => Some("stream-type-video"),
            STREAM_TYPE_COMMAND => Some("stream-type-command"),
            EC_AUDIO_SPREAD => Some("error-correction-audio-spread"),
            EC_NONE => Some("error-correction-none"),
            _ => None,
        }
    }
}

impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

// Top-level objects.
/// Header Object (observed: every fixture's first object).
pub const HEADER: Guid = Guid([
    0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11, 0xA6, 0xD9, 0x00, 0xAA, 0x00, 0x62, 0xCE,
    0x6C,
]);
/// Data Object (observed: follows the Header in every fixture).
pub const DATA: Guid = Guid([
    0x36, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11, 0xA6, 0xD9, 0x00, 0xAA, 0x00, 0x62, 0xCE,
    0x6C,
]);
/// Simple Index Object (observed: video fixtures only, after Data;
/// entries all zero in ffmpeg output — plan §2.3/§4.3).
pub const SIMPLE_INDEX: Guid = Guid([
    0x90, 0x08, 0x00, 0x33, 0xB1, 0xE5, 0xCF, 0x11, 0x89, 0xF4, 0x00, 0xA0, 0xC9, 0x03, 0x49,
    0xCB,
]);

// Header children.
/// File Properties Object (observed).
pub const FILE_PROPERTIES: Guid = Guid([
    0xA1, 0xDC, 0xAB, 0x8C, 0x47, 0xA9, 0xCF, 0x11, 0x8E, 0xE4, 0x00, 0xC0, 0x0C, 0x20, 0x53,
    0x65,
]);
/// Header Extension Object (observed).
pub const HEADER_EXTENSION: Guid = Guid([
    0xB5, 0x03, 0xBF, 0x5F, 0x2E, 0xA9, 0xCF, 0x11, 0x8E, 0xE3, 0x00, 0xC0, 0x0C, 0x20, 0x53,
    0x65,
]);
/// Extended Content Description Object (observed).
pub const EXTENDED_CONTENT_DESCRIPTION: Guid = Guid([
    0x40, 0xA4, 0xD0, 0xD2, 0x07, 0xE3, 0xD2, 0x11, 0x97, 0xF0, 0x00, 0xA0, 0xC9, 0x5E, 0xA8,
    0x50,
]);
/// Content Description Object (`[FORMAT KNOWLEDGE]` — ffmpeg output
/// carries none; recognized for the skip list only).
pub const CONTENT_DESCRIPTION: Guid = Guid([
    0xF0, 0x2E, 0x92, 0x75, 0x8E, 0x66, 0xCF, 0x11, 0xA6, 0xD9, 0x00, 0xAA, 0x00, 0x62, 0xCE,
    0x6C,
]);
/// Stream Properties Object (observed).
pub const STREAM_PROPERTIES: Guid = Guid([
    0x91, 0x07, 0xDC, 0xB7, 0xB7, 0xA9, 0xCF, 0x11, 0x8E, 0xE6, 0x00, 0xC0, 0x0C, 0x20, 0x53,
    0x65,
]);
/// Codec List Object (observed).
pub const CODEC_LIST: Guid = Guid([
    0x40, 0x52, 0xD1, 0x86, 0x1D, 0x31, 0xD0, 0x11, 0xA3, 0xA4, 0x00, 0xA0, 0xC9, 0x03, 0x48,
    0xF6,
]);

// Stream Properties inner GUIDs.
/// Audio stream type (observed in the SP type field).
pub const STREAM_TYPE_AUDIO: Guid = Guid([
    0x40, 0x9E, 0x69, 0xF8, 0x4D, 0x5B, 0xCF, 0x11, 0xA8, 0xFD, 0x00, 0x80, 0x5F, 0x5C, 0x44,
    0x2B,
]);
/// Video stream type (observed in the SP type field).
pub const STREAM_TYPE_VIDEO: Guid = Guid([
    0xC0, 0xEF, 0x19, 0xBC, 0x4D, 0x5B, 0xCF, 0x11, 0xA8, 0xFD, 0x00, 0x80, 0x5F, 0x5C, 0x44,
    0x2B,
]);
/// Command/script stream type (`[FORMAT KNOWLEDGE]` — the plan §4.5
/// command row; recognized so such streams skip as a known family).
pub const STREAM_TYPE_COMMAND: Guid = Guid([
    0xC0, 0xCF, 0xDA, 0x59, 0xE6, 0x59, 0xD0, 0x11, 0xA3, 0xAC, 0x00, 0xA0, 0xC9, 0x03, 0x48,
    0xF6,
]);
/// Audio-spread error correction type (observed on every audio SP).
pub const EC_AUDIO_SPREAD: Guid = Guid([
    0x50, 0xCD, 0xC3, 0xBF, 0x8F, 0x61, 0xCF, 0x11, 0x8B, 0xB2, 0x00, 0xAA, 0xB4, 0xE2, 0x20,
    0xC0,
]);
/// No error correction type (the video SPs' observed type field is
/// the all-zero GUID in ffmpeg output).
pub const EC_NONE: Guid = Guid([0u8; 16]);

// Header Extension children.
/// Extended Stream Properties Object (observed as the only Header
/// Extension child in ffmpeg output; contents capture-and-ignored —
/// plan §4.1 risk register).
pub const EXTENDED_STREAM_PROPERTIES: Guid = Guid([
    0xEA, 0xCB, 0xF8, 0xC5, 0xAF, 0x5B, 0x77, 0x48, 0x84, 0x67, 0xAA, 0x8C, 0x44, 0xFA, 0x4C,
    0xCA,
]);

/// Little-endian `WORD` at `offset` (caller bounds-checks; a short
/// buffer reads as 0 — the typed truncation path fires before this is
/// trusted).
#[must_use]
pub fn u16le(data: &[u8], offset: usize) -> u16 {
    if offset + 2 > data.len() {
        return 0;
    }
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

/// Little-endian `DWORD` at `offset` (same contract as [`u16le`]).
#[must_use]
pub fn u32le(data: &[u8], offset: usize) -> u32 {
    if offset + 4 > data.len() {
        return 0;
    }
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

/// Little-endian `QWORD` at `offset` (same contract as [`u16le`]).
#[must_use]
pub fn u64le(data: &[u8], offset: usize) -> u64 {
    if offset + 8 > data.len() {
        return 0;
    }
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[offset..offset + 8]);
    u64::from_le_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_guid_round_trips_from_wire_bytes() {
        // Plan §2.1's od-verified wire bytes for the Header GUID.
        let wire = [
            0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11, 0xA6, 0xD9, 0x00, 0xAA, 0x00, 0x62,
            0xCE, 0x6C,
        ];
        assert_eq!(Guid::from_bytes(wire), HEADER);
        assert_eq!(HEADER.render(), "75B22630-668E-11CF-A6D9-00AA0062CE6C");
        assert_eq!(HEADER.known_name(), Some("header"));
        assert_eq!(HEADER.to_string(), HEADER.render());
    }

    #[test]
    fn known_table_covers_the_verified_set() {
        let verified = [
            (HEADER, "header"),
            (DATA, "data"),
            (SIMPLE_INDEX, "simple-index"),
            (FILE_PROPERTIES, "file-properties"),
            (HEADER_EXTENSION, "header-extension"),
            (EXTENDED_CONTENT_DESCRIPTION, "extended-content-description"),
            (STREAM_PROPERTIES, "stream-properties"),
            (CODEC_LIST, "codec-list"),
            (STREAM_TYPE_AUDIO, "stream-type-audio"),
            (STREAM_TYPE_VIDEO, "stream-type-video"),
            (EC_AUDIO_SPREAD, "error-correction-audio-spread"),
            (EXTENDED_STREAM_PROPERTIES, "extended-stream-properties"),
        ];
        for (guid, name) in verified {
            assert_eq!(guid.known_name(), Some(name), "{}", guid.render());
        }
        // And a GUID outside the table stays unnamed.
        let unknown = Guid::from_bytes([0xEE; 16]);
        assert_eq!(unknown.known_name(), None);
        assert_eq!(unknown.render(), "EEEEEEEE-EEEE-EEEE-EEEE-EEEEEEEEEEEE");
    }

    #[test]
    fn readers_decode_little_endian_and_tolerate_short_buffers() {
        let data = [0x78, 0x56, 0x34, 0x12, 0xEF, 0xCD, 0xAB, 0x90];
        assert_eq!(u16le(&data, 0), 0x5678);
        assert_eq!(u32le(&data, 0), 0x1234_5678);
        assert_eq!(u64le(&data, 0), 0x90AB_CDEF_1234_5678);
        // Short slices read as zero — callers bound-check first.
        assert_eq!(u16le(&data[..1], 0), 0);
        assert_eq!(u32le(&data[..2], 0), 0);
        assert_eq!(u64le(&data[..4], 0), 0);
        assert_eq!(u16le(&data, 7), 0);
    }
}
