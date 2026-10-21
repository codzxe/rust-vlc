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

/// One fully decoded record: header + owned payload (the decoder's
/// output; the encoder works from the engine's frame types).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedFrame {
    /// The record header.
    pub header: FrameHeader,
    /// The payload bytes (`header.payload_len` of them, row-major).
    pub payload: Vec<u8>,
}

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
#[derive(Debug, Clone)]
pub struct WireDecoder {
    /// Bytes not yet consumed (partial header or partial record).
    pending: Vec<u8>,
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
            fed: 0,
            consumed: 0,
        }
    }

    /// Feed one chunk of the stream (any length, any alignment).
    pub fn feed(&mut self, chunk: &[u8]) {
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
    /// # Errors
    /// [`WireError`] for malformed records (sticky — see the type
    /// docs).
    /// Returns `Ok(None)` when more input is needed.
    pub fn next_frame(&mut self) -> Result<Option<DecodedFrame>, WireError> {
        if self.pending.len() < MIN_HEADER_LEN {
            return Ok(None);
        }
        let (header, header_len) = Self::parse_header(&self.pending).map_err(|mut err| {
            // Stream-offset honesty: the record under the cursor
            // starts at `consumed` in fed-stream coordinates.
            if let WireError::BadMagic { offset, .. } = &mut err {
                *offset = self.consumed;
            }
            err
        })?;
        let total = header_len + header.payload_len as usize;
        if self.pending.len() < total {
            return Ok(None);
        }
        let payload = self.pending[header_len..total].to_vec();
        self.pending.drain(..total);
        self.consumed += total as u64;
        Ok(Some(DecodedFrame { header, payload }))
    }

    /// Assert the stream ended at a record boundary.
    ///
    /// # Errors
    /// [`WireError::Truncated`] when a partial record is still
    /// buffered (its declared length is known — the payload says how
    /// much is missing).
    pub fn finish(&self) -> Result<(), WireError> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let missing = if self.pending.len() < MIN_HEADER_LEN {
            (MIN_HEADER_LEN - self.pending.len()) as u64
        } else {
            let (header, header_len) = Self::parse_header(&self.pending)?;
            (header_len + header.payload_len as usize - self.pending.len()) as u64
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
}
