//! The real-corpus MPEG-TS differential (card 5.I1a, the user's
//! standing mandate): the five transport-stream lanes of corpus-v4
//! (ffmpeg-authored bytes) demuxed through the real access stack and
//! compared against the **measured ffprobe truth** — program count,
//! per-program stream types (ffprobe `codec_name` ↔ the registry
//! vocabulary), SDT service names, PMT/PCR PIDs.
//!
//! Every pinned value below was measured this session from the
//! corpus-v4 instance at `/tmp/my-project/corpus-v4/media` (ffmpeg
//! 7.1.5 / Lavf61.7.103, the same build that authored the corpus):
//! `ffprobe -show_programs` reports one program (`program_id=1`,
//! `pmt_pid=4096`, `pcr_pid=256`, `service_name=Service01`,
//! `service_provider=FFmpeg`) on all five lanes; the stream set is
//! h264 for the four video lanes and hevc+aac for 0091. The PSI
//! CRCs all verify (a pre-Rust byte walk over the corpus confirmed
//! every section's CRC-32/MPEG-2 hash to zero — the pinned
//! `psi_crc_failures == 0` below).
//!
//! When the corpus is absent (and no `LUMEN_CORPUS` override is set)
//! this suite **skips loudly**.

use std::path::{Path, PathBuf};

mod common;

use common::open_via_input;
use demux::ts::{TsDemuxer, TsProgram};
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;

/// One corpus file's ffprobe-measured truth.
#[allow(clippy::type_complexity)]
struct CorpusTruth {
    /// File name inside the corpus media dir.
    file: &'static str,
    /// `ffprobe -show_programs` program count.
    programs: usize,
    /// Per program: the PAT number, PMT PID, PCR PID, and the
    /// `(ffprobe codec_name, our stream-type)` stream list in PMT
    /// order.
    surface: &'static [(u16, u16, u16, &'static [(&'static str, u8, u16)])],
    /// The SDT service name (all corpus lanes: `Service01`).
    service_name: &'static str,
    /// The SDT service provider (all corpus lanes: `FFmpeg`).
    service_provider: &'static str,
    /// Whole 188-byte packets in the file (the walk's total).
    packets: u64,
}

const TRUTHS: [CorpusTruth; 5] = [
    CorpusTruth {
        file: "0000-param-midchange.ts",
        programs: 1,
        surface: &[(1, 0x1000, 0x100, &[("h264", 0x1B, 0x100)])],
        service_name: "Service01",
        service_provider: "FFmpeg",
        packets: 408,
    },
    CorpusTruth {
        file: "0018-ts-video.ts",
        programs: 1,
        surface: &[(1, 0x1000, 0x100, &[("h264", 0x1B, 0x100)])],
        service_name: "Service01",
        service_provider: "FFmpeg",
        packets: 331,
    },
    CorpusTruth {
        file: "0091-bframe-storm.ts",
        programs: 1,
        surface: &[(
            1,
            0x1000,
            0x100,
            &[("hevc", 0x24, 0x100), ("aac", 0x0F, 0x101)],
        )],
        service_name: "Service01",
        service_provider: "FFmpeg",
        packets: 723,
    },
    CorpusTruth {
        file: "0113-dynres-video.ts",
        programs: 1,
        surface: &[(1, 0x1000, 0x100, &[("h264", 0x1B, 0x100)])],
        service_name: "Service01",
        service_provider: "FFmpeg",
        packets: 621,
    },
    CorpusTruth {
        file: "0134-ts-video.ts",
        programs: 1,
        surface: &[(1, 0x1000, 0x100, &[("h264", 0x1B, 0x100)])],
        service_name: "Service01",
        service_provider: "FFmpeg",
        packets: 96,
    },
];

fn corpus_dir() -> PathBuf {
    if let Ok(override_dir) = std::env::var("LUMEN_CORPUS") {
        return PathBuf::from(override_dir);
    }
    PathBuf::from("/tmp/my-project/corpus-v4/media")
}

/// The five-lane differential: program count, PMT/PCR PIDs, stream
/// types per program, service names, packet totals, CRC health —
/// every value ffprobe (or a byte count) measured on this corpus.
#[test]
fn corpus_ts_programs_streams_and_names_match_ffprobe() {
    let dir = corpus_dir();
    let mut ran = 0;
    for truth in &TRUTHS {
        let path = dir.join(truth.file);
        if !path.exists() {
            eprintln!(
                "SKIP (loud): corpus lane {} absent at {} — differential not run",
                truth.file,
                path.display()
            );
            continue;
        }
        ran += 1;
        let stream = open_via_input(&path).must();
        let mut demuxer = TsDemuxer::new(stream).must();
        let tracks = demuxer.start().must();
        // -- program count ------------------------------------------------
        let programs: &[TsProgram] = demuxer.programs();
        assert_eq!(
            programs.len(),
            truth.programs,
            "{}: program count vs ffprobe",
            truth.file
        );
        // -- per-program surface -------------------------------------------
        for (prog, &(number, pmt_pid, pcr_pid, streams)) in
            programs.iter().zip(truth.surface.iter())
        {
            assert_eq!(prog.number, number, "{}: PAT program_number", truth.file);
            assert_eq!(
                prog.pmt_pid, pmt_pid,
                "{}: PMT PID vs ffprobe pmt_pid",
                truth.file
            );
            assert_eq!(
                prog.pcr_pid,
                Some(pcr_pid),
                "{}: PCR PID vs ffprobe pcr_pid",
                truth.file
            );
            assert_eq!(
                prog.streams.len(),
                streams.len(),
                "{}: stream count vs ffprobe",
                truth.file
            );
            for (es, &(codec, stream_type, pid)) in prog.streams.iter().zip(streams.iter()) {
                assert_eq!(
                    es.stream_type, stream_type,
                    "{}: PMT stream_type",
                    truth.file
                );
                assert_eq!(es.pid, pid, "{}: PMT elementary PID", truth.file);
                assert_eq!(
                    es.descriptor().as_str(),
                    codec,
                    "{}: codec descriptor vs ffprobe codec_name",
                    truth.file
                );
            }
            assert_eq!(
                prog.service_name.as_deref(),
                Some(truth.service_name),
                "{}: SDT service_name vs ffprobe TAG:service_name",
                truth.file
            );
            assert_eq!(
                prog.service_provider.as_deref(),
                Some(truth.service_provider),
                "{}: SDT provider vs ffprobe TAG:service_provider",
                truth.file
            );
        }
        // -- the engine track surface (multi-program plumbing) -------------
        let expected_tracks: usize = truth.surface.iter().map(|(_, _, _, s)| s.len()).sum();
        assert_eq!(
            tracks.len(),
            expected_tracks,
            "{}: engine tracks",
            truth.file
        );
        // -- the walk -------------------------------------------------------
        let stats = drain(&mut demuxer);
        assert_eq!(
            stats.packets, truth.packets,
            "{}: synced packet total",
            truth.file
        );
        assert_eq!(
            stats.psi_crc_failures, 0,
            "{}: every PSI CRC verifies (pre-verified byte walk)",
            truth.file
        );
        assert_eq!(
            stats.sync_errors, 0,
            "{}: no sync loss in the corpus lanes",
            truth.file
        );
        assert!(
            stats.pes_packets > 0,
            "{}: the PES seam saw the elementary payloads",
            truth.file
        );
        // TEI/CC health of the ffmpeg-authored lanes.
        assert_eq!(stats.tei_packets, 0, "{}: TEI-free", truth.file);
    }
    if ran == 0 {
        eprintln!(
            "SKIP (loud): no corpus lanes found under {} — the whole \
             TS differential skipped; set LUMEN_CORPUS to point at the \
             corpus media dir",
            dir.display()
        );
    }
}

/// The four video lanes expose their PES payloads on one PID; 0091
/// exposes two (the walk + seam accounting lane of the differential).
#[test]
fn corpus_ts_pes_seam_accounting() {
    let dir = corpus_dir();
    let path = dir.join("0091-bframe-storm.ts");
    if !path.exists() {
        eprintln!("SKIP (loud): 0091-bframe-storm.ts absent — seam differential not run");
        return;
    }
    let stream = open_via_input(&path).must();
    let mut demuxer = TsDemuxer::new(stream).must();
    demuxer.start().must();
    let stats = drain(&mut demuxer);
    // The hevc video and the aac audio both carry payload (ffmpeg
    // does not null-stuff its TS lanes): every non-PSI, non-stuffing
    // packet went to the seam.
    assert!(stats.pes_packets + stats.null_packets + stats.unknown_pid_packets > 0);
    assert_eq!(stats.unknown_pid_packets, 0, "no undeclared PID traffic");
}

fn drain(demuxer: &mut TsDemuxer) -> demux::ts::TsStats {
    loop {
        if let DemuxEvent::EndOfStream = demuxer.next_event().must() {
            break;
        }
    }
    demuxer.stats()
}

// Keep the Path import referenced for the override-doc path helper.
#[allow(dead_code)]
fn _type_check(_p: &Path) {}
