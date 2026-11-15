//! 6.OPW-cli-conformance — the W3 cross-container CLI conformance
//! matrix, through the REAL binary (session scripts over piped stdio,
//! the `session_script.rs`/`unsupported_path.rs` pattern): one
//! systematic lane per wired container family — avi, wav, aiff,
//! adts(aac), ogg(vorbis), ogg(opus), mp4/mov/m4a, mkv/webm,
//! mpegts, ivf(av1/vp9), raw es (mp3/mp2/flac).
//!
//! Every row asserts the same five behaviors, each pinned to the
//! FAMILY's measured honest answer (this session's measurements, not
//! a uniform wish): open (accept, or the typed exit-65 refusal shape
//! where the lane's video limb has no decoder), the `advance` play
//! script, the `--no-video`/`--no-audio` selection limbs (accept on
//! the survivor; the typed `session.no-decodable-track` refusal when
//! the deselection empties the session), `--start-time` (the real
//! landing, or the typed `demux.ts.seek.unsupported` refusal on the
//! None-seek ts family), and `quit` → exit 0.
//!
//! Corpus lanes follow the `es.rs` convention: `LUMEN_CORPUS` or the
//! session default `/tmp/my-project/corpus-v4/media`, skipped (not
//! failed) when the corpus is absent. Committed-fixture lanes always
//! run. Positions pinned exactly where the lane measured
//! byte-stable across repeated runs; anchor-prefix pins carry a
//! comment naming the measured run-to-run jitter (see the worklog
//! findings: aac/av1/vp9 post-seek decode recovery is not
//! byte-deterministic, unlike the AVI parent's pinned determinism).
//! F3 (issue #16) closed the audio-less half of that jitter: an
//! audio-less session's position is the presentation latch (last
//! presented slot, else the anchor) — never wall-elapsed — so its
//! `--start-time` lanes pin exactly and run-to-run identically
//! (`audio_less_start_time_is_deterministic_across_runs`); the
//! audio-limb lanes' recovery jitter (mkv/webm) keeps its prefix
//! pins.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use testkit::Must;

/// Run the binary once with `args` and a piped stdin script; answer
/// (stdout, stderr, exit code). A fast-failing open exits before
/// consuming stdin — a closed pipe there is the expected shape.
fn run(args: &[&str], script: &str) -> (String, String, i32) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lumen"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .must();
    {
        let stdin = child.stdin.as_mut().must();
        if let Err(err) = stdin.write_all(script.as_bytes()) {
            assert_eq!(
                err.kind(),
                std::io::ErrorKind::BrokenPipe,
                "unexpected stdin write failure: {err}"
            );
        }
    }
    let output = child.wait_with_output().must();
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code().unwrap_or(-1),
    )
}

/// A committed fixture under this crate's `tests/fixtures`.
fn fixture(relative: &str) -> String {
    format!(
        "{}/../../tests/fixtures/{relative}",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// A committed fixture under the codec crates' fixture home (the
/// `es.rs`/`adts.rs` convention — real ffmpeg-authored bytes).
fn codec_fixture(name: &str) -> String {
    format!(
        "{}/../../crates/codecs/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// A corpus media file (`LUMEN_CORPUS` or the session default); the
/// es.rs skip convention — `None` means the corpus is absent and the
/// lane answers nothing (skipped, not failed).
fn corpus_file(name: &str) -> Option<String> {
    let dir = match std::env::var("LUMEN_CORPUS") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => PathBuf::from("/tmp/my-project/corpus-v4/media"),
    };
    let path = dir.join(name);
    path.is_file().then(|| path.to_string_lossy().into_owned())
}

/// The common assertion core for an ACCEPTING lane row: the open
/// receipt, and the clean quit.
fn assert_accepted(out: &str, code: i32, lane: &str) {
    assert_eq!(code, 0, "lane {lane} must exit clean:\nstdout:\n{out}");
    assert!(
        out.contains("ok: accepted session=1"),
        "lane {lane}:\n{out}"
    );
}

/// The common assertion core for the `--no-<kind>` deselection that
/// empties the session: the typed exit-65 `no-decodable-track`
/// refusal naming the container (measured shape on every audio-only
/// and video-only family).
fn assert_no_decodable_track(out: &str, err: &str, code: i32, container: &str, lane: &str) {
    assert_eq!(code, 65, "lane {lane}:\nstdout:\n{out}\nstderr:\n{err}");
    assert!(err.contains("[control.unsupported]"), "lane {lane}:\n{err}");
    assert!(
        err.contains("session.no-decodable-track"),
        "lane {lane}:\n{err}"
    );
    assert!(
        err.contains(&format!("container `{container}` tracks: (none reported)")),
        "lane {lane}:\n{err}"
    );
    assert!(!out.contains("ok: accepted"), "lane {lane}:\n{out}");
}

// ---------------------------------------------------------------------------
// AVI (the first vertical — mjpeg + pcm_s16le, the session_script
// parent fixture): the full-green row every other family is measured
// against.
// ---------------------------------------------------------------------------

/// AVI row: open accepts, `advance 512` runs the 2.000 s media to its
/// exact end, both selection flags accept (both limbs decodable),
/// `--start-time 1s` lands exactly at half the media, quit exits 0.
#[test]
fn avi_family_lane() {
    let file = fixture("generated/valid/avi_mjpeg_pcm.avi");
    let (out, err, code) = run(&["open", &file], "advance 512\ntime\nquit\n");
    assert_accepted(&out, code, "avi");
    assert!(err.is_empty(), "lane avi stderr:\n{err}");
    assert!(out.contains("event: end-of-media"), "{out}");
    assert!(out.contains("ok: time 2.000000s (100.0%)"), "{out}");

    let (out, err, code) = run(&["open", &file, "--no-video"], "quit\n");
    assert_accepted(&out, code, "avi --no-video");
    assert!(err.is_empty(), "lane avi --no-video stderr:\n{err}");

    let (out, _err, code) = run(&["open", &file, "--no-audio"], "quit\n");
    assert_accepted(&out, code, "avi --no-audio");

    let (out, err, code) = run(&["open", &file, "--start-time", "1s"], "time\nquit\n");
    assert_accepted(&out, code, "avi --start-time");
    assert!(err.is_empty(), "lane avi --start-time stderr:\n{err}");
    assert!(out.contains("ok: time 1.000000s (50.0%)"), "{out}");
}

// ---------------------------------------------------------------------------
// WAV / AIFF (the PCM families): audio-only rows — `--no-audio` is
// the typed empty-session refusal, `--no-video` is a no-op.
// ---------------------------------------------------------------------------

/// WAV row (committed 0.200 s s16le fixture): accepts, runs to the
/// exact end, `--no-audio` refuses typed, `--start-time 100ms` lands
/// at exactly half the media.
#[test]
fn wav_family_lane() {
    let file = fixture("generated/valid/wav_pcm_s16le_44k1_st.wav");
    let (out, err, code) = run(&["open", &file], "advance 512\ntime\nquit\n");
    assert_accepted(&out, code, "wav");
    assert!(err.is_empty(), "lane wav stderr:\n{err}");
    assert!(out.contains("event: end-of-media"), "{out}");
    assert!(out.contains("ok: time 0.200000s (100.0%)"), "{out}");

    let (out, _err, code) = run(&["open", &file, "--no-video"], "quit\n");
    assert_accepted(&out, code, "wav --no-video");

    let (out, err, code) = run(&["open", &file, "--no-audio"], "quit\n");
    assert_no_decodable_track(&out, &err, code, "wav", "wav --no-audio");

    let (out, err, code) = run(&["open", &file, "--start-time", "100ms"], "time\nquit\n");
    assert_accepted(&out, code, "wav --start-time");
    assert!(err.is_empty(), "lane wav --start-time stderr:\n{err}");
    assert!(out.contains("ok: time 0.100000s (50.0%)"), "{out}");
}

/// AIFF row (committed 0.200 s s16be fixture): the WAV row's exact
/// twin through the big-endian PCM family.
#[test]
fn aiff_family_lane() {
    let file = fixture("generated/valid/aiff_pcm_s16be_44k1_st.aiff");
    let (out, err, code) = run(&["open", &file], "advance 512\ntime\nquit\n");
    assert_accepted(&out, code, "aiff");
    assert!(err.is_empty(), "lane aiff stderr:\n{err}");
    assert!(out.contains("event: end-of-media"), "{out}");
    assert!(out.contains("ok: time 0.200000s (100.0%)"), "{out}");

    let (out, _err, code) = run(&["open", &file, "--no-video"], "quit\n");
    assert_accepted(&out, code, "aiff --no-video");

    let (out, err, code) = run(&["open", &file, "--no-audio"], "quit\n");
    assert_no_decodable_track(&out, &err, code, "aiff", "aiff --no-audio");

    let (out, err, code) = run(&["open", &file, "--start-time", "100ms"], "time\nquit\n");
    assert_accepted(&out, code, "aiff --start-time");
    assert!(err.is_empty(), "lane aiff --start-time stderr:\n{err}");
    assert!(out.contains("ok: time 0.100000s (50.0%)"), "{out}");
}

// ---------------------------------------------------------------------------
// ADTS / AAC-LC: the recorded limb-less gap — the container declares
// no audio geometry (the config rides in-band), so the open accepts
// but the audio limb is absent (the typed event, never a silent
// no-op); the clock only covers the decoded pre-roll.
// ---------------------------------------------------------------------------

/// ADTS row (corpus 0059, 4.083 s aac): accepts with the honest
/// `cli.audio_limb_absent` event, `--no-audio` refuses typed,
/// `--start-time 1s` lands exactly at the anchor (F3, issue #16:
/// the limb-less session's position is the anchor until a frame
/// presents — pre-fix the read wall-elapsed past the landing, the
/// measured 1.004017 s vs 1.004071 s across identical runs).
#[test]
fn adts_aac_family_lane() {
    let Some(file) = corpus_file("0059-audio-only.aac") else {
        eprintln!("SKIP adts lane: corpus dir absent");
        return;
    };
    let (out, err, code) = run(&["open", &file], "advance 512\ntime\nquit\n");
    assert_accepted(&out, code, "adts");
    assert!(err.is_empty(), "lane adts stderr:\n{err}");
    assert!(
        out.contains("[audio-output.unsupported]"),
        "the limb-less gap is a typed event:\n{out}"
    );
    assert!(out.contains("cli.audio_limb_absent"), "{out}");

    let (out, _err, code) = run(&["open", &file, "--no-video"], "quit\n");
    assert_accepted(&out, code, "adts --no-video");

    let (out, err, code) = run(&["open", &file, "--no-audio"], "quit\n");
    assert_no_decodable_track(&out, &err, code, "adts", "adts --no-audio");

    let (out, err, code) = run(&["open", &file, "--start-time", "1s"], "time\nquit\n");
    assert_accepted(&out, code, "adts --start-time");
    assert!(err.is_empty(), "lane adts --start-time stderr:\n{err}");
    assert!(
        out.contains("ok: time 1.000000s"),
        "the limb-less landing is the anchor, exactly (F3):\n{out}"
    );
}

// ---------------------------------------------------------------------------
// OGG (vorbis and opus): audio-only rows that play to their exact
// ends at ANY sample rate (the compressed packet cadence — contrast
// the sub-44.1 kHz PCM stall finding).
// ---------------------------------------------------------------------------

/// OGG/Vorbis row (corpus 0085, 4.032 s): accepts, runs to the exact
/// end, `--no-audio` refuses typed, `--start-time 1s` lands exactly.
#[test]
fn ogg_vorbis_family_lane() {
    let Some(file) = corpus_file("0085-audio-only.ogg") else {
        eprintln!("SKIP ogg-vorbis lane: corpus dir absent");
        return;
    };
    let (out, err, code) = run(&["open", &file], "advance 512\ntime\nquit\n");
    assert_accepted(&out, code, "ogg-vorbis");
    assert!(err.is_empty(), "lane ogg-vorbis stderr:\n{err}");
    assert!(out.contains("event: end-of-media"), "{out}");
    assert!(out.contains("ok: time 4.032000s (100.0%)"), "{out}");

    let (out, _err, code) = run(&["open", &file, "--no-video"], "quit\n");
    assert_accepted(&out, code, "ogg-vorbis --no-video");

    let (out, err, code) = run(&["open", &file, "--no-audio"], "quit\n");
    assert_no_decodable_track(&out, &err, code, "ogg", "ogg-vorbis --no-audio");

    let (out, err, code) = run(&["open", &file, "--start-time", "1s"], "time\nquit\n");
    assert_accepted(&out, code, "ogg-vorbis --start-time");
    assert!(
        err.is_empty(),
        "lane ogg-vorbis --start-time stderr:\n{err}"
    );
    assert!(out.contains("ok: time 1.000000s (25.0%)"), "{out}");
}

/// OGG/Opus row (corpus 0047, 4.007 s): the vorbis row's twin — the
/// eom lands at 99.8% (the container-declared 4.007 s extent vs the
/// 4.000 s decoded window — the honest container-fact answer).
#[test]
fn ogg_opus_family_lane() {
    let Some(file) = corpus_file("0047-audio-only.ogg") else {
        eprintln!("SKIP ogg-opus lane: corpus dir absent");
        return;
    };
    let (out, err, code) = run(&["open", &file], "advance 512\ntime\nquit\n");
    assert_accepted(&out, code, "ogg-opus");
    assert!(err.is_empty(), "lane ogg-opus stderr:\n{err}");
    assert!(out.contains("event: end-of-media"), "{out}");
    assert!(out.contains("ok: time 4.000000s (99.8%)"), "{out}");

    let (out, err, code) = run(&["open", &file, "--no-audio"], "quit\n");
    assert_no_decodable_track(&out, &err, code, "ogg", "ogg-opus --no-audio");

    let (out, err, code) = run(&["open", &file, "--start-time", "1s"], "time\nquit\n");
    assert_accepted(&out, code, "ogg-opus --start-time");
    assert!(err.is_empty(), "lane ogg-opus --start-time stderr:\n{err}");
    assert!(out.contains("ok: time 1.000000s (25.0%)"), "{out}");
}

// ---------------------------------------------------------------------------
// MP4 / MOV / M4A (one registry family, one demuxer): the mjpeg+pcm
// fixture is the full-green row; the mov and m4a corpus lanes cover
// the quicktime and audio-only probe shapes.
// ---------------------------------------------------------------------------

/// MP4 row (committed mjpeg+pcm fixture, 2.000 s): the AVI row's
/// exact twin through the ISO family — full play-out, both limbs
/// selectable, the seek lands at half.
#[test]
fn mp4_family_lane() {
    let file = fixture("generated/valid/mp4_mjpeg_pcm.mp4");
    let (out, err, code) = run(&["open", &file], "advance 512\ntime\nquit\n");
    assert_accepted(&out, code, "mp4");
    assert!(err.is_empty(), "lane mp4 stderr:\n{err}");
    assert!(out.contains("event: end-of-media"), "{out}");
    assert!(out.contains("ok: time 2.000000s (100.0%)"), "{out}");

    let (out, _err, code) = run(&["open", &file, "--no-video"], "quit\n");
    assert_accepted(&out, code, "mp4 --no-video");

    let (out, _err, code) = run(&["open", &file, "--no-audio"], "quit\n");
    assert_accepted(&out, code, "mp4 --no-audio");

    let (out, err, code) = run(&["open", &file, "--start-time", "1s"], "time\nquit\n");
    assert_accepted(&out, code, "mp4 --start-time");
    assert!(err.is_empty(), "lane mp4 --start-time stderr:\n{err}");
    assert!(out.contains("ok: time 1.000000s (50.0%)"), "{out}");
}

/// MOV row (corpus 0051, vfr mjpeg + pcm_s16le 32 kHz 6ch): the open
/// surface is fully green (both flags accept, the seek lands at a
/// third of the 2.322 s media), and — since 6.FIX-pcm closed the
/// sub-44.1 kHz stall (the 6-channel ring-alignment half; the
/// measured face: every `write_bytes` chunk push refused, 256 % 6 = 4,
/// misreported as `audio.ring_full`) — the play script runs the
/// media to its end: eom at the audio-master clock's end (2.976 s of
/// the container's 3.005 s video-led extent — 99.0%, the honest
/// audio-master answer; pre-fix the position was frozen at the
/// anchor under any advance count).
#[test]
fn mov_family_lane() {
    let Some(file) = corpus_file("0051-vfr-av.mov") else {
        eprintln!("SKIP mov lane: corpus dir absent");
        return;
    };
    let (out, err, code) = run(&["open", &file], "advance 4096\ntime\nquit\n");
    assert_accepted(&out, code, "mov");
    assert!(err.is_empty(), "lane mov stderr:\n{err}");
    assert!(out.contains("event: end-of-media"), "{out}");
    assert!(
        out.contains("ok: time 2.976000s (99.0%)"),
        "the sub-44.1 kHz 6-channel lane must run to its audio-master end:\n{out}"
    );
    assert!(
        !out.contains("audio.ring_full"),
        "the 6-channel ring alignment must not refuse writes:\n{out}"
    );

    let (out, _err, code) = run(&["open", &file, "--no-video"], "quit\n");
    assert_accepted(&out, code, "mov --no-video");

    let (out, _err, code) = run(&["open", &file, "--no-audio"], "quit\n");
    assert_accepted(&out, code, "mov --no-audio");

    let (out, err, code) = run(&["open", &file, "--start-time", "1s"], "time\nquit\n");
    assert_accepted(&out, code, "mov --start-time");
    assert!(err.is_empty(), "lane mov --start-time stderr:\n{err}");
    assert!(out.contains("ok: time 1.000000s (33.3%)"), "{out}");
}

/// M4A row (corpus 0029, aac + mjpeg cover, 0.171 s): the esds
/// geometry makes this the PLAYING aac lane (contrast the limb-less
/// ADTS row) — full play-out to the container-declared end, both
/// flags accept, the seek lands near half.
#[test]
fn m4a_family_lane() {
    let Some(file) = corpus_file("0029-cover-audio.m4a") else {
        eprintln!("SKIP m4a lane: corpus dir absent");
        return;
    };
    let (out, err, code) = run(&["open", &file], "advance 512\ntime\nquit\n");
    assert_accepted(&out, code, "m4a");
    assert!(err.is_empty(), "lane m4a stderr:\n{err}");
    assert!(out.contains("event: end-of-media"), "{out}");
    assert!(out.contains("ok: time 0.170666s (100.0%)"), "{out}");

    let (out, _err, code) = run(&["open", &file, "--no-video"], "quit\n");
    assert_accepted(&out, code, "m4a --no-video");

    let (out, _err, code) = run(&["open", &file, "--no-audio"], "quit\n");
    assert_accepted(&out, code, "m4a --no-audio");

    let (out, err, code) = run(&["open", &file, "--start-time", "40ms"], "time\nquit\n");
    assert_accepted(&out, code, "m4a --start-time");
    assert!(err.is_empty(), "lane m4a --start-time stderr:\n{err}");
    assert!(out.contains("ok: time 0.040000s (46.9%)"), "{out}");
}

// ---------------------------------------------------------------------------
// MKV / WebM (the matroska registry id): the corpus mkv lane and the
// committed webm fixture both DECODE since the 2026-11-15 landings
// (the H.264 software track + the color-converter extension) — the
// rows assert the playing shape (the pre-landing video-refusal /
// color-gate shapes are closed).
// ---------------------------------------------------------------------------

/// MP4/H.264 row (committed h264+aac fixture, 1.533 s): the
/// 2026-11-15 software-track landing's own lane — full play-out on
/// both limbs (the `rust_h264` engine behind the codec-api
/// contract, the avcC-seeded `extra_data` path, I420 +
/// BT.709-limited present through the extended converter), the
/// seek lands at the anchor, and `--no-video`/`--no-audio` keep
/// their limbs.
#[test]
fn mp4_h264_family_lane() {
    let file = fixture("generated/valid/mp4_h264_aac.mp4");
    let (out, err, code) = run(&["open", &file], "advance 512\ntime\nquit\n");
    assert_accepted(&out, code, "mp4-h264");
    assert!(err.is_empty(), "lane mp4-h264 stderr:\n{err}");
    assert!(out.contains("event: end-of-media"), "{out}");
    assert!(out.contains("state-changed Playing → Ended"), "{out}");
    assert!(out.contains("ok: time 1.532517s (100.0%)"), "{out}");
    assert!(
        !out.contains("cli.codec_unavailable"),
        "the h264 limb must resolve to the software decoder:\n{out}"
    );
    assert!(
        !out.contains("cli.i420_color_unsupported"),
        "BT.709-limited I420 presents through the extended converter:\n{out}"
    );

    let (out, _err, code) = run(&["open", &file, "--no-video"], "quit\n");
    assert_accepted(&out, code, "mp4-h264 --no-video");

    let (out, _err, code) = run(&["open", &file, "--no-audio"], "quit\n");
    assert_accepted(&out, code, "mp4-h264 --no-audio");

    let (out, err, code) = run(&["open", &file, "--start-time", "500ms"], "time\nquit\n");
    assert_accepted(&out, code, "mp4-h264 --start-time");
    assert!(err.is_empty(), "lane mp4-h264 --start-time stderr:\n{err}");
    assert!(
        out.contains("ok: time 0.50"),
        "the anchor lands past 500 ms (jitter tail):\n{out}"
    );
}

/// MKV row (corpus 0003, h264 + mp3 + vorbis + subs): since the
/// 2026-11-15 software-track landing the h264 limb DECODES (the
/// pre-landing exit-65 refusal shape is closed — the lane was the
/// `cli.codec_unavailable` pin); the open accepts on all limbs, the
/// audio-led playback reaches its end. Exact position pins stay
/// [VERIFY] until a corpus-enabled run re-measures them (the corpus
/// regenerates with fresh seeds; this tree self-skips without it).
#[test]
fn mkv_family_lane() {
    let Some(file) = corpus_file("0003-multi-av.mkv") else {
        eprintln!("SKIP mkv lane: corpus dir absent");
        return;
    };
    let (out, err, code) = run(&["open", &file], "advance 4096\ntime\nquit\n");
    assert_accepted(&out, code, "mkv");
    assert!(
        !out.contains("cli.codec_unavailable"),
        "the h264 limb resolves to the software decoder:\n{out}"
    );
    assert!(
        !err.contains("no decoder for `h264`"),
        "the pre-landing refusal shape is closed:\n{err}"
    );
    assert!(out.contains("state-changed Buffering → Playing"), "{out}");
    assert!(out.contains("event: end-of-media"), "{out}");
    assert!(out.contains("state-changed Playing → Ended"), "{out}");

    let (out, err, code) = run(&["open", &file, "--no-video"], "quit\n");
    assert_accepted(&out, code, "mkv --no-video");
    assert!(err.is_empty(), "lane mkv --no-video stderr:\n{err}");

    let (out, err, code) = run(&["open", &file, "--no-audio"], "quit\n");
    assert_accepted(&out, code, "mkv --no-audio");
    assert!(
        !err.contains("no decoder for `h264`"),
        "the h264 video limb carries the session without audio:\n{err}"
    );

    let (out, err, code) = run(
        &["open", &file, "--no-video", "--start-time", "500ms"],
        "time\nquit\n",
    );
    assert_accepted(&out, code, "mkv --no-video --start-time");
    assert!(
        err.is_empty(),
        "lane mkv --no-video --start-time stderr:\n{err}"
    );
    assert!(
        out.contains("ok: time 0.50"),
        "the anchor lands past 500 ms (jitter tail):\n{out}"
    );
}

/// WebM row (committed vp9+opus fixture, 2.008 s): since the
/// 2026-11-15 color-converter extension the I420 BT.709-limited
/// frames PRESENT (the pre-extension shape — the
/// `cli.i420_color_unsupported` refusal parking the engine in
/// `Error` — is closed): full play-out to the end on both limbs,
/// and the seek still lands at the anchor (prefix pin: vp9 recovery
/// jitter).
#[test]
fn webm_family_lane() {
    let file = fixture("generated/valid/webm_vp9_opus.webm");
    // The audio-master null sink advances on real time — the pumps
    // are repeated until the 2.008 s media plays out (each `advance`
    // drives the render loop while wall time passes).
    let mut script = String::new();
    for _ in 0..40 {
        script.push_str("advance 512\n");
    }
    script.push_str("time\nquit\n");
    let (out, err, code) = run(&["open", &file], &script);
    assert_accepted(&out, code, "webm");
    assert!(err.is_empty(), "lane webm stderr:\n{err}");
    assert!(out.contains("event: end-of-media"), "{out}");
    assert!(out.contains("state-changed Playing → Ended"), "{out}");
    assert!(
        out.contains("ok: time 2.013500s"),
        "the audio-master clock runs the media to its end:\n{out}"
    );
    assert!(
        !out.contains("cli.i420_color_unsupported"),
        "BT.709-limited I420 presents through the extended converter:\n{out}"
    );
    assert!(
        !out.contains("state-changed Buffering → Error"),
        "the pre-extension parked-Error shape is closed:\n{out}"
    );

    let (out, _err, code) = run(&["open", &file, "--no-video"], "quit\n");
    assert_accepted(&out, code, "webm --no-video");

    let (out, _err, code) = run(&["open", &file, "--no-audio"], "quit\n");
    assert_accepted(&out, code, "webm --no-audio");

    let (out, err, code) = run(&["open", &file, "--start-time", "500ms"], "time\nquit\n");
    assert_accepted(&out, code, "webm --start-time");
    assert!(err.is_empty(), "lane webm --start-time stderr:\n{err}");
    assert!(
        out.contains("ok: time 0.50"),
        "the anchor lands past 500 ms (jitter tail):\n{out}"
    );
}

// ---------------------------------------------------------------------------
// MPEG-TS: the None-seek family — `--start-time` is the typed
// `demux.ts.seek.unsupported` refusal (never a faked placement),
// and the hevc corpus lane pins the video-refusal row with the aac
// limb surviving under `--no-video`.
// ---------------------------------------------------------------------------

/// MPEG-TS row (corpus 0091, hevc + aac): the open refuses typed on
/// the hevc limb; `--no-video` accepts on the aac limb (with the
/// same honest limb-less event the raw-ADTS row pins — no PSI
/// geometry); `--no-audio` still refuses; `--start-time` refuses at
/// the demux stage, the family's SeekSupport::None answer.
#[test]
fn mpegts_family_lane() {
    let Some(file) = corpus_file("0091-bframe-storm.ts") else {
        eprintln!("SKIP mpegts lane: corpus dir absent");
        return;
    };
    let (out, err, code) = run(&["open", &file], "quit\n");
    assert_eq!(code, 65, "the hevc refusal is exit 65:\n{out}\n{err}");
    assert!(out.contains("event: error [decode.unsupported]"), "{out}");
    assert!(err.contains("no decoder for `hevc`"), "{err}");

    let (out, err, code) = run(&["open", &file, "--no-video"], "quit\n");
    assert_accepted(&out, code, "mpegts --no-video");
    assert!(err.is_empty(), "lane mpegts --no-video stderr:\n{err}");
    assert!(out.contains("cli.audio_limb_absent"), "{out}");

    let (out, err, code) = run(&["open", &file, "--no-audio"], "quit\n");
    assert_eq!(code, 65, "the hevc limb remains:\n{out}\n{err}");
    assert!(err.contains("no decoder for `hevc`"), "{err}");

    let (out, err, code) = run(&["open", &file, "--start-time", "1s"], "quit\n");
    assert_eq!(code, 65, "the None-seek refusal is exit 65:\n{out}\n{err}");
    assert!(err.contains("[demux.unsupported]"), "{err}");
    assert!(err.contains("demux.ts.seek.unsupported"), "{err}");
    assert!(!out.contains("ok: accepted"), "{out}");
}

// ---------------------------------------------------------------------------
// IVF (av1 + vp9): video-only rows — `--no-video` empties the
// session (the typed refusal). Since the 2026-11-15 color-converter
// extension the decoded i420 frames PRESENT (the pre-extension
// `cli.i420_color_unsupported` refusal parking the engine in `Error`
// is closed); the wall-clock pacing of a video-only session keeps
// the row at the playing boundary (end-of-media is the webm twin's
// audio-master assertion); `--start-time` lands at the anchor
// (prefix pins: the rav1d/vp9 mid-GOP recovery jitter).
// ---------------------------------------------------------------------------

/// The shared IVF row body (both committed fixtures measured the
/// same shape).
fn ivf_row(file: &str, lane: &str) {
    let (out, err, code) = run(&["open", file], "advance 512\ntime\nquit\n");
    assert_accepted(&out, code, lane);
    assert!(err.is_empty(), "lane {lane} stderr:\n{err}");
    assert!(out.contains("state-changed Buffering → Playing"), "{out}");
    assert!(
        out.contains("state: Playing (position "),
        "the video limb plays through the extended converter:\n{out}"
    );
    assert!(
        !out.contains("cli.i420_color_unsupported"),
        "BT.709-limited I420 presents (the pre-extension gate is closed):\n{out}"
    );
    assert!(
        !out.contains("state-changed Buffering → Error"),
        "the pre-extension parked-Error shape is closed:\n{out}"
    );

    let (out, err, code) = run(&["open", file, "--no-video"], "quit\n");
    assert_no_decodable_track(&out, &err, code, "ivf", &format!("{lane} --no-video"));

    let (out, _err, code) = run(&["open", file, "--no-audio"], "quit\n");
    assert_accepted(&out, code, &format!("{lane} --no-audio"));

    let (out, err, code) = run(&["open", file, "--start-time", "200ms"], "time\nquit\n");
    assert_accepted(&out, code, &format!("{lane} --start-time"));
    assert!(err.is_empty(), "lane {lane} --start-time stderr:\n{err}");
    assert!(
        out.contains("ok: time 0.200000s"),
        "the audio-less landing is the anchor, exactly (F3):\n{out}"
    );
}

/// F3 (issue #16): an audio-less session's `--start-time` answer is
/// deterministic — the same input run twice answers byte-identical
/// output. Pre-fix the position read wall-elapsed past the anchor
/// (measured ±60-200 µs across identical runs on the limb-less
/// lanes); the cure is the CLI clock policy: report the presentation
/// latch (last presented slot, else the anchor), never wall-elapsed.
/// The committed av1 fixture is the always-running audio-less lane
/// (video-only media, no audio limb to master the clock).
#[test]
fn audio_less_start_time_is_deterministic_across_runs() {
    let file = fixture("generated/ivf/av1_64x48_30fps.ivf");
    let args = ["open", &file, "--start-time", "200ms"];
    let (first, first_err, first_code) = run(&args, "time\nquit\n");
    assert_accepted(&first, first_code, "ivf determinism run 1");
    assert!(first_err.is_empty(), "run 1 stderr:\n{first_err}");
    let (second, second_err, second_code) = run(&args, "time\nquit\n");
    assert_accepted(&second, second_code, "ivf determinism run 2");
    assert!(second_err.is_empty(), "run 2 stderr:\n{second_err}");
    assert_eq!(
        first, second,
        "identical inputs must answer byte-identical output (F3 \
         wall-elapsed position read):\nrun 1:\n{first}\nrun 2:\n{second}"
    );
    assert!(
        first.contains("ok: time 0.200000s"),
        "the landing is the anchor, exactly:\n{first}"
    );
}

/// IVF/AV1 row (committed 64x48 30 fps fixture).
#[test]
fn ivf_av1_family_lane() {
    ivf_row(&fixture("generated/ivf/av1_64x48_30fps.ivf"), "ivf-av1");
}

/// IVF/VP9 row (committed 64x48 25 fps fixture) — the av1 row's
/// twin through the vp9 decoder (the W3 reconciliation's second
/// provider).
#[test]
fn ivf_vp9_family_lane() {
    ivf_row(&fixture("generated/ivf/vp9_64x48_25fps.ivf"), "ivf-vp9");
}

// ---------------------------------------------------------------------------
// Raw ES (mp3 / mp2 / flac): the elementary-stream rows — honest
// `(0.0%)` positions (no declared extent), and the flac row is the
// registry's honest typed refusal (the demuxer has not landed).
// ---------------------------------------------------------------------------

/// ES/MP3 row (corpus 0139, 4.127 s): accepts, plays to the decoded
/// end (the honest 0.0% — the stream declares no extent), the
/// deselection refuses typed, the seek lands exactly.
#[test]
fn es_mp3_family_lane() {
    let Some(file) = corpus_file("0139-audio-only.mp3") else {
        eprintln!("SKIP es-mp3 lane: corpus dir absent");
        return;
    };
    let (out, err, code) = run(&["open", &file], "advance 512\ntime\nquit\n");
    assert_accepted(&out, code, "es-mp3");
    assert!(err.is_empty(), "lane es-mp3 stderr:\n{err}");
    assert!(out.contains("event: end-of-media"), "{out}");
    assert!(out.contains("ok: time 4.079365s (0.0%)"), "{out}");

    let (out, _err, code) = run(&["open", &file, "--no-video"], "quit\n");
    assert_accepted(&out, code, "es-mp3 --no-video");

    let (out, err, code) = run(&["open", &file, "--no-audio"], "quit\n");
    assert_no_decodable_track(&out, &err, code, "mp3", "es-mp3 --no-audio");

    let (out, err, code) = run(&["open", &file, "--start-time", "1s"], "time\nquit\n");
    assert_accepted(&out, code, "es-mp3 --start-time");
    assert!(err.is_empty(), "lane es-mp3 --start-time stderr:\n{err}");
    assert!(out.contains("ok: time 1.000000s (0.0%)"), "{out}");
}

/// ES/MP2 row (committed codec fixture, 1.512 s): the mp3 row's twin
/// through the mp2 lane of the same mpeg-audio host.
#[test]
fn es_mp2_family_lane() {
    let file = codec_fixture("mp2_48k.mp2");
    let (out, err, code) = run(&["open", &file], "advance 512\ntime\nquit\n");
    assert_accepted(&out, code, "es-mp2");
    assert!(err.is_empty(), "lane es-mp2 stderr:\n{err}");
    assert!(out.contains("event: end-of-media"), "{out}");
    assert!(out.contains("ok: time 1.512000s (0.0%)"), "{out}");

    let (out, _err, code) = run(&["open", &file, "--no-video"], "quit\n");
    assert_accepted(&out, code, "es-mp2 --no-video");

    let (out, err, code) = run(&["open", &file, "--no-audio"], "quit\n");
    assert_no_decodable_track(&out, &err, code, "mp2", "es-mp2 --no-audio");

    let (out, err, code) = run(&["open", &file, "--start-time", "1s"], "time\nquit\n");
    assert_accepted(&out, code, "es-mp2 --start-time");
    assert!(err.is_empty(), "lane es-mp2 --start-time stderr:\n{err}");
    assert!(out.contains("ok: time 1.000000s (0.0%)"), "{out}");
}

/// ES/FLAC row (committed codec fixture): the family's honest answer
/// TODAY is the typed probe-stage refusal — the registry arm exists
/// but the raw-FLAC elementary-stream demuxer has not landed (the
/// refusal names the landing card's contract; FLAC plays through the
/// wired containers — the mkv row's flac limb — just not as raw ES).
#[test]
fn es_flac_family_lane_is_the_honest_refusal() {
    let file = codec_fixture("flac_s16_stereo_44100.flac");
    let (out, err, code) = run(&["open", &file], "quit\n");
    assert_eq!(code, 65, "the missing demuxer is exit 65:\n{out}\n{err}");
    assert!(err.contains("[probe.unsupported]"), "{err}");
    assert!(err.contains("format `flac` probes"), "{err}");
    assert!(
        err.contains("raw-FLAC elementary-stream demuxer has not landed"),
        "{err}"
    );
    assert!(!out.contains("ok: accepted"), "{out}");
}
