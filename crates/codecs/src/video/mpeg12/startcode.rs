//! Start-code layer (ISO/IEC 11172-2 §6.2.2 / 13818-2 §6.2.2,
//! Table 6-1): the `00 00 01` prefix that threads every MPEG-1/2
//! elementary stream, and the zero-copy scanner that walks a byte
//! buffer start code by start code.
//!
//! MPEG-1/2 has no emulation-prevention rule (unlike H.264): a
//! `00 00 01` sequence *is* a start code wherever it appears, so
//! scanning is a flat byte search and units are plain borrowed
//! slices.

/// One start code's identity (the byte after `00 00 01`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartCode {
    /// `0x00` — picture start; a picture header follows.
    PictureStart,
    /// `0x01..=0xAF` — slice number (row) `n` follows.
    Slice(u8),
    /// `0xB2` — user data (skipped by the fact scan, counted).
    UserData,
    /// `0xB3` — sequence header follows.
    SequenceHeader,
    /// `0xB4` — sequence error (broadcast corruption marker).
    SequenceError,
    /// `0xB5` — extension start; the first nibble picks the kind.
    ExtensionStart,
    /// `0xB7` — end of sequence.
    SequenceEnd,
    /// `0xB8` — group of pictures header follows.
    GroupStart,
    /// Reserved (`0xB0`, `0xB1`, `0xB6`) and system-range bytes —
    /// preserved verbatim, never guessed into a semantic bucket.
    Other(u8),
}

impl StartCode {
    /// Classify the code byte (Table 6-1).
    #[must_use]
    pub fn from_byte(byte: u8) -> StartCode {
        match byte {
            0x00 => StartCode::PictureStart,
            0x01..=0xAF => StartCode::Slice(byte),
            0xB2 => StartCode::UserData,
            0xB3 => StartCode::SequenceHeader,
            0xB4 => StartCode::SequenceError,
            0xB5 => StartCode::ExtensionStart,
            0xB7 => StartCode::SequenceEnd,
            0xB8 => StartCode::GroupStart,
            other => StartCode::Other(other),
        }
    }

    /// The raw code byte (inverse of [`StartCode::from_byte`]).
    #[must_use]
    pub fn to_byte(self) -> u8 {
        match self {
            StartCode::PictureStart => 0x00,
            StartCode::Slice(n) => n,
            StartCode::UserData => 0xB2,
            StartCode::SequenceHeader => 0xB3,
            StartCode::SequenceError => 0xB4,
            StartCode::ExtensionStart => 0xB5,
            StartCode::SequenceEnd => 0xB7,
            StartCode::GroupStart => 0xB8,
            StartCode::Other(byte) => byte,
        }
    }
}

/// One scanned unit: the start code and the payload bytes between it
/// and the next start code (slices may run to end-of-buffer).
#[derive(Debug, Clone, Copy)]
pub struct StartUnit<'a> {
    /// The unit's start code.
    pub code: StartCode,
    /// Bytes after the code byte, up to (not including) the next
    /// `00 00 01` prefix or end of buffer.
    pub payload: &'a [u8],
}

/// Find the next `00 00 01` prefix at/after `from`: the index of the
/// `01` byte. `None` when none follows.
fn next_prefix(data: &[u8], from: usize) -> Option<usize> {
    let end = data.len().saturating_sub(2);
    let mut i = from.min(end);
    while i < end {
        if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
            return Some(i + 2);
        }
        i += 1;
    }
    None
}

/// Zero-copy start-code scanner over an elementary-stream buffer.
///
/// Yields every unit in order, including `Other` codes (the caller
/// filters). A buffer with no start codes yields nothing — the same
/// total-input guarantee as the demuxers' fuzz surfaces.
#[derive(Debug, Clone)]
pub struct StartCodeScan<'a> {
    data: &'a [u8],
    /// Bit position of the next candidate prefix's `01` byte
    /// (None = exhausted).
    next: Option<usize>,
}

impl<'a> StartCodeScan<'a> {
    /// Scan `data` from the beginning.
    #[must_use]
    pub fn new(data: &'a [u8]) -> StartCodeScan<'a> {
        StartCodeScan {
            data,
            next: next_prefix(data, 0),
        }
    }

    /// True when the buffer starts with a start-code prefix (probe
    /// signal: elementary streams begin with `00 00 01 B3`).
    #[must_use]
    pub fn starts_with_prefix(data: &[u8]) -> bool {
        data.len() >= 4 && data[0] == 0 && data[1] == 0 && data[2] == 1
    }
}

impl<'a> Iterator for StartCodeScan<'a> {
    type Item = StartUnit<'a>;

    fn next(&mut self) -> Option<StartUnit<'a>> {
        let start = self.next?;
        // A prefix needs its code byte.
        if start + 1 >= self.data.len() {
            self.next = None;
            return None;
        }
        let code = StartCode::from_byte(self.data[start + 1]);
        let body_from = start + 2;
        // Payload runs to the next prefix (or end of buffer). The
        // search starts after this unit's code byte: a following
        // `00 00 01` (or the 4-byte `00 00 00 01` form, whose extra
        // zero the `idx - 2` cut correctly leaves to the next unit)
        // cannot begin earlier.
        let hit = next_prefix(self.data, body_from);
        self.next = hit;
        let body_to = hit.map_or(self.data.len(), |idx| idx - 2);
        Some(StartUnit {
            code,
            payload: &self.data[body_from..body_to],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn units(data: &[u8]) -> Vec<(StartCode, usize)> {
        StartCodeScan::new(data)
            .map(|u| (u.code, u.payload.len()))
            .collect()
    }

    #[test]
    fn classifies_every_table_6_1_bucket() {
        assert_eq!(StartCode::from_byte(0x00), StartCode::PictureStart);
        assert_eq!(StartCode::from_byte(0x01), StartCode::Slice(1));
        assert_eq!(StartCode::from_byte(0xAF), StartCode::Slice(0xAF));
        assert_eq!(StartCode::from_byte(0xB0), StartCode::Other(0xB0));
        assert_eq!(StartCode::from_byte(0xB2), StartCode::UserData);
        assert_eq!(StartCode::from_byte(0xB3), StartCode::SequenceHeader);
        assert_eq!(StartCode::from_byte(0xB4), StartCode::SequenceError);
        assert_eq!(StartCode::from_byte(0xB5), StartCode::ExtensionStart);
        assert_eq!(StartCode::from_byte(0xB6), StartCode::Other(0xB6));
        assert_eq!(StartCode::from_byte(0xB7), StartCode::SequenceEnd);
        assert_eq!(StartCode::from_byte(0xB8), StartCode::GroupStart);
        assert_eq!(StartCode::from_byte(0xFF), StartCode::Other(0xFF));
        for byte in 0..=u8::MAX {
            assert_eq!(StartCode::from_byte(byte).to_byte(), byte);
        }
    }

    #[test]
    fn walks_a_two_unit_stream() {
        // sequence_header payload 4 bytes, then picture_start with 2.
        let data = [0, 0, 1, 0xB3, 1, 2, 3, 4, 0, 0, 1, 0x00, 9, 9];
        assert_eq!(
            units(&data),
            vec![(StartCode::SequenceHeader, 4), (StartCode::PictureStart, 2)]
        );
    }

    #[test]
    fn four_byte_prefixes_and_shared_zeros() {
        // 00 00 00 01 B7: the 4-byte form; scanner sees prefix at
        // index 2 (code B7) then B8 after.
        let data = [0, 0, 0, 1, 0xB7, 0, 0, 1, 0xB8, 5];
        assert_eq!(
            units(&data),
            vec![(StartCode::SequenceEnd, 0), (StartCode::GroupStart, 1)]
        );
    }

    #[test]
    fn garbage_yields_nothing() {
        assert!(units(&[0; 64]).is_empty());
        assert!(units(b"hello world").is_empty());
        assert!(units(&[]).is_empty());
        // Prefix at the very end without a code byte.
        assert!(units(&[1, 2, 3, 0, 0, 1]).is_empty());
    }

    #[test]
    fn prefix_detection() {
        assert!(StartCodeScan::starts_with_prefix(&[0, 0, 1, 0xB3]));
        assert!(!StartCodeScan::starts_with_prefix(&[0, 0, 1]));
        assert!(!StartCodeScan::starts_with_prefix(&[0, 0, 2, 1]));
    }
}
