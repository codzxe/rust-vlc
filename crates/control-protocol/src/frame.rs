//! RFC 6455 §5 frame codec — card 7.J1, in-tree.
//!
//! Server-side parsing (masked client frames) and server-side writing
//! (unmasked server frames), with:
//!
//! * all three length forms (7-bit, 16-bit, 64-bit — the 64-bit form's
//!   MSB must be zero and the claim is checked against the cap BEFORE
//!   any allocation: an oversized claim is a typed
//!   [`ProtocolError::MessageTooLarge`], never an OOM vector);
//! * fragmentation (continuation frames reassembled by
//!   [`MessageAssembler`], with interleaved control frames);
//! * control frames: `ping`/`pong` (payload ≤ 125, answered by the
//!   session loop), `close` (payload = 2-byte code + UTF-8 reason,
//!   validated);
//! * UTF-8 validation where the RFC requires it (text messages, close
//!   reasons) — invalid UTF-8 is a typed 1007 close;
//! * hard caps on single frames and reassembled messages (D-1).
//!
//! Every dirty-world input (truncated headers, RSV bits set, unmasked
//! client frames, fragmented control frames, oversize claims, garbage
//! opcodes) is a typed [`ProtocolError`] — never a panic.

use std::io::Read;
use std::io::Write;

use crate::error::CloseCode;
use crate::error::ProtocolError;

/// Data-frame opcodes (RFC 6455 §5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Opcode {
    /// 0 — continuation of a fragmented message.
    Continuation,
    /// 1 — text (must be valid UTF-8 once reassembled).
    Text,
    /// 2 — binary.
    Binary,
    /// 8 — close.
    Close,
    /// 9 — ping.
    Ping,
    /// 10 — pong.
    Pong,
}

impl Opcode {
    /// From the 4-bit wire value; `None` for reserved/garbage opcodes.
    pub const fn from_u8(value: u8) -> Option<Opcode> {
        match value {
            0 => Some(Opcode::Continuation),
            1 => Some(Opcode::Text),
            2 => Some(Opcode::Binary),
            8 => Some(Opcode::Close),
            9 => Some(Opcode::Ping),
            10 => Some(Opcode::Pong),
            _ => None,
        }
    }

    /// The wire value.
    pub const fn as_u8(self) -> u8 {
        match self {
            Opcode::Continuation => 0,
            Opcode::Text => 1,
            Opcode::Binary => 2,
            Opcode::Close => 8,
            Opcode::Ping => 9,
            Opcode::Pong => 10,
        }
    }

    /// Whether this is a control opcode (RFC 6455 §5.5).
    pub const fn is_control(self) -> bool {
        matches!(self, Opcode::Close | Opcode::Ping | Opcode::Pong)
    }
}

/// One parsed frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Final-fragment flag.
    pub fin: bool,
    /// The opcode.
    pub opcode: Opcode,
    /// The (already unmasked) payload.
    pub payload: Vec<u8>,
}

/// Read exactly `n` bytes; `Ok(None)` on clean EOF at a frame boundary.
fn read_exact<R: Read>(stream: &mut R, n: usize) -> Result<Option<Vec<u8>>, ProtocolError> {
    let mut buf = vec![0u8; n];
    let mut filled = 0;
    while filled < n {
        match stream.read(&mut buf[filled..]) {
            Ok(0) => {
                return if filled == 0 {
                    Ok(None)
                } else {
                    Err(ProtocolError::Frame {
                        code: CloseCode::ProtocolError,
                        reason: "eof mid-frame".to_owned(),
                    })
                };
            }
            Ok(count) => filled += count,
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::Interrupted
                        | std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                ) =>
            {
                if err.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(ProtocolError::Io { what: "reading frame", source: err });
            }
            Err(err) => {
                return if is_disconnect(&err) && filled == 0 {
                    Ok(None)
                } else {
                    Err(ProtocolError::Io { what: "reading frame", source: err })
                };
            }
        }
    }
    Ok(Some(buf))
}

/// Whether an I/O error is a peer disconnect.
fn is_disconnect(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::BrokenPipe
    )
}

/// Which side of the connection a frame reader serves (mirror
/// strictness: the server demands masked frames, the client demands
/// unmasked ones — RFC 6455 §5.1 both ways).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endpoint {
    /// Reading client → server frames: mask REQUIRED.
    Server,
    /// Reading server → client frames: mask FORBIDDEN.
    Client,
}

/// Read one frame from `stream` in the SERVER role (client frames
/// MUST be masked — RFC 6455 §5.1).
///
/// `max_frame` bounds both the payload and the length claim — a claim
/// above the cap is rejected before any allocation.
///
/// # Errors
/// Typed [`ProtocolError`] on every RFC 6455 violation; `Ok(None)` on
/// clean EOF between frames.
pub fn read_frame<R: Read>(stream: &mut R, max_frame: u64) -> Result<Option<Frame>, ProtocolError> {
    read_frame_endpoint(stream, max_frame, Endpoint::Server)
}

/// Read one frame in the CLIENT role (server frames MUST be unmasked;
/// a masked server frame is a protocol violation — the mirror of the
/// server's unmasked-client rejection).
///
/// # Errors
/// Typed [`ProtocolError`] on every RFC 6455 violation; `Ok(None)` on
/// clean EOF between frames.
pub fn read_frame_unmasked<R: Read>(
    stream: &mut R,
    max_frame: u64,
) -> Result<Option<Frame>, ProtocolError> {
    read_frame_endpoint(stream, max_frame, Endpoint::Client)
}

fn read_frame_endpoint<R: Read>(
    stream: &mut R,
    max_frame: u64,
    endpoint: Endpoint,
) -> Result<Option<Frame>, ProtocolError> {
    let Some(head) = read_exact(stream, 2)? else {
        return Ok(None);
    };
    let fin = head[0] & 0x80 != 0;
    let rsv = head[0] & 0x70;
    let opcode_value = head[0] & 0x0f;
    let masked = head[1] & 0x80 != 0;
    let len7 = head[1] & 0x7f;
    if rsv != 0 {
        return Err(ProtocolError::Frame {
            code: CloseCode::ProtocolError,
            reason: "reserved bits set (no extension negotiated)".to_owned(),
        });
    }
    let opcode = Opcode::from_u8(opcode_value).ok_or_else(|| ProtocolError::Frame {
        code: CloseCode::ProtocolError,
        reason: format!("reserved opcode 0x{opcode_value:x}"),
    })?;
    if !masked && endpoint == Endpoint::Server {
        // RFC 6455 §5.1: the server MUST close on unmasked client frames.
        return Err(ProtocolError::Frame {
            code: CloseCode::ProtocolError,
            reason: "unmasked client frame".to_owned(),
        });
    }
    if masked && endpoint == Endpoint::Client {
        // The mirror: a server MUST NOT mask its frames.
        return Err(ProtocolError::Frame {
            code: CloseCode::ProtocolError,
            reason: "masked server frame".to_owned(),
        });
    }
    let length: u64 = match len7 {
        126 => {
            let Some(bytes) = read_exact(stream, 2)? else {
                return Err(truncated("16-bit length"));
            };
            u16::from_be_bytes([bytes[0], bytes[1]]) as u64
        }
        127 => {
            let Some(bytes) = read_exact(stream, 8)? else {
                return Err(truncated("64-bit length"));
            };
            let mut wide = [0u8; 8];
            wide.copy_from_slice(&bytes);
            let value = u64::from_be_bytes(wide);
            if value & (1 << 63) != 0 {
                return Err(ProtocolError::Frame {
                    code: CloseCode::ProtocolError,
                    reason: "64-bit length with MSB set".to_owned(),
                });
            }
            value
        }
        small => small as u64,
    };
    if opcode.is_control() {
        if !fin {
            return Err(ProtocolError::Frame {
                code: CloseCode::ProtocolError,
                reason: "fragmented control frame".to_owned(),
            });
        }
        if length > 125 {
            return Err(ProtocolError::Frame {
                code: CloseCode::ProtocolError,
                reason: "control frame payload above 125 bytes".to_owned(),
            });
        }
    }
    if length > max_frame {
        return Err(ProtocolError::MessageTooLarge { limit: max_frame, claimed: length });
    }
    let Some(mask_bytes) = read_exact(stream, 4)? else {
        return Err(truncated("masking key"));
    };
    let mask = [mask_bytes[0], mask_bytes[1], mask_bytes[2], mask_bytes[3]];
    let Some(mut payload) = read_exact(stream, length as usize)? else {
        return Err(truncated("payload"));
    };
    if masked {
        // RFC 6455 §5.3: unmask with the repeating 4-byte key.
        for (i, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[i & 3];
        }
    }
    Ok(Some(Frame { fin, opcode, payload }))
}

fn truncated(what: &'static str) -> ProtocolError {
    ProtocolError::Frame {
        code: CloseCode::ProtocolError,
        reason: format!("connection truncated inside the {what}"),
    }
}

/// Write one frame, server side (unmasked, RFC 6455 §5.1).
///
/// # Errors
/// [`ProtocolError::Io`] on write failure.
pub fn write_frame<W: Write>(
    stream: &mut W,
    fin: bool,
    opcode: Opcode,
    payload: &[u8],
) -> Result<(), ProtocolError> {
    let mut head = Vec::with_capacity(10);
    head.push(if fin { 0x80 } else { 0x00 } | opcode.as_u8());
    let len = payload.len();
    if opcode.is_control() && len > 125 {
        // Server bug, not peer input: refuse loudly.
        return Err(ProtocolError::Frame {
            code: CloseCode::ProtocolError,
            reason: "refusing to write an oversized control frame".to_owned(),
        });
    }
    match len {
        0..=125 => head.push(len as u8),
        126..=65_535 => {
            head.push(126);
            head.extend_from_slice(&(len as u16).to_be_bytes());
        }
        _ => {
            head.push(127);
            head.extend_from_slice(&(len as u64).to_be_bytes());
        }
    }
    stream
        .write_all(&head)
        .and_then(|()| stream.write_all(payload))
        .map_err(|source| ProtocolError::Io { what: "writing frame", source })
}

/// A fully-reassembled data message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// `text` or `binary`.
    pub opcode: Opcode,
    /// Payload; for text messages, valid UTF-8 by construction.
    pub payload: Vec<u8>,
}

/// Fragmentation reassembly (RFC 6455 §5.4) with a hard message cap.
#[derive(Debug)]
pub struct MessageAssembler {
    /// Opcode of the first fragment (the message's opcode).
    opcode: Option<Opcode>,
    buffer: Vec<u8>,
    cap: u64,
}

impl MessageAssembler {
    /// New assembler allowing messages up to `cap` bytes total.
    pub fn new(cap: u64) -> MessageAssembler {
        MessageAssembler { opcode: None, buffer: Vec::new(), cap }
    }

    /// Whether a fragmented message is mid-reassembly.
    pub fn is_mid_message(&self) -> bool {
        self.opcode.is_some()
    }

    /// Feed one DATA frame (control frames are the session loop's
    /// business). `Ok(Some(message))` when the final fragment arrived.
    ///
    /// # Errors
    /// Typed [`ProtocolError`] on continuation without start, new
    /// message while mid-reassembly, message cap exceeded, or invalid
    /// UTF-8 in a completed text message.
    pub fn push(&mut self, frame: Frame) -> Result<Option<Message>, ProtocolError> {
        match frame.opcode {
            Opcode::Continuation => {
                let Some(opcode) = self.opcode else {
                    return Err(ProtocolError::Frame {
                        code: CloseCode::ProtocolError,
                        reason: "continuation frame without a started message".to_owned(),
                    });
                };
                let fin = frame.fin;
                let done = self.append(&frame)?;
                if fin && done {
                    self.opcode = None;
                    let payload = std::mem::take(&mut self.buffer);
                    if opcode == Opcode::Text {
                        validate_utf8(&payload)?;
                    }
                    return Ok(Some(Message { opcode, payload }));
                }
            }
            Opcode::Text | Opcode::Binary => {
                if self.opcode.is_some() {
                    return Err(ProtocolError::Frame {
                        code: CloseCode::ProtocolError,
                        reason: "new data frame while a fragmented message is open".to_owned(),
                    });
                }
                if frame.fin {
                    if frame.opcode == Opcode::Text {
                        validate_utf8(&frame.payload)?;
                    }
                    return Ok(Some(Message { opcode: frame.opcode, payload: frame.payload }));
                }
                self.opcode = Some(frame.opcode);
                self.buffer = frame.payload;
            }
            _ => {
                return Err(ProtocolError::Frame {
                    code: CloseCode::ProtocolError,
                    reason: "control frame fed to the message assembler".to_owned(),
                });
            }
        }
        Ok(None)
    }

    /// Append a continuation's payload; `true` when this call completed
    /// the message (always the case for `fin` frames here).
    fn append(&mut self, frame: &Frame) -> Result<bool, ProtocolError> {
        let total = self.buffer.len() as u64 + frame.payload.len() as u64;
        if total > self.cap {
            return Err(ProtocolError::MessageTooLarge { limit: self.cap, claimed: total });
        }
        self.buffer.extend_from_slice(&frame.payload);
        Ok(frame.fin)
    }
}

/// RFC 6455 §8.1: text payloads must be valid UTF-8.
fn validate_utf8(payload: &[u8]) -> Result<(), ProtocolError> {
    if std::str::from_utf8(payload).is_ok() {
        Ok(())
    } else {
        Err(ProtocolError::InvalidUtf8)
    }
}

/// Parse a close frame's payload into `(code, reason)`; an empty
/// payload means "no code" (RFC 6455 §5.5.1) — represented as code 1005
/// semantically, returned here as `None`.
///
/// # Errors
/// Typed [`ProtocolError`] for a 1-byte payload (truncated code),
/// an invalid UTF-8 reason, or a close code that MUST NOT appear on
/// the wire (RFC 6455 §7.4: 0–999, 1004, 1005, 1006, 1016–2999).
pub fn parse_close_payload(payload: &[u8]) -> Result<Option<(u16, String)>, ProtocolError> {
    match payload.len() {
        0 => Ok(None),
        1 => Err(ProtocolError::Frame {
            code: CloseCode::ProtocolError,
            reason: "close payload of 1 byte (truncated status code)".to_owned(),
        }),
        _ => {
            let code = u16::from_be_bytes([payload[0], payload[1]]);
            if !is_valid_on_wire_close_code(code) {
                return Err(ProtocolError::Frame {
                    code: CloseCode::ProtocolError,
                    reason: format!("close code {code} is not valid on the wire"),
                });
            }
            let reason = std::str::from_utf8(&payload[2..])
                .map_err(|_| ProtocolError::InvalidUtf8)?
                .to_owned();
            Ok(Some((code, reason)))
        }
    }
}

/// RFC 6455 §7.4 on-wire validity: 1000–1003, 1007–1011, 3000–4999.
pub fn is_valid_on_wire_close_code(code: u16) -> bool {
    matches!(code, 1000..=1003 | 1007..=1011 | 3000..=4999)
}

/// Build a close frame payload.
pub fn close_payload(code: CloseCode, reason: &str) -> Vec<u8> {
    let mut payload = Vec::with_capacity(2 + reason.len().min(123));
    payload.extend_from_slice(&code.as_u16().to_be_bytes());
    // Cap the reason at 123 bytes on a char boundary (control frames
    // are ≤ 125 total).
    let mut reason_bytes = reason.as_bytes();
    while reason_bytes.len() > 123 {
        let mut cut = 123;
        while cut > 0 && !reason.is_char_boundary(cut) {
            cut -= 1;
        }
        reason_bytes = &reason_bytes[..cut.max(1)];
    }
    payload.extend_from_slice(reason_bytes);
    payload
}

/// Unmask `payload` in place with `mask` (client-side writer helper).
pub fn apply_mask(payload: &mut [u8], mask: [u8; 4]) {
    for (i, byte) in payload.iter_mut().enumerate() {
        *byte ^= mask[i & 3];
    }
}

/// Write one frame in the CLIENT role: masked (RFC 6455 §5.1/§5.3)
/// with the caller-supplied key — tests pin exact masks so the wire
/// bytes are reproducible.
///
/// # Errors
/// [`ProtocolError::Io`] on write failure.
pub fn write_masked_frame<W: Write>(
    stream: &mut W,
    fin: bool,
    opcode: Opcode,
    payload: &[u8],
    mask: [u8; 4],
) -> Result<(), ProtocolError> {
    let mut head = Vec::with_capacity(10);
    head.push(if fin { 0x80 } else { 0x00 } | opcode.as_u8());
    let len = payload.len();
    if opcode.is_control() && len > 125 {
        return Err(ProtocolError::Frame {
            code: CloseCode::ProtocolError,
            reason: "refusing to write an oversized control frame".to_owned(),
        });
    }
    match len {
        0..=125 => head.push(0x80 | len as u8),
        126..=65_535 => {
            head.push(0x80 | 126);
            head.extend_from_slice(&(len as u16).to_be_bytes());
        }
        _ => {
            head.push(0x80 | 127);
            head.extend_from_slice(&(len as u64).to_be_bytes());
        }
    }
    head.extend_from_slice(&mask);
    let mut masked = payload.to_vec();
    apply_mask(&mut masked, mask);
    stream
        .write_all(&head)
        .and_then(|()| stream.write_all(&masked))
        .map_err(|source| ProtocolError::Io { what: "writing frame", source })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An in-memory reader for codec-level tests.
    fn reader(bytes: &[u8]) -> std::io::Cursor<&[u8]> {
        std::io::Cursor::new(bytes)
    }

    /// Build a masked client frame (the same shape the test client
    /// writes and the server parses).
    fn client_frame(fin: bool, opcode: Opcode, payload: &[u8], mask: [u8; 4]) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(if fin { 0x80 } else { 0x00 } | opcode.as_u8());
        out.push(0x80 | match payload.len() {
            0..=125 => payload.len() as u8,
            126..=65_535 => 126,
            _ => 127,
        });
        if (126..=65_535).contains(&payload.len()) {
            out.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        } else if payload.len() > 65_535 {
            out.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        }
        out.extend_from_slice(&mask);
        let mut masked = payload.to_vec();
        apply_mask(&mut masked, mask);
        out.extend_from_slice(&masked);
        out
    }

    #[test]
    fn roundtrip_all_length_forms() {
        for len in [0usize, 1, 125, 126, 127, 65535] {
            let payload: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
            let bytes = client_frame(true, Opcode::Text, &payload, [1, 2, 3, 4]);
            let mut stream = reader(&bytes);
            match read_frame(&mut stream, 1 << 20) {
                Ok(Some(frame)) => {
                    assert!(frame.fin);
                    assert_eq!(frame.opcode, Opcode::Text);
                    assert_eq!(frame.payload, payload, "len {len}");
                }
                Ok(None) => unreachable!("frame of len {len} vanished"),
                Err(err) => unreachable!("frame of len {len} failed: {err}"),
            }
        }
    }

    #[test]
    fn unmasked_client_frame_is_rejected() {
        let bytes = vec![0x81, 0x02, b'h', b'i'];
        let mut stream = reader(&bytes);
        match read_frame(&mut stream, 1024) {
            Err(ProtocolError::Frame { code, .. }) => assert_eq!(code, CloseCode::ProtocolError),
            other => unreachable!("unmasked frame accepted: {other:?}"),
        }
    }

    #[test]
    fn rsv_bits_and_bad_opcode_are_rejected() {
        // RSV1 set.
        let mut bytes = client_frame(true, Opcode::Text, b"x", [0; 4]);
        bytes[0] |= 0x40;
        let mut stream = reader(&bytes);
        match read_frame(&mut stream, 1024) {
            Err(ProtocolError::Frame { reason, .. }) => {
                assert!(reason.contains("reserved bits"))
            }
            other => unreachable!("rsv frame accepted: {other:?}"),
        }
        // Opcode 3 (reserved).
        let mut bytes = client_frame(true, Opcode::Text, b"x", [0; 4]);
        bytes[0] = 0x83;
        let mut stream = reader(&bytes);
        match read_frame(&mut stream, 1024) {
            Err(ProtocolError::Frame { reason, .. }) => {
                assert!(reason.contains("reserved opcode"))
            }
            other => unreachable!("opcode-3 frame accepted: {other:?}"),
        }
    }

    #[test]
    fn oversized_length_claims_reject_before_allocating() {
        // 64-bit length claiming 4 GiB with a tiny actual body.
        let mut bytes = vec![0x81, 0x80 | 127];
        bytes.extend_from_slice(&(4u64 << 30).to_be_bytes());
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        let mut stream = reader(&bytes);
        match read_frame(&mut stream, 1024) {
            Err(ProtocolError::MessageTooLarge { limit, claimed }) => {
                assert_eq!(limit, 1024);
                assert_eq!(claimed, 4 << 30);
            }
            other => unreachable!("oversize accepted: {other:?}"),
        }
        // Control frame claiming 126+ bytes.
        let mut bytes = vec![0x89, 0x80 | 126];
        bytes.extend_from_slice(&200u16.to_be_bytes());
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        let mut stream = reader(&bytes);
        match read_frame(&mut stream, 1 << 20) {
            Err(ProtocolError::Frame { reason, .. }) => {
                assert!(reason.contains("control frame payload"))
            }
            other => unreachable!("oversized control accepted: {other:?}"),
        }
    }

    #[test]
    fn truncated_headers_are_typed_errors() {
        for cut in 1..=7 {
            let full = client_frame(true, Opcode::Text, b"hello", [9, 9, 9, 9]);
            let mut stream = reader(&full[..full.len().min(cut)]);
            let outcome = read_frame(&mut stream, 1024);
            match outcome {
                Err(ProtocolError::Frame { reason, .. }) => {
                    assert!(reason.contains("truncated") || reason.contains("eof"))
                }
                other => unreachable!("truncation at {cut} gave {other:?}"),
            }
        }
    }

    #[test]
    fn fragmented_text_reassembles_with_interleaved_ping() {
        let mut assembler = MessageAssembler::new(1024);
        let part1 = client_frame(false, Opcode::Text, b"hello ", [1, 1, 1, 1]);
        let ping = client_frame(true, Opcode::Ping, b"?", [2, 2, 2, 2]);
        let part2 = client_frame(false, Opcode::Continuation, b"wor", [3, 3, 3, 3]);
        let part3 = client_frame(true, Opcode::Continuation, b"ld", [4, 4, 4, 4]);
        let wire = [part1, ping, part2, part3].concat();
        let mut stream = reader(&wire);
        let mut message = None;
        while message.is_none() {
            match read_frame(&mut stream, 1024) {
                Ok(Some(frame)) if frame.opcode.is_control() => {
                    assert_eq!(frame.opcode, Opcode::Ping);
                }
                Ok(Some(frame)) => {
                    message = match assembler.push(frame) {
                        Ok(done) => done,
                        Err(err) => unreachable!("fragment rejected: {err}"),
                    };
                }
                Ok(None) => unreachable!("stream ended mid-message"),
                Err(err) => unreachable!("fragment failed: {err}"),
            }
        }
        let message = match message {
            Some(message) => message,
            None => unreachable!("loop exited without a message"),
        };
        assert_eq!(message.opcode, Opcode::Text);
        assert_eq!(message.payload, b"hello world");
    }

    #[test]
    fn assembler_rejects_dirty_fragment_sequences() {
        // Continuation without start.
        let mut assembler = MessageAssembler::new(1024);
        let frame = Frame { fin: true, opcode: Opcode::Continuation, payload: vec![1] };
        match assembler.push(frame) {
            Err(ProtocolError::Frame { reason, .. }) => {
                assert!(reason.contains("without a started message"))
            }
            other => unreachable!("orphan continuation accepted: {other:?}"),
        }
        // New message while mid-reassembly.
        let mut assembler = MessageAssembler::new(1024);
        let _ = assembler.push(Frame { fin: false, opcode: Opcode::Text, payload: vec![1] });
        let frame = Frame { fin: true, opcode: Opcode::Text, payload: vec![2] };
        match assembler.push(frame) {
            Err(ProtocolError::Frame { reason, .. }) => {
                assert!(reason.contains("fragmented message is open"))
            }
            other => unreachable!("interleaved message accepted: {other:?}"),
        }
        // Message cap exceeded by fragments.
        let mut assembler = MessageAssembler::new(4);
        let _ = assembler.push(Frame { fin: false, opcode: Opcode::Text, payload: vec![1, 2] });
        let frame = Frame { fin: true, opcode: Opcode::Continuation, payload: vec![3, 4, 5] };
        match assembler.push(frame) {
            Err(ProtocolError::MessageTooLarge { limit, claimed }) => {
                assert_eq!((limit, claimed), (4, 5));
            }
            other => unreachable!("cap not enforced: {other:?}"),
        }
    }

    #[test]
    fn invalid_utf8_text_is_a_typed_close() {
        let bad = vec![0x81, 0x80 | 0x02, 0, 0, 0, 0, 0xff, 0xfe];
        let mut stream = reader(&bad);
        match read_frame(&mut stream, 1024) {
            Ok(Some(frame)) => {
                let mut assembler = MessageAssembler::new(1024);
                match assembler.push(frame) {
                    Err(ProtocolError::InvalidUtf8) => {}
                    other => unreachable!("invalid utf8 accepted: {other:?}"),
                }
            }
            other => unreachable!("frame read failed: {other:?}"),
        }
    }

    #[test]
    fn close_payload_validation() {
        match parse_close_payload(&[]) {
            Ok(None) => {}
            other => unreachable!("empty close payload: {other:?}"),
        }
        match parse_close_payload(&[0x0f]) {
            Err(ProtocolError::Frame { reason, .. }) => {
                assert!(reason.contains("truncated status code"))
            }
            other => unreachable!("1-byte close accepted: {other:?}"),
        }
        match parse_close_payload(&[0x03, 0xe9, 0xff]) {
            Err(ProtocolError::InvalidUtf8) => {}
            other => unreachable!("bad close reason accepted: {other:?}"),
        }
        // 1005 must not appear on the wire.
        match parse_close_payload(&[0x03, 0xed, b'o', b'k']) {
            Err(ProtocolError::Frame { reason, .. }) => {
                assert!(reason.contains("not valid on the wire"))
            }
            other => unreachable!("1005 accepted: {other:?}"),
        }
        match parse_close_payload(&[0x11, 0x39, b'o', b'k']) {
            Ok(Some((4409, reason))) => assert_eq!(reason, "ok"),
            other => unreachable!("4409 rejected: {other:?}"),
        }
    }

    #[test]
    fn server_writer_roundtrips_through_the_parser_via_masking() {
        // The writer produces unmasked frames; simulate a client by
        // re-masking with a known key and parsing back (the parser's
        // unmask must invert the writer's mask exactly).
        for (fin, opcode, payload) in [
            (true, Opcode::Text, b"{}".as_slice()),
            (true, Opcode::Pong, b"abc"),
            (false, Opcode::Binary, &[0u8; 200]),
        ] {
            let mut written = Vec::new();
            match write_frame(&mut written, fin, opcode, payload) {
                Ok(()) => {}
                Err(err) => unreachable!("write failed: {err}"),
            }
            // The masked writer must produce the same wire shape.
            let mut client_wire = Vec::new();
            let mask = [0xa5, 0x5a, 0x0f, 0xf0];
            match write_masked_frame(&mut client_wire, fin, opcode, payload, mask) {
                Ok(()) => {}
                Err(err) => unreachable!("masked write failed: {err}"),
            }
            let head_len = client_wire.len() - payload.len() - 4;
            let mut masked = Vec::with_capacity(written.len() + 4);
            masked.extend_from_slice(&written[..head_len]);
            masked[1] |= 0x80;
            masked.extend_from_slice(&mask);
            let mut body = written[head_len..].to_vec();
            apply_mask(&mut body, mask);
            masked.extend_from_slice(&body);
            assert_eq!(masked, client_wire, "hand-masked and write_masked_frame agree");
            let mut stream = reader(&masked);
            match read_frame(&mut stream, 1 << 20) {
                Ok(Some(frame)) => {
                    assert_eq!(frame.fin, fin);
                    assert_eq!(frame.opcode, opcode);
                    assert_eq!(frame.payload, payload);
                }
                other => unreachable!("roundtrip failed: {other:?}"),
            }
        }
    }
}
