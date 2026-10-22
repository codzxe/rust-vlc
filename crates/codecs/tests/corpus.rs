//! Randomized real-media corpus suite — the decode surface (ED-FIX-5; the
//! user's randomized-corpus directive: real ffmpeg-authored media, decoded
//! and measured, with ffmpeg's own decode as the external oracle).
//!
//! This is the vertical composition the echo-chamber audit demanded: real
//! AVIs (randomized codec/resolution/fps/quality, ffmpeg-authored) opened
//! through `input`, demuxed by the real `AviDemuxer`, decoded by the real
//! MJPEG/PCM decoders — end to end, no synthetic fixture anywhere in the
//! path. Two measured comparisons per case:
//!
//! * **MJPEG**: every video packet is decoded; the ffmpeg reference decode
//!   (rawvideo yuv420p, extracted by the harness driver) is compared plane
//!   by plane. Byte-exactness across independent JPEG IDCT implementations
//!   is not expected — the recorded metrics are PSNR (min/mean, millidB),
//!   max absolute sample difference, and the exactly-equal byte fraction;
//!   plus decode throughput (payload MB/s, frames/s) per file.
//! * **PCM**: the concatenated audio payloads are decoded and compared
//!   byte-exact against ffmpeg's raw decode — PCM has no implementation
//!   freedom, so any mismatch is a real defect signal, not rounding.
//!
//! Both suites also emit the DECODED OUTPUT shape each file actually
//! produced (W2-09, closing W1-04's registered suite gap — the harness's
//! decoded-fidelity ledger consumed "not-emitted (suite gap)" cells until
//! these columns landed): mjpeg rows carry `out_format`/`out_w`/`out_h` —
//! the first successfully decoded frame's `DecodedFormat` (snake_case)
//! and dimensions — and pcm rows carry `out_desc`/`out_rate`/`out_ch` —
//! the descriptor and the rate/channels the decode actually ran with. A
//! file that decodes nothing records the explicit `-` marker in those
//! cells: a measured gap, never a fabricated guess (D-5/D-11).
//!
//! The suite fails only on contract violations (panic, missing files);
//! verdict quality is the aggregator's judgement, reported as numbers.
//!
//! Activation (the harness driver runs exactly this):
//!
//! ```text
//! LUMEN_CORPUS=<dir> LUMEN_CORPUS_CASES=<tsv> LUMEN_CORPUS_RESULTS=<tsv> \
//!   cargo test --release -p codecs --test corpus -- --ignored --nocapture
//! ```

use std::fs::{File, OpenOptions};
use std::io::{Read, Write as _};
use std::path::{Path, PathBuf};
use std::time::Instant;

use codecs::audio::pcm::{PcmFormat, PcmSource, decode_pcm};
use codecs::video::mjpeg::{DecodedFormat, decode_image};
use demux::avi::AviDemuxer;
use media_types::EngineError;
use media_types::ids::TrackId;
use plugin_api::{Access, DemuxEvent, Demuxer, SourceStream};
use testkit::Must;

/// Open a corpus file through the real access stack (card 2.A1's seam).
fn open_via_input(path: &Path) -> Result<Box<dyn SourceStream>, EngineError> {
    let text = match path.to_str() {
        Some(text) => format!("file://{text}"),
        None => unreachable!("corpus paths are UTF-8 ({path:?})"),
    };
    let uri = match media_types::ids::SourceUri::parse(&text) {
        Ok(uri) => uri,
        Err(err) => unreachable!("corpus path is a valid file URI ({path:?}): {err}"),
    };
    let access = input::FileAccess::new();
    access.open(&uri)
}

/// A ref-streaming reader over the ffmpeg rawvideo reference (frames are
/// read one at a time into a reusable buffer — a 4K corpus file's full
/// reference never needs to be resident).
struct RefReader {
    file: File,
    buf: Vec<u8>,
}

impl RefReader {
    fn new(path: &Path, frame_bytes: usize) -> RefReader {
        RefReader {
            file: File::open(path).must(),
            buf: vec![0u8; frame_bytes],
        }
    }

    /// Read the next reference frame, or `None` at end of file.
    fn next_frame(&mut self) -> Option<&[u8]> {
        let mut filled = 0usize;
        while filled < self.buf.len() {
            match self.file.read(&mut self.buf[filled..]) {
                Ok(0) => {
                    return if filled == 0 {
                        None
                    } else {
                        Some(&self.buf[..filled]) // short tail frame
                    };
                }
                Ok(n) => filled += n,
                Err(err) => unreachable!("ref stream is a plain file: {err}"),
            }
        }
        Some(&self.buf[..filled])
    }
}

/// Frame-difference metrics over the I420 planes (Y, Cb, Cr) — MSE, max
/// absolute difference, exactly-equal bytes, bytes compared. Zip stops at
/// the shorter plane (a length mismatch is a recorded note, not a panic).
fn diff_frame(got: &[u8], want: &[u8]) -> (f64, u32, u64, u64) {
    let mut sq_sum: f64 = 0.0;
    let mut max_abs: u32 = 0;
    let mut exact: u64 = 0;
    let mut n: u64 = 0;
    for (a, b) in got.iter().zip(want.iter()) {
        let a = i32::from(*a);
        let b = i32::from(*b);
        let d = (a - b).abs();
        if d == 0 {
            exact += 1;
        }
        if d as u32 > max_abs {
            max_abs = d as u32;
        }
        sq_sum += f64::from(d * d);
        n += 1;
    }
    let mse = if n > 0 {
        sq_sum / n as f64
    } else {
        f64::INFINITY
    };
    (mse, max_abs, exact, n)
}

fn psnr_millidb(mse: f64) -> i64 {
    if mse <= 0.0 {
        return 999_999; // bit-exact frame
    }
    let psnr = 10.0 * (255.0 * 255.0 / mse).log10();
    (psnr * 1000.0) as i64
}

// -------------------------------------------------------------------------
// MJPEG case: demux + decode every video packet, compare vs raw reference.
// -------------------------------------------------------------------------

struct MjpegCase {
    rel: String,
    ref_path: String,
    width: u32,
    height: u32,
    /// The harness's frame count (ref bytes / frame size) — the Rust
    /// side streams the ref to EOF instead, so the field is carried for
    /// the row schema only.
    #[allow(dead_code)]
    frames: u64,
}

/// One mjpeg result row. Named fields, not a positional tuple: the row
/// must stay column-honest by construction as columns land (W2-09's
/// `out_*` triple is the second extension — a 14-slot tuple would be a
/// swap-defect waiting to happen).
struct MjpegOutcome {
    decoded: u64,
    errors: u64,
    concealed: u64,
    psnr_min_mdb: i64,
    psnr_mean_mdb: i64,
    max_abs: u32,
    exact_bytes: u64,
    compared_bytes: u64,
    payload_bytes: u64,
    decode_us: u128,
    note: String,
    /// First successfully decoded frame's output shape: `DecodedFormat`
    /// snake_case + dimensions. `None` when nothing decoded — the row
    /// then carries the explicit `-` marker, never a guess. Concealed
    /// frames count as decoded (they are real decoder output; the
    /// `concealed` column already distinguishes them).
    out: Option<(&'static str, u32, u32)>,
}

impl MjpegOutcome {
    /// The all-zero row for a pipeline that never reached decoding (the
    /// typed error code rides in `note`; the `out_*` cells are `-`).
    fn failed(note: String) -> MjpegOutcome {
        MjpegOutcome {
            decoded: 0,
            errors: 0,
            concealed: 0,
            psnr_min_mdb: 0,
            psnr_mean_mdb: 0,
            max_abs: 0,
            exact_bytes: 0,
            compared_bytes: 0,
            payload_bytes: 0,
            decode_us: 0,
            note,
            out: None,
        }
    }
}

/// The result-TSV spelling of `DecodedFormat` — W1-04's registered
/// snake_case vocabulary (`i420`/`rgba8`); the harness maps it to
/// sampling families for the oracle comparison.
fn format_snake(format: DecodedFormat) -> &'static str {
    match format {
        DecodedFormat::I420 => "i420",
        DecodedFormat::Rgba8 => "rgba8",
    }
}

fn run_mjpeg_case(corpus: &Path, case: &MjpegCase) -> MjpegOutcome {
    let path = corpus.join(&case.rel);
    let ref_path = corpus.join(&case.ref_path);
    let stream = match open_via_input(&path) {
        Ok(s) => s,
        Err(err) => return MjpegOutcome::failed(err.context.code.to_owned()),
    };
    let mut dem = match AviDemuxer::new(stream) {
        Ok(d) => d,
        Err(err) => return MjpegOutcome::failed(err.context.code.to_owned()),
    };
    let tracks = match dem.start() {
        Ok(t) => t,
        Err(err) => return MjpegOutcome::failed(err.context.code.to_owned()),
    };
    let video_ids: Vec<TrackId> = tracks
        .iter()
        .filter(|t| matches!(t.kind, media_types::MediaKind::Video))
        .map(|t| t.id)
        .collect();
    let mut refs = RefReader::new(&ref_path, frame_bytes(case.width, case.height));
    let (mut decoded, mut errors, mut concealed) = (0u64, 0u64, 0u64);
    let (mut psnr_min, mut psnr_sum, mut psnr_n) = (i64::MAX, 0i64, 0u64);
    let mut max_abs: u32 = 0;
    let (mut exact_bytes, mut compared_bytes) = (0u64, 0u64);
    let mut payload_bytes = 0u64;
    let mut decode_elapsed = 0u128;
    let mut first_out: Option<(&'static str, u32, u32)> = None;
    loop {
        let event = match dem.next_event() {
            Ok(e) => e,
            Err(_) => break, // terminal damage — counts are what they are
        };
        let DemuxEvent::Packet(packet) = event else {
            if matches!(event, DemuxEvent::EndOfStream) {
                break;
            }
            continue;
        };
        if !video_ids.contains(&packet.track) {
            continue;
        }
        payload_bytes += packet.data.len() as u64;
        let t0 = Instant::now();
        let frame = decode_image(packet.data.as_slice());
        decode_elapsed += t0.elapsed().as_nanos();
        match frame {
            Ok(frame) => {
                decoded += 1;
                // The decoded-output row (W2-09): the FIRST frame the
                // decoder returned fixes the emitted format/dims; later
                // frames are cross-checkable against the existing
                // shape gate and PSNR columns.
                if first_out.is_none() {
                    first_out = Some((format_snake(frame.format), frame.width, frame.height));
                }
                if frame.concealed {
                    concealed += 1;
                }
                // Compare only when the decoded shape matches the case's
                // expected geometry — a dims disagreement is recorded by
                // the missing comparison, not by garbage metrics.
                let shape_ok = frame.format == DecodedFormat::I420
                    && frame.width == case.width
                    && frame.height == case.height;
                if shape_ok && let Some(want) = refs.next_frame() {
                    let (mse, m_abs, exact, n) = diff_frame(frame.data.as_slice(), want);
                    let psnr = psnr_millidb(mse);
                    if psnr < psnr_min {
                        psnr_min = psnr;
                    }
                    psnr_sum += psnr;
                    psnr_n += 1;
                    if m_abs > max_abs {
                        max_abs = m_abs;
                    }
                    exact_bytes += exact;
                    compared_bytes += n;
                }
            }
            Err(_) => errors += 1,
        }
    }
    if psnr_min == i64::MAX {
        psnr_min = 0;
    }
    let psnr_mean = if psnr_n > 0 {
        psnr_sum / psnr_n as i64
    } else {
        0
    };
    MjpegOutcome {
        decoded,
        errors,
        concealed,
        psnr_min_mdb: psnr_min,
        psnr_mean_mdb: psnr_mean,
        max_abs,
        exact_bytes,
        compared_bytes,
        payload_bytes,
        decode_us: decode_elapsed / 1_000, // µs
        note: String::new(),
        out: first_out,
    }
}

/// I420 planar size with ceil-half chroma (odd dims round up).
fn frame_bytes(w: u32, h: u32) -> usize {
    let y = w as usize * h as usize;
    let c = (w as usize).div_ceil(2) * (h as usize).div_ceil(2);
    y + 2 * c
}

// -------------------------------------------------------------------------
// PCM case: demux, concatenate audio payloads, decode, compare byte-exact.
// -------------------------------------------------------------------------

struct PcmCase {
    rel: String,
    ref_path: String,
    descriptor: String,
    rate: u32,
    channels: u32,
}

/// One pcm result row (named fields for the same column-honesty reason
/// as [`MjpegOutcome`]).
struct PcmOutcome {
    ref_bytes: u64,
    compared: u64,
    mismatch: u64,
    max_abs: u32,
    dropped_tail: u64,
    decode_us: u128,
    note: String,
    /// The decode's actual output configuration: the `PcmFormat`
    /// descriptor that was constructed and the rate/channels the
    /// decode ran with (from the decoded `AudioFormat`, which the
    /// byte-exact comparison below was computed against). `None` when
    /// no samples were decoded — the row then carries the explicit `-`
    /// marker, never a guess.
    ///
    /// Provenance, stated honestly (D-5): the decode configuration is
    /// harness-supplied via the case TSV (the corpus manifest's
    /// generator intent), NOT re-derived from the file's strf header —
    /// the emitted cells record what this decode actually ran with, and
    /// the harness compares them against ffprobe's independently
    /// measured stream facts.
    out: Option<(&'static str, u32, u32)>,
}

impl PcmOutcome {
    /// The all-zero row for a pipeline that never decoded anything
    /// (typed error code / unknown-descriptor note; `out_*` cells `-`).
    fn failed(note: String) -> PcmOutcome {
        PcmOutcome {
            ref_bytes: 0,
            compared: 0,
            mismatch: 0,
            max_abs: 0,
            dropped_tail: 0,
            decode_us: 0,
            note,
            out: None,
        }
    }
}

fn run_pcm_case(corpus: &Path, case: &PcmCase) -> PcmOutcome {
    let path = corpus.join(&case.rel);
    let ref_path = corpus.join(&case.ref_path);
    let stream = match open_via_input(&path) {
        Ok(s) => s,
        Err(err) => return PcmOutcome::failed(err.context.code.to_owned()),
    };
    let mut dem = match AviDemuxer::new(stream) {
        Ok(d) => d,
        Err(err) => return PcmOutcome::failed(err.context.code.to_owned()),
    };
    let tracks = match dem.start() {
        Ok(t) => t,
        Err(err) => return PcmOutcome::failed(err.context.code.to_owned()),
    };
    let audio_ids: Vec<TrackId> = tracks
        .iter()
        .filter(|t| matches!(t.kind, media_types::MediaKind::Audio))
        .map(|t| t.id)
        .collect();
    let mut payload: Vec<u8> = Vec::new();
    loop {
        let event = match dem.next_event() {
            Ok(e) => e,
            Err(_) => break,
        };
        let DemuxEvent::Packet(packet) = event else {
            if matches!(event, DemuxEvent::EndOfStream) {
                break;
            }
            continue;
        };
        if audio_ids.contains(&packet.track) {
            payload.extend_from_slice(packet.data.as_slice());
        }
    }
    let sample = match PcmFormat::from_descriptor(&case.descriptor) {
        Some(f) => f,
        None => return PcmOutcome::failed("pcm.unknown-descriptor".to_owned()),
    };
    let source = PcmSource {
        sample,
        planar: false,
        channels: case.channels,
    };
    let t0 = Instant::now();
    let decoded = match decode_pcm(payload.as_slice(), source, case.rate) {
        Ok(d) => d,
        Err(err) => return PcmOutcome::failed(err.context.code.to_owned()),
    };
    let decode_us = t0.elapsed().as_micros();
    let mut reference = Vec::new();
    File::open(&ref_path)
        .must()
        .read_to_end(&mut reference)
        .must();
    let (mut mismatch, mut max_abs) = (0u64, 0u32);
    let n = decoded.samples.len().min(reference.len());
    for (a, b) in decoded.samples.iter().zip(reference.iter()) {
        let a = i32::from(*a);
        let b = i32::from(*b);
        let d = (a - b).abs();
        if d != 0 {
            mismatch += 1;
        }
        if d as u32 > max_abs {
            max_abs = d as u32;
        }
    }
    // The decoded-output cells (W2-09): emitted only when the decode
    // actually produced samples. Zero decoded samples (no audio packets
    // demuxed, or a tail too short to form one sample-frame) is the
    // pcm-side analog of "zero frames decoded" — recorded as `-`.
    let out = if decoded.samples.is_empty() {
        None
    } else {
        Some((
            sample.descriptor(),
            decoded.format.rate,
            decoded.format.channels,
        ))
    };
    PcmOutcome {
        ref_bytes: reference.len() as u64,
        compared: n as u64,
        mismatch,
        max_abs,
        dropped_tail: decoded.dropped_tail_bytes as u64,
        decode_us,
        note: if decoded.samples.len() != reference.len() {
            format!("len:{}vs{}", decoded.samples.len(), reference.len())
        } else {
            String::new()
        },
        out,
    }
}

// -------------------------------------------------------------------------
// The env-gated corpus runs.
// -------------------------------------------------------------------------

fn parse_mjpeg_cases(text: &str) -> Vec<MjpegCase> {
    let mut out = Vec::new();
    for line in text.lines().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 5 {
            continue;
        }
        out.push(MjpegCase {
            rel: f[0].to_owned(),
            ref_path: f[1].to_owned(),
            width: f[2].parse().must(),
            height: f[3].parse().must(),
            frames: f[4].parse().must(),
        });
    }
    out
}

fn parse_pcm_cases(text: &str) -> Vec<PcmCase> {
    let mut out = Vec::new();
    for line in text.lines().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 5 {
            continue;
        }
        out.push(PcmCase {
            rel: f[0].to_owned(),
            ref_path: f[1].to_owned(),
            descriptor: f[2].to_owned(),
            rate: f[3].parse().must(),
            channels: f[4].parse().must(),
        });
    }
    out
}

#[test]
#[ignore = "randomized-corpus harness: run via tools/corpus/run_harness.py"]
fn corpus_mjpeg_decode_measures_against_oracle() {
    let (Some(corpus), Some(cases_path), Some(results_path)) = (
        std::env::var("LUMEN_CORPUS").ok(),
        std::env::var("LUMEN_CORPUS_CASES").ok(),
        std::env::var("LUMEN_CORPUS_RESULTS").ok(),
    ) else {
        eprintln!("corpus: LUMEN_CORPUS env trio not set — no-op");
        return;
    };
    let corpus = PathBuf::from(corpus);
    let cases = parse_mjpeg_cases(&std::fs::read_to_string(&cases_path).must());
    assert!(
        !cases.is_empty(),
        "harness handed an empty case list ({cases_path:?})"
    );
    let mut out = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&results_path)
        .must();
    writeln!(
        out,
        "rel\tdecoded\terrors\tconcealed\tpsnr_min_mdb\tpsnr_mean_mdb\tmax_abs\
         \texact_bytes\tcompared_bytes\tpayload_bytes\tdecode_us\tnote\
         \tout_format\tout_w\tout_h"
    )
    .must();
    for case in &cases {
        assert!(corpus.join(&case.rel).is_file(), "missing {}", case.rel);
        assert!(
            corpus.join(&case.ref_path).is_file(),
            "missing ref {}",
            case.ref_path
        );
        let r = run_mjpeg_case(&corpus, case);
        // Decoded-output cells: the real first-frame shape, or the
        // explicit `-` marker when nothing decoded — never a guess.
        let (out_format, out_w, out_h) = match r.out {
            Some((format, w, h)) => (format.to_owned(), w.to_string(), h.to_string()),
            None => ("-".to_owned(), "-".to_owned(), "-".to_owned()),
        };
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            case.rel,
            r.decoded,
            r.errors,
            r.concealed,
            r.psnr_min_mdb,
            r.psnr_mean_mdb,
            r.max_abs,
            r.exact_bytes,
            r.compared_bytes,
            r.payload_bytes,
            r.decode_us,
            r.note,
            out_format,
            out_w,
            out_h
        )
        .must();
    }
    eprintln!(
        "corpus(mjpeg): {} cases measured -> {results_path:?}",
        cases.len()
    );
}

#[test]
#[ignore = "randomized-corpus harness: run via tools/corpus/run_harness.py"]
fn corpus_pcm_decode_is_byte_exact_or_recorded() {
    let (Some(corpus), Some(cases_path), Some(results_path)) = (
        std::env::var("LUMEN_CORPUS").ok(),
        std::env::var("LUMEN_CORPUS_CASES").ok(),
        std::env::var("LUMEN_CORPUS_RESULTS").ok(),
    ) else {
        eprintln!("corpus: LUMEN_CORPUS env trio not set — no-op");
        return;
    };
    let corpus = PathBuf::from(corpus);
    let cases = parse_pcm_cases(&std::fs::read_to_string(&cases_path).must());
    assert!(
        !cases.is_empty(),
        "harness handed an empty case list ({cases_path:?})"
    );
    let mut out = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&results_path)
        .must();
    writeln!(
        out,
        "rel\tref_bytes\tcompared\tmismatch\tmax_abs\tdropped_tail\tdecode_us\tnote\
         \tout_desc\tout_rate\tout_ch"
    )
    .must();
    for case in &cases {
        assert!(corpus.join(&case.rel).is_file(), "missing {}", case.rel);
        assert!(
            corpus.join(&case.ref_path).is_file(),
            "missing ref {}",
            case.ref_path
        );
        let r = run_pcm_case(&corpus, case);
        // Decoded-output cells: the configuration the decode actually
        // ran with, or the explicit `-` marker when no samples were
        // decoded — never a guess.
        let (out_desc, out_rate, out_ch) = match r.out {
            Some((desc, rate, ch)) => (desc.to_owned(), rate.to_string(), ch.to_string()),
            None => ("-".to_owned(), "-".to_owned(), "-".to_owned()),
        };
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            case.rel,
            r.ref_bytes,
            r.compared,
            r.mismatch,
            r.max_abs,
            r.dropped_tail,
            r.decode_us,
            r.note,
            out_desc,
            out_rate,
            out_ch
        )
        .must();
    }
    eprintln!(
        "corpus(pcm): {} cases measured -> {results_path:?}",
        cases.len()
    );
}
