//! Local file sources (card 2.A1): the `plugin_api::Access`
//! implementation doc 01 §2.4 assigns to this crate, plus the raw
//! [`FileSource`] it opens and the [`FileStream`] adapter that satisfies
//! the plugin-api `SourceStream` contract over a
//! [`BufferedReader`](crate::BufferedReader).
//!
//! Regular files only: directories, FIFOs and device nodes are typed
//! rejections at open time — a device like `/dev/zero` would be an
//! unbounded read (MP §12 bounds memory; doc 01 §2.4 "no unbounded
//! read-ahead").
//!
//! # Open path
//!
//! [`FileAccess::open`] resolves the URI
//! ([`uri::resolve_file`](crate::uri::resolve_file): scheme, shape,
//! percent-decoding, canonicalization), opens the canonical path with
//! `std::fs` only, verifies regular-file-ness from the handle's own
//! metadata (no open/stat race), and hands the source to a
//! fixed-capacity [`BufferedReader`]. The returned [`FileStream`] is
//! seekable with a live `size()` (growing files report their current
//! truth; the truncation reference stays the open-time snapshot inside
//! the reader).

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use media_types::EngineError;
use media_types::ids::SourceUri;
use plugin_api::{Access, SeekOrigin, SourcePosition, SourceStream};

use crate::buffered::{BufferedReader, DEFAULT_BUFFER_BYTES, MAX_BUFFER_BYTES};
use crate::contract::resolve_origin;
use crate::error;
use crate::seekable::{RawRead, Seekable};

/// A raw local-file byte source: an open `std::fs` handle plus the
/// position and length bookkeeping the [`Seekable`] contract needs
/// (`&self` position queries; `std::io::Seek::stream_position` demands
/// `&mut`, so the position is tracked, not polled).
pub struct FileSource {
    file: std::fs::File,
    pos: u64,
    /// The open-time length: the fallback when a live re-stat fails.
    fallback_len: Option<u64>,
}

impl FileSource {
    /// Open `path` for reading; refuse anything that is not a regular
    /// file.
    ///
    /// # Errors
    /// [`EngineError`] — `input.file.missing` / `input.file.permission`
    /// / `input.file.open-io` for open failures (OS error preserved in
    /// the source chain), `input.file.not-a-regular-file` for
    /// directories, FIFOs and devices.
    pub fn open(path: &Path) -> Result<FileSource, EngineError> {
        let file = std::fs::File::open(path).map_err(error::open_io)?;
        // Metadata from the handle itself: what was opened is what was
        // inspected (no open/stat race).
        let meta = file.metadata().map_err(error::open_io)?;
        if !meta.is_file() {
            return Err(error::not_a_regular_file());
        }
        Ok(FileSource {
            file,
            pos: 0,
            fallback_len: Some(meta.len()),
        })
    }
}

impl Seekable for FileSource {
    fn position(&self) -> u64 {
        self.pos
    }

    fn length(&self) -> Option<u64> {
        // Live truth with the open-time value as fallback: a growing
        // file reports its current size; a failing stat keeps the last
        // known length instead of inventing ignorance.
        self.file
            .metadata()
            .map(|meta| meta.len())
            .ok()
            .or(self.fallback_len)
    }

    fn seek(&mut self, to: u64) -> Result<u64, EngineError> {
        if let Some(len) = self.length()
            && to > len
        {
            return Err(error::seek_out_of_range());
        }
        self.file
            .seek(SeekFrom::Start(to))
            .map_err(error::seek_io)?;
        self.pos = to;
        Ok(to)
    }
}

impl RawRead for FileSource {
    fn read(&mut self, out: &mut [u8]) -> Result<usize, EngineError> {
        let filled = self.file.read(out).map_err(error::read_io)?;
        // Saturating by construction: a position can only approach the
        // file's length, which is bounded far below `u64::MAX`.
        self.pos = self.pos.saturating_add(filled as u64);
        Ok(filled)
    }
}

/// The file `Access` implementation (doc 01 §2.4): opens `file:` URIs
/// as seekable [`FileStream`]s with a fixed-capacity bounded reader.
///
/// Construct with [`FileAccess::new`] (default read-ahead) or
/// [`FileAccess::with_buffer_capacity`] for a specific window.
#[derive(Debug, Clone, Copy)]
pub struct FileAccess {
    capacity: usize,
}

impl FileAccess {
    /// A file access with the default read-ahead window
    /// ([`DEFAULT_BUFFER_BYTES`]).
    pub fn new() -> FileAccess {
        FileAccess {
            capacity: DEFAULT_BUFFER_BYTES,
        }
    }

    /// A file access with a specific read-ahead window.
    ///
    /// # Errors
    /// [`EngineError`] — the same capacity bounds as
    /// [`BufferedReader::new`] (zero / over [`MAX_BUFFER_BYTES`] are
    /// typed errors).
    pub fn with_buffer_capacity(bytes: usize) -> Result<FileAccess, EngineError> {
        if bytes == 0 {
            return Err(error::capacity_zero());
        }
        if bytes > MAX_BUFFER_BYTES {
            return Err(error::capacity_over_budget(bytes, MAX_BUFFER_BYTES));
        }
        Ok(FileAccess { capacity: bytes })
    }

    /// The configured read-ahead window.
    pub fn buffer_capacity(&self) -> usize {
        self.capacity
    }
}

impl Default for FileAccess {
    fn default() -> Self {
        FileAccess::new()
    }
}

impl Access for FileAccess {
    fn accepts(&self, uri: &SourceUri) -> bool {
        uri.scheme().as_str() == "file"
    }

    fn open(&self, uri: &SourceUri) -> Result<Box<dyn SourceStream>, EngineError> {
        if !self.accepts(uri) {
            return Err(plugin_api::access::scheme_not_served(uri));
        }
        let path = crate::uri::resolve_file(uri)?;
        let source = FileSource::open(&path)?;
        let reader = BufferedReader::new(source, self.capacity)?;
        Ok(Box::new(FileStream {
            reader,
            uri: uri.clone(),
        }))
    }
}

/// A local file behind a [`BufferedReader`], shaped as the plugin-api
/// [`SourceStream`] the engine consumes.
pub struct FileStream {
    reader: BufferedReader<FileSource>,
    uri: SourceUri,
}

impl SourceStream for FileStream {
    fn uri(&self) -> &SourceUri {
        &self.uri
    }

    fn size(&self) -> Option<u64> {
        self.reader.source_length()
    }

    fn supports_seek(&self) -> bool {
        true
    }

    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        self.reader.read(len)
    }

    fn seek(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError> {
        let target = resolve_origin(self.reader.position(), origin)?;
        let to = self.reader.seek(target)?;
        Ok(SourcePosition(to))
    }

    fn position(&self) -> SourcePosition {
        SourcePosition(self.reader.position())
    }
}
