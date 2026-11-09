//! MKV extras wiring suite (issue #34): full-file hand-built vectors
//! that pin the **open() wiring** — `Tags`/`Attachments`/`Chapters`
//! parsed and retained on the demuxer at `start()`, `Info.Title` and
//! `TrackUID` retained on their models, and the chapter seek-target
//! surface — plus an **ffmpeg-generated differential lane** (real
//! `-metadata title=`, a real attached TTF font, a JPEG cover and
//! ffmetadata chapters) asserted through the wired getters.
//!
//! # What the hand-built vectors pin that the 5.D2 suites could not
//!
//! The 5.D1 walk stopped at the first cluster once `Cues` was known
//! and *skipped* the extras; these fixtures place `Cues` **before**
//! the clusters and the extras **after** them (ffmpeg's own layout),
//! so any regression to the early break or the parse-and-skip arm
//! fails here. The unknown-size `Tags` vector pins the streaming
//! shape end to end (parsed at open, playback still reaches EOS).

use std::path::PathBuf;
use std::process::Command;

mod common;

use common::open_via_input;
use demux::avi::test_support::mem_stream;
use demux::mkv::attachments::{self, FileData};
use demux::mkv::chapters::{self, ChapterTarget};
use demux::mkv::ebml;
use demux::mkv::tags;
use demux::mkv::MkvDemuxer;
use media_types::time::Stamp;
use media_types::TrackId;
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;

// ---------------------------------------------------------------------------
// EBML byte-builder helpers (the mkv_editions_tags.rs vocabulary)
// ---------------------------------------------------------------------------

/// The minimal big-endian bytes of an element id (marker form).
fn id_bytes(id: u32) -> Vec<u8> {
    let be = id.to_be_bytes();
    let first = be.iter().position(|&b| b != 0).unwrap_or(3);
    be[first..].to_vec()
}

/// The EBML size vint for `n` (shortest encoding, marker bit set).
fn size_vint(n: u64) -> Vec<u8> {
    for len in 1usize..=8 {
        let max_val = (1u64 << (7 * len)) - 1;
        if n <= max_val {
            let mut out = n.to_be_bytes()[8 - len..].to_vec();
            out[0] |= 1u8 << (8 - len);
            return out;
        }
    }
    unreachable!("u64 fits the 8-octet vint")
}

/// One element: id + size + payload.
fn el(id: u32, payload: &[u8]) -> Vec<u8> {
    let mut out = id_bytes(id);
    out.extend(size_vint(payload.len() as u64));
    out.extend_from_slice(payload);
    out
}

/// A uint element (minimal-width big-endian payload).
fn el_uint(id: u32, v: u64) -> Vec<u8> {
    let be = v.to_be_bytes();
    let first = be.iter().position(|&b| b != 0).unwrap_or(7);
    el(id, &be[first..])
}

/// A string element.
fn el_str(id: u32, s: &str) -> Vec<u8> {
    el(id, s.as_bytes())
}

/// Concatenate element buffers.
fn cat(parts: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    for p in parts {
        out.extend_from_slice(p);
    }
    out
}

/// A minimal `EBML` header for DocType `matroska`.
fn ebml_header() -> Vec<u8> {
    cat(&[
        el(0x4286, &[1]), // EBMLVersion
        el(0x42F7, &[1]), // EBMLReadVersion
        el(0x42F2, &[4]), // EBMLMaxIDLength
        el(0x42F3, &[8]), // EBMLMaxSizeLength
        el_str(0x4282, "matroska"),
        el(0x4287, &[2]), // DocTypeVersion
        el(0x4285, &[2]), // DocTypeReadVersion
    ])
}

/// One `SimpleBlock`: single frame, keyframe, no lacing. The
/// relative timecode is the spec's **16-bit signed** field — two
/// big-endian bytes (the `mkv_fixtures.rs` block-body convention:
/// track byte, tc_hi, tc_lo, flags).
fn simple_block(track: u64, rel_tc: i64, payload: &[u8]) -> Vec<u8> {
    let tc = i16::try_from(rel_tc).unwrap_or_else(|_| {
        unreachable!("fixture contract: rel_tc {rel_tc} fits i16")
    });
    let mut body = vec![0x80 | (track as u8)];
    body.extend_from_slice(&tc.to_be_bytes());
    body.push(0x80);
    body.extend_from_slice(payload);
    el(ebml::ID_SIMPLE_BLOCK, &body)
}

/// A one-cluster-at-timecode block carrier (two single-frame blocks).
fn cluster(timecode: i64, blocks: &[Vec<u8>]) -> Vec<u8> {
    let mut body = el_uint(ebml::ID_TIMECODE, u64::try_from(timecode).unwrap_or(0));
    for block in blocks {
        body.extend_from_slice(block);
    }
    el(ebml::ID_CLUSTER, &body)
}

/// One audio `TrackEntry` with `TrackNumber`, `TrackType`, `CodecID`
/// and (when given) `TrackUID`.
fn track_entry(number: u64, uid: Option<u64>) -> Vec<u8> {
    let mut entry = el_uint(ebml::ID_TRACK_NUMBER, number);
    if let Some(uid) = uid {
        entry.extend(el_uint(ebml::ID_TRACK_UID, uid));
    }
    entry.extend(el_uint(ebml::ID_TRACK_TYPE, 2)); // audio
    entry.extend(el_str(ebml::ID_CODEC_ID, "A_OPUS"));
    el(ebml::ID_TRACK_ENTRY, &entry)
}

/// A `Cues` master with one point per (time, cluster-offset) pair
/// (offsets relative to the Segment payload start).
fn cues(points: &[(i64, u64)]) -> Vec<u8> {
    let mut body = Vec::new();
    for (time, pos) in points {
        let positions = cat(&[
            el_uint(ebml::ID_CUE_TRACK, 1),
            el_uint(ebml::ID_CUE_CLUSTER_POSITION, *pos),
        ]);
        body.extend(el(
            ebml::ID_CUE_POINT,
            &cat(&[
                el_uint(ebml::ID_CUE_TIME, u64::try_from(*time).unwrap_or(0)),
                el(ebml::ID_CUE_TRACK_POSITIONS, &positions),
            ]),
        ));
    }
    el(ebml::ID_CUES, &body)
}

/// A full hand-built file: EBML header + `Segment` over `children`
/// (the segment declares its payload size; children are placed in the
/// order given).
fn mkv_file(children: &[Vec<u8>]) -> Vec<u8> {
    let mut out = el(0x1A45DFA3, &ebml_header());
    let payload = cat(children);
    out.extend(id_bytes(ebml::ID_SEGMENT));
    out.extend(size_vint(payload.len() as u64));
    out.extend(payload);
    out
}

/// The `Info` master (scale 1 ms + duration + title when given).
fn info(duration_secs: Option<f64>, title: Option<&str>) -> Vec<u8> {
    let mut body = el_uint(ebml::ID_TIMESTAMP_SCALE, 1_000_000);
    if let Some(secs) = duration_secs {
        body.extend(el(ebml::ID_DURATION, &secs.to_f64_bytes()));
    }
    if let Some(title) = title {
        body.extend(el_str(ebml::ID_INFO_TITLE, title));
    }
    el(ebml::ID_INFO, &body)
}

/// f64 payload helper (the `Duration` element's own encoding).
trait F64Bytes {
    fn to_f64_bytes(&self) -> Vec<u8>;
}

impl F64Bytes for f64 {
    fn to_f64_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

/// Drain a started demuxer to EOS, counting packets.
fn drain(demuxer: &mut MkvDemuxer) -> u64 {
    let mut packets = 0u64;
    loop {
        match demuxer.next_event().must() {
            DemuxEvent::Packet(_) => packets += 1,
            DemuxEvent::EndOfStream => break,
            _ => {}
        }
    }
    packets
}

// ---------------------------------------------------------------------------
// The wiring vectors
// ---------------------------------------------------------------------------

/// The card's namesake fixture: every extra retained through `start()`
/// with `Cues` before the clusters and the extras trailing them (the
/// exact shape the 5.D1 early break lost).
#[test]
fn tags_attachments_chapters_wire_through_start_retained() {
    // -- the extras' payloads (built first so the trailing placement
    //    is explicit in the segment order below) ---------------------
    // Tags: one global record (ARTIST + a nested sub-tag pair) and one
    // record scoped at TrackUID 77 (the join the retention enables).
    let global_pair = cat(&[
        el_str(tags::ID_TAG_NAME, "ARTIST"),
        el_str(tags::ID_TAG_LANGUAGE, "eng"),
        el_str(tags::ID_TAG_STRING, "W3I Fixture Artist"),
    ]);
    let nested_child = cat(&[
        el_str(tags::ID_TAG_NAME, "SUBPIECE"),
        el_str(tags::ID_TAG_STRING, "nested value"),
    ]);
    let nested_parent = cat(&[
        el_str(tags::ID_TAG_NAME, "PARENT_PAIR"),
        el_str(tags::ID_TAG_STRING, "parent value"),
        el(tags::ID_SIMPLE_TAG, &nested_child),
    ]);
    let scoped_pair = cat(&[
        el_str(tags::ID_TAG_NAME, "DESCRIPTION"),
        el_str(tags::ID_TAG_STRING, "scoped at uid 77"),
    ]);
    let tags_element = el(
        tags::ID_TAGS,
        &cat(&[
            el(
                tags::ID_TAG,
                &cat(&[
                    el(tags::ID_SIMPLE_TAG, &global_pair),
                    el(tags::ID_SIMPLE_TAG, &nested_parent),
                ]),
            ),
            el(
                tags::ID_TAG,
                &cat(&[
                    el(
                        tags::ID_TARGETS,
                        &el_uint(tags::ID_TAG_TRACK_UID, 77),
                    ),
                    el(tags::ID_SIMPLE_TAG, &scoped_pair),
                ]),
            ),
        ]),
    );
    // Attachments: a font (the ASS stack's input, issue #17) and a cover.
    let font = el(
        attachments::ID_ATTACHED_FILE,
        &cat(&[
            el_str(attachments::ID_FILE_NAME, "FixtureFont.ttf"),
            el_str(attachments::ID_FILE_MIME_TYPE, "application/x-truetype-font"),
            el_uint(attachments::ID_FILE_UID, 9),
            el(attachments::ID_FILE_DATA, b"ttf-payload-bytes"),
        ]),
    );
    let cover = el(
        attachments::ID_ATTACHED_FILE,
        &cat(&[
            el_str(attachments::ID_FILE_NAME, "cover.jpg"),
            el_str(attachments::ID_FILE_MIME_TYPE, "image/jpeg"),
            el_uint(attachments::ID_FILE_UID, 10),
            el(attachments::ID_FILE_DATA, b"JFIF-cover-bytes"),
        ]),
    );
    let attachments_element = el(
        attachments::ID_ATTACHMENTS,
        &cat(&[font, cover]),
    );
    // Chapters: one default edition, two titled atoms with ns times.
    let display = |title: &str| {
        el(
            chapters::ID_CHAPTER_DISPLAY,
            &cat(&[
                el_str(chapters::ID_CHAP_STRING, title),
                el_str(chapters::ID_CHAP_LANGUAGE, "eng"),
            ]),
        )
    };
    let atom = |uid: u64, start_ns: u64, end_ns: Option<u64>, title: &str| {
        let mut body = el_uint(chapters::ID_CHAPTER_TIME_START, start_ns);
        if let Some(end_ns) = end_ns {
            body.extend(el_uint(chapters::ID_CHAPTER_TIME_END, end_ns));
        }
        body.extend(el_uint(chapters::ID_CHAPTER_UID, uid));
        body.extend(display(title));
        el(chapters::ID_CHAPTER_ATOM, &body)
    };
    let edition = el(
        chapters::ID_EDITION_ENTRY,
        &cat(&[
            el_uint(chapters::ID_EDITION_UID, 5),
            el(chapters::ID_EDITION_FLAG_DEFAULT, &[1]),
            atom(501, 0, Some(1_500_000_000), "First Chapter"),
            atom(502, 1_500_000_000, Some(3_900_000_000), "Second Chapter"),
        ]),
    );
    let chapters_element = el(chapters::ID_CHAPTERS, &edition);

    // -- the segment order: Info, Tracks, Cues, clusters, THEN extras --
    let info_element = info(Some(4.0), Some("Info Title"));
    let tracks_element = el(
        ebml::ID_TRACKS,
        &track_entry(1, Some(77)),
    );
    let cluster1 = cluster(0, &[simple_block(1, 0, b"pkt-one")]);
    let cluster2 = cluster(1000, &[simple_block(1, 500, b"pkt-two")]);
    // Cue cluster positions are relative to the Segment payload start;
    // the second point indexes cluster1, which sits after Info + Tracks
    // + the (two-point) Cues itself. The placeholder point below has
    // the same byte width as the real one because both positions land
    // in the one-byte uint range (pinned by the assert).
    let cues_element = cues(&[(0, 0), (1000, 0)]);
    let cue_pos = (info_element.len() + tracks_element.len() + cues_element.len()) as u64;
    assert!(cue_pos < 0x80, "fixture contract: one-byte cue position");
    let bytes = mkv_file(&[
        info_element,
        tracks_element,
        cues(&[(0, 0), (1000, cue_pos)]),
        cluster1,
        cluster2,
        tags_element,
        attachments_element,
        chapters_element,
    ]);

    let mut demuxer = MkvDemuxer::new(mem_stream(bytes.clone()).must()).must();
    let tracks = demuxer.start().must();
    assert_eq!(tracks.len(), 1);

    // -- Tags retained --------------------------------------------------
    let list = demuxer.tags();
    assert_eq!(list.tags.len(), 2, "both records retained");
    assert_eq!(
        list.global_text_pairs(),
        vec![
            ("ARTIST".to_string(), "W3I Fixture Artist".to_string()),
            ("PARENT_PAIR".to_string(), "parent value".to_string()),
        ]
    );
    // The nested SimpleTag survives the wiring (structure, not text
    // flattening — ffprobe's own shape).
    let parent = list.tags[0]
        .simple_tags
        .iter()
        .find(|t| t.name == "PARENT_PAIR")
        .must();
    assert_eq!(parent.children.len(), 1);
    assert_eq!(parent.children[0].name, "SUBPIECE");
    assert_eq!(parent.children[0].value.as_text(), Some("nested value"));
    // The track scope joins through the RETAINED TrackUID.
    assert_eq!(
        demuxer.track_uid(TrackId::new(0)),
        Some(77),
        "TrackUID retained on the track model"
    );
    assert_eq!(
        list.track_text_pairs(77),
        vec![("DESCRIPTION".to_string(), "scoped at uid 77".to_string())]
    );
    assert!(list.track_text_pairs(78).is_empty());

    // -- title: Info.Title wins over the tag scope -----------------------
    assert_eq!(demuxer.title(), Some("Info Title"));

    // -- Attachments retained -------------------------------------------
    let attachments = demuxer.attachments();
    assert_eq!(attachments.files.len(), 2);
    assert!(attachments.damage.is_empty());
    let by_name = |name: &str| {
        attachments
            .files
            .iter()
            .find(|f| f.name.as_deref() == Some(name))
            .unwrap_or_else(|| unreachable!("fixture contract: {name} attached"))
    };
    let got_font = by_name("FixtureFont.ttf");
    assert_eq!(
        got_font.mime_type.as_deref(),
        Some("application/x-truetype-font")
    );
    assert_eq!(got_font.uid, 9);
    assert_eq!(
        got_font.data,
        FileData::Inline(b"ttf-payload-bytes".to_vec()),
        "font bytes byte-exact on the model (issue #17's input seam)"
    );
    let art = attachments.cover_art().must();
    assert_eq!(art.name.as_deref(), Some("cover.jpg"));
    assert_eq!(art.data, FileData::Inline(b"JFIF-cover-bytes".to_vec()));

    // -- Chapters retained ------------------------------------------------
    let chapters = demuxer.chapters();
    assert_eq!(chapters.editions.len(), 1);
    let edition = &chapters.editions[0];
    assert_eq!(edition.uid, 5);
    assert!(edition.default);
    assert_eq!(edition.atoms.len(), 2);
    assert_eq!(edition.atoms[0].uid, 501);
    assert_eq!(edition.atoms[0].start_ns, 0);
    assert_eq!(edition.atoms[0].end_ns, Some(1_500_000_000));
    assert_eq!(edition.atoms[0].title(), Some("First Chapter"));
    assert_eq!(edition.atoms[1].start_ns, 1_500_000_000);
    assert_eq!(edition.atoms[1].title(), Some("Second Chapter"));
    assert_eq!(chapters.beyond_duration, 0, "both atoms inside 4 s");
    assert!(chapters.damage.is_empty());

    // -- the seek-target surface ------------------------------------------
    let targets = demuxer.chapter_targets();
    assert_eq!(targets.len(), 2);
    assert_eq!(
        targets,
        vec![
            ChapterTarget {
                uid: 501,
                title: Some("First Chapter".to_string()),
                span: media_types::time::TimeSpan {
                    start: Stamp::Present(media_types::time::Timestamp::new(
                        0,
                        demuxer.segment_info().time_base().must(),
                    )),
                    end: Stamp::Present(media_types::time::Timestamp::new(
                        1500,
                        demuxer.segment_info().time_base().must(),
                    )),
                },
            },
            ChapterTarget {
                uid: 502,
                title: Some("Second Chapter".to_string()),
                span: media_types::time::TimeSpan {
                    start: Stamp::Present(media_types::time::Timestamp::new(
                        1500,
                        demuxer.segment_info().time_base().must(),
                    )),
                    end: Stamp::Present(media_types::time::Timestamp::new(
                        3900,
                        demuxer.segment_info().time_base().must(),
                    )),
                },
            },
        ],
        "ns → scale ticks (1 ms scale: 1.5e9 ns = 1500 ticks)"
    );

    // -- the packet walk is untouched by the retained extras --------------
    assert_eq!(drain(&mut demuxer), 2, "both blocks still demux");
    assert_eq!(demuxer.damage_bytes(), 0);
}

/// The title seam's fallback: no `Info.Title` → the global `TITLE` tag
/// pair (the mkvtoolnix spelling, case-insensitive).
#[test]
fn title_falls_back_to_the_global_title_tag() {
    let pair = cat(&[
        el_str(tags::ID_TAG_NAME, "Title"),
        el_str(tags::ID_TAG_STRING, "Tag-Scope Title"),
    ]);
    let tags_element = el(
        tags::ID_TAGS,
        &el(tags::ID_TAG, &el(tags::ID_SIMPLE_TAG, &pair)),
    );
    let bytes = mkv_file(&[
        info(Some(1.0), None),
        el(ebml::ID_TRACKS, &track_entry(1, None)),
        cluster(0, &[simple_block(1, 0, b"x")]),
        tags_element,
    ]);
    let mut demuxer = MkvDemuxer::new(mem_stream(bytes).must()).must();
    demuxer.start().must();
    assert_eq!(demuxer.title(), Some("Tag-Scope Title"));

    // Neither surface declared → the honest absence.
    let bytes = mkv_file(&[
        info(Some(1.0), None),
        el(ebml::ID_TRACKS, &track_entry(1, None)),
        cluster(0, &[simple_block(1, 0, b"x")]),
    ]);
    let mut demuxer = MkvDemuxer::new(mem_stream(bytes).must()).must();
    demuxer.start().must();
    assert_eq!(demuxer.title(), None);
}

/// TrackUID retention is the stable identity: two fresh opens of the
/// same source agree per track handle (the playlist-reload contract).
#[test]
fn track_uids_are_stable_across_reopens() {
    let bytes = mkv_file(&[
        info(None, None),
        el(
            ebml::ID_TRACKS,
            &cat(&[track_entry(1, Some(0xAA)), track_entry(2, Some(0xBB))]),
        ),
        cluster(0, &[simple_block(1, 0, b"a"), simple_block(2, 0, b"b")]),
    ]);
    let mut uids: Vec<Vec<Option<u64>>> = Vec::new();
    for _ in 0..2 {
        let mut demuxer = MkvDemuxer::new(mem_stream(bytes.clone()).must()).must();
        let tracks = demuxer.start().must();
        assert_eq!(tracks.len(), 2);
        let table: Vec<Option<u64>> = (0..2)
            .map(|i| demuxer.track_uid(TrackId::new(i)))
            .collect();
        // The model and the accessor agree.
        assert_eq!(
            table,
            demuxer
                .track_entries()
                .iter()
                .map(|e| Some(e.uid))
                .collect::<Vec<_>>()
        );
        uids.push(table);
    }
    assert_eq!(uids[0], vec![Some(0xAA), Some(0xBB)]);
    assert_eq!(uids[0], uids[1], "identity stable across reloads");
    // Out-of-range handle: the honest None (0 = absent UID is Some(0)).
    let mut demuxer = MkvDemuxer::new(mem_stream(bytes).must()).must();
    demuxer.start().must();
    assert_eq!(demuxer.track_uid(TrackId::new(7)), None);
}

/// Multi-edition chapters: every edition retained, the seek targets
/// come from the default edition only, and hidden atoms stay on the
/// model without becoming targets.
#[test]
fn multi_edition_chapters_retain_all_and_target_the_default() {
    let display = |title: &str| {
        el(
            chapters::ID_CHAPTER_DISPLAY,
            &el_str(chapters::ID_CHAP_STRING, title),
        )
    };
    let atom = |uid: u64, start_ns: u64, hidden: bool, title: &str| {
        let mut body = el_uint(chapters::ID_CHAPTER_TIME_START, start_ns);
        body.extend(el_uint(chapters::ID_CHAPTER_UID, uid));
        if hidden {
            body.extend(el(chapters::ID_CHAPTER_FLAG_HIDDEN, &[1]));
        }
        body.extend(display(title));
        el(chapters::ID_CHAPTER_ATOM, &body)
    };
    let hidden_edition = el(
        chapters::ID_EDITION_ENTRY,
        &cat(&[
            el_uint(chapters::ID_EDITION_UID, 1),
            el(chapters::ID_EDITION_FLAG_HIDDEN, &[1]),
            atom(101, 0, false, "Hidden Cut"),
        ]),
    );
    let default_edition = el(
        chapters::ID_EDITION_ENTRY,
        &cat(&[
            el_uint(chapters::ID_EDITION_UID, 2),
            el(chapters::ID_EDITION_FLAG_DEFAULT, &[1]),
            atom(201, 0, false, "Theatrical"),
            atom(202, 2_000_000_000, true, "Deleted Scene (hidden atom)"),
            atom(203, 3_000_000_000, false, "Finale"),
        ]),
    );
    let plain_edition = el(
        chapters::ID_EDITION_ENTRY,
        &cat(&[
            el_uint(chapters::ID_EDITION_UID, 3),
            atom(301, 0, false, "Alternate"),
        ]),
    );
    let bytes = mkv_file(&[
        info(Some(5.0), None),
        el(ebml::ID_TRACKS, &track_entry(1, None)),
        cluster(0, &[simple_block(1, 0, b"x")]),
        el(
            chapters::ID_CHAPTERS,
            &cat(&[hidden_edition, default_edition, plain_edition]),
        ),
    ]);
    let mut demuxer = MkvDemuxer::new(mem_stream(bytes).must()).must();
    demuxer.start().must();
    let chapters = demuxer.chapters();
    assert_eq!(chapters.editions.len(), 3, "every edition retained");
    assert_eq!(chapters.editions[0].uid, 1);
    assert!(chapters.editions[0].hidden);
    assert!(chapters.editions[1].default);
    assert_eq!(chapters.editions[1].atoms.len(), 3);
    assert_eq!(chapters.editions[2].atoms.len(), 1);
    // The whole-model flatten crosses editions; targets do not.
    assert_eq!(chapters.atoms_flattened().len(), 5);
    let targets = demuxer.chapter_targets();
    assert_eq!(
        targets
            .iter()
            .map(|t| t.title.clone().unwrap_or_default())
            .collect::<Vec<_>>(),
        vec![
            "Theatrical".to_string(),
            "Finale".to_string(), // the hidden atom is not a target
        ],
        "targets = default edition, visible atoms only"
    );
    assert_eq!(targets[0].uid, 201);
    assert_eq!(targets[1].uid, 203);
}

/// The chapter seek targets seek: a target's span fed to `seek` lands
/// at-or-before its start (the keyframe-safe rule), never past it.
#[test]
fn chapter_targets_seek_lands_at_or_before() {
    // 1 ms scale; clusters at 0 and 1000 ticks; chapters at 0 and
    // 1500 ticks (1.5 s). Cues index both clusters.
    let info_element = info(Some(4.0), None);
    let tracks_element = el(ebml::ID_TRACKS, &track_entry(1, None));
    let cluster1 = cluster(0, &[simple_block(1, 0, b"a")]);
    let cluster2 = cluster(1000, &[simple_block(1, 0, b"b")]);
    let cues_pos = (info_element.len() + tracks_element.len()) as u64;
    let atom = |uid: u64, start_ns: u64, title: &str| {
        let mut body = el_uint(chapters::ID_CHAPTER_TIME_START, start_ns);
        body.extend(el_uint(chapters::ID_CHAPTER_UID, uid));
        body.extend(el(
            chapters::ID_CHAPTER_DISPLAY,
            &el_str(chapters::ID_CHAP_STRING, title),
        ));
        el(chapters::ID_CHAPTER_ATOM, &body)
    };
    let bytes = mkv_file(&[
        info_element,
        tracks_element,
        cues(&[(0, 0), (1000, cues_pos)]),
        cluster1,
        cluster2,
        el(
            chapters::ID_CHAPTERS,
            &el(
                chapters::ID_EDITION_ENTRY,
                &cat(&[atom(11, 0, "Zero"), atom(12, 1_500_000_000, "Mid")]),
            ),
        ),
    ]);
    let mut demuxer = MkvDemuxer::new(mem_stream(bytes).must()).must();
    demuxer.start().must();
    let targets = demuxer.chapter_targets();
    assert_eq!(targets.len(), 2);
    // Chapter "Mid" starts at 1500 ticks; the keyframe-safe landing is
    // the 1000-tick cluster, at-or-before the target and past the
    // previous chapter's start.
    let mid = &targets[1];
    let start_ticks = match &mid.span.start {
        Stamp::Present(ts) => ts.ticks,
        Stamp::Missing => unreachable!("a chapter target always starts"),
    };
    assert_eq!(start_ticks, 1500);
    let landed = demuxer.seek(mid.span).must();
    let landing_ticks = match &landed.start {
        Stamp::Present(ts) => ts.ticks,
        Stamp::Missing => unreachable!("seek lands on a present stamp"),
    };
    assert!(landing_ticks <= start_ticks, "never past the target");
    assert!(landing_ticks >= 1000, "past the previous chapter's start");
    assert_eq!(landing_ticks, 1000, "the cue-indexed landing");
    // Chapter "Zero" lands at 0 exactly.
    let zero = &targets[0];
    let landed = demuxer.seek(zero.span).must();
    assert_eq!(
        match &landed.start {
            Stamp::Present(ts) => ts.ticks,
            Stamp::Missing => unreachable!(),
        },
        0
    );
}

/// The streaming shape: an unknown-size `Tags` master (extends to the
/// stream end) parses at open AND the packet walk still reaches EOS
/// (the mid-playback skip fix).
#[test]
fn unknown_size_tags_master_parses_and_playback_reaches_eos() {
    let pair = cat(&[
        el_str(tags::ID_TAG_NAME, "ENCODER"),
        el_str(tags::ID_TAG_STRING, "streaming form"),
    ]);
    let tag_payload = el(
        tags::ID_TAG,
        &el(tags::ID_SIMPLE_TAG, &pair),
    );
    // The unknown-size master: id + 0xFF marker + payload (no size).
    let mut tags_element = id_bytes(tags::ID_TAGS);
    tags_element.push(0xFF);
    tags_element.extend(tag_payload);
    let bytes = mkv_file(&[
        info(None, None),
        el(ebml::ID_TRACKS, &track_entry(1, None)),
        cluster(0, &[simple_block(1, 0, b"a"), simple_block(1, 40, b"b")]),
        tags_element,
    ]);
    let mut demuxer = MkvDemuxer::new(mem_stream(bytes).must()).must();
    demuxer.start().must();
    assert_eq!(
        demuxer.tags().global_text("ENCODER"),
        Some("streaming form"),
        "the unknown-size master parsed at open"
    );
    assert_eq!(drain(&mut demuxer), 2, "playback reaches EOS past it");
}

/// The mid-cluster damage livelock (found by this suite's fixtures):
/// a block that fails to parse inside a declared cluster must cost
/// ONE `Recovered` (the cluster's remainder skipped, the walk moved
/// past it) and playback must still reach EOS past the trailing
/// extras — the window-leak regression this pins re-loaded the same
/// cluster forever, one `Recovered` per attempt, never EOS.
#[test]
fn damaged_block_recovers_once_and_still_reaches_eos() {
    // A Xiph-laced block head claiming 2 frames whose first is 200
    // bytes, with zero frame bytes following: the contained lie.
    let malformed = el(ebml::ID_SIMPLE_BLOCK, &[0x81, 0x00, 0x00, 0x82, 2, 200]);
    let pair = cat(&[
        el_str(tags::ID_TAG_NAME, "AFTER"),
        el_str(tags::ID_TAG_STRING, "trailing the damage"),
    ]);
    let tags_element = el(tags::ID_TAGS, &el(tags::ID_TAG, &el(tags::ID_SIMPLE_TAG, &pair)));
    let bytes = mkv_file(&[
        info(None, None),
        el(ebml::ID_TRACKS, &track_entry(1, None)),
        cluster(0, &[simple_block(1, 0, b"good")]),
        cluster(10, &[malformed]),
        tags_element,
    ]);
    let mut demuxer = MkvDemuxer::new(mem_stream(bytes).must()).must();
    demuxer.start().must();
    let mut packets = 0u64;
    let mut recoveries = 0u64;
    loop {
        match demuxer.next_event().must() {
            DemuxEvent::Packet(_) => packets += 1,
            DemuxEvent::Recovered { .. } => recoveries += 1,
            DemuxEvent::EndOfStream => break,
            _ => {}
        }
    }
    assert_eq!(packets, 1, "the good block before the damage demuxes");
    assert_eq!(recoveries, 1, "one contained recovery, never a loop");
    assert!(demuxer.damage_bytes() > 0, "the skipped remainder counted");
    // The extras trailing the damaged cluster were still retained.
    assert_eq!(
        demuxer.tags().global_text("AFTER"),
        Some("trailing the damage")
    );
}

/// Damage folding: a degenerate attachment (no `FileData`) records its
/// damage row on the list AND in the demuxer's honesty counter.
#[test]
fn degenerate_attachment_damage_folds_into_the_counter() {
    let broken = el(
        attachments::ID_ATTACHED_FILE,
        &el_str(attachments::ID_FILE_NAME, "no-data.bin"),
    );
    let good = el(
        attachments::ID_ATTACHED_FILE,
        &cat(&[
            el_str(attachments::ID_FILE_NAME, "ok.bin"),
            el(attachments::ID_FILE_DATA, b"fine"),
        ]),
    );
    let bytes = mkv_file(&[
        info(None, None),
        el(ebml::ID_TRACKS, &track_entry(1, None)),
        cluster(0, &[simple_block(1, 0, b"x")]),
        el(attachments::ID_ATTACHMENTS, &cat(&[broken, good])),
    ]);
    let mut demuxer = MkvDemuxer::new(mem_stream(bytes).must()).must();
    demuxer.start().must();
    let attachments = demuxer.attachments();
    assert_eq!(attachments.files.len(), 1, "the survivor is retained");
    assert_eq!(attachments.files[0].name.as_deref(), Some("ok.bin"));
    assert_eq!(attachments.damage.len(), 1);
    assert_eq!(attachments.damage[0].1, "attached-file-without-data");
    assert!(
        demuxer.damage_bytes() > 0,
        "the skip is counted in the honesty counter"
    );
    assert_eq!(drain(&mut demuxer), 1);
}

// ---------------------------------------------------------------------------
// The ffmpeg-generated lane (title + font + cover + chapters, wired)
// ---------------------------------------------------------------------------

/// A real system TTF for the font attachment (the ASS stack's input
/// seam, issue #17). Skips loudly when the sandbox lacks fonts.
fn system_font() -> Option<PathBuf> {
    for candidate in [
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Oblique.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationMono-Italic.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ] {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Some(path);
        }
    }
    eprintln!("SKIP: no system TTF for the font-attachment lane");
    None
}

#[test]
fn generated_lane_wires_title_font_cover_and_chapters() {
    let Some(font_path) = system_font() else {
        return;
    };
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        eprintln!("SKIP: ffmpeg absent (generated wiring lane)");
        return;
    }
    let dir = std::env::temp_dir().join("lumen-w3i-mkv-wiring");
    std::fs::create_dir_all(&dir).must();
    let cover = dir.join("w3i-cover.jpg");
    let font_copy = dir.join("w3i-font.ttf");
    let meta = dir.join("w3i-chapters.ffmeta");
    let out = dir.join("w3i-wiring.mkv");

    if !cover.exists() {
        let status = Command::new("ffmpeg")
            .args(["-y", "-v", "error", "-f", "lavfi", "-i"])
            .arg("testsrc=duration=0.5:size=96x64:rate=5")
            .args(["-frames:v", "1", "-c:v", "mjpeg", "-q:v", "9"])
            .arg(cover.to_str().unwrap_or("cover.jpg"))
            .status()
            .must();
        assert!(status.success(), "cover generation failed");
    }
    std::fs::copy(&font_path, &font_copy).must();
    std::fs::write(
        &meta,
        ";FFMETADATA1\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=250\ntitle=Generated First\n\
         [CHAPTER]\nTIMEBASE=1/1000\nSTART=250\nEND=500\ntitle=Generated Second\n",
    )
    .must();
    if !out.exists() {
        let status = Command::new("ffmpeg")
            .args(["-y", "-v", "error", "-f", "lavfi", "-i"])
            .arg("testsrc2=duration=0.5:size=96x64:rate=10")
            .args(["-f", "lavfi", "-i"])
            .arg("sine=duration=0.5:frequency=440")
            .args(["-i"])
            .arg(meta.to_str().unwrap_or("chapters.ffmeta"))
            .args(["-map", "0:v", "-map", "1:a", "-map_chapters", "2"])
            .args(["-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt", "yuv420p"])
            .args(["-c:a", "aac"])
            .args(["-metadata", "title=W3I Wiring Probe"])
            .args(["-attach"])
            .arg(font_copy.to_str().unwrap_or("font.ttf"))
            .args(["-metadata:s:t:0", "mimetype=application/x-truetype-font"])
            .args(["-metadata:s:t:0", "filename=GeneratedFont.ttf"])
            .args(["-attach"])
            .arg(cover.to_str().unwrap_or("cover.jpg"))
            .args(["-metadata:s:t:1", "mimetype=image/jpeg"])
            .args(["-metadata:s:t:1", "filename=cover.jpg"])
            .arg(out.to_str().unwrap_or("out.mkv"))
            .status()
            .must();
        assert!(
            status.success(),
            "ffmpeg wiring-lane generation failed"
        );
    }

    // ffprobe truth for the title.
    let probe = Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", "format_tags=title", "-of", "csv"])
        .arg(&out)
        .output()
        .must();
    let probe_title = String::from_utf8_lossy(&probe.stdout);
    assert!(probe_title.contains("W3I Wiring Probe"), "{probe_title}");

    let mut demuxer = MkvDemuxer::new(open_via_input(&out).must()).must();
    let tracks = demuxer.start().must();
    assert_eq!(tracks.len(), 2, "the generated lane is A/V");

    // The record's gap 1, closed: ffmpeg writes the container title
    // into Info.Title — retained and surfaced.
    assert_eq!(demuxer.title(), Some("W3I Wiring Probe"));

    // TrackUIDs: ffmpeg writes nonzero, distinct uids — the stable
    // identity seam, joined against the track-scoped tags.
    let uids: Vec<u64> = (0..2)
        .filter_map(|i| demuxer.track_uid(TrackId::new(i)))
        .collect();
    assert_eq!(uids.len(), 2);
    assert!(uids.iter().all(|&u| u != 0), "ffmpeg writes real uids");
    assert_ne!(uids[0], uids[1]);
    for &uid in &uids {
        let pairs = demuxer.tags().track_text_pairs(uid);
        assert!(
            pairs.iter().any(|(name, _)| name == "ENCODER"),
            "stream scope joins through uid {uid}: {pairs:?}"
        );
    }

    // The font: byte-exact on the model (the ASS stack's input).
    let font_bytes = std::fs::read(&font_copy).must();
    let attachments = demuxer.attachments();
    let font = attachments
        .files
        .iter()
        .find(|f| f.name.as_deref() == Some("GeneratedFont.ttf"))
        .unwrap_or_else(|| unreachable!("the font is attached"));
    assert_eq!(
        font.mime_type.as_deref(),
        Some("application/x-truetype-font")
    );
    match &font.data {
        FileData::Inline(bytes) => assert_eq!(bytes, &font_bytes, "font byte-exact"),
        other => unreachable!("a 250 KiB font is inline, got {other:?}"),
    }
    // The cover: picked as artwork, byte-exact.
    let cover_bytes = std::fs::read(&cover).must();
    let art = attachments.cover_art().must();
    assert_eq!(art.name.as_deref(), Some("cover.jpg"));
    match &art.data {
        FileData::Inline(bytes) => assert_eq!(bytes, &cover_bytes, "cover byte-exact"),
        other => unreachable!("a small cover is inline, got {other:?}"),
    }

    // Chapters: the ffmetadata times, as seek targets on the clock.
    let targets = demuxer.chapter_targets();
    assert_eq!(
        targets
            .iter()
            .map(|t| t.title.clone().unwrap_or_default())
            .collect::<Vec<_>>(),
        vec![
            "Generated First".to_string(),
            "Generated Second".to_string(),
        ]
    );
    assert_eq!(targets.len(), 2);
    let start_ticks = |t: &ChapterTarget| match &t.span.start {
        Stamp::Present(ts) => ts.ticks,
        Stamp::Missing => unreachable!("targets always start"),
    };
    assert_eq!(start_ticks(&targets[0]), 0, "0 ms");
    assert_eq!(start_ticks(&targets[1]), 250, "250 ms at the 1 ms scale");
    // And the walk still yields packets (the wiring changed nothing
    // about the payload path).
    assert!(drain(&mut demuxer) > 0);
    println!("PASS w3i-wiring.mkv: title + font + cover + 2 chapters wired");
}
