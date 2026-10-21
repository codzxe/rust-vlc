//! RIFF byte primitives (card 2.B2; doc 03 §4.3/§7 — the AVI writer's
//! own byte layout, `tools/xtask/src/cmd/fixtures.rs`, is the ground
//! truth this parser reads).
//!
//! Two layers:
//!
//! - [`SourceReader`] — the bounded positional reader over a
//!   [`SourceStream`](plugin_api::SourceStream). Every read is
//!   chunk-bounded (`READ_STEP_MAX` per source call), exact-length
//!   reads surface truncation as the caller's typed error, and nothing
//!   trusts a declared size before it has been ceiling-checked by the
//!   caller.
//! - [`FourCC`] / [`ChunkHeader`] — RIFF's little-endian chunk
//!   vocabulary. Chunks are WORD-aligned: a payload of odd length is
//!   followed by one pad byte that is **not** counted in the chunk
//!   size. The demuxer must skip that pad positionally while never
//!   letting it leak into a packet (the fixture AC pins this: odd
//!   JPEG payloads hash-match only when the pad byte is stripped).

use media_types::{EngineError, ErrorKind, PipelineStage};

use crate::error;

/// A RIFF four-character code (chunk ids, list forms, codec handlers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FourCC(pub [u8; 4]);

impl FourCC {
    /// Build from raw bytes (no validation — fourcc space is open data).
    pub const fn new(bytes: [u8; 4]) -> FourCC {
        FourCC(bytes)
    }

    /// Build from a static four-byte tag (`FourCC::tag(b"RIFF")`).
    pub const fn tag(bytes: &[u8; 4]) -> FourCC {
        FourCC(*bytes)
    }

    /// The raw bytes.
    pub const fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }

    /// Render as printable text when all four bytes are ASCII graphic;
    /// otherwise `#XX..` hex (for logs and debug rendering — never for
    /// parsing).
    pub fn render(&self) -> String {
        if self.0.iter().all(|b| (0x20..0x7f).contains(b)) {
            self.0.iter().map(|&b| b as char).collect()
        } else {
            let hex: Vec<String> = self.0.iter().map(|b| format!("{b:02x}")).collect();
            format!("#{}", hex.join(""))
        }
    }
}

/// `RIFF`, the top-level chunk.
pub const RIFF: FourCC = FourCC::tag(b"RIFF");
/// `LIST`, the nesting chunk.
pub const LIST: FourCC = FourCC::tag(b"LIST");
/// `JUNK` — padding chunks, skipped wherever they appear.
pub const JUNK: FourCC = FourCC::tag(b"JUNK");
/// The `AVI ` RIFF form type.
pub const FORM_AVI: FourCC = FourCC::tag(b"AVI ");
/// The `hdrl` list form (headers).
pub const FORM_HDRL: FourCC = FourCC::tag(b"hdrl");
/// The `movi` list form (packet data).
pub const FORM_MOVI: FourCC = FourCC::tag(b"movi");
/// The `strl` list form (one per stream).
pub const FORM_STRL: FourCC = FourCC::tag(b"strl");
/// The `rec ` list form (record groupings inside `movi`).
pub const FORM_REC: FourCC = FourCC::tag(b"rec ");

/// One parsed chunk header: `fourcc` + `u32le` size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkHeader {
    /// The chunk id.
    pub fourcc: FourCC,
    /// The declared payload size (pad byte excluded, per RIFF).
    pub size: u32,
}

/// How a `movi` chunk classified against the track table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoviChunkKind {
    /// A packet chunk: `(stream ordinal, payload bytes follow)`.
    Packet { stream: u32 },
    /// A nested `rec ` list: its payload is itself a chunk sequence.
    RecList,
    /// Something else (`JUNK`, unknown ids, unparseable stream ids):
    /// skip the payload, count damage.
    Other,
}

/// Classify a `movi`-interior chunk id per the AVI 1.0 convention
/// `<stream-number><type>`: `00dc` = stream 0 compressed video, `01wb` =
/// stream 1 waveform audio. v0 recognizes the `dc` (compressed video)
/// and `wb` (audio) type suffixes; `pc` (palette change), `tx` (text)
/// and anything else are skipped as damage `[PROPOSAL: palette-change
/// application is a later polish]`.
pub fn classify_movi_chunk(fourcc: &FourCC) -> MoviChunkKind {
    if *fourcc == LIST {
        // A nested list inside `movi` is a `rec ` record grouping —
        // transparent descent (the v0 scope note in the demuxer docs).
        return MoviChunkKind::RecList;
    }
    let b = fourcc.as_bytes();
    let digits = [b[0], b[1]];
    let suffix = [b[2], b[3]];
    let stream = digits
        .iter()
        .all(|c| c.is_ascii_digit())
        .then(|| {
            let tens = u32::from(digits[0] - b'0');
            let ones = u32::from(digits[1] - b'0');
            tens.checked_mul(10)?.checked_add(ones)
        })
        .flatten();
    match (stream, &suffix) {
        (Some(stream), b"dc") | (Some(stream), b"wb") => MoviChunkKind::Packet { stream },
        _ => MoviChunkKind::Other,
    }
}

/// Read a little-endian `u16` at `data[offset]` (offset bounds-checked
/// by the caller — the reader only slices complete buffers).
pub fn u16le(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

/// Read a little-endian `u32` at `data[offset]`.
pub fn u32le(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

/// Read a little-endian `i32` at `data[offset]` (AVI declares some
/// fields as signed, e.g. `dwQuality = -1`).
pub fn i32le(data: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

/// WORD-alignment pad for a payload of `size` bytes: 1 when odd, 0 when
/// even (RIFF chunks never carry more than one pad byte).
pub const fn pad_for(size: u64) -> u64 {
    size & 1
}

/// Maximum bytes requested per `SourceStream::read` call
/// `[PROPOSAL: 64 KiB — matches input's default buffer; large enough
/// that fixture payloads (≤ ~6 KiB) and index reads are single-shot]`.
pub const READ_STEP_MAX: usize = 64 * 1024;

/// The outcome of a bounded exact read.
#[derive(Debug, PartialEq, Eq)]
pub enum ReadExact {
    /// All requested bytes.
    Complete(Vec<u8>),
    /// The stream ended first; the bytes that existed.
    Ended { partial: Vec<u8> },
}

/// The bounded positional reader over the demuxer's byte source.
///
/// Owns the `Box<dyn SourceStream>` for the demuxer's lifetime. All
/// positioning is absolute (`seek_to`); reads are **bounded per source
/// call** by [`READ_STEP_MAX`] so a single oversized request can never
/// ask the source for an unbounded buffer (MP §12 — the input layer
/// would stream it, but the contract should not depend on that).
///
/// Truncation is *data the caller classifies*: [`read_exact_or_end`]
/// reports how many bytes were actually obtained before end-of-stream,
/// and the demux stage mints the typed `Corrupt::Truncated` with the
/// structure's own expected/found numbers (a container's declared
/// extent is a better "expected" than a raw byte count).
///
/// [`read_exact_or_end`]: SourceReader::read_exact_or_end
pub struct SourceReader {
    stream: Box<dyn plugin_api::SourceStream>,
    /// Total length, required up front: AVI 1.0 needs positional seeks
    /// (index after `movi`, pad skipping) and extent clamping.
    length: u64,
}

impl SourceReader {
    /// Wrap a source stream for demuxing.
    ///
    /// # Errors
    /// A source with no known length or no positional seek cannot carry
    /// an AVI 1.0 file (the format needs random access for `idx1` and
    /// pad skipping) — typed `Unsupported` at stage `Open`, the stage
    /// that owns source-capability truth. An empty stream is likewise
    /// not a container.
    pub fn new(stream: Box<dyn plugin_api::SourceStream>) -> Result<SourceReader, EngineError> {
        let unsupported = |code: &'static str| {
            EngineError::new(
                PipelineStage::Open,
                ErrorKind::Unsupported { format: None },
                media_types::error::ErrorContext::new(code),
            )
        };
        let Some(length) = stream.size() else {
            return Err(unsupported("demux.source.unknown-length"));
        };
        if !stream.supports_seek() {
            return Err(unsupported("demux.source.unseekable"));
        }
        if length == 0 {
            return Err(unsupported("demux.source.empty"));
        }
        Ok(SourceReader { stream, length })
    }

    /// The stream's total length (known by construction).
    pub fn length(&self) -> u64 {
        self.length
    }

    /// Current absolute position.
    pub fn position(&self) -> u64 {
        self.stream.position().0
    }

    /// Reposition absolutely. Errors are the source's own (stage `Open`).
    pub fn seek_to(&mut self, to: u64) -> Result<(), EngineError> {
        self.stream
            .seek(plugin_api::SeekOrigin::Start(to))
            .map(|_| ())
    }

    /// Read exactly `len` bytes at the current position.
    ///
    /// On end-of-stream before `len` bytes: the position is left at the
    /// end and the partial bytes are reported; the caller mints the
    /// typed truncation error with the structure's numbers. A mid-read
    /// source error propagates unchanged (the access layer's taxonomy).
    pub fn read_exact_or_end(&mut self, len: usize) -> Result<ReadExact, EngineError> {
        let mut out = Vec::with_capacity(len.min(READ_STEP_MAX));
        while out.len() < len {
            let step = (len - out.len()).min(READ_STEP_MAX);
            let got = self.stream.read(step)?;
            if got.is_empty() {
                return Ok(ReadExact::Ended { partial: out });
            }
            out.extend_from_slice(&got);
        }
        Ok(ReadExact::Complete(out))
    }

    /// Read exactly `len` bytes at absolute position `to` (the
    /// non-destructive peek the walks and the seek fallback need).
    /// `Ok(None)` = the stream ended before `len` bytes.
    pub fn read_exact_or_end_at(
        &mut self,
        len: usize,
        to: u64,
    ) -> Result<Option<Vec<u8>>, EngineError> {
        self.seek_to(to)?;
        match self.read_exact_or_end(len)? {
            ReadExact::Complete(bytes) => Ok(Some(bytes)),
            ReadExact::Ended { .. } => Ok(None),
        }
    }

    /// Read **at most** `len` bytes at absolute position `to`, returning
    /// whatever the stream actually carries there (possibly fewer, never
    /// more, possibly empty when `to` is at or past the end).
    ///
    /// This is the dirty-world clamp primitive (D-3/D-9): a chunk whose
    /// declared size runs past the stream still delivers its real
    /// available prefix instead of discarding it — the caller counts the
    /// difference as damage. Reads are bounded per source call by
    /// [`READ_STEP_MAX`] as everywhere else.
    pub fn read_prefix_at(&mut self, len: usize, to: u64) -> Result<Vec<u8>, EngineError> {
        // A position past the stream end clamps to the end (the empty
        // prefix) — sources may refuse seeks past EOF, and "nothing
        // there" is the honest answer, never an access error.
        let to = to.min(self.length);
        self.seek_to(to)?;
        Ok(match self.read_exact_or_end(len)? {
            ReadExact::Complete(bytes) => bytes,
            ReadExact::Ended { partial } => partial,
        })
    }
}

/// Read one chunk header (`fourcc` + `u32le` size) at `pos` without
/// disturbing the source's read position (a true peek: seek, read,
/// seek back). This is what keeps the seek path non-destructive —
/// the landed `Demuxer::seek` contract requires the read position to
/// be untouched, and the linear-scan fallback peeks its way across
/// the `movi` list.
///
/// # Errors
/// [`error::truncated`] when the stream ends before a full header
/// (expected = `pos + 8`, found = the stream length); source errors
/// propagate. On any error the position restore is best-effort — the
/// original error wins.
pub fn peek_chunk_header(reader: &mut SourceReader, pos: u64) -> Result<ChunkHeader, EngineError> {
    let saved = reader.position();
    let outcome = reader.read_exact_or_end_at(8, pos);
    match outcome {
        Ok(Some(bytes)) => {
            reader.seek_to(saved)?;
            Ok(ChunkHeader {
                fourcc: FourCC::new([bytes[0], bytes[1], bytes[2], bytes[3]]),
                size: u32le(&bytes, 4),
            })
        }
        Ok(None) => {
            let _ = reader.seek_to(saved);
            Err(error::truncated(pos + 8, reader.length()))
        }
        Err(err) => {
            let _ = reader.seek_to(saved);
            Err(err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::ids::SourceUri;

    /// In-memory seekable source (the plugin-api test pattern, trimmed
    /// to what the reader needs).
    struct MemStream {
        uri: SourceUri,
        data: Vec<u8>,
        pos: usize,
    }

    impl MemStream {
        fn new(data: &[u8]) -> MemStream {
            MemStream {
                uri: SourceUri::parse("memory://test").unwrap_or_else(|e| unreachable!("{e}")),
                data: data.to_vec(),
                pos: 0,
            }
        }
    }

    impl plugin_api::SourceStream for MemStream {
        fn uri(&self) -> &SourceUri {
            &self.uri
        }
        fn size(&self) -> Option<u64> {
            Some(self.data.len() as u64)
        }
        fn supports_seek(&self) -> bool {
            true
        }
        fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
            let end = (self.pos + len).min(self.data.len());
            let out = self.data[self.pos..end].to_vec();
            self.pos = end;
            Ok(out)
        }
        fn seek(
            &mut self,
            origin: plugin_api::SeekOrigin,
        ) -> Result<plugin_api::SourcePosition, EngineError> {
            let target = match origin {
                plugin_api::SeekOrigin::Start(to) => to,
                plugin_api::SeekOrigin::Current(delta) => {
                    let cur = self.pos as i64;
                    match cur.checked_add(delta) {
                        Some(n) if n >= 0 => n as u64,
                        _ => u64::MAX,
                    }
                }
            };
            if target > self.data.len() as u64 {
                return Err(EngineError::new(
                    PipelineStage::Open,
                    ErrorKind::Inaccessible {
                        reason: media_types::error::AccessDenied::NotFound,
                    },
                    media_types::error::ErrorContext::new("test.mem.seek-out-of-range"),
                ));
            }
            self.pos = target as usize;
            Ok(plugin_api::SourcePosition(target))
        }
        fn position(&self) -> plugin_api::SourcePosition {
            plugin_api::SourcePosition(self.pos as u64)
        }
    }

    fn reader(data: &[u8]) -> SourceReader {
        SourceReader::new(Box::new(MemStream::new(data))).unwrap_or_else(|e| unreachable!("{e}"))
    }

    #[test]
    fn fourcc_render_printable_and_hex() {
        assert_eq!(FourCC::tag(b"00dc").render(), "00dc");
        assert_eq!(FourCC::new([0x00, 0x01, 0x02, 0x7f]).render(), "#0001027f");
    }

    #[test]
    fn little_endian_readers() {
        let data = [0x78, 0x56, 0x34, 0x12, 0xff, 0xff, 0xff, 0xff];
        assert_eq!(u16le(&data, 0), 0x5678);
        assert_eq!(u32le(&data, 0), 0x1234_5678);
        assert_eq!(i32le(&data, 4), -1);
    }

    #[test]
    fn pad_is_one_only_for_odd_payloads() {
        assert_eq!(pad_for(0), 0);
        assert_eq!(pad_for(1), 1);
        assert_eq!(pad_for(1193), 1);
        assert_eq!(pad_for(5880), 0);
    }

    #[test]
    fn movi_chunk_classification() {
        assert_eq!(
            classify_movi_chunk(&FourCC::tag(b"00dc")),
            MoviChunkKind::Packet { stream: 0 }
        );
        assert_eq!(
            classify_movi_chunk(&FourCC::tag(b"01wb")),
            MoviChunkKind::Packet { stream: 1 }
        );
        assert_eq!(
            classify_movi_chunk(&FourCC::tag(b"10dc")),
            MoviChunkKind::Packet { stream: 10 }
        );
        assert_eq!(
            classify_movi_chunk(&FourCC::tag(b"JUNK")),
            MoviChunkKind::Other
        );
        assert_eq!(
            classify_movi_chunk(&FourCC::tag(b"00pc")),
            MoviChunkKind::Other
        );
        assert_eq!(
            classify_movi_chunk(&FourCC::tag(b"##dc")),
            MoviChunkKind::Other
        );
        assert_eq!(
            classify_movi_chunk(&FourCC::tag(b"LIST")),
            MoviChunkKind::RecList
        );
    }

    #[test]
    fn read_exact_and_end_detection() {
        let mut r = reader(&[1, 2, 3, 4, 5]);
        let got = r
            .read_exact_or_end(4)
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(got, ReadExact::Complete(vec![1, 2, 3, 4]));
        // The remaining single byte then end-of-stream.
        let got = r
            .read_exact_or_end(4)
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(got, ReadExact::Ended { partial: vec![5] });
    }

    #[test]
    fn read_prefix_at_clamps_to_the_stream() {
        let mut r = reader(&[1, 2, 3, 4, 5]);
        // Full ask at an interior position.
        let got = r
            .read_prefix_at(2, 1)
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(got, vec![2, 3]);
        // Ask running past the end: the real prefix, never padded.
        let got = r
            .read_prefix_at(10, 3)
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(got, vec![4, 5]);
        // At the end, and past it: empty, not an error (the caller
        // classifies — the dirty-world clamp primitive).
        let got = r
            .read_prefix_at(4, 5)
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert!(got.is_empty());
        let got = r
            .read_prefix_at(4, 9)
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert!(got.is_empty());
    }

    #[test]
    fn peek_header_does_not_move_the_position() {
        let mut r = reader(b"abcdefgh");
        r.seek_to(3).unwrap_or_else(|e| unreachable!("{e}"));
        let header = peek_chunk_header(&mut r, 0).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(header.fourcc, FourCC::tag(b"abcd"));
        assert_eq!(header.size, 0x6867_6665);
        assert_eq!(r.position(), 3, "peek must restore the position");
    }

    #[test]
    fn peek_header_past_end_is_typed_truncation() {
        let mut r = reader(b"abcd");
        let err = match peek_chunk_header(&mut r, 2) {
            Ok(_) => unreachable!("peek past end is typed"),
            Err(err) => err,
        };
        assert!(matches!(
            err.kind,
            ErrorKind::Corrupt {
                detail: media_types::error::CorruptDetail::Truncated {
                    expected: 10,
                    found: 4
                }
            }
        ));
    }
}
