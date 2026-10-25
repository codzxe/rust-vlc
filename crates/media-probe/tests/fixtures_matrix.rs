//! Card 2.A2's fixture-classification AC (doc 13 wave 2.A row 2.A2):
//! the whole committed probe corpus — both class-A parents and all 20
//! class-B derivatives — classified through the **real access stack**
//! (`input`'s `FileAccess` → `FileStream`, card 2.A1's composition).
//!
//! Every expected verdict below is derived from the fixture's own
//! recorded nature (the `expectation/1` TOMLs' `[derivation]` blocks
//! and the byte-level ground truth the 2.A3 writer pinned), not from
//! the probe's current output — the matrix is the spec the signature
//! must reproduce:
//!
//! | fixture | verdict | why |
//! |---------|---------|-----|
//! | both parents | AVI/MP4 @ 95, both track hints | full header in the window |
//! | `trunc0` (12 B / 28 B) | AVI @ 70 / MP4 @ 70 | magic/ftyp only — past the magic, still the family |
//! | `trunc1` (52 B / 64 B) | @ 90 | primary header chunk visible, tracks not yet |
//! | `trunc2..4` | @ 95, both hints | whole header region in the window |
//! | `flip1` | like the parent | advisory byte, structure intact |
//! | `flip4` | like the parent | lying sizes clamp to the window, walk survives |
//! | `flip16` | typed unknown | the seeded flips break the RIFF magic / `ftyp` type — no anchor |
//! | `garbage0` | like the parent | the 4 KiB seeded garbage is an **append** past the header (corpus-plan B1); the probe never reaches it |
//! | `noidx0` | AVI @ 95 | idx1 lives past hdrl; the signature never reads it |
//! | `stszlie0` | MP4 @ 95 | the stsz count lie sits *inside* the 64 KiB window but in a box the signature never opens |
//!
//! The two honest deviations from the card's parenthetical are
//! recorded in the worklog: (1) "garbage → unknown" is delivered by
//! the pure-garbage real-file test in `probe_io.rs` — the corpus's
//! `garbage0` is an append, so the honest verdict is the parent's;
//! (2) "trunc-past-magic → unknown" — every committed trunc fixture
//! keeps at least the magic by design (2.A3's `trunc0` cuts *after*
//! the magic), so the sub-magic cut is proven by the truncation sweep
//! in `probe_io.rs` over real files.

mod common;

use common::{fixture, probe_file};
use media_probe::{Confidence, ProbeConfig, ProbeReport};
use media_types::PipelineStage;
use media_types::ids::MediaKind;
use testkit::Must;

// ---------------------------------------------------------------------------
// expectation/1 subset: the recorded fields this suite consumes
// ---------------------------------------------------------------------------

/// The `[tracks]` codec names an expectation TOML records.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TrackCodecs {
    video: String,
    audio: String,
}

/// Read the `[tracks]` video/audio codec names from an `expectation/1`
/// TOML (`video = [{ … codec = "mjpeg" … }]`).
fn recorded_track_codecs(relative: &str) -> TrackCodecs {
    let text = std::fs::read_to_string(fixture(relative)).must();
    let mut video = String::new();
    let mut audio = String::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("video = [") {
            video = inline_string_field(rest, "codec");
        }
        if let Some(rest) = line.strip_prefix("audio = [") {
            audio = inline_string_field(rest, "codec");
        }
    }
    assert!(!video.is_empty(), "video codec recorded in {relative}");
    assert!(!audio.is_empty(), "audio codec recorded in {relative}");
    TrackCodecs { video, audio }
}

/// `codec = "…"` inside an inline table — the first quoted value after
/// the field name.
fn inline_string_field(line: &str, field: &str) -> String {
    let needle = format!("{field} = \"");
    let start = match line.find(&needle) {
        Some(at) => at + needle.len(),
        None => unreachable!("field {field} recorded"),
    };
    let rest = &line[start..];
    let end = match rest.find('"') {
        Some(at) => at,
        None => unreachable!("field {field} is a quoted string"),
    };
    rest[..end].to_owned()
}

/// A class-B `[derivation]` block.
struct Derivation {
    op: String,
    cut_offset: Option<u64>,
}

/// Read the `[derivation]` op and cut offset from an `expectation/1`
/// TOML — the recorded derivation the verdict rationale cites.
fn recorded_derivation(relative: &str) -> Derivation {
    let text = std::fs::read_to_string(fixture(relative)).must();
    let mut op = String::new();
    let mut cut_offset = None;
    let mut in_derivation = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_derivation = &line[1..line.len() - 1] == "derivation";
            continue;
        }
        if !in_derivation {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let value = value.trim().trim_matches('"');
            match key.trim() {
                "op" => op = value.to_owned(),
                "cut_offset" => {
                    let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
                    cut_offset = digits.parse::<u64>().ok();
                }
                _ => {}
            }
        }
    }
    assert!(!op.is_empty(), "derivation op recorded in {relative}");
    Derivation { op, cut_offset }
}

/// One expected matrix verdict.
enum Expected {
    /// A selectable hit: format, rung, and the sniffed hints (kind,
    /// codec) in walk order.
    Hit {
        format: &'static str,
        score: u8,
        hints: &'static [(&'static str, &'static str)],
    },
    /// No signature anchors: the typed unknown of doc 03 §4.2(d),
    /// carrying **no** format name (nothing matched at all).
    Unknown,
}

const BOTH_HINTS: &[(&str, &str)] = &[("video", "mjpeg"), ("audio", "pcm_s16le")];
const AUDIO_ONLY: &[(&str, &str)] = &[("audio", "pcm_s16le")];

fn default_config() -> ProbeConfig {
    // The doc 03 §4.2 defaults: 64 KiB window, min score 60.
    ProbeConfig::default()
}

fn kind_of(name: &str) -> MediaKind {
    match name {
        "video" => MediaKind::Video,
        "audio" => MediaKind::Audio,
        other => unreachable!("matrix only names video/audio tracks: {other}"),
    }
}

fn assert_verdict(fixture_relative: &str, expected: &Expected) {
    let config = default_config();
    let path = fixture(fixture_relative);
    let size = std::fs::metadata(&path).must().len() as usize;
    match expected {
        Expected::Hit {
            format,
            score,
            hints,
        } => {
            let report = match probe_file(&path, &config) {
                Ok(report) => report,
                Err(err) => unreachable!("{fixture_relative} must probe: {err}"),
            };
            assert_eq!(
                report.format.as_str(),
                *format,
                "{fixture_relative}: format"
            );
            assert_eq!(
                report.confidence.as_u8(),
                *score,
                "{fixture_relative}: score"
            );
            assert!(
                report.confidence.meets(config.min_score),
                "{fixture_relative}: selectable above min_score"
            );
            let got: Vec<(MediaKind, &str)> = report
                .track_hints
                .iter()
                .map(|h| (h.kind, h.codec.as_str()))
                .collect();
            let want: Vec<(MediaKind, &str)> = hints
                .iter()
                .map(|(kind, codec)| (kind_of(kind), *codec))
                .collect();
            assert_eq!(got, want, "{fixture_relative}: hints");
            // The window is the leading min(bound, file) bytes.
            let window = config.max_lookahead.min(size);
            assert_eq!(
                report.bytes_examined, window,
                "{fixture_relative}: bytes examined"
            );
        }
        Expected::Unknown => {
            let err = match probe_file(&path, &config) {
                Ok(report) => unreachable!("{fixture_relative} must not probe: {report:?}"),
                Err(err) => err,
            };
            assert_eq!(
                err.context.code, "probe.no-demuxer",
                "{fixture_relative}: code"
            );
            assert_eq!(err.stage, PipelineStage::Probe, "{fixture_relative}: stage");
            match &err.kind {
                media_types::ErrorKind::Unsupported { format } => {
                    // No signature anchored at all — not even a
                    // sub-threshold family claim survives a broken
                    // magic, so nothing is named (never a guess).
                    if let Some(info) = format {
                        assert!(
                            info.name.is_none(),
                            "{fixture_relative}: the broken magic names no format"
                        );
                    }
                }
                other => unreachable!("{fixture_relative}: wrong kind: {other:?}"),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Class-A parents
// ---------------------------------------------------------------------------

#[test]
fn avi_parent_probes_avi_with_both_track_hints() {
    let report = match probe_file(
        &fixture("generated/valid/avi_mjpeg_pcm.avi"),
        &default_config(),
    ) {
        Ok(report) => report,
        Err(err) => unreachable!("parent must probe: {err}"),
    };
    assert_eq!(report.format.as_str(), "avi");
    assert_eq!(report.confidence.as_u8(), 95);
    // The hints name the demuxers' own codec names, verified against
    // the RECORDED expectation TOML (not a transcription).
    let recorded = recorded_track_codecs("expectations/valid/avi_mjpeg_pcm.avi.toml");
    let hints: Vec<&str> = report
        .track_hints
        .iter()
        .map(|h| h.codec.as_str())
        .collect();
    assert_eq!(
        hints,
        vec![recorded.video.as_str(), recorded.audio.as_str()]
    );
    assert_eq!(report.track_hints[0].kind, MediaKind::Video);
    assert_eq!(report.track_hints[1].kind, MediaKind::Audio);
    assert_eq!(
        report.bytes_examined, 65_536,
        "window-capped, not file-capped"
    );
}

#[test]
fn mp4_parent_probes_mp4_with_both_track_hints() {
    let report = match probe_file(
        &fixture("generated/valid/mp4_mjpeg_pcm.mp4"),
        &default_config(),
    ) {
        Ok(report) => report,
        Err(err) => unreachable!("parent must probe: {err}"),
    };
    assert_eq!(report.format.as_str(), "mp4");
    assert_eq!(report.confidence.as_u8(), 95);
    let recorded = recorded_track_codecs("expectations/valid/mp4_mjpeg_pcm.mp4.toml");
    let hints: Vec<&str> = report
        .track_hints
        .iter()
        .map(|h| h.codec.as_str())
        .collect();
    assert_eq!(
        hints,
        vec![recorded.video.as_str(), recorded.audio.as_str()]
    );
    assert_eq!(report.track_hints[0].kind, MediaKind::Video);
    assert_eq!(report.track_hints[1].kind, MediaKind::Audio);
    assert_eq!(report.bytes_examined, 65_536);
}

// ---------------------------------------------------------------------------
// The class-B matrix
// ---------------------------------------------------------------------------

#[test]
fn avi_class_b_derivatives_are_classed_per_their_nature() {
    // trunc0: 12 bytes — the RIFF magic and form exactly. Past the
    // magic, still AVI, magic-rung only (the card's "trunc-after-magic
    // → still AVI").
    let derivation = recorded_derivation("expectations/malformed/avi_mjpeg_pcm__trunc0.avi.toml");
    assert_eq!(derivation.op, "truncate");
    assert_eq!(derivation.cut_offset, Some(12), "cut right after the magic");
    assert_verdict(
        "generated/malformed/avi_mjpeg_pcm__trunc0.avi",
        &Expected::Hit {
            format: "avi",
            score: 70,
            hints: &[],
        },
    );

    // trunc1: 52 bytes — mid-avih; the avih chunk header is visible,
    // the strl pairs are not.
    let derivation = recorded_derivation("expectations/malformed/avi_mjpeg_pcm__trunc1.avi.toml");
    assert_eq!(derivation.cut_offset, Some(52));
    assert_verdict(
        "generated/malformed/avi_mjpeg_pcm__trunc1.avi",
        &Expected::Hit {
            format: "avi",
            score: 90,
            hints: &[],
        },
    );

    // trunc2/3/4: the cut lands at the movi LIST start / mid-payload /
    // last byte — the whole hdrl (avih + both strl) is inside the
    // window in all three.
    for name in ["trunc2", "trunc3", "trunc4"] {
        let relative = format!("generated/malformed/avi_mjpeg_pcm__{name}.avi");
        assert_verdict(
            &relative,
            &Expected::Hit {
                format: "avi",
                score: 95,
                hints: BOTH_HINTS,
            },
        );
    }

    // flip1: one advisory byte inside the avih payload (offset 58) —
    // the structure and both strl pairs are untouched.
    assert_verdict(
        "generated/malformed/avi_mjpeg_pcm__flip1.avi",
        &Expected::Hit {
            format: "avi",
            score: 95,
            hints: BOTH_HINTS,
        },
    );

    // flip4: the hdrl LIST size field and the RIFF size lie (plus two
    // advisory bytes) — the declared extent explodes past the window,
    // the walk clamps and still harvests both strl pairs.
    assert_verdict(
        "generated/malformed/avi_mjpeg_pcm__flip4.avi",
        &Expected::Hit {
            format: "avi",
            score: 95,
            hints: BOTH_HINTS,
        },
    );

    // flip16: the seeded flips include RIFF[2] ('F'→'G') and the AVI
    // form — the magic itself is broken, so no signature anchors: the
    // card's "magic-breaking flips → rejected".
    assert_verdict(
        "generated/malformed/avi_mjpeg_pcm__flip16.avi",
        &Expected::Unknown,
    );

    // garbage0: 4 KiB of seeded garbage APPENDED past the RIFF extent
    // (corpus-plan B1) — the header window is pristine, the garbage is
    // never reached. The pure-garbage → unknown axis is probe_io's
    // real-file test; this fixture's honest verdict is the parent's.
    assert_verdict(
        "generated/malformed/avi_mjpeg_pcm__garbage0.avi",
        &Expected::Hit {
            format: "avi",
            score: 95,
            hints: BOTH_HINTS,
        },
    );

    // noidx0: idx1 dropped, RIFF size patched — idx1 lives past hdrl
    // and the signature never reads it (idx1 absence is a demux-level
    // concern; the 2.B2 movi-walk fallback owns it there).
    assert_verdict(
        "generated/malformed/avi_mjpeg_pcm__noidx0.avi",
        &Expected::Hit {
            format: "avi",
            score: 95,
            hints: BOTH_HINTS,
        },
    );
}

#[test]
fn mp4_class_b_derivatives_are_classed_per_their_nature() {
    // trunc0: 28 bytes — the writer's complete ftyp (isom + version +
    // isom/iso2/mp41). Brand-rung only.
    let derivation = recorded_derivation("expectations/malformed/mp4_mjpeg_pcm__trunc0.mp4.toml");
    assert_eq!(derivation.op, "truncate");
    assert_eq!(derivation.cut_offset, Some(28), "cut right after ftyp");
    assert_verdict(
        "generated/malformed/mp4_mjpeg_pcm__trunc0.mp4",
        &Expected::Hit {
            format: "mp4",
            score: 70,
            hints: &[],
        },
    );

    // trunc1: 64 bytes — ftyp + the moov and mvhd box headers (the
    // cut lands 20 bytes into mvhd's payload).
    let derivation = recorded_derivation("expectations/malformed/mp4_mjpeg_pcm__trunc1.mp4.toml");
    assert_eq!(derivation.cut_offset, Some(64));
    assert_verdict(
        "generated/malformed/mp4_mjpeg_pcm__trunc1.mp4",
        &Expected::Hit {
            format: "mp4",
            score: 90,
            hints: &[],
        },
    );

    // trunc2/3/4: the cut lands 12 bytes past the moov extent (2208 =
    // moov end 2196 + mdat header + 4) / mid-payload / last byte —
    // the whole moov (mvhd + both traks) is inside the window.
    for name in ["trunc2", "trunc3", "trunc4"] {
        let relative = format!("generated/malformed/mp4_mjpeg_pcm__{name}.mp4");
        assert_verdict(
            &relative,
            &Expected::Hit {
                format: "mp4",
                score: 95,
                hints: BOTH_HINTS,
            },
        );
    }

    // flip1: the video trak's tkhd size field lies (0x5c → 0x40005c)
    // — the walk cannot reach that trak's mdia, so the video hint
    // degrades away honestly; the untouched audio trak still hints.
    assert_verdict(
        "generated/malformed/mp4_mjpeg_pcm__flip1.mp4",
        &Expected::Hit {
            format: "mp4",
            score: 95,
            hints: AUDIO_ONLY,
        },
    );

    // flip4: additionally the video hdlr type ('h'→'H') and the stsd
    // entry fourcc ('jpeg'→'jpmg') — same honest degradation.
    assert_verdict(
        "generated/malformed/mp4_mjpeg_pcm__flip4.mp4",
        &Expected::Hit {
            format: "mp4",
            score: 95,
            hints: AUDIO_ONLY,
        },
    );

    // flip16: the seeded flips include ftyp[6] ('y'→'9') — the ftyp
    // box TYPE itself is broken, so the signature does not anchor:
    // rejected, nothing named.
    assert_verdict(
        "generated/malformed/mp4_mjpeg_pcm__flip16.mp4",
        &Expected::Unknown,
    );

    // garbage0: seeded append past the pristine header — the parent's
    // verdict (see the AVI row for the recorded rationale).
    assert_verdict(
        "generated/malformed/mp4_mjpeg_pcm__garbage0.mp4",
        &Expected::Hit {
            format: "mp4",
            score: 95,
            hints: BOTH_HINTS,
        },
    );

    // stszlie0: the video stsz sample_count is 2^31−1 (offset 827) —
    // INSIDE the 64 KiB window but in a box the signature never
    // opens; the stsd-driven hints are unaffected.
    let derivation = recorded_derivation("expectations/malformed/mp4_mjpeg_pcm__stszlie0.mp4.toml");
    assert_eq!(derivation.op, "stsz-count-lie");
    assert_verdict(
        "generated/malformed/mp4_mjpeg_pcm__stszlie0.mp4",
        &Expected::Hit {
            format: "mp4",
            score: 95,
            hints: BOTH_HINTS,
        },
    );
}

// ---------------------------------------------------------------------------
// Depth-shaped classification: the same corpus at a small window
// ---------------------------------------------------------------------------

#[test]
fn a_small_window_degrades_the_verdicts_honestly() {
    // A 120-byte window reaches the AVI video strh's fccType/handler
    // (strl at 88, strh payload at 108..116) but not the audio strl
    // (at 212) — the video hint appears alone. The MP4 side at 120
    // bytes has confirmed moov+mvhd but not yet reached the first
    // mdia (at 244): rung 90, no hints. Window size is a first-class
    // classification input, not just an I/O bound.
    let config = match ProbeConfig::new(120, 60) {
        Ok(config) => config,
        Err(err) => unreachable!("{err}"),
    };
    let report = match probe_file(&fixture("generated/valid/avi_mjpeg_pcm.avi"), &config) {
        Ok(report) => report,
        Err(err) => unreachable!("{err}"),
    };
    assert_eq!(report.format.as_str(), "avi");
    assert_eq!(report.confidence.as_u8(), 95);
    assert_eq!(report.bytes_examined, 120);
    let hints: Vec<(MediaKind, &str)> = report
        .track_hints
        .iter()
        .map(|h| (h.kind, h.codec.as_str()))
        .collect();
    assert_eq!(hints, vec![(MediaKind::Video, "mjpeg")]);

    let report = match probe_file(&fixture("generated/valid/mp4_mjpeg_pcm.mp4"), &config) {
        Ok(report) => report,
        Err(err) => unreachable!("{err}"),
    };
    assert_eq!(report.format.as_str(), "mp4");
    assert_eq!(report.confidence.as_u8(), 90);
    assert_eq!(report.bytes_examined, 120);
    assert!(report.track_hints.is_empty());
}

#[test]
fn every_fixture_is_window_bounded_whatever_the_verdict() {
    // The depth bound is orthogonal to the verdict: every corpus file
    // examined under three different windows reports having read at
    // most that many bytes — including the rejected ones (the error
    // path reads the window too, doc 03 §4.2(d)'s sniffed payload).
    let files = [
        "generated/valid/avi_mjpeg_pcm.avi",
        "generated/valid/mp4_mjpeg_pcm.mp4",
        "generated/malformed/avi_mjpeg_pcm__flip16.avi",
        "generated/malformed/mp4_mjpeg_pcm__flip16.mp4",
        "generated/malformed/avi_mjpeg_pcm__garbage0.avi",
        "generated/malformed/mp4_mjpeg_pcm__stszlie0.mp4",
    ];
    for bound in [64usize, 1024, 65_536] {
        let config = match ProbeConfig::new(bound, 60) {
            Ok(config) => config,
            Err(err) => unreachable!("{err}"),
        };
        for relative in files {
            let path = fixture(relative);
            match probe_file(&path, &config) {
                Ok(report) => {
                    assert!(
                        report.bytes_examined <= bound,
                        "{relative} at {bound}: {}",
                        report.bytes_examined
                    );
                }
                Err(err) => {
                    assert_eq!(err.context.code, "probe.no-demuxer", "{relative}");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Static sanity of the AC vocabulary itself
// ---------------------------------------------------------------------------

#[test]
fn the_ladder_rungs_are_what_the_matrix_assumes() {
    // The matrix's 70/90/95 are the signature module's published rungs
    // (magic-anchor / primary-header / track-hints). If a future rung
    // changes, this fails and the matrix above must be re-derived —
    // deliberately brittle, per AGENTS.md rule 5.
    assert_eq!(
        media_probe::signature::ProbeRegistry::with_builtins().len(),
        10
    );
    assert!(Confidence::from_score(95).must().meets(60));
    assert!(!Confidence::from_score(50).must().meets(60));
}

#[test]
fn parents_report_shape_matches_the_card() {
    // ProbeReport {format id, confidence, track hints} (+ the window
    // observable): the card's own shape check, over the real stack.
    let report: ProbeReport = match probe_file(
        &fixture("generated/valid/avi_mjpeg_pcm.avi"),
        &default_config(),
    ) {
        Ok(report) => report,
        Err(err) => unreachable!("{err}"),
    };
    let _ = format!(
        "{} {} {:?}",
        report.format, report.confidence, report.track_hints
    );
    assert_eq!(report.format.to_string(), "avi");
    assert_eq!(report.confidence.to_string(), "95/100");
}
