//! The W5-14 regression suite — the `idx1` stream-scope contract
//! (JOB 1: the pre-existing defect W4-R1's worklog registered).
//!
//! A clean ffmpeg-authored AVI whose video codec sits outside the v0
//! filter (FMP4 — the 0203-avi-av shape) walks with the audio stream
//! intact while its video `strl` is damage-counted **once** at parse
//! time (`unsupported-video-handler`, a decision, 0 bytes). Pre-fix,
//! every `idx1` entry of that filtered stream was *re-counted* as
//! `idx1-entry-unknown-stream` damage (16 bytes per entry — the real
//! file's 101 entries walked with damage 1616): the damage channel
//! cried wolf on a clean file exactly the way `vprp` did before W4-R1.
//!
//! The contract this suite pins, both directions:
//!
//! 1. **Filtered-stream idx1 entries are silent** — the strl parse
//!    already counted the skip; the walk already skips the chunks
//!    silently; the idx1 says nothing more. `damage_total() == 0`
//!    beyond the parse-time decision note.
//! 2. **Genuinely-unknown ordinals are still damage** (the D-3
//!    dirty-world guarantee the fix must not weaken) — an idx1 entry
//!    whose stream ordinal no `strl` ever declared is noted exactly
//!    once, 16 bytes, and the walk survives to EOS with every
//!    deliverable packet intact.
//! 3. **The two discriminate** — in one file, the filtered stream's
//!    entries vanish while an undeclared ordinal's entry surfaces.

use demux::avi::AviDemuxer;
use demux::avi::test_support;
use media_types::Stamp;
use plugin_api::{DemuxEvent, Demuxer};

// ---------------------------------------------------------------------------
// A tiny AVI builder — the same byte rules the strl-advisory suite and
// the xtask fixture writer use (WORD-aligned chunks, patched LIST/RIFF
// sizes, idx1 offsets relative to the `movi` form FourCC). The axes it
// adds: strl streams of every disposition, and arbitrary idx1 entries.
// ---------------------------------------------------------------------------

fn chunk(out: &mut Vec<u8>, fourcc: &[u8; 4], payload: &[u8]) {
    out.extend_from_slice(fourcc);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
    if payload.len() & 1 == 1 {
        out.push(0);
    }
}

fn end_sized(out: &mut [u8], size_pos: usize) {
    let size = out.len() - size_pos - 4;
    out[size_pos..size_pos + 4].copy_from_slice(&(size as u32).to_le_bytes());
}

fn avih(streams: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(56);
    for value in [
        33_333u32, // dwMicroSecPerFrame
        0,         // dwMaxBytesPerSec
        0,         // dwPaddingGranularity
        0x10,      // AVIF_HASINDEX
        0,         // dwTotalFrames (advisory)
        0,         // dwInitialFrames
        streams, 0, // dwSuggestedBufferSize
        320, 240,
    ] {
        v.extend_from_slice(&value.to_le_bytes());
    }
    v.extend(&[0u8; 16]);
    v
}

/// A `strh` payload (56 bytes, the fields the demuxer reads).
fn strh(
    fcc: &[u8; 4],
    handler: &[u8; 4],
    scale: u32,
    rate: u32,
    length: u32,
    ssize: u32,
) -> Vec<u8> {
    let mut v = Vec::with_capacity(56);
    v.extend_from_slice(fcc);
    v.extend_from_slice(handler);
    v.extend_from_slice(&[0u8; 12]); // flags, priority, language, initial
    v.extend_from_slice(&scale.to_le_bytes());
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes()); // dwStart
    v.extend_from_slice(&length.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes()); // dwSuggestedBufferSize
    v.extend_from_slice(&0u32.to_le_bytes()); // dwQuality
    v.extend_from_slice(&ssize.to_le_bytes()); // dwSampleSize
    v.extend_from_slice(&[0u8; 8]); // rcFrame
    v
}

/// A BITMAPINFOHEADER `strf` naming `compression`.
fn strf_video(compression: &[u8; 4]) -> Vec<u8> {
    let mut v = Vec::with_capacity(40);
    v.extend_from_slice(&40u32.to_le_bytes());
    v.extend_from_slice(&320u32.to_le_bytes());
    v.extend_from_slice(&240u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&24u16.to_le_bytes());
    v.extend_from_slice(compression);
    v.extend(&[0u8; 20]);
    v
}

/// A WAVE_FORMAT_PCM `strf` (stereo 16-bit 48 kHz, block align 4).
fn strf_pcm() -> Vec<u8> {
    let mut v = Vec::with_capacity(16);
    v.extend_from_slice(&1u16.to_le_bytes()); // wFormatTag
    v.extend_from_slice(&2u16.to_le_bytes()); // nChannels
    v.extend_from_slice(&48_000u32.to_le_bytes()); // nSamplesPerSec
    v.extend_from_slice(&192_000u32.to_le_bytes()); // nAvgBytesPerSec
    v.extend_from_slice(&4u16.to_le_bytes()); // nBlockAlign
    v.extend_from_slice(&16u16.to_le_bytes()); // wBitsPerSample
    v
}

/// The strl dispositions the suite needs.
enum Strl {
    /// `vids`+`MJPG` — inside the v0 filter, kept.
    MjpgVideo,
    /// `vids`+`FMP4` — a real video stream outside the filter (the
    /// 0203-avi-av shape): declared, filtered, parse-time counted.
    Fmp4Video,
    /// `auds`+PCM — kept.
    PcmAudio,
}

/// One idx1 entry spec: (tag, which movi chunk it points at, size).
/// Entries are emitted in listed order — the builder hands the tests
/// position-ordered lists so no out-of-order note rides along.
struct IdxEntry {
    tag: &'static [u8; 4],
    chunk: usize,
    size: u32,
}

/// Build the AVI: `streams`' strl lists, the interleaved `movi`
/// (`(tag, payload)` per chunk), and one idx1 entry per `entries`.
fn build_avi(streams: &[Strl], movi: &[(&[u8; 4], Vec<u8>)], entries: &[IdxEntry]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"RIFF");
    let riff_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"AVI ");

    // LIST hdrl: avih + one LIST strl per stream.
    out.extend_from_slice(b"LIST");
    let hdrl_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"hdrl");
    chunk(&mut out, b"avih", &avih(streams.len() as u32));
    for stream in streams {
        out.extend_from_slice(b"LIST");
        let strl_size_pos = out.len();
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(b"strl");
        match *stream {
            Strl::MjpgVideo => {
                chunk(&mut out, b"strh", &strh(b"vids", b"MJPG", 1, 30, 3, 0));
                chunk(&mut out, b"strf", &strf_video(b"MJPG"));
            }
            Strl::Fmp4Video => {
                chunk(&mut out, b"strh", &strh(b"vids", b"FMP4", 1, 30, 3, 0));
                chunk(&mut out, b"strf", &strf_video(b"FMP4"));
            }
            Strl::PcmAudio => {
                chunk(
                    &mut out,
                    b"strh",
                    &strh(b"auds", b"\0\0\0\0", 1, 48_000, 3, 4),
                );
                chunk(&mut out, b"strf", &strf_pcm());
            }
        }
        end_sized(&mut out, strl_size_pos);
    }
    end_sized(&mut out, hdrl_size_pos);

    // LIST movi.
    let movi_form_pos = (out.len() + 8) as u64;
    out.extend_from_slice(b"LIST");
    let movi_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"movi");
    let mut chunk_positions: Vec<u64> = Vec::with_capacity(movi.len());
    for (tag, payload) in movi {
        chunk_positions.push(out.len() as u64);
        chunk(&mut out, tag, payload);
    }
    end_sized(&mut out, movi_size_pos);

    // idx1: dwOffset relative to the movi form FourCC, all keyframes.
    let mut idx = Vec::with_capacity(entries.len() * 16);
    for entry in entries {
        idx.extend_from_slice(entry.tag);
        idx.extend_from_slice(&0x10u32.to_le_bytes());
        let rel = chunk_positions[entry.chunk] - movi_form_pos;
        idx.extend_from_slice(&(rel as u32).to_le_bytes());
        idx.extend_from_slice(&entry.size.to_le_bytes());
    }
    chunk(&mut out, b"idx1", &idx);
    end_sized(&mut out, riff_size_pos);
    out
}

// ---------------------------------------------------------------------------
// Drivers
// ---------------------------------------------------------------------------

fn demuxer_over_bytes(bytes: Vec<u8>) -> AviDemuxer {
    let stream = match test_support::mem_stream(bytes) {
        Ok(stream) => stream,
        Err(err) => unreachable!("mem stream: {err}"),
    };
    match AviDemuxer::new(stream) {
        Ok(demuxer) => demuxer,
        Err(err) => unreachable!("demuxer over memory: {err}"),
    }
}

/// One packet's observable row: stream ordinal, ticks on its own base,
/// raw payload bytes.
type PacketRow = (u32, i64, Vec<u8>);

/// One surfaced damage record: skipped bytes, pinned reason.
type DamageRow = (u64, &'static str);

/// Walk to EOS, collecting every packet row and damage record. Returns
/// only at EOS: these walks are bounded and well-formed by
/// construction, so any error is a contract violation, not a case.
fn walk(avi: &mut AviDemuxer) -> (Vec<PacketRow>, Vec<DamageRow>) {
    let mut rows = Vec::new();
    let mut reasons = Vec::new();
    loop {
        match avi.next_event() {
            Ok(DemuxEvent::Packet(packet)) => {
                let ticks = match packet.pts {
                    Stamp::Present(ts) => ts.ticks,
                    Stamp::Missing => unreachable!("stamps present by construction"),
                };
                rows.push((
                    packet.track.handle(),
                    ticks,
                    packet.data.as_slice().to_vec(),
                ));
            }
            Ok(DemuxEvent::Recovered { skipped, reason }) => reasons.push((skipped, reason)),
            Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break,
            Err(err) => unreachable!("the idx1-scope walk is bounded and clean: {err}"),
        }
    }
    (rows, reasons)
}

/// The interleaved FMP4+PCM movi (three frames each, payloads
/// sample-aligned for the audio).
fn fmp4_pcm_movi() -> Vec<(&'static [u8; 4], Vec<u8>)> {
    let frame = |index: u8| vec![index; 12];
    let pcm = |index: u8| vec![index; 8]; // 2 sample frames of align 4
    vec![
        (b"00dc", frame(0xa0)),
        (b"01wb", pcm(0x50)),
        (b"00dc", frame(0xa1)),
        (b"01wb", pcm(0x51)),
        (b"00dc", frame(0xa2)),
        (b"01wb", pcm(0x52)),
    ]
}

/// The audio truth of the FMP4+PCM shape: ticks 0/2/4 on the 1/48 000
/// base, the pcm payloads byte-exact.
fn pcm_rows() -> Vec<PacketRow> {
    vec![
        (1, 0, vec![0x50; 8]),
        (1, 2, vec![0x51; 8]),
        (1, 4, vec![0x52; 8]),
    ]
}

// ---------------------------------------------------------------------------
// 1. Filtered-stream idx1 entries are silent
// ---------------------------------------------------------------------------

/// The 0203-avi-av shape: FMP4 video (outside the v0 codec filter) +
/// PCM audio, every movi chunk indexed. Pre-fix this walked with
/// `Recovered { skipped: 96, reason: "idx1-entry-unknown-stream" }`
/// (6 entries × 16 bytes; the real file: 101 × 16 = 1616). Post-fix
/// the ONLY damage is the parse-time filter decision — zero bytes, one
/// event — and the audio walks byte-exact.
#[test]
fn filtered_stream_idx1_entries_are_silent() {
    let movi = fmp4_pcm_movi();
    let entries: Vec<IdxEntry> = (0..movi.len())
        .map(|chunk| IdxEntry {
            tag: match chunk % 2 {
                0 => b"00dc",
                _ => b"01wb",
            },
            chunk,
            size: movi[chunk].1.len() as u32,
        })
        .collect();
    let bytes = build_avi(&[Strl::Fmp4Video, Strl::PcmAudio], &movi, &entries);
    let mut avi = demuxer_over_bytes(bytes);
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 1, "only the PCM stream survives the filter"),
        Err(err) => unreachable!("the filtered-video shape starts: {err}"),
    }
    let (rows, reasons) = walk(&mut avi);
    assert_eq!(
        reasons,
        vec![(0, "unsupported-video-handler")],
        "the parse-time filter decision, once — nothing from idx1"
    );
    assert_eq!(
        avi.damage_total(),
        0,
        "a decision, not bytes: the ledger stays empty"
    );
    assert_eq!(rows, pcm_rows(), "every audio packet walks, byte-exact");
}

// ---------------------------------------------------------------------------
// 2. Genuinely-unknown ordinals are still damage
// ---------------------------------------------------------------------------

/// An idx1 entry whose stream ordinal (5) no `strl` ever declared is
/// noted exactly once — 16 bytes, the established per-entry convention
/// — and the walk survives it with every frame intact. This is the
/// non-weakening pin: the silent-skip rule above must be scoped to
/// *declared* streams, never to everything unknown.
#[test]
fn genuinely_unknown_idx1_stream_is_damage() {
    let movi: Vec<(&[u8; 4], Vec<u8>)> = vec![
        (b"00dc", vec![0xa0; 5]),
        (b"00dc", vec![0xa1; 7]),
        (b"00dc", vec![0xa2; 6]),
    ];
    // Position-ordered (the unknown entry shares chunk 2's position):
    // no out-of-order note rides along — the suite pins exactly one
    // damage event.
    let entries = vec![
        IdxEntry {
            tag: b"00dc",
            chunk: 0,
            size: 5,
        },
        IdxEntry {
            tag: b"00dc",
            chunk: 1,
            size: 7,
        },
        IdxEntry {
            tag: b"05dc",
            chunk: 2,
            size: 4,
        },
        IdxEntry {
            tag: b"00dc",
            chunk: 2,
            size: 6,
        },
    ];
    let bytes = build_avi(&[Strl::MjpgVideo], &movi, &entries);
    let mut avi = demuxer_over_bytes(bytes);
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 1, "one video track"),
        Err(err) => unreachable!("an unknown idx1 entry never blocks start: {err}"),
    }
    // Damage-record-first: the Recovered row leads the event stream.
    match avi.next_event() {
        Ok(DemuxEvent::Recovered { skipped, reason }) => {
            assert_eq!(skipped, 16, "one entry, the 16-byte convention");
            assert_eq!(
                reason, "idx1-entry-unknown-stream",
                "the reason string is the pinned unknown-stream signal"
            );
        }
        other => unreachable!("the damage record leads: {other:?}"),
    }
    let (rows, reasons) = walk(&mut avi);
    assert!(
        reasons.is_empty(),
        "exactly one note — no double counting after the surfaced record: {reasons:?}"
    );
    assert_eq!(avi.damage_total(), 16, "the ledger carries the entry");
    assert_eq!(
        rows,
        vec![
            (0, 0, vec![0xa0; 5]),
            (0, 1, vec![0xa1; 7]),
            (0, 2, vec![0xa2; 6]),
        ],
        "every frame still walks, byte-exact"
    );
}

// ---------------------------------------------------------------------------
// 3. The two discriminate in one file
// ---------------------------------------------------------------------------

/// FMP4 video + PCM audio, with the filtered stream's entries AND one
/// undeclared-ordinal (7) entry in the same idx1: only the undeclared
/// entry surfaces. The filtered stream's entries vanish silently while
/// the genuinely-unknown one is still counted — the discrimination the
/// fix exists to make.
#[test]
fn filtered_and_unknown_idx1_entries_discriminate() {
    let movi = fmp4_pcm_movi();
    let mut entries: Vec<IdxEntry> = Vec::new();
    for (index, (tag, payload)) in movi.iter().enumerate() {
        // The undeclared-ordinal entry rides in position order (it
        // shares chunk 4's position — no out-of-order note).
        if index == 4 {
            entries.push(IdxEntry {
                tag: b"07dc",
                chunk: 4,
                size: 4,
            });
        }
        entries.push(IdxEntry {
            tag,
            chunk: index,
            size: payload.len() as u32,
        });
    }
    let bytes = build_avi(&[Strl::Fmp4Video, Strl::PcmAudio], &movi, &entries);
    let mut avi = demuxer_over_bytes(bytes);
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 1, "only the PCM stream survives"),
        Err(err) => unreachable!("mixed dispositions start: {err}"),
    }
    let (rows, reasons) = walk(&mut avi);
    assert_eq!(
        reasons,
        vec![
            (0, "unsupported-video-handler"),
            (16, "idx1-entry-unknown-stream"),
        ],
        "the parse-time decision plus exactly the undeclared entry"
    );
    assert_eq!(avi.damage_total(), 16, "only the unknown entry's bytes");
    assert_eq!(rows, pcm_rows(), "every audio packet walks, byte-exact");
}
