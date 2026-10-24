//! FLAC seeking suite (card 5.A1 AC): SEEKTABLE resolution + the
//! sample-accurate binary-search fallback, over real encoder bytes.
//!
//! The invariant under test is the resolution contract of
//! `seek::SeekPlanner`: for any target sample inside the stream, both
//! resolvers (the advisory SEEKTABLE path and the tableless bisection)
//! return the **containing** frame (`first_sample ≤ target <
//! first_sample + block_size`), and the bytes from that frame onward
//! decode to exactly the full decode's suffix from `first_sample` —
//! the end-to-end proof that a seek loses nothing and lands exactly.
//! A damaged seek table must degrade to the same answer via
//! bisection, never a wrong seek; a target past the stream's declared
//! sample range and a metadata-only stream are typed errors.

mod flac;

use codecs::audio::flac::{SeekPlanner, frame_header_at, parse_stream};
use flac::{concatenated, corpus_dir, decode_all, decode_all_from, fixture_bytes};
use media_types::ErrorKind;
use testkit::Must;

/// The SEEKTABLE carrier (44 frames, 6 points, a short tail block).
fn carrier() -> Vec<u8> {
    fixture_bytes("flac_s16_stereo_44100_seektable.flac")
}

/// Target samples that bracket the interesting cases: the stream
/// head, every seek point and its +1, gap midpoints, the frame right
/// before the short tail, deep inside the tail, and the final sample.
fn targets(facts: &codecs::audio::flac::StreamInfo, points: &[u64]) -> Vec<u64> {
    let mut targets = vec![0u64, 1];
    for point in points {
        targets.push(*point);
        targets.push(point + 1);
        targets.push(point + 4608 / 2);
    }
    let total = facts.total_samples.must();
    targets.push(total - 1);
    targets.push(total - 2306 / 2); // deep in the tail block
    targets.push(198_144 + 100); // inside the tail frame specifically
    targets.sort_unstable();
    targets.dedup();
    targets.retain(|t| *t < total);
    targets
}

/// Resolve `target` with both resolvers; assert containment, mutual
/// agreement, and the decode-suffix property.
fn assert_resolve(
    data: &[u8],
    facts: &codecs::audio::flac::StreamInfo,
    frames_start: u64,
    index: Option<&codecs::audio::flac::SeekIndex>,
    target: u64,
    full_pcm: &[u8],
    width: usize,
) {
    let with_table = SeekPlanner::new(data, facts, index, frames_start)
        .resolve(target)
        .must();
    let bisecting = SeekPlanner::new(data, facts, None, frames_start)
        .resolve(target)
        .must();
    assert_eq!(
        with_table, bisecting,
        "target {target}: both resolvers agree"
    );

    // Containment, judged against the frame's own header.
    let header = frame_header_at(data, with_table.byte_offset)
        .unwrap_or_else(|| unreachable!("target {target}: plan offset is a frame header"));
    assert_eq!(
        header.first_sample(facts),
        with_table.first_sample,
        "target {target}"
    );
    let end = with_table.first_sample + u64::from(header.block_size);
    assert!(
        with_table.first_sample <= target && target < end,
        "target {target}: containing frame [{}, {end})",
        with_table.first_sample
    );

    // The bytes from the plan's offset decode to the full decode's
    // suffix from the plan's first sample.
    let (_, sink) = decode_all_from(data, with_table.byte_offset);
    let tail = concatenated(&sink);
    let skip = (with_table.first_sample as usize) * facts.channels as usize * width;
    assert_eq!(
        tail,
        full_pcm[skip..],
        "target {target}: decode-from-offset equals the full-decode suffix"
    );
}

#[test]
fn seektable_and_binary_search_agree_across_the_stream() {
    let data = carrier();
    let layout = match parse_stream(&data) {
        Ok(layout) => layout,
        Err(err) => unreachable!("{err}"),
    };
    let facts = &layout.stream_info;
    let index = layout.seek_index.as_ref().must();
    let (.., sink) = decode_all(&data);
    let full_pcm = concatenated(&sink);

    let points: Vec<u64> = index.points.iter().map(|p| p.sample).collect();
    for target in targets(facts, &points) {
        assert_resolve(
            &data,
            facts,
            layout.frames_start,
            Some(index),
            target,
            &full_pcm,
            2,
        );
    }
}

/// A damaged SEEKTABLE row must fail its verification and fall back to
/// bisection — the same answer, never a wrong seek.
#[test]
fn corrupted_seektable_degrades_to_bisection() {
    let mut data = carrier();
    let layout = match parse_stream(&data) {
        Ok(layout) => layout,
        Err(err) => unreachable!("{err}"),
    };
    let facts = &layout.stream_info;
    let index = layout.seek_index.as_ref().must();
    let (.., sink) = decode_all(&data);
    let full_pcm = concatenated(&sink);

    // Corrupt the second point's byte offset (points past its frame).
    let mut pos = 4usize; // past "fLaC"
    let table_body;
    loop {
        let header: [u8; 4] = data[pos..pos + 4].try_into().must();
        let block_type = header[0] & 0x7f;
        let length =
            usize::try_from(u32::from_be_bytes([0, header[1], header[2], header[3]])).must();
        if block_type == 3 {
            table_body = pos + 4;
            break;
        }
        pos += 4 + length;
    }
    let row = table_body + 18; // second point
    let wrong_offset = u64::from_be_bytes(data[row + 8..row + 16].try_into().must()) + 13;
    data[row + 8..row + 16].copy_from_slice(&wrong_offset.to_be_bytes());

    // The parse still yields the (now lying) table; resolution must
    // still be correct — the verify rule rejects the row, bisection
    // lands the containing frame.
    let damaged = match parse_stream(&data) {
        Ok(layout) => layout,
        Err(err) => unreachable!("a lying row is advisory damage, not walk damage: {err}"),
    };
    let damaged_index = damaged.seek_index.as_ref().must();
    let points: Vec<u64> = index.points.iter().map(|p| p.sample).collect();
    for target in targets(facts, &points) {
        assert_resolve(
            &data,
            facts,
            layout.frames_start,
            Some(damaged_index),
            target,
            &full_pcm,
            2,
        );
    }
}

#[test]
fn past_end_and_frameless_streams_are_typed_errors() {
    let data = carrier();
    let layout = match parse_stream(&data) {
        Ok(layout) => layout,
        Err(err) => unreachable!("{err}"),
    };
    let facts = &layout.stream_info;
    let total = facts.total_samples.must();
    for target in [total, total + 1, u64::MAX / 2] {
        let err = match SeekPlanner::new(&data, facts, None, layout.frames_start).resolve(target) {
            Err(err) => err,
            Ok(plan) => unreachable!("target {target} is past the stream: {plan:?}"),
        };
        assert_eq!(err.context.code, "flac.seek.past_end", "target {target}");
        assert!(matches!(err.kind, ErrorKind::Corrupt { .. }));
    }

    // A metadata-only stream (no frames at all): typed no_frame. The
    // corpus provides the real one; a synthetic carrier is built by
    // truncating ours right after the metadata chain.
    let mut frameless = data.clone();
    frameless.truncate(layout.frames_start as usize);
    let frameless_layout = parse_stream(&frameless).must();
    let err = match SeekPlanner::new(
        &frameless,
        &frameless_layout.stream_info,
        None,
        frameless_layout.frames_start,
    )
    .resolve(0)
    {
        Err(err) => err,
        Ok(plan) => unreachable!("no frames to seek to: {plan:?}"),
    };
    assert_eq!(err.context.code, "flac.seek.no_frame");
}

/// The real corpus FLAC with audio (0046, 88.2 kHz 24-bit, no
/// SEEKTABLE — the bisection is its only resolver) seeks correctly,
/// tail frame included. SKIPs with a message when the corpus is
/// absent.
#[test]
fn corpus_stream_seeks_by_bisection_alone() {
    let Some(root) = corpus_dir() else {
        eprintln!("corpus: LUMEN_CORPUS unset and /tmp/my-project/corpus-v4 absent — SKIP");
        return;
    };
    let data = std::fs::read(root.join("media/0046-audio-only.flac")).must();
    let layout = match parse_stream(&data) {
        Ok(layout) => layout,
        Err(err) => unreachable!("{err}"),
    };
    let facts = &layout.stream_info;
    assert_eq!(
        layout.seek_index, None,
        "0046 carries no seektable — bisection only"
    );
    let (.., sink) = decode_all(&data);
    let full_pcm = concatenated(&sink);

    let total = facts.total_samples.must();
    let block = u64::from(facts.max_block_size);
    for target in [
        0,
        1,
        total / 2,
        total - block - 1,
        total - 544 - 1,
        total - 1,
    ] {
        assert_resolve(
            &data,
            facts,
            layout.frames_start,
            None,
            target,
            &full_pcm,
            3,
        );
    }
}
