//! The 6.OPW-ttf-seek measurement harness: time-to-first-output and
//! in-session seek, per container family, through the REAL binary
//! (the W3 row the ivf unit flagged — "in-session native seek
//! (reanchor) not exercised through a playing session; the seek
//! re-measure card owns it").
//!
//! # What is measured, and in which profile (the honesty contract)
//!
//! Every number this harness prints is a WALL-CLOCK measurement of
//! ONE `lumen open <lane>` process in the DEV (debug, unoptimized)
//! profile — the profile `cargo test` builds. The numbers are
//! COMPARATIVE across families in that one profile; they are not
//! absolute performance claims (a release build answers different
//! magnitudes; the comparisons are the deliverable).
//!
//! - **ttf (time-to-first-output)**: spawn → the first
//!   `state-changed Buffering → Playing` line on stdout. That
//!   transition is this host's first-output watermark (the settle
//!   pumps run until the engine's `Primed` signal — the first
//!   presentable frame). Also recorded: spawn → `ok: accepted`
//!   (the open receipt: probe + pre-pass), so the engine-only
//!   window (accepted → first output) stands next to the
//!   process-wide one.
//! - **in-session seek latency**: the first `Buffering → Playing`
//!   line (end of the open settle) → the seek's terminal line
//!   (`TimingSeekCompleted` / `TimingSeekFailed`; for the
//!   accepted-then-silent shape, process exit — the host's bounded
//!   wait expired without a terminal). The script's `seek` verb is
//!   buffered in the child's stdin, so the dispatch follows the
//!   open settle immediately; the interval is the seek's own
//!   synchronous work (reanchor + pre-roll + settle) plus
//!   line-printing overhead, comparable across families.
//!
//! # Structural truths only (the assertions)
//!
//! Timing is RECORDED (eprintln, the bench_decode convention), never
//! asserted: the pins are structural — first output arrives (or the
//! typed refusal names why it cannot), the seek lands in the
//! requested neighborhood (or refuses typed per the family's
//! seekability class), the seek never reopens the session, and the
//! `--start-time` pure-seek open path lands near its target (the
//! watermark that powers open-time seeking where in-session reanchor
//! is absent).
//!
//! # The honest landscape this pins (measured this session)
//!
//! Through the REAL binary the playable families are narrower than
//! the pipeline-layer evidence: the CLI host's present limb accepts
//! exactly BT.601 full-range i420 (the MJPEG case), so every
//! vp9/av1 lane decodes but refuses at the renderer; the audio limb
//! needs a geometry-bearing harvest (PCM pair, Vorbis
//! identification, or OpusHead), so ADTS/AAC and MKA/FLAC-ALAC
//! lanes open limb-less and refuse on the first audio frame; h264
//! is not dispatched by this host's decoder providers. The
//! in-session seek classes: AVI/AIFF land; WAV lands on some lanes
//! (the sr-chaos corpus lane and the 44.1 kHz fixture land exactly)
//! but the 11.025 kHz corpus lane 0026 answers every below-end
//! target with a landing deep past the request (the recorded
//! wrong-landing anomaly — the pure `--start-time` seek on the same
//! file lands exactly, so the defect is the in-session seam, not
//! the demuxer's byte math), and the s24le/u8 lanes accept the
//! seek, rebuffer in-pipeline, and never report a terminal (the
//! recorded accepted-then-silent gap); MP4/MOV/M4A refuse typed at
//! demux reanchor (the recorded 6.OPW-mp4 gap); OGG refuses typed
//! at decode (the reanchor does not replay the codec headers); raw
//! ES mp3/mp2 are unseekable.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use testkit::Must;

// ---------------------------------------------------------------------------
// The harness
// ---------------------------------------------------------------------------

/// One timed child transcript: every stdout line with its wall-clock
/// offset from just-before-spawn (milliseconds, dev profile), the
/// collected stderr, and the exit code.
struct Timed {
    lines: Vec<(String, f64)>,
    stderr: String,
    code: i32,
}

/// Run the real binary with `args` and a pre-buffered stdin script,
/// timestamping every stdout line as it arrives (Rust's stdout is
/// line-buffered even piped, so each receipt is readable at print
/// time). The script rides stdin from t=0; the REPL dispatches each
/// line only when its turn comes (the open settles before the first
/// script line is read), so line offsets are causal.
fn timed_session(args: &[&str], script: &str) -> Timed {
    let t0 = Instant::now();
    let mut child = Command::new(env!("CARGO_BIN_EXE_lumen"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .must();
    {
        let mut stdin = child.stdin.take().must();
        stdin.write_all(script.as_bytes()).must();
        // Dropped here: the script's lines stay buffered for the
        // child, EOF follows them, and the M17 teardown runs.
    }
    let stdout = child.stdout.take().must();
    let mut lines = Vec::new();
    for line in BufReader::new(stdout).lines() {
        let line = line.must();
        lines.push((line, t0.elapsed().as_secs_f64() * 1000.0));
    }
    let status = child.wait().must();
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
        use std::io::Read as _;
        pipe.read_to_string(&mut stderr).must();
    }
    Timed {
        lines,
        stderr,
        code: status.code().unwrap_or(-1),
    }
}

impl Timed {
    /// The transcript holds the needle.
    fn contains(&self, needle: &str) -> bool {
        self.lines.iter().any(|(line, _)| line.contains(needle))
    }

    /// The wall-clock offset (ms) of the FIRST line holding the
    /// needle.
    fn first_ms(&self, needle: &str) -> Option<f64> {
        self.lines
            .iter()
            .find(|(line, _)| line.contains(needle))
            .map(|(_, ms)| *ms)
    }

    /// The integer payload of the first `key(NNN)` token in the
    /// first line holding `needle` (e.g. the µs position of a
    /// `TimingSeekCompleted`, the µs duration of a
    /// `MediaInfoReady`).
    fn first_int_after(&self, needle: &str, key: &str) -> Option<i64> {
        self.lines.iter().find_map(|(line, _)| {
            let at = line.find(needle)? + needle.len();
            let tail = &line[at..];
            let open = tail.find(key)? + key.len();
            let digits: String = tail[open..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            digits.parse::<i64>().ok()
        })
    }
}

/// The corpus media dir (the house corpus convention: `LUMEN_CORPUS`
/// or the default checkout; absent → the measurement no-ops).
fn corpus_media() -> Option<PathBuf> {
    let root = std::env::var("LUMEN_CORPUS")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            let default = PathBuf::from("/tmp/my-project/corpus-v4");
            default.is_dir().then_some(default)
        })?;
    let media = root.join("media");
    media.is_dir().then_some(media)
}

/// A path under this crate's `tests/fixtures` tree.
fn fixture(relative: &str) -> String {
    format!(
        "{}/../../tests/fixtures/{relative}",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// A path under the codecs crate's fixture tree (the ivf lanes — no
/// corpus .ivf lanes exist, the recorded 6.OPW-ivf gap).
fn codec_fixture(relative: &str) -> String {
    format!(
        "{}/../../crates/codecs/tests/fixtures/{relative}",
        env!("CARGO_MANIFEST_DIR")
    )
}

// ---------------------------------------------------------------------------
// The lane table (measured classes, this session, dev profile)
// ---------------------------------------------------------------------------

/// The in-session seek class a lane answers through the binary.
enum SeekClass {
    /// The seek lands in-pipeline: one terminal at the keyframe-safe
    /// actual, no reopen.
    Lands,
    /// A terminal arrives, but NOT in the requested neighborhood —
    /// the recorded wav 0026 anomaly (every below-end target answers
    /// a landing deep past the request; the pure seek on the same
    /// file lands exactly).
    LandsWrong,
    /// Accepted, then the typed demux refusal (the mp4 family's
    /// trait-default reanchor — the recorded 6.OPW-mp4 gap).
    DemuxReanchor,
    /// Accepted, then the typed decode refusal (the ogg reanchor
    /// does not replay the codec headers).
    DecodeHeaders,
    /// The source is unseekable (raw ES mpeg audio) / the target
    /// unresolvable (no duration): the typed control refusal.
    NotSeekable,
    /// Accepted, the rebuffer episode runs, and NO terminal ever
    /// arrives (the host's bounded wait expires) — the recorded
    /// wav s24le/u8 gap.
    SilentWedge,
}

/// One measurement lane: a family key, its representative media, and
/// the honest through-the-binary truths.
struct Lane {
    family: &'static str,
    path: String,
    /// `true`: first output (`Buffering → Playing`) arrives.
    plays: bool,
    /// For non-playing lanes: the typed refusal substring the
    /// transcript must name.
    refusal: Option<&'static str>,
    seek: SeekClass,
}

/// The full lane table (corpus paths resolved against `media`).
fn lanes(media: &Path) -> Vec<Lane> {
    let corpus = |name: &str| media.join(name).to_string_lossy().into_owned();
    vec![
        // Playing families.
        Lane {
            family: "avi",
            path: corpus("0076-avi-av.avi"),
            plays: true,
            refusal: None,
            seek: SeekClass::Lands,
        },
        Lane {
            family: "wav/s16le",
            path: corpus("0013-sr-chaos.wav"),
            plays: true,
            refusal: None,
            seek: SeekClass::Lands,
        },
        Lane {
            family: "wav/11k",
            path: corpus("0026-audio-only.wav"),
            plays: true,
            refusal: None,
            seek: SeekClass::LandsWrong,
        },
        Lane {
            family: "wav/s24le",
            path: corpus("0004-audio-only.wav"),
            plays: true,
            refusal: None,
            seek: SeekClass::SilentWedge,
        },
        Lane {
            family: "aiff",
            path: fixture("generated/valid/aiff_pcm_s16be_44k1_st.aiff"),
            plays: true,
            refusal: None,
            seek: SeekClass::Lands,
        },
        Lane {
            family: "ogg/opus",
            path: corpus("0047-audio-only.ogg"),
            plays: true,
            refusal: None,
            seek: SeekClass::DecodeHeaders,
        },
        Lane {
            family: "mp4/mjpeg",
            path: corpus("0115-fps-extremes.mp4"),
            plays: true,
            refusal: None,
            seek: SeekClass::DemuxReanchor,
        },
        Lane {
            family: "mov/mjpeg",
            path: corpus("0079-mov-video.mov"),
            plays: true,
            refusal: None,
            seek: SeekClass::DemuxReanchor,
        },
        Lane {
            family: "m4a/aac",
            path: corpus("0029-cover-audio.m4a"),
            plays: true,
            refusal: None,
            seek: SeekClass::DemuxReanchor,
        },
        Lane {
            family: "es/mp3",
            path: corpus("0139-audio-only.mp3"),
            plays: true,
            refusal: None,
            seek: SeekClass::NotSeekable,
        },
        Lane {
            family: "es/mp2",
            path: corpus("0005-audio-only.mp2"),
            plays: true,
            refusal: None,
            seek: SeekClass::NotSeekable,
        },
        // Families that open but never produce first output through
        // this host's limbs (each refusal named, measured).
        Lane {
            family: "adts/aac",
            path: corpus("0059-audio-only.aac"),
            plays: false,
            refusal: Some("cli.audio_limb_absent"),
            seek: SeekClass::DemuxReanchor,
        },
        Lane {
            family: "mpegts/h264",
            path: corpus("0018-ts-video.ts"),
            plays: false,
            refusal: Some("cli.codec_unavailable"),
            seek: SeekClass::NotSeekable,
        },
        Lane {
            family: "ivf/vp9",
            path: codec_fixture("vp9/testsrc2_640x480_g30_90f.ivf"),
            plays: false,
            refusal: Some("cli.i420_color_unsupported"),
            seek: SeekClass::DemuxReanchor,
        },
        Lane {
            family: "matroska/vp9",
            path: corpus("0075-mkv-video.mkv"),
            plays: false,
            refusal: Some("cli.i420_color_unsupported"),
            seek: SeekClass::DemuxReanchor,
        },
    ]
}

// ---------------------------------------------------------------------------
// (a) time-to-first-output per family
// ---------------------------------------------------------------------------

/// THE ttf table: every lane opens through the real binary; the
/// playing families must reach first output (`Buffering → Playing`,
/// exit 0) with the open receipt ordered before it, the refusing
/// families must name their typed refusal and never claim first
/// output. The measured durations print on stderr (the evidence
/// table; `-- --nocapture` to read them).
#[test]
fn ttf_first_output_per_family_through_the_binary() {
    let Some(media) = corpus_media() else {
        eprintln!("ttf-seek: corpus absent — measurement no-op");
        return;
    };
    let mut rows = Vec::new();
    for lane in lanes(&media) {
        let timed = timed_session(&["open", &lane.path], "quit\n");
        if lane.plays {
            assert_eq!(
                timed.code,
                0,
                "{}: stdout:\n{}",
                lane.family,
                transcript(&timed)
            );
            let accepted = timed.first_ms("ok: accepted session=1");
            let first_output = timed.first_ms("state-changed Buffering → Playing");
            let (Some(accepted), Some(first_output)) = (accepted, first_output) else {
                unreachable!(
                    "{}: the open receipt and first output must both print:\n{}",
                    lane.family,
                    transcript(&timed)
                );
            };
            assert!(
                first_output > accepted,
                "{}: first output must follow the open receipt:\n{}",
                lane.family,
                transcript(&timed)
            );
            rows.push(format!(
                "{:<15} ttf spawn→accepted {:>8.1} ms | accepted→first-output {:>8.1} ms | \
                 spawn→first-output {:>8.1} ms",
                lane.family,
                accepted,
                first_output - accepted,
                first_output
            ));
        } else {
            let refusal = lane.refusal.must();
            assert!(
                timed.contains(refusal),
                "{}: the typed refusal `{refusal}` must be named:\n{}",
                lane.family,
                transcript(&timed)
            );
            assert!(
                !timed.contains("state-changed Buffering → Playing"),
                "{}: a refusing lane must never claim first output:\n{}",
                lane.family,
                transcript(&timed)
            );
            let accepted = timed.first_ms("ok: accepted session=1");
            let refusal_at = timed.first_ms("event: error ");
            rows.push(format!(
                "{:<15} ttf REFUSED after open ({refusal}); open→accepted {}, refusal {}",
                lane.family,
                match accepted {
                    Some(ms) => format!("{ms:>8.1} ms"),
                    None => "   no receipt (the open itself refused)".to_owned(),
                },
                match refusal_at {
                    Some(ms) => format!("observed at {ms:.1} ms"),
                    None => "answered the open itself (stderr)".to_owned(),
                }
            ));
        }
    }
    eprintln!("--- ttf-seek: time-to-first-output (DEV profile, this session) ---");
    for row in &rows {
        eprintln!("{row}");
    }
}

// ---------------------------------------------------------------------------
// (b) the in-session seek re-measure per family class
// ---------------------------------------------------------------------------

/// THE seek re-measure: a playing session per lane, one `seek 50%`
/// through the script verb, and the honest class pinned —
///
/// - **Lands**: exactly one `TimingSeekCompleted` at the
///   keyframe-safe actual (within ±10% of the media's half — the
///   demuxer's landing, never the request), the seek's minted
///   generation receipt, and NO reopen (one `Idle → Opening` for the
///   whole transcript — the D-4/D-7 shape);
/// - **DemuxReanchor / DecodeHeaders / NotSeekable**: the typed
///   `TimingSeekFailed` naming the refusing stage, no terminal
///   fabricated;
/// - **SilentWedge**: no terminal on stdout ever, and the host's
///   bounded-wait expiry on stderr (the recorded wav s24le gap — the
///   seek neither completes nor fails, honestly).
///
/// Latency (first `Buffering → Playing` → terminal line) prints on
/// stderr per lane.
#[test]
fn in_session_seek_per_family_class_through_the_binary() {
    let Some(media) = corpus_media() else {
        eprintln!("ttf-seek: corpus absent — measurement no-op");
        return;
    };
    let mut rows = Vec::new();
    for lane in lanes(&media) {
        // The refusal-only lanes never reach `Playing`: the seek
        // verb exists but in-session seek is unreachable — recorded,
        // not invented (their open-path pure seek is the
        // `--start-time` watermarks test below).
        if !lane.plays {
            rows.push(format!(
                "{:<15} seek UNREACHABLE (no first output — {})",
                lane.family,
                lane.refusal.must()
            ));
            continue;
        }
        let timed = timed_session(&["open", &lane.path], "seek 50%\nquit\n");
        assert_eq!(
            timed.code,
            0,
            "{}: stdout:\n{}",
            lane.family,
            transcript(&timed)
        );
        let anchor = timed.first_ms("state-changed Buffering → Playing");
        let anchor = anchor.must();
        let completed = timed.first_ms("event: TimingSeekCompleted");
        let failed = timed.first_ms("event: TimingSeekFailed");
        match lane.seek {
            SeekClass::Lands => {
                let terminal = completed.must_or("the seek must land with a terminal");
                let position_us = timed
                    .first_int_after("TimingSeekCompleted", "position: Timestamp(")
                    .must_or("the terminal carries its position payload");
                let duration_us = timed
                    .first_int_after("duration: Known", "time: Timestamp(")
                    .must_or("the media duration must be known for a landing lane");
                let half = duration_us as f64 / 2.0;
                assert!(
                    position_us as f64 >= half * 0.9 && position_us as f64 <= half * 1.1,
                    "{}: the landing {position_us} µs must be the ±10% neighborhood of \
                     the half-media {half} µs:\n{}",
                    lane.family,
                    transcript(&timed)
                );
                assert!(
                    failed.is_none(),
                    "{}: a landing seek fabricates no failure:\n{}",
                    lane.family,
                    transcript(&timed)
                );
                assert_eq!(
                    timed
                        .lines
                        .iter()
                        .filter(|(line, _)| line.contains("state-changed Idle → Opening"))
                        .count(),
                    1,
                    "{}: one open only — the seek must not reopen:\n{}",
                    lane.family,
                    transcript(&timed)
                );
                rows.push(format!(
                    "{:<15} seek LANDS at {position_us} µs (target 50% of {duration_us} µs); \
                     latency first-output→terminal {:>8.1} ms",
                    lane.family,
                    terminal - anchor
                ));
            }
            SeekClass::LandsWrong => {
                let terminal = completed.must_or("the wrong landing still carries a terminal");
                let position_us = timed
                    .first_int_after("TimingSeekCompleted", "position: Timestamp(")
                    .must_or("the terminal carries its position payload");
                let duration_us = timed
                    .first_int_after("duration: Known", "time: Timestamp(")
                    .must_or("the media duration must be known for a landing lane");
                let half = duration_us as f64 / 2.0;
                assert!(
                    position_us as f64 >= half * 1.1,
                    "{}: the recorded wrong-landing anomaly must stay OUTSIDE the ±10% \
                     half-neighborhood (it is the pinned honest shape, not a flake) \
                     — landed {position_us} µs of {duration_us} µs:\n{}",
                    lane.family,
                    transcript(&timed)
                );
                assert!(
                    position_us <= duration_us,
                    "{}: even the wrong landing stays in-media:\n{}",
                    lane.family,
                    transcript(&timed)
                );
                assert_eq!(
                    timed
                        .lines
                        .iter()
                        .filter(|(line, _)| line.contains("state-changed Idle → Opening"))
                        .count(),
                    1,
                    "{}: one open only — the seek must not reopen:\n{}",
                    lane.family,
                    transcript(&timed)
                );
                rows.push(format!(
                    "{:<15} seek LANDS WRONG at {position_us} µs (target 50% of {duration_us} µs — \
                     the recorded 11 kHz reanchor anomaly; the pure seek lands exactly); \
                     latency first-output→terminal {:>8.1} ms",
                    lane.family,
                    terminal - anchor
                ));
            }
            SeekClass::DemuxReanchor => {
                let terminal = failed.must_or("the reanchor refusal must carry a terminal");
                assert!(
                    timed.contains("plugin.demux.reanchor.unsupported"),
                    "{}: the demux reanchor refusal must be named:\n{}",
                    lane.family,
                    transcript(&timed)
                );
                assert!(
                    completed.is_none(),
                    "{}: no completion fabricated:\n{}",
                    lane.family,
                    transcript(&timed)
                );
                rows.push(format!(
                    "{:<15} seek REFUSES typed (demux reanchor unsupported — the recorded \
                     6.OPW-mp4 gap); latency first-output→terminal {:>8.1} ms",
                    lane.family,
                    terminal - anchor
                ));
            }
            SeekClass::DecodeHeaders => {
                let terminal = failed.must_or("the decode refusal must carry a terminal");
                assert!(
                    timed.contains("stage: Decode"),
                    "{}: the decode-stage refusal must be named:\n{}",
                    lane.family,
                    transcript(&timed)
                );
                assert!(
                    completed.is_none(),
                    "{}: no completion fabricated:\n{}",
                    lane.family,
                    transcript(&timed)
                );
                rows.push(format!(
                    "{:<15} seek REFUSES typed (decode: codec headers not replayed on \
                     reanchor); latency first-output→terminal {:>8.1} ms",
                    lane.family,
                    terminal - anchor
                ));
            }
            SeekClass::NotSeekable => {
                let terminal = failed.must_or("the unseekable refusal must carry a terminal");
                assert!(
                    timed.contains("the source cannot seek")
                        || timed.contains("could not be resolved"),
                    "{}: the unseekable/unresolvable refusal must be named:\n{}",
                    lane.family,
                    transcript(&timed)
                );
                rows.push(format!(
                    "{:<15} seek REFUSES typed (source unseekable / target unresolvable); \
                     latency first-output→terminal {:>8.1} ms",
                    lane.family,
                    terminal - anchor
                ));
            }
            SeekClass::SilentWedge => {
                assert!(
                    completed.is_none() && failed.is_none(),
                    "{}: the wedge fabricates no terminal:\n{}",
                    lane.family,
                    transcript(&timed)
                );
                assert!(
                    timed.stderr.contains("never reported the seek terminal"),
                    "{}: the bounded-wait expiry must be the honest stderr answer:\n{}\
                     stderr:\n{}",
                    lane.family,
                    transcript(&timed),
                    timed.stderr
                );
                // The rebuffer episode still ran in-pipeline (the
                // seek was accepted and minted); no reopen either.
                assert_eq!(
                    timed
                        .lines
                        .iter()
                        .filter(|(line, _)| line.contains("state-changed Idle → Opening"))
                        .count(),
                    1,
                    "{}: one open only — the seek must not reopen:\n{}",
                    lane.family,
                    transcript(&timed)
                );
                let exit_ms = timed.lines.last().map(|(_, ms)| *ms).unwrap_or_default();
                rows.push(format!(
                    "{:<15} seek ACCEPTED-THEN-SILENT (no terminal; bounded wait expired — \
                     the recorded wav gap); envelope first-output→exit {:>8.1} ms",
                    lane.family,
                    exit_ms - anchor
                ));
            }
        }
    }
    eprintln!("--- ttf-seek: in-session seek classes (DEV profile, this session) ---");
    for row in &rows {
        eprintln!("{row}");
    }
}

// ---------------------------------------------------------------------------
// (c) the --start-time pure-seek open-path watermarks
// ---------------------------------------------------------------------------

/// One `--start-time` lane: the open path's own seek (the pure
/// query + skip wrapper — the surface that works where in-session
/// reanchor is absent), measured as the open-receipt watermark.
const START_TIME_LANES: [(&str, &str, &str); 9] = [
    // (family, corpus-relative-or-fixture path, target)
    ("avi", "corpus:0076-avi-av.avi", "1s"),
    ("wav/11k", "corpus:0026-audio-only.wav", "2s"),
    ("wav/s24le", "corpus:0004-audio-only.wav", "1s"),
    (
        "aiff",
        "fixture:generated/valid/aiff_pcm_s16be_44k1_st.aiff",
        "50ms",
    ),
    ("mp4/mjpeg", "corpus:0115-fps-extremes.mp4", "1s"),
    ("mov/mjpeg", "corpus:0079-mov-video.mov", "1s"),
    ("adts/aac", "corpus:0059-audio-only.aac", "1s"),
    ("ivf/vp9", "codec:vp9/testsrc2_640x480_g30_90f.ivf", "500ms"),
    ("matroska/vp9", "corpus:0075-mkv-video.mkv", "500ms"),
];

/// The pure-seek watermarks: every lane opens twice — bare (the
/// baseline open receipt) and `--start-time <target>` (the same open
/// plus the demuxer's real seek query + skip wrapper) — and the
/// with-flag receipt must land the clock NEAR the target (the
/// keyframe-safe actual, never the request, never zero) except the
/// ts lane, whose open refuses typed (seekability `None` — pinned in
/// the same sweep). The delta between the two receipts is the
/// pure-seek watermark cost, printed on stderr.
#[test]
fn start_time_pure_seek_open_path_watermarks() {
    let Some(media) = corpus_media() else {
        eprintln!("ttf-seek: corpus absent — measurement no-op");
        return;
    };
    let resolve = |spec: &str| -> String {
        match spec.split_once(':') {
            Some(("corpus", name)) => media.join(name).to_string_lossy().into_owned(),
            Some(("fixture", rel)) => fixture(rel),
            Some(("codec", rel)) => codec_fixture(rel),
            _ => spec.to_owned(),
        }
    };
    // The ts lane: seekability None — the open itself refuses typed.
    let ts = media
        .join("0018-ts-video.ts")
        .to_string_lossy()
        .into_owned();
    let timed = timed_session(&["open", &ts, "--start-time", "1s"], "time\nquit\n");
    assert!(
        timed.stderr.contains("demux.ts.seek.unsupported"),
        "ts: the unseekable open refusal must be named:\n{}stderr:\n{}",
        transcript(&timed),
        timed.stderr
    );
    assert!(
        !timed.contains("ok: time"),
        "ts: an unseekable open answers no time snapshot:\n{}",
        transcript(&timed)
    );
    assert_eq!(
        timed.code, 65,
        "ts: the typed refusal is the process answer"
    );

    let mut rows = Vec::new();
    for (family, spec, target) in START_TIME_LANES {
        let path = resolve(spec);
        let target_s: f64 = match target.strip_suffix("ms") {
            Some(ms) => ms.parse::<f64>().must() / 1000.0,
            None => target
                .strip_suffix('s')
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
        };
        let bare = timed_session(&["open", &path], "quit\n");
        let skipped = timed_session(&["open", &path, "--start-time", target], "time\nquit\n");
        // The landing: the `ok: time <sec>s` snapshot after the open.
        let landing = skipped
            .lines
            .iter()
            .find_map(|(line, _)| line.strip_prefix("ok: time "))
            .and_then(|rest| rest.split('s').next())
            .and_then(|sec| sec.parse::<f64>().ok());
        let Some(landing) = landing else {
            unreachable!(
                "{family}: the --start-time open must answer a time snapshot:\n{}",
                transcript(&skipped)
            );
        };
        assert!(
            landing >= target_s - 0.05 && landing <= target_s + 1.0,
            "{family}: the pure-seek landing {landing}s must be near the {target_s}s target \
             (keyframe-safe, never zero; the observed run-to-run spread on the mp4/ivf \
             lanes is recorded in the worklog):\n{}",
            transcript(&skipped)
        );
        let bare_ms = bare.first_ms("ok: accepted session=1");
        let skip_ms = skipped.first_ms("ok: accepted session=1");
        let (Some(bare_ms), Some(skip_ms)) = (bare_ms, skip_ms) else {
            unreachable!(
                "{family}: both opens must answer their receipts:\nbare:\n{}\nskipped:\n{}",
                transcript(&bare),
                transcript(&skipped)
            );
        };
        rows.push(format!(
            "{:<15} --start-time {target}: lands at {landing:.6}s; open receipt bare \
             {:>8.1} ms → with-seek {:>8.1} ms (pure-seek watermark Δ {:>7.1} ms)",
            family,
            bare_ms,
            skip_ms,
            skip_ms - bare_ms
        ));
    }
    eprintln!("--- ttf-seek: --start-time pure-seek watermarks (DEV profile, this session) ---");
    for row in &rows {
        eprintln!("{row}");
    }
}

/// The transcript as text (for assertion messages).
fn transcript(timed: &Timed) -> String {
    timed
        .lines
        .iter()
        .map(|(line, ms)| format!("{ms:>9.1}  {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The house `Must` bridge for this file's `Option` answers (the
/// assertion-message carrier; never a bare unwrap).
trait MustOr<T> {
    fn must_or(self, context: &str) -> T;
}

impl<T> MustOr<T> for Option<T> {
    fn must_or(self, context: &str) -> T {
        match self {
            Some(value) => value,
            None => unreachable!("{context}"),
        }
    }
}
