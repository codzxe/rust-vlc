//! Vorbis bit-exact/differential suite (card 5.B2 AC): the four
//! committed fixtures + corpus 0085, decoded through the **real
//! composition** (ogg demux → header chain as `extra_data` → the
//! codec-api `VorbisDecoder`), pinned against
//! `tests/data/vorbis_reference.txt` (the ffmpeg 7.1.5 oracle) and —
//! when an ffmpeg binary is present — re-diffed **live**.
//!
//! Vorbis is lossy: independent decoders differ in float rounding, so
//! the assertions are (1) the exact granule-timeline geometry (sample
//! counts, packet counts, final granule), (2) our aligned window's
//! canonicalized-f32 MD5 as the hard regression pin, and (3) the live
//! per-sample max |delta| ≤ the recorded bound (1e-7 on the sine
//! lanes; the digital-silence corridor must be **bit-exact** — every
//! conforming decoder emits exact zeros there, and the MD5s are
//! asserted equal). The differential table this test prints is the
//! card's worklog table.

mod common;
mod vorbis;

use std::path::Path;

use testkit::Must;
use vorbis::{
    canonical_f32_md5, corpus_dir, decode_through_ogg, f32_max_delta, ffmpeg_available,
    ffmpeg_decode_f32le, fixture_bytes, fixture_path, reference_section, window_at,
};

/// The committed fixture set (the reference data carries a section
/// per name).
const FIXTURES: [&str; 4] = [
    "vorbis_sine_44100_stereo.ogg",
    "vorbis_sine_8000_mono.ogg",
    "vorbis_silence_48000_stereo.ogg",
    "vorbis_tagged_sine.ogg",
];

/// `section[key]` parsed as `usize` (recorded expectations are
/// integers by construction).
fn usiz(section: &std::collections::HashMap<String, String>, key: &str) -> usize {
    section
        .get(key)
        .unwrap_or_else(|| unreachable!("missing key {key}"))
        .parse::<usize>()
        .must()
}

/// `section[key]` as `usize` when present (corpus sections omit the
/// blocksize keys — they are a fixture-lane assertion).
fn opt_usiz(section: &std::collections::HashMap<String, String>, key: &str) -> Option<usize> {
    section.get(key).map(|v| v.parse::<usize>().must())
}

/// `section[key]` parsed as `f32` (the max-delta bound).
fn f32_of(section: &std::collections::HashMap<String, String>, key: &str) -> f32 {
    section
        .get(key)
        .unwrap_or_else(|| unreachable!("missing key {key}"))
        .parse::<f32>()
        .must()
}

/// One fixture row: geometry + window pin + live differential. The
/// `name` prints into the worklog's differential table.
fn assert_row(name: &str, data: &[u8], path: &Path, live: bool) {
    // The reference sections: `[fixture.<stem>]` for fixtures (the
    // `.ogg` suffix stripped), `[corpus.…]` verbatim.
    let section_name = if name.starts_with("corpus.") {
        name.to_string()
    } else {
        format!("fixture.{}", name.trim_end_matches(".ogg"))
    };
    let reference = reference_section(&section_name);
    let run = decode_through_ogg(data);
    let ident = run.headers.identification;

    // (1) The granule-timeline geometry, exactly as ffmpeg recorded it.
    assert_eq!(
        ident.channels as usize,
        usiz(reference, "channels"),
        "{name}: channels"
    );
    assert_eq!(
        ident.sample_rate as usize,
        usiz(reference, "rate"),
        "{name}: rate"
    );
    if let Some(short) = opt_usiz(reference, "blocksize_short") {
        assert_eq!(
            ident.blocksize_short as usize, short,
            "{name}: blocksize_short"
        );
        assert_eq!(
            ident.blocksize_long as usize,
            usiz(reference, "blocksize_long"),
            "{name}: blocksize_long"
        );
    }
    assert_eq!(run.packets, usiz(reference, "packets"), "{name}: packets");
    let channels = ident.channels as usize;
    let raw_samples: usize = run
        .sink
        .buffers
        .iter()
        .map(|b| b.len() / (channels * 4))
        .sum();
    assert_eq!(
        raw_samples,
        usiz(reference, "raw_samples"),
        "{name}: raw samples"
    );
    assert_eq!(
        run.last_granule.map(|g| g as usize),
        Some(usiz(reference, "final_granule")),
        "{name}: final granule (the Ogg EOS stamp)"
    );

    // (2) The aligned window: head-trimmed to the encoder delay, cut
    // to ffmpeg's own sample count, hard-pinned by MD5.
    let head_trim = usiz(reference, "head_trim");
    let window_samples = usiz(reference, "window_samples");
    let window = window_at(&run.sink, channels, head_trim, window_samples);
    assert_eq!(
        window.len(),
        window_samples * channels * 4,
        "{name}: aligned window length"
    );
    let ours = canonical_f32_md5(&window);
    assert_eq!(
        &ours, &reference["lumen_window_md5"],
        "{name}: our window MD5 vs the recorded pin"
    );

    // (3) The live differential (when the oracle is present): same
    // length, bounded per-sample delta; the silence corridor is
    // bit-exact (canonicalized MD5s equal, delta 0).
    let bound = f32_of(reference, "max_delta");
    let mut delta_line = String::from("live-oracle-absent");
    if live {
        let ff = ffmpeg_decode_f32le(path);
        assert_eq!(ff.len(), window.len(), "{name}: ffmpeg f32le length");
        let delta = f32_max_delta(&window, &ff);
        assert!(
            delta <= bound,
            "{name}: live max |delta| {delta:e} > bound {bound:e}"
        );
        let ff_md5 = canonical_f32_md5(&ff);
        assert_eq!(
            &ff_md5, &reference["ffmpeg_md5_f32le"],
            "{name}: ffmpeg digest drift"
        );
        if bound == 0.0 {
            assert_eq!(ours, ff_md5, "{name}: the silence corridor is bit-exact");
        }
        delta_line = format!("max_delta={delta:e} (bound {bound:e})");
    }

    // The worklog differential row.
    eprintln!(
        "DIFF-ROW {name}: samples={window_samples} ch={channels} rate={} \
         lumen_window_md5={ours} ffmpeg_md5_f32le={} {delta_line}",
        ident.sample_rate, reference["ffmpeg_md5_f32le"],
    );
}

#[test]
fn fixtures_are_pinned_against_the_ffmpeg_reference() {
    let live = ffmpeg_available();
    if !live {
        eprintln!("no ffmpeg on PATH — the live differential half SKIPs (pins still asserted)");
    }
    for name in FIXTURES {
        assert_row(name, &fixture_bytes(name), &fixture_path(name), live);
    }
}

#[test]
fn tagged_fixture_exposes_its_vorbis_comments() {
    // The metadata AC: vendor + user comments parsed from the comment
    // header, case-insensitive lookup, the tagged fixture's five
    // fields exactly as ffmpeg wrote them.
    let run = decode_through_ogg(&fixture_bytes("vorbis_tagged_sine.ogg"));
    let comments = &run.headers.comments;
    let reference = reference_section("fixture.vorbis_tagged_sine");
    assert_eq!(comments.vendor, reference["vendor"]);
    assert_eq!(comments.title(), Some(reference["title"].as_str()));
    assert_eq!(comments.artists(), vec![reference["artist"].as_str()]);
    assert_eq!(comments.album(), Some(reference["album"].as_str()));
    assert_eq!(comments.date(), Some(reference["date"].as_str()));
    // The case-insensitive + unknown-field surface.
    assert_eq!(comments.get("TiTlE"), Some(reference["title"].as_str()));
    assert_eq!(comments.get("no_such_field"), None);
    // The other fixtures carry no TITLE (only the vendor string).
    let plain = decode_through_ogg(&fixture_bytes("vorbis_sine_44100_stereo.ogg"));
    assert_eq!(plain.headers.comments.title(), None);
}

#[test]
fn corpus_0085_differential_or_skip() {
    // The real-corpus row (doc 04 §3.1): /tmp-resident — absent corpus
    // is a SKIP with a message, never a silent pass.
    let Some(root) = corpus_dir() else {
        eprintln!("corpus: LUMEN_CORPUS unset and /tmp/my-project/corpus-v4 absent — SKIP");
        return;
    };
    let path = root.join("media/0085-audio-only.ogg");
    if !path.is_file() {
        eprintln!(
            "corpus 0085 (vorbis ogg) absent at {} — SKIP",
            path.display()
        );
        return;
    }
    let data = std::fs::read(&path).must();
    assert_row("corpus.0085-audio-only", &data, &path, ffmpeg_available());
}

#[test]
fn mid_stream_corruption_is_typed_and_recovery_scoped() {
    // The damage AC (doc 03 §4.5(d), overlap-scoped): a corrupted DATA
    // packet (bytes flipped AFTER the container CRC — damage inside
    // the codec, not the container) is a typed `vorbis.packet.decode`
    // refusal, and decoding RESUMES with the next packet (one poisoned
    // half-block at the seam, never a poisoned stream, never a panic).
    use codec_api::prelude::*;
    use codecs::audio::vorbis::VorbisDecoder;
    use demux::ogg::OggDemuxer;
    use media_types::packet::SharedBytes;
    use media_types::{EngineError, Packet, TrackId};
    use plugin_api::{DemuxEvent, Demuxer};

    let data = fixture_bytes("vorbis_sine_44100_stereo.ogg");
    let stream = demux::ogg::test_support::mem_stream(data.as_slice()).must();
    let mut demuxer = OggDemuxer::new(stream).must();
    let tracks = demuxer.start().must();
    let track = tracks
        .iter()
        .find(|t| t.codec.as_str() == "vorbis")
        .unwrap_or_else(|| unreachable!("no vorbis track"));
    let track_id: TrackId = track.id;
    let serial = *demuxer
        .serial_numbers()
        .first()
        .unwrap_or_else(|| unreachable!());
    let mut extra = Vec::new();
    for packet in demuxer.header_packets(serial) {
        extra.extend_from_slice(packet.as_slice());
    }

    let mut decoder = VorbisDecoder::new();
    let mut corrupt_at: Option<usize> = None;
    let mut index = 0usize;
    let mut typed_refusals = 0usize;
    let mut emitted = 0usize;
    let mut sink = vorbis::CaptureSink::default();
    loop {
        match demuxer.next_event().must() {
            DemuxEvent::Packet(packet) => {
                if packet.track != track_id {
                    continue;
                }
                index += 1;
                if decoder.output_format().is_none() {
                    let generation = packet.generation;
                    decoder
                        .configure(&DecoderConfig::new(
                            media_types::ids::CodecDescriptor::new("vorbis").must(),
                            generation,
                            ThreadBudget::default(),
                            None,
                            Some(SharedBytes::from_vec(extra.clone())),
                        ))
                        .must();
                }
                let packet = if index == 20 {
                    // The corruption: replace the 20th data packet's
                    // payload with an all-0xFF blast (a maximal-entropy
                    // garbage block — the first bit reads as a header
                    // type, the codebook lookup walks nonsense). A
                    // single flipped byte inside a Vorbis payload can
                    // still decode (the format is entropy-coded and
                    // locally robust — recorded honestly); the AC's
                    // damage path needs the guaranteed refusal.
                    let len = packet.data.as_slice().len();
                    let bytes = vec![0xFF; len.max(8)];
                    corrupt_at = Some(index);
                    Packet {
                        data: SharedBytes::from_vec(bytes),
                        ..packet
                    }
                } else {
                    packet
                };
                match decoder.decode(packet, &mut sink) {
                    Ok(DecodeOutcome::Emitted { frames }) => emitted += frames as usize,
                    // Zero-length packets are legal Ogg: consumed,
                    // nothing decodable.
                    Ok(DecodeOutcome::NeedMoreInput) => {}
                    Ok(other) => unreachable!("one packet = one block: {other:?}"),
                    Err(EngineError { context, .. }) => {
                        assert_eq!(
                            context.code, "vorbis.packet.decode",
                            "the corruption must be the typed packet refusal"
                        );
                        typed_refusals += 1;
                    }
                }
            }
            DemuxEvent::EndOfStream => break,
            _ => continue,
        }
    }
    match decoder.flush(&mut sink) {
        Ok(DecodeOutcome::Drained) => {}
        Ok(other) => unreachable!("flush: {other:?}"),
        Err(err) => unreachable!("flush after recovery: {err:?}"),
    }
    assert_eq!(corrupt_at, Some(20), "the corrupted packet was reached");
    assert_eq!(typed_refusals, 1, "exactly the poisoned block refused");
    // Recovery: every OTHER packet decoded (67 packets, one poisoned,
    // minus any zero-length packets which are NeedMoreInput).
    assert!(
        emitted >= 60,
        "decoding resumed after the seam (emitted {emitted} of 67 packets' blocks)"
    );
    eprintln!(
        "corruption verdict: 1 typed refusal at packet 20, {emitted} blocks emitted after/around it"
    );
}

/// 6.FIX-vorbis-headtrim regression (the defect ledger's follow-up to
/// 6.FIX-ogg-reanchor): the stream-head encoder delay (128 samples @
/// 44.1 kHz — 2,902 µs) rode the ring UN-TRIMMED because the head
/// block was emitted before the first container stamp could measure
/// it. The decoder now WITHHOLDS the head until the first stamped
/// page resolves the trim, then releases it minus the delay. The
/// multi-page emission therefore IS the granule timeline.
#[test]
fn stream_head_encoder_delay_is_trimmed_decoder_side() {
    // (a) Multi-page fixtures: the first data page is stamped
    // mid-stream, the hold resolves, the trim is applied.
    for name in ["vorbis_sine_44100_stereo.ogg", "vorbis_sine_8000_mono.ogg"] {
        let section = format!("fixture.{}", name.trim_end_matches(".ogg"));
        let reference = reference_section(&section);
        let run = decode_through_ogg(&fixture_bytes(name));
        let granule = usiz(reference, "final_granule") as i64;
        let delay = usiz(reference, "head_trim") as i64;
        let channels = run.headers.identification.channels as usize;
        let total: i64 = run
            .sink
            .buffers
            .iter()
            .map(|b| b.len() / (channels * 4))
            .sum::<usize>() as i64;
        // The emission is EXACTLY the valid timeline: the head delay
        // is gone. (Pre-fix: `granule + delay` — 66 278 / 9 856.)
        assert_eq!(total, granule, "{name}: head-trimmed emission length");
        // The valid timeline starts at pts 0. (Pre-fix: the first
        // Present frame landed at `raw − delay`, i.e. late by the
        // delay — the measured 2,902 µs position defect.)
        let first_present = run.sink.pts.iter().find(|t| **t != i64::MIN).copied();
        assert_eq!(first_present, Some(0), "{name}: first Present pts");
        // Every Present frame lies on [0, granule]: none of the
        // withheld head (the delay samples) and none past the EOS
        // granule survives into the emission.
        for (index, &ticks) in run.sink.pts.iter().enumerate() {
            if ticks == i64::MIN {
                continue;
            }
            assert!(ticks >= 0, "{name}: frame {index} pts {ticks} < 0");
            let len = run.sink.buffers[index].len() / (channels * 4);
            assert!(
                ticks + len as i64 <= granule,
                "{name}: frame {index} ends at {} past granule {granule}",
                ticks + len as i64
            );
        }
        eprintln!(
            "HEADTRIM {name}: emission {total} == granule (delay {delay} trimmed decoder-side)"
        );
    }
    // (b) Single-data-page fixtures: the only stamp is the EOS page,
    // which cannot separate delay from tail padding — the flush
    // releases the hold UN-TRIMMED (the pinned honest shape: an
    // all-Missing pts series, emission == the raw pin). The fix must
    // not drop or invent anything there.
    for name in ["vorbis_silence_48000_stereo.ogg", "vorbis_tagged_sine.ogg"] {
        let section = format!("fixture.{}", name.trim_end_matches(".ogg"));
        let reference = reference_section(&section);
        let run = decode_through_ogg(&fixture_bytes(name));
        let channels = run.headers.identification.channels as usize;
        assert!(
            run.sink.pts.iter().all(|t| *t == i64::MIN),
            "{name}: single-data-page pts must stay Missing"
        );
        let total: usize = run
            .sink
            .buffers
            .iter()
            .map(|b| b.len() / (channels * 4))
            .sum();
        assert_eq!(
            total,
            usiz(reference, "raw_samples"),
            "{name}: un-trimmed flush release (the pinned pre-fix shape)"
        );
    }
}
