//! MKV editions + tags suite (card 5.D2b): hand-built spec fixtures
//! for every structural and error path, the **real-corpus
//! differential vs ffprobe** (tag names/values on format and stream
//! scopes — 0017-meta-bomb, 0014, 0003, 0007-chapter-bomb, 0063 and
//! the full mkv sweep), and **ffmpeg-generated** metadata/chapter
//! files (`-metadata title/artist/comment`, ffmetadata chapters).
//!
//! # The two scopes ffprobe exposes (the differential shape)
//!
//! - A `Tag` with an absent `Targets` or all-zero UIDs is **global** —
//!   ffprobe renders it as `format` metadata.
//! - A `Tag` with `TagTrackUID N` is stream metadata — ffprobe renders
//!   it on the stream whose `TrackUID` (0x73C5, the `TrackEntry`
//!   element — not the tags-zone `TagTrackUID` 0x63C5) is `N`.
//!
//! Two ffprobe-side names are **excluded** with reasons (recorded
//! gaps, not silent skips):
//! - `title` on the format side: ffmpeg's muxer writes the container
//!   title into the `Info` master's `Title` element (0x7BA9) — a 5.D1
//!   element this card's `SegmentInfo` does not retain (the generated
//!   differential pins exactly this).
//! - `language` on the stream side: ffprobe derives it from the
//!   `TrackEntry` `Language` element (5.D1 surface), not from `Tags`.

use std::path::PathBuf;
use std::process::Command;

use demux::avi::test_support::mem_stream;
use demux::mkv::ebml::{self, EbmlReader, ElementHeader, ID_SEGMENT};
use demux::mkv::editions::{self, EditionList};
use demux::mkv::header;
use demux::mkv::tags::{self, TagList, TagValue};
use media_types::EngineError;
use testkit::Must;

/// `TrackEntry` → `TrackUID` (0x73C5) — the track-side join key of
/// the differential (kept test-side: it is a 5.D1 element id the
/// tags/editions zones do not parse).
const ID_TRACK_UID: u32 = 0x73C5;
/// `TrackEntry` itself (0xAE, `ebml::ID_TRACK_ENTRY`).
const ID_TRACK_ENTRY: u32 = 0xAE;

// ---------------------------------------------------------------------------
// EBML byte-builder helpers (the hand-built spec fixtures)
// ---------------------------------------------------------------------------

/// The minimal big-endian bytes of an element id (ids are stored in
/// their shortest marker form — the lib's constants already are).
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
    // 8 octets always suffice for u64 (the marker eats one bit); the
    // loop's last arm returns first.
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

/// Open a hand-built buffer as an `EbmlReader` and parse the single
/// top-level element at offset 0 with the given parser shape (the
/// fixture IS the element — no segment wrapper needed).
fn open_tags(bytes: Vec<u8>) -> Result<TagList, EngineError> {
    let mut reader = EbmlReader::new(mem_stream(bytes).must()).must();
    let bound = reader.length();
    let element = ebml::parse_header_at(&mut reader, 0, bound).must().must();
    tags::parse_tags(&mut reader, &element, bound)
}

fn open_editions(bytes: Vec<u8>) -> Result<EditionList, EngineError> {
    let mut reader = EbmlReader::new(mem_stream(bytes).must()).must();
    let bound = reader.length();
    let element = ebml::parse_header_at(&mut reader, 0, bound).must().must();
    editions::parse_editions(&mut reader, &element, bound)
}

// ---------------------------------------------------------------------------
// Hand-built fixtures: tags
// ---------------------------------------------------------------------------

#[test]
fn global_and_track_scoped_tags_parse_with_languages() {
    // One global Tag (no Targets) + one track-scoped Tag with a
    // declared TargetTypeValue/TargetType, IETF language, TagDefault 0.
    let global_pair = cat(&[
        el_str(tags::ID_TAG_NAME, "TITLE_PIECE"),
        el_str(tags::ID_TAG_LANGUAGE, "eng"),
        el_uint(tags::ID_TAG_DEFAULT, 1),
        el_str(tags::ID_TAG_STRING, "global value"),
    ]);
    let targets = cat(&[
        el_uint(tags::ID_TARGET_TYPE_VALUE, 30),
        el_str(tags::ID_TARGET_TYPE, "ALBUM"),
        el_uint(tags::ID_TAG_TRACK_UID, 7),
    ]);
    let scoped_pair = cat(&[
        el_str(tags::ID_TAG_NAME, "ENCODER"),
        el_str(tags::ID_TAG_LANGUAGE, "ger"),
        el_str(tags::ID_TAG_LANGUAGE_IETF, "de-DE"),
        el_uint(tags::ID_TAG_DEFAULT, 0),
        el_str(tags::ID_TAG_STRING, "lumen fixture"),
    ]);
    let buffer = el(
        tags::ID_TAGS,
        &cat(&[
            el(tags::ID_TAG, &cat(&[el(tags::ID_SIMPLE_TAG, &global_pair)])),
            el(
                tags::ID_TAG,
                &cat(&[
                    el(tags::ID_TARGETS, &targets),
                    el(tags::ID_SIMPLE_TAG, &scoped_pair),
                ]),
            ),
        ]),
    );
    let list = open_tags(buffer).must();
    assert_eq!(list.tags.len(), 2);
    // The global record: absent Targets = the UID-0 global form.
    assert!(list.tags[0].targets.is_global());
    assert_eq!(
        list.global_text_pairs(),
        vec![("TITLE_PIECE".to_string(), "global value".to_string())]
    );
    // The scoped record: target facts recorded verbatim.
    let scoped = &list.tags[1];
    assert!(!scoped.targets.is_global());
    assert_eq!(scoped.targets.target_type_value, 30);
    assert_eq!(scoped.targets.target_type.as_deref(), Some("ALBUM"));
    assert_eq!(scoped.targets.track_uids, vec![7]);
    let simple = &scoped.simple_tags[0];
    assert_eq!(simple.name, "ENCODER");
    assert_eq!(simple.language, "ger");
    assert_eq!(simple.language_ietf.as_deref(), Some("de-DE"));
    assert!(!simple.default);
    assert_eq!(simple.value.as_text(), Some("lumen fixture"));
    // The differential accessors.
    assert_eq!(
        list.track_text_pairs(7),
        vec![("ENCODER".to_string(), "lumen fixture".to_string())]
    );
    assert!(list.track_text_pairs(8).is_empty());
}

#[test]
fn uid_zero_targets_are_the_global_form() {
    // Explicit all-zero UID lists = global (the spec's UID 0 rule —
    // what ffmpeg renders as format metadata).
    let targets = cat(&[
        el_uint(tags::ID_TAG_TRACK_UID, 0),
        el_uint(tags::ID_TAG_EDITION_UID, 0),
        el_uint(tags::ID_TAG_CHAPTER_UID, 0),
        el_uint(tags::ID_TAG_ATTACHMENT_UID, 0),
    ]);
    let pair = cat(&[
        el_str(tags::ID_TAG_NAME, "ENCODER"),
        el_str(tags::ID_TAG_STRING, "Lavf61.7.103"),
    ]);
    let buffer = el(
        tags::ID_TAGS,
        &el(
            tags::ID_TAG,
            &cat(&[
                el(tags::ID_TARGETS, &targets),
                el(tags::ID_SIMPLE_TAG, &pair),
            ]),
        ),
    );
    let list = open_tags(buffer).must();
    assert!(list.tags[0].targets.is_global());
    assert_eq!(
        list.global_text_pairs(),
        vec![("ENCODER".to_string(), "Lavf61.7.103".to_string())]
    );
}

#[test]
fn nested_simple_tags_and_binary_values_parse() {
    // A SimpleTag nesting a child (the spec's sub-tag form), a
    // TagBinary value, and NUL-padded text (ffmpeg's DURATION form).
    let child = el(
        tags::ID_SIMPLE_TAG,
        &cat(&[
            el_str(tags::ID_TAG_NAME, "SUB_PART"),
            el_str(tags::ID_TAG_STRING, "child value"),
        ]),
    );
    let binary_pair = cat(&[
        el_str(tags::ID_TAG_NAME, "COVER"),
        el(tags::ID_TAG_BINARY, &[0xDE, 0xAD, 0xBE, 0xEF]),
    ]);
    let padded_pair = cat(&[
        el_str(tags::ID_TAG_NAME, "DURATION"),
        el_str(tags::ID_TAG_STRING, "00:00:02.104000000\0"),
    ]);
    let buffer = el(
        tags::ID_TAGS,
        &el(
            tags::ID_TAG,
            &cat(&[
                el(
                    tags::ID_SIMPLE_TAG,
                    &cat(&[el_str(tags::ID_TAG_NAME, "PARENT"), child]),
                ),
                el(tags::ID_SIMPLE_TAG, &binary_pair),
                el(tags::ID_SIMPLE_TAG, &padded_pair),
            ]),
        ),
    );
    let list = open_tags(buffer).must();
    let tag = &list.tags[0];
    assert_eq!(tag.simple_tags.len(), 3);
    // Nested child recorded one level down.
    assert_eq!(tag.simple_tags[0].children.len(), 1);
    assert_eq!(tag.simple_tags[0].children[0].name, "SUB_PART");
    assert_eq!(
        tag.simple_tags[0].children[0].value.as_text(),
        Some("child value")
    );
    // Binary passes through raw and never surfaces as text.
    assert_eq!(
        tag.simple_tags[1].value,
        TagValue::Binary(vec![0xDE, 0xAD, 0xBE, 0xEF])
    );
    // NUL padding stripped exactly like ffmpeg's reader.
    assert_eq!(
        tag.simple_tags[2].value.as_text(),
        Some("00:00:02.104000000")
    );
    // Only text pairs shape the differential: PARENT is name-only,
    // COVER is binary, DURATION is text.
    assert_eq!(list.global_text_pairs().len(), 1);
}

#[test]
fn unknown_target_type_value_is_tolerantly_recorded() {
    // TargetTypeValue 99 is not in the spec's 10..=70 hierarchy —
    // the record (and its pairs) still parse; the level is carried
    // verbatim, never refused and never guessed.
    let targets = cat(&[el_uint(tags::ID_TARGET_TYPE_VALUE, 99)]);
    let pair = cat(&[
        el_str(tags::ID_TAG_NAME, "PRIVATE_LEVEL"),
        el_str(tags::ID_TAG_STRING, "kept"),
    ]);
    let buffer = el(
        tags::ID_TAGS,
        &el(
            tags::ID_TAG,
            &cat(&[
                el(tags::ID_TARGETS, &targets),
                el(tags::ID_SIMPLE_TAG, &pair),
            ]),
        ),
    );
    let list = open_tags(buffer).must();
    assert_eq!(list.tags[0].targets.target_type_value, 99);
    assert_eq!(
        list.global_text_pairs(),
        vec![("PRIVATE_LEVEL".to_string(), "kept".to_string())]
    );
}

#[test]
fn truncated_simple_tag_is_the_typed_truncation() {
    // A well-formed Tags element cut mid-TagString by the physical
    // stream end: the value read refuses typed (Corrupt::Truncated),
    // never a short string.
    let full = el(
        tags::ID_TAGS,
        &el(
            tags::ID_TAG,
            &el(
                tags::ID_SIMPLE_TAG,
                &cat(&[
                    el_str(tags::ID_TAG_NAME, "COMMENT"),
                    el_str(tags::ID_TAG_STRING, "0123456789abcdef"),
                ]),
            ),
        ),
    );
    // Cut inside the value payload (past the declared size vint).
    let cut_at = full.len() - 8;
    let err = open_tags(full[..cut_at].to_vec()).err().must();
    assert!(
        matches!(
            err.kind,
            media_types::ErrorKind::Corrupt {
                detail: media_types::error::CorruptDetail::Truncated { .. }
            }
        ),
        "typed truncation, got: {err}"
    );
    assert_eq!(err.context.code, "demux.mkv.truncated");
}

#[test]
fn over_deep_simple_tag_nesting_is_the_typed_structure_error() {
    // MAX_TAG_NESTING+1 levels of SimpleTag: the depth guard's typed
    // refusal (no unbounded recursion).
    let mut payload = el_str(tags::ID_TAG_NAME, "BOTTOM");
    for _ in 0..=tags::MAX_TAG_NESTING {
        payload = el(tags::ID_SIMPLE_TAG, &payload);
    }
    let buffer = el(tags::ID_TAGS, &el(tags::ID_TAG, &payload));
    let err = open_tags(buffer).err().must();
    assert!(
        matches!(
            err.kind,
            media_types::ErrorKind::Corrupt {
                detail: media_types::error::CorruptDetail::InvalidStructure { .. }
            }
        ),
        "typed structure error, got: {err}"
    );
    // The guard sits exactly at the ceiling: MAX levels parse (the
    // pair sits at the deepest nesting — surfaced by walking the
    // chain, since the differential accessor reads top-level only).
    let mut ok_payload = cat(&[
        el_str(tags::ID_TAG_NAME, "BOTTOM"),
        el_str(tags::ID_TAG_STRING, "at the limit"),
    ]);
    for _ in 1..=tags::MAX_TAG_NESTING {
        ok_payload = el(tags::ID_SIMPLE_TAG, &ok_payload);
    }
    let ok_buffer = el(tags::ID_TAGS, &el(tags::ID_TAG, &ok_payload));
    let list = open_tags(ok_buffer).must();
    let mut node = list.tags.first().must().simple_tags.first().must();
    let mut depth = 1usize;
    while !node.children.is_empty() {
        node = &node.children[0];
        depth += 1;
    }
    assert_eq!(depth, tags::MAX_TAG_NESTING);
    assert_eq!(
        node.text_pair(),
        Some(("BOTTOM", "at the limit")),
        "the deepest pair carries its value"
    );
}

#[test]
fn tag_bomb_ceilings_are_typed_resource_limits() {
    // Tag-count bomb: MAX_TAGS+1 records.
    let mut many = Vec::new();
    for _ in 0..=tags::MAX_TAGS {
        many.push(el(tags::ID_TAG, &[]));
    }
    let err = open_tags(el(tags::ID_TAGS, &cat(&many))).err().must();
    assert_eq!(err.context.code, "demux.mkv.tags.budget");
    assert!(matches!(
        err.kind,
        media_types::ErrorKind::ResourceLimit { .. }
    ));
    // Value-size bomb: one TagString declaring 2 MiB (over the
    // 1 MiB per-value ceiling) — refused before any read.
    let mut huge = id_bytes(tags::ID_TAG_STRING);
    huge.extend(size_vint(2 * 1024 * 1024));
    let bomb = el(
        tags::ID_TAGS,
        &el(
            tags::ID_TAG,
            &el(
                tags::ID_SIMPLE_TAG,
                &cat(&[el_str(tags::ID_TAG_NAME, "BIG"), huge]),
            ),
        ),
    );
    let err = open_tags(bomb).err().must();
    assert_eq!(err.context.code, "demux.mkv.tags.budget");
}

// ---------------------------------------------------------------------------
// Hand-built fixtures: editions
// ---------------------------------------------------------------------------

#[test]
fn edition_flags_and_chapter_uid_links_parse() {
    // One edition: UID + all three flags + two atoms, the second
    // nesting a sub-atom (the ordered-edition form). The non-UID
    // atom content (times, displays) is present-but-opaque — the
    // D2a seam — and must not disturb the UID walk.
    let atom1 = cat(&[
        el_uint(editions::ID_CHAPTER_UID, 101),
        el(0x91, &1u32.to_be_bytes()), // ChapterTimeStart (D2a's)
        el(0x92, &2u32.to_be_bytes()), // ChapterTimeEnd (D2a's)
    ]);
    let sub_atom = cat(&[el_uint(editions::ID_CHAPTER_UID, 203)]);
    let atom2 = cat(&[
        el_uint(editions::ID_CHAPTER_UID, 202),
        el(0x91, &3u32.to_be_bytes()),
        el(editions::ID_CHAPTER_ATOM, &sub_atom), // nested sub-chapter
    ]);
    let edition = cat(&[
        el_uint(editions::ID_EDITION_UID, 42),
        el_uint(editions::ID_EDITION_FLAG_HIDDEN, 1),
        el_uint(editions::ID_EDITION_FLAG_DEFAULT, 1),
        el_uint(editions::ID_EDITION_FLAG_ORDERED, 1),
        el(editions::ID_CHAPTER_ATOM, &atom1),
        el(editions::ID_CHAPTER_ATOM, &atom2),
    ]);
    let list = open_editions(el(
        editions::ID_CHAPTERS,
        &el(editions::ID_EDITION_ENTRY, &edition),
    ))
    .must();
    assert_eq!(list.editions.len(), 1);
    let e = &list.editions[0];
    assert_eq!(e.uid, 42);
    assert!(e.hidden && e.default && e.ordered);
    assert_eq!(e.atom_count, 3, "nested sub-atoms count");
    // The link seam: every UID, file order, nested included.
    assert_eq!(e.chapter_uids, vec![101, 202, 203]);
    assert_eq!(list.default_edition().must().uid, 42);
    assert_eq!(list.ordered_editions().len(), 1);
    assert_eq!(list.by_uid(42).must().atom_count, 3);
}

#[test]
fn edition_defaults_follow_the_spec_when_absent() {
    // No flags declared: default=0, hidden=0, ordered=0; the
    // selection still resolves (first visible).
    let atom = el(
        editions::ID_CHAPTER_ATOM,
        &el_uint(editions::ID_CHAPTER_UID, 1),
    );
    let buffer = el(
        editions::ID_CHAPTERS,
        &el(editions::ID_EDITION_ENTRY, &atom),
    );
    let list = open_editions(buffer).must();
    let e = &list.editions[0];
    assert_eq!(e.uid, 0, "UID absent = the spec default 0");
    assert!(!e.hidden && !e.default && !e.ordered);
    assert_eq!(e.chapter_uids, vec![1]);
    assert_eq!(list.default_edition().must().uid, 0);
}

#[test]
fn atom_nesting_depth_guard_is_typed() {
    // MAX_ATOM_NESTING+1 nested atoms: the typed refusal.
    let mut atom = el(
        editions::ID_CHAPTER_ATOM,
        &el_uint(editions::ID_CHAPTER_UID, 1),
    );
    for _ in 0..editions::MAX_ATOM_NESTING {
        atom = el(editions::ID_CHAPTER_ATOM, &atom);
    }
    let buffer = el(
        editions::ID_CHAPTERS,
        &el(editions::ID_EDITION_ENTRY, &atom),
    );
    let err = open_editions(buffer).err().must();
    assert!(
        matches!(
            err.kind,
            media_types::ErrorKind::Corrupt {
                detail: media_types::error::CorruptDetail::InvalidStructure { .. }
            }
        ),
        "typed structure error, got: {err}"
    );
}

#[test]
fn chapter_bomb_ceiling_is_the_typed_resource_limit() {
    // MAX_CHAPTER_ATOMS+1 empty atoms in one edition (the corpus
    // chapter-bomb shape, pushed past the ceiling).
    let mut atoms = Vec::new();
    for _ in 0..=editions::MAX_CHAPTER_ATOMS {
        atoms.push(el(editions::ID_CHAPTER_ATOM, &[]));
    }
    let err = open_editions(el(
        editions::ID_CHAPTERS,
        &el(editions::ID_EDITION_ENTRY, &cat(&atoms)),
    ))
    .err()
    .must();
    assert_eq!(err.context.code, "demux.mkv.editions.budget");
    assert!(matches!(
        err.kind,
        media_types::ErrorKind::ResourceLimit { .. }
    ));
}

// ---------------------------------------------------------------------------
// The corpus / generated differential vs ffprobe
// ---------------------------------------------------------------------------

fn corpus_dir() -> Option<PathBuf> {
    let dir = std::env::var("LUMEN_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/my-project/corpus-v4"));
    let media = dir.join("media");
    media.is_dir().then_some(media)
}

/// The segment's direct children (walked once per file; the Tags and
/// Chapters elements are handed to this card's parsers, Tracks to the
/// UID extractor — the seam the Lead merge wires into `MkvDemuxer`).
fn segment_children(reader: &mut EbmlReader) -> Result<Vec<ElementHeader>, EngineError> {
    let length = reader.length();
    let (_, header_end) = header::parse_ebml_header(reader, length)?;
    let mut pos = header_end;
    let mut segment: Option<ElementHeader> = None;
    while pos < length {
        let Some(el) = ebml::parse_header_at(reader, pos, length)? else {
            break;
        };
        if el.id == ID_SEGMENT {
            segment = Some(el);
            break;
        }
        let Some(end) = el.declared_end() else {
            return Err(demux::mkv::error::bad_structure(el.data_pos));
        };
        pos = end;
    }
    let Some(segment) = segment else {
        return Err(demux::mkv::error::not_matroska(&[]));
    };
    let end = segment.declared_end().unwrap_or(length).min(length);
    let mut out = Vec::new();
    let mut pos = segment.data_pos;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            break;
        };
        out.push(child);
        pos = child_end;
    }
    Ok(out)
}

/// The `TrackUID` of every `TrackEntry`, in table order (the join key
/// of the stream-scope differential).
fn track_uids(reader: &mut EbmlReader, tracks: &ElementHeader) -> Vec<u64> {
    let end = tracks.declared_end().unwrap_or(reader.length());
    let mut uids = Vec::new();
    let mut pos = tracks.data_pos;
    while pos < end {
        let Ok(Some(child)) = ebml::parse_header_at(reader, pos, end) else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            break;
        };
        if child.id == ID_TRACK_ENTRY {
            let mut uid = 0u64;
            let mut inner = child.data_pos;
            while inner < child_end {
                let Ok(Some(field)) = ebml::parse_header_at(reader, inner, child_end) else {
                    break;
                };
                let Some(field_end) = field.declared_end() else {
                    break;
                };
                if field.id == ID_TRACK_UID
                    && field.size <= 8
                    && let Ok(bytes) =
                        reader.read_exact_at(field.size as usize, field.data_pos, field_end)
                {
                    uid = ebml::uint_from_be(&bytes).unwrap_or(0);
                }
                inner = field_end;
            }
            uids.push(uid);
        }
        pos = child_end;
    }
    uids
}

/// Read one ffprobe JSON output.
fn ffprobe_json(path: &std::path::Path, entries: &str) -> String {
    let out = Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", entries, "-of", "json"])
        .arg(path)
        .output()
        .must();
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// An escape-aware JSON string reader (ffprobe emits raw UTF-8 for
/// non-ASCII; \" and \\ still occur).
fn read_json_string(bytes: &[u8], i: &mut usize) -> Option<String> {
    if *i >= bytes.len() || bytes[*i] != b'"' {
        return None;
    }
    *i += 1;
    let mut out = Vec::new();
    while *i < bytes.len() {
        match bytes[*i] {
            b'\\' => {
                *i += 1;
                if *i >= bytes.len() {
                    return None;
                }
                let c = match bytes[*i] {
                    b'n' => b'\n',
                    b't' => b'\t',
                    b'r' => b'\r',
                    other => other,
                };
                out.push(c);
            }
            b'"' => {
                *i += 1;
                return String::from_utf8(out).ok();
            }
            c => out.push(c),
        }
        *i += 1;
    }
    None
}

/// The `"key": { "k": "v", … }` object after `key` (one level).
fn json_object_after(text: &str, key: &str) -> Vec<(String, String)> {
    let pat = format!("\"{key}\":");
    let Some(at) = text.find(&pat) else {
        return Vec::new();
    };
    let bytes = text.as_bytes();
    let mut i = at + pat.len();
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() || bytes[i] != b'{' {
        return Vec::new();
    }
    i += 1;
    let mut pairs = Vec::new();
    loop {
        while i < bytes.len() && (bytes[i] == b',' || bytes[i].is_ascii_whitespace()) {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] == b'}' {
            break;
        }
        let Some(k) = read_json_string(bytes, &mut i) else {
            break;
        };
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b':' {
            break;
        }
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let Some(v) = read_json_string(bytes, &mut i) else {
            break;
        };
        pairs.push((k, v));
    }
    pairs
}

/// ffprobe's format-level tags (the global-tag truth).
fn ffprobe_format_tags(path: &std::path::Path) -> Vec<(String, String)> {
    let json = ffprobe_json(path, "format_tags");
    json_object_after(&json, "tags")
}

/// ffprobe's per-stream tags, in stream order (the track-scope truth).
fn ffprobe_stream_tags(path: &std::path::Path) -> Vec<Vec<(String, String)>> {
    // `stream_tags` alone omits the index; request both so each
    // stream chunk carries its position.
    let json = ffprobe_json(path, "stream=index:stream_tags");
    let mut streams = Vec::new();
    for chunk in json.split("\"index\":").skip(1) {
        streams.push(json_object_after(chunk, "tags"));
    }
    streams
}

/// Sorted pair sets for equality (mkv tag order is not ffprobe's).
fn sorted(pairs: &[(String, String)]) -> Vec<(String, String)> {
    let mut out = pairs.to_vec();
    out.sort();
    out
}

/// The three recorded-gap exclusions (pinned by name, reasoned
/// above): `title` on both scopes — ffmpeg's muxer writes the
/// container title to `Info.Title` (0x7BA9) and per-stream titles to
/// `TrackEntry.Name` (0x536E), both 5.D1 elements this card does not
/// retain — and `language` on streams (`TrackEntry.Language`).
fn is_recorded_gap(name: &str, scope: char) -> bool {
    match scope {
        'f' => name == "title", // Info.Title (5.D1 element, not retained)
        _ => name == "language" || name == "title", // TrackEntry elements
    }
}

fn parse_file_tags(path: &std::path::Path) -> (TagList, EditionList, Vec<u64>) {
    let stream = crate::open_source(path);
    let mut reader = EbmlReader::new(stream).must();
    let children = segment_children(&mut reader).must();
    let mut list = TagList::default();
    let mut editions = EditionList::default();
    let mut uids = Vec::new();
    let bound = reader.length();
    for child in &children {
        match child.id {
            tags::ID_TAGS => {
                let more = tags::parse_tags(&mut reader, child, bound).must();
                list.tags.extend(more.tags);
            }
            editions::ID_CHAPTERS => {
                editions = editions::parse_editions(&mut reader, child, bound).must();
            }
            ebml::ID_TRACKS => uids = track_uids(&mut reader, child),
            _ => {}
        }
    }
    (list, editions, uids)
}

/// Open through `input`'s FileAccess (the 2.A1+demux composition).
fn open_source(path: &std::path::Path) -> Box<dyn plugin_api::SourceStream> {
    use plugin_api::Access;
    let uri =
        media_types::ids::SourceUri::parse(&format!("file://{}", path.to_str().unwrap_or("")))
            .must();
    let access = input::FileAccess::new();
    access.open(&uri).must()
}

#[test]
fn corpus_tags_match_ffprobe_format_and_stream_scopes() {
    let Some(media) = corpus_dir() else {
        eprintln!("SKIP: corpus-v4 absent (LUMEN_CORPUS unset, /tmp/my-project/corpus-v4 missing)");
        return;
    };
    let mut files: Vec<PathBuf> = std::fs::read_dir(&media)
        .must()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "mkv"))
        .collect();
    files.sort();
    let mut rows = 0usize;
    let mut checked = 0usize;
    for path in &files {
        let (list, editions, uids) = parse_file_tags(path);
        // Every corpus Tags element parses (30 of 31 mkv lanes carry
        // one — the exception is the chapter-bomb, below).
        let truth = ffprobe_format_tags(path);
        let truth_streams = ffprobe_stream_tags(path);
        if truth.is_empty() && list.tags.is_empty() && editions.editions.is_empty() {
            continue;
        }
        checked += 1;
        // Global scope: exact set equality minus the recorded gap.
        let mine_global: Vec<(String, String)> = list
            .global_text_pairs()
            .into_iter()
            .filter(|(n, _)| !is_recorded_gap(n, 'f'))
            .collect();
        let ff_global: Vec<(String, String)> = truth
            .into_iter()
            .filter(|(n, _)| !is_recorded_gap(n, 'f'))
            .collect();
        assert_eq!(
            sorted(&mine_global),
            sorted(&ff_global),
            "global tags differ on {}",
            path.display()
        );
        // Stream scope: per-track set equality minus the recorded gap.
        assert_eq!(
            uids.len(),
            truth_streams.len(),
            "track-uid table vs ffprobe stream count on {}",
            path.display()
        );
        for (i, stream_tags) in truth_streams.iter().enumerate() {
            let mine: Vec<(String, String)> = list
                .track_text_pairs(uids[i])
                .into_iter()
                .filter(|(n, _)| !is_recorded_gap(n, 's'))
                .collect();
            let ff: Vec<(String, String)> = stream_tags
                .iter()
                .filter(|(n, _)| !is_recorded_gap(n, 's'))
                .cloned()
                .collect();
            assert_eq!(
                sorted(&mine),
                sorted(&ff),
                "stream {i} tags differ on {}",
                path.display()
            );
        }
        rows += 1;
        println!(
            "TAGS DIFF: {} — {} global, {} tracks, {} tag records",
            path.file_name().must().to_string_lossy(),
            mine_global.len(),
            uids.len(),
            list.tags.len()
        );
    }
    // The corpus carries real tags (30 lanes measured) — a regression
    // to zero-checks would be a silently-skipped differential.
    assert!(checked >= 25, "only {checked} corpus lanes checked");
    println!("CORPUS TAGS DIFFERENTIAL: {rows} rows, all global+stream scopes matched");
}

#[test]
fn corpus_editions_match_ffprobe_chapter_counts() {
    let Some(media) = corpus_dir() else {
        eprintln!("SKIP: corpus-v4 absent (LUMEN_CORPUS unset, /tmp/my-project/corpus-v4 missing)");
        return;
    };
    // The two known chapter carriers (verified structurally before
    // this suite ran: one default edition each, flat atoms).
    let expectations = [
        (
            "0007-chapter-bomb.mkv",
            1usize,    // editions
            2580usize, // chapter UIDs == ffprobe chapter count
        ),
        ("0063-multi-av.mkv", 1, 10),
    ];
    for (name, edition_count, uid_count) in expectations {
        let path = media.join(name);
        if !path.exists() {
            eprintln!("SKIP: {name} absent from corpus");
            continue;
        }
        let (_, editions, _) = parse_file_tags(&path);
        assert_eq!(editions.editions.len(), edition_count, "{name}");
        let e = editions.default_edition().must();
        assert!(
            e.default,
            "{name}: ffmpeg writes FlagDefault on its editions"
        );
        assert!(
            !e.ordered && !e.hidden,
            "{name}: ffmpeg writes no ordered/hidden"
        );
        assert_eq!(e.chapter_uids.len(), uid_count, "{name}");
        // The UID links are the ffprobe chapter count exactly.
        let out = Command::new("ffprobe")
            .args(["-v", "error", "-show_chapters", "-of", "csv"])
            .arg(&path)
            .output()
            .must();
        let chapters = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| l.starts_with("chapter,"))
            .count();
        assert_eq!(e.chapter_uids.len(), chapters, "{name}: ffprobe chapters");
        println!(
            "EDITIONS DIFF: {name} — {edition_count} edition(s), {} chapter-UID links == ffprobe",
            e.chapter_uids.len()
        );
    }
    // The sweep: every corpus mkv's Chapters element (when present)
    // parses without error; files without chapters yield empty lists.
    let mut files: Vec<PathBuf> = std::fs::read_dir(&media)
        .must()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "mkv"))
        .collect();
    files.sort();
    let mut with_chapters = 0usize;
    for path in &files {
        let (_, editions, _) = parse_file_tags(path);
        if !editions.editions.is_empty() {
            with_chapters += 1;
        }
    }
    assert_eq!(with_chapters, 2, "exactly the two known chapter lanes");
}

#[test]
fn generated_metadata_files_match_ffprobe() {
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        eprintln!("SKIP: ffmpeg absent (generated-file differential)");
        return;
    }
    let dir = std::env::temp_dir().join("lumen-d2b-generated");
    std::fs::create_dir_all(&dir).must();
    let meta = dir.join("meta_title.mkv");
    let chapters = dir.join("meta_chapters.mkv");
    let ffmeta = dir.join("chapters.ffmetadata");

    // 1. -metadata title/artist/comment (the card's generated lane).
    let ok = Command::new("ffmpeg")
        .args(["-y", "-v", "error", "-f", "lavfi", "-i"])
        .arg("testsrc2=duration=0.4:size=64x48:rate=10")
        .args(["-f", "lavfi", "-i"])
        .arg("sine=duration=0.4:frequency=440")
        .args([
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
        ])
        .args(["-c:a", "aac"])
        .args(["-metadata", "title=D2b Title Probe"])
        .args(["-metadata", "artist=Lumen Agent"])
        .args(["-metadata", "COMMENT=corpus differential row"])
        .arg(&meta)
        .output()
        .must();
    assert!(
        ok.status.success(),
        "ffmpeg generate: {}",
        String::from_utf8_lossy(&ok.stderr)
    );

    // 2. ffmetadata chapters (editions + atoms from ffmpeg's muxer).
    std::fs::write(
        &ffmeta,
        "FFMETADATA1\ntitle=d2b chapters\n\n\
         [CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=100\ntitle=First\n\n\
         [CHAPTER]\nTIMEBASE=1/1000\nSTART=100\nEND=400\ntitle=Second\n",
    )
    .must();
    let ok = Command::new("ffmpeg")
        .args(["-y", "-v", "error", "-i"])
        .arg(&meta)
        .args(["-f", "ffmetadata", "-i"])
        .arg(&ffmeta)
        .args(["-map", "0", "-map_metadata", "1", "-c", "copy"])
        .arg(&chapters)
        .output()
        .must();
    assert!(
        ok.status.success(),
        "ffmpeg chapters: {}",
        String::from_utf8_lossy(&ok.stderr)
    );

    // --- Tags differential on the generated title lane.
    let (list, _, uids) = parse_file_tags(&meta);
    let global = list.global_text_pairs();
    let truth = ffprobe_format_tags(&meta);
    // ffmpeg's muxer splits metadata: ARTIST/COMMENT/ENCODER land in
    // the global Tags element; `title` lands in Info.Title (0x7BA9,
    // the recorded 5.D1 gap ffprobe still surfaces).
    for (name, value) in [
        ("ARTIST", "Lumen Agent"),
        ("COMMENT", "corpus differential row"),
    ] {
        assert!(
            global.iter().any(|(n, v)| n == name && v == value),
            "generated global pair {name} missing: {global:?}"
        );
    }
    assert!(
        truth
            .iter()
            .any(|(n, v)| n == "title" && v == "D2b Title Probe"),
        "ffprobe surfaces the Info.Title gap: {truth:?}"
    );
    let mine: Vec<_> = global
        .into_iter()
        .filter(|(n, _)| !is_recorded_gap(n, 'f'))
        .collect();
    let ff: Vec<_> = truth
        .into_iter()
        .filter(|(n, _)| !is_recorded_gap(n, 'f'))
        .collect();
    assert_eq!(sorted(&mine), sorted(&ff), "generated global set equality");
    // Stream scope: ENCODER + DURATION per track (NUL-stripped).
    for (i, uid) in uids.iter().enumerate() {
        let mine = list.track_text_pairs(*uid);
        let ff: Vec<_> = ffprobe_stream_tags(&meta)[i]
            .iter()
            .filter(|(n, _)| !is_recorded_gap(n, 's'))
            .cloned()
            .collect();
        assert_eq!(sorted(&mine), sorted(&ff), "generated stream {i} scope");
    }
    println!(
        "GENERATED TAGS DIFF: meta_title.mkv — global + {} stream scopes matched",
        uids.len()
    );

    // --- Editions differential on the generated chapters lane.
    let (_, editions, _) = parse_file_tags(&chapters);
    assert_eq!(editions.editions.len(), 1);
    let e = editions.default_edition().must();
    assert!(e.default);
    assert_eq!(e.atom_count, 2);
    assert_eq!(e.chapter_uids.len(), 2);
    let out = Command::new("ffprobe")
        .args(["-v", "error", "-show_chapters", "-of", "csv"])
        .arg(&chapters)
        .output()
        .must();
    let count = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.starts_with("chapter,"))
        .count();
    assert_eq!(e.chapter_uids.len(), count, "ffprobe chapters on generated");
    println!("GENERATED EDITIONS DIFF: meta_chapters.mkv — 1 edition, {count} chapter links");
}
