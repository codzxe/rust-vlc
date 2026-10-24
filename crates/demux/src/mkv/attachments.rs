//! Matroska `Attachments` — the attached-files element (card 5.D2a).
//!
//! Card 5.D1's top-level walk parse-and-skipped `Attachments` with
//! every other extra; this module parses it into the typed
//! [`AttachedFile`] records a UI needs (cover art, fonts, arbitrary
//! companion files), following the landed core's rules:
//!
//! - **Declared sizes are hints**: every child read is clamped to the
//!   `Attachments` master's effective end and the stream's physical
//!   end; a `FileData` whose declared extent overruns the master is
//!   the typed truncation (the contained-lie discipline), a
//!   physically short stream is the typed truncation at its own
//!   boundary.
//! - **Bounded memory (MP §12)**: per-file payloads at or under
//!   [`MAX_ATTACHMENT_BYTES`] are read inline; larger declared
//!   payloads stay the **non-inline note** ([`FileData::NonInline`] —
//!   position + length, never read by this parse) so a bomb-sized
//!   attachment cannot become a bomb-sized allocation. The file count
//!   itself is ceiling-checked ([`MAX_ATTACHMENTS`]); over-ceiling
//!   files are skipped with damage, the rest survive.
//! - **Advisory text is never fabricated**: `FileName` /
//!   `FileMimeType` / `FileDescription` over the text ceiling or with
//!   invalid UTF-8 are treated as absent (the file and its data
//!   survive — the fields are labels, not structure).
//! - A `AttachedFile` without any `FileData` element is degenerate:
//!   skipped with damage (the spec makes the element mandatory).
//!
//! # Exposure seam (the card's "info/attachment surface")
//!
//! [`MkvDemuxer::attachments`] is the typed getter. The 5.D1 struct
//! has no attachments field (this card's mod.rs grant is two
//! `pub mod` lines only, file-disjoint with the parallel 5.D2b
//! agent), so the getter parses on demand: a positional top-level
//! scan ([`scan_segment_for`]) over the demuxer's own reader and
//! segment bounds. Every read is positional, so the demuxer's packet
//! walk is untouched — call once after `start()` and keep the result;
//! repeated calls re-read.

use media_types::EngineError;

use crate::mkv::MkvDemuxer;
use crate::mkv::ebml::{self, EbmlReader, ElementHeader};
use crate::mkv::error;
use crate::mkv::header::MAX_TEXT_FIELD;

// ---- Element IDs (on-disk form, length-marker bits intact) ----------

/// `Attachments` — the top-level attached-files master.
pub const ID_ATTACHMENTS: u32 = 0x1941_A469;
/// `AttachedFile` — one attached file record.
pub const ID_ATTACHED_FILE: u32 = 0x61A7;
/// `FileDescription` — a free-form description (advisory).
pub const ID_FILE_DESCRIPTION: u32 = 0x467E;
/// `FileName` — the file's name (advisory label).
pub const ID_FILE_NAME: u32 = 0x466E;
/// `FileMimeType` — the MIME type (advisory label).
pub const ID_FILE_MIME_TYPE: u32 = 0x4660;
/// `FileData` — the payload bytes.
pub const ID_FILE_DATA: u32 = 0x465C;
/// `FileUID` — the unique id (default 0 when absent).
pub const ID_FILE_UID: u32 = 0x46AE;

/// Maximum attached files trusted into the list
/// `[PROPOSAL: 1024 — real files carry < 10 (covers, fonts); a file
/// past this is a metadata bomb and is skipped with damage, the rest
/// survive]`.
pub const MAX_ATTACHMENTS: usize = 1024;

/// Per-file inline ceiling — payloads at or under this are read into
/// memory; larger ones stay the non-inline note
/// `[PROPOSAL: 32 MiB — the landed core's MAX_BLOCK_BYTES; cover art
/// and fonts are KBs]`.
pub const MAX_ATTACHMENT_BYTES: u64 = 32 * 1024 * 1024;

/// Where an attached file's bytes live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileData {
    /// Read into memory (the common cover-art / font case).
    Inline(Vec<u8>),
    /// An over-ceiling declared payload: its absolute position and
    /// length, **never read by this parse** — the non-inline note.
    /// The consumer fetches it on demand through its own bounded
    /// access; a bomb-sized attachment cannot become a bomb-sized
    /// allocation here.
    NonInline {
        /// Absolute position of the payload.
        offset: u64,
        /// Declared payload length.
        len: u64,
    },
}

/// One attached file (cover art, font, companion document).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedFile {
    /// `FileDescription`, when declared and valid UTF-8.
    pub description: Option<String>,
    /// `FileName`, when declared and valid UTF-8.
    pub name: Option<String>,
    /// `FileMimeType`, when declared and valid UTF-8.
    pub mime_type: Option<String>,
    /// `FileUID` (0 when absent).
    pub uid: u64,
    /// The payload: inline bytes or the non-inline positional note.
    pub data: FileData,
}

/// Attachment damage rows: (bytes, reason) — the landed core's
/// `TrackDamage` shape.
pub type AttachmentDamage = Vec<(u64, &'static str)>;

/// The parsed `Attachments` element: the files plus the per-skip
/// damage notes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttachmentList {
    /// The attached files, in declared order.
    pub files: Vec<AttachedFile>,
    /// Damage rows (bytes skipped, reason).
    pub damage: AttachmentDamage,
}

/// The `Attachments` parse. An unknown-size `Attachments` clamps to
/// `bound` (the segment end — masters may legally carry the unknown
/// size).
///
/// # Errors
/// [`error::truncated`] when an `AttachedFile`'s declared extent
/// overruns the `Attachments` master (the contained lie) or the
/// stream physically ends inside a payload; [`error::bad_structure`]
/// for unknown-size inner elements and out-of-grammar fixed-width
/// fields; [`error::bad_vint`] never leaves this walk unpositioned.
pub fn parse_attachments(
    reader: &mut EbmlReader,
    attachments: &ElementHeader,
    bound: u64,
) -> Result<AttachmentList, EngineError> {
    let end = attachments.declared_end().unwrap_or(bound).min(bound);
    let mut list = AttachmentList::default();
    let mut pos = attachments.data_pos;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            // Unknown size is legal only for the top-level master —
            // inside it the streaming form has no meaning.
            return Err(error::bad_structure(child.data_pos));
        };
        if child.id == ID_ATTACHED_FILE {
            if child_end > end {
                // The record claims bytes past the master: the
                // contained lie, typed at the master's end.
                return Err(error::truncated(child_end, end));
            }
            if list.files.len() >= MAX_ATTACHMENTS {
                list.damage
                    .push((child_end - pos, "attachment-over-ceiling"));
            } else {
                match parse_attached_file(reader, &child, child_end)? {
                    Some(file) => list.files.push(file),
                    None => list
                        .damage
                        .push((child_end - pos, "attached-file-without-data")),
                }
            }
        }
        // Unknown `Attachments` children: skipped by declared size.
        pos = child_end;
    }
    Ok(list)
}

/// One `AttachedFile` parse. `Ok(None)` = degenerate (no `FileData`
/// — skip with damage); `Ok(Some(_))` = a usable record.
fn parse_attached_file(
    reader: &mut EbmlReader,
    file: &ElementHeader,
    end: u64,
) -> Result<Option<AttachedFile>, EngineError> {
    let mut description: Option<String> = None;
    let mut name: Option<String> = None;
    let mut mime_type: Option<String> = None;
    let mut uid = 0u64;
    let mut data: Option<FileData> = None;

    let mut pos = file.data_pos;
    while pos < end {
        let Some(child) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let Some(child_end) = child.declared_end() else {
            return Err(error::bad_structure(child.data_pos));
        };
        if child_end > end {
            // A field claiming bytes past its `AttachedFile`: the
            // contained lie, typed at the record's end.
            return Err(error::truncated(child_end, end));
        }
        let len = usize::try_from(child.size).map_err(|_| error::bad_structure(child.data_pos))?;
        match child.id {
            ID_FILE_DESCRIPTION | ID_FILE_NAME | ID_FILE_MIME_TYPE => {
                // Advisory labels: tolerant both ways (over-ceiling or
                // invalid UTF-8 → absent, never fatal, never
                // fabricated).
                if len <= MAX_TEXT_FIELD as usize {
                    let bytes = reader.read_prefix_at(len, child.data_pos, child_end)?;
                    let text = ebml::utf8_from(&bytes);
                    match child.id {
                        ID_FILE_DESCRIPTION => description = text,
                        ID_FILE_NAME => name = text,
                        _ => mime_type = text,
                    }
                }
            }
            ID_FILE_UID => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
                if let Some(value) = ebml::uint_from_be(&bytes) {
                    uid = value;
                }
            }
            ID_FILE_DATA => {
                if child.size <= MAX_ATTACHMENT_BYTES {
                    // The exact read: a short stream is the typed
                    // truncation at the payload's own boundary (the
                    // card's truncated-AttachmentFile-data path).
                    let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
                    data = Some(FileData::Inline(bytes));
                } else {
                    // Over-ceiling: the non-inline positional note —
                    // the bytes are never read (bounded memory).
                    data = Some(FileData::NonInline {
                        offset: child.data_pos,
                        len: child.size,
                    });
                }
            }
            _ => {
                // Unknown `AttachedFile` children: skipped by
                // declared size (FileReferral and friends).
            }
        }
        pos = child_end;
    }
    // `FileData` is mandatory per spec: a record without it is
    // degenerate (the data IS the attachment).
    let Some(data) = data else {
        return Ok(None);
    };
    Ok(Some(AttachedFile {
        description,
        name,
        mime_type,
        uid,
        data,
    }))
}

/// Scan a segment's top-level children for every element with `want`'s
/// id, header reads only (positional — safe against the demuxer's
/// packet walk). Unknown-size children end the scan: only the target
/// master itself may legally carry the unknown size in this scope, and
/// it extends to the segment end by definition (the landed `open()`
/// walk's policy, shared with the chapters getter and reusable by the
/// editions/tags card).
///
/// # Errors
/// Typed per the ebml layer (bad vint / truncation at a header).
pub(crate) fn scan_segment_for(
    reader: &mut EbmlReader,
    start: u64,
    end: u64,
    want: u32,
) -> Result<Vec<ElementHeader>, EngineError> {
    let mut found = Vec::new();
    let mut pos = start;
    while pos < end {
        let Some(el) = ebml::parse_header_at(reader, pos, end)? else {
            break;
        };
        let element_end = match el.declared_end() {
            Some(element_end) => element_end.min(end),
            None => {
                // Unknown size: the target master extends to the
                // segment end (nothing can follow it); anything else
                // cannot be skipped past — the scan ends here.
                if el.id == want {
                    found.push(el);
                }
                break;
            }
        };
        if el.id == want {
            found.push(el);
        }
        pos = element_end;
    }
    Ok(found)
}

impl MkvDemuxer {
    /// The segment's attached files (cover art, fonts, companions):
    /// the typed getter over the `Attachments` element.
    ///
    /// Parses on demand — a positional top-level scan over this
    /// demuxer's own reader and segment bounds (the 5.D1 struct has
    /// no attachments field; this card's mod.rs grant is the two
    /// module lines). Every read is positional, so the packet walk is
    /// untouched; keep the result rather than re-calling on a hot
    /// path.
    ///
    /// # Errors
    /// [`error::not_started`] before `start`; typed per
    /// [`parse_attachments`] afterwards.
    pub fn attachments(&mut self) -> Result<AttachmentList, EngineError> {
        if !self.started {
            return Err(error::not_started());
        }
        let mut list = AttachmentList::default();
        for el in scan_segment_for(
            &mut self.reader,
            self.segment_data_start,
            self.segment_end,
            ID_ATTACHMENTS,
        )? {
            let parsed = parse_attachments(&mut self.reader, &el, self.segment_end)?;
            list.files.extend(parsed.files);
            list.damage.extend(parsed.damage);
        }
        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::avi::test_support::mem_stream;
    use testkit::Must;

    /// The on-disk id vint (marker bits intact, minimal width).
    fn id_bytes(id: u32) -> Vec<u8> {
        // The marker bits of the id's first byte fix its width:
        // 0x10..0x1F → 4 bytes, 0x20..0x3F → 3, 0x40..0x7F → 2,
        // 0x80..0xFF → 1 (the u32's leading zeros count them).
        let width = 4 - id.leading_zeros() / 8;
        id.to_be_bytes()[4 - width as usize..].to_vec()
    }

    /// The minimal size vint for `len`.
    fn size_bytes(len: usize) -> Vec<u8> {
        if len < 0x80 {
            vec![0x80 | len as u8]
        } else if len < 0x4000 {
            vec![0x40 | (len >> 8) as u8, (len & 0xFF) as u8]
        } else {
            // The 8-byte form (0x01 marker): any width ≥ minimal is
            // legal EBML.
            let bytes = (len as u64).to_be_bytes();
            let mut out = vec![0x01];
            out.extend_from_slice(&bytes[1..]);
            out
        }
    }

    /// One element: id vint + size vint + payload.
    fn el(id: u32, payload: &[u8]) -> Vec<u8> {
        let mut out = id_bytes(id);
        out.extend(size_bytes(payload.len()));
        out.extend_from_slice(payload);
        out
    }

    /// `Attachments` master bytes over `payload` (the element at 0);
    /// `unknown` writes the unknown-size master (extends to the
    /// caller's bound — the streaming form).
    fn attachments_element(payload: &[u8]) -> Vec<u8> {
        let mut out = id_bytes(ID_ATTACHMENTS);
        out.extend(size_bytes(payload.len()));
        out.extend_from_slice(payload);
        out
    }

    /// The unknown-size `Attachments` master over `payload`.
    fn attachments_element_unknown(payload: &[u8]) -> Vec<u8> {
        let mut out = id_bytes(ID_ATTACHMENTS);
        out.push(0xFF);
        out.extend_from_slice(payload);
        out
    }

    /// Parse an `Attachments` element carried by a fresh in-memory
    /// stream of `bytes` (the element starts at 0).
    fn parse(bytes: &[u8], bound: u64) -> Result<AttachmentList, EngineError> {
        let mut reader = EbmlReader::new(mem_stream(bytes.to_vec()).must()).must();
        let header = ebml::parse_header_at(&mut reader, 0, bound).must().must();
        parse_attachments(&mut reader, &header, bound)
    }

    #[test]
    fn attached_files_parse_the_full_record() {
        let file = [
            el(ID_FILE_DESCRIPTION, b"front cover"),
            el(ID_FILE_NAME, b"cover.jpg"),
            el(ID_FILE_MIME_TYPE, b"image/jpeg"),
            el(ID_FILE_UID, &[0, 0, 0, 0, 0, 0, 0, 7]),
            el(ID_FILE_DATA, b"JFIF-bytes"),
        ]
        .concat();
        let bytes = attachments_element(&el(ID_ATTACHED_FILE, &file));
        let list = parse(&bytes, bytes.len() as u64).must();
        assert_eq!(list.files.len(), 1);
        let got = &list.files[0];
        assert_eq!(got.description.as_deref(), Some("front cover"));
        assert_eq!(got.name.as_deref(), Some("cover.jpg"));
        assert_eq!(got.mime_type.as_deref(), Some("image/jpeg"));
        assert_eq!(got.uid, 7);
        assert_eq!(got.data, FileData::Inline(b"JFIF-bytes".to_vec()));
        assert!(list.damage.is_empty());
    }

    #[test]
    fn over_ceiling_data_stays_the_non_inline_note() {
        // A declared 33 MiB payload inside a master whose declared
        // extent covers it (the synthetic bound stands in for
        // the honest larger file — the parse never reads the payload,
        // so the stream itself stays small).
        let huge = 33 * 1024 * 1024usize;
        let mut file = el(ID_FILE_NAME, b"huge.bin");
        file.extend_from_slice(&id_bytes(ID_FILE_DATA));
        // The 8-byte size vint (0x01 marker + 7 data bytes).
        file.push(0x01);
        file.extend_from_slice(&(huge as u64).to_be_bytes()[1..]);
        // The record itself must declare the honest extent (an 8-byte
        // size vint over name field + data header + the huge payload)
        // — a size covering only the present bytes would be the
        // contained lie instead.
        let mut record = id_bytes(ID_ATTACHED_FILE);
        record.push(0x01);
        record.extend_from_slice(&((file.len() + huge) as u64).to_be_bytes()[1..]);
        record.extend_from_slice(&file);
        let bytes = attachments_element_unknown(&record);
        // The bound covers the record's honest declared extent (the
        // 33 MiB payload included — never read, only noted).
        let bound = (bytes.len() + huge) as u64;
        let list = parse(&bytes, bound).must();
        assert_eq!(list.files.len(), 1);
        match &list.files[0].data {
            FileData::NonInline { offset, len } => {
                // The payload begins exactly where the constructed
                // element ends (no payload bytes were appended).
                assert_eq!(*offset, bytes.len() as u64);
                assert_eq!(*len, huge as u64);
            }
            other => unreachable!("over-ceiling data stays non-inline, got {other:?}"),
        }
    }

    #[test]
    fn truncated_file_data_is_the_typed_truncation() {
        // FileData declares 10 bytes; the stream carries 4.
        let mut file = el(ID_FILE_NAME, b"x.jpg");
        file.extend_from_slice(&id_bytes(ID_FILE_DATA));
        file.extend(size_bytes(10));
        file.extend_from_slice(b"JFIF"); // 4 of the declared 10
        let bytes = attachments_element(&el(ID_ATTACHED_FILE, &file));
        let err = match parse(&bytes, bytes.len() as u64) {
            Ok(_) => unreachable!("this fixture must type-refuse"),
            Err(err) => err,
        };
        assert!(
            matches!(
                err.kind,
                media_types::ErrorKind::Corrupt {
                    detail: media_types::error::CorruptDetail::Truncated { .. }
                }
            ),
            "typed truncation, got {err}"
        );
        assert_eq!(err.context.code, "demux.mkv.truncated");
    }

    #[test]
    fn file_data_overrunning_the_master_is_the_typed_truncation() {
        // The record claims 10 bytes of data but the master declares
        // an extent 4 bytes short of it — the contained lie, typed at
        // the master's end (the stream itself carries all bytes).
        let mut file = el(ID_FILE_NAME, b"x.jpg");
        file.extend_from_slice(&id_bytes(ID_FILE_DATA));
        file.extend(size_bytes(10));
        file.extend_from_slice(b"0123456789");
        let record = el(ID_ATTACHED_FILE, &file);
        let mut bytes = id_bytes(ID_ATTACHMENTS);
        bytes.extend(size_bytes(record.len() - 4));
        bytes.extend_from_slice(&record);
        let err = match parse(&bytes, bytes.len() as u64) {
            Ok(_) => unreachable!("this fixture must type-refuse"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "demux.mkv.truncated");
    }

    #[test]
    fn attached_file_without_data_is_damage_skipped() {
        let file = el(ID_FILE_NAME, b"no-data.bin");
        let bytes = attachments_element(&el(ID_ATTACHED_FILE, &file));
        let list = parse(&bytes, bytes.len() as u64).must();
        assert!(list.files.is_empty());
        assert_eq!(list.damage.len(), 1);
        assert_eq!(list.damage[0].1, "attached-file-without-data");
    }

    #[test]
    fn invalid_utf8_labels_are_tolerated_as_absent() {
        let mut file = el(ID_FILE_NAME, &[0xFF, 0xFE]);
        file.extend(el(ID_FILE_DATA, b"payload"));
        let bytes = attachments_element(&el(ID_ATTACHED_FILE, &file));
        let list = parse(&bytes, bytes.len() as u64).must();
        assert_eq!(list.files.len(), 1);
        assert_eq!(list.files[0].name, None);
        assert_eq!(list.files[0].data, FileData::Inline(b"payload".to_vec()));
    }

    #[test]
    fn attachment_count_ceiling_skips_with_damage() {
        let mut payload = Vec::new();
        for i in 0..=MAX_ATTACHMENTS {
            let mut file = el(ID_FILE_UID, &(i as u64).to_be_bytes());
            file.extend(el(ID_FILE_DATA, b"x"));
            payload.extend(el(ID_ATTACHED_FILE, &file));
        }
        let bytes = attachments_element(&payload);
        let list = parse(&bytes, bytes.len() as u64).must();
        assert_eq!(list.files.len(), MAX_ATTACHMENTS);
        assert_eq!(list.damage.len(), 1);
        assert_eq!(list.damage[0].1, "attachment-over-ceiling");
        // The survivors keep their declared order (uid = index).
        assert_eq!(list.files[0].uid, 0);
        assert_eq!(
            list.files[MAX_ATTACHMENTS - 1].uid,
            (MAX_ATTACHMENTS - 1) as u64
        );
    }

    #[test]
    fn unknown_size_inside_the_master_is_bad_structure() {
        // AttachedFile with an unknown-size marker.
        let mut payload = id_bytes(ID_ATTACHED_FILE);
        payload.push(0xFF);
        payload.extend(el(ID_FILE_DATA, b"x"));
        let bytes = attachments_element(&payload);
        let err = match parse(&bytes, bytes.len() as u64) {
            Ok(_) => unreachable!("this fixture must type-refuse"),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "demux.mkv.bad-structure");
    }
}
