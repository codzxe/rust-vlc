//! The ASF Data Object packet walk (card 6.ASF-b).
//!
//! The packet/payload bit grammar this module parses is the
//! ffmpeg-7.1.5-verified subset (the spec-acquisition gate never
//! opened — see the worklog's [VERIFY-P0] verdict; every layout
//! below was byte-decoded against live muxer output and closes each
//! packet exactly):
//!
//! - **Error correction field**: one flags byte — high bit set means
//!   an EC-type block of `(byte & 0x0F)` bytes follows (ffmpeg writes
//!   `82` + 2 zero bytes); a clear high bit means that many EC data
//!   bytes. Either way the walk skips `1 + (byte & 0x0F)` bytes.
//! - **Packet flags** (`f1`): bits 1-0 are the observed constant
//!   `01` on every fixture (packet extent comes from the File
//!   Properties min/max packet size, not an in-packet length field)
//!   — any other value is refused typed, never guessed. The padding
//!   length field: bit `0x10` = a 2-byte field follows the property
//!   flags (ffmpeg writes `11`), bit `0x08` = a 1-byte field (ffmpeg
//!   writes `09` — measured on the same corpus; every packet closes
//!   exactly under this reading), both clear = no padding; both set
//!   is an unobserved shape, refused.
//! - **Property flags** (`f2`): pairs of length-type bits for the
//!   per-payload fields — bits 1-0 replicated-data length (01=byte),
//!   bits 3-2 offset-into-media-object (11=dword, 00=absent),
//!   bits 5-4 media-object number (01=byte), bits 7-6 payload length
//!   (01=word). ffmpeg writes the constant `5d`.
//! - **Packet header tail**: send time (DWORD, ms) · duration
//!   (WORD, ms) · payload-count byte (bit 7 set, count in the low 6
//!   bits — ffmpeg writes `81`/`82`).
//! - **Payload**: stream number (1 byte; bit 7 = keyframe) · media
//!   object number · offset into media object · replicated-data
//!   length · replicated data · payload length · payload bytes.
//!   Replicated data of 8 bytes is `[media object size DWORD]
//!   [presentation time DWORD, ms, preroll-offset]` (order verified
//!   against ffprobe pts); length 1 is the compressed form, which no
//!   fixture exercises — refused typed, not guessed.
//! - **Padding**: `padding length` zero bytes at the packet tail
//!   (ffmpeg pads with 0x00).
//!
//! Media objects span packets as fragments keyed by (stream, media
//! object number, offset); [`ObjectAssembler`] reassembles them under
//! a hard memory ceiling (MP §12) — the demuxer hands the decoder one
//! completed object per [`Packet`](plugin_api packet).
//!
//! ## House rules
//!
//! Std-only, no `unsafe`, no `unwrap`/`expect`/`panic!` in this
//! non-test code; every length is checked against the packet slice
//! before it is trusted (the parse never indexes past its input).

use media_types::EngineError;

use super::error;

/// Hard ceiling on one reassembled media object (MP §12): the
/// replicated-data size field is container-trust input, never a
/// promise to allocate.
pub const MAX_OBJECT_BYTES: u64 = 32 << 20;

/// One parsed payload header + the byte range of its data inside the
/// packet slice (the demuxer copies out only what reassembly needs).
#[derive(Debug, Clone, Copy)]
pub struct ParsedPayload {
    /// The payload's stream number (1-based; joins the header's
    /// Stream Properties records).
    pub stream: u16,
    /// The keyframe bit from the stream-number byte.
    pub keyframe: bool,
    /// The media object number (per-stream fragment group key).
    pub media_object: u32,
    /// This fragment's offset into its media object.
    pub offset_into_object: u32,
    /// The 8-byte replicated data, when present:
    /// `(media object size, presentation time in ms, preroll-offset)`.
    pub replicated: Option<(u32, u32)>,
    /// The payload's declared data length.
    pub length: u32,
    /// `(start, end)` of the payload data inside the packet slice.
    pub data_range: (usize, usize),
}

/// One parsed data packet (headers only; payload bytes stay borrowed
/// from the packet slice).
#[derive(Debug)]
pub struct ParsedPacket {
    /// The packet's send time (ms).
    pub send_time_ms: u32,
    /// The packet's duration (ms).
    pub duration_ms: u16,
    /// The packet's padding length (bytes at the tail).
    pub padding: u32,
    /// The declared payload count.
    pub payload_count: u8,
    /// The parsed payloads, in packet order.
    pub payloads: Vec<ParsedPayload>,
}

/// The length-type codes of the property-flags byte, as widths.
fn type_width(code: u8) -> Option<usize> {
    match code {
        1 => Some(1), // byte
        2 => Some(2), // word
        3 => Some(4), // dword
        _ => None,    // 0 = absent (only legal for the offset field)
    }
}

fn u16le(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn u32le(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// Parse one data packet from `bytes` (exactly one packet's worth).
///
/// The parse is total: every field is bounds-checked against the
/// slice and the packet must close exactly (payloads + padding sum to
/// the slice length) or the typed `demux.asf.packet.bad-length`
/// refusal comes back — a short or overlong packet is corruption,
/// never a guess.
///
/// # Errors
/// Typed `Corrupt` refusals (see `error`): `packet.bad-flags` for an
/// unverified flag shape, `packet.bad-length` for lengths that do not
/// close, `payload.bad-replication` for the unexercised compressed
/// form.
pub fn parse_packet(bytes: &[u8]) -> Result<ParsedPacket, EngineError> {
    eprintln!("TRACE parse_packet enter len={}", bytes.len());
    // Error-correction field: 1 flags byte + (flags & 0x0F) bytes.
    let mut pos = 0_usize;
    let Some(&ec) = bytes.first() else {
        return Err(error::packet_bad_flags(0));
    };
    pos += 1 + usize::from(ec & 0x0F);

    // Packet flags + property flags.
    if bytes.len() < pos + 2 {
        return Err(error::truncated(bytes.len() as u64 + 1, bytes.len() as u64));
    }
    let f1 = bytes[pos];
    let f2 = bytes[pos + 1];
    pos += 2;
    // The observed constant: bits 1-0 == 01 (packet extent rides the
    // FPO packet size). Anything else is an unverified shape.
    if f1 & 0x03 != 0x01 {
        return Err(error::packet_bad_flags(pos as u64));
    }
    // The padding-length field: 2-byte (0x10), 1-byte (0x08), or
    // absent — both forms measured on the corpus; both set at once
    // is an unobserved shape, refused.
    let padding = match f1 & 0x18 {
        0x00 => 0,
        0x08 => {
            let Some(&p) = bytes.get(pos) else {
                return Err(error::truncated(bytes.len() as u64 + 1, bytes.len() as u64));
            };
            pos += 1;
            u32::from(p)
        }
        0x10 => {
            if bytes.len() < pos + 2 {
                return Err(error::truncated(bytes.len() as u64 + 1, bytes.len() as u64));
            }
            let p = u32::from(u16le(bytes, pos));
            pos += 2;
            p
        }
        _ => return Err(error::packet_bad_flags(pos as u64)),
    };

    eprintln!("TRACE f1={f1:02x} f2={f2:02x} pos={pos} padding={padding}");
    let repl_width = type_width(f2 & 0x03).ok_or_else(|| error::packet_bad_flags(pos as u64))?;
    let offset_code = (f2 >> 2) & 0x03;
    let offset_width = if offset_code == 0 {
        0
    } else {
        type_width(offset_code).ok_or_else(|| error::packet_bad_flags(pos as u64))?
    };
    let mo_width = type_width((f2 >> 4) & 0x03).ok_or_else(|| error::packet_bad_flags(pos as u64))?;
    let plen_code = (f2 >> 6) & 0x03;
    let plen_width = if plen_code == 0 {
        0
    } else {
        type_width(plen_code).ok_or_else(|| error::packet_bad_flags(pos as u64))?
    };

    // Send time (DWORD, ms) + duration (WORD, ms).
    if bytes.len() < pos + 6 {
        return Err(error::truncated(bytes.len() as u64 + 1, bytes.len() as u64));
    }
    let send_time_ms = u32le(bytes, pos);
    pos += 4;
    let duration_ms = u16le(bytes, pos);
    pos += 2;

    // Payload-count byte (bit 7 set on every fixture; count low 6).
    let Some(&count_byte) = bytes.get(pos) else {
        return Err(error::truncated(bytes.len() as u64 + 1, bytes.len() as u64));
    };
    pos += 1;
    if count_byte & 0x80 == 0 {
        return Err(error::packet_bad_flags(pos as u64));
    }
    let payload_count = count_byte & 0x3F;

    eprintln!("TRACE hdr end pos={pos} send={send_time_ms} dur={duration_ms} count={payload_count}");
    let mut payloads = Vec::with_capacity(payload_count as usize);
    for _ in 0..payload_count {
        // Stream number + keyframe bit.
        let Some(&stream_byte) = bytes.get(pos) else {
            return Err({ let e = error::packet_bad_length(pos as u64); eprintln!("TRACE bad-length pos={pos}"); e });
        };
        pos += 1;
        let stream = u16::from(stream_byte & 0x7F);
        let keyframe = stream_byte & 0x80 != 0;

        // Media object number.
        let Some(mo) = read_field(bytes, &mut pos, mo_width) else {
            return Err({ let e = error::packet_bad_length(pos as u64); eprintln!("TRACE bad-length pos={pos}"); e });
        };
        // Offset into media object.
        let Some(offset) = read_field(bytes, &mut pos, offset_width) else {
            return Err({ let e = error::packet_bad_length(pos as u64); eprintln!("TRACE bad-length pos={pos}"); e });
        };
        // Replicated-data length.
        let Some(repl_len) = read_field(bytes, &mut pos, repl_width) else {
            return Err({ let e = error::packet_bad_length(pos as u64); eprintln!("TRACE bad-length pos={pos}"); e });
        };
        // Replicated data: 8 bytes = (object size, presentation time).
        let mut replicated = None;
        let repl_len = repl_len as usize;
        if repl_len == 8 {
            if bytes.len() < pos + 8 {
                return Err({ let e = error::packet_bad_length(pos as u64); eprintln!("TRACE bad-length pos={pos}"); e });
            }
            replicated = Some((u32le(bytes, pos), u32le(bytes, pos + 4)));
            pos += 8;
        } else if repl_len == 1 {
            // The compressed-payload form — no fixture exercises it
            // (ffmpeg never writes it); refused rather than guessed.
            return Err(error::payload_bad_replication(pos as u64));
        } else if repl_len != 0 {
            // Non-8 replication blocks are legal containers for
            // stream-extension data we do not interpret; skip them.
            if bytes.len() < pos + repl_len {
                return Err({ let e = error::packet_bad_length(pos as u64); eprintln!("TRACE bad-length pos={pos}"); e });
            }
            pos += repl_len;
        }

        // Payload length (absent for the final payload only when the
        // property flags say so; ffmpeg always writes the word).
        eprintln!("TRACE payload parsed stream={stream} mo={mo} offset={offset} repl_len_r={repl_len} pos={pos}");
        let length = if plen_width == 0 {
            let rest = bytes.len().saturating_sub(pos + padding as usize);
            u32::try_from(rest).unwrap_or(u32::MAX)
        } else {
            let Some(len) = read_field(bytes, &mut pos, plen_width) else {
                return Err({ let e = error::packet_bad_length(pos as u64); eprintln!("TRACE bad-length pos={pos}"); e });
            };
            len
        };
        let start = pos;
        let end = pos.checked_add(length as usize).ok_or_else(|| error::packet_bad_length(pos as u64))?;
        if end > bytes.len().saturating_sub(padding as usize) {
            return Err({ let e = error::packet_bad_length(pos as u64); eprintln!("TRACE bad-length pos={pos}"); e });
        }
        pos = end;
        payloads.push(ParsedPayload {
            stream,
            keyframe,
            media_object: mo as u32,
            offset_into_object: offset as u32,
            replicated,
            length,
            data_range: (start, end),
        });
    }

    // The packet must close exactly: payloads + padding = slice.
    if pos + padding as usize != bytes.len() {
        return Err({ let e = error::packet_bad_length(pos as u64); eprintln!("TRACE bad-length pos={pos}"); e });
    }

    Ok(ParsedPacket {
        send_time_ms,
        duration_ms,
        padding,
        payload_count,
        payloads,
    })
}

/// Read one length-typed field at `pos`, advancing it.
fn read_field(bytes: &[u8], pos: &mut usize, width: usize) -> Option<u32> {
    if width == 0 {
        return Some(0);
    }
    if bytes.len() < *pos + width {
        return None;
    }
    let value = match width {
        1 => u32::from(bytes[*pos]),
        2 => u32::from(u16le(bytes, *pos)),
        _ => u32le(bytes, *pos),
    };
    *pos += width;
    Some(value)
}

/// One media object under reassembly (the bounded pool is one slot
/// per stream — a stream carries one open object at a time in every
/// observed file).
struct PendingObject {
    number: u32,
    size: u32,
    presentation_time_ms: u32,
    keyframe: bool,
    buffer: Vec<u8>,
}

/// Fragment reassembly per stream (the "repacking" seam: the decoder
/// gets one media object per packet, never fragments).
#[derive(Default)]
pub struct ObjectAssembler {
    slots: Vec<Option<PendingObject>>,
}

impl ObjectAssembler {
    /// An assembler with `stream_count` slots (one per header stream).
    pub fn new(stream_count: usize) -> ObjectAssembler {
        ObjectAssembler {
            slots: (0..stream_count).map(|_| None).collect(),
        }
    }

    /// Reset every slot (seek re-anchor clears half-delivered objects).
    pub fn clear(&mut self) {
        for slot in &mut self.slots {
            *slot = None;
        }
    }

    /// Feed one payload's bytes; `Some(bytes)` comes back when the
    /// object completed — the exact replicated-data size worth of
    /// concatenated fragments, in offset order.
    ///
    /// A new media-object number abandons any incomplete predecessor
    /// on that stream (a gap the container declares by moving on —
    /// counted by the demuxer's damage observability, never invented
    /// into the stream).
    ///
    /// # Errors
    /// Typed refusals: `payload.oversized` when the object exceeds
    /// [`MAX_OBJECT_BYTES`], `packet.bad-length` when a fragment
    /// overruns the declared object size.
    pub fn feed(
        &mut self,
        slot: usize,
        stream: u16,
        media_object: u32,
        offset: u32,
        replicated: Option<(u32, u32)>,
        keyframe: bool,
        data: &[u8],
    ) -> Result<Option<(u32, bool, Vec<u8>)>, EngineError> {
        let Some(slot) = self.slots.get_mut(slot) else {
            // Unknown stream: the demuxer skips the payload before
            // this point; a bad index here is a caller bug, refused.
            let _ = stream;
            return Ok(None);
        };
        match slot {
            Some(pending) if pending.number == media_object => {
                // Continuation fragment (must land exactly at the end).
                let start = pending.buffer.len() as u32;
                if offset != start {
                    return Err(error::packet_bad_length(offset as u64));
                }
                let new_len = u64::from(start) + u64::from(data.len() as u32);
                if new_len > u64::from(pending.size) || new_len > MAX_OBJECT_BYTES {
                    return Err(error::payload_oversized(MAX_OBJECT_BYTES, new_len));
                }
                pending.buffer.extend_from_slice(data);
            }
            _ => {
                // First fragment (or a new object replacing a gap).
                let (size, time) = replicated
                    .ok_or_else(|| error::payload_bad_replication(media_object as u64))?;
                if u64::from(size) > MAX_OBJECT_BYTES {
                    return Err(error::payload_oversized(MAX_OBJECT_BYTES, u64::from(size)));
                }
                if offset != 0 {
                    return Err(error::packet_bad_length(offset as u64));
                }
                if data.len() as u64 > u64::from(size) {
                    return Err(error::packet_bad_length(offset as u64));
                }
                *slot = Some(PendingObject {
                    number: media_object,
                    size,
                    presentation_time_ms: time,
                    keyframe,
                    buffer: data.to_vec(),
                });
            }
        }
        let complete = slot
            .as_ref()
            .is_some_and(|p| p.buffer.len() as u64 == u64::from(p.size));
        if complete {
            // Statically Some (just checked); `take` moves it out.
            if let Some(p) = slot.take() {
                return Ok(Some((p.presentation_time_ms, p.keyframe, p.buffer)));
            }
        }
        Ok(None)
    }
}
