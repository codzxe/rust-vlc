//! Card 4.D1 test suite: golden fixtures, roundtrip pins, dirty-world
//! corpus, D-1 line-cap boundaries, redacted error Display, and the two
//! deterministic fuzz-lite harnesses (random bytes + mutations of a valid
//! playlist). CI-able: no I/O beyond compile-time `include_str!`, bounded
//! runtime (the 10 MB case is the heaviest and completes in milliseconds).

use playlists::{
    Entry, HlsSegment, MAX_LINE_BYTES, ParseError, ParseWarning, Playlist, parse, parse_bytes,
    parse_detailed, write,
};

const BASIC: &str = include_str!("fixtures/basic_extended.m3u");
const PLAIN: &str = include_str!("fixtures/plain_urls.m3u");
const DIRECTIVES: &str = include_str!("fixtures/directives.m3u");
const COMMENTS: &str = include_str!("fixtures/comments_blanks.m3u");
const DIRTY: &str = include_str!("fixtures/dirty.m3u");

/// Golden-file roundtrip law: parse → write → parse is stable, for every
/// input that parses (including dirty ones).
fn assert_roundtrip(src: &str) {
    let Ok(first) = parse_detailed(src) else {
        unreachable!("input must parse for a roundtrip pin: {src:?}");
    };
    let written = write(&first.playlist);
    let Ok(second) = parse_detailed(&written) else {
        unreachable!("canonical write must re-parse: {written:?}");
    };
    assert_eq!(
        second.playlist, first.playlist,
        "roundtrip stability violated for {src:?}"
    );
}

// ---------------------------------------------------------------- golden

#[test]
fn golden_basic_extended() {
    let Ok(p) = parse(BASIC) else {
        unreachable!("basic fixture must parse");
    };
    assert_eq!(p.len(), 5, "five location lines");
    assert_eq!(
        p.entries[0],
        Entry {
            location: "sample.mp3".into(),
            duration: Some(123.0),
            title: Some("Sample Artist - Sample Title".into()),
            directives: vec![],
            hls: HlsSegment::default(),
        }
    );
    // Windows path kept verbatim (separators un-normalized).
    assert_eq!(p.entries[1].location, r"C:\Music\Example.flac");
    assert_eq!(p.entries[2].duration, Some(10.0));
    // Relative path kept relative.
    assert_eq!(p.entries[3].location, "../relative/track01.ogg");
    // The -1 "unknown" marker is preserved verbatim, not re-interpreted.
    assert_eq!(p.entries[4].duration, Some(-1.0));
    let Ok(d) = parse_detailed(BASIC) else {
        unreachable!("basic fixture must parse clean");
    };
    assert!(d.warnings.is_empty(), "clean fixture: {:?}", d.warnings);
}

#[test]
fn golden_plain_urls_without_header() {
    let Ok(p) = parse(PLAIN) else {
        unreachable!("plain fixture must parse");
    };
    assert_eq!(p.len(), 4);
    for e in &p.entries {
        assert_eq!(e.duration, None);
        assert_eq!(e.title, None);
        assert!(e.directives.is_empty());
    }
    assert_eq!(p.entries[3].location, r"..\sibling\song3.wav");
}

#[test]
fn golden_directives_attach_in_order() {
    let Ok(d) = parse_detailed(DIRECTIVES) else {
        unreachable!("directives fixture must parse");
    };
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    assert_eq!(d.playlist.len(), 2);
    assert_eq!(
        d.playlist.entries[0].directives,
        vec![
            ("EXTVLCOPT".to_string(), "http-user-agent=Lumen/1.0".into()),
            ("EXTGRP".to_string(), "Favourites".into()),
            (
                "EXTCUSTOM-UNKNOWN".to_string(),
                "some value: with colons".into()
            ),
        ]
    );
    // Title containing a colon survives (split is at the FIRST comma only).
    assert_eq!(
        d.playlist.entries[0].title.as_deref(),
        Some("Title with: colon")
    );
    assert_eq!(d.playlist.entries[0].duration, Some(42.0));
    // `#EXTINF:7.25,` — trailing comma with empty title normalizes to None.
    assert_eq!(d.playlist.entries[1].title, None);
    assert_eq!(d.playlist.entries[1].duration, Some(7.25));
    assert_eq!(
        d.playlist.entries[1].directives,
        vec![(
            "EXTVLCOPT".to_string(),
            "http-referrer=http://example.com/".into()
        )]
    );
}

#[test]
fn golden_comments_and_blanks_ignored() {
    let Ok(d) = parse_detailed(COMMENTS) else {
        unreachable!("comments fixture must parse");
    };
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    assert_eq!(d.playlist.len(), 2);
    assert_eq!(d.playlist.entries[0].location, "a/b.wav");
    // Location with an internal space is kept verbatim.
    assert_eq!(d.playlist.entries[1].location, "c d.wav");
}

#[test]
fn bom_and_crlf_tolerance() {
    let src = "\u{FEFF}#EXTM3U\r\n#EXTINF:5.5,Title\r\npath/one.wav\r\n\r\n#EXTINF:4,\r\npath two.wav\r\n";
    let Ok(d) = parse_detailed(src) else {
        unreachable!("BOM+CRLF source must parse");
    };
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    assert_eq!(d.playlist.len(), 2);
    assert_eq!(
        d.playlist.entries[0],
        Entry {
            location: "path/one.wav".into(),
            duration: Some(5.5),
            title: Some("Title".into()),
            directives: vec![],
            hls: HlsSegment::default(),
        }
    );
    assert_eq!(d.playlist.entries[1].duration, Some(4.0));
    assert_eq!(d.playlist.entries[1].title, None);
    assert_roundtrip(src);
}

#[test]
fn lf_crlf_mix() {
    let src = "#EXTM3U\n#EXTINF:1,A\r\none.wav\n#EXTINF:2,B\ntwo.wav\r\n";
    let Ok(p) = parse(src) else {
        unreachable!("mixed terminators must parse");
    };
    assert_eq!(p.len(), 2);
    assert_eq!(p.entries[0].title.as_deref(), Some("A"));
    assert_eq!(p.entries[1].title.as_deref(), Some("B"));
}

#[test]
fn repeated_header_anywhere_is_ignored() {
    let src = "#EXTM3U\n#EXTM3U\nurl\n#EXTM3U\nurl2\n";
    let Ok(d) = parse_detailed(src) else {
        unreachable!("repeated headers must parse");
    };
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    assert_eq!(d.playlist.len(), 2);
}

// ------------------------------------------------------------ dirty world

#[test]
fn dirty_fixture_degrades_honestly() {
    let Ok(d) = parse_detailed(DIRTY) else {
        unreachable!("dirty fixture must still parse (entry-not-death)");
    };
    // #EXTM (truncated header), #EXTINF w/o colon, bad duration, dangling.
    assert_eq!(
        d.warnings,
        vec![
            ParseWarning::DirectiveMissingColon { line: 1 },
            ParseWarning::ExtinfMissingColon { line: 2 },
            ParseWarning::BadDuration { line: 3 },
            ParseWarning::DanglingEntry { line: 7 },
        ]
    );
    assert_eq!(d.playlist.len(), 2, "entries survive the damage");
    // Bad duration dropped, title kept.
    assert_eq!(d.playlist.entries[0].duration, None);
    assert_eq!(d.playlist.entries[0].title.as_deref(), Some("Bad Duration"));
    assert_eq!(d.playlist.entries[0].location, "still-plays.mp3");
    assert_eq!(d.playlist.entries[1].duration, Some(7.0));
    assert_eq!(d.playlist.entries[1].title, None);
    assert_roundtrip(DIRTY);
}

#[test]
fn non_finite_durations_are_bad_duration() {
    for bad in ["inf", "NaN", "1e999", ""] {
        let src = format!("#EXTM3U\n#EXTINF:{bad},T\nu.mp3\n");
        let Ok(d) = parse_detailed(&src) else {
            unreachable!("non-finite duration must not be fatal");
        };
        assert_eq!(
            d.warnings,
            vec![ParseWarning::BadDuration { line: 2 }],
            "duration {bad:?}"
        );
        assert_eq!(d.playlist.entries[0].duration, None);
        assert_eq!(d.playlist.entries[0].title.as_deref(), Some("T"));
        assert_roundtrip(&src);
    }
}

#[test]
fn empty_and_comment_only_inputs() {
    let Ok(empty) = parse_detailed("") else {
        unreachable!("empty file must parse");
    };
    assert!(empty.playlist.is_empty());
    assert!(empty.warnings.is_empty());

    let Ok(newlines) = parse_detailed("\n\n\r\n\n") else {
        unreachable!("blank-only file must parse");
    };
    assert!(newlines.playlist.is_empty());
    assert!(newlines.warnings.is_empty());

    let Ok(comments) = parse_detailed("# just\n# comments\n") else {
        unreachable!("comment-only file must parse");
    };
    assert!(comments.playlist.is_empty());
    assert!(comments.warnings.is_empty());

    let Ok(header) = parse_detailed("#EXTM3U\n") else {
        unreachable!("header-only file must parse");
    };
    assert!(header.playlist.is_empty());
    assert!(header.warnings.is_empty());
}

#[test]
fn garbage_bytes_are_typed_invalid_utf8() {
    let garbage = [0x23, 0x45, 0x58, 0x54, 0xFF, 0xFE, 0x00, 0x80];
    match parse_bytes(&garbage) {
        Err(ParseError::InvalidUtf8) => {}
        other => unreachable!("expected InvalidUtf8, got {other:?}"),
    }
    // BOM prefix then invalid UTF-8: still typed, still no panic.
    let bom_garbage = [0xEF, 0xBB, 0xBF, b'#', 0xC3, 0x28];
    assert!(matches!(
        parse_bytes(&bom_garbage),
        Err(ParseError::InvalidUtf8)
    ));
}

#[test]
fn lone_cr_file_does_not_panic() {
    // CR-only terminators are one weird line by design (spec: CRLF/LF);
    // pinned as no-panic + single entry.
    let Ok(d) = parse_detailed("a\rb\rc") else {
        unreachable!("CR-only input must parse");
    };
    assert_eq!(d.playlist.len(), 1);
    assert_eq!(d.playlist.entries[0].location, "a\rb\rc");
}

// ------------------------------------------------- D-1: bounded line input

#[test]
fn ten_megabyte_line_is_a_typed_error() {
    let huge = "a".repeat(10 * 1024 * 1024);
    match parse(&huge) {
        Err(
            e @ ParseError::LineTooLong {
                line: 1,
                length: 10_485_760,
            },
        ) => {
            // Redaction pin: exact message — line number and byte counts
            // only, no line content or paths (MP §12).
            assert_eq!(
                e.to_string(),
                "playlists: line 1 is 10485760 bytes, over the 65536-byte line cap (D-1 bounded input)"
            );
        }
        other => unreachable!("10 MB line must be LineTooLong, got {other:?}"),
    }
}

#[test]
fn line_cap_boundaries() {
    // Exactly at the cap: accepted.
    let at_cap = format!("#EXTINF:1,T\n{}\n", "b".repeat(MAX_LINE_BYTES));
    assert!(parse(&at_cap).is_ok());

    // One byte over (in the location line): rejected with the length.
    let over = format!("{}\n", "c".repeat(MAX_LINE_BYTES + 1));
    match parse(&over) {
        Err(ParseError::LineTooLong { line, length }) => {
            assert_eq!(line, 1);
            assert_eq!(length, MAX_LINE_BYTES + 1);
        }
        other => unreachable!("expected LineTooLong, got {other:?}"),
    }

    // Over-cap line later in the file reports its own line number.
    let late = format!("#EXTM3U\nu.wav\n{}\n", "d".repeat(MAX_LINE_BYTES + 7));
    assert!(matches!(
        parse(&late),
        Err(ParseError::LineTooLong { line: 3, .. })
    ));
}

// ------------------------------------------------------- writer + roundtrip

#[test]
fn write_canonical_golden_pin() {
    let pl = Playlist {
        entries: vec![
            Entry {
                location: "a.mp3".into(),
                duration: Some(123.5),
                title: Some("Artist - T".into()),
                directives: vec![("EXTVLCOPT".to_string(), "http-user-agent=Lumen".into())],
                hls: HlsSegment::default(),
            },
            Entry {
                location: "b.flac".into(),
                duration: Some(123.0),
                title: None,
                directives: vec![],
                hls: HlsSegment::default(),
            },
            Entry {
                location: "c.ogg".into(),
                duration: None,
                title: None,
                directives: vec![],
                hls: HlsSegment::default(),
            },
        ],
        end_list: false,
    };
    assert_eq!(
        write(&pl),
        "#EXTM3U\n\
         #EXTVLCOPT:http-user-agent=Lumen\n\
         #EXTINF:123.5,Artist - T\n\
         a.mp3\n\
         #EXTINF:123\n\
         b.flac\n\
         c.ogg\n"
    );
    assert_eq!(pl.to_m3u_string(), write(&pl));
    // Empty playlist: header only.
    assert_eq!(write(&Playlist::new()), "#EXTM3U\n");
}

#[test]
fn roundtrip_all_fixtures_and_dirty_corpus() {
    let corpus = [
        BASIC,
        PLAIN,
        DIRECTIVES,
        COMMENTS,
        DIRTY,
        // Additional dirty corpus, one line each.
        "",
        "\n",
        "#EXTM3U",
        "#EXTM3U\n#EXTM3U:weird header payload\nu\n",
        "#EXTINF:1,T\n#EXTINF:2,U\nurl\n", // last EXTINF wins
        "#EXTVLCOPT:x=1\nu\n",             // directive-only entry
        "#EXT:\n#EXTINF:1,T\nu\n",         // nameless directive ignored
        "#EXTM\n",
        "#EXTINF:,only-title\nu\n",
        "#EXTINF:0,zero\nu\n",
        "#EXTINF:-1,live\nu\n",
        "#EXTINF:1.5,half\n#EXTGRP:G\nu\n",
        "  spaced-location.wav  \n",
        "#EXTINF: 42 ,padded\nu\n", // tolerant duration trim
        "#EXTINF:42,\n#EXTINF:43,after-empty\nu\n",
        "\u{FEFF}#EXTM3U\nu\n",
        "#EXTINF:1,ünïcode título\nu\n", // non-ASCII title
        "http://x/y?a=b&c=d#frag\n",
        "#EXTINF:1e-3,tiny\nu\n",               // exotic-but-valid float
        "#EXTINF:1234567890123456789,big\nu\n", // parses to a finite f64
        "#EXTINF:1,T\n#EXTVLCOPT:a=1\n#EXTGRP:G\nu\n#EXTVLCOPT:b=2\nv\n",
    ];
    for src in corpus {
        assert_roundtrip(src);
    }
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
    /// Uniform-ish value below `n` (n > 0).
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    fn byte(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
}

/// Interesting bytes that steer generation toward parser branch coverage.
const INTERESTING: &[u8] = b"#:,\r\nEXTIGM0123-. \t\xff\xfe\xef\xbb\xbf";

fn random_input(rng: &mut SplitMix64) -> Vec<u8> {
    // Mostly short (high UTF-8 validity, so both parse paths get hit); an
    // occasional long one to probe multi-line state.
    let len = if rng.below(8) == 0 {
        1 + rng.below(2048) as usize
    } else {
        1 + rng.below(48) as usize
    };
    let mut buf = Vec::with_capacity(len);
    for _ in 0..len {
        let roll = rng.below(100);
        if roll < 70 {
            // Printable ASCII: keeps most inputs valid UTF-8.
            buf.push(0x20 + (rng.below(0x5F)) as u8);
        } else if roll < 85 {
            buf.push(INTERESTING[(rng.below(INTERESTING.len() as u64)) as usize]);
        } else {
            buf.push(rng.byte());
        }
    }
    buf
}

#[test]
fn fuzz_lite_random_bytes_10k_no_panic() {
    let mut rng = SplitMix64::new(0x4D31_C0DE_5EED_0001); // "4.D1" seed
    let mut ok_count = 0usize;
    for _ in 0..10_000 {
        let buf = random_input(&mut rng);
        if let Ok(first) = parse_bytes(&buf) {
            ok_count += 1;
            // Roundtrip law holds for every input that parses.
            let written = write(&first.playlist);
            let Ok(second) = parse_detailed(&written) else {
                unreachable!("fuzz roundtrip re-parse failed for {written:?}");
            };
            assert_eq!(second.playlist, first.playlist);
        }
        // Err results (UTF-8 / line cap) are fine: typed, no panic.
    }
    // Sanity: the mix must exercise both paths (not all-UTF-8-rejects).
    assert!(ok_count > 100, "fuzz too weak: only {ok_count} parses");
}

#[test]
fn fuzz_lite_mutations_of_valid_playlist_5k() {
    let base = DIRECTIVES.as_bytes();
    let mut rng = SplitMix64::new(0x4D31_C0DE_5EED_0002);
    let mut ok_count = 0usize;
    for _ in 0..5_000 {
        let mut buf = base.to_vec();
        let mutations = 1 + rng.below(8);
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
        if let Ok(first) = parse_bytes(&buf) {
            ok_count += 1;
            let written = write(&first.playlist);
            let Ok(second) = parse_detailed(&written) else {
                unreachable!("mutation roundtrip re-parse failed for {written:?}");
            };
            assert_eq!(second.playlist, first.playlist);
        }
    }
    assert!(
        ok_count > 100,
        "mutation fuzz too weak: only {ok_count} parses"
    );
}
