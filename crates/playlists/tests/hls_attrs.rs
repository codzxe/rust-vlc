//! Card 5.O1a test suite: HLS `#EXT-X-*` media-playlist subset
//! (attribute lists, BYTERANGE / DISCONTINUITY / KEY / MAP / ENDLIST),
//! RFC 3986 URI joining, and the ffmpeg-generated acceptance fixtures.
//!
//! ## Acceptance fixtures (real ffmpeg 7.1.5.5-0+deb13u1 output)
//!
//! Generated this session with the card's recipe
//! (`ffmpeg -i <src> -c copy -f hls …`, source = 9 s mono 22.05 kHz AAC
//! sine at 24 kb/s), then measured with ffprobe — the pinned constants
//! below ARE those measurements (regenerate: see the doc comments on each
//! test):
//!
//! | case | playlist | truth |
//! |---|---|---|
//! | TS VOD | `fixtures/hls/ts_vod/pl.m3u8` | ffprobe duration 9.046543, 5 segments on disk |
//! | single-file byteranges | `fixtures/hls/single_file/pl.m3u8` | duration 9.046543, `pl.ts` = 35344 bytes |
//! | fMP4 + MAP | `fixtures/hls/fmp4/pl.m3u8` | duration 9.046439, `init.mp4` = 764 bytes |
//!
//! The tests re-derive segment counts and byte totals from the committed
//! files themselves, so the pins cannot drift from the fixtures.

use playlists::{
    ByteRange, EncryptionMethod, Entry, HlsKey, HlsMap, ParseWarning, Playlist, parse_bytes,
    parse_detailed, resolve_uri, write,
};
use std::path::PathBuf;

const TS_VOD: &str = include_str!("fixtures/hls/ts_vod/pl.m3u8");
const SINGLE_FILE: &str = include_str!("fixtures/hls/single_file/pl.m3u8");
const FMP4: &str = include_str!("fixtures/hls/fmp4/pl.m3u8");

fn fixture_dir(case: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/hls")
        .join(case)
}

/// Sum of parsed `#EXTINF` durations.
fn total_duration(p: &Playlist) -> f64 {
    p.entries.iter().filter_map(|e| e.duration).sum()
}

/// The 4.D1 let-else pattern, wrapped once (clippy denies unwrap/expect).
fn must_parse(src: &str) -> playlists::Parsed {
    let Ok(d) = parse_detailed(src) else {
        unreachable!("input must parse: {src:?}")
    };
    d
}

/// Roundtrip law: `parse(write(p)) == p`.
fn assert_roundtrip(p: &Playlist) {
    let rewritten = write(p);
    let second = must_parse(&rewritten);
    assert_eq!(second.playlist, *p, "roundtrip stability violated");
}

/// A fixture file's byte length (the on-disk truth).
fn fixture_len(case: &str, name: &str) -> u64 {
    match std::fs::metadata(fixture_dir(case).join(name)) {
        Ok(m) => m.len(),
        Err(e) => unreachable!("fixture {case}/{name}: {e}"),
    }
}

/// ffmpeg HLS case 1 — plain TS VOD (ffmpeg 7.1.5:
/// `ffmpeg -i src.m4a -c copy -f hls -hls_time 2 -hls_list_size 0
///  -hls_playlist_type vod -start_number 0 pl.m3u8`).
/// ffprobe truth (measured this session): format duration 9.046543.
#[test]
fn ffmpeg_hls_ts_vod_matches_ffprobe_truth() {
    let d = must_parse(TS_VOD);
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    let p = &d.playlist;

    // Entry count == segment files on disk (not a constant — derived).
    let segments = match std::fs::read_dir(fixture_dir("ts_vod")) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "ts"))
            .count(),
        Err(e) => unreachable!("fixture dir: {e}"),
    };
    assert_eq!(p.len(), 5, "five EXTINF/URI pairs");
    assert_eq!(p.len(), segments, "entries == .ts files on disk");

    // Durations: ffmpeg's own EXTINF values, verbatim.
    let expected = [2.043_356, 1.996_922, 1.996_911, 1.996_922, 1.012_433];
    for (e, want) in p.entries.iter().zip(expected) {
        assert_eq!(e.duration, Some(want), "duration of {}", e.location);
        assert_eq!(e.title, None, "HLS EXTINF has a bare trailing comma");
    }
    // Sum vs ffprobe's 9.046543 (EXTINF rounding is ffmpeg's, not ours).
    let sum = total_duration(p);
    assert!(
        (sum - 9.046_543).abs() < 0.01,
        "EXTINF sum {sum} vs ffprobe duration 9.046543"
    );

    // Segment URIs in order.
    for (i, e) in p.entries.iter().enumerate() {
        assert_eq!(e.location, format!("pl{i}.ts"));
    }

    // Playlist-level facts: ENDLIST recorded, header tags preserved as
    // verbatim directives (nothing invented).
    assert!(p.end_list);
    assert!(!p.is_live());
    assert_eq!(
        p.entries[0].directives,
        vec![
            ("EXT-X-VERSION".to_string(), "3".into()),
            ("EXT-X-TARGETDURATION".to_string(), "2".into()),
            ("EXT-X-MEDIA-SEQUENCE".to_string(), "0".into()),
            ("EXT-X-PLAYLIST-TYPE".to_string(), "VOD".into()),
        ]
    );
    // Roundtrip law holds for real ffmpeg output.
    assert_roundtrip(p);
}

/// ffmpeg HLS case 2 — `-hls_flags single_file`: every segment is a
/// BYTERANGE into one `pl.ts` (35 344 bytes on disk, measured this
/// session; ffprobe duration 9.046543).
#[test]
fn ffmpeg_hls_single_file_byteranges_match_disk_truth() {
    let d = must_parse(SINGLE_FILE);
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    let p = &d.playlist;

    assert_eq!(p.len(), 5);
    assert!(p.end_list);
    for e in &p.entries {
        assert_eq!(e.location, "pl.ts", "all segments point into one file");
    }

    // ffmpeg's exact ranges (explicit offsets, verbatim).
    let expected = [
        (8084, Some(0)),
        (7708, Some(8084)),
        (7708, Some(15792)),
        (7708, Some(23500)),
        (4136, Some(31208)),
    ];
    for (e, (len, off)) in p.entries.iter().zip(expected) {
        assert_eq!(
            e.hls.byte_range,
            Some(ByteRange {
                length: len,
                offset: off
            }),
            "byterange of {}",
            e.location
        );
    }

    // Disk truth, derived live (not a constant): the ranges tile the file.
    let file_len = fixture_len("single_file", "pl.ts");
    assert_eq!(file_len, 35344, "fixture must be the measured one");
    let mut cursor = 0u64;
    for e in &p.entries {
        let Some(br) = e.hls.byte_range.as_ref() else {
            unreachable!("every segment has a range")
        };
        assert_eq!(br.offset, Some(cursor), "ranges are contiguous");
        cursor += br.length;
    }
    assert_eq!(cursor, file_len, "ranges cover the file exactly");

    let sum = total_duration(p);
    assert!(
        (sum - 9.046_543).abs() < 0.01,
        "EXTINF sum {sum} vs ffprobe duration 9.046543"
    );
    // Roundtrip.
    assert_roundtrip(p);
}

/// ffmpeg HLS case 3 — `-hls_segment_type fmp4`: an `#EXT-X-MAP` init
/// section plus fMP4 segments (ffprobe duration 9.046439, `init.mp4` =
/// 764 bytes, both measured this session).
#[test]
fn ffmpeg_hls_fmp4_map_matches_ffprobe_truth() {
    let d = must_parse(FMP4);
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    let p = &d.playlist;

    assert_eq!(p.len(), 5);
    assert!(p.end_list);

    // MAP is stamped on every segment it governs (stateful tag).
    let init_len = fixture_len("fmp4", "init.mp4");
    assert_eq!(init_len, 764, "fixture must be the measured one");
    for e in &p.entries {
        assert_eq!(
            e.hls.map,
            Some(HlsMap {
                uri: "init.mp4".into(),
                byte_range: None,
            }),
            "MAP governs {}",
            e.location
        );
        assert_eq!(e.hls.key, None);
    }

    let expected = [2.043_356, 1.996_916, 1.996_916, 1.996_916, 1.012_336];
    for (e, want) in p.entries.iter().zip(expected) {
        assert_eq!(e.duration, Some(want));
    }
    let sum = total_duration(p);
    assert!(
        (sum - 9.046_439).abs() < 0.01,
        "EXTINF sum {sum} vs ffprobe duration 9.046439"
    );
    assert_roundtrip(p);
}

// -------------------------------------------------------- HLS subset units

#[test]
fn byterange_forms_and_malformed_tolerance() {
    let src = "#EXTM3U\n\
               #EXT-X-BYTERANGE:8084\n\
               a.ts\n\
               #EXT-X-BYTERANGE:7708@8084\n\
               a.ts\n\
               #EXT-X-BYTERANGE:\n\
               #EXT-X-BYTERANGE:abc\n\
               #EXT-X-BYTERANGE:12@x\n\
               #EXT-X-BYTERANGE:18446744073709551616\n\
               #EXT-X-BYTERANGE:10@0\n\
               b.ts\n";
    let d = must_parse(src);
    // Line-level skips: 5 bad lines, entries survive.
    assert_eq!(
        d.warnings,
        vec![
            ParseWarning::BadExtXTag { line: 6 },
            ParseWarning::BadExtXTag { line: 7 },
            ParseWarning::BadExtXTag { line: 8 },
            ParseWarning::BadExtXTag { line: 9 },
        ]
    );
    assert_eq!(d.playlist.len(), 3);
    assert_eq!(
        d.playlist.entries[0].hls.byte_range,
        Some(ByteRange {
            length: 8084,
            offset: None
        })
    );
    assert_eq!(
        d.playlist.entries[1].hls.byte_range,
        Some(ByteRange {
            length: 7708,
            offset: Some(8084)
        })
    );
    // The four malformed lines were skipped; the later good one still hit.
    assert_eq!(
        d.playlist.entries[2].hls.byte_range,
        Some(ByteRange {
            length: 10,
            offset: Some(0)
        })
    );
    // Roundtrip: model equality (bad lines stay dropped after rewrite).
    assert_roundtrip(&d.playlist);
}

#[test]
fn discontinuity_flags_next_segment_only() {
    let src = "#EXTM3U\n\
               #EXT-X-DISCONTINUITY\n\
               a.ts\n\
               b.ts\n\
               #EXT-X-DISCONTINUITY\n\
               #EXT-X-DISCONTINUITY:junk\n\
               c.ts\n\
               #EXT-X-ENDLIST\n";
    let d = must_parse(src);
    assert_eq!(
        d.warnings,
        vec![ParseWarning::BadExtXTag { line: 6 }],
        "junk on a valueless tag is damage"
    );
    assert_eq!(d.playlist.len(), 3);
    assert!(d.playlist.entries[0].hls.discontinuity);
    assert!(!d.playlist.entries[1].hls.discontinuity);
    assert!(
        d.playlist.entries[2].hls.discontinuity,
        "repeated tag is not a toggle"
    );
    assert!(d.playlist.end_list);
}

#[test]
fn key_recorded_statefully_and_none_clears() {
    // Spec-example attribute values (RFC 8216 §4.3.2.4 style).
    let src = "#EXTM3U\n\
               #EXT-X-KEY:METHOD=AES-128,URI=\"https://priv.example.com/key.php?r=52\"\n\
               #EXTINF:4,\n\
               a.ts\n\
               b.ts\n\
               #EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"skd://key65\",IV=0X9c7db8778570d05c3177c349fd9236aa,KEYFORMAT=\"com.apple.streamingkeydelivery\",KEYFORMATVERSIONS=\"1\"\n\
               c.ts\n\
               #EXT-X-KEY:METHOD=NONE\n\
               d.ts\n";
    let d = must_parse(src);
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    let p = &d.playlist;
    assert_eq!(p.len(), 4);

    let want_first = HlsKey {
        method: EncryptionMethod::Aes128,
        uri: "https://priv.example.com/key.php?r=52".into(),
        iv: None,
        keyformat: None,
        keyformat_versions: None,
    };
    assert_eq!(p.entries[0].hls.key.as_ref(), Some(&want_first));
    // Stateful: b.ts is governed by the same key (stamped, not replayed).
    assert_eq!(p.entries[1].hls.key.as_ref(), Some(&want_first));

    // Commas inside the quoted URI do not split; hex is case-tolerant.
    let want_second = HlsKey {
        method: EncryptionMethod::SampleAes,
        uri: "skd://key65".into(),
        iv: Some(vec![
            0x9c, 0x7d, 0xb8, 0x77, 0x85, 0x70, 0xd0, 0x5c, 0x31, 0x77, 0xc3, 0x49, 0xfd, 0x92,
            0x36, 0xaa,
        ]),
        keyformat: Some("com.apple.streamingkeydelivery".into()),
        keyformat_versions: Some("1".into()),
    };
    assert_eq!(p.entries[2].hls.key.as_ref(), Some(&want_second));

    // METHOD=NONE clears the effective key.
    assert_eq!(p.entries[3].hls.key, None);

    // Roundtrip: the writer delta-encodes (emits KEY only at changes) and
    // re-parsing re-stamps the same state.
    assert_roundtrip(p);
    let text = write(p);
    assert_eq!(
        text.matches("#EXT-X-KEY").count(),
        3,
        "one KEY line per change"
    );
    assert!(text.contains("#EXT-X-KEY:METHOD=NONE\n"));
}

#[test]
fn unknown_enumerated_key_method_is_recorded() {
    let src = "#EXTM3U\n#EXT-X-KEY:METHOD=FUTURE-METHOD,URI=\"k\",UNKNOWNATTR=1\nx.ts\n";
    let d = must_parse(src);
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    assert_eq!(
        d.playlist.entries[0].hls.key.as_ref().map(|k| &k.method),
        Some(&EncryptionMethod::Other("FUTURE-METHOD".into())),
        "unknown methods are recorded, not rejected"
    );
    // Roundtrip preserves it.
    assert_roundtrip(&d.playlist);
}

#[test]
fn malformed_keys_skip_line_and_keep_state() {
    let bads = [
        "#EXT-X-KEY:",                                                  // empty list
        "#EXT-X-KEY:METHOD",                                            // no '='
        "#EXT-X-KEY:METHOD=AES-128",                                    // URI required, missing
        "#EXT-X-KEY:METHOD=AES-128,URI=key",                            // URI must be quoted
        "#EXT-X-KEY:METHOD=AES-128,URI=\"unterminated",                 // no closing quote
        "#EXT-X-KEY:METHOD=AES-128,URI=\"a\",METHOD=AES-128,URI=\"b\"", // duplicate attrs
        "#EXT-X-KEY:METHOD=AES-128,URI=\"a\",IV=0x123",                 // odd-length hex
        "#EXT-X-KEY:METHOD=AES-128,URI=\"a\",IV=12",                    // IV must be hex
        "#EXT-X-KEY:METHOD=NONE,URI=\"a\"",                             // NONE must not carry URI
        "#EXT-X-KEY:URI=\"a\"",                                         // METHOD required
        "#EXT-X-KEY:METHOD=\"AES-128\",URI=\"a\"", // METHOD is enumerated, not quoted
        "#EXT-X-KEY:,URI=\"a\"",                   // empty name
        "#EXT-X-KEY:METHOD=AES-128,URI=\"a\",",    // trailing comma
    ];
    for bad in bads {
        let src = format!(
            "#EXTM3U\n\
             #EXT-X-KEY:METHOD=AES-128,URI=\"good\"\n\
             first.ts\n\
             {bad}\n\
             second.ts\n"
        );
        let d = must_parse(&src);
        assert_eq!(
            d.warnings,
            vec![ParseWarning::BadExtXTag { line: 4 }],
            "case {bad:?}"
        );
        // Prior state kept: the malformed line was skipped entirely.
        let Some(key) = d.playlist.entries[1].hls.key.as_ref() else {
            unreachable!("state kept")
        };
        assert_eq!(key.uri, "good", "case {bad:?}");
        assert_eq!(d.playlist.len(), 2, "case {bad:?}");
    }
}

#[test]
fn map_with_quoted_byterange() {
    let src = "#EXTM3U\n\
               #EXT-X-MAP:URI=\"init.mp4\"\n\
               a.m4s\n\
               b.m4s\n\
               #EXT-X-MAP:URI=\"init2.mp4\",BYTERANGE=\"739@0\"\n\
               c.m4s\n";
    let d = must_parse(src);
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    let p = &d.playlist;
    assert_eq!(p.len(), 3);
    assert_eq!(
        p.entries[0].hls.map,
        Some(HlsMap {
            uri: "init.mp4".into(),
            byte_range: None
        })
    );
    assert_eq!(p.entries[1].hls.map, p.entries[0].hls.map, "stateful");
    assert_eq!(
        p.entries[2].hls.map,
        Some(HlsMap {
            uri: "init2.mp4".into(),
            byte_range: Some(ByteRange {
                length: 739,
                offset: Some(0)
            }),
        })
    );
    // Bad MAPs skip the line, keep prior state.
    let bad = "#EXTM3U\n\
               #EXT-X-MAP:URI=\"init.mp4\"\n\
               a.m4s\n\
               #EXT-X-MAP:BYTERANGE=\"10@0\"\n\
               #EXT-X-MAP:URI=init.mp4\n\
               #EXT-X-MAP:URI=\"x\",BYTERANGE=\"abc\"\n\
               b.m4s\n";
    let d = must_parse(bad);
    assert_eq!(
        d.warnings,
        vec![
            ParseWarning::BadExtXTag { line: 4 },
            ParseWarning::BadExtXTag { line: 5 },
            ParseWarning::BadExtXTag { line: 6 },
        ]
    );
    assert_eq!(d.playlist.entries[1].hls.map, d.playlist.entries[0].hls.map);
}

#[test]
fn endlist_positions_and_dangling_rules() {
    // ENDLIST after the last segment: no dangling warning (VOD shape).
    let d = must_parse("#EXTM3U\n#EXTINF:1,\na.ts\n#EXT-X-ENDLIST\n");
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    assert!(d.playlist.end_list);

    // Segment-tag metadata with no following segment: dangling (honest).
    let d = must_parse("#EXTM3U\na.ts\n#EXT-X-BYTERANGE:10\n");
    assert_eq!(d.warnings, vec![ParseWarning::DanglingEntry { line: 3 }]);

    // ENDLIST is not entry metadata: never dangles on its own.
    let d = must_parse("#EXTM3U\n#EXT-X-ENDLIST\n");
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    assert!(d.playlist.end_list);
    assert!(d.playlist.is_empty());

    // Junk on ENDLIST is damage.
    let d = must_parse("#EXTM3U\n#EXT-X-ENDLIST:no-really\n");
    assert_eq!(d.warnings, vec![ParseWarning::BadExtXTag { line: 2 }]);
    assert!(!d.playlist.end_list);
}

#[test]
fn quoted_attribute_contents_survive_awkward_chars() {
    // Commas, '@', '=' and a URI query inside quoted strings.
    let src = "#EXTM3U\n\
               #EXT-X-KEY:METHOD=AES-128,URI=\"https://k.example/x?a=1,2&b=@3\"\n\
               a.ts\n";
    let d = must_parse(src);
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    let Some(key) = d.playlist.entries[0].hls.key.as_ref() else {
        unreachable!("key must parse")
    };
    assert_eq!(key.uri, "https://k.example/x?a=1,2&b=@3");
}

#[test]
fn write_hls_canonical_golden_pin() {
    let mut pl = Playlist::new();
    let mut e = Entry::new("media1.ts");
    e.duration = Some(4.0);
    e.hls.key = Some(HlsKey {
        method: EncryptionMethod::Aes128,
        uri: "https://priv.example.com/key.php?r=52".into(),
        iv: Some(vec![0x9c, 0x7d, 0xb8, 0x77]),
        keyformat: None,
        keyformat_versions: None,
    });
    e.hls.map = Some(HlsMap {
        uri: "init.mp4".into(),
        byte_range: None,
    });
    e.hls.byte_range = Some(ByteRange {
        length: 8084,
        offset: Some(0),
    });
    pl.push(e);

    let mut e = Entry::new("media2.ts");
    e.duration = Some(4.0);
    e.hls.discontinuity = true;
    // Same key/map as the previous entry: delta encoding must NOT repeat.
    e.hls.key = Some(HlsKey {
        method: EncryptionMethod::Aes128,
        uri: "https://priv.example.com/key.php?r=52".into(),
        iv: Some(vec![0x9c, 0x7d, 0xb8, 0x77]),
        keyformat: None,
        keyformat_versions: None,
    });
    e.hls.map = Some(HlsMap {
        uri: "init.mp4".into(),
        byte_range: None,
    });
    e.hls.byte_range = Some(ByteRange {
        length: 7708,
        offset: Some(8084),
    });
    pl.push(e);

    let mut e = Entry::new("media3.ts");
    e.duration = Some(4.0);
    // Key dropped: the writer must emit an explicit METHOD=NONE.
    e.hls.map = Some(HlsMap {
        uri: "init2.mp4".into(),
        byte_range: Some(ByteRange {
            length: 739,
            offset: Some(0),
        }),
    });
    pl.push(e);
    pl.end_list = true;

    assert_eq!(
        write(&pl),
        concat!(
            "#EXTM3U\n",
            "#EXT-X-KEY:METHOD=AES-128,URI=\"https://priv.example.com/key.php?r=52\",IV=0x9c7db877\n",
            "#EXT-X-MAP:URI=\"init.mp4\"\n",
            "#EXT-X-BYTERANGE:8084@0\n",
            "#EXTINF:4\n",
            "media1.ts\n",
            "#EXT-X-DISCONTINUITY\n",
            "#EXT-X-BYTERANGE:7708@8084\n",
            "#EXTINF:4\n",
            "media2.ts\n",
            "#EXT-X-KEY:METHOD=NONE\n",
            "#EXT-X-MAP:URI=\"init2.mp4\",BYTERANGE=\"739@0\"\n",
            "#EXTINF:4\n",
            "media3.ts\n",
            "#EXT-X-ENDLIST\n",
        )
    );
    // Roundtrip.
    assert_roundtrip(&pl);
}

#[test]
fn hls_dirty_corpus_roundtrips() {
    let dynamic = {
        let attrs: String = (0..100).map(|i| format!("A{i}=\"v{i}\",")).collect();
        format!("#EXT-X-KEY:METHOD=AES-128,URI=\"k\",{attrs}OTHER=1\nx.ts\n")
    };
    let corpus = [
        // Key at EOF with no following segment: dangling, state dropped.
        "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"k\"\n",
        // Interleaved unknown EXT-X tags and EXTINF: directives order kept.
        "#EXTM3U\n#EXT-X-VERSION:3\n#EXTINF:1,\n#EXT-X-CUE:in=1\na.ts\n",
        // Discontinuity between segments with no EXTINF at all.
        "a.ts\n#EXT-X-DISCONTINUITY\nb.ts\n",
        // ENDLIST mid-file (weird but tolerated).
        "#EXT-X-ENDLIST\na.ts\n",
        // Quoted string containing colons and slashes.
        "#EXT-X-MAP:URI=\"http://h/i.mp4?x=1:y\"\na.m4s\n",
        // Hex with mixed case and 0X prefix.
        "#EXT-X-KEY:METHOD=AES-128,URI=\"k\",IV=0XaB\na.ts\n",
        // Enumerated value that looks numeric is an *integer* value, not
        // enumerated — badly-typed for METHOD, so the line is damage
        // (honest typing; the entry survives without a key).
        "#EXT-X-KEY:METHOD=1234,URI=\"k\"\na.ts\n",
        // Empty quoted URI is a legal quoted string.
        "#EXT-X-MAP:URI=\"\"\na.m4s\n",
        // BYTERANGE before EXTINF (order within a segment is free).
        "#EXTM3U\n#EXT-X-BYTERANGE:5@1\n#EXTINF:2.5,\na.ts\n",
        // Lone "#EXT-X-" prefix.
        "#EXT-X-\n",
        "#EXT-X-:\n",
        // Extremely long attribute list (one line, still under the cap).
        dynamic.as_str(),
    ];
    for src in corpus {
        let first = must_parse(src);
        let rewritten = write(&first.playlist);
        let second = must_parse(&rewritten);
        assert_eq!(second.playlist, first.playlist, "roundtrip for {src:?}");
    }
}

// ------------------------------------------------------------ URI joining

#[test]
fn resolve_uri_rfc3986_normal_examples() {
    // The full §5.4.1 matrix, base = "http://a/b/c/d;p?q".
    let base = "http://a/b/c/d;p?q";
    let cases = [
        ("g:h", "g:h"),
        ("g", "http://a/b/c/g"),
        ("./g", "http://a/b/c/g"),
        ("g/", "http://a/b/c/g/"),
        ("/g", "http://a/g"),
        ("//g", "http://g"),
        ("?y", "http://a/b/c/d;p?y"),
        ("g?y", "http://a/b/c/g?y"),
        ("#s", "http://a/b/c/d;p?q#s"),
        ("g#s", "http://a/b/c/g#s"),
        ("g?y#s", "http://a/b/c/g?y#s"),
        (";x", "http://a/b/c/;x"),
        ("g;x", "http://a/b/c/g;x"),
        ("g;x?y#s", "http://a/b/c/g;x?y#s"),
        ("", "http://a/b/c/d;p?q"),
        (".", "http://a/b/c/"),
        ("./", "http://a/b/c/"),
        ("..", "http://a/b/"),
        ("../", "http://a/b/"),
        ("../g", "http://a/b/g"),
        ("../..", "http://a/"),
        ("../../", "http://a/"),
        ("../../g", "http://a/g"),
    ];
    for (reference, want) in cases {
        assert_eq!(resolve_uri(base, reference), want, "ref {reference:?}");
    }
}

#[test]
fn resolve_uri_rfc3986_abnormal_examples() {
    let base = "http://a/b/c/d;p?q";
    let cases = [
        ("../../../g", "http://a/g"),
        ("../../../../g", "http://a/g"),
        ("/./g", "http://a/g"),
        ("/../g", "http://a/g"),
        ("g.", "http://a/b/c/g."),
        (".g", "http://a/b/c/.g"),
        ("g..", "http://a/b/c/g.."),
        ("..g", "http://a/b/c/..g"),
        ("./../g", "http://a/b/g"),
        ("./g/.", "http://a/b/c/g/"),
        ("g/./h", "http://a/b/c/g/h"),
        ("g/../h", "http://a/b/c/h"),
        ("g;x=1/./y", "http://a/b/c/g;x=1/y"),
        ("g;x=1/../y", "http://a/b/c/y"),
        ("g?y/./x", "http://a/b/c/g?y/./x"),
        ("g?y/../x", "http://a/b/c/g?y/../x"),
        ("g#s/./x", "http://a/b/c/g#s/./x"),
        ("g#s/../x", "http://a/b/c/g#s/../x"),
        ("http:g", "http:g"),
    ];
    for (reference, want) in cases {
        assert_eq!(resolve_uri(base, reference), want, "ref {reference:?}");
    }
}

#[test]
fn resolve_uri_hls_shaped_cases() {
    // The card's named cases plus real HLS shapes.
    let base = "https://cdn.example/live/stream.m3u8";
    assert_eq!(
        resolve_uri(base, "../seg/00001.ts"),
        "https://cdn.example/seg/00001.ts"
    );
    assert_eq!(
        resolve_uri(base, "./seg0.ts"),
        "https://cdn.example/live/seg0.ts"
    );
    assert_eq!(
        resolve_uri(base, "seg0.ts"),
        "https://cdn.example/live/seg0.ts"
    );
    assert_eq!(
        resolve_uri(base, "https://other.example/absolutely.ts"),
        "https://other.example/absolutely.ts",
        "absolute reference wins verbatim"
    );
    assert_eq!(
        resolve_uri(base, "sub/dir/seg.m4s"),
        "https://cdn.example/live/sub/dir/seg.m4s"
    );
    assert_eq!(
        resolve_uri(base, "init.mp4?f=1#frag"),
        "https://cdn.example/live/init.mp4?f=1#frag"
    );
    // Root-relative against an http base: RFC meaning (server root).
    assert_eq!(
        resolve_uri(base, "/media/x.ts"),
        "https://cdn.example/media/x.ts"
    );
    // Scheme-relative.
    assert_eq!(
        resolve_uri(base, "//mirror.example/x.ts"),
        "https://mirror.example/x.ts"
    );
    // Query/fragment on the base never survive a non-empty reference.
    assert_eq!(
        resolve_uri("https://h/p/l.m3u8?token=1#x", "s.ts"),
        "https://h/p/s.ts"
    );
    assert_eq!(
        resolve_uri("https://h/p/l.m3u8?token=1#x", ""),
        "https://h/p/l.m3u8?token=1"
    );
}

#[test]
fn resolve_uri_windows_posix_and_local_bases() {
    // Windows drive paths: verbatim, separators never rewritten.
    for win in [r"C:\Music\a.mp3", "C:/Music/a.mp3", r"z:\rel"] {
        assert_eq!(resolve_uri("http://h/d/l.m3u", win), win, "case {win}");
    }
    // UNC paths: verbatim.
    assert_eq!(
        resolve_uri("http://h/d/l.m3u", r"\\server\share\a.mp3"),
        r"\\server\share\a.mp3"
    );
    // Local absolute base (POSIX).
    assert_eq!(
        resolve_uri("/home/u/music/l.m3u", "song.mp3"),
        "/home/u/music/song.mp3"
    );
    assert_eq!(
        resolve_uri("/home/u/music/l.m3u", "../video/v.mkv"),
        "/home/u/video/v.mkv"
    );
    assert_eq!(resolve_uri("/l.m3u", "s.mp3"), "/s.mp3");
    // Local relative base.
    assert_eq!(resolve_uri("music/l.m3u", "s.mp3"), "music/s.mp3");
    assert_eq!(resolve_uri("music/l.m3u", "../s.mp3"), "s.mp3");
    assert_eq!(resolve_uri("l.m3u", "s.mp3"), "s.mp3");
    // ".." cannot climb above a local root.
    assert_eq!(resolve_uri("/a/l.m3u", "../../s.mp3"), "/s.mp3");
    // http base + POSIX-absolute reference: root-relative join (the RFC's
    // meaning of a leading '/'), documented in the module docs.
    assert_eq!(
        resolve_uri("http://h/d/l.m3u", "/usr/x.mp3"),
        "http://h/usr/x.mp3"
    );
    // An absolute reference wins even against a local base.
    assert_eq!(resolve_uri("/a/l.m3u", "http://x/y.mp3"), "http://x/y.mp3");
}

#[test]
fn resolve_uri_on_parsed_ffmpeg_playlists() {
    // End-to-end: resolve every segment of the fMP4 case against its
    // playlist URL shape.
    let d = must_parse(FMP4);
    let base = "https://cdn.example/vod/fmp4/pl.m3u8";
    assert_eq!(
        resolve_uri(base, &d.playlist.entries[0].location),
        "https://cdn.example/vod/fmp4/pl0.m4s"
    );
    let Some(map) = d.playlist.entries[0].hls.map.as_ref() else {
        unreachable!("fMP4 entry carries a MAP")
    };
    assert_eq!(
        resolve_uri(base, &map.uri),
        "https://cdn.example/vod/fmp4/init.mp4"
    );
    let d = must_parse(SINGLE_FILE);
    assert_eq!(
        resolve_uri("http://h/s/x.m3u8", &d.playlist.entries[0].location),
        "http://h/s/pl.ts"
    );
}

// ---------------------------------------------------------------- fuzz-lite

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

/// Attribute-parser-steering bytes (extends 4.D1's set with quote, '@',
/// 'X', hex letters).
const INTERESTING: &[u8] = b"#:,\r\n\"@=/EXTXIGM0123-.xXaAfF \t";

/// A soup of HLS-shaped lines: real tag skeletons with randomized
/// attribute junk spliced in. Every generated input is bounded well under
/// 2 KiB (the fuzz budget is <200 KiB total across the whole run).
fn hls_soup(rng: &mut SplitMix64) -> Vec<u8> {
    const TEMPLATES: &[&str] = &[
        "#EXTM3U",
        "#EXT-X-VERSION:3",
        "#EXT-X-TARGETDURATION:2",
        "#EXT-X-MEDIA-SEQUENCE:0",
        "#EXT-X-PLAYLIST-TYPE:VOD",
        "#EXT-X-DISCONTINUITY",
        "#EXT-X-ENDLIST",
        "#EXT-X-BYTERANGE:8084@0",
        "#EXT-X-BYTERANGE:",
        "#EXT-X-KEY:METHOD=AES-128,URI=\"https://k.example/key?r=52\"",
        "#EXT-X-KEY:METHOD=NONE",
        "#EXT-X-KEY:",
        "#EXT-X-MAP:URI=\"init.mp4\"",
        "#EXT-X-MAP:URI=\"i\",BYTERANGE=\"739@0\"",
        "#EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"skd://k\",IV=0x9c7db8778570d05c",
        "#EXTINF:2.043356,",
        "#EXTINF:-1,live",
        "pl0.ts",
        "media1.m4s",
        "#EXT-X-PROGRAM-DATE-TIME:2026-01-01T00:00:00.000+00:00",
        "#EXT-X-UNKNOWN-TAG:whatever=1",
    ];
    let lines = 1 + rng.below(24) as usize;
    let mut out = Vec::new();
    for _ in 0..lines {
        let t = TEMPLATES[(rng.below(TEMPLATES.len() as u64)) as usize];
        out.extend_from_slice(t.as_bytes());
        // Splice 0-3 random interesting bytes into the line.
        for _ in 0..rng.below(4) {
            let pos = rng.below(out.len() as u64 + 1) as usize;
            out.insert(
                pos,
                INTERESTING[(rng.below(INTERESTING.len() as u64)) as usize],
            );
        }
        out.push(if rng.below(4) == 0 { b'\r' } else { b'\n' });
    }
    out
}

#[test]
fn fuzz_lite_hls_soup_and_mutations_no_panic() {
    let mut rng = SplitMix64::new(0x5E0_1A05_EED0_0001); // "5.O1a" seed
    let mut parsed_count = 0usize;
    let mut hls_fields_seen = 0usize;
    for i in 0..10_000 {
        let buf = if i % 2 == 0 {
            hls_soup(&mut rng)
        } else {
            // Mutations of the real ffmpeg single-file playlist.
            let mut buf = SINGLE_FILE.as_bytes().to_vec();
            for _ in 0..(1 + rng.below(8)) {
                if buf.is_empty() {
                    break;
                }
                match rng.below(3) {
                    0 => {
                        let j = rng.below(buf.len() as u64) as usize;
                        buf[j] = INTERESTING[(rng.below(INTERESTING.len() as u64)) as usize];
                    }
                    1 => {
                        buf.remove(rng.below(buf.len() as u64) as usize);
                    }
                    _ => {
                        buf.insert(rng.below(buf.len() as u64 + 1) as usize, rng.byte());
                    }
                }
            }
            buf
        };
        assert!(buf.len() < 8192, "fuzz inputs stay small (D-1 spirit)");
        if let Ok(first) = parse_bytes(&buf) {
            parsed_count += 1;
            hls_fields_seen += usize::from(first.playlist.end_list);
            hls_fields_seen += first
                .playlist
                .entries
                .iter()
                .filter(|e| {
                    e.hls.byte_range.is_some() || e.hls.key.is_some() || e.hls.map.is_some()
                })
                .count();
            // Roundtrip law holds for every input that parses.
            let rewritten = write(&first.playlist);
            let second = parse_detailed(&rewritten)
                .unwrap_or_else(|_| unreachable!("fuzz roundtrip re-parse failed"));
            assert_eq!(second.playlist, first.playlist);
        }
        // Err results (UTF-8 / line cap) are fine: typed, no panic.
    }
    assert!(parsed_count > 100, "fuzz too weak: {parsed_count} parses");
    assert!(
        hls_fields_seen > 100,
        "HLS fields never exercised: {hls_fields_seen}"
    );
}

#[test]
fn fuzz_lite_resolve_uri_no_panic_and_total() {
    // resolve_uri is total: any &str pair either returns or... it always
    // returns, never panics, never allocates unboundedly.
    let mut rng = SplitMix64::new(0x5E0_1A05_EED0_0002);
    const BASES: &[&str] = &[
        "http://a/b/c/d;p?q",
        "https://cdn.example/live/stream.m3u8",
        "/home/u/music/list.m3u",
        "music/list.m3u",
        "list.m3u",
        "http://h/",
        "http://h",
        "file:///x/y.m3u",
        "",
        "//host/p",
        "C:\\Music\\list.m3u",
    ];
    let mut count = 0usize;
    for _ in 0..5_000 {
        let base = BASES[(rng.below(BASES.len() as u64)) as usize];
        // Random reference: mix of soup bytes and interesting fragments.
        let mut reference = String::new();
        for _ in 0..rng.below(6) {
            let frag = [
                "../",
                "./",
                "/",
                "//",
                "?q=1",
                "#f",
                "seg.ts",
                "http://x/",
                "C:\\d\\f",
                "\\\\unc\\share",
                "a/../b",
                "g;x",
                "..",
                ".",
                "",
                "0x",
            ][(rng.below(16)) as usize];
            reference.push_str(frag);
        }
        let resolved = resolve_uri(base, &reference);
        assert!(
            resolved.len() < base.len() + reference.len() + 8,
            "bounded output"
        );
        // Absolute-form fixed point: resolving the result against the
        // same base again is a no-op — but only when the base is itself
        // absolute (a relative base re-merges; that is POSIX-honest, not
        // a bug). Checked only for scheme'd or root-absolute bases.
        let absolute_base = base.contains("://") || base.starts_with('/');
        if absolute_base {
            assert_eq!(
                resolve_uri(base, &resolved),
                resolved,
                "fixed point broken: {resolved:?} <- base {base:?} ref {reference:?}"
            );
        }
        count += 1;
    }
    assert_eq!(count, 5_000);
}
