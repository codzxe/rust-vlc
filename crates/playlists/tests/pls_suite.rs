//! Card 5.O1b (PLS half) test suite: spec-shaped golden fixtures, sparse
//! indexes, the dirty corpus with exact damage-notes, D-1 line-cap
//! boundaries, the M3U cross-format bridge, and the deterministic
//! fuzz-lite harnesses (random bytes + mutations of a valid playlist).
//! CI-able: no I/O beyond compile-time `include_str!`.
//!
//! Fuzz budget: this file generates at most ~98 KiB of input in the worst
//! case (1 000 random inputs ≤ 32 B each + 200 mutations of a ~330 B
//! fixture) — the two 5.O1b suites together stay under the card's 200 KB
//! total fuzz-input budget, pinned by runtime byte counters.

use playlists::{
    Entry, MAX_LINE_BYTES, PlsError, PlsWarning, parse_pls, parse_pls_bytes, parse_pls_detailed,
};

const BASIC: &str = include_str!("fixtures/basic.pls");
const SPARSE: &str = include_str!("fixtures/sparse.pls");
const DIRTY: &str = include_str!("fixtures/dirty.pls");

// ---------------------------------------------------------------- golden

#[test]
fn golden_basic_spec_shaped() {
    let Ok(d) = parse_pls_detailed(BASIC) else {
        unreachable!("basic fixture must parse");
    };
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    assert_eq!(d.playlist.len(), 3, "three File keys, three entries");
    assert_eq!(
        d.playlist.entries[0],
        Entry {
            location: "http://relay.example.com:8000/live.mp3".into(),
            duration: Some(-1.0), // the spec's unknown/live marker, preserved
            title: Some("Live Stream".into()),
            directives: vec![],
        }
    );
    assert_eq!(
        d.playlist.entries[1],
        Entry {
            location: "music/track01.ogg".into(),
            duration: Some(213.0),
            title: Some("Track One".into()),
            directives: vec![],
        }
    );
    // Windows path kept verbatim; fractional length parses.
    assert_eq!(d.playlist.entries[2].location, r"C:\Music\Example.flac");
    assert_eq!(d.playlist.entries[2].duration, Some(321.5));
    assert_eq!(
        d.playlist.entries[2].title.as_deref(),
        Some("Example Artist - Example Title")
    );
}

#[test]
fn golden_sparse_indexes_case_and_comments() {
    let Ok(d) = parse_pls_detailed(SPARSE) else {
        unreachable!("sparse fixture must parse");
    };
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    // Index order, not source-line order: idx 1 then idx 5 (2..4 absent).
    assert_eq!(d.playlist.len(), 2);
    assert_eq!(
        d.playlist.entries[0],
        Entry {
            location: "one.wav".into(),
            duration: None,
            title: None,
            directives: vec![],
        }
    );
    assert_eq!(d.playlist.entries[1].location, "five.wav");
    assert_eq!(d.playlist.entries[1].title.as_deref(), Some("Five"));
}

#[test]
fn crlf_bom_and_case_tolerance() {
    let src = "\u{FEFF}[playlist]\r\nFILE1=UPPER.WAV\r\nlength1 = 42\r\nnumberofentries=1\r\nversion=2\r\n";
    let Ok(d) = parse_pls_detailed(src) else {
        unreachable!("BOM+CRLF source must parse");
    };
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    assert_eq!(d.playlist.len(), 1);
    assert_eq!(d.playlist.entries[0].location, "UPPER.WAV");
    assert_eq!(d.playlist.entries[0].duration, Some(42.0));
}

#[test]
fn empty_and_comment_only_inputs() {
    for src in [
        "",
        "\n\n\r\n",
        "; only a comment\n",
        "# hash comment\n",
        "[playlist]\n",
    ] {
        let Ok(d) = parse_pls_detailed(src) else {
            unreachable!("empty-ish input {src:?} must parse");
        };
        assert!(d.playlist.is_empty(), "{src:?}");
        assert!(d.warnings.is_empty(), "{src:?}: {:?}", d.warnings);
    }
}

#[test]
fn pre_section_keys_parse_with_one_warning() {
    // Missing [playlist] header: known keys still parse (tolerant), with a
    // single damage note at the first key.
    let src = "File1=headless.wav\nTitle1=Headless\nNumberOfEntries=1\nVersion=2\n";
    let Ok(d) = parse_pls_detailed(src) else {
        unreachable!("headerless PLS must parse");
    };
    assert_eq!(
        d.warnings,
        vec![PlsWarning::MissingSectionHeader { line: 1 }]
    );
    assert_eq!(d.playlist.len(), 1);
    assert_eq!(d.playlist.entries[0].location, "headless.wav");
    assert_eq!(d.playlist.entries[0].title.as_deref(), Some("Headless"));
}

// ------------------------------------------------------------ dirty world

#[test]
fn dirty_fixture_degrades_honestly() {
    let Ok(d) = parse_pls_detailed(DIRTY) else {
        unreachable!("dirty fixture must still parse (entry-not-death)");
    };
    assert_eq!(
        d.warnings,
        vec![
            PlsWarning::BadIndex { line: 2 },           // File0
            PlsWarning::BadIndex { line: 3 },           // Filex
            PlsWarning::BadLength { line: 4 },          // Length1=not-a-number
            PlsWarning::BadLength { line: 8 },          // Length2=abc
            PlsWarning::DuplicateKey { line: 9 },       // File2 twice
            PlsWarning::EmptyLocation { line: 10 },     // File3=
            PlsWarning::NotAKeyValuePair { line: 11 },  // no '=' at all
            PlsWarning::UnexpectedVersion { line: 17 }, // Version=1
            PlsWarning::OrphanMetadata { line: 4 },     // Title1/Length1, no File1
            PlsWarning::EntryCountMismatch {
                declared: 99,
                found: 3
            },
        ]
    );
    // Entries survive the damage; duplicate File2: last value wins.
    assert_eq!(d.playlist.len(), 3);
    assert_eq!(d.playlist.entries[0].location, "two-duplicate.wav");
    assert_eq!(d.playlist.entries[0].title.as_deref(), Some("Two"));
    assert_eq!(d.playlist.entries[0].duration, None);
    assert_eq!(d.playlist.entries[1].location, "", "empty File3 kept");
    assert_eq!(d.playlist.entries[2].location, "four.wav");
    // File9 under [other-section]: INI semantics, ignored (no entry).
}

#[test]
fn non_finite_and_negative_lengths() {
    // -1 is the unknown marker (preserved); inf/nan/garbage are damage.
    let Ok(d) = parse_pls_detailed("[playlist]\nFile1=a.wav\nLength1=-1\n") else {
        unreachable!();
    };
    assert_eq!(d.playlist.entries[0].duration, Some(-1.0));
    for bad in ["inf", "NaN", "1e999", ""] {
        let src = format!("[playlist]\nFile1=a.wav\nLength1={bad}\n");
        let Ok(d) = parse_pls_detailed(&src) else {
            unreachable!("bad length must not be fatal");
        };
        assert_eq!(
            d.warnings,
            vec![PlsWarning::BadLength { line: 3 }],
            "length {bad:?}"
        );
        assert_eq!(d.playlist.entries[0].duration, None);
        assert_eq!(d.playlist.entries[0].location, "a.wav");
    }
}

#[test]
fn overflowing_index_and_count() {
    let src =
        "[playlist]\nFile99999999999999999999999=x.wav\nNumberOfEntries=99999999999999999999999\n";
    let Ok(d) = parse_pls_detailed(src) else {
        unreachable!("overflowing numbers must not be fatal");
    };
    assert_eq!(
        d.warnings,
        vec![
            PlsWarning::BadIndex { line: 2 },
            PlsWarning::BadEntryCount { line: 3 },
        ]
    );
    assert!(d.playlist.is_empty());
}

#[test]
fn value_with_equals_and_unknown_keys() {
    // Split at the FIRST '='; unknown keys ignored per INI convention.
    let src =
        "[playlist]\nFile1=url?a=b=c\nGenre1=Rock\nCustom=thing\nNumberOfEntries=1\nVersion=2\n";
    let Ok(d) = parse_pls_detailed(src) else {
        unreachable!();
    };
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    assert_eq!(d.playlist.len(), 1);
    assert_eq!(d.playlist.entries[0].location, "url?a=b=c");
    assert_eq!(d.playlist.entries[0].title, None);
}

#[test]
fn garbage_bytes_are_typed_invalid_utf8() {
    let garbage = [b'[', b'p', 0xFF, 0xFE, 0x00, 0x80];
    match parse_pls_bytes(&garbage) {
        Err(PlsError::InvalidUtf8) => {}
        other => unreachable!("expected InvalidUtf8, got {other:?}"),
    }
}

// ------------------------------------------------- D-1: bounded line input

#[test]
fn over_cap_line_is_a_typed_error() {
    // 6 + 65536 = 65542 bytes on line 1.
    let huge = format!("[playlist]\nFile1={}", "a".repeat(MAX_LINE_BYTES));
    match parse_pls(&huge) {
        Err(PlsError::LineTooLong { line, length }) => {
            assert_eq!((line, length), (2, 6 + MAX_LINE_BYTES));
            // Redaction pin: exact message — line number and byte counts
            // only, no content (MP §12).
            assert_eq!(
                PlsError::LineTooLong { line, length }.to_string(),
                format!(
                    "playlists: line 2 is {} bytes, over the 65536-byte line cap (D-1 bounded input)",
                    6 + MAX_LINE_BYTES
                )
            );
        }
        other => unreachable!("over-cap line must be LineTooLong, got {other:?}"),
    }

    // Exactly at the cap (on the value): accepted.
    let at_cap = format!("File1={}\n", "b".repeat(MAX_LINE_BYTES - 6));
    assert!(parse_pls(&at_cap).is_ok());
}

// ------------------------------------------------- cross-format M3U bridge

#[test]
fn pls_bridges_losslessly_through_m3u() {
    // PLS entries carry exactly the fields M3U models, so the canonical
    // M3U writer must round-trip them (documented in pls.rs module docs).
    for src in [BASIC, SPARSE] {
        let Ok(first) = parse_pls_detailed(src) else {
            unreachable!("fixture must parse");
        };
        let written = first.playlist.to_m3u_string();
        let Ok(second) = playlists::parse_detailed(&written) else {
            unreachable!("m3u write must re-parse: {written:?}");
        };
        assert_eq!(second.playlist, first.playlist, "bridge broke for {src:?}");
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
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    fn byte(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
}

/// Bytes that steer generation toward PLS branch coverage.
const INTERESTING: &[u8] = b"[]=;#\r\nFileTitlengh0123-9. \t\xff\xfe\xef\xbb\xbf";

fn random_input(rng: &mut SplitMix64) -> Vec<u8> {
    let len = 1 + rng.below(32) as usize;
    let mut buf = Vec::with_capacity(len);
    for _ in 0..len {
        let roll = rng.below(100);
        if roll < 70 {
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
    let mut rng = SplitMix64::new(0x50_4C53_0000_0001); // "PLS" seed
    let mut ok_count = 0usize;
    let mut generated = 0usize;
    for _ in 0..1_000 {
        let buf = random_input(&mut rng);
        generated += buf.len();
        if let Ok(d) = parse_pls_bytes(&buf) {
            ok_count += 1;
            // Sanity: entries and warnings are bounded by input size.
            assert!(d.playlist.len() <= buf.len());
            assert!(d.warnings.len() <= buf.len() * 4 + 64);
        }
        // Err results (UTF-8 / line cap) are fine: typed, no panic.
    }
    // Damage degrades to warnings, so nearly every *valid-UTF-8* input
    // parses; the generator's random-byte tail keeps the count near 30%.
    assert!(ok_count > 200, "fuzz too weak: only {ok_count} parses");
    assert!(
        generated < 100_000,
        "fuzz budget exceeded: {generated} bytes"
    );
}

#[test]
fn fuzz_lite_mutations_of_valid_pls_no_panic() {
    let base = BASIC.as_bytes();
    let mut rng = SplitMix64::new(0x50_4C53_0000_0002);
    let mut ok_count = 0usize;
    let mut generated = 0usize;
    for _ in 0..200 {
        let mut buf = base.to_vec();
        let mutations = 1 + rng.below(8);
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
                    // truncation
                    let cut = rng.below(buf.len() as u64 + 1) as usize;
                    buf.truncate(cut);
                }
            }
        }
        generated += buf.len();
        if let Ok(d) = parse_pls_bytes(&buf) {
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
