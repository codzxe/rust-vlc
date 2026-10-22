//! The variant-B frame-streaming wire protocol, v1 (card 2.D3; doc 08
//! §7 candidate B; ADR-003 decision 2: "frame-streaming B is a
//! transport of the same presented frames").
//!
//! # What this protocol is
//!
//! A transport-agnostic **record framing** for the RGBA frames the
//! engine's presentation scheduler has decided to put on screen: one
//! self-describing little-endian record per frame, concatenated into a
//! byte stream. The same bytes flow over a Tauri IPC channel
//! (`[PROPOSAL: TauriChannelTransport`, `transport.rs`]`), a
//! WebSocket, or a `SharedArrayBuffer` ring — the decoder never knows.
//! The reference canvas-side implementation is
//! `apps/desktop/prototype/lumen-frames.mjs` (byte-for-byte the same
//! state machine, cross-language conformance via
//! `apps/desktop/prototype/harness.mjs`).
//!
//! # Record layout (v1, all integers little-endian)
//!
//! | off | len | field          | notes                                  |
//! |----:|----:|----------------|----------------------------------------|
//! |   0 |   4 | magic          | `"LUF1"` (0x4C 0x55 0x46 0x31)         |
//! |   4 |   2 | header_len     | 48..=1024; bytes 48..header_len are    |
//! |     |     |                | v1-ignored extension space             |
//! |   6 |   2 | version        | 1                                      |
//! |   8 |   1 | format code    | 1 = Rgba8, 2 = Bgra8                   |
//! |   9 |   1 | flags          | bit 0 = repeat; bits 1..=7 must be 0   |
//! |  10 |   2 | reserved       | must be zero                           |
//! |  12 |   4 | width          | non-zero                               |
//! |  16 |   4 | height         | non-zero                               |
//! |  20 |   4 | payload_len    | must equal width·height·4              |
//! |  24 |   8 | frame_id       | monotonic per stream                   |
//! |  32 |   8 | pts_ns         | scheduled slot, media-clock ns base    |
//! |  40 |   8 | emitted_ns     | serialize instant on the bridge's      |
//! |     |     |                | monotonic emit clock (latency origin)  |
//! |  48 | …   | payload        | width·height·4 bytes, row-major        |
//!
//! Header overhead at 1080p (1920·1080·4 = 8 294 400 B) is 48 B ≈
//! 0.000 58 % of the record; at 4K it is 0.000 14 % — framing is not
//! a latency factor (the measured serialization cost is recorded in
//! `docs/prototypes/video-in-webview.md` §4, never asserted here).
//!
//! # Honesty rules baked into the codec
//!
//! - **The encoder refuses what it cannot express** (zero dims, YUV
//!   formats, payload/dimension disagreement) — a malformed record is
//!   never put on the wire.
//! - **The decoder is strict**: unknown magic/version/format, reserved
//!   bits set, or geometry that cannot describe a real frame are
//!   typed [`WireError`]s, never guesses. There is no resync: a
//!   corrupted stream is dropped at the host and re-established (the
//!   generation purge in `mailbox.rs` is the engine-side twin).
//! - **`pts_ns` vs `emitted_ns` live on different clocks by design**:
//!   the slot is media time (A/V-sync accounting), the emit stamp is
//!   the bridge's monotonic clock (transport-latency accounting).
//!   Cross-boundary end-to-end latency additionally needs an origin
//!   handshake between the engine clock and the webview's
//!   `performance.now()` — `[PROPOSAL]` protocol v2; the prototype
//!   measures each half separately and the harness (single virtual
//!   clock) measures the whole.

use std::sync::{Arc, Mutex};

use media_types::PixelFormat;

use crate::error::WireError;

/// The v1 stream magic: `"LUF1"` (Lumen Frame, version 1 in the magic
/// itself so a mis-fed text stream fails the first four bytes).
pub const MAGIC: [u8; 4] = [0x4C, 0x55, 0x46, 0x31];

/// The protocol version this module speaks.
pub const VERSION: u16 = 1;

/// The length of the fixed v1 header (the layout table above).
pub const MIN_HEADER_LEN: usize = 48;

/// The largest accepted `header_len` (forward-compatibility window:
/// records may carry extension bytes this decoder ignores).
pub const MAX_HEADER_LEN: usize = 1024;

/// The payload cap: 1 GiB. A frame larger than this is not a frame a
/// canvas will ever draw (8K RGBA is 132 MB); bigger means corruption.
pub const MAX_PAYLOAD_LEN: u64 = 1 << 30;

/// The largest accepted dimension (width and height): 65 535 — the
/// u32 payload arithmetic stays in safe territory and no real display
/// exceeds it.
pub const MAX_DIMENSION: u32 = 65_535;

/// Per-record flags (header byte 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameFlags {
    /// The record re-presents the previous slot (the scheduler's
    /// bounded repeat — canvas compositors must honor it; see the
    /// [`Presenter`](video_engine::Presenter) contract).
    pub repeat: bool,
}

impl FrameFlags {
    /// Encode to the wire byte (bit 0).
    #[must_use = "pure serialization"]
    pub fn to_bits(self) -> u8 {
        u8::from(self.repeat)
    }

    /// Decode from the wire byte; any set bit outside bit 0 is
    /// [`WireError::ReservedNotZero`].
    ///
    /// # Errors
    /// [`WireError::ReservedNotZero`] when bits 1..=7 are set.
    pub fn from_bits(bits: u8) -> Result<FrameFlags, WireError> {
        if bits & 0xFE != 0 {
            return Err(WireError::ReservedNotZero { field: "flags" });
        }
        Ok(FrameFlags {
            repeat: bits & 0x01 != 0,
        })
    }
}

/// The v1 pixel-format code table (exactly what renderer-wgpu's
/// upload path accepts: RGBA8/Bgra8 — the YUV family is a typed
/// refusal naming the GPU shader path, `uploads_format`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatCode {
    /// 8-bit RGBA, straight alpha (code 1).
    Rgba8,
    /// 8-bit BGRA (code 2).
    Bgra8,
}

impl FormatCode {
    /// The wire code for this format.
    #[must_use = "pure serialization"]
    pub fn code(self) -> u8 {
        match self {
            FormatCode::Rgba8 => 1,
            FormatCode::Bgra8 => 2,
        }
    }

    /// Decode a wire code.
    ///
    /// # Errors
    /// [`WireError::UnknownFormatCode`] for anything outside the v1
    /// table.
    pub fn from_code(code: u8) -> Result<FormatCode, WireError> {
        match code {
            1 => Ok(FormatCode::Rgba8),
            2 => Ok(FormatCode::Bgra8),
            found => Err(WireError::UnknownFormatCode { found }),
        }
    }

    /// The media-types pixel format (the frame vocabulary the engine
    /// speaks).
    #[must_use = "pure mapping"]
    pub fn pixel_format(self) -> PixelFormat {
        match self {
            FormatCode::Rgba8 => PixelFormat::Rgba8,
            FormatCode::Bgra8 => PixelFormat::Bgra8,
        }
    }

    /// The reverse mapping; `None` for the YUV family (the encoder
    /// refuses those with the shader path named — the caller maps
    /// that to its typed error).
    #[must_use = "pure mapping"]
    pub fn for_pixel_format(format: PixelFormat) -> Option<FormatCode> {
        match format {
            PixelFormat::Rgba8 => Some(FormatCode::Rgba8),
            PixelFormat::Bgra8 => Some(FormatCode::Bgra8),
            PixelFormat::Nv12 | PixelFormat::I420 | PixelFormat::Yuyv422 => None,
        }
    }
}

/// One framed record's header (the layout table above; the payload
/// follows separately so large frames never need header copies).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
    /// Pixel format + wire code in one.
    pub format: FormatCode,
    /// Per-record flags.
    pub flags: FrameFlags,
    /// Frame width in pixels (1..=65 535).
    pub width: u32,
    /// Frame height in pixels (1..=65 535).
    pub height: u32,
    /// Payload length in bytes (width·height·4).
    pub payload_len: u32,
    /// Monotonic per-stream frame id (wraps only after ~584 years at
    /// 60 fps; a wrap means stream restart, which the id consumer
    /// treats as a new epoch).
    pub frame_id: u64,
    /// The scheduled presentation slot, nanoseconds on the media
    /// clock's base (may be negative — clocks start where they
    /// start).
    pub pts_ns: i64,
    /// The serialize instant on the bridge's monotonic emit clock —
    /// the transport-latency accounting origin.
    pub emitted_ns: u64,
}

impl FrameHeader {
    /// Validate a header's geometry fields (shared by encode and
    /// decode paths — one rule set, no drift).
    fn validate_geometry(width: u32, height: u32, payload_len: u32) -> Result<u64, WireError> {
        if width == 0 || height == 0 {
            return Err(WireError::BadGeometry {
                reason: "zero dimension",
            });
        }
        if width > MAX_DIMENSION || height > MAX_DIMENSION {
            return Err(WireError::BadGeometry {
                reason: "dimension exceeds 65535",
            });
        }
        let implied = u64::from(width) * u64::from(height) * 4;
        if implied > MAX_PAYLOAD_LEN {
            return Err(WireError::BadGeometry {
                reason: "payload exceeds the 1 GiB cap",
            });
        }
        if u64::from(payload_len) != implied {
            return Err(WireError::PayloadLengthMismatch {
                declared: payload_len,
                implied,
            });
        }
        Ok(implied)
    }

    /// Write the fixed 48-byte header to `out`.
    fn write_to(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&(MIN_HEADER_LEN as u16).to_le_bytes());
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.push(self.format.code());
        out.push(self.flags.to_bits());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&self.width.to_le_bytes());
        out.extend_from_slice(&self.height.to_le_bytes());
        out.extend_from_slice(&self.payload_len.to_le_bytes());
        out.extend_from_slice(&self.frame_id.to_le_bytes());
        out.extend_from_slice(&self.pts_ns.to_le_bytes());
        out.extend_from_slice(&self.emitted_ns.to_le_bytes());
    }
}

/// How many payload buffers the decode recycle pool keeps (D-1:
/// bounded). Three covers the frames currently queued in a
/// default-capacity mailbox plus one handed to the consumer — the
/// renderer-wgpu staging C5a precedent; a wider queue simply keeps
/// its extra frames as plain owners (bounded by the queue's own
/// capacity, never by the pool).
const RECYCLE_SLOTS: usize = 3;

/// The bounded payload-buffer recycle pool shared between a
/// [`WireDecoder`] and the [`DecodedFrame`]s it handed out: a frame
/// dropping returns its `payload` Vec to the pool; the next decode
/// pops one big enough and refills it in place (the ED-FIX-2
/// renderer-wgpu `RecyclePool` pattern applied to the wire decode
/// path — the C7 fix's answer to the per-frame `to_vec`). Memory is
/// bounded at `RECYCLE_SLOTS · max-frame-bytes`; a frame nobody
/// drops simply never returns (correct, never leaks into unbounded
/// growth).
#[derive(Debug)]
struct RecyclePool {
    slots: Mutex<Vec<Vec<u8>>>,
}

impl RecyclePool {
    fn new() -> RecyclePool {
        RecyclePool {
            slots: Mutex::new(Vec::new()),
        }
    }

    /// Take a buffer for a `needed`-byte fill: prefer one whose
    /// capacity already covers `needed` (pure reuse, no realloc);
    /// else any buffer (it grows once — the geometry-change path).
    /// Allocates only when the pool is empty (the warm-up path).
    fn acquire(&self, needed: usize) -> Vec<u8> {
        let mut slots = match self.slots.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        // Best fit: smallest capacity that covers `needed` (keeps the
        // big buffers big-frame-sized).
        let mut best: Option<usize> = None;
        for (i, buf) in slots.iter().enumerate() {
            if buf.capacity() >= needed && best.is_none_or(|b| buf.capacity() < slots[b].capacity())
            {
                best = Some(i);
            }
        }
        match best {
            Some(i) => slots.swap_remove(i),
            None => slots.pop().unwrap_or_default(),
        }
    }

    /// Return a buffer (bounded: never grows past `RECYCLE_SLOTS`).
    fn release(&self, buffer: Vec<u8>) {
        let mut slots = match self.slots.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if slots.len() < RECYCLE_SLOTS {
            slots.push(buffer);
        }
        // A full pool drops the returnee: bounded memory wins over
        // retention (the next acquire allocates — cold path only).
    }
}

/// One fully decoded record: header + owned payload (the decoder's
/// output; the encoder works from the engine's frame types).
///
/// The `payload` buffer is **pooled**: frames the decoder produced
/// return their buffer to the owning [`WireDecoder`]'s bounded
/// recycle pool on drop, so the steady-state decode path allocates
/// nothing (D-1). Frames constructed by hand (tests) carry no pool
/// and behave like plain owners.
#[derive(Clone)]
pub struct DecodedFrame {
    /// The record header.
    pub header: FrameHeader,
    /// The payload bytes (`header.payload_len` of them, row-major).
    pub payload: Vec<u8>,
    /// The buffer's return address (absent for hand-built frames).
    recycle: Option<Arc<RecyclePool>>,
}

impl Drop for DecodedFrame {
    fn drop(&mut self) {
        if let Some(pool) = self.recycle.take() {
            // Move the buffer out so Drop of the Vec never runs on a
            // pooled buffer (replace with the cheapest empty Vec).
            pool.release(std::mem::take(&mut self.payload));
        }
    }
}

impl DecodedFrame {
    /// A hand-built frame with no pool attachment (in-crate test
    /// builders): behaves like a plain owner — dropping it frees the
    /// buffer instead of recycling it.
    #[cfg(test)]
    pub(crate) fn detached(header: FrameHeader, payload: Vec<u8>) -> DecodedFrame {
        DecodedFrame {
            header,
            payload,
            recycle: None,
        }
    }
}

impl std::fmt::Debug for DecodedFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Same rendering the old derive produced (the pool handle is
        // machinery, not frame state).
        f.debug_struct("DecodedFrame")
            .field("header", &self.header)
            .field("payload", &self.payload)
            .finish()
    }
}

impl PartialEq for DecodedFrame {
    fn eq(&self, other: &DecodedFrame) -> bool {
        // Frame equality is header + payload equality; the pool handle
        // is machinery (matches the old derive's observable behavior).
        self.header == other.header && self.payload == other.payload
    }
}

impl Eq for DecodedFrame {}

/// Encode one record (header + payload) into a fresh `Vec`.
///
/// # Errors
/// [`WireError`] when the geometry is unexpressible or the payload
/// length disagrees — the encoder never ships a malformed record.
#[must_use = "allocating serialization; prefer encode_into to reuse buffers"]
pub fn encode(header: &FrameHeader, payload: &[u8]) -> Result<Vec<u8>, WireError> {
    let mut out = Vec::with_capacity(MIN_HEADER_LEN + payload.len());
    encode_into(header, payload, &mut out)?;
    Ok(out)
}

/// Encode one record into `out` (buffer reuse for the hot path — the
/// transport owns its buffer, the encoder appends).
///
/// # Errors
/// [`WireError`] when the geometry is unexpressible or the payload
/// length disagrees.
pub fn encode_into(
    header: &FrameHeader,
    payload: &[u8],
    out: &mut Vec<u8>,
) -> Result<(), WireError> {
    FrameHeader::validate_geometry(header.width, header.height, header.payload_len)?;
    if payload.len() != header.payload_len as usize {
        return Err(WireError::PayloadLengthMismatch {
            declared: header.payload_len,
            implied: payload.len() as u64,
        });
    }
    header.write_to(out);
    out.extend_from_slice(payload);
    Ok(())
}

/// The incremental record decoder: feed arbitrary chunks (IPC
/// messages, WebSocket frames, ring segments — chunk boundaries are
/// irrelevant by construction), pull complete records out.
///
/// A decode error is *sticky*: after a malformed record the decoder
/// holds its position and the host drops the stream (no resync —
/// re-establish instead, the generation purge is the engine-side
/// twin).
///
/// # Zero-allocation discipline (the C7 fix)
///
/// The decoder is built **once per stream and reused** (the mailbox
/// transport owns a single instance for its whole life). The pending
/// buffer is a cursor-positioned ring: consumed records advance the
/// cursor (`next_frame` never shifts bytes — the O(n) `drain` is
/// gone), a fully drained buffer resets in place with its capacity
/// retained, and `feed` compacts the live tail to the front only when
/// a partial record spans feeds. Payload buffers come from the
/// bounded [`RecyclePool`] and cycle through the handed-out frames —
/// after one warm-up record, the steady-state path allocates nothing
/// (proven by `tests/zero_alloc_transport.rs` under the counting
/// allocator).
#[derive(Debug, Clone)]
pub struct WireDecoder {
    /// Bytes not yet consumed (partial header or partial record);
    /// `pos` is the read cursor (bytes before it are consumed).
    pending: Vec<u8>,
    /// Read cursor into `pending` (see the field docs).
    pos: usize,
    /// The payload-buffer recycle pool shared with the frames this
    /// decoder hands out (bounded at `RECYCLE_SLOTS`; clones of the
    /// decoder share it — one pool per stream, not per frame).
    pool: Arc<RecyclePool>,
    /// Total bytes ever fed (diagnostics + stream offsets).
    fed: u64,
    /// Total bytes consumed into complete records (diagnostics).
    consumed: u64,
}

impl Default for WireDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl WireDecoder {
    /// A fresh decoder (empty buffer, zero counters).
    #[must_use = "pure constructor"]
    pub fn new() -> WireDecoder {
        WireDecoder {
            pending: Vec::new(),
            pos: 0,
            pool: Arc::new(RecyclePool::new()),
            fed: 0,
            consumed: 0,
        }
    }

    /// Feed one chunk of the stream (any length, any alignment).
    pub fn feed(&mut self, chunk: &[u8]) {
        if self.pos > 0 {
            // Compact the consumed prefix away before extending
            // (bounded memory: the ring never carries dead bytes
            // across feeds). O(live tail) — and zero on the common
            // fully-drained path, where `next_frame` already reset
            // the buffer in place.
            let pos = self.pos;
            self.pending.copy_within(pos.., 0);
            self.pending.truncate(self.pending.len() - pos);
            self.pos = 0;
        }
        self.pending.extend_from_slice(chunk);
        self.fed += chunk.len() as u64;
    }

    /// Total bytes fed so far (stream-offset diagnostics).
    #[must_use = "observable evidence"]
    pub fn bytes_fed(&self) -> u64 {
        self.fed
    }

    /// Total bytes consumed into complete records (with
    /// [`Self::bytes_fed`], the in-flight backlog size).
    #[must_use = "observable evidence"]
    pub fn bytes_consumed(&self) -> u64 {
        self.consumed
    }

    /// Try to decode the next complete record.
    ///
    /// The payload moves out of the ring in **one copy** into a
    /// pooled buffer (ownership transfer — the frame's consumer owns
    /// its bytes; the buffer returns to this decoder's pool on drop).
    /// The old fresh `to_vec` + O(n) `drain(..total)` per frame is the
    /// C7 violation this replaces.
    ///
    /// # Errors
    /// [`WireError`] for malformed records (sticky — see the type
    /// docs).
    /// Returns `Ok(None)` when more input is needed.
    pub fn next_frame(&mut self) -> Result<Option<DecodedFrame>, WireError> {
        let remaining = self.pending.len() - self.pos;
        if remaining < MIN_HEADER_LEN {
            return Ok(None);
        }
        let (header, header_len) =
            Self::parse_header(&self.pending[self.pos..]).map_err(|mut err| {
                // Stream-offset honesty: the record under the cursor
                // starts at `consumed` in fed-stream coordinates.
                if let WireError::BadMagic { offset, .. } = &mut err {
                    *offset = self.consumed;
                }
                err
            })?;
        let total = header_len + header.payload_len as usize;
        if remaining < total {
            return Ok(None);
        }
        let start = self.pos;
        let mut payload = self.pool.acquire(header.payload_len as usize);
        payload.clear();
        payload.extend_from_slice(&self.pending[start + header_len..start + total]);
        // Cursor advancement, never a byte shift.
        self.pos = start + total;
        self.consumed += total as u64;
        if self.pos == self.pending.len() {
            // Fully drained: reset in place (capacity retained — the
            // next feed of the same size class does not allocate).
            self.pending.clear();
            self.pos = 0;
        }
        Ok(Some(DecodedFrame {
            header,
            payload,
            recycle: Some(Arc::clone(&self.pool)),
        }))
    }

    /// Decode exactly ONE complete record, direct from the slice (the
    /// in-process hand-off path — the C7 fix): the header is parsed
    /// through a slice window and the payload moves into a pooled
    /// buffer in one copy, with **no** pending-buffer traffic and no
    /// per-call decoder state. This is what a byte producer with one
    /// record per message calls (the mailbox transport's `send`);
    /// chunked stream consumers use [`Self::feed`] +
    /// [`Self::next_frame`].
    ///
    /// `record` must hold exactly one record: fewer bytes is
    /// [`WireError::Truncated`]; leftover bytes after the complete
    /// record are [`WireError::TrailingBytes`] (a host bug — one
    /// record per hand-off is the producer's contract).
    ///
    /// # Errors
    /// [`WireError`] for anything that is not exactly one
    /// well-formed record.
    pub fn decode_record(&mut self, record: &[u8]) -> Result<DecodedFrame, WireError> {
        if record.len() < MIN_HEADER_LEN {
            return Err(WireError::Truncated {
                missing: (MIN_HEADER_LEN - record.len()) as u64,
            });
        }
        let (header, header_len) = Self::parse_header(record)?;
        let total = header_len + header.payload_len as usize;
        if record.len() < total {
            return Err(WireError::Truncated {
                missing: (total - record.len()) as u64,
            });
        }
        if record.len() > total {
            return Err(WireError::TrailingBytes {
                count: (record.len() - total) as u64,
            });
        }
        let mut payload = self.pool.acquire(header.payload_len as usize);
        payload.clear();
        payload.extend_from_slice(&record[header_len..]);
        self.fed += record.len() as u64;
        self.consumed += record.len() as u64;
        Ok(DecodedFrame {
            header,
            payload,
            recycle: Some(Arc::clone(&self.pool)),
        })
    }

    /// Assert the stream ended at a record boundary.
    ///
    /// # Errors
    /// [`WireError::Truncated`] when a partial record is still
    /// buffered (its declared length is known — the payload says how
    /// much is missing).
    pub fn finish(&self) -> Result<(), WireError> {
        let unconsumed = &self.pending[self.pos..];
        if unconsumed.is_empty() {
            return Ok(());
        }
        let missing = if unconsumed.len() < MIN_HEADER_LEN {
            (MIN_HEADER_LEN - unconsumed.len()) as u64
        } else {
            let (header, header_len) = Self::parse_header(unconsumed)?;
            (header_len + header.payload_len as usize - unconsumed.len()) as u64
        };
        Err(WireError::Truncated { missing })
    }

    /// Parse + validate the fixed header from the buffer start;
    /// returns the header and its declared wire length (the payload
    /// starts at that offset — extension bytes are skipped, never
    /// interpreted).
    fn parse_header(buf: &[u8]) -> Result<(FrameHeader, usize), WireError> {
        let magic = [buf[0], buf[1], buf[2], buf[3]];
        if magic != MAGIC {
            return Err(WireError::BadMagic {
                found: magic,
                offset: 0,
            });
        }
        let header_len = u16::from_le_bytes([buf[4], buf[5]]) as usize;
        if !(MIN_HEADER_LEN..=MAX_HEADER_LEN).contains(&header_len) {
            return Err(WireError::BadHeaderLen {
                found: header_len as u16,
            });
        }
        let version = u16::from_le_bytes([buf[6], buf[7]]);
        if version != VERSION {
            return Err(WireError::UnsupportedVersion {
                found: version,
                supported: VERSION,
            });
        }
        let format = FormatCode::from_code(buf[8])?;
        let flags = FrameFlags::from_bits(buf[9])?;
        let reserved = u16::from_le_bytes([buf[10], buf[11]]);
        if reserved != 0 {
            return Err(WireError::ReservedNotZero { field: "reserved" });
        }
        let width = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]);
        let height = u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]);
        let payload_len = u32::from_le_bytes([buf[20], buf[21], buf[22], buf[23]]);
        FrameHeader::validate_geometry(width, height, payload_len)?;
        let frame_id = u64::from_le_bytes([
            buf[24], buf[25], buf[26], buf[27], buf[28], buf[29], buf[30], buf[31],
        ]);
        let pts_ns = i64::from_le_bytes([
            buf[32], buf[33], buf[34], buf[35], buf[36], buf[37], buf[38], buf[39],
        ]);
        let emitted_ns = u64::from_le_bytes([
            buf[40], buf[41], buf[42], buf[43], buf[44], buf[45], buf[46], buf[47],
        ]);
        Ok((
            FrameHeader {
                format,
                flags,
                width,
                height,
                payload_len,
                frame_id,
                pts_ns,
                emitted_ns,
            },
            header_len,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    fn header(w: u32, h: u32) -> FrameHeader {
        FrameHeader {
            format: FormatCode::Rgba8,
            flags: FrameFlags::default(),
            width: w,
            height: h,
            payload_len: w * h * 4,
            frame_id: 7,
            pts_ns: 1_000_000,
            emitted_ns: 1_700_000_000_000_000_000,
        }
    }

    fn payload(len: usize) -> Vec<u8> {
        (0..len as u32).map(|i| (i % 251) as u8).collect()
    }

    #[test]
    fn round_trip_exact_fields() {
        let mut h = header(3, 2);
        h.flags = FrameFlags { repeat: true };
        h.pts_ns = -42_000;
        h.frame_id = u64::MAX;
        let bytes = encode(&h, &payload(24)).must();
        let mut decoder = WireDecoder::new();
        decoder.feed(&bytes);
        let frame = decoder.next_frame().must().must();
        assert_eq!(frame.header, h);
        assert_eq!(frame.payload, payload(24));
        decoder.finish().must();
    }

    #[test]
    fn golden_bytes_match_the_spec_table() {
        // Derived from the layout TABLE, not from the encoder: a
        // 2x1 Rgba8 record, repeat flag set, payload [10,20,30,40,
        // 50,60,70,80]. Every field written by hand.
        let h = FrameHeader {
            format: FormatCode::Rgba8,
            flags: FrameFlags { repeat: true },
            width: 2,
            height: 1,
            payload_len: 8,
            frame_id: 0x1122_3344_5566_7788,
            pts_ns: -1_000,
            emitted_ns: 0x0102_0304_0506_0708,
        };
        let bytes = encode(&h, &[10, 20, 30, 40, 50, 60, 70, 80]).must();
        // Written by hand from the layout table, field by field.
        let mut expected = Vec::new();
        expected.extend_from_slice(&[0x4C, 0x55, 0x46, 0x31]); // magic
        expected.extend_from_slice(&48u16.to_le_bytes()); // header_len
        expected.extend_from_slice(&1u16.to_le_bytes()); // version
        expected.push(1); // format = Rgba8
        expected.push(0b0000_0001); // flags = repeat
        expected.extend_from_slice(&0u16.to_le_bytes()); // reserved
        expected.extend_from_slice(&2u32.to_le_bytes()); // width
        expected.extend_from_slice(&1u32.to_le_bytes()); // height
        expected.extend_from_slice(&8u32.to_le_bytes()); // payload_len
        expected.extend_from_slice(&0x1122_3344_5566_7788u64.to_le_bytes()); // frame_id
        expected.extend_from_slice(&(-1_000i64).to_le_bytes()); // pts_ns
        expected.extend_from_slice(&0x0102_0304_0506_0708u64.to_le_bytes()); // emitted_ns
        expected.extend_from_slice(&[10, 20, 30, 40, 50, 60, 70, 80]); // payload
        assert_eq!(bytes, expected, "wire bytes must match the spec table");
        assert_eq!(bytes.len(), 56);
    }

    #[test]
    fn chunked_feed_at_every_split_point() {
        let mut h = header(4, 3);
        h.frame_id = 99;
        let bytes = encode(&h, &payload(48)).must();
        // Feed one record split at EVERY byte boundary, two frames
        // back to back split mid-payload: chunk alignment is
        // irrelevant by construction.
        for split in 0..=bytes.len() {
            let mut decoder = WireDecoder::new();
            decoder.feed(&bytes[..split]);
            let early = decoder.next_frame().must();
            // Before the final split the record is incomplete; AT the
            // final split it may already be complete (and drained).
            assert!(
                early.is_none() || split == bytes.len(),
                "no complete record before the final split ({split})"
            );
            let frame = match early {
                Some(frame) => frame,
                None => {
                    decoder.feed(&bytes[split..]);
                    decoder.next_frame().must().must()
                }
            };
            assert_eq!(frame.header, h);
            assert_eq!(frame.payload, payload(48));
            decoder.finish().must();
        }
        // Two records in one chunk, split between them at a non-header
        // boundary.
        let mut stream = encode(&h, &payload(48)).must();
        stream.extend(encode(&h, &payload(48)).must());
        let mut decoder = WireDecoder::new();
        decoder.feed(&stream[..70]);
        decoder.feed(&stream[70..]);
        let a = decoder.next_frame().must().must();
        let b = decoder.next_frame().must().must();
        assert_eq!(a.header.frame_id, b.header.frame_id);
        assert!(decoder.next_frame().must().is_none());
        assert_eq!(decoder.bytes_fed(), stream.len() as u64);
        assert_eq!(decoder.bytes_consumed(), stream.len() as u64);
        decoder.finish().must();
    }

    #[test]
    fn malformed_inputs_are_typed_refusals() {
        let h = header(2, 2);
        let good = encode(&h, &payload(16)).must();

        // Bad magic.
        let mut bad = good.clone();
        bad[0] = b'X';
        let mut d = WireDecoder::new();
        d.feed(&bad);
        assert_eq!(
            d.next_frame().err(),
            Some(WireError::BadMagic {
                found: [b'X', 0x55, 0x46, 0x31],
                offset: 0
            })
        );

        // Bad version.
        let mut bad = good.clone();
        bad[6] = 2;
        let mut d = WireDecoder::new();
        d.feed(&bad);
        assert_eq!(
            d.next_frame().err(),
            Some(WireError::UnsupportedVersion {
                found: 2,
                supported: 1
            })
        );

        // header_len below the fixed header.
        let mut bad = good.clone();
        bad[4] = 47;
        let mut d = WireDecoder::new();
        d.feed(&bad);
        assert_eq!(
            d.next_frame().err(),
            Some(WireError::BadHeaderLen { found: 47 })
        );

        // Unknown format code.
        let mut bad = good.clone();
        bad[8] = 9;
        let mut d = WireDecoder::new();
        d.feed(&bad);
        assert_eq!(
            d.next_frame().err(),
            Some(WireError::UnknownFormatCode { found: 9 })
        );

        // Flags bits 1..=7 set.
        let mut bad = good.clone();
        bad[9] = 0x02;
        let mut d = WireDecoder::new();
        d.feed(&bad);
        assert_eq!(
            d.next_frame().err(),
            Some(WireError::ReservedNotZero { field: "flags" })
        );

        // Reserved u16 non-zero.
        let mut bad = good.clone();
        bad[10] = 1;
        let mut d = WireDecoder::new();
        d.feed(&bad);
        assert_eq!(
            d.next_frame().err(),
            Some(WireError::ReservedNotZero { field: "reserved" })
        );

        // payload_len disagrees with the geometry.
        let mut bad = good.clone();
        bad[20] = 17;
        let mut d = WireDecoder::new();
        d.feed(&bad);
        assert_eq!(
            d.next_frame().err(),
            Some(WireError::PayloadLengthMismatch {
                declared: 17,
                implied: 16
            })
        );

        // Zero dimension.
        let mut bad = good.clone();
        bad[12] = 0;
        bad[13] = 0;
        bad[14] = 0;
        bad[15] = 0;
        bad[20] = 0;
        bad[21] = 0;
        bad[22] = 0;
        bad[23] = 0;
        let mut d = WireDecoder::new();
        d.feed(&bad);
        assert_eq!(
            d.next_frame().err(),
            Some(WireError::BadGeometry {
                reason: "zero dimension"
            })
        );
    }

    #[test]
    fn truncated_streams_report_exact_missing_bytes() {
        let h = header(2, 2);
        let bytes = encode(&h, &payload(16)).must();
        // Cut inside the header: 48 - 20 = 28 missing.
        let mut d = WireDecoder::new();
        d.feed(&bytes[..20]);
        assert!(d.next_frame().must().is_none());
        assert_eq!(d.finish().err(), Some(WireError::Truncated { missing: 28 }));
        // Cut inside the payload: 64 - 50 = 14 missing.
        let mut d = WireDecoder::new();
        d.feed(&bytes[..50]);
        assert!(d.next_frame().must().is_none());
        assert_eq!(d.finish().err(), Some(WireError::Truncated { missing: 14 }));
    }

    #[test]
    fn extension_headers_are_skipped_not_guessed() {
        // A record with header_len 56 (8 extension bytes this decoder
        // does not interpret): framing must survive, semantics must
        // not be invented.
        let h = header(2, 1);
        let mut bytes = encode(&h, &payload(8)).must();
        bytes[4] = 56;
        let mut extension = vec![0xAB; 8];
        let mut record = bytes[..48].to_vec();
        extension.extend_from_slice(&bytes[48..]);
        record.extend_from_slice(&extension);
        let mut d = WireDecoder::new();
        d.feed(&record);
        let frame = d.next_frame().must().must();
        assert_eq!(frame.header, h);
        assert_eq!(frame.payload, payload(8));
        d.finish().must();
    }

    #[test]
    fn encoder_refuses_unexpressible_records() {
        // Zero dimension.
        let mut h = header(0, 2);
        h.payload_len = 0;
        assert_eq!(
            encode(&h, &[]).err(),
            Some(WireError::BadGeometry {
                reason: "zero dimension"
            })
        );
        // Dimension cap.
        let h = header(65_536, 1);
        assert_eq!(
            encode(&h, &[]).err(),
            Some(WireError::BadGeometry {
                reason: "dimension exceeds 65535"
            })
        );
        // Payload cap (20000x20000x4 = 1.6 GiB overflows the 1 GiB
        // cap; dims themselves stay legal).
        let mut h = header(20_000, 20_000);
        h.payload_len = u32::MAX;
        assert_eq!(
            encode(&h, &[]).err(),
            Some(WireError::BadGeometry {
                reason: "payload exceeds the 1 GiB cap"
            })
        );
        // Payload length disagrees with the buffer.
        let h = header(2, 2);
        assert_eq!(
            encode(&h, &payload(15)).err(),
            Some(WireError::PayloadLengthMismatch {
                declared: 16,
                implied: 15
            })
        );
    }

    #[test]
    fn format_code_table_is_exactly_the_upload_path() {
        assert_eq!(FormatCode::Rgba8.code(), 1);
        assert_eq!(FormatCode::Bgra8.code(), 2);
        assert_eq!(FormatCode::from_code(1).must(), FormatCode::Rgba8);
        assert_eq!(FormatCode::from_code(2).must(), FormatCode::Bgra8);
        assert_eq!(
            FormatCode::from_code(3).err(),
            Some(WireError::UnknownFormatCode { found: 3 })
        );
        // The table mirrors renderer-wgpu's upload routing exactly.
        for format in [
            PixelFormat::Rgba8,
            PixelFormat::Bgra8,
            PixelFormat::Nv12,
            PixelFormat::I420,
            PixelFormat::Yuyv422,
        ] {
            assert_eq!(
                FormatCode::for_pixel_format(format).is_some(),
                renderer_wgpu::uploads_format(format),
                "wire table must match the renderer upload path for {format:?}"
            );
        }
        assert_eq!(FormatCode::Rgba8.pixel_format(), PixelFormat::Rgba8);
        assert_eq!(FormatCode::Bgra8.pixel_format(), PixelFormat::Bgra8);
    }

    #[test]
    fn header_overhead_is_recorded_math_not_assertions() {
        // Framing overhead as arithmetic (the doc's §4 numbers).
        for (w, h) in [(1920u32, 1080u32), (3840, 2160)] {
            let payload = u64::from(w) * u64::from(h) * 4;
            let overhead =
                f64::from(MIN_HEADER_LEN as u32) / ((payload + MIN_HEADER_LEN as u64) as f64);
            assert!(overhead < 0.000_01, "{overhead}");
        }
    }

    #[test]
    fn decode_record_is_the_streaming_result_direct_from_the_slice() {
        // The direct-hand-off API (the mailbox transport's send path)
        // and the chunked streaming API agree on every field and every
        // payload byte — one codec, two entry points, no drift.
        let mut h = header(3, 2);
        h.frame_id = 11;
        let record = encode(&h, &payload(24)).must();

        let mut direct = WireDecoder::new();
        let frame = direct.decode_record(&record).must();
        assert_eq!(frame.header, h);
        assert_eq!(frame.payload, payload(24));
        // Diagnostics accumulate exactly like the streaming path.
        assert_eq!(direct.bytes_fed(), record.len() as u64);
        assert_eq!(direct.bytes_consumed(), record.len() as u64);

        let mut streaming = WireDecoder::new();
        streaming.feed(&record);
        let twin = streaming.next_frame().must().must();
        assert_eq!(frame, twin, "direct and streaming decodes are equal");
        streaming.finish().must();
    }

    #[test]
    fn decode_record_refuses_truncated_and_trailing_input() {
        let h = header(2, 2);
        let record = encode(&h, &payload(16)).must();
        let mut d = WireDecoder::new();

        // Cut inside the header: 48 - 20 = 28 missing.
        assert_eq!(
            d.decode_record(&record[..20]).err(),
            Some(WireError::Truncated { missing: 28 })
        );
        // Cut inside the payload: 64 - 50 = 14 missing.
        assert_eq!(
            d.decode_record(&record[..50]).err(),
            Some(WireError::Truncated { missing: 14 })
        );
        // Two records in one hand-off: the second record's 64 bytes
        // are trailing.
        let mut two = record.clone();
        two.extend(record.clone());
        assert_eq!(
            d.decode_record(&two).err(),
            Some(WireError::TrailingBytes { count: 64 })
        );
        // Garbage after one complete record is trailing too.
        let mut trailing = record.clone();
        trailing.extend(b"XX");
        assert_eq!(
            d.decode_record(&trailing).err(),
            Some(WireError::TrailingBytes { count: 2 })
        );
        // A refusal leaves no state behind: the same decoder takes
        // the good record next.
        assert_eq!(d.decode_record(&record).must().header.frame_id, h.frame_id);
    }

    #[test]
    fn one_decoder_recycles_payload_buffers_across_records() {
        // The C7 shape: ONE decoder, N records, each frame dropped by
        // its consumer — the payload buffers cycle through the pool
        // (the measured zero-allocation proof is the dedicated
        // counting-allocator binary; this pins the *correctness* of
        // the reuse: no state bleed between recycled buffers).
        let mut d = WireDecoder::new();
        for id in 0..8u64 {
            let mut h = header(2, 2);
            h.frame_id = id;
            h.pts_ns = id as i64 * 1_000;
            let record = encode(&h, &payload(16)).must();
            let frame = d.decode_record(&record).must();
            assert_eq!(frame.header.frame_id, id);
            assert_eq!(frame.header.pts_ns, id as i64 * 1_000);
            assert_eq!(frame.payload, payload(16));
            // The consumer drops the frame: the buffer returns to the
            // pool (the next iteration must reuse it cleanly).
            drop(frame);
        }
        // Geometry changes ride the same pool (grow-once path).
        let big = header(4, 4);
        let record = encode(&big, &payload(64)).must();
        let frame = d.decode_record(&record).must();
        assert_eq!(frame.payload, payload(64));
    }

    #[test]
    fn partial_records_compact_without_losing_bytes() {
        // Cursor + compaction: a record split across feeds with
        // consumed bytes in front of it — the live tail moves to the
        // front, nothing is lost or duplicated.
        let mut first = header(2, 2);
        first.frame_id = 1;
        let mut second = header(2, 2);
        second.frame_id = 2;
        let mut third = header(2, 2);
        third.frame_id = 3;
        let r1 = encode(&first, &payload(16)).must();
        let r2 = encode(&second, &payload(16)).must();
        let r3 = encode(&third, &payload(16)).must();

        let mut d = WireDecoder::new();
        // Feed record 1 whole plus the first half of record 2.
        d.feed(&r1);
        d.feed(&r2[..r2.len() / 2]);
        assert_eq!(d.next_frame().must().must().header.frame_id, 1);
        // The consumed prefix is still in the buffer (cursor at r1's
        // end); feeding the rest of record 2 plus all of record 3
        // must compact, not corrupt.
        d.feed(&r2[r2.len() / 2..]);
        d.feed(&r3);
        assert_eq!(d.next_frame().must().must().header.frame_id, 2);
        assert_eq!(d.next_frame().must().must().header.frame_id, 3);
        assert!(d.next_frame().must().is_none());
        assert_eq!(d.bytes_fed(), (r1.len() + r2.len() + r3.len()) as u64);
        assert_eq!(d.bytes_consumed(), d.bytes_fed());
        d.finish().must();
    }

    #[test]
    fn clones_and_hand_built_frames_behave_like_plain_owners() {
        // No pool: a hand-built frame drops like a plain Vec owner.
        let hand = DecodedFrame::detached(header(1, 1), payload(4));
        assert_eq!(hand.payload.len(), 4);
        drop(hand);
        // A clone shares nothing pooled: each owns its Vec; both
        // return cleanly (bounded by the pool's slot count).
        let mut d = WireDecoder::new();
        let record = encode(&header(1, 1), &payload(4)).must();
        let frame = d.decode_record(&record).must();
        let clone = frame.clone();
        assert_eq!(clone, frame, "equality ignores the pool handle");
        drop(frame);
        assert_eq!(clone.payload.len(), 4);
        drop(clone);
        // Equality is header + payload, pool handle invisible.
        let twin = DecodedFrame::detached(header(1, 1), payload(4));
        assert_eq!(d.decode_record(&record).must(), twin);
    }
}
