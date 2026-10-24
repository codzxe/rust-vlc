//! Card 5.O1b (XSPF half) test suite: three handcrafted XML fixtures with
//! varied nesting/entities (ffmpeg emits no XSPF — these are hand-built
//! against the spec), exact damage-note pins for the dirty corpus, the
//! D-1 depth guard and line-cap bounds, the M3U cross-format bridge, and
//! the deterministic fuzz-lite harnesses. CI-able: `include_str!` only.
//!
//! Fuzz budget: this file generates at most ~92 KiB in the worst case
//! (800 random inputs ≤ 48 B each + 150 mutations of a ~200 B mini-XSPF +
//! 30 mutations of the ~730 B golden fixture) — together with the PLS
//! suite the two stay under the card's 200 KB total fuzz-input budget,
//! pinned by runtime byte counters.

use playlists::{
    Entry, MAX_LINE_BYTES, MAX_XML_DEPTH, XspfError, XspfWarning, parse_xspf, parse_xspf_bytes,
    parse_xspf_detailed,
};

const BASIC: &str = include_str!("fixtures/basic.xspf");
const NAMESPACED: &str = include_str!("fixtures/namespaced.xspf");
const DIRTY: &str = include_str!("fixtures/dirty.xspf");

/// Compact valid XSPF used as the mutation-fuzz base (keeps the byte
/// budget in check while exercising every field kind plus entities).
const MINI: &str = "<playlist version=\"1\"><trackList><track>\
<location>a.mp3</location><title>A &amp; B</title><creator>C</creator>\
<duration>1500</duration></track></trackList></playlist>";

// ---------------------------------------------------------------- golden

#[test]
fn golden_basic_entities_and_fields() {
    let Ok(d) = parse_xspf_detailed(BASIC) else {
        unreachable!("basic fixture must parse");
    };
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    assert_eq!(d.playlist.len(), 3);
    assert_eq!(
        d.playlist.entries[0],
        Entry {
            location: "file:///music/song1.mp3".into(),
            duration: Some(213.0), // 213000 ms -> seconds
            title: Some("Song & Dance".into()),
            directives: vec![
                ("creator".to_string(), "Artist <One>".into()),
                ("annotation".to_string(), "First & foremost".into()),
            ],
        }
    );
    // Title-less track; entities in creator; 0 ms duration kept.
    assert_eq!(
        d.playlist.entries[1],
        Entry {
            location: "http://example.com/stream".into(),
            duration: Some(0.0),
            title: None,
            directives: vec![("creator".to_string(), "\"Quoted\" 'name'".into())],
        }
    );
    // Decimal numeric reference decodes (é = U+00E9).
    assert_eq!(
        d.playlist.entries[2].title.as_deref(),
        Some("Unicode: étude")
    );
    assert_eq!(d.playlist.entries[2].duration, Some(321.5));
    assert_eq!(d.playlist.entries[2].location, "../relative/track03.ogg");
}

#[test]
fn golden_namespaced_prefixes_cdata_and_subsets() {
    let Ok(d) = parse_xspf_detailed(NAMESPACED) else {
        unreachable!("namespaced fixture must parse");
    };
    assert_eq!(
        d.warnings,
        vec![
            XspfWarning::DuplicateField { line: 11 }, // second location
            XspfWarning::TrackWithoutLocation { line: 25 }, // <xspf:track/>
        ]
    );
    assert_eq!(d.playlist.len(), 3);
    // First location wins; <album> (unknown element) ignored.
    assert_eq!(
        d.playlist.entries[0],
        Entry {
            location: "first-location.mp3".into(),
            duration: Some(10.0),
            title: Some("First wins".into()),
            directives: vec![],
        }
    );
    // CDATA content is raw (no entity decoding); multiline annotation is
    // trimmed; <meta> with attributes ignored.
    assert_eq!(
        d.playlist.entries[1],
        Entry {
            location: "multi.wav".into(),
            duration: None,
            title: Some("CDATA <title> & raw & stuff".into()),
            directives: vec![
                ("creator".to_string(), "The <b>Creator</b>".into()),
                ("annotation".to_string(), "text with\nmultiple lines".into()),
            ],
        }
    );
    // Self-closing track: kept, empty location.
    assert_eq!(d.playlist.entries[2].location, "");
    assert_eq!(d.playlist.entries[2].title, None);
}

#[test]
fn bom_and_xml_declaration_tolerance() {
    let src = "\u{FEFF}<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<playlist version=\"1\">\n  <trackList>\n    <track><location>bom.wav</location></track>\n  </trackList>\n</playlist>\n";
    let Ok(d) = parse_xspf_detailed(src) else {
        unreachable!("BOM + XML declaration must parse");
    };
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    assert_eq!(d.playlist.len(), 1);
    assert_eq!(d.playlist.entries[0].location, "bom.wav");
}

#[test]
fn empty_and_markup_only_inputs() {
    for src in [
        "",
        "\n  \n",
        "<!-- a comment -->",
        "<?pi content?>",
        "<!DOCTYPE playlist>",
        "<playlist/>",
        "<playlist><trackList/></playlist>",
    ] {
        let Ok(d) = parse_xspf_detailed(src) else {
            unreachable!("empty-ish input {src:?} must parse");
        };
        assert!(d.playlist.is_empty(), "{src:?}");
        assert!(d.warnings.is_empty(), "{src:?}: {:?}", d.warnings);
    }
}

// ------------------------------------------------------------ dirty world

#[test]
fn dirty_fixture_salvages_entries() {
    let Ok(d) = parse_xspf_detailed(DIRTY) else {
        unreachable!("dirty fixture must still parse (salvage, not death)");
    };
    assert_eq!(
        d.warnings,
        vec![
            XspfWarning::UnexpectedVersion { line: 1 }, // version="2"
            XspfWarning::BadDuration { line: 5 },       // not-a-number
            XspfWarning::UnclosedElement { line: 6 },   // <title> left open
            XspfWarning::BadEntity { line: 10 },        // &badent;
            XspfWarning::UnclosedElement { line: 10 },  // <creator> left open
            XspfWarning::UnclosedElement { line: 8 },   // <track> closed by </trackList>
            XspfWarning::UnexpectedEndTag { line: 12 }, // </wrong>
            XspfWarning::UnclosedElement { line: 13 },  // second <playlist> at EOF
            XspfWarning::UnclosedElement { line: 1 },   // root at EOF
        ]
    );
    // Both tracks survive: unclosed <title>/<creator> still deliver.
    assert_eq!(d.playlist.len(), 2);
    assert_eq!(
        d.playlist.entries[0],
        Entry {
            location: "one.mp3".into(),
            duration: None,
            title: Some("First Track".into()),
            directives: vec![],
        }
    );
    assert_eq!(
        d.playlist.entries[1],
        Entry {
            location: "two.mp3".into(),
            duration: None,
            title: None,
            directives: vec![("creator".to_string(), "Second &badent; Track".into())],
        }
    );
}

#[test]
fn prefix_mismatched_end_tags_and_stray_text() {
    let src = "<playlist><trackList><track>stray\
<location>x.mp3</xspf:location><title>T</title></track>\
</trackList></wrong></playlist>";
    let Ok(d) = parse_xspf_detailed(src) else {
        unreachable!();
    };
    assert_eq!(
        d.warnings,
        vec![
            XspfWarning::StrayText { line: 1 },
            XspfWarning::UnexpectedEndTag { line: 1 }, // </wrong>
        ]
    );
    assert_eq!(d.playlist.len(), 1);
    assert_eq!(d.playlist.entries[0].location, "x.mp3");
    assert_eq!(d.playlist.entries[0].title.as_deref(), Some("T"));
}

#[test]
fn nested_track_is_skipped_with_note() {
    let src = "<playlist><track><location>outer.mp3</location>\
<track><location>inner.mp3</location></track>\
<title>Outer</title></track></playlist>";
    let Ok(d) = parse_xspf_detailed(src) else {
        unreachable!();
    };
    assert_eq!(d.warnings, vec![XspfWarning::NestedTrack { line: 1 }]);
    assert_eq!(d.playlist.len(), 1, "inner track ignored");
    assert_eq!(d.playlist.entries[0].location, "outer.mp3");
    assert_eq!(d.playlist.entries[0].title.as_deref(), Some("Outer"));
}

#[test]
fn malformed_markup_degrades_not_dies() {
    // Unquoted attribute, bad tag, degenerate end tag, bad numeric entity,
    // unterminated CDATA — all damage notes, entry still parsed.
    let src = "<playlist version=2><trackList><track version=1> \
< broken<location>a&amp;b.mp3</location> \
<title>t&#x110000;t</title> \
<annotation><![CDATA[never closed\
</track></trackList></playlist>";
    let Ok(d) = parse_xspf_detailed(src) else {
        unreachable!("malformed markup must degrade to warnings");
    };
    assert_eq!(d.playlist.len(), 1);
    assert_eq!(d.playlist.entries[0].location, "a&b.mp3");
    assert_eq!(d.playlist.entries[0].title.as_deref(), Some("t&#x110000;t"));
    // The unterminated CDATA honestly swallows the rest of the input.
    assert_eq!(
        d.playlist.entries[0].directives,
        vec![(
            "annotation".to_string(),
            "never closed</track></trackList></playlist>".into()
        )]
    );
    let kinds = d
        .warnings
        .iter()
        .filter(|w| {
            matches!(
                w,
                XspfWarning::BadAttribute { .. }
                    | XspfWarning::BadTag { .. }
                    | XspfWarning::StrayText { .. }
                    | XspfWarning::BadEntity { .. }
                    | XspfWarning::UnterminatedCdata { .. }
                    | XspfWarning::UnclosedElement { .. }
            )
        })
        .count();
    assert!(
        kinds >= 5,
        "expected the damage kinds to fire, got {:?}",
        d.warnings
    );
}

#[test]
fn garbage_bytes_are_typed_invalid_utf8() {
    let garbage = [b'<', b't', 0xFF, 0xFE, 0x00, 0x80];
    match parse_xspf_bytes(&garbage) {
        Err(XspfError::InvalidUtf8) => {}
        other => unreachable!("expected InvalidUtf8, got {other:?}"),
    }
}

// ------------------------------------------------- D-1: bounded input

#[test]
fn depth_guard_is_fatal_at_65() {
    let deep = format!(
        "{}{}",
        "<a>".repeat(MAX_XML_DEPTH + 1),
        "</a>".repeat(MAX_XML_DEPTH + 1)
    );
    match parse_xspf(&deep) {
        Err(XspfError::TooDeep { line, depth }) => {
            assert_eq!((line, depth), (1, MAX_XML_DEPTH + 1));
            assert_eq!(
                XspfError::TooDeep { line, depth }.to_string(),
                "playlists: line 1: XML nesting depth 65 exceeds the 64-element depth guard (D-1 bounded input)"
            );
        }
        other => unreachable!("depth 65 must be TooDeep, got {other:?}"),
    }
    // Exactly at the guard: parses cleanly (all elements close).
    let at_guard = format!(
        "{}{}",
        "<a>".repeat(MAX_XML_DEPTH),
        "</a>".repeat(MAX_XML_DEPTH)
    );
    let Ok(d) = parse_xspf_detailed(&at_guard) else {
        unreachable!("depth at the guard must parse");
    };
    assert!(d.playlist.is_empty());
}

#[test]
fn over_cap_line_is_a_typed_error() {
    // One 64 KiB+ text line: fatal before any scanning.
    let huge = format!("<playlist><title>{}", "a".repeat(MAX_LINE_BYTES));
    match parse_xspf(&huge) {
        Err(XspfError::LineTooLong { line, length }) => {
            assert_eq!((line, length), (1, 17 + MAX_LINE_BYTES));
            assert_eq!(
                XspfError::LineTooLong { line, length }.to_string(),
                format!(
                    "playlists: line 1 is {} bytes, over the 65536-byte line cap (D-1 bounded input)",
                    17 + MAX_LINE_BYTES
                )
            );
        }
        other => unreachable!("over-cap line must be LineTooLong, got {other:?}"),
    }
    // Multi-line text under the cap per line: fine.
    let multi = format!(
        "<playlist><trackList><track><annotation>{}</annotation></track></trackList></playlist>",
        "x".repeat(1000)
    );
    assert!(parse_xspf(&multi).is_ok());
}

// ------------------------------------------------- cross-format M3U bridge

#[test]
fn xspf_bridges_through_m3u_with_documented_asymmetry() {
    // location/title/duration survive the canonical-M3U bridge verbatim;
    // creator/annotation are serialized as #creator:/#annotation: lines
    // — kept in the file text, though the M3U parser's subset only
    // re-attaches #EXT… directives (documented in xspf.rs module docs).
    let Ok(first) = parse_xspf_detailed(BASIC) else {
        unreachable!("fixture must parse");
    };
    let written = first.playlist.to_m3u_string();
    let Ok(second) = playlists::parse_detailed(&written) else {
        unreachable!("m3u write must re-parse: {written:?}");
    };
    assert_eq!(second.playlist.len(), first.playlist.len());
    for (bridged, orig) in second.playlist.entries.iter().zip(&first.playlist.entries) {
        assert_eq!(bridged.location, orig.location);
        assert_eq!(bridged.title, orig.title);
        assert_eq!(bridged.duration, orig.duration);
    }
    assert!(written.contains("#creator:Artist <One>"));
    assert!(written.contains("#annotation:First & foremost"));
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

/// Bytes that steer generation toward XML branch coverage.
const INTERESTING: &[u8] = b"<>/=&;\"'![?]-CDATAtrackplaylistlocation\x00\xff\xfe\r\n\t ";

fn random_input(rng: &mut SplitMix64) -> Vec<u8> {
    let len = 1 + rng.below(48) as usize;
    let mut buf = Vec::with_capacity(len);
    for _ in 0..len {
        let roll = rng.below(100);
        if roll < 65 {
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
fn fuzz_lite_random_bytes_no_panic() {
    let mut rng = SplitMix64::new(0x58_5350_4600_0001); // "XSPF" seed
    let mut ok_count = 0usize;
    let mut generated = 0usize;
    for _ in 0..800 {
        let buf = random_input(&mut rng);
        generated += buf.len();
        if let Ok(d) = parse_xspf_bytes(&buf) {
            ok_count += 1;
            // Sanity: entries and warnings stay bounded by input size
            // (a damage byte can cost a couple of warnings; 4× is roomy).
            assert!(d.playlist.len() <= buf.len());
            assert!(d.warnings.len() <= buf.len() * 4 + 64);
        }
        // Err results (UTF-8 / line cap / depth) are fine: typed, no panic.
    }
    // Damage degrades to warnings, so nearly every *valid-UTF-8* input
    // parses; the generator's random-byte tail keeps the count near 30%.
    assert!(ok_count > 150, "fuzz too weak: only {ok_count} parses");
    assert!(
        generated < 100_000,
        "fuzz budget exceeded: {generated} bytes"
    );
}

#[test]
fn fuzz_lite_mutations_of_valid_xspf_no_panic() {
    let mut rng = SplitMix64::new(0x58_5350_4600_0002);
    let mut ok_count = 0usize;
    let mut generated = 0usize;
    for round in 0..180u32 {
        // 150 rounds mutate the compact MINI base, 30 the golden fixture.
        let base: &[u8] = if round % 6 == 0 {
            BASIC.as_bytes()
        } else {
            MINI.as_bytes()
        };
        let mut buf = base.to_vec();
        let mutations = 1 + rng.below(10);
        for _ in 0..mutations {
            if buf.is_empty() {
                break;
            }
            match rng.below(4) {
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
                2 => {
                    let i = rng.below(buf.len() as u64 + 1) as usize;
                    buf.insert(i, rng.byte());
                }
                _ => {
                    let cut = rng.below(buf.len() as u64 + 1) as usize;
                    buf.truncate(cut);
                }
            }
        }
        generated += buf.len();
        if let Ok(d) = parse_xspf_bytes(&buf) {
            ok_count += 1;
            assert!(d.playlist.len() <= buf.len());
            assert!(d.warnings.len() <= buf.len() * 4 + 64);
        }
    }
    assert!(ok_count > 100, "mutation fuzz too weak: only {ok_count}");
    assert!(
        generated < 100_000,
        "fuzz budget exceeded: {generated} bytes"
    );
}
