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

use media_types::packet::SharedBytes;
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

impl ChunkHeader {
    /// Parse from the raw 8 little-endian header bytes — the movi
    /// walk's stack-buffer path (card 3.B3: zero heap for the header
    /// read; the `[u8; 8]` lives on the caller's stack).
    pub const fn from_le_bytes(bytes: &[u8; 8]) -> ChunkHeader {
        ChunkHeader {
            fourcc: FourCC::new([bytes[0], bytes[1], bytes[2], bytes[3]]),
            size: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        }
    }
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
    ///
    /// Built on the borrow seam ([`plugin_api::SourceStream::read_into`],
    /// card 3.B3): one exact-sized allocation for the result, zero
    /// intermediate buffers, per-source-call steps bounded by
    /// [`READ_STEP_MAX`].
    pub fn read_exact_or_end(&mut self, len: usize) -> Result<ReadExact, EngineError> {
        let mut out = vec![0u8; len];
        let mut filled = 0;
        while filled < len {
            let step = (len - filled).min(READ_STEP_MAX);
            let got = self.stream.read_into(&mut out[filled..filled + step])?;
            if got == 0 {
                out.truncate(filled);
                return Ok(ReadExact::Ended { partial: out });
            }
            filled += got;
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
    /// [`READ_STEP_MAX`] as everywhere else. One exact-sized allocation
    /// (the borrow seam fills it directly, card 3.B3).
    pub fn read_prefix_at(&mut self, len: usize, to: u64) -> Result<Vec<u8>, EngineError> {
        let mut out = vec![0u8; len];
        let got = self.read_prefix_into(&mut out, to)?;
        out.truncate(got);
        Ok(out)
    }

    /// Read exactly `out.len()` bytes at the current position into
    /// caller memory (card 3.B3 — the borrow twin of
    /// [`read_exact_or_end`](SourceReader::read_exact_or_end) for
    /// callers that treat a short stream as a typed truncation rather
    /// than data).
    ///
    /// # Errors
    /// [`error::truncated`] when the stream ends before `out.len()`
    /// bytes (expected = the position a complete read would reach,
    /// found = the stream length); source errors propagate.
    pub fn read_exact_into(&mut self, out: &mut [u8]) -> Result<(), EngineError> {
        let start = self.position();
        let mut filled = 0;
        while filled < out.len() {
            let step = (out.len() - filled).min(READ_STEP_MAX);
            let got = self.stream.read_into(&mut out[filled..filled + step])?;
            if got == 0 {
                return Err(error::truncated(start + out.len() as u64, self.length));
            }
            filled += got;
        }
        Ok(())
    }

    /// Read **at most** `out.len()` bytes at absolute position `to`
    /// into caller memory (card 3.B3 — the borrow twin of
    /// [`read_prefix_at`], the D-3/D-9 clamp primitive): returns the
    /// count actually obtained. A position past the stream end clamps
    /// to the end (the empty prefix) — "nothing there" is the honest
    /// answer, never an access error. Per-source-call steps stay
    /// bounded by [`READ_STEP_MAX`].
    pub fn read_prefix_into(&mut self, out: &mut [u8], to: u64) -> Result<usize, EngineError> {
        let to = to.min(self.length);
        self.seek_to(to)?;
        let mut filled = 0;
        while filled < out.len() {
            let step = (out.len() - filled).min(READ_STEP_MAX);
            let got = self.stream.read_into(&mut out[filled..filled + step])?;
            if got == 0 {
                break;
            }
            filled += got;
        }
        Ok(filled)
    }

    /// Peek one chunk header at `pos` into caller memory without
    /// moving the read position (card 3.B3 — the borrow twin of
    /// [`peek_chunk_header`]): 8 bytes via the stream's `read_into`
    /// and a positional restore. For buffered local sources both the
    /// seek-away and the seek-back are window arithmetic — no source
    /// I/O beyond the natural refills, no allocation.
    ///
    /// # Errors
    /// [`error::truncated`] when the stream ends before a full header
    /// (expected = `pos + 8`, found = the stream length); source errors
    /// propagate. On any error the position restore is best-effort —
    /// the original error wins.
    pub fn peek_chunk_header_into(
        &mut self,
        pos: u64,
        hdr: &mut [u8; 8],
    ) -> Result<(), EngineError> {
        let saved = self.position();
        self.seek_to(pos)?;
        match self.read_exact_into(hdr) {
            Ok(()) => self.seek_to(saved).map(|_| ()),
            Err(err) => {
                let _ = self.seek_to(saved);
                Err(err)
            }
        }
    }
}

/// Read one chunk header (`fourcc` + `u32le` size) at `pos` without
/// disturbing the source's read position (a true peek: seek, read,
/// seek back). This is what keeps the seek path non-destructive —
/// the landed `Demuxer::seek` contract requires the read position to
/// be untouched, and the linear-scan fallback peeks its way across
/// the `movi` list.
///
/// Card 3.B3: implemented over [`SourceReader::peek_chunk_header_into`]
/// — the 8 header bytes land in a stack buffer (zero heap, zero
/// intermediate copies; for buffered local sources the seek pair is
/// window arithmetic, not I/O).
///
/// # Errors
/// [`error::truncated`] when the stream ends before a full header
/// (expected = `pos + 8`, found = the stream length); source errors
/// propagate. On any error the position restore is best-effort — the
/// original error wins.
pub fn peek_chunk_header(reader: &mut SourceReader, pos: u64) -> Result<ChunkHeader, EngineError> {
    let mut hdr = [0u8; 8];
    reader.peek_chunk_header_into(pos, &mut hdr)?;
    Ok(ChunkHeader::from_le_bytes(&hdr))
}

/// The demuxer's bounded payload slot pool (card 3.B3, design D4 as
/// landed — see the deviation note below).
///
/// # What it is
///
/// A fixed-count pool of exact-size [`SharedBytes`] slots. A payload
/// of `want` bytes reuses a pooled slot of exactly that length when
/// one is idle ([`SharedBytes::get_mut`] — the refcount-1 recycle
/// primitive): the read fills the slot in place and the packet takes a
/// clone — **zero allocation, one copy** per payload. A size with no
/// idle slot takes the spill path (one exact-size allocation, the
/// packet's own buffer) and the fresh slot enters the pool for future
/// same-size payloads. Streams with recurring payload sizes (PCM
/// `##wb` chunks are uniform) reach a zero-allocation steady state;
/// varying sizes (JPEG `##dc`) degrade gracefully to one allocation
/// per payload — never a wait: a busy slot (its clone still queued
/// downstream) is skipped, never blocking the demux thread on
/// consumer progress (that would be a second backpressure channel).
///
/// # Memory bound (D-1)
///
/// The pool itself holds at most `max_slots` buffers of at most
/// `slot_cap` bytes each — `max_slots × slot_cap` bytes, allocated
/// lazily and never exceeded (default 2 × 64 KiB = 128 KiB). Payloads
/// larger than `slot_cap` always spill and are never pooled, so an
/// oversized chunk cannot hog the pool. In-flight (unconsumed)
/// payload bytes are the queues' own byte caps' accountability.
///
/// # Deviation from the W1-19 design (recorded, D-6)
///
/// The design's literal shape — fixed 64 KiB slots handed out as
/// `slot.clone()` — cannot produce exact-length packet data:
/// `SharedBytes` is `Arc<[u8]>` with a fixed length per allocation,
/// so a 300-byte payload in a 64 KiB slot would claim `data.len() ==
/// 65 536` and corrupt every downstream consumer (the queue's byte
/// axis, the decoders' entropy reads). Exact-size slots preserve the
/// design's intent (bounded pool, `get_mut` recycling, spill escape)
/// within the landed `SharedBytes`; the true fixed-slot zero-copy is
/// the design's own D5 `[PROPOSAL]` — a `(Arc, offset, len)` range
/// flavor, media-types' Lead-owned surface, recorded as the follow-up.
#[derive(Debug)]
pub struct PayloadSlots {
    /// The pooled buffers (grows lazily to `max_slots`, each ≤
    /// `slot_cap` bytes).
    slots: Vec<SharedBytes>,
    /// Round-robin cursor for the reuse scan.
    cursor: usize,
    /// Per-slot byte ceiling; larger payloads spill unpooled.
    slot_cap: usize,
    /// The slot-count bound (the D-1 memory bound with `slot_cap`).
    max_slots: usize,
}

impl PayloadSlots {
    /// An empty pool with the given bounds (default shape:
    /// `PayloadSlots::default()` = 2 slots × 64 KiB).
    #[must_use]
    pub fn new(max_slots: usize, slot_cap: usize) -> PayloadSlots {
        PayloadSlots {
            slots: Vec::new(),
            cursor: 0,
            slot_cap,
            max_slots,
        }
    }

    /// The pool's slot-count bound.
    #[must_use]
    pub fn max_slots(&self) -> usize {
        self.max_slots
    }

    /// The pool's current slot population (≤ [`PayloadSlots::max_slots`]).
    #[must_use]
    pub fn resident_slots(&self) -> usize {
        self.slots.len()
    }

    /// Fill one slot with `want` bytes produced by `read` (the caller
    /// supplies the positioned borrow read — e.g. a closure over
    /// [`SourceReader::read_prefix_into`]) and return the packet's
    /// handle. See the type docs for the reuse/spill policy and the
    /// memory bound.
    ///
    /// # Errors
    /// The `read` closure's error, propagated unchanged (the pool
    /// state stays consistent: a failed read leaves no half-filled
    /// slot behind — spill buffers are only committed on success).
    pub fn fill<F>(&mut self, want: usize, read: F) -> Result<SharedBytes, EngineError>
    where
        F: FnOnce(&mut [u8]) -> Result<usize, EngineError>,
    {
        if want == 0 {
            // Nothing deliverable: a static empty buffer, no pool
            // interaction (the walk skips empty payloads anyway).
            return Ok(SharedBytes::from_static(b""));
        }
        if want <= self.slot_cap {
            let len = self.slots.len();
            for i in 0..len {
                let index = (self.cursor + i) % len;
                if self.slots[index].len() == want
                    && let Some(slice) = self.slots[index].get_mut()
                {
                    let got = read(slice)?;
                    self.cursor = (index + 1) % len;
                    if got == want {
                        return Ok(self.slots[index].clone());
                    }
                    // Short (clamped) read: the packet's data must be
                    // exactly what was delivered — spill the prefix
                    // (the rare damage path pays one copy).
                    return Ok(SharedBytes::from_vec(slice[..got].to_vec()));
                }
            }
        }
        // Spill: one exact-size buffer, filled directly. Same-size
        // payloads recycle it through the pool from now on.
        let mut fresh = vec![0u8; want];
        let got = read(&mut fresh)?;
        if got < want {
            fresh.truncate(got);
        }
        let shared = SharedBytes::from_vec(fresh);
        if want <= self.slot_cap {
            self.remember(shared.clone());
        }
        Ok(shared)
    }

    /// Admit a fresh buffer into the pool: append while below the
    /// bound, else recycle the first idle slot (refcount 1); when
    /// every slot is busy downstream the buffer rides with the packet
    /// only (the queue's byte cap bounds it — nothing leaks past the
    /// two budgets).
    fn remember(&mut self, fresh: SharedBytes) {
        if self.slots.len() < self.max_slots {
            self.slots.push(fresh);
            return;
        }
        for slot in &mut self.slots {
            if slot.get_mut().is_some() {
                *slot = fresh;
                return;
            }
        }
    }
}

impl Default for PayloadSlots {
    /// 2 slots × 64 KiB (design D4's `[PROPOSAL]` figure): 128 KiB of
    /// bounded payload memory per demuxer.
    fn default() -> PayloadSlots {
        PayloadSlots::new(2, 64 * 1024)
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

    #[test]
    fn header_parses_from_stack_bytes() {
        let raw = [b'0', b'0', b'd', b'c', 0x39, 0x30, 0x00, 0x00];
        let header = ChunkHeader::from_le_bytes(&raw);
        assert_eq!(header.fourcc, FourCC::tag(b"00dc"));
        assert_eq!(header.size, 12_345);
        // Round-trips with the Vec-flavored peek's parse.
        let mut r = reader(&raw);
        let via_peek = peek_chunk_header(&mut r, 0).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(header, via_peek);
    }

    #[test]
    fn peek_header_into_restores_the_position() {
        let mut r = reader(b"abcdefgh");
        r.seek_to(3).unwrap_or_else(|e| unreachable!("{e}"));
        let mut hdr = [0u8; 8];
        r.peek_chunk_header_into(0, &mut hdr).must();
        assert_eq!(&hdr, b"abcdefgh");
        assert_eq!(r.position(), 3, "the borrow peek restores the position");
        // Past the end keeps the typed truncation shape.
        let mut r = reader(b"abcd");
        let err = match r.peek_chunk_header_into(2, &mut hdr) {
            Ok(()) => unreachable!("peek past end is typed"),
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

    #[test]
    fn read_exact_into_fills_or_errors_typed() {
        let mut r = reader(&[1, 2, 3, 4, 5]);
        let mut out = [0u8; 3];
        r.read_exact_into(&mut out).must();
        assert_eq!(out, [1, 2, 3]);
        assert_eq!(r.position(), 3);
        // More than remains: typed truncation (expected = 3 + 4,
        // found = the stream length).
        let mut rest = [0u8; 4];
        let err = match r.read_exact_into(&mut rest) {
            Ok(()) => unreachable!("short stream must be typed"),
            Err(err) => err,
        };
        assert!(matches!(
            err.kind,
            ErrorKind::Corrupt {
                detail: media_types::error::CorruptDetail::Truncated {
                    expected: 7,
                    found: 5
                }
            }
        ));
    }

    #[test]
    fn read_prefix_into_clamps_like_read_prefix_at() {
        let mut r = reader(&[1, 2, 3, 4, 5]);
        let mut out = [0u8; 10];
        assert_eq!(r.read_prefix_into(&mut out, 3).must(), 2);
        assert_eq!(&out[..2], &[4, 5]);
        // Past the end clamps to the empty prefix, at the end too.
        assert_eq!(r.read_prefix_into(&mut out, 9).must(), 0);
        // Interior position, full ask.
        let mut out = [0u8; 2];
        assert_eq!(r.read_prefix_into(&mut out, 1).must(), 2);
        assert_eq!(out, [2, 3]);
        // Agreement with the Vec twin.
        let vec = r.read_prefix_at(2, 3).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(vec, vec![4, 5]);
    }

    #[test]
    fn read_exact_or_end_agrees_with_the_borrow_twin() {
        let data: Vec<u8> = (0..300u32).map(|i| (i % 251) as u8).collect();
        let mut r = reader(&data);
        let mut borrow = [0u8; 300];
        r.read_exact_into(&mut borrow).must();
        assert_eq!(borrow.to_vec(), data);
        // The Vec twin over a fresh reader returns the same bytes.
        let mut r = reader(&data);
        match r.read_exact_or_end(300).unwrap_or_else(|e| unreachable!("{e}")) {
            ReadExact::Complete(bytes) => assert_eq!(bytes, data),
            other => unreachable!("complete, got {other:?}"),
        }
        // And the Ended shape still reports the partial as data.
        let mut r = reader(&data);
        match r.read_exact_or_end(400).unwrap_or_else(|e| unreachable!("{e}")) {
            ReadExact::Ended { partial } => assert_eq!(partial, data),
            other => unreachable!("ended with the real prefix, got {other:?}"),
        }
    }

    /// Uniform payload sizes recycle through the pool: the third
    /// fill's bytes come from the first fill's allocation (pointer
    /// identity), with zero allocation on the steady path.
    #[test]
    fn payload_slots_recycle_exact_size_buffers() {
        let mut pool = PayloadSlots::new(2, 4096);
        let payload: Vec<u8> = (0..512u32).map(|i| (i % 251) as u8).collect();

        let first = pool
            .fill(512, |out| {
                out.copy_from_slice(&payload);
                Ok(512)
            })
            .must();
        assert_eq!(pool.resident_slots(), 1, "the spill entered the pool");
        assert_eq!(first.as_slice(), payload.as_slice());

        // A live clone keeps the slot busy: the second fill spills a
        // fresh buffer (never blocks, never waits).
        let second = pool
            .fill(512, |out| {
                out.copy_from_slice(&payload);
                Ok(512)
            })
            .must();
        assert_eq!(pool.resident_slots(), 2, "pool grows to the bound");
        assert_ne!(
            first.as_slice().as_ptr(),
            second.as_slice().as_ptr(),
            "a busy slot is never handed out twice"
        );

        // Drop both handles: the slots go idle and recycle in place —
        // the next fills reuse the exact allocations.
        drop(first);
        drop(second);
        let third = pool
            .fill(512, |out| {
                out.copy_from_slice(&payload);
                Ok(512)
            })
            .must();
        let fourth = pool
            .fill(512, |out| {
                out.copy_from_slice(&payload);
                Ok(512)
            })
            .must();
        assert_eq!(third.as_slice(), payload.as_slice());
        assert_eq!(fourth.as_slice(), payload.as_slice());
        assert_eq!(pool.resident_slots(), 2, "the pool never grows past the bound");
        // At most two distinct allocations serve the steady state.
        let mut ptrs = [third.as_slice().as_ptr(), fourth.as_slice().as_ptr()];
        ptrs.sort_unstable();
        let mut distinct = 1;
        for window in ptrs.windows(2) {
            if window[0] != window[1] {
                distinct += 1;
            }
        }
        assert_eq!(distinct, 2, "both pooled slots are live — exactly the two");
    }

    /// Varying sizes spill, enter the pool, and recycle per size; the
    /// population never exceeds the bound; a busy pool degrades to
    /// unpooled spills instead of waiting.
    #[test]
    fn payload_slots_spill_varying_sizes_and_stay_bounded() {
        let mut pool = PayloadSlots::new(2, 4096);
        let byte = |want: usize, fill: u8| {
            move |out: &mut [u8]| {
                out.fill(fill);
                Ok(out.len())
            }
        };

        let a = pool.fill(100, byte(100, 0xA1)).must();
        let b = pool.fill(200, byte(200, 0xB2)).must();
        assert_eq!(pool.resident_slots(), 2);
        // Pool full and both slots busy: a third size spills unpooled.
        let c = pool.fill(300, byte(300, 0xC3)).must();
        assert_eq!(pool.resident_slots(), 2, "no growth past the bound");
        assert_eq!(c.len(), 300);
        assert!(c.as_slice().iter().all(|&b| b == 0xC3));
        drop(a);
        // Now one slot is idle: a fresh size recycles it.
        let d = pool.fill(150, byte(150, 0xD4)).must();
        assert_eq!(pool.resident_slots(), 2);
        assert!(d.as_slice().iter().all(|&b| b == 0xD4));
        drop(b);
        drop(c);
        drop(d);

        // Oversized payloads always spill and never pool.
        let oversized = pool.fill(8192 + 1, byte(8193, 0xE5)).must();
        assert_eq!(oversized.len(), 8193);
        assert_eq!(pool.resident_slots(), 2, "oversized never enters the pool");

        // Empty wants are static, not pooled.
        let empty = pool.fill(0, |_out| Ok(0)).must();
        assert_eq!(empty.len(), 0);
        assert_eq!(pool.resident_slots(), 2);
    }

    /// A short (clamped) read through a pooled slot still delivers
    /// exactly the bytes that existed — the packet's length is the
    /// delivered count, never the slot's.
    #[test]
    fn payload_slots_short_reads_deliver_exact_lengths() {
        let mut pool = PayloadSlots::new(2, 4096);
        // Prime a 256-byte slot.
        let first = pool
            .fill(256, |out| {
                out.fill(0x11);
                Ok(256)
            })
            .must();
        assert_eq!(first.len(), 256);
        drop(first);
        // Reuse it, but the stream only delivers 100 of 256 bytes.
        let clamped = pool
            .fill(256, |out| {
                out[..100].fill(0x22);
                Ok(100)
            })
            .must();
        assert_eq!(clamped.len(), 100, "the delivered count, never the slot's");
        assert!(clamped.as_slice().iter().all(|&b| b == 0x22));
        // The spill path clamps the same way.
        let spilled = pool
            .fill(999, |out| {
                out[..40].fill(0x33);
                Ok(40)
            })
            .must();
        assert_eq!(spilled.len(), 40);
        assert!(spilled.as_slice().iter().all(|&b| b == 0x33));
    }

    /// The pool over a real reader: uniform payloads read through
    /// `read_prefix_into` recycle slot allocations end to end.
    #[test]
    fn payload_slots_over_a_real_reader_recycle() {
        let data: Vec<u8> = (0..4096u32).map(|i| (i % 251) as u8).collect();
        let mut r = reader(&data);
        let mut pool = PayloadSlots::default();

        let mut handles = Vec::new();
        for offset in [0usize, 512, 1024, 1536] {
            let handle = pool
                .fill(512, |out| r.read_prefix_into(out, offset as u64))
                .must();
            assert_eq!(handle.len(), 512);
            assert_eq!(handle.as_slice(), &data[offset..offset + 512]);
            handles.push(handle);
        }
        assert!(pool.resident_slots() <= pool.max_slots());
        // Drop everything: the next uniform fills reuse the pooled
        // allocations (pointer identity against a captured slot).
        let probe = pool
            .fill(512, |out| r.read_prefix_into(out, 2048))
            .must();
        let mut found_recycled = false;
        for offset in [2560usize, 3072] {
            let next = pool
                .fill(512, |out| r.read_prefix_into(out, offset as u64))
                .must();
            if next.as_slice().as_ptr() == probe.as_slice().as_ptr() {
                found_recycled = true;
            }
        }
        assert!(
            found_recycled || pool.resident_slots() < pool.max_slots(),
            "steady state reuses pooled allocations"
        );
    }
}
