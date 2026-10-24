//! The WAV demuxer's unit tests (card 5.A2): framing, oddities, and
//! the AC error paths — truncated header/chunk, declared-size >
//! actual, zero data size, unaligned chunk skip, unknown-format tag
//! (0xFFFE without extensible body), unsupported bit depth. The
//! ffmpeg-generated fixtures get their differential + mutated-family
//! suite in `crates/demux/tests/wav_aiff_fixtures.rs`; the real
//! randomized corpus runs in `crates/demux/tests/corpus_wav.rs`.

use media_types::time::{Stamp, TimeBase, TimeSpan, Timestamp};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};

use super::error;
use super::format::WavCodec;
use super::test_support::{mem_stream, minimal_pcm_wav};
use super::{MAX_PACKET_BYTES, WavDemuxer};

/// Drain the demuxer, returning (payload bytes, events) — the packets'
/// concatenated data plus every non-packet event.
fn drain(wav: &mut WavDemuxer) -> (Vec<u8>, Vec<DemuxEvent>) {
    let mut bytes = Vec::new();
    let mut events = Vec::new();
    loop {
        match wav.next_event() {
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

fn started_wav(bytes: std::sync::Arc<[u8]>) -> WavDemuxer {
    let mut wav = WavDemuxer::new(mem_stream(bytes).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("new: {e}"));
    let _ = wav.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    wav
}

/// fmt(16) + data builder over raw fmt bytes (the odd-shape tests).
fn wav_over(fmt: &[u8], data: &[u8], data_size: u32) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(b"WAVE");
    b.extend_from_slice(b"fmt ");
    b.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
    b.extend_from_slice(fmt);
    if fmt.len() % 2 == 1 {
        b.push(0);
    }
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_size.to_le_bytes());
    b.extend_from_slice(data);
    let riff_size = (b.len() - 8) as u32;
    b[4..8].copy_from_slice(&riff_size.to_le_bytes());
    b
}

fn pcm16_fmt(channels: u16, rate: u32) -> Vec<u8> {
    let mut f = Vec::new();
    f.extend_from_slice(&1u16.to_le_bytes());
    f.extend_from_slice(&channels.to_le_bytes());
    f.extend_from_slice(&rate.to_le_bytes());
    let align = u32::from(channels) * 2;
    f.extend_from_slice(&(rate * align).to_le_bytes());
    f.extend_from_slice(&(align as u16).to_le_bytes());
    f.extend_from_slice(&16u16.to_le_bytes());
    f
}

#[test]
fn drains_plain_pcm16_whole_frames() {
    // 3 frames of s16le stereo = 12 bytes; one packet.
    let samples: Vec<u8> = (0..12u8).collect();
    let mut wav = started_wav(minimal_pcm_wav(1, 2, 8_000, 16, &samples));
    let (bytes, events) = drain(&mut wav);
    assert_eq!(bytes, samples);
    assert_eq!(events, vec![DemuxEvent::EndOfStream]);
    let fmt = wav.format().unwrap_or_else(|| unreachable!("fmt"));
    assert_eq!(fmt.channels, 2);
    assert_eq!(fmt.samples_per_sec, 8_000);
    assert_eq!(fmt.codec_descriptor(), "pcm_s16le");
    assert_eq!(wav.fmt_chunk_bytes().map_or(0, <[u8]>::len), 16);
    assert_eq!(wav.fact_sample_length(), None);
    assert_eq!(wav.damage_total(), 0);
}

#[test]
fn packets_are_bounded_and_laddered() {
    // 64 KiB + 1 frames of mono s16 → two packets, pts in frames.
    let frames = (MAX_PACKET_BYTES / 2 + 1) as usize;
    let mut samples = Vec::with_capacity(frames * 2);
    for i in 0..frames * 2 {
        samples.push((i % 251) as u8);
    }
    let mut wav = started_wav(minimal_pcm_wav(1, 1, 8_000, 16, &samples));
    let (bytes, events) = drain(&mut wav);
    assert_eq!(bytes, samples);
    assert!(
        events.iter().all(|e| matches!(e, DemuxEvent::EndOfStream)),
        "no damage expected: {events:?}"
    );
    // The pool settled at the quantum: one lazily-grown arena of
    // exactly MAX_PACKET_BYTES (the D-1 observable).
    assert_eq!(wav.pool_bytes(), MAX_PACKET_BYTES as usize);
}

#[test]
fn pts_ladder_counts_sample_frames() {
    // 300 frames stereo s16 at 8 kHz: tick = one frame.
    let samples = vec![0u8; 300 * 4];
    let mut wav = started_wav(minimal_pcm_wav(1, 2, 8_000, 16, &samples));
    let mut seen = Vec::new();
    loop {
        match wav.next_event() {
            Ok(DemuxEvent::Packet(p)) => match p.pts {
                Stamp::Present(ts) => seen.push(ts.ticks),
                Stamp::Missing => unreachable!("pts must be present"),
            },
            Ok(DemuxEvent::EndOfStream) => break,
            Ok(event) => unreachable!("unexpected {event:?}"),
            Err(err) => unreachable!("next: {err}"),
        }
    }
    // 1200 bytes = 1 packet ≤ 64 KiB — but assert the ladder start
    // and the track span independently.
    assert_eq!(seen.first().copied(), Some(0));
    let track = &wav.tracks.clone().unwrap_or_else(|| unreachable!())[0];
    match track.timing.span.end {
        Stamp::Present(ts) => assert_eq!(ts.ticks, 300),
        Stamp::Missing => unreachable!("span end must be present"),
    }
    assert_eq!(
        track.timing.base,
        TimeBase::new(1, 8_000).unwrap_or_else(|e| unreachable!("{e}"))
    );
}

#[test]
fn odd_sized_chunks_skip_their_pad_byte() {
    // fmt(16) + an odd-sized JUNK (3 bytes + 1 pad) + data — the walk
    // must land the data header past the pad, not inside it.
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(b"WAVE");
    b.extend_from_slice(b"fmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes()); // PCM
    b.extend_from_slice(&1u16.to_le_bytes()); // mono
    b.extend_from_slice(&8_000u32.to_le_bytes());
    b.extend_from_slice(&16_000u32.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"JUNK");
    b.extend_from_slice(&3u32.to_le_bytes());
    b.extend_from_slice(b"abc");
    b.push(0); // pad byte (not counted in the size)
    let payload = [0x11u8, 0x22, 0x33, 0x44];
    b.extend_from_slice(b"data");
    b.extend_from_slice(&4u32.to_le_bytes());
    b.extend_from_slice(&payload);
    let riff_size = (b.len() - 8) as u32;
    b[4..8].copy_from_slice(&riff_size.to_le_bytes());

    let mut wav = started_wav(std::sync::Arc::from(b));
    let (bytes, events) = drain(&mut wav);
    assert_eq!(bytes, payload);
    assert!(
        events.iter().all(|e| matches!(e, DemuxEvent::EndOfStream)),
        "{events:?}"
    );
}

#[test]
fn odd_data_payload_strips_its_pad() {
    // A 5-byte data payload (odd) + pad, mono u8 (unit = 1 byte, so
    // every payload byte is a whole sample frame — the sibling
    // `trailing_partial_frame` test pins the s16 sliver path): the
    // packet stream carries exactly the 5 declared bytes, never the
    // pad (the ffmpeg MD5 parity depends on this).
    let mut fmt = Vec::new();
    fmt.extend_from_slice(&1u16.to_le_bytes()); // PCM
    fmt.extend_from_slice(&1u16.to_le_bytes()); // mono
    fmt.extend_from_slice(&8_000u32.to_le_bytes());
    fmt.extend_from_slice(&8_000u32.to_le_bytes());
    fmt.extend_from_slice(&1u16.to_le_bytes()); // align
    fmt.extend_from_slice(&8u16.to_le_bytes()); // bits
    let mut b = wav_over(&fmt, &[9, 8, 7, 6, 5], 5);
    b.push(0); // the odd data's pad byte
    let riff_size = (b.len() - 8) as u32;
    b[4..8].copy_from_slice(&riff_size.to_le_bytes());
    let mut wav = started_wav(std::sync::Arc::from(b));
    let (bytes, _) = drain(&mut wav);
    assert_eq!(bytes, vec![9, 8, 7, 6, 5]);
}

#[test]
fn fact_and_info_metadata_are_parsed() {
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(b"WAVE");
    b.extend_from_slice(b"fmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&8_000u32.to_le_bytes());
    b.extend_from_slice(&32_000u32.to_le_bytes());
    b.extend_from_slice(&4u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"fact");
    b.extend_from_slice(&4u32.to_le_bytes());
    b.extend_from_slice(&7u32.to_le_bytes()); // dwSampleLength
    // LIST INFO with one 13-byte (odd) INAM value + 1 pad byte:
    // list size = 4 (INFO) + 8 + 13 + 1 = 26.
    b.extend_from_slice(b"LIST");
    b.extend_from_slice(&26u32.to_le_bytes());
    b.extend_from_slice(b"INFO");
    b.extend_from_slice(b"INAM");
    b.extend_from_slice(&13u32.to_le_bytes());
    b.extend_from_slice(b"Lumen Player1");
    b.push(0); // the odd value's pad byte
    b.extend_from_slice(b"data");
    let payload = [0u8; 28]; // 7 frames × 4 bytes
    b.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    b.extend_from_slice(&payload);
    let riff_size = (b.len() - 8) as u32;
    b[4..8].copy_from_slice(&riff_size.to_le_bytes());

    let mut wav = started_wav(std::sync::Arc::from(b));
    assert_eq!(wav.fact_sample_length(), Some(7));
    assert_eq!(wav.info().len(), 1);
    assert_eq!(wav.info()[0].id.as_bytes(), b"INAM");
    // The track's title rides the INAM value (lossy UTF-8).
    let track = &wav.tracks.clone().unwrap_or_else(|| unreachable!())[0];
    assert!(
        track.title.is_some(),
        "INAM must surface as the track title"
    );
    let (bytes, _) = drain(&mut wav);
    assert_eq!(bytes.len(), 28);
}

#[test]
fn rf64_ds64_sizes_are_the_truth() {
    // RF64 magic + ds64 + fmt + data(0xFFFFFFFF sentinel).
    let mut b = Vec::new();
    b.extend_from_slice(b"RF64");
    b.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    b.extend_from_slice(b"WAVE");
    b.extend_from_slice(b"ds64");
    b.extend_from_slice(&28u32.to_le_bytes());
    b.extend_from_slice(&0u64.to_le_bytes()); // riffSize — patched below
    b.extend_from_slice(&8u64.to_le_bytes()); // dataSize
    b.extend_from_slice(&2u64.to_le_bytes()); // sampleCount
    b.extend_from_slice(&0u32.to_le_bytes()); // tableLength
    b.extend_from_slice(b"fmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&8_000u32.to_le_bytes());
    b.extend_from_slice(&32_000u32.to_le_bytes());
    b.extend_from_slice(&4u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    let payload = [1u8, 0, 2, 0, 3, 0, 4, 0];
    b.extend_from_slice(&payload);
    let riff_size = (b.len() - 8) as u64;
    b[20..28].copy_from_slice(&riff_size.to_le_bytes()); // ds64.riffSize (payload starts at 20)

    let mut wav = started_wav(std::sync::Arc::from(b));
    assert!(wav.is_rf64_like());
    let fmt = wav.format().unwrap_or_else(|| unreachable!("fmt"));
    assert_eq!(fmt.codec_descriptor(), "pcm_s16le");
    let (bytes, events) = drain(&mut wav);
    assert_eq!(bytes, payload);
    assert!(
        events.iter().all(|e| matches!(e, DemuxEvent::EndOfStream)),
        "{events:?}"
    );
}

#[test]
fn adpcm_packets_are_block_aligned_with_frame_accounting() {
    // IMA ADPCM, the 0042 corpus shape: 1024-byte blocks × 2ch,
    // 1017 samples per block; 2 blocks of payload + a trailing
    // sliver of half a block (the not-aligned tail is damage).
    let mut fmt = Vec::new();
    fmt.extend_from_slice(&0x0011u16.to_le_bytes()); // IMA ADPCM
    fmt.extend_from_slice(&2u16.to_le_bytes());
    fmt.extend_from_slice(&44_056u32.to_le_bytes());
    fmt.extend_from_slice(&16_000u32.to_le_bytes());
    fmt.extend_from_slice(&1_024u16.to_le_bytes());
    fmt.extend_from_slice(&4u16.to_le_bytes());
    fmt.extend_from_slice(&2u16.to_le_bytes()); // cbSize
    fmt.extend_from_slice(&1_017u16.to_le_bytes()); // samples/block
    let mut data = vec![0x5Au8; 1_024 * 2];
    data.extend_from_slice(&[0x33u8; 512]); // the half-block sliver
    let file = wav_over(&fmt, &data, data.len() as u32);

    let mut wav = started_wav(std::sync::Arc::from(file));
    let fmt = wav.format().unwrap_or_else(|| unreachable!("fmt"));
    assert_eq!(fmt.codec_descriptor(), "adpcm_ima_wav");
    assert_eq!(
        fmt.codec,
        WavCodec::AdpcmImaWav {
            samples_per_block: 1_017
        }
    );
    assert_eq!(wav.frames(), Some(2 * 1_017));
    // The first packet: 64 KiB quantum ÷ 1024-byte blocks = 64
    // blocks… but only 2 whole blocks exist — one packet of 2048
    // bytes, 2034 sample frames, then the sliver is damage.
    let mut packets = 0;
    let mut first_ticks = None;
    let mut bytes = 0usize;
    loop {
        match wav.next_event() {
            Ok(DemuxEvent::Packet(p)) => {
                packets += 1;
                bytes += p.data.len();
                if first_ticks.is_none() {
                    first_ticks = match p.pts {
                        Stamp::Present(ts) => Some(ts.ticks),
                        Stamp::Missing => unreachable!("adpcm pts must be present"),
                    };
                }
            }
            Ok(DemuxEvent::EndOfStream) => break,
            Ok(DemuxEvent::Recovered { skipped, reason }) => {
                assert_eq!(skipped, 512, "{reason}");
                assert_eq!(reason, "data-not-frame-aligned");
            }
            Ok(event) => unreachable!("unexpected {event:?}"),
            Err(err) => unreachable!("next: {err}"),
        }
    }
    assert_eq!(packets, 1);
    assert_eq!(bytes, 2_048);
    assert_eq!(first_ticks, Some(0));
    let track = &wav.tracks.clone().unwrap_or_else(|| unreachable!())[0];
    match track.timing.span.end {
        Stamp::Present(ts) => assert_eq!(ts.ticks, 2_034),
        Stamp::Missing => unreachable!("span end must be present"),
    }
}

#[test]
fn flac_in_wav_enumerates_with_declared_span_and_unstamped_packets() {
    // The 0016 corpus shape: 74-byte extensible fmt (FLAC GUID +
    // STREAMINFO vendor tail) + fact + data.
    let mut fmt = Vec::new();
    fmt.extend_from_slice(&0xFFFEu16.to_le_bytes());
    fmt.extend_from_slice(&2u16.to_le_bytes());
    fmt.extend_from_slice(&48_000u32.to_le_bytes());
    fmt.extend_from_slice(&16_000u32.to_le_bytes());
    fmt.extend_from_slice(&4u16.to_le_bytes());
    fmt.extend_from_slice(&16u16.to_le_bytes());
    fmt.extend_from_slice(&56u16.to_le_bytes()); // cbSize
    fmt.extend_from_slice(&16u16.to_le_bytes()); // validBits
    fmt.extend_from_slice(&0x3u32.to_le_bytes()); // channel mask
    fmt.extend_from_slice(&super::format::SUBFORMAT_FLAC_GUID);
    fmt.extend_from_slice(&[0xA5u8; 34]); // the STREAMINFO tail
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(b"WAVE");
    b.extend_from_slice(b"fmt ");
    b.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
    b.extend_from_slice(&fmt);
    b.extend_from_slice(b"fact");
    b.extend_from_slice(&4u32.to_le_bytes());
    b.extend_from_slice(&96_000u32.to_le_bytes()); // 2 s × 48 kHz
    b.extend_from_slice(b"data");
    let flac_frames = vec![0xFFu8, 0xF8, 0xAB, 0xCD, 0xEF, 0x01, 0x23];
    b.extend_from_slice(&(flac_frames.len() as u32).to_le_bytes());
    b.extend_from_slice(&flac_frames);
    let riff_size = (b.len() - 8) as u32;
    b[4..8].copy_from_slice(&riff_size.to_le_bytes());

    let mut wav = started_wav(std::sync::Arc::from(b));
    let fmt = wav.format().unwrap_or_else(|| unreachable!("fmt"));
    assert_eq!(fmt.codec_descriptor(), "flac");
    assert!(matches!(
        fmt.codec,
        WavCodec::Flac {
            streaminfo: Some(_)
        }
    ));
    // The declared span rides fact (the frame grammar is not ours).
    let track = &wav.tracks.clone().unwrap_or_else(|| unreachable!())[0];
    match track.timing.span.end {
        Stamp::Present(ts) => assert_eq!(ts.ticks, 96_000),
        Stamp::Missing => unreachable!("fact-declared span must be present"),
    }
    // Seek is honestly refused for the frame-unknown payload.
    assert_eq!(wav.seekability(), SeekSupport::None);
    // Packets deliver the payload verbatim, stamps absent (raw
    // FLAC-in-a-box), duration unknown.
    let (bytes, _) = drain(&mut wav);
    assert_eq!(bytes, flac_frames);
    let mut wav2 = started_wav(minimal_pcm_wav(1, 2, 8_000, 16, &[0u8; 4]));
    let _ = &mut wav2;
}

#[test]
fn flac_packets_carry_no_stamps() {
    let mut fmt = Vec::new();
    fmt.extend_from_slice(&0xFFFEu16.to_le_bytes());
    fmt.extend_from_slice(&2u16.to_le_bytes());
    fmt.extend_from_slice(&48_000u32.to_le_bytes());
    fmt.extend_from_slice(&16_000u32.to_le_bytes());
    fmt.extend_from_slice(&4u16.to_le_bytes());
    fmt.extend_from_slice(&16u16.to_le_bytes());
    fmt.extend_from_slice(&22u16.to_le_bytes());
    fmt.extend_from_slice(&16u16.to_le_bytes());
    fmt.extend_from_slice(&0x3u32.to_le_bytes());
    fmt.extend_from_slice(&super::format::SUBFORMAT_FLAC_GUID);
    let file = wav_over(&fmt, &[0xFF, 0xF8, 0x00, 0x11], 4);
    let mut wav = started_wav(std::sync::Arc::from(file));
    match wav.next_event() {
        Ok(DemuxEvent::Packet(p)) => {
            assert_eq!(p.pts, Stamp::Missing);
            assert_eq!(p.dts, Stamp::Missing);
            assert!(p.duration.is_none());
            assert_eq!(p.data.as_ref(), &[0xFF, 0xF8, 0x00, 0x11]);
        }
        other => unreachable!("packet expected, got {other:?}"),
    }
    match wav.next_event() {
        Ok(DemuxEvent::EndOfStream) => {}
        Ok(event) => unreachable!("unexpected {event:?}"),
        Err(err) => unreachable!("next: {err}"),
    }
}

#[test]
fn truncated_head_is_typed_not_wav_or_truncated() {
    // 4 magic bytes only: a cut inside the head — truncation flavor.
    let mut wav =
        WavDemuxer::new(mem_stream(b"RIFF".to_vec()).unwrap_or_else(|e| unreachable!("{e}")))
            .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match wav.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.wav.truncated");

    // 12 bytes without the WAVE form: not a WAV.
    let mut wav = WavDemuxer::new(
        mem_stream(b"RIFF\x10\x00\x00\x00AVIXXXXX".to_vec())
            .unwrap_or_else(|e| unreachable!("{e}")),
    )
    .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match wav.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.wav.not-wav");
}

#[test]
fn truncated_mid_header_is_typed_truncation() {
    // Cut inside the fmt chunk: the stream ends before the declared
    // structure completes.
    let full = minimal_pcm_wav(1, 2, 8_000, 16, &[0u8; 8]);
    let cut: std::sync::Arc<[u8]> = full[..24].into();
    let mut wav = WavDemuxer::new(mem_stream(cut).unwrap_or_else(|e| unreachable!("{e}")))
        .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match wav.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.wav.truncated");
    // Sticky: the next start replays the same shape.
    let err2 = match wav.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err2.context.code, "demux.wav.truncated");
}

#[test]
fn declared_size_past_actual_is_typed_truncation_after_packets() {
    // data declares 16 bytes; the file carries only 8.
    let mut b = wav_over(&pcm16_fmt(2, 8_000), &[1u8, 0, 2, 0, 3, 0, 4, 0], 16);

    let mut wav = started_wav(std::sync::Arc::from(b.clone()));
    // Parse-time damage surfaces first (the damage-record-first
    // contract), then the real prefix delivers (D-3: never forfeit
    // silently), then the walk refuses typed.
    match wav.next_event() {
        Ok(DemuxEvent::Recovered { skipped, reason }) => {
            assert_eq!(skipped, 8);
            assert_eq!(reason, "data-extends-past-stream");
        }
        other => unreachable!("expected the damage record first, got {other:?}"),
    }
    match wav.next_event() {
        Ok(DemuxEvent::Packet(packet)) => assert_eq!(packet.data.len(), 8),
        other => unreachable!("expected the prefix packet, got {other:?}"),
    }
    let err = match wav.next_event() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.wav.truncated");
    assert_eq!(wav.damage_total(), 8, "the missing 8 bytes are counted");
    let _ = &mut b;
}

#[test]
fn zero_data_size_is_typed_no_data() {
    // fmt + data with size 0: nothing to demux.
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36u32).to_le_bytes());
    b.extend_from_slice(b"WAVE");
    b.extend_from_slice(b"fmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&8_000u32.to_le_bytes());
    b.extend_from_slice(&32_000u32.to_le_bytes());
    b.extend_from_slice(&4u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&0u32.to_le_bytes());

    let mut wav = WavDemuxer::new(
        mem_stream(std::sync::Arc::<[u8]>::from(b)).unwrap_or_else(|e| unreachable!("{e}")),
    )
    .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match wav.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.wav.no-data");
}

#[test]
fn data_sliver_below_one_frame_is_no_data() {
    // 1 byte of mono s16 (needs 2): not one whole sample frame.
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(b"WAVE");
    b.extend_from_slice(b"fmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&8_000u32.to_le_bytes());
    b.extend_from_slice(&8_000u32.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&1u32.to_le_bytes());
    b.extend_from_slice(&[1]);
    b.push(0); // the odd value's pad byte
    let riff_size = (b.len() - 8) as u32;
    b[4..8].copy_from_slice(&riff_size.to_le_bytes());

    let mut wav = WavDemuxer::new(
        mem_stream(std::sync::Arc::<[u8]>::from(b)).unwrap_or_else(|e| unreachable!("{e}")),
    )
    .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match wav.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.wav.no-data");
}

#[test]
fn trailing_partial_frame_is_damaged_and_dropped() {
    // 2 whole frames + 1 stray byte of mono s16.
    let mut b = wav_over(&pcm16_fmt(1, 8_000), &[1, 0, 2, 0, 9], 5);
    if b.len() % 2 == 1 {
        b.push(0);
    }

    let mut wav = started_wav(std::sync::Arc::from(b));
    let (bytes, events) = drain(&mut wav);
    assert_eq!(bytes, vec![1, 0, 2, 0], "only whole frames deliver");
    assert!(
        events.iter().any(|e| matches!(
            *e,
            DemuxEvent::Recovered {
                reason: "data-not-frame-aligned",
                ..
            }
        )),
        "{events:?}"
    );
}

#[test]
fn missing_fmt_and_data_are_typed() {
    // RIFF WAVE with no chunks at all: no-fmt.
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&4u32.to_le_bytes());
    b.extend_from_slice(b"WAVE");
    let mut wav = WavDemuxer::new(
        mem_stream(std::sync::Arc::<[u8]>::from(b)).unwrap_or_else(|e| unreachable!("{e}")),
    )
    .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match wav.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.wav.no-fmt");

    // fmt but no data (and a complete RIFF declaration): no-data.
    let b = wav_over(&pcm16_fmt(2, 8_000), &[], 0);
    let mut wav = WavDemuxer::new(
        mem_stream(std::sync::Arc::<[u8]>::from(b)).unwrap_or_else(|e| unreachable!("{e}")),
    )
    .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match wav.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.wav.no-data");
}

#[test]
fn trait_misuse_is_typed_not_a_panic() {
    let mut wav = WavDemuxer::new(
        mem_stream(minimal_pcm_wav(1, 2, 8_000, 16, &[0u8; 4]))
            .unwrap_or_else(|e| unreachable!("{e}")),
    )
    .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match wav.next_event() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.wav.not-started");
    // seek before start: typed (the same operational shape).
    let target = TimeSpan {
        start: Stamp::Present(Timestamp::new(
            0,
            TimeBase::new(1, 8_000).unwrap_or_else(|e| unreachable!("{e}")),
        )),
        end: Stamp::Missing,
    };
    let err = match wav.seek(target) {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.wav.not-started");
    // Operational misuse is sticky (the landed convention — the
    // instance replays its shape; a fresh demuxer is the recovery).
    let err = match wav.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.wav.not-started");
}

#[test]
fn seek_maps_frames_and_reanchors() {
    // 1000 frames stereo s16 at 8 kHz = 125 ms; seek to 62.5 ms.
    let mut wav = started_wav(minimal_pcm_wav(1, 2, 8_000, 16, &vec![0u8; 1000 * 4]));
    assert_eq!(wav.seekability(), SeekSupport::Fast { indexed: false });
    let base = TimeBase::new(1, 8_000).unwrap_or_else(|e| unreachable!("{e}"));
    let target = TimeSpan {
        start: Stamp::Present(Timestamp::new(500, base)),
        end: Stamp::Missing,
    };
    let landing = wav
        .seek(target)
        .unwrap_or_else(|e| unreachable!("seek: {e}"));
    match landing.start {
        Stamp::Present(ts) => assert_eq!(ts.ticks, 500),
        Stamp::Missing => unreachable!("landing must carry a stamp"),
    }
    // Out of range (past the end) is typed.
    let past = TimeSpan {
        start: Stamp::Present(Timestamp::new(1_001, base)),
        end: Stamp::Missing,
    };
    let err = match wav.seek(past) {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.wav.seek.out-of-range");

    // Reanchor commits; the next packet starts at frame 500, and the
    // post-seek remainder is exactly the second half (the consumed
    // packet plus the drain — 500 frames × 4 bytes).
    wav.reanchor(target)
        .unwrap_or_else(|e| unreachable!("reanchor: {e}"));
    let mut delivered = 0usize;
    match wav.next_event() {
        Ok(DemuxEvent::Packet(p)) => {
            match p.pts {
                Stamp::Present(ts) => assert_eq!(ts.ticks, 500),
                Stamp::Missing => unreachable!("post-seek pts must be present"),
            }
            delivered += p.data.len();
        }
        other => unreachable!("packet expected, got {other:?}"),
    }
    let (bytes, _) = drain(&mut wav);
    delivered += bytes.len();
    assert_eq!(delivered, 500 * 4);
}

#[test]
fn adpcm_seek_lands_on_block_boundaries() {
    // 3 IMA blocks; a target inside block 1 lands at its start.
    let mut fmt = Vec::new();
    fmt.extend_from_slice(&0x0011u16.to_le_bytes());
    fmt.extend_from_slice(&1u16.to_le_bytes());
    fmt.extend_from_slice(&8_000u32.to_le_bytes());
    fmt.extend_from_slice(&4_000u32.to_le_bytes());
    fmt.extend_from_slice(&1_024u16.to_le_bytes());
    fmt.extend_from_slice(&4u16.to_le_bytes());
    fmt.extend_from_slice(&2u16.to_le_bytes());
    // samples/block for mono, 1024-byte blocks: ((1024-4)/4)*8+1 = 2041
    fmt.extend_from_slice(&2_041u16.to_le_bytes());
    let data = vec![0u8; 1_024 * 3];
    let file = wav_over(&fmt, &data, data.len() as u32);
    let mut wav = started_wav(std::sync::Arc::from(file));
    assert_eq!(wav.seekability(), SeekSupport::Fast { indexed: false });
    let base = TimeBase::new(1, 8_000).unwrap_or_else(|e| unreachable!("{e}"));
    // Frame 3000 sits inside block 1 (2041..4082): the landing is
    // block 1's first sample frame (2041), byte 1024.
    let target = TimeSpan {
        start: Stamp::Present(Timestamp::new(3_000, base)),
        end: Stamp::Missing,
    };
    let landing = wav
        .seek(target)
        .unwrap_or_else(|e| unreachable!("seek: {e}"));
    match landing.start {
        Stamp::Present(ts) => assert_eq!(ts.ticks, 2_041),
        Stamp::Missing => unreachable!("landing must carry a stamp"),
    }
    wav.reanchor(target)
        .unwrap_or_else(|e| unreachable!("reanchor: {e}"));
    match wav.next_event() {
        Ok(DemuxEvent::Packet(p)) => {
            match p.pts {
                Stamp::Present(ts) => assert_eq!(ts.ticks, 2_041),
                Stamp::Missing => unreachable!("post-seek pts must be present"),
            }
            assert_eq!(p.data.len(), 2 * 1_024, "the two remaining blocks");
        }
        other => unreachable!("packet expected, got {other:?}"),
    }
}

#[test]
fn seek_errors_stay_sticky_free_and_demuxer_unmoved() {
    // D4: a failed seek leaves the demuxer's position and state —
    // drain still yields the full stream.
    let mut wav = started_wav(minimal_pcm_wav(1, 2, 8_000, 16, &[0x5Au8; 4 * 10]));
    let base = TimeBase::new(1, 8_000).unwrap_or_else(|e| unreachable!("{e}"));
    let past = TimeSpan {
        start: Stamp::Present(Timestamp::new(99_999, base)),
        end: Stamp::Missing,
    };
    assert!(wav.seek(past).is_err());
    let (bytes, _) = drain(&mut wav);
    assert_eq!(bytes.len(), 40);
}

#[test]
fn rf64_sentinel_without_ds64_is_a_declared_lie() {
    // RF64 magic, 0xFFFFFFFF sizes, no ds64: the mandatory size-truth
    // chunk never arrived — the structural refusal.
    let mut b = Vec::new();
    b.extend_from_slice(b"RF64");
    b.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    b.extend_from_slice(b"WAVE");
    b.extend_from_slice(b"fmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&8_000u32.to_le_bytes());
    b.extend_from_slice(&32_000u32.to_le_bytes());
    b.extend_from_slice(&4u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&8u32.to_le_bytes());
    b.extend_from_slice(&[0u8; 8]);

    let mut wav = WavDemuxer::new(
        mem_stream(std::sync::Arc::<[u8]>::from(b)).unwrap_or_else(|e| unreachable!("{e}")),
    )
    .unwrap_or_else(|e| unreachable!("new: {e}"));
    let err = match wav.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.wav.bad-structure");
}

#[test]
fn plain_riff_data_sentinel_means_to_stream_end() {
    // The streaming-writer convention: a 0xFFFFFFFF data size in a
    // plain RIFF delivers the real available payload, damage-noted —
    // never read as 4 GiB, never a structural refusal (the ffmpeg
    // behavior on live-recorded WAVs).
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes()); // head sentinel too
    b.extend_from_slice(b"WAVE");
    b.extend_from_slice(b"fmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&8_000u32.to_le_bytes());
    b.extend_from_slice(&32_000u32.to_le_bytes());
    b.extend_from_slice(&4u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    let payload = [1u8, 0, 2, 0, 3, 0, 4, 0];
    b.extend_from_slice(&payload);

    let mut wav = WavDemuxer::new(
        mem_stream(std::sync::Arc::<[u8]>::from(b)).unwrap_or_else(|e| unreachable!("{e}")),
    )
    .unwrap_or_else(|e| unreachable!("new: {e}"));
    let tracks = wav.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    assert_eq!(tracks.len(), 1);
    // Both sentinel decisions are damage-noted (0-byte notes: the
    // decision itself, not a byte range).
    let (bytes, events) = drain(&mut wav);
    assert_eq!(bytes, payload);
    assert_eq!(
        events,
        vec![
            DemuxEvent::Recovered {
                skipped: 0,
                reason: "riff-size-sentinel"
            },
            DemuxEvent::Recovered {
                skipped: 0,
                reason: "data-sentinel-without-ds64"
            },
            DemuxEvent::EndOfStream,
        ],
        "{events:?}"
    );
}

#[test]
fn probe_recognizes_the_three_magics_and_rejects_other_forms() {
    assert!(super::probe(b"RIFF\x24\x00\x00\x00WAVE"));
    assert!(super::probe(b"RF64\xFF\xFF\xFF\xFFWAVE"));
    assert!(super::probe(b"BW64\xFF\xFF\xFF\xFFWAVE"));
    assert!(!super::probe(b"RIFF\x24\x00\x00\x00AVI "));
    assert!(!super::probe(b"RIFX\x24\x00\x00\x00WAVE"));
    assert!(!super::probe(b"RIFF\x24\x00")); // too short to claim
    assert!(!super::probe(b"FORM\x00\x00\x89\xfeAIFF"));
}

#[test]
fn oversized_info_value_is_damage_not_fatal() {
    // An INFO value above MAX_INFO_VALUE_BYTES is skipped, the file
    // still demuxes.
    let big_value = vec![b'x'; super::MAX_INFO_VALUE_BYTES + 1];
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(b"WAVE");
    b.extend_from_slice(b"fmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&8_000u32.to_le_bytes());
    b.extend_from_slice(&8_000u32.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"LIST");
    let list_payload_len = 4 + 8 + big_value.len() + (big_value.len() % 2);
    b.extend_from_slice(&(list_payload_len as u32).to_le_bytes());
    b.extend_from_slice(b"INFO");
    b.extend_from_slice(b"ICMT");
    b.extend_from_slice(&(big_value.len() as u32).to_le_bytes());
    b.extend_from_slice(&big_value);
    if big_value.len() % 2 == 1 {
        b.push(0);
    }
    b.extend_from_slice(b"data");
    b.extend_from_slice(&4u32.to_le_bytes());
    b.extend_from_slice(&[7, 0, 8, 0]);
    let riff_size = (b.len() - 8) as u32;
    b[4..8].copy_from_slice(&riff_size.to_le_bytes());

    let mut wav = started_wav(std::sync::Arc::from(b));
    assert!(wav.info().is_empty());
    assert!(
        wav.damage_total() > 0,
        "the oversized value is counted, not silent"
    );
    let (bytes, _) = drain(&mut wav);
    assert_eq!(bytes, vec![7, 0, 8, 0]);
}

#[test]
fn error_constructors_keep_their_codes() {
    assert_eq!(error::not_wav(b"RIFF").context.code, "demux.wav.not-wav");
    assert_eq!(error::no_fmt().context.code, "demux.wav.no-fmt");
    assert_eq!(error::no_data().context.code, "demux.wav.no-data");
    assert_eq!(
        error::unsupported_format(0).context.code,
        "demux.wav.unsupported-format"
    );
}
