//! The AIFF demuxer's unit tests (card 5.A2): COMM/SSND framing, the
//! 80-bit rate, MARK/INST skip-tolerance, and the AC error paths —
//! truncated head/chunk, declared > actual, zero SSND, unsupported
//! compression/depth, non-normalizable rate. The ffmpeg-generated
//! fixtures get their differential + mutated-family suite in
//! `crates/demux/tests/wav_aiff_fixtures.rs`.

use media_types::time::{Stamp, TimeBase, TimeSpan, Timestamp};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};

use super::AiffDemuxer;
use super::chunks::{self, CommChunk};
use super::error;
use super::test_support::{extended80, mem_stream, minimal_aiff};

/// Drain the demuxer, returning (payload bytes, events).
fn drain(aiff: &mut AiffDemuxer) -> (Vec<u8>, Vec<DemuxEvent>) {
    let mut bytes = Vec::new();
    let mut events = Vec::new();
    loop {
        match aiff.next_event() {
            Ok(DemuxEvent::Packet(packet)) => bytes.extend_from_slice(&packet.data),
            Ok(event) => events.push(event),
            Err(err) => unreachable!("drain failed: {err}"),
        }
        if events.last() == Some(&DemuxEvent::EndOfStream) {
            break;
        }
    }
    (bytes, events)
}

fn started_aiff(bytes: std::sync::Arc<[u8]>) -> AiffDemuxer {
    let mut aiff = AiffDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("new: {e}"));
    let _ = aiff.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    aiff
}

/// A classic-AIFF file body builder with explicit chunks (the
/// odd-shape tests).
fn form(chunks_out: &mut Vec<u8>, form_type: &[u8; 4]) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(b"FORM");
    b.extend_from_slice(&0u32.to_be_bytes());
    b.extend_from_slice(form_type);
    b.append(chunks_out);
    let form_size = (b.len() - 8) as u32;
    b[4..8].copy_from_slice(&form_size.to_be_bytes());
    b
}

fn comm_chunk(channels: u16, frames: u32, bits: u16, rate: f64, tail: &[u8]) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&channels.to_be_bytes());
    payload.extend_from_slice(&frames.to_be_bytes());
    payload.extend_from_slice(&bits.to_be_bytes());
    payload.extend_from_slice(&match extended80(rate) {
        Ok(ext) => ext,
        Err(e) => unreachable!("rate: {e}"),
    });
    payload.extend_from_slice(tail);
    let mut b = Vec::new();
    b.extend_from_slice(b"COMM");
    b.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    b.extend_from_slice(&payload);
    if payload.len() % 2 == 1 {
        b.push(0);
    }
    b
}

fn ssnd_chunk(offset: u32, block_size: u32, data: &[u8], declared_extra: i64) -> Vec<u8> {
    let declared = (data.len() as i64 + 8 + declared_extra).max(0) as u32;
    let mut b = Vec::new();
    b.extend_from_slice(b"SSND");
    b.extend_from_slice(&declared.to_be_bytes());
    b.extend_from_slice(&offset.to_be_bytes());
    b.extend_from_slice(&block_size.to_be_bytes());
    b.extend_from_slice(data);
    if b.len() % 2 == 1 {
        b.push(0);
    }
    b
}

fn mark_and_inst_chunks() -> Vec<u8> {
    // MARK: 1 marker (id 1, position 0, pstring "loop").
    let mut mark = Vec::new();
    mark.extend_from_slice(&1u16.to_be_bytes());
    mark.extend_from_slice(&1u16.to_be_bytes());
    mark.extend_from_slice(&0u32.to_be_bytes());
    mark.push(4);
    mark.extend_from_slice(b"loop");
    mark.push(0); // pstring pad
    let mut b = Vec::new();
    b.extend_from_slice(b"MARK");
    b.extend_from_slice(&(mark.len() as u32).to_be_bytes());
    b.extend_from_slice(&mark);
    // INST: the 20-byte fixed body (baseNote…gain) + no loops.
    let inst = [0u8; 20];
    b.extend_from_slice(b"INST");
    b.extend_from_slice(&(inst.len() as u32).to_be_bytes());
    b.extend_from_slice(&inst);
    b
}

#[test]
fn drains_classic_pcm16be_whole_frames() {
    let samples: Vec<u8> = (0..12u8).collect();
    let mut aiff = started_aiff(minimal_aiff(2, 8_000, 16, &samples));
    let (bytes, events) = drain(&mut aiff);
    assert_eq!(bytes, samples);
    assert_eq!(events, vec![DemuxEvent::EndOfStream]);
    let comm = aiff.comm().unwrap_or_else(|| unreachable!("comm"));
    assert_eq!(comm.channels, 2);
    assert_eq!(comm.sample_rate, 8_000);
    assert_eq!(comm.sample_size, 16);
    assert_eq!(aiff.comm_chunk_bytes().map_or(0, <[u8]>::len), 18);
    let track = &aiff.tracks.clone().unwrap_or_else(|| unreachable!())[0];
    assert_eq!(track.codec.as_str(), "pcm_s16be");
    assert_eq!(aiff.damage_total(), 0);
    assert_eq!(aiff.ssnd_offset_block(), Some((0, 0)));
}

#[test]
fn big_endian_samples_arrive_verbatim() {
    // One frame of s16be: 0x1234 — the payload must be untouched
    // (byte-order normalization is the codec's job, not the
    // demuxer's).
    let mut aiff = started_aiff(minimal_aiff(1, 44_100, 16, &[0x12, 0x34]));
    match aiff.next_event() {
        Ok(DemuxEvent::Packet(p)) => assert_eq!(p.data.as_ref(), &[0x12, 0x34]),
        other => unreachable!("packet expected, got {other:?}"),
    }
    match aiff.next_event() {
        Ok(DemuxEvent::EndOfStream) => {}
        Ok(event) => unreachable!("unexpected {event:?}"),
        Err(err) => unreachable!("next: {err}"),
    }
}

#[test]
fn pts_ladder_and_span_count_sample_frames() {
    let samples = vec![0u8; 300 * 4];
    let mut aiff = started_aiff(minimal_aiff(2, 8_000, 16, &samples));
    match aiff.next_event() {
        Ok(DemuxEvent::Packet(p)) => match p.pts {
            Stamp::Present(ts) => assert_eq!(ts.ticks, 0),
            Stamp::Missing => unreachable!("pts must be present"),
        },
        other => unreachable!("packet expected, got {other:?}"),
    }
    let track = &aiff.tracks.clone().unwrap_or_else(|| unreachable!())[0];
    match track.timing.span.end {
        Stamp::Present(ts) => assert_eq!(ts.ticks, 300),
        Stamp::Missing => unreachable!("span end must be present"),
    }
    assert_eq!(
        track.timing.base,
        TimeBase::new(1, 8_000).unwrap_or_else(|e| unreachable!("{e}"))
    );
    // The pool settled at the quantum (the D-1 observable).
    assert_eq!(aiff.pool_bytes(), 1_200);
}

#[test]
fn mark_and_inst_chunks_are_skip_tolerant() {
    let mut body = Vec::new();
    body.extend_from_slice(&comm_chunk(1, 2, 16, 44_100.0, &[]));
    body.extend_from_slice(&mark_and_inst_chunks());
    body.extend_from_slice(&ssnd_chunk(0, 0, &[0x11, 0x22, 0x33, 0x44], 0));
    let file = form(&mut body, b"AIFF");
    let mut aiff = started_aiff(std::sync::Arc::from(file));
    let (bytes, events) = drain(&mut aiff);
    assert_eq!(bytes, vec![0x11, 0x22, 0x33, 0x44]);
    assert_eq!(events, vec![DemuxEvent::EndOfStream], "no damage");
}

#[test]
fn ssnd_offset_skips_into_the_sound_data() {
    // offset 2: the first 2 sound-area bytes are skipped (the offset
    // semantics — sound starts at SSND payload + 8 + offset), the
    // rest demuxes.
    let mut body = Vec::new();
    body.extend_from_slice(&comm_chunk(1, 1, 16, 44_100.0, &[]));
    body.extend_from_slice(&ssnd_chunk(2, 0, &[0xAA, 0xBB, 0x11, 0x22], 0));
    let file = form(&mut body, b"AIFF");
    let mut aiff = started_aiff(std::sync::Arc::from(file));
    assert_eq!(aiff.ssnd_offset_block(), Some((2, 0)));
    let (bytes, events) = drain(&mut aiff);
    assert_eq!(bytes, vec![0x11, 0x22]);
    assert_eq!(events, vec![DemuxEvent::EndOfStream]);
}

#[test]
fn odd_comm_payload_takes_its_pad() {
    // An AIFC COMM with a 2-char pstring name: 18 + 4 + 1 + 2 = 25
    // bytes (odd) + 1 pad — the walk must land SSND past the pad.
    let mut tail = Vec::new();
    tail.extend_from_slice(b"sowt");
    tail.push(2);
    tail.extend_from_slice(b"tw");
    let mut body = Vec::new();
    body.extend_from_slice(&comm_chunk(1, 2, 16, 22_050.0, &tail));
    body.extend_from_slice(&ssnd_chunk(0, 0, &[0x01, 0x02, 0x03, 0x04], 0));
    let file = form(&mut body, b"AIFC");
    let mut aiff = started_aiff(std::sync::Arc::from(file));
    let comm = aiff.comm().unwrap_or_else(|| unreachable!("comm"));
    assert_eq!(
        comm.compression.map_or("", |(_, c)| c.descriptor()),
        "pcm_s16le"
    );
    let (bytes, _) = drain(&mut aiff);
    assert_eq!(bytes, vec![0x01, 0x02, 0x03, 0x04]);
}

#[test]
fn aifc_compression_vocabulary_demuxes() {
    for (ctype, bits, descriptor) in [
        (b"NONE", 24u16, "pcm_s24be"),
        (b"sowt", 16, "pcm_s16le"),
        (b"fl32", 32, "pcm_f32be"),
        (b"fl64", 64, "pcm_f64be"),
    ] {
        let mut tail = Vec::new();
        tail.extend_from_slice(ctype);
        tail.push(0); // empty pstring
        let mut body = Vec::new();
        body.extend_from_slice(&comm_chunk(2, 1, bits, 48_000.0, &tail));
        let frame_bytes = match descriptor {
            "pcm_s24be" => 6,
            "pcm_f64be" => 16,
            _ => 8,
        };
        body.extend_from_slice(&ssnd_chunk(0, 0, &vec![0x5Au8; frame_bytes], 0));
        let file = form(&mut body, b"AIFC");
        let mut aiff = started_aiff(std::sync::Arc::from(file));
        let track = &aiff.tracks.clone().unwrap_or_else(|| unreachable!())[0];
        assert_eq!(track.codec.as_str(), descriptor, "{}", descriptor);
        let (bytes, _) = drain(&mut aiff);
        assert_eq!(bytes.len(), frame_bytes, "{descriptor}");
    }
}

#[test]
fn truncated_head_is_typed() {
    let mut aiff =
        AiffDemuxer::new(mem_stream(b"FORM".to_vec()).unwrap_or_else(|e| unreachable!("{e}")))
            .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match aiff.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.aiff.truncated");

    // A FORM that is not AIFF/AIFC.
    let mut aiff = AiffDemuxer::new(
        mem_stream(b"FORM\x00\x00\x00\x00WAVEjunk".to_vec())
            .unwrap_or_else(|e| unreachable!("{e}")),
    )
    .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match aiff.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.aiff.not-aiff");
}

#[test]
fn truncated_comm_is_typed_truncation() {
    let full = minimal_aiff(1, 44_100, 16, &[0x12, 0x34]);
    // Cut 4 bytes into COMM's payload (head is 12, chunk header 8).
    let cut: std::sync::Arc<[u8]> = full[..24].into();
    let mut aiff = AiffDemuxer::new(mem_stream(cut).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match aiff.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.aiff.truncated");
}

#[test]
fn declared_ssnd_past_actual_delivers_prefix_then_truncated() {
    // SSND declares 2 more bytes than the file carries.
    let mut body = Vec::new();
    body.extend_from_slice(&comm_chunk(1, 2, 16, 44_100.0, &[]));
    body.extend_from_slice(&ssnd_chunk(0, 0, &[0x11, 0x22, 0x33, 0x44], 2));
    let mut file = form(&mut body, b"AIFF");
    // Shrink the FORM declaration to the real size (the SSND lie
    // stays): the walk clamps to the stream.
    let real = (file.len() - 8) as u32;
    file[4..8].copy_from_slice(&real.to_be_bytes());
    let mut aiff = started_aiff(std::sync::Arc::from(file));
    match aiff.next_event() {
        Ok(DemuxEvent::Recovered { skipped, reason }) => {
            assert_eq!(skipped, 2);
            assert_eq!(reason, "ssnd-extends-past-stream");
        }
        other => unreachable!("damage record first, got {other:?}"),
    }
    match aiff.next_event() {
        Ok(DemuxEvent::Packet(p)) => assert_eq!(p.data.as_ref(), &[0x11, 0x22, 0x33, 0x44]),
        other => unreachable!("prefix packet expected, got {other:?}"),
    }
    let err = match aiff.next_event() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.aiff.truncated");
}

#[test]
fn missing_comm_and_ssnd_are_typed() {
    // FORM AIFF with only an INST chunk: no COMM.
    let mut body = mark_and_inst_chunks();
    let file = form(&mut body, b"AIFF");
    let mut aiff = AiffDemuxer::new(mem_stream(file).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match aiff.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.aiff.no-comm");

    // COMM but no SSND.
    let mut body = comm_chunk(1, 4, 16, 44_100.0, &[]);
    let file = form(&mut body, b"AIFF");
    let mut aiff = AiffDemuxer::new(mem_stream(file).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match aiff.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.aiff.no-ssnd");
}

#[test]
fn zero_or_offset_past_ssnd_is_no_ssnd() {
    // An SSND whose offset runs past the declared sound bytes: no
    // sound at all.
    let mut body = Vec::new();
    body.extend_from_slice(&comm_chunk(1, 1, 16, 44_100.0, &[]));
    body.extend_from_slice(&ssnd_chunk(16, 0, &[0x11, 0x22], 0));
    let file = form(&mut body, b"AIFF");
    let mut aiff = AiffDemuxer::new(mem_stream(file).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match aiff.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.aiff.no-ssnd");
}

#[test]
fn comm_declarations_disagreeing_with_ssnd_are_damage() {
    // COMM says 1 frame; SSND carries 3.
    let mut body = Vec::new();
    body.extend_from_slice(&comm_chunk(1, 1, 16, 44_100.0, &[]));
    body.extend_from_slice(&ssnd_chunk(0, 0, &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06], 0));
    let file = form(&mut body, b"AIFF");
    let mut aiff = started_aiff(std::sync::Arc::from(file));
    match aiff.next_event() {
        Ok(DemuxEvent::Recovered { skipped, reason }) => {
            assert_eq!(skipped, 4, "2 frames × 2 bytes");
            assert_eq!(reason, "comm-frames-short-of-ssnd");
        }
        other => unreachable!("damage record first, got {other:?}"),
    }
    let (bytes, _) = drain(&mut aiff);
    assert_eq!(bytes, vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
}

#[test]
fn non_normalizable_rate_is_structural() {
    // A denormal rate (exponent 0, nonzero mantissa).
    let mut body = Vec::new();
    let mut comm = comm_chunk(1, 1, 16, 44_100.0, &[]);
    comm[8 + 8] = 0x00; // exponent hi
    comm[8 + 9] = 0x00; // exponent lo
    comm[8 + 17] = 0x01; // mantissa tail
    body.extend_from_slice(&comm);
    body.extend_from_slice(&ssnd_chunk(0, 0, &[0x11, 0x22], 0));
    let file = form(&mut body, b"AIFF");
    let mut aiff = AiffDemuxer::new(mem_stream(file).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match aiff.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.aiff.bad-structure");
}

#[test]
fn unsupported_depths_are_typed() {
    // Classic 8-bit (signed — no vocabulary).
    let mut body = Vec::new();
    body.extend_from_slice(&comm_chunk(1, 1, 8, 44_100.0, &[]));
    body.extend_from_slice(&ssnd_chunk(0, 0, &[0x00], 0));
    let file = form(&mut body, b"AIFF");
    let mut aiff = AiffDemuxer::new(mem_stream(file).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match aiff.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.aiff.unsupported-compression");

    // 12-bit NONE (AIFC).
    let mut tail = Vec::new();
    tail.extend_from_slice(b"NONE");
    tail.push(0);
    let mut body = Vec::new();
    body.extend_from_slice(&comm_chunk(1, 1, 12, 44_100.0, &tail));
    body.extend_from_slice(&ssnd_chunk(0, 0, &[0x00, 0x00], 0));
    let file = form(&mut body, b"AIFC");
    let mut aiff = AiffDemuxer::new(mem_stream(file).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match aiff.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.aiff.unsupported-compression");
}

#[test]
fn trailing_partial_frame_is_damaged_and_dropped() {
    let mut body = Vec::new();
    body.extend_from_slice(&comm_chunk(1, 2, 16, 8_000.0, &[]));
    body.extend_from_slice(&ssnd_chunk(0, 0, &[1, 0, 2, 0, 9], 0));
    let file = form(&mut body, b"AIFF");
    let mut aiff = started_aiff(std::sync::Arc::from(file));
    let (bytes, events) = drain(&mut aiff);
    assert_eq!(bytes, vec![1, 0, 2, 0]);
    assert!(
        events.iter().any(|e| matches!(
            *e,
            DemuxEvent::Recovered {
                reason: "ssnd-not-frame-aligned",
                ..
            }
        )),
        "{events:?}"
    );
}

#[test]
fn trait_misuse_is_typed_not_a_panic() {
    let mut aiff = AiffDemuxer::new(
        mem_stream(minimal_aiff(1, 44_100, 16, &[0x12, 0x34]))
            .unwrap_or_else(|e| unreachable!("{e}")),
    )
    .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match aiff.next_event() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.aiff.not-started");
    let target = TimeSpan {
        start: Stamp::Present(Timestamp::new(
            0,
            TimeBase::new(1, 44_100).unwrap_or_else(|e| unreachable!("{e}")),
        )),
        end: Stamp::Missing,
    };
    let err = match aiff.seek(target) {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.aiff.not-started");
    let err = match aiff.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.aiff.not-started");
}

#[test]
fn seek_maps_frames_and_reanchors() {
    let samples = vec![0u8; 1000 * 4];
    let mut aiff = started_aiff(minimal_aiff(2, 8_000, 16, &samples));
    assert_eq!(aiff.seekability(), SeekSupport::Fast { indexed: false });
    let base = TimeBase::new(1, 8_000).unwrap_or_else(|e| unreachable!("{e}"));
    let target = TimeSpan {
        start: Stamp::Present(Timestamp::new(500, base)),
        end: Stamp::Missing,
    };
    let landing = aiff
        .seek(target)
        .unwrap_or_else(|e| unreachable!("seek: {e}"));
    match landing.start {
        Stamp::Present(ts) => assert_eq!(ts.ticks, 500),
        Stamp::Missing => unreachable!("landing must carry a stamp"),
    }
    let past = TimeSpan {
        start: Stamp::Present(Timestamp::new(1_001, base)),
        end: Stamp::Missing,
    };
    let err = match aiff.seek(past) {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.aiff.seek.out-of-range");
    aiff.reanchor(target)
        .unwrap_or_else(|e| unreachable!("reanchor: {e}"));
    let (bytes, _) = drain(&mut aiff);
    assert_eq!(bytes.len(), 500 * 4);
}

#[test]
fn probe_recognizes_both_forms() {
    assert!(super::probe(b"FORM\x24\x00\x00\x00AIFF"));
    assert!(super::probe(b"FORM\x24\x00\x00\x00AIFC"));
    assert!(!super::probe(b"FORM\x24\x00\x00\x00WAVE"));
    assert!(!super::probe(b"RIFF\x24\x00\x00\x00WAVE"));
    assert!(!super::probe(b"FORM\x24\x00"));
}

#[test]
fn error_constructors_keep_their_codes() {
    assert_eq!(error::not_aiff(b"FORM").context.code, "demux.aiff.not-aiff");
    assert_eq!(error::no_comm().context.code, "demux.aiff.no-comm");
    assert_eq!(error::no_ssnd().context.code, "demux.aiff.no-ssnd");
    assert_eq!(
        error::unsupported_compression(0).context.code,
        "demux.aiff.unsupported-compression"
    );
    let _ = CommChunk {
        channels: 1,
        num_frames: 0,
        sample_size: 16,
        sample_rate: 44_100,
        compression: None,
    };
    let _ = chunks::MARK;
}
