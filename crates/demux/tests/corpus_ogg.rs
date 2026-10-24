//! The real-corpus Ogg differential (card 5.B1, the user's standing
//! mandate): every Ogg in the randomized real corpus (corpus-v4,
//! ffmpeg-authored bytes — 3 `.ogg` audio lanes + 1 `.ogv` video lane)
//! demuxed through the real access stack and compared against the
//! **measured ffmpeg truth** — stream parameters and packet counts
//! from `ffprobe -count_packets`, payload digests from ffmpeg's raw
//! dump (`ffmpeg -i <file> -map 0:a -c copy -f md5 -` for audio,
//! `-map 0:v` for the Theora `.ogv`).
//!
//! Every pinned value below was measured this session from the
//! corpus-v4 instance at `/tmp/my-project/corpus-v4/media` (ffmpeg
//! 7.1.5 / Lavf61.7.103, the same build that authored the corpus).
//! The corpus regenerates with fresh seeds: when it is absent (and no
//! `LUMEN_CORPUS` override is set) this suite **skips loudly** — the
//! per-file PASS/FAIL table for this instance lives in the worklog
//! entry, exactly as measured.
//!
//! # What is demux-domain and what is not (honest scope)
//!
//! Ogg carries no container-level channel-count or width/height
//! fields: Vorbis channels live in the Vorbis ID header packet,
//! Opus channels in OpusHead, Theora dimensions in the info header —
//! all **codec-domain** bytes we stash verbatim in `header_packets()`
//! (the decoder card consumes them; the worklog table records the
//! ffprobe-side values for review). The container-comparable surface
//! — codec identity, granule clock (rate/fps), EOS granule, packet
//! count, and the full payload digest — is asserted exactly here.

use std::path::PathBuf;

mod common;

use common::{md5_hex, open_via_input};
use demux::ogg::OggDemuxer;
use media_types::time::Stamp;
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;

/// One corpus file's ffmpeg-measured truth.
struct CorpusTruth {
    /// File name inside the corpus media dir.
    file: &'static str,
    /// ffprobe `codec_name` (the registry descriptor we must emit).
    codec: &'static str,
    /// The track kind (audio lanes + the one Theora video lane).
    kind: media_types::MediaKind,
    /// The granule clock's time base as (num, den) — one tick = one
    /// granule-domain sample at the stream's rate (Opus is always
    /// 48 kHz by RFC 7845; Theora ticks are frames at the info
    /// header's frame rate, here 1/2 fps).
    base: (u32, u32),
    /// ffprobe `duration_ts` — the EOS granule in track ticks.
    granule: i64,
    /// `ffprobe -count_packets nb_read_packets` (ffmpeg strips codec
    /// headers into extradata; we stash them — both sides count data
    /// packets only).
    packets: u64,
    /// `ffmpeg -map 0:<a|v> -c copy -f md5` (the raw packet dump).
    md5: &'static str,
}

/// The four real Ogg-family files of corpus-v4 (all measured this
/// session; ffprobe-side channels/dimensions for the record: 0047
/// opus mono, 0077 opus 5.1 (6ch), 0085 vorbis stereo, 0090 theora
/// 360×640 @ 1/2 fps).
const TRUTHS: [CorpusTruth; 4] = [
    CorpusTruth {
        file: "0047-audio-only.ogg",
        codec: "opus",
        kind: media_types::MediaKind::Audio,
        base: (1, 48_000),
        granule: 192_312,
        packets: 201,
        md5: "f2d264496a62b16f9d064f7ba7b8c955",
    },
    CorpusTruth {
        file: "0077-audio-only.ogg",
        codec: "opus",
        kind: media_types::MediaKind::Audio,
        base: (1, 48_000),
        granule: 192_312,
        packets: 201,
        md5: "fb151c11ff11c460fa420831ca0fd220",
    },
    CorpusTruth {
        file: "0085-audio-only.ogg",
        codec: "vorbis",
        kind: media_types::MediaKind::Audio,
        base: (1, 8_000),
        granule: 32_000,
        packets: 126,
        md5: "d0e0f86e6cd41d1d84fbe17f0ee64776",
    },
    CorpusTruth {
        file: "0090-ogv-video.ogv",
        codec: "theora",
        kind: media_types::MediaKind::Video,
        base: (2, 1), // 1/2 fps: one tick = one frame
        granule: 2,
        packets: 2,
        md5: "c0cd6bbf4fb3d61528923fc1f4f06d83",
    },
];

/// The corpus media dir: the `LUMEN_CORPUS` override, else the
/// sandbox default. `None` when neither exists (the loud skip).
fn corpus_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("LUMEN_CORPUS") {
        return Some(PathBuf::from(dir));
    }
    let default = PathBuf::from("/tmp/my-project/corpus-v4/media");
    default.is_dir().then_some(default)
}

#[test]
fn corpus_oggs_match_the_ffmpeg_truth() {
    let Some(dir) = corpus_dir() else {
        eprintln!(
            "corpus(ogg): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \
             — SKIP (the differential table for the 2026-10-24 instance is in the worklog)"
        );
        return;
    };
    let mut table = Vec::new();
    for truth in &TRUTHS {
        let path = dir.join(truth.file);
        assert!(path.is_file(), "corpus file missing: {path:?}");
        let stream = open_via_input(&path).must();
        let mut ogg = match OggDemuxer::new(stream) {
            Ok(ogg) => ogg,
            Err(err) => unreachable!("{} constructs: {err}", truth.file),
        };
        let tracks = match ogg.start() {
            Ok(tracks) => tracks,
            Err(err) => unreachable!("{} starts: {err}", truth.file),
        };
        assert_eq!(tracks.len(), 1, "{}", truth.file);
        let track = &tracks[0];
        assert_eq!(track.kind, truth.kind, "{}", truth.file);
        assert_eq!(track.codec.as_str(), truth.codec, "{}", truth.file);
        assert_eq!(
            track.timing.base.as_parts(),
            truth.base,
            "{}: the granule clock's time base",
            truth.file
        );

        // The EOS granule is the span end in track ticks.
        match track.timing.span.end {
            Stamp::Present(ts) => assert_eq!(ts.ticks, truth.granule, "{}: span end", truth.file),
            Stamp::Missing => unreachable!("{}: span end present", truth.file),
        }
        // The duration follows from the clock (e.g. 192_312 @ 48 kHz
        // = 4.0065 s; 2 frames @ 1/2 fps = 4.0 s).
        let duration = ogg
            .track_duration(track.id)
            .unwrap_or_else(|| unreachable!("{}: clock known", truth.file));
        let (num, den) = (u64::from(truth.base.0), u64::from(truth.base.1));
        let nanos = u64::try_from(
            u128::from(truth.granule.unsigned_abs()) * u128::from(num) * 1_000_000_000u128
                / u128::from(den),
        )
        .must();
        assert_eq!(
            duration,
            media_types::time::Duration::Known(media_types::time::Nanoseconds(nanos)),
            "{}: duration = granule × time base",
            truth.file
        );

        // The codec headers are stashed verbatim (decode-domain
        // parameters live there — channels/dimensions, see the module
        // docs): one serial, its headers present with the right magic.
        let serials = ogg.serial_numbers();
        assert_eq!(serials.len(), 1, "{}", truth.file);
        let headers = ogg.header_packets(serials[0]);
        assert!(!headers.is_empty(), "{}: headers stashed", truth.file);
        match truth.codec {
            "opus" => assert!(
                headers[0].as_ref().starts_with(b"OpusHead"),
                "{}",
                truth.file
            ),
            "vorbis" => assert!(
                headers[0]
                    .as_ref()
                    .starts_with(&[0x01, b'v', b'o', b'r', b'b']),
                "{}",
                truth.file
            ),
            "theora" => assert!(
                headers[0]
                    .as_ref()
                    .starts_with(&[0x80, b't', b'h', b'e', b'o']),
                "{}",
                truth.file
            ),
            _ => unreachable!("codec table is exhaustive"),
        }

        // Drain: payload parity with ffmpeg's raw dump, packet-count
        // parity with ffprobe, clean walk, one link.
        let mut payload = Vec::new();
        let mut reasons = Vec::new();
        let mut terminal = String::new();
        let mut packets = 0u64;
        loop {
            match ogg.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    packets += 1;
                    payload.extend_from_slice(&packet.data);
                }
                Ok(DemuxEvent::Recovered { reason, .. }) => reasons.push(reason),
                Ok(DemuxEvent::Discontinuity) => {}
                Ok(DemuxEvent::EndOfStream) => break,
                Err(err) => {
                    terminal = err.context.code.to_owned();
                    break;
                }
            }
        }
        let digest = md5_hex(&payload);
        assert_eq!(
            digest, truth.md5,
            "{}: payload parity vs ffmpeg raw dump",
            truth.file
        );
        assert_eq!(packets, truth.packets, "{}: packet count", truth.file);
        assert_eq!(
            ogg.packet_counts(),
            vec![(serials[0], truth.packets)],
            "{}: per-serial count",
            truth.file
        );
        assert!(
            terminal.is_empty(),
            "{}: clean EOS (got {terminal})",
            truth.file
        );
        assert!(
            reasons.is_empty(),
            "{}: no damage ({reasons:?})",
            truth.file
        );
        assert_eq!(ogg.links(), 1, "{}", truth.file);
        assert_eq!(ogg.granule_discontinuities(), 0, "{}", truth.file);
        assert_eq!(
            ogg.damage_total(),
            0,
            "{}: a clean corpus file walks clean",
            truth.file
        );
        table.push(format!(
            "PASS {} codec={} base={}/{} granule={} packets={} md5={} bytes={}",
            truth.file,
            truth.codec,
            num,
            den,
            truth.granule,
            packets,
            digest,
            payload.len()
        ));
    }
    for row in &table {
        eprintln!("corpus(ogg): {row}");
    }
    eprintln!(
        "corpus(ogg): {}/{} files PASS the ffmpeg differential",
        table.len(),
        TRUTHS.len()
    );
}

/// Every corpus Ogg (and the dirty/garbage siblings, when the harness
/// hands them over) survives the fuzz harness — the `+FB` no-panic/
/// no-hang/bounded-memory contract over real-world bytes. The
/// corpus's dirty/garbage lanes carry no `.ogg`-extension files this
/// instance, so the sweep feeds **all** of them (any bytes are legal
/// fuzz input — D-3 over the real corpus family) plus deterministic
/// truncation/flip sweeps over the four real Ogg lanes.
#[test]
fn corpus_oggs_survive_the_fuzz_harness() {
    let Some(dir) = corpus_dir() else {
        eprintln!("corpus(ogg): corpus absent — SKIP (fuzz sweep)");
        return;
    };
    for truth in &TRUTHS {
        let bytes = std::fs::read(dir.join(truth.file)).must();
        demux::ogg::fuzz::fuzz_demux_ogg(&bytes);
        // Deterministic mutation sweeps over the real bytes: every
        // 4 KiB-stride truncation prefix and single-bit flip.
        for cut in (0..bytes.len()).step_by(4096) {
            demux::ogg::fuzz::fuzz_demux_ogg(&bytes[..cut]);
        }
        for off in (0..bytes.len()).step_by(4096) {
            let mut mutated = bytes.clone();
            mutated[off] ^= 0x01;
            demux::ogg::fuzz::fuzz_demux_ogg(&mutated);
        }
    }
    // The dirty/garbage siblings (all of them — any bytes are legal
    // fuzz input; this instance has no ogg-labeled lanes there).
    for subdir in ["dirty", "garbage"] {
        let dir = dir.join(subdir);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut swept = 0usize;
        for entry in entries {
            let Ok(entry) = entry else {
                continue;
            };
            let path = entry.path();
            if path.is_file() {
                let Ok(bytes) = std::fs::read(&path) else {
                    continue;
                };
                demux::ogg::fuzz::fuzz_demux_ogg(&bytes);
                swept += 1;
            }
        }
        eprintln!("corpus(ogg): fuzz sweep over {subdir}/: {swept} files");
    }
}
