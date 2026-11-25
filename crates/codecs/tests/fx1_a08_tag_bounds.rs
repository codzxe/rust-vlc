//! FX1-A08 regression pins (wave FX1, FIX-PLAN §2 F0.4k+l): the two
//! crafted-input truncation windows the audit derived/executed.
//!
//! * `parse_vbri`'s fixed fields run **22 bytes past the `VBRI`
//!   magic**; the old `len < 14` guard admitted tail lengths
//!   14..22 and then indexed `rest[14]`..`rest[21]` — a slice panic
//!   on a truncated tag (W07-A04 F-001 S1, re-verified W20-A01 F6:
//!   "the guard admits len ∈ [14, ∞); `rest[14]` needs len ≥ 15;
//!   `rest[20]` needs len ≥ 21"). Every tail in the window must
//!   refuse typed `mp3.vbri.truncated`; the sibling Xing path
//!   refuses its truncations typed as well (no panic either side).
//! * The FLAC-in-Ogg second header's STREAMINFO **body is 34 bytes**
//!   (W13-A06 F-001 S1: the ogg host seeded the 38-byte packet
//!   verbatim — block header parsed AS body — and the shipped lane
//!   refused `flac.streaminfo.rate: sample rate 0`). The 34-byte
//!   body parses with the exact facts; a 33-byte body is
//!   typed-refused; the verbatim 38-byte packet parses as garbage —
//!   the audit's executed refusal code, pinned here so the packet is
//!   never handed to the decoder as the body.

use codecs::audio::flac::parse_streaminfo;
use codecs::audio::mp3::{parse_vbri, parse_xing};
use media_types::EngineError;

/// A first frame carrying `VBRI` at the MPEG-1 L3 128 kbps/44.1 kHz
/// stereo ancillary offset (36) with `tail` nonzero bytes after the
/// magic — the audit's crafted shape (scratch/W20-A01/vbri_repro.rs;
/// nonzero filler keeps the zero-totals gate satisfied so the tail
/// truncation is what the parser must survive).
fn vbri_frame_with_tail(tail: usize) -> Vec<u8> {
    let mut f = Vec::new();
    f.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x00]); // MPEG-1 L3 stereo
    f.extend_from_slice(&[0u8; 32]); // side info (stereo MPEG-1 = 32)
    f.extend_from_slice(b"VBRI");
    f.extend(std::iter::repeat_n(0x41u8, tail));
    f
}

/// A first frame carrying `Xing` at the same ancillary offset with a
/// flags word plus `tail` bytes of claim.
fn xing_frame_with_tail(flags: u32, tail: usize) -> Vec<u8> {
    let mut f = Vec::new();
    f.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
    f.extend_from_slice(&[0u8; 32]);
    f.extend_from_slice(b"Xing");
    f.extend_from_slice(&flags.to_be_bytes());
    f.extend(std::iter::repeat_n(0x41u8, tail));
    f
}

/// The audit's crafted STREAMINFO body (scratch/W20-A01/mkoga.py):
/// 48 kHz, stereo, 16-bit, fixed 4096 block size, 4096 total samples.
fn streaminfo_body() -> [u8; 34] {
    let mut body = [0u8; 34];
    body[0..2].copy_from_slice(&4096u16.to_be_bytes());
    body[2..4].copy_from_slice(&4096u16.to_be_bytes());
    let packed: u64 = (48_000u64 << 44) | (1 << 41) | (15 << 36) | 4096;
    body[10..18].copy_from_slice(&packed.to_be_bytes());
    body
}

/// The typed refusal code of an outcome (the parse itself must never
/// panic — a regressed guard aborts the test process before this).
fn code_of(err: &EngineError) -> &str {
    err.context.code.as_str()
}

#[test]
fn fx1_a08_truncated_vbri_and_xing_tails_refuse_typed_never_panic() {
    // The whole panic window plus the sub-14 window the old guard
    // already refused: every tail length below the fixed-field span
    // (22 bytes) must come back as the typed truncation.
    for tail in 0..22usize {
        match parse_vbri(&vbri_frame_with_tail(tail), 0) {
            Ok(Some(_)) => unreachable!("a {tail}-byte VBRI tail cannot parse"),
            Ok(None) => unreachable!("the VBRI magic is present, so not None"),
            Err(e) => assert_eq!(
                code_of(&e),
                "mp3.vbri.truncated",
                "tail {tail} must refuse typed, never index"
            ),
        }
    }
    // The top of the window exactly: 22 bytes of tail parse the fixed
    // fields and then refuse the (nonzero, hence oversized) TOC
    // typed — the pre-existing behavior, unchanged by the guard fix.
    match parse_vbri(&vbri_frame_with_tail(22), 0) {
        Ok(Some(_)) => unreachable!("a 22-byte tail still owes a TOC"),
        Ok(None) => unreachable!("the VBRI magic is present, so not None"),
        Err(e) => assert_eq!(code_of(&e), "mp3.vbri.toc_truncated"),
    }

    // The sibling Xing path: flags claiming fields the tail does not
    // carry refuse typed (bounds-checked `take`, no panic).
    match parse_xing(&xing_frame_with_tail(0x0001, 2), 0) {
        Ok(Some(_)) => unreachable!("a 2-byte frames field cannot parse"),
        Ok(None) => unreachable!("the Xing magic is present, so not None"),
        Err(e) => assert_eq!(code_of(&e), "mp3.xing.frames_truncated"),
    }
    match parse_xing(&xing_frame_with_tail(0x0004, 50), 0) {
        Ok(Some(_)) => unreachable!("a 50-byte TOC cannot parse"),
        Ok(None) => unreachable!("the Xing magic is present, so not None"),
        Err(e) => assert_eq!(code_of(&e), "mp3.xing.toc_truncated"),
    }
}

#[test]
fn fx1_a08_ogg_flac_second_header_body_is_exactly_34_bytes() {
    let body = streaminfo_body();

    // The 34-byte body — what the ogg host must seed after stripping
    // the second header packet's 4-byte metadata-block prefix —
    // parses with the exact facts.
    let facts = match parse_streaminfo(&body) {
        Ok(facts) => facts,
        Err(e) => unreachable!("the 34-byte body must parse: {e}"),
    };
    assert_eq!(facts.sample_rate, 48_000);
    assert_eq!(facts.channels, 2);
    assert_eq!(facts.bits_per_sample, 16);
    assert_eq!(facts.total_samples, Some(4096));
    assert_eq!((facts.min_block_size, facts.max_block_size), (4096, 4096));

    // A 33-byte body is typed-refused: the spec's own 34.
    match parse_streaminfo(&body[..33]) {
        Ok(_) => unreachable!("a 33-byte body must not parse"),
        Err(e) => assert_eq!(code_of(&e), "flac.streaminfo.short"),
    }

    // The verbatim 38-byte second header packet (4-byte block prefix
    // + body) parses as GARBAGE — the pre-fix host seed: the prefix
    // shifts every field and the rate reads 0, W13-A06's executed
    // refusal through the shipped `lumen open` path. Pin it so the
    // packet is stripped before it is ever handed as the body.
    let mut packet = vec![0x80, 0x00, 0x00, 34]; // last+STREAMINFO, len 34
    packet.extend_from_slice(&body);
    assert_eq!(packet.len(), 38);
    match parse_streaminfo(&packet) {
        Ok(_) => unreachable!("the verbatim 38-byte packet must not parse as a body"),
        Err(e) => assert_eq!(code_of(&e), "flac.streaminfo.rate"),
    }
}
