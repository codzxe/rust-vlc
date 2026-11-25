//! `decode-bench` — the Lumen runner of the ffmpeg speed-parity
//! benchmark (card W4-R2; the runner half of `tools/corpus/
//! speed_bench.py --runner lumen='lumen decode-bench {file}'`,
//! the gap 4.CORPUS-V3 registered).
//!
//! # What it measures
//!
//! Open → demux → **decode to null** over the real pipeline pieces —
//! the very [`crate::demux_host::open_source`] path the player uses
//! (canonicalize → `input` access → `media-probe` → the container
//! registry → the per-container facts pre-pass), then the decoders
//! **THE dispatch selects** ([`crate::pipeline_host::HostDecoderProvider`]
//! — the single registry `lumen open` wires; the bench keeps no codec
//! list of its own), behind the [`codec_api::Decoder`] contract, fed
//! straight from [`Demuxer::next_event`] — no engine, no queues, no
//! devices: the benchmark isolates open + demux + decode, the stages
//! whose speed ffmpeg's `-f null -` run exercises.
//!
//! # The BENCH line (the contract with speed_bench.py)
//!
//! stdout carries exactly one machine-parseable line on success:
//!
//! ```text
//! BENCH file=<path> bytes=<n> v_frames=<n> v_decoded=<n> a_frames=<n> a_decoded=<n> wall_s=<x.xxx> decode_s=<x.xxx>
//! ```
//!
//! `file` is the argv path verbatim (speed_bench substitutes an
//! absolute `{file}`; paths with whitespace are outside the contract).
//! `wall_s` is the whole process wall ([`mark_process_start`] seeds
//! the origin from `main`; the library/test path falls back to the
//! bench's own start), `decode_s` starts after open (demuxer started
//! and decoders configured) and covers the packet loop plus flush.
//! Every failure is a typed `error:` line on stderr and a nonzero
//! exit — never a BENCH line, never a panic (D-3: bounded parsing,
//! graceful errors; the adversarial corpus is an expected input).
//!
//! # Per-stream coverage (honest)
//!
//! The first video stream and the first audio stream are benched;
//! their packet/decoded counts are the line's `v_*`/`a_*` pairs.
//! The limbs come from the dispatch, so a codec family outside its
//! arms answers with the dispatch's own typed refusal ("no decoder
//! for `<codec>` in this build", the landed inventory named): as the
//! coverage note when the other kind still benches, and — when
//! nothing at all was benched — as the command's typed failure (no
//! BENCH line, nonzero exit: a bench that measured nothing is not a
//! successful zero). Human notes ride stderr only.
//!
//! # Decode-loop discipline (D-1)
//!
//! The loop introduces zero per-frame allocations of its own: packets
//! move by value into `decode`, the demuxer's payload slot pool
//! recycles, and the sink drops frames without touching them.
//! Per-frame damage is concealed exactly like the engine does it
//! (`playback-engine`'s `per_frame_damage` rule + the
//! 1000-consecutive budget), so dirty files stay bounded.

use std::io::Write;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink, ThreadBudget};
use commands::catalog::types::SourceSpec;
use commands::{CommandError, ErrorCode, Stage};
use media_types::error::DecodeCause;
use media_types::{EngineError, ErrorKind, Frame, GenerationId, MediaKind, Packet};
use playback_engine::DecoderProvider;
use plugin_api::DemuxEvent;

use crate::demux_host;
use crate::exit;
use crate::pipeline_host::HostDecoderProvider;

/// The maximum run of consecutive concealed per-frame damage before
/// the bench gives up (mirrors `playback-engine`'s
/// `VIDEO_CONCEALMENT_BUDGET` — one budget for both limbs by design;
/// a stream whose every packet fails is not being benchmarked, it is
/// broken).
const CONCEALMENT_BUDGET: u32 = 1000;

/// The per-limb end-of-stream flush budget: flush calls before the
/// decoder's `Drained` answer is taken as final (the engine's own
/// flush-to-`Drained` loop shape, `TickBudget::flush_calls_per_limb`
/// per tick at EOF — the bench drains once, so it carries its own
/// bound). The stateless codecs the bench originally hand-coded
/// (MJPEG/PCM) answered the first flush; the dispatch's inter-frame
/// codecs (h264 reorder) emit held frames on flush first and answer
/// `Drained` after.
const FLUSH_BUDGET: u32 = 64;

/// The wall-clock origin, seeded by `main` (true process wall). The
/// library/test path (no `main`) falls back to first use — the
/// in-memory tests then measure from their own `run` call.
static PROCESS_START: OnceLock<Instant> = OnceLock::new();

/// Seed the process wall origin — `main`'s first statement.
pub fn mark_process_start() {
    let _ = PROCESS_START.set(Instant::now());
}

/// The seeded origin (or now, when the library is driven without a
/// `main` — the in-memory test path).
fn process_start() -> Instant {
    PROCESS_START.get().copied().unwrap_or_else(Instant::now)
}

/// The decode-to-null frame sink: accepts every frame the decoder
/// contract requires and drops it untouched (this runner's `-f null
/// -`). The emitted-count comes back on the [`DecodeOutcome`], so the
/// sink itself does no per-frame work beyond the safety counter.
struct NullSink {
    /// Frames accepted (cross-checked against the outcomes' sums).
    frames: u64,
}

impl FrameSink for NullSink {
    fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
        self.frames += 1;
        Ok(())
    }
}

/// One benched stream: the decoder plus its counters.
struct Limb {
    decoder: Box<dyn Decoder>,
    /// The demuxed track this limb consumes.
    track: media_types::TrackId,
    /// Packets fed.
    frames: u64,
    /// Frames the decoder emitted.
    decoded: u64,
    /// Per-frame damage concealed (kept going).
    concealed: u64,
}

impl Limb {
    /// Feed one packet (the D-1 loop body: move, decode, count —
    /// nothing else).
    fn feed(
        &mut self,
        packet: Packet,
        sink: &mut NullSink,
        consecutive: &mut u32,
    ) -> Result<(), CommandError> {
        self.frames += 1;
        match self.decoder.decode(packet, sink) {
            Ok(DecodeOutcome::Emitted { frames }) => {
                self.decoded += u64::from(frames);
                *consecutive = 0;
            }
            Ok(DecodeOutcome::Recovered { .. }) => {
                // Damage survived (the decoder stayed usable): damage
                // streak accounting only — the run continues.
                *consecutive = consecutive.saturating_add(1);
            }
            Ok(DecodeOutcome::NeedMoreInput | DecodeOutcome::Drained) => {}
            Err(error) if per_frame_damage(&error) => {
                self.concealed += 1;
                *consecutive = consecutive.saturating_add(1);
                if *consecutive >= CONCEALMENT_BUDGET {
                    return Err(CommandError::new(
                        Stage::Decode,
                        ErrorCode::ResourceLimit,
                        format!(
                            "decode-bench: {} consecutive damaged packets — concealment budget exhausted",
                            CONCEALMENT_BUDGET
                        ),
                    )
                    .with_detail("budget", CONCEALMENT_BUDGET.to_string())
                    .with_detail("last", error.to_string()));
                }
            }
            Err(error) => return Err(demux_host::wire_error(&error)),
        }
        Ok(())
    }

    /// Drain at end-of-stream to the decoder's own `Drained` answer,
    /// bounded by [`FLUSH_BUDGET`] (a reordered video limb can hold
    /// frames past the last packet — a single flush call would
    /// under-count them; stateless codecs answer immediately).
    fn drain(&mut self, sink: &mut NullSink) -> Result<(), CommandError> {
        for _ in 0..FLUSH_BUDGET {
            match self.decoder.flush(sink) {
                Ok(DecodeOutcome::Emitted { frames }) => {
                    self.decoded += u64::from(frames);
                }
                Ok(DecodeOutcome::Recovered { .. }) => {}
                // `Drained` is final; `NeedMoreInput` at end-of-stream
                // means no more input exists — the residue stays the
                // decoder's honest answer either way.
                Ok(DecodeOutcome::Drained | DecodeOutcome::NeedMoreInput) => break,
                Err(error) => return Err(demux_host::wire_error(&error)),
            }
        }
        Ok(())
    }
}

/// Whether an error is **per-frame damage** — the class the engine
/// conceals on a limb instead of tearing the session down (the
/// `per_frame_damage` rule of `playback-engine`'s pipeline.rs,
/// mirrored locally: the engine's own is crate-private).
fn per_frame_damage(error: &EngineError) -> bool {
    matches!(
        &error.kind,
        ErrorKind::Corrupt { .. }
            | ErrorKind::Decoder {
                cause: DecodeCause::Bitstream
            }
    )
}

/// What one successful bench measured.
struct BenchReport {
    bytes: u64,
    v_frames: u64,
    v_decoded: u64,
    a_frames: u64,
    a_decoded: u64,
    wall_s: f64,
    decode_s: f64,
    v_concealed: u64,
    a_concealed: u64,
    demux_recovered: u64,
    /// The per-stream coverage notes (data — [`run_bench`] prints
    /// them to stderr; see the module docs).
    notes: Vec<String>,
    /// The probe's container id (the coverage note's trailer).
    container: String,
}

/// The `decode-bench` host verb: run the bench, print the BENCH line
/// (stdout) or the typed error (stderr), answer the exit code.
///
/// Never panics — every failure path is a typed [`CommandError`]
/// through the one honest printer, including garbage input (the
/// probe's refusal), mid-walk container damage (the demuxer's
/// sticky fatal) and a source no dispatch decoder can bench (the
/// dispatch's own refusal — never a fabricated zero line).
pub fn run_bench<W: Write, E: Write>(source: &str, out: &mut W, err: &mut E) -> u8 {
    match measure(source) {
        Ok(report) => {
            // The per-stream coverage note: stderr only (stdout stays
            // the one machine line).
            let _ = writeln!(
                err,
                "bench: {} (container {})",
                report.notes.join("; "),
                report.container
            );
            let _ = writeln!(out, "{}", bench_line(&report, source));
            if report.v_concealed + report.a_concealed + report.demux_recovered > 0 {
                let _ = writeln!(
                    err,
                    "bench: concealed {} video + {} audio decode errors; {} demux \
                     recoveries (coverage: video {}/{}, audio {}/{})",
                    report.v_concealed,
                    report.a_concealed,
                    report.demux_recovered,
                    report.v_decoded,
                    report.v_frames,
                    report.a_decoded,
                    report.a_frames
                );
            }
            0
        }
        Err(error) => {
            exit::print_error_to(&error, err);
            exit::exit_code(error.code)
        }
    }
}

/// The machine-parseable result line (exact contract — see the
/// module docs; stdout's only content on success).
fn bench_line(report: &BenchReport, source: &str) -> String {
    format!(
        "BENCH file={source} bytes={} v_frames={} v_decoded={} a_frames={} a_decoded={} \
         wall_s={:.3} decode_s={:.3}",
        report.bytes,
        report.v_frames,
        report.v_decoded,
        report.a_frames,
        report.a_decoded,
        report.wall_s,
        report.decode_s
    )
}

/// The bench itself (FX2-A02: the real measurement): open (the
/// player's own path), offer the first video + first audio stream to
/// THE decoder dispatch, decode every packet to null.
///
/// A pure measurement seam — the only I/O is the input path itself
/// (access, probe, demux, stat); stdout/stderr lines are
/// [`run_bench`]'s to print, with the coverage notes riding back in
/// the report as data.
///
/// # Errors
/// Typed [`CommandError`]s at the honest stages: the open path's
/// (probe refusal, demux parse, mid-walk demux fatals — the loop
/// carries the partial counts as details), and the dispatch's own
/// typed refusal when the benched kinds' codecs have no decoder arm
/// — which, when NOTHING was benched, is the command's answer (no
/// BENCH line, nonzero exit) instead of a successful zero.
fn measure(source: &str) -> Result<BenchReport, CommandError> {
    // The wall origin: `main`'s seeded process start in the binary
    // (true process wall), the bench's own start in the library path
    // (no `main` — the in-memory tests). Read once, up front, so the
    // wall always covers the open + the loop.
    let wall_start = process_start();
    // 1. Open — the exact path `lumen open` takes (canonicalize →
    //    access → probe → the container-host registry → the per-
    //    container facts pre-pass). Garbage files refuse here
    //    (probe), unwired containers refuse here (typed, named).
    let spec = SourceSpec::File {
        path: Path::new(source).to_path_buf(),
    };
    let mut opened = demux_host::open_source(&spec, None, false, false)?;
    // The ONE registry: the provider over the open path's harvested
    // facts — the same construction the `open` path hands the
    // engine, so what the bench decodes is exactly what the player
    // decodes (the bench keeps no codec list of its own; the
    // extra_data seeding, the PCM facts seam and the honest
    // per-codec configure gates all live in the dispatch).
    let facts = Arc::new(opened.facts);
    let provider = HostDecoderProvider::new(Arc::clone(&facts));

    // 2. Source size (the file resolved and opened; metadata is the
    //    same truth the access layer read).
    let bytes = std::fs::metadata(source)
        .map_err(|error| {
            CommandError::new(
                Stage::Input,
                ErrorCode::Io,
                format!("cannot stat {}: {error}", Path::new(source).display()),
            )
        })?
        .len();

    // 3. Limb selection: the FIRST video stream and the FIRST audio
    //    stream are offered to the dispatch — one offer per kind,
    //    the engine's `wire`/`obtain_decoder` shape (the container
    //    default's first-of-kind in stream order here). The demuxers
    //    stamp every packet `GenerationId::FIRST` (each container
    //    host's pre-pass), so the config carries FIRST — anything
    //    else would drop every packet as stale.
    let generation = GenerationId::FIRST;
    let mut video: Option<Limb> = None;
    let mut audio: Option<Limb> = None;
    let mut notes: Vec<String> = Vec::new();
    let mut refusal: Option<CommandError> = None;
    let mut first_video_offered = false;
    let mut first_audio_offered = false;
    for track in &facts.tracks {
        let Some(fact) = facts.fact_for(track.id) else {
            continue;
        };
        let handle = track.id.handle();
        let offered = match track.kind {
            MediaKind::Video if !first_video_offered => {
                first_video_offered = true;
                true
            }
            MediaKind::Audio if !first_audio_offered => {
                first_audio_offered = true;
                true
            }
            _ => false,
        };
        let reason: String = if offered {
            // The engine's own `obtain_decoder` config shape, fed to
            // the dispatch: descriptor + generation + one thread, no
            // accelerator, no extradata (the dispatch seeds the
            // container's blobs from the facts itself).
            let config = DecoderConfig::new(
                track.codec.clone(),
                generation,
                ThreadBudget::default(),
                None,
                None,
            );
            match provider.decoder_for(track, &config) {
                Ok(decoder) => {
                    let limb = Limb {
                        decoder,
                        track: track.id,
                        frames: 0,
                        decoded: 0,
                        concealed: 0,
                    };
                    match track.kind {
                        MediaKind::Video => video = Some(limb),
                        MediaKind::Audio => audio = Some(limb),
                        _ => {}
                    }
                    notes.push(format!(
                        "{} track#{handle} {} — decoding",
                        kind_name(track.kind),
                        fact.codec
                    ));
                    continue;
                }
                // The dispatch's own typed refusal ("no decoder for
                // `<codec>` in this build", the landed inventory
                // named) — the note's reason here, and the command's
                // answer below if nothing else benched.
                Err(error) => {
                    let wired = demux_host::wire_error(&error);
                    let reason = wired.message.clone();
                    refusal.get_or_insert(wired);
                    reason
                }
            }
        } else {
            let reason = match track.kind {
                MediaKind::Video | MediaKind::Audio => {
                    if (matches!(track.kind, MediaKind::Video) && video.is_some())
                        || (matches!(track.kind, MediaKind::Audio) && audio.is_some())
                    {
                        "first-of-kind already benched"
                    } else {
                        "first-of-kind not benched"
                    }
                }
                _ => "not an audio/video stream",
            };
            reason.to_string()
        };
        notes.push(format!(
            "{} track#{handle} {} — not benched ({reason})",
            kind_name(track.kind),
            fact.codec
        ));
    }
    // Nothing benched is a typed refusal, not a successful zero
    // line (the audit's executed F-003 hole: an h264-only file
    // answered all-zeros + exit 0 while the same binary played it).
    // The dispatch's own refusal is the honest sentence it prints.
    if video.is_none() && audio.is_none() {
        return Err(refusal.unwrap_or_else(|| {
            CommandError::new(
                Stage::Decode,
                ErrorCode::Unsupported,
                "no audio or video track in this source — nothing to bench",
            )
            .with_detail("container", facts.container.clone())
        }));
    }

    // 4. The decode loop — decode_s starts here (after open).
    let decode_start = Instant::now();
    let mut sink = NullSink { frames: 0 };
    let mut consecutive: u32 = 0;
    let mut demux_recovered: u64 = 0;
    loop {
        match opened.demuxer.next_event() {
            Ok(DemuxEvent::Packet(packet)) => {
                // Route by track (no closure captures the packet — it
                // moves into `feed` whole; D-1: nothing else happens
                // per packet).
                let track = packet.track;
                let routed = match (video.as_mut(), audio.as_mut()) {
                    (Some(limb), _) if limb.track == track => Some(limb),
                    (_, Some(limb)) if limb.track == track => Some(limb),
                    _ => None,
                };
                if let Some(limb) = routed {
                    limb.feed(packet, &mut sink, &mut consecutive)?;
                }
                // Packets of unbenched tracks are dropped here (the
                // coverage note above said which and why).
            }
            Ok(DemuxEvent::Recovered { .. }) => {
                demux_recovered += 1;
            }
            Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break,
            Err(error) => {
                // A demux fatal mid-walk: the typed error is the
                // answer, with the partial coverage as details.
                let mut wired = demux_host::wire_error(&error);
                if let Some(limb) = video.as_ref() {
                    wired = wired
                        .with_detail("v_frames", limb.frames.to_string())
                        .with_detail("v_decoded", limb.decoded.to_string());
                }
                if let Some(limb) = audio.as_ref() {
                    wired = wired
                        .with_detail("a_frames", limb.frames.to_string())
                        .with_detail("a_decoded", limb.decoded.to_string());
                }
                return Err(wired);
            }
        }
    }
    if let Some(limb) = video.as_mut() {
        limb.drain(&mut sink)?;
    }
    if let Some(limb) = audio.as_mut() {
        limb.drain(&mut sink)?;
    }
    let decode_s = decode_start.elapsed().as_secs_f64();

    let (v_frames, v_decoded, v_concealed) = video.as_ref().map_or((0, 0, 0), |limb| {
        (limb.frames, limb.decoded, limb.concealed)
    });
    let (a_frames, a_decoded, a_concealed) = audio.as_ref().map_or((0, 0, 0), |limb| {
        (limb.frames, limb.decoded, limb.concealed)
    });
    Ok(BenchReport {
        bytes,
        v_frames,
        v_decoded,
        a_frames,
        a_decoded,
        wall_s: wall_start.elapsed().as_secs_f64(),
        decode_s,
        v_concealed,
        a_concealed,
        demux_recovered,
        notes,
        container: facts.container.clone(),
    })
}

/// A track kind's note name (MediaKind is Display-less by design in
/// L0; the bench note is the one place that wants a word).
fn kind_name(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Audio => "audio",
        MediaKind::Video => "video",
        MediaKind::Subtitle => "subtitle",
        MediaKind::Data => "data",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(relative: &str) -> String {
        format!(
            "{}/../../tests/fixtures/{relative}",
            env!("CARGO_MANIFEST_DIR")
        )
    }

    /// The line is the exact contract (field order and shapes pinned;
    /// speed_bench.py and the report parser depend on it).
    #[test]
    fn bench_line_is_the_exact_contract() {
        let report = BenchReport {
            bytes: 425_516,
            v_frames: 60,
            v_decoded: 60,
            a_frames: 60,
            a_decoded: 60,
            wall_s: 0.1234,
            decode_s: 0.1,
            v_concealed: 0,
            a_concealed: 0,
            demux_recovered: 0,
            notes: Vec::new(),
            container: "avi".to_string(),
        };
        assert_eq!(
            bench_line(&report, "/corpus/media/0001-avi-video.avi"),
            "BENCH file=/corpus/media/0001-avi-video.avi bytes=425516 v_frames=60 \
             v_decoded=60 a_frames=60 a_decoded=60 wall_s=0.123 decode_s=0.100"
        );
    }

    /// The in-memory happy path (the real open path + real decoders,
    /// no process): the class-A AVI fixture decodes every packet of
    /// both streams. Never panics — a panic here fails the suite.
    #[test]
    fn happy_path_fixture_reports_bench_line() {
        let path = fixture("generated/valid/avi_mjpeg_pcm.avi");
        let mut input = std::io::empty();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = crate::run::run(
            &[String::from("decode-bench"), path.clone()],
            &mut input,
            &mut out,
            &mut err,
            false,
        );
        assert_eq!(code, 0, "stderr:\n{}", String::from_utf8_lossy(&err));
        let line = String::from_utf8_lossy(&out)
            .lines()
            .find(|line| line.starts_with("BENCH "))
            .unwrap_or_default()
            .to_string();
        assert!(line.starts_with("BENCH file="), "stdout:\n{line}");
        let fields: std::collections::HashMap<&str, &str> = line
            .split_whitespace()
            .skip(1)
            .filter_map(|token| token.split_once('='))
            .collect();
        // The pinned fixture truth (MANIFEST.toml: 425_516 bytes).
        assert_eq!(fields.get("bytes").copied(), Some("425516"), "{line}");
        let number = |key: &str| -> f64 {
            fields
                .get(key)
                .and_then(|value| value.parse::<f64>().ok())
                .unwrap_or(-1.0)
        };
        let v_frames = number("v_frames");
        let v_decoded = number("v_decoded");
        let a_frames = number("a_frames");
        let a_decoded = number("a_decoded");
        assert!(v_frames > 0.0, "video packets demuxed: {line}");
        assert_eq!(v_decoded, v_frames, "every video frame decoded: {line}");
        assert!(a_frames > 0.0, "audio packets demuxed: {line}");
        assert_eq!(a_decoded, a_frames, "every audio frame decoded: {line}");
        assert!(
            number("decode_s") <= number("wall_s") + 0.002,
            "decode loop inside the wall: {line}"
        );
    }

    /// Garbage bytes: the probe's typed refusal, nonzero exit, no
    /// BENCH line, no panic (the adversarial-corpus contract).
    #[test]
    fn garbage_file_is_a_typed_refusal() {
        let garbage = std::env::temp_dir().join("lumen-cli-bench-garbage.bin"); // lumen-allow-literal: cfg(test) scratch filename — test-only, never a shipped surface
        std::fs::write(&garbage, [0x37u8, 0x13, 0x00, 0x42, 0xff, 0xee].repeat(64))
            .unwrap_or_default();
        let mut input = std::io::empty();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = crate::run::run(
            &[
                String::from("decode-bench"),
                garbage.to_string_lossy().into_owned(),
            ],
            &mut input,
            &mut out,
            &mut err,
            false,
        );
        std::fs::remove_file(&garbage).ok();
        assert_eq!(code, 65, "stderr:\n{}", String::from_utf8_lossy(&err));
        assert!(
            String::from_utf8_lossy(&err).contains("error: [probe.unsupported]"),
            "stderr:\n{}",
            String::from_utf8_lossy(&err)
        );
        assert!(
            !String::from_utf8_lossy(&out).contains("BENCH"),
            "no result line on failure:\n{}",
            String::from_utf8_lossy(&out)
        );
    }

    /// FX2-A02, the seam: the measurement itself (no output I/O)
    /// counts the h264+aac MP4 the same binary plays — the audit's
    /// F-001 reproduction inverted (the old hand-rolled registry
    /// answered v_frames=0 + "no decoder in this build" for exactly
    /// this class of file).
    #[test]
    fn fx2_a02_measure_counts_h264_aac_the_open_path_decodes() {
        let report = measure(&fixture("generated/valid/mp4_h264_aac.mp4"))
            .unwrap_or_else(|error| panic!("the dispatch decodes this fixture: {error}"));
        assert!(
            report.v_frames > 0,
            "video packets fed to the h264 limb (old bench: 0)"
        );
        assert!(
            report.v_decoded > 0,
            "h264 frames decoded (old bench: 0 — the lie)"
        );
        assert!(
            report.a_frames > 0,
            "audio packets fed to the aac limb (old bench: 0)"
        );
        assert!(
            report.a_decoded > 0,
            "aac frames decoded (old bench: 0 — the lie)"
        );
        // The coverage notes name both limbs as decoding.
        let notes = report.notes.join("; ");
        assert!(notes.contains("h264 — decoding"), "notes: {notes}");
        assert!(notes.contains("aac — decoding"), "notes: {notes}");
    }
}
