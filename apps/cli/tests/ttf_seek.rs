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
//! in-session seek classes: AVI/AIFF land; the WAV lanes land in the
//! requested neighborhood — including every sub-44.1 kHz lane since
//! 6.FIX-pcm closed the recorded reanchor defects (pre-fix the
//! 11.025 kHz corpus lane 0026 answered every below-end target with a
//! landing deep past the request — measured 3 486 078 µs for the
//! 2 000 000 µs half, exactly one dropped 64 KiB packet past the
//! target: the queue's duration-cap livelock refused the first
//! post-reanchor packet, the pending-slot overwrite dropped it, and
//! the completion anchored on the SECOND packet; the s24le/u8 lanes
//! accepted the seek, rebuffered in-pipeline, and never reported a
//! terminal — the same livelock with every post-reanchor packet
//! unadmittable, wedging the bounded wait); MP4/MOV/M4A refuse typed at
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
    /// Accepted, then the typed demux refusal (the mp4 family's
    /// trait-default reanchor — the recorded 6.OPW-mp4 gap).
    DemuxReanchor,
    /// The seek lands in-pipeline at the PAGE-GRANULAR actual (the
    /// ogg family: pages are ~1 s on the corpus fixture, so the
    /// landing is the last page-start at-or-before the target — a
    /// coarse window, not the ±10% keyframe-dense containers answer).
    PageGranular,
    /// The source is unseekable (raw ES mpeg audio) / the target
    /// unresolvable (no duration): the typed control refusal.
    NotSeekable,
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
            // 6.FIX-pcm's reanchor pin: pre-fix this lane answered
            // every below-end target one dropped 64 KiB packet deep
            // past the request (measured 3 486 078 µs for the 2 s
            // half); the queue's single-item admission + the demux
            // pending-slot fix land it at the target exactly.
            seek: SeekClass::Lands,
        },
        Lane {
            family: "wav/s24le",
            path: corpus("0004-audio-only.wav"),
            plays: true,
            refusal: None,
            // 6.FIX-pcm's terminal pin: pre-fix this lane accepted
            // the seek and never reported a terminal (every
            // post-reanchor packet carried >1 s against the audio
            // queue's 1 s duration cap — the admission livelock
            // wedged the bounded wait).
            seek: SeekClass::Lands,
        },
        Lane {
            family: "wav/u8",
            path: corpus("0135-audio-only.wav"),
            plays: true,
            refusal: None,
            // The s24le lane's recorded twin (the W4 ledger's second
            // accepted-then-silent lane; 22.05 kHz — its 64 KiB
            // packets carried 1.486 s, over the same cap).
            seek: SeekClass::Lands,
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
            // 6.FIX-ogg-reanchor's pin: pre-fix this lane answered
            // the typed decode refusal `opus.head.missing` (the
            // engine's generation reconfigure minted a header-less
            // config); the host's re-seeding wrapper carries the
            // open-path OpusHead‖OpusTags blob across every
            // reconfigure and the demuxer's landing mark re-anchors
            // the granule counter — the seek now lands and plays out
            // (page-granular: the fixture's pages are ~1 s).
            seek: SeekClass::PageGranular,
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
///   fabricated.
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
            SeekClass::PageGranular => {
                let terminal = completed.must_or("the seek must land with a terminal");
                let position_us = timed
                    .first_int_after("TimingSeekCompleted", "position: Timestamp(")
                    .must_or("the terminal carries its position payload");
                let duration_us = timed
                    .first_int_after("duration: Known", "time: Timestamp(")
                    .must_or("the media duration must be known for a landing lane");
                let half = duration_us as f64 / 2.0;
                // The page-granular truth: the landing is the last
                // page-START at-or-before the target (the 6.FIX-ogg
                // landing-report semantics — delivery resumes at the
                // page's own bytes). One page of this fixture ≈ 1 s,
                // so the honest window is (target − 1.2 s, target].
                assert!(
                    (position_us as f64) <= half && position_us as f64 > half - 1_200_000.0,
                    "{}: the page-granular landing {position_us} µs must sit at-or-before \
                     the half-media {half} µs and within one page of it:\n{}",
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
                    "{:<15} seek LANDS page-granular at {position_us} µs (target 50% of \
                     {duration_us} µs); latency first-output→terminal {:>8.1} ms",
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
const START_TIME_LANES: [(&str, &str, &str); 10] = [
    // (family, corpus-relative-or-fixture path, target)
    ("avi", "corpus:0076-avi-av.avi", "1s"),
    ("wav/11k", "corpus:0026-audio-only.wav", "2s"),
    ("wav/s24le", "corpus:0004-audio-only.wav", "1s"),
    ("wav/u8", "corpus:0135-audio-only.wav", "1s"),
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
