//! Card 5.O2 test suite: the three handcrafted AC sheets with
//! hand-computed window truth, dirty-world tolerance (bad times, missing
//! INDEX 01, BOM/CRLF/case, structural damage), the D-1 line-cap pin, and
//! the two deterministic fuzz-lite harnesses (random bytes + mutations of
//! a valid sheet, all inputs < 200 KB). No I/O, bounded runtime.

use playlists::{
    CueFileKind, CueFlag, CueTrackType, CueWarning, ParseError, TimeContext, parse_cue,
    parse_cue_bytes,
};

// ------------------------------------------------------- AC-1: gapless
// Single FILE, three tracks, INDEX-00 pregaps between them. Every number
// below was hand-computed from MM:SS:FF at 75 fps (see each assert).
const AC_GAPLESS: &str = "\
REM GENRE \"Ambient\"
REM DATE 2021
PERFORMER \"Artist\"
TITLE \"Album\"
FILE \"album.flac\" FLAC
  TRACK 01 AUDIO
    TITLE \"One\"
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    TITLE \"Two\"
    INDEX 00 03:00:00
    INDEX 01 03:02:25
  TRACK 03 AUDIO
    TITLE \"Three\"
    INDEX 00 07:10:10
    INDEX 01 07:11:00
";

#[test]
fn ac1_single_file_gapless_windows() {
    let Ok(sheet) = parse_cue(AC_GAPLESS) else {
        unreachable!("AC-1 must parse clean");
    };
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    assert_eq!(sheet.files.len(), 1);
    assert_eq!(sheet.files[0].name, "album.flac");
    assert_eq!(sheet.files[0].kind, CueFileKind::Flac);
    assert_eq!(sheet.rem[0], ("GENRE".to_string(), "Ambient".into()));
    assert_eq!(sheet.rem[1], ("DATE".to_string(), "2021".into()));
    assert_eq!(sheet.performer.as_deref(), Some("Artist"));
    assert_eq!(sheet.title.as_deref(), Some("Album"));
    assert_eq!(
        sheet
            .tracks
            .iter()
            .map(|t| t.title.as_deref())
            .collect::<Vec<_>>(),
        [Some("One"), Some("Two"), Some("Three")]
    );

    let w = sheet.track_windows();
    assert_eq!(w.len(), 3);
    // Hand truth (frames -> ns, round-to-nearest at 1/75 s):
    //   T1 INDEX 01 = 0;            T2 INDEX 01 = 13,675 fr = 182.333..s
    //   T2 pregap   = 175 fr;       T3 INDEX 01 = 32,325 fr = 431 s
    //   T3 pregap   = 65 fr.
    assert_eq!(w[0].start_ns, 0);
    assert_eq!(w[0].duration_ns, Some(182_333_333_333));
    assert_eq!(w[0].pregap_ns, None, "T1 has no pregap");
    assert!(!w[0].gapless_join, "first track cannot join");

    assert_eq!(w[1].start_ns, 182_333_333_333);
    assert_eq!(w[1].duration_ns, Some(248_666_666_667)); // 32,325-13,675 fr
    assert_eq!(w[1].pregap_ns, Some(2_333_333_333)); // 175 fr, in-file
    assert!(!w[1].pregap_synthetic);
    assert!(w[1].gapless_join, "same FILE, monotonic: zero-gap join");

    assert_eq!(w[2].start_ns, 431_000_000_000); // 32,325 fr = 431 s
    assert_eq!(
        w[2].duration_ns, None,
        "last track of the FILE is open-ended"
    );
    assert_eq!(w[2].pregap_ns, Some(866_666_667)); // 65 fr
    assert!(w[2].gapless_join);

    // The gapless handoff, pinned exactly: window N ends where window
    // N+1 begins — no gap, no overlap, pregap excluded from every start.
    for pair in w.windows(2) {
        let [a, b] = pair else {
            unreachable!("slice::windows(2) yields exactly 2 elements");
        };
        if let Some(d) = a.duration_ns {
            assert_eq!(a.start_ns + d, b.start_ns, "zero-gap join broken");
        }
        if let Some(p) = b.pregap_ns {
            assert!(
                !b.pregap_synthetic,
                "AC-1 pregaps are all in-file INDEX arithmetic"
            );
            assert!(p > 0, "hand-computed pregap must be positive");
        }
    }
    // Every window addresses the same FILE reference.
    assert!(w.iter().all(|x| x.file == 0));
    assert_eq!(
        sheet.file_of(0).map(|f| f.name.as_str()),
        Some("album.flac")
    );
}

// ------------------------------------------------------ AC-2: multi-FILE
const AC_MULTI_FILE: &str = "\
CATALOG 0123456789012
CDTEXTFILE \"cdtext.bin\"
TITLE \"Greatest Hits\"
PERFORMER \"Band\"
FILE \"01.wav\" WAVE
  TRACK 01 AUDIO
    ISRC ABCDE12000001
    FLAGS DCP
    INDEX 01 00:00:00
FILE \"02.wav\" WAVE
  TRACK 02 AUDIO
    INDEX 01 00:00:33
FILE \"03.mp3\" MP3
  TRACK 03 AUDIO
    INDEX 01 00:01:15
";

#[test]
fn ac2_multi_file_windows_open_ended() {
    let Ok(sheet) = parse_cue(AC_MULTI_FILE) else {
        unreachable!("AC-2 must parse clean");
    };
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    assert_eq!(sheet.catalog.as_deref(), Some("0123456789012"));
    assert_eq!(sheet.cdtextfile.as_deref(), Some("cdtext.bin"));
    assert_eq!(
        sheet
            .files
            .iter()
            .map(|f| (f.name.as_str(), f.kind.clone()))
            .collect::<Vec<_>>(),
        [
            ("01.wav", CueFileKind::Wave),
            ("02.wav", CueFileKind::Wave),
            ("03.mp3", CueFileKind::Mp3),
        ]
    );
    assert_eq!(sheet.tracks[0].isrc.as_deref(), Some("ABCDE12000001"));
    assert_eq!(sheet.tracks[0].flags, [CueFlag::Dcp]);

    let w = sheet.track_windows();
    assert_eq!(w.len(), 3);
    // Hand truth: 00:00:33 = 33 fr = 0.44 s; 00:01:15 = 90 fr = 1.2 s.
    assert_eq!(w[0].file, 0);
    assert_eq!(w[0].start_ns, 0);
    assert_eq!(w[1].file, 1);
    assert_eq!(w[1].start_ns, 440_000_000);
    assert_eq!(w[2].file, 2);
    assert_eq!(w[2].start_ns, 1_200_000_000);
    // Each track is the last of its own FILE: durations stay unknown and
    // no join is claimed across file boundaries.
    assert!(w.iter().all(|x| x.duration_ns.is_none()));
    assert!(w.iter().all(|x| !x.gapless_join));
    assert!(w.iter().all(|x| x.pregap_ns.is_none()));
}

// --------------------------------------------- AC-3: pregaps + postgaps
const AC_GAPS: &str = "\
FILE \"live.wav\" WAVE
  TRACK 01 AUDIO
    TITLE \"Crowd\"
    INDEX 00 00:00:00
    INDEX 01 00:00:33
    POSTGAP 00:02:00
  TRACK 02 AUDIO
    TITLE \"Show\"
    PREGAP 00:05:12
    INDEX 01 00:10:00
  TRACK 03 AUDIO
    TITLE \"Encore\"
    INDEX 00 00:12:00
    INDEX 01 00:12:50
    POSTGAP 00:00:60
";

#[test]
fn ac3_pregap_postgap_arithmetic() {
    let Ok(sheet) = parse_cue(AC_GAPS) else {
        unreachable!("AC-3 must parse clean");
    };
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let w = sheet.track_windows();
    assert_eq!(w.len(), 3);

    // T1: in-file pregap 0..33 fr (0.44 s), window [33, 750) fr.
    assert_eq!(w[0].start_ns, 440_000_000); // 33 fr
    assert_eq!(w[0].pregap_ns, Some(440_000_000));
    assert!(
        !w[0].pregap_synthetic,
        "INDEX 00 arithmetic, not PREGAP cmd"
    );
    assert_eq!(w[0].duration_ns, Some(9_560_000_000)); // 750-33 = 717 fr
    assert_eq!(w[0].postgap_ns, Some(2_000_000_000)); // 00:02:00 = 150 fr = 2 s
    assert!(!w[0].gapless_join);

    // T2: synthetic PREGAP 387 fr = 5.16 s (silence not in the file).
    assert_eq!(w[1].start_ns, 10_000_000_000); // 750 fr = 10 s
    assert_eq!(w[1].pregap_ns, Some(5_160_000_000));
    assert!(w[1].pregap_synthetic, "PREGAP command, no INDEX 00");
    assert_eq!(w[1].postgap_ns, None);
    assert_eq!(w[1].duration_ns, Some(2_666_666_667)); // 950-750 = 200 fr
    assert!(w[1].gapless_join);

    // T3: in-file pregap 900..950 fr = 50 fr, plus POSTGAP.
    assert_eq!(w[2].start_ns, 12_666_666_667); // 950 fr
    assert_eq!(w[2].pregap_ns, Some(666_666_667)); // 50 fr
    assert!(!w[2].pregap_synthetic);
    assert_eq!(w[2].postgap_ns, Some(800_000_000)); // 60 fr
    assert_eq!(w[2].duration_ns, None);
    assert!(w[2].gapless_join);

    // Gapless chain across all three (durations exclude every pregap).
    let e1 = w[0].start_ns + w[0].duration_ns.unwrap_or(0);
    let e2 = w[1].start_ns + w[1].duration_ns.unwrap_or(0);
    assert_eq!(e1, w[1].start_ns);
    assert_eq!(e2, w[2].start_ns);
    // The pregap region ends exactly at the window start (INDEX 00 at
    // 900 frames = 12 s exactly).
    assert_eq!(w[2].start_ns - w[2].pregap_ns.unwrap_or(0), 12_000_000_000);
}

// ---------------------------------------------------------- dirty world

#[test]
fn bad_times_are_typed_and_tracks_degrade() {
    let src = "\
FILE \"x.wav\" WAVE
  TRACK 01 AUDIO
    INDEX 01 100:00:00
  TRACK 02 AUDIO
    INDEX 01 00:60:00
  TRACK 03 AUDIO
    INDEX 01 00:00:75
  TRACK 04 AUDIO
    INDEX 01 00:00:00
";
    let Ok(sheet) = parse_cue(src) else {
        unreachable!("bad times must not be fatal");
    };
    // Each bad INDEX 01 drops its line (BadTime) and its track closes
    // without a start point (MissingIndex01 damage-note + skip).
    assert_eq!(
        sheet.warnings,
        vec![
            CueWarning::BadTime {
                line: 3,
                context: TimeContext::Index
            },
            CueWarning::MissingIndex01 { line: 2 },
            CueWarning::BadTime {
                line: 5,
                context: TimeContext::Index
            },
            CueWarning::MissingIndex01 { line: 4 },
            CueWarning::BadTime {
                line: 7,
                context: TimeContext::Index
            },
            CueWarning::MissingIndex01 { line: 6 },
        ]
    );
    assert_eq!(sheet.tracks.len(), 1, "only TRACK 04 survives");
    assert_eq!(sheet.windows.len(), 1);
    assert_eq!(sheet.windows[0].number, 4);
    assert_eq!(sheet.windows[0].start_ns, 0);
}

#[test]
fn more_bad_time_shapes() {
    for bad in ["abc", "00:00", "1:2:3:4", "-1:00:00", "00:00:"] {
        let src = format!("FILE \"x\" WAVE\nTRACK 01 AUDIO\nINDEX 01 {bad}\n");
        let Ok(sheet) = parse_cue(&src) else {
            unreachable!("bad time {bad:?} must not be fatal");
        };
        assert_eq!(
            sheet.warnings[0],
            CueWarning::BadTime {
                line: 3,
                context: TimeContext::Index
            },
            "time {bad:?}"
        );
        assert!(sheet.tracks.is_empty());
    }
    // Boundary values are legal: 99:59:74 is the maximum time.
    let Ok(ok) = parse_cue("FILE \"x\" WAVE\nTRACK 01 AUDIO\nINDEX 01 99:59:74\n") else {
        unreachable!("99:59:74 is in range");
    };
    assert!(ok.warnings.is_empty());
    // 449,999 frames = 5999.986..s -> 5,999,986,666,667 ns (rounded).
    assert_eq!(ok.windows[0].start_ns, 5_999_986_666_667);
}

#[test]
fn pregap_postgap_bad_times_and_contexts() {
    let src = "\
FILE \"x.wav\" WAVE
  TRACK 01 AUDIO
    PREGAP 00:61:00
    POSTGAP 12:00:00
    INDEX 01 00:00:00
";
    let Ok(sheet) = parse_cue(src) else {
        unreachable!("bad PREGAP must not be fatal");
    };
    assert_eq!(
        sheet.warnings,
        vec![CueWarning::BadTime {
            line: 3,
            context: TimeContext::Pregap
        }]
    );
    assert_eq!(sheet.tracks[0].pregap_frames, None, "bad PREGAP dropped");
    assert_eq!(sheet.tracks[0].postgap_frames, Some(54_000)); // 12 min in frames
    assert_eq!(sheet.windows[0].postgap_ns, Some(720_000_000_000));
    assert_eq!(sheet.windows[0].pregap_ns, None);

    let src2 = "FILE \"x\" WAVE\nTRACK 01 AUDIO\nPOSTGAP 00:00:75\nINDEX 01 00:00:00\n";
    let Ok(s2) = parse_cue(src2) else {
        unreachable!("bad POSTGAP must not be fatal");
    };
    assert_eq!(
        s2.warnings,
        vec![CueWarning::BadTime {
            line: 3,
            context: TimeContext::Postgap
        }]
    );
}

#[test]
fn bom_crlf_lowercase_and_indentation() {
    let src =
        "\u{FEFF}file \"MiXed.wav\" wave\r\n  track 01 audio\r\n    index 01 00:01:00\r\n\r\n";
    let Ok(sheet) = parse_cue(src) else {
        unreachable!("BOM + CRLF + lowercase keywords must parse");
    };
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    assert_eq!(sheet.files[0].name, "MiXed.wav");
    assert_eq!(sheet.files[0].kind, CueFileKind::Wave);
    assert_eq!(sheet.tracks[0].track_type, CueTrackType::Audio);
    assert_eq!(sheet.windows[0].start_ns, 1_000_000_000); // 75 fr = 1 s
}

#[test]
fn track_without_file_and_outside_track_commands() {
    let src = "\
  TRACK 01 AUDIO
    INDEX 01 00:00:00
FLAGS DCP
ISRC XX1234567890
PREGAP 00:00:10
";
    let Ok(sheet) = parse_cue(src) else {
        unreachable!("structural damage must not be fatal");
    };
    assert_eq!(
        sheet.warnings,
        vec![
            CueWarning::TrackWithoutFile { line: 1 },
            CueWarning::CommandOutsideTrack { line: 2 },
            CueWarning::CommandOutsideTrack { line: 3 },
            CueWarning::CommandOutsideTrack { line: 4 },
            CueWarning::CommandOutsideTrack { line: 5 },
        ]
    );
    assert!(sheet.tracks.is_empty());
    assert!(sheet.windows.is_empty());
}

#[test]
fn malformed_numbers_and_missing_arguments() {
    let src = "\
FILE
TRACK XX AUDIO
FILE \"a.wav\" WAVE
TRACK 100 AUDIO
TRACK 05
INDEX
INDEX abc 00:00:00
INDEX 200 00:00:00
TITLE
REM
TRACK 05 AUDIO
INDEX 01 00:00:00
";
    let Ok(sheet) = parse_cue(src) else {
        unreachable!("dirty sheet must still parse");
    };
    assert_eq!(
        sheet.warnings,
        vec![
            CueWarning::MissingArgument { line: 1 },
            CueWarning::BadTrackNumber { line: 2 },
            CueWarning::BadTrackNumber { line: 4 },
            CueWarning::TrackWithoutType { line: 5 },
            CueWarning::MissingArgument { line: 6 },
            CueWarning::BadIndexNumber { line: 7 },
            CueWarning::BadIndexNumber { line: 8 },
            CueWarning::MissingArgument { line: 9 },
            CueWarning::MissingArgument { line: 10 },
            // TRACK 05 (line 5) closed at line 11 with no valid INDEX.
            CueWarning::MissingIndex01 { line: 5 },
        ]
    );
    // The surviving track is line 11's TRACK 05 AUDIO (line 5's copy
    // was skipped for lack of a valid INDEX).
    assert_eq!(sheet.tracks.len(), 1);
    assert_eq!(sheet.tracks[0].track_type, CueTrackType::Audio);
    assert_eq!(sheet.tracks[0].number, 5);
}

#[test]
fn unknown_command_and_flags() {
    let src = "\
FOO BAR
FILE \"a.wav\" WAVE
TRACK 01 AUDIO
FLAGS DCP PRE XYZ 4CH SCMS
INDEX 01 00:00:00
";
    let Ok(sheet) = parse_cue(src) else {
        unreachable!("unknown command must not be fatal");
    };
    assert_eq!(
        sheet.warnings,
        vec![
            CueWarning::UnknownCommand { line: 1 },
            CueWarning::UnknownFlag { line: 4 },
        ]
    );
    assert_eq!(
        sheet.tracks[0].flags,
        [CueFlag::Dcp, CueFlag::Pre, CueFlag::FourCh, CueFlag::Scms]
    );
}

#[test]
fn non_monotonic_index_empties_window() {
    let src = "\
FILE \"a.wav\" WAVE
  TRACK 01 AUDIO
    INDEX 01 00:02:00
  TRACK 02 AUDIO
    INDEX 01 00:01:00
";
    let Ok(sheet) = parse_cue(src) else {
        unreachable!("backwards INDEX must not be fatal");
    };
    assert_eq!(
        sheet.warnings,
        vec![CueWarning::NonMonotonicIndex { track: 2 }]
    );
    let w = sheet.track_windows();
    assert_eq!(w[0].duration_ns, Some(0), "window emptied, not negative");
    assert!(!w[1].gapless_join, "T2's start is before T1's");
    assert_eq!(w[1].duration_ns, None);
}

#[test]
fn bad_pregap_arithmetic_is_typed() {
    // INDEX 00 after INDEX 01: pregap unusable.
    let src = "\
FILE \"a.wav\" WAVE
  TRACK 01 AUDIO
    INDEX 00 00:01:00
    INDEX 01 00:00:50
";
    let Ok(sheet) = parse_cue(src) else {
        unreachable!("bad pregap must not be fatal");
    };
    assert_eq!(sheet.warnings, vec![CueWarning::BadPregap { track: 1 }]);
    assert_eq!(sheet.windows[0].pregap_ns, None);
    assert_eq!(sheet.windows[0].start_ns, playlists::frames_to_ns(50));
}

#[test]
fn quoted_unquoted_and_verbatim_values() {
    let src = "\
TITLE Unquoted Album
SONGWRITER \"Smith, John\"
FILE my music file.wav WAVE
  TRACK 01 AUDIO
    TITLE \"Track \\\"One\\\"\"
    REM COMMENT \"ExactAudioCopy v1.6\"
    INDEX 01 00:00:00
";
    let Ok(sheet) = parse_cue(src) else {
        unreachable!("quoted/unquoted mix must parse");
    };
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    assert_eq!(sheet.title.as_deref(), Some("Unquoted Album"));
    assert_eq!(sheet.songwriter.as_deref(), Some("Smith, John"));
    // Unquoted FILE: last token is the type, the rest is the name.
    assert_eq!(sheet.files[0].name, "my music file.wav");
    assert_eq!(sheet.files[0].kind, CueFileKind::Wave);
    // Embedded escaped quotes are kept verbatim (CUE has no escapes).
    assert_eq!(sheet.tracks[0].title.as_deref(), Some(r#"Track \"One\""#));
    assert_eq!(
        sheet.tracks[0].rem[0],
        ("COMMENT".to_string(), "ExactAudioCopy v1.6".into())
    );
}

#[test]
fn empty_and_minimal_inputs() {
    for src in ["", "\n\n\r\n", "   \t  \n", "\u{FEFF}"] {
        let Ok(sheet) = parse_cue(src) else {
            unreachable!("empty-ish input must parse");
        };
        assert!(sheet.tracks.is_empty());
        assert!(
            sheet.warnings.is_empty(),
            "{:?} for {src:?}",
            sheet.warnings
        );
    }
    // A FILE with no tracks parses: files are exposed even when unused.
    let Ok(solo) = parse_cue("FILE \"lone.wav\" WAVE\n") else {
        unreachable!("file-only sheet must parse");
    };
    assert_eq!(solo.files.len(), 1);
    assert!(solo.tracks.is_empty());
    assert!(solo.windows.is_empty());
}

#[test]
fn garbage_bytes_are_typed_invalid_utf8() {
    let garbage = [b'F', b'I', b'L', b'E', 0xFF, 0xFE, 0x00, 0x80];
    match parse_cue_bytes(&garbage) {
        Err(ParseError::InvalidUtf8) => {}
        other => unreachable!("expected InvalidUtf8, got {other:?}"),
    }
}

#[test]
fn line_cap_is_fatal_and_redacted() {
    // "REM " (4 bytes) + MAX-3 x's = exactly MAX+1 bytes on line 1.
    let huge = format!("REM {}", "x".repeat(playlists::MAX_LINE_BYTES - 3));
    match parse_cue(&huge) {
        Err(e @ ParseError::LineTooLong { line: 1, .. }) => {
            assert_eq!(
                e.to_string(),
                "playlists: line 1 is 65537 bytes, over the 65536-byte line cap (D-1 bounded input)"
            );
        }
        other => unreachable!("expected LineTooLong, got {other:?}"),
    }
    // Exactly at the cap: accepted.
    let at_cap = format!("REM {}\n", "y".repeat(playlists::MAX_LINE_BYTES - 4));
    assert!(parse_cue(&at_cap).is_ok());
}

#[test]
fn lone_cr_is_one_weird_line() {
    let Ok(sheet) = parse_cue("INDEX\rbad") else {
        unreachable!("CR-only input must not panic");
    };
    // One weird line by design (spec terminators are LF/CRLF): the whole
    // input is a single INDEX line whose number token is garbage.
    assert_eq!(sheet.warnings, vec![CueWarning::BadIndexNumber { line: 1 }]);
}

// ------------------------------------------------ scale (still < 200 KB)

#[test]
fn large_valid_sheet_under_200kb() {
    let mut src = String::from("FILE \"big.flac\" FLAC\n");
    for n in 1..=4600u32 {
        let t = n % 99 + 1;
        src.push_str(&format!("  TRACK {t:02} AUDIO\n    INDEX 01 00:00:00\n"));
    }
    assert!(
        src.len() < 200 * 1024,
        "stress input must stay under 200 KB"
    );
    let Ok(sheet) = parse_cue(&src) else {
        unreachable!("large valid sheet must parse");
    };
    assert!(sheet.warnings.is_empty());
    assert_eq!(sheet.tracks.len(), 4600);
    assert_eq!(sheet.windows.len(), 4600);
    // Equal starts: every window but the last is empty-but-joined
    // (monotonic-equal); the last track of the FILE stays open-ended.
    assert!(
        sheet.windows[..4599]
            .iter()
            .all(|w| w.duration_ns == Some(0))
    );
    assert_eq!(sheet.windows[4599].duration_ns, None);
    assert!(sheet.windows.iter().skip(1).all(|w| w.gapless_join));
}

// -------------------------------------------------------------- fuzz-lite

/// Deterministic splitmix64 — fixed seeds, reproducible failures, no deps.
struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    fn byte(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
}

/// Bytes that steer generation toward CUE branch coverage.
const INTERESTING: &[u8] = b"\" \t\r\n0123456789:IFTXREMADGPNELCOSWBU/79.,-x\xef\xbb\xbf\xff\xfe";

fn random_input(rng: &mut SplitMix64) -> Vec<u8> {
    // Mostly short (high UTF-8 validity); rare cases up to ~190 KB so the
    // whole < 200 KB budget is exercised without exploding runtime.
    let len = if rng.below(256) == 0 {
        1 + rng.below(190_000) as usize
    } else if rng.below(8) == 0 {
        1 + rng.below(2048) as usize
    } else {
        1 + rng.below(64) as usize
    };
    let mut buf = Vec::with_capacity(len);
    for _ in 0..len {
        let roll = rng.below(100);
        if roll < 65 {
            buf.push(0x20 + (rng.below(0x5F)) as u8);
        } else if roll < 90 {
            buf.push(INTERESTING[(rng.below(INTERESTING.len() as u64)) as usize]);
        } else {
            buf.push(rng.byte());
        }
    }
    buf
}

/// Structural invariants that must hold for every successful parse.
fn assert_cue_invariants(sheet: &playlists::CueSheet) {
    assert_eq!(sheet.windows.len(), sheet.tracks.len());
    for (i, w) in sheet.windows.iter().enumerate() {
        assert_eq!(w.track, i, "windows[i].track == i");
        assert!(w.file < sheet.files.len(), "file reference in range");
        if w.pregap_synthetic {
            assert!(w.pregap_ns.is_some(), "synthetic implies a pregap value");
        }
        if let Some(t) = sheet.tracks.get(i) {
            assert_eq!(t.number, w.number);
            assert_eq!(t.file, w.file);
        }
    }
}

#[test]
fn fuzz_lite_random_bytes_10k_no_panic() {
    let mut rng = SplitMix64::new(0x5E02_C0DE_5EED_0001); // "5.O2" seed
    let mut ok_count = 0usize;
    for _ in 0..10_000 {
        let buf = random_input(&mut rng);
        if let Ok(sheet) = parse_cue_bytes(&buf) {
            ok_count += 1;
            assert_cue_invariants(&sheet);
        }
        // Err results (UTF-8 / line cap) are fine: typed, no panic.
    }
    assert!(ok_count > 100, "fuzz too weak: only {ok_count} parses");
}

#[test]
fn fuzz_lite_mutations_of_valid_sheet_5k() {
    let base = AC_GAPS.as_bytes();
    let mut rng = SplitMix64::new(0x5E02_C0DE_5EED_0002);
    let mut ok_count = 0usize;
    for _ in 0..5_000 {
        let mut buf = base.to_vec();
        let mutations = 1 + rng.below(10);
        for _ in 0..mutations {
            if buf.is_empty() {
                break;
            }
            match rng.below(3) {
                0 => {
                    let i = rng.below(buf.len() as u64) as usize;
                    buf[i] = if rng.below(2) == 0 {
                        INTERESTING[(rng.below(INTERESTING.len() as u64)) as usize]
                    } else {
                        rng.byte()
                    };
                }
                1 => {
                    let i = rng.below(buf.len() as u64) as usize;
                    buf.remove(i);
                }
                _ => {
                    let i = rng.below(buf.len() as u64 + 1) as usize;
                    buf.insert(i, rng.byte());
                }
            }
        }
        if let Ok(sheet) = parse_cue_bytes(&buf) {
            ok_count += 1;
            assert_cue_invariants(&sheet);
        }
    }
    assert!(
        ok_count > 100,
        "mutation fuzz too weak: only {ok_count} parses"
    );
}
