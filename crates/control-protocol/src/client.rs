//! The in-tree WebSocket test client (card 7.J1; doc 01 §2.3 (e)).
//!
//! A minimal RFC 6455 CLIENT (the mirror of [`crate::server`]'s server
//! side): masked writes, unmasked reads, the same in-tree SHA-1/base64
//! handshake. Its purpose is the conformance suites of this crate and
//! `apps/control-server` — driving the real server over a real
//! loopback socket (doc 10: no synthetic echo chambers; D-5). It is
//! deliberately NOT a general client: no TLS, no extensions, no
//! convenience beyond what a test needs — and it can write RAW BYTES
//! ([`TestClient::send_raw`]) so dirty-world frames (truncated
//! headers, bad masks, oversize claims) reach the server exactly as a
//! hostile peer would send them.
//!
//! Not part of any production path: `pub` because integration tests
//! in the app crate drive the same server through it.

use std::io::Read;
use std::net::TcpStream;

use crate::error::ProtocolError;
use crate::frame::Opcode;
use crate::frame::write_masked_frame;
use crate::http;
use crate::protocol::ServerFrame;

/// A loopback conformance client.
pub struct TestClient {
    stream: TcpStream,
    /// The nonce this client offered in the upgrade (kept so tests can
    /// verify the accept computation was truly key-derived).
    pub key: [u8; 16],
}

impl TestClient {
    /// Connect to `addr` and perform the RFC 6455 opening handshake.
    ///
    /// # Errors
    /// [`ProtocolError::Io`] on connect trouble;
    /// [`ProtocolError::HttpUpgrade`] when the server's 101/accept is
    /// not exactly what the RFC demands for our nonce.
    pub fn connect(addr: &str) -> Result<TestClient, ProtocolError> {
        let mut stream = TcpStream::connect(addr)
            .map_err(|source| ProtocolError::Io { what: "connecting the test client", source })?;
        // A locally-generated nonce (this is a TEST client; the value
        // only needs to be 16 varied bytes — the accept check pins the
        // derivation).
        let mut key = [0u8; 16];
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or_default();
        for (i, byte) in key.iter_mut().enumerate() {
            *byte = (nanos >> ((i % 8) * 8)) as u8 ^ (0x5a_u8.rotate_left(i as u32 % 8));
        }
        let request = http::client_request("/lumen-control", &key, addr);
        http::write_all(&mut stream, &request)?;
        let head = http::read_response_head(&mut stream)?;
        http::validate_server_response(&head, &key)?;
        Ok(TestClient { stream, key })
    }

    /// A per-call mask (RFC 6455 §5.3: clients pick a new key per
    /// frame; a time-mixed value is plenty for a loopback test
    /// client — the SERVER's strictness is what tests exercise).
    fn fresh_mask() -> [u8; 4] {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or_default();
        let entropy = nanos ^ (nanos << 11) ^ (nanos >> 7);
        [
            (entropy & 0xff) as u8,
            ((entropy >> 8) & 0xff) as u8,
            ((entropy >> 16) & 0xff) as u8 | 1,
            ((entropy >> 24) & 0xff) as u8,
        ]
    }

    /// Send one masked text frame with `payload` (JSON text).
    ///
    /// # Errors
    /// [`ProtocolError::Io`] on write failure.
    pub fn send_text(&mut self, payload: &str) -> Result<(), ProtocolError> {
        write_masked_frame(&mut self.stream, true, Opcode::Text, payload.as_bytes(), Self::fresh_mask())
    }

    /// Send one masked BINARY frame (dirty-world cases).
    ///
    /// # Errors
    /// [`ProtocolError::Io`] on write failure.
    pub fn send_binary(&mut self, payload: &[u8]) -> Result<(), ProtocolError> {
        write_masked_frame(&mut self.stream, true, Opcode::Binary, payload, Self::fresh_mask())
    }

    /// Send RAW BYTES as-is (malformed-frame conformance: truncated
    /// headers, wrong masks, oversized length claims).
    ///
    /// # Errors
    /// [`ProtocolError::Io`] on write failure.
    pub fn send_raw(&mut self, bytes: &[u8]) -> Result<(), ProtocolError> {
        http::write_all(&mut self.stream, bytes)
    }

    /// Send a masked ping; the server must answer with a pong carrying
    /// the same payload (RFC 6455 §5.5.2).
    ///
    /// # Errors
    /// [`ProtocolError::Io`] on write failure.
    pub fn ping(&mut self, payload: &[u8]) -> Result<(), ProtocolError> {
        write_masked_frame(&mut self.stream, true, Opcode::Ping, payload, Self::fresh_mask())
    }

    /// Send a close frame with `code` (RFC 6455 §5.5.1).
    ///
    /// # Errors
    /// [`ProtocolError::Io`] on write failure.
    pub fn close(&mut self, code: u16) -> Result<(), ProtocolError> {
        write_masked_frame(&mut self.stream, true, Opcode::Close, &code.to_be_bytes(), Self::fresh_mask())
    }

    /// Send one masked TEXT frame with an EXPLICIT mask (wire-byte
    /// reproducibility for fragmentation tests).
    ///
    /// # Errors
    /// [`ProtocolError::Io`] on write failure.
    pub fn send_text_masked(
        &mut self,
        fin: bool,
        opcode: Opcode,
        payload: &[u8],
        mask: [u8; 4],
    ) -> Result<(), ProtocolError> {
        write_masked_frame(&mut self.stream, fin, opcode, payload, mask)
    }

    /// Read one frame (server → client: unmasked). `Ok(None)` on EOF.
    ///
    /// # Errors
    /// Typed [`ProtocolError`] for malformed frames (a server bug —
    /// exactly what a conformance test wants to catch).
    pub fn read_frame(&mut self) -> Result<Option<Frame>, ProtocolError> {
        read_client_side_frame(&mut self.stream)
    }

    /// Read one frame and require it to be a text message with this
    /// exact payload.
    ///
    /// # Errors
    /// [`ProtocolError`] when the next frame is not that text message.
    pub fn expect_text(&mut self, payload: &str) -> Result<(), ProtocolError> {
        match self.read_frame()? {
            Some(frame) if frame.opcode == Opcode::Text && frame.fin => {
                let text = String::from_utf8(frame.payload)
                    .map_err(|_| ProtocolError::InvalidUtf8)?;
                if text == payload {
                    Ok(())
                } else {
                    Err(ProtocolError::BadEnvelope {
                        detail: format!("expected {payload:?}, got {text:?}"),
                    })
                }
            }
            Some(frame) => Err(ProtocolError::BadEnvelope {
                detail: format!("expected a final text frame, got {frame:?}"),
            }),
            None => Err(ProtocolError::Closed),
        }
    }

    /// Read one frame and require it to be a pong echoing `payload`.
    ///
    /// # Errors
    /// [`ProtocolError`] when the next frame is not that pong.
    pub fn expect_pong(&mut self, payload: &[u8]) -> Result<(), ProtocolError> {
        match self.read_frame()? {
            Some(frame) if frame.opcode == Opcode::Pong && frame.payload == payload => Ok(()),
            Some(frame) => Err(ProtocolError::BadEnvelope {
                detail: format!("expected a pong {:?}, got {frame:?}", payload),
            }),
            None => Err(ProtocolError::Closed),
        }
    }

    /// Read one close frame and return its code (`None` = no payload).
    ///
    /// # Errors
    /// [`ProtocolError`] when the next frame is not a close, or its
    /// payload violates §5.5.1.
    pub fn expect_close(&mut self) -> Result<Option<u16>, ProtocolError> {
        match self.read_frame()? {
            Some(frame) if frame.opcode == Opcode::Close => {
                crate::frame::parse_close_payload(&frame.payload).map(|parsed| parsed.map(|(code, _)| code))
            }
            Some(frame) => Err(ProtocolError::BadEnvelope {
                detail: format!("expected a close frame, got {frame:?}"),
            }),
            None => Err(ProtocolError::Closed),
        }
    }

    /// Read one text frame and parse it as a server application frame.
    ///
    /// # Errors
    /// [`ProtocolError`] on transport/parsing trouble.
    pub fn read_server_frame(&mut self) -> Result<Option<ServerFrame>, ProtocolError> {
        let Some(frame) = self.read_frame()? else {
            return Ok(None);
        };
        if frame.opcode != Opcode::Text || !frame.fin {
            return Err(ProtocolError::BadEnvelope {
                detail: format!("expected a final text frame, got {frame:?}"),
            });
        }
        let text = String::from_utf8(frame.payload).map_err(|_| ProtocolError::InvalidUtf8)?;
        serde_json::from_str(&text)
            .map(Some)
            .map_err(|err| ProtocolError::BadEnvelope { detail: err.to_string() })
    }
}

/// One frame as the client sees it (mirrors [`crate::frame::Frame`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Final-fragment flag.
    pub fin: bool,
    /// The opcode.
    pub opcode: Opcode,
    /// The (already unmasked — servers never mask) payload.
    pub payload: Vec<u8>,
}

/// Read one frame in the CLIENT role: the frame must arrive UNMASKED
/// (RFC 6455 §5.1: a masked server frame is a protocol violation the
/// client must fail on — mirrored strictness).
fn read_client_side_frame<R: Read>(stream: &mut R) -> Result<Option<Frame>, ProtocolError> {
    crate::frame::read_frame_unmasked(stream, 1 << 20).map(|option| {
        option.map(|frame| Frame { fin: frame.fin, opcode: frame.opcode, payload: frame.payload })
    })
}
