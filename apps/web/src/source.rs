//! The in-memory byte source + blob registry (card 7.H1; doc 09
//! §6.1's File/Blob lane).
//!
//! The web host has no filesystem (`std::fs` on
//! `wasm32-unknown-unknown` always errors, and the privacy posture
//! of doc 09 §9.2 is that user-selected files never leave the page
//! anyway): bytes arrive through the port as [`HostToWorker::LoadBlob`]
//! messages and are served back to the probe and demuxer through
//! [`SourceStream`] over shared, reference-counted storage — zero
//! copies per read beyond the demuxer's own buffer fill.
//!
//! The registry is bounded (at most [`MAX_BLOBS`] entries, LRU-free:
//! a later same-name load REPLACES the bytes — the CLI's
//! source-replacement posture is card 3.A2's, so the registry only
//! ever holds data, never opens sessions). Bytes are retained for
//! the session's life because the AVI demuxer's seek table re-reads
//! the stream positionally.

use std::collections::BTreeMap;
use std::sync::Arc;

use media_types::ids::SourceUri;
use media_types::{EngineError, PipelineStage};
use plugin_api::{SeekOrigin, SourcePosition, SourceStream};

/// The registry bound (bounded memory, MP §11 — a host that loads
/// more distinct blobs than this gets the typed refusal, never an
/// unbounded map).
pub const MAX_BLOBS: usize = 16;

/// A registry error, typed at the open stage (the command layer's
/// `input` vocabulary — the same stage `input`'s own refusals use).
#[derive(Debug)]
pub enum RegistryError {
    /// The registry is full (bounded at [`MAX_BLOBS`]).
    Full,
    /// No blob is registered under this name.
    Unknown { name: String },
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::Full => write!(f, "blob registry full ({MAX_BLOBS} entries)"),
            RegistryError::Unknown { name } => write!(f, "no blob named `{name}` is loaded"),
        }
    }
}

/// The named-blob registry the `loadBlob` message fills and the
/// `blob:<name>` URI resolves against.
#[derive(Debug, Default)]
pub struct BlobRegistry {
    blobs: BTreeMap<String, Arc<[u8]>>,
}

impl BlobRegistry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> BlobRegistry {
        BlobRegistry {
            blobs: BTreeMap::new(),
        }
    }

    /// Register (or replace) `name`'s bytes.
    ///
    /// # Errors
    /// [`RegistryError::Full`] when a NEW name would exceed
    /// [`MAX_BLOBS`] (a replacement never grows the map).
    pub fn load(&mut self, name: &str, bytes: Vec<u8>) -> Result<u64, RegistryError> {
        if !self.blobs.contains_key(name) && self.blobs.len() >= MAX_BLOBS {
            return Err(RegistryError::Full);
        }
        let size = bytes.len() as u64;
        self.blobs.insert(name.to_string(), Arc::from(bytes));
        Ok(size)
    }

    /// The registered size, when the name exists.
    #[must_use]
    pub fn size_of(&self, name: &str) -> Option<u64> {
        self.blobs.get(name).map(|blob| blob.len() as u64)
    }

    /// Open `name`'s stream (position 0, fully seekable).
    ///
    /// # Errors
    /// [`RegistryError::Unknown`] when the name is not loaded.
    pub fn open(&self, name: &str) -> Result<BlobSource, RegistryError> {
        let bytes = self.blobs.get(name).ok_or(RegistryError::Unknown {
            name: name.to_string(),
        })?;
        Ok(BlobSource {
            uri: SourceUri::parse(&format!("blob:{name}")).map_err(|_| RegistryError::Unknown {
                name: name.to_string(),
            })?,
            bytes: Arc::clone(bytes),
            pos: 0,
        })
    }
}

/// A seekable, shared, in-memory [`SourceStream`] — the blob twin of
/// `input`'s file access (the trait's own test-fixture shape, lifted
/// to the production lane the web host needs).
pub struct BlobSource {
    uri: SourceUri,
    bytes: Arc<[u8]>,
    pos: usize,
}

impl BlobSource {
    /// A source over `bytes` addressed as `blob:<name>` (used
    /// directly by tests; the registry is the production entry).
    ///
    /// # Errors
    /// [`EngineError`] when the name does not form a valid `blob:`
    /// URI (RFC 3986 scheme grammar).
    pub fn new(name: &str, bytes: impl Into<Arc<[u8]>>) -> Result<BlobSource, EngineError> {
        Ok(BlobSource {
            uri: SourceUri::parse(&format!("blob:{name}"))?,
            bytes: bytes.into(),
            pos: 0,
        })
    }
}

impl SourceStream for BlobSource {
    fn uri(&self) -> &SourceUri {
        &self.uri
    }

    fn size(&self) -> Option<u64> {
        Some(self.bytes.len() as u64)
    }

    fn supports_seek(&self) -> bool {
        true
    }

    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        let end = (self.pos + len).min(self.bytes.len());
        let out = self.bytes[self.pos..end].to_vec();
        self.pos = end;
        Ok(out)
    }

    fn read_into(&mut self, out: &mut [u8]) -> Result<usize, EngineError> {
        let end = (self.pos + out.len()).min(self.bytes.len());
        let count = end - self.pos;
        out[..count].copy_from_slice(&self.bytes[self.pos..end]);
        self.pos = end;
        Ok(count)
    }

    fn seek(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError> {
        let target: i64 = match origin {
            SeekOrigin::Start(offset) => offset as i64,
            SeekOrigin::Current(delta) => (self.pos as i64).saturating_add(delta),
        };
        if target < 0 || target > self.bytes.len() as i64 {
            return Err(EngineError::new(
                PipelineStage::Open,
                media_types::ErrorKind::Inaccessible {
                    reason: media_types::error::AccessDenied::NotFound,
                },
                media_types::error::ErrorContext::new("web.blob.seek-out-of-range"),
            ));
        }
        self.pos = target as usize;
        Ok(SourcePosition(target as u64))
    }

    fn position(&self) -> SourcePosition {
        SourcePosition(self.pos as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    /// The source honors the trait contract: short reads are data,
    /// `Ok(0)` only at end of stream, seeks are positional.
    #[test]
    fn blob_source_reads_seeks_and_ends() {
        let mut source = BlobSource::new("fixture", b"RIFF-bytes".as_slice()).must();
        assert_eq!(source.uri().scheme().as_str(), "blob");
        assert_eq!(source.size(), Some(10));
        assert!(source.supports_seek());
        assert_eq!(source.read(4).must(), b"RIFF");
        // A short read to end of stream is data, not an error.
        assert_eq!(source.read(100).must(), b"-bytes");
        assert!(source.read(8).must().is_empty(), "zero only at EOS");
        // Positional seeks, both origins.
        assert_eq!(source.seek(SeekOrigin::Start(5)).must(), SourcePosition(5));
        assert_eq!(source.read(2).must(), b"by");
        assert_eq!(
            source.seek(SeekOrigin::Current(-2)).must(),
            SourcePosition(5)
        );
        // Out-of-range is the typed refusal.
        let err = testkit::assert_err(source.seek(SeekOrigin::Start(99)), "seek past end");
        assert_eq!(err.stage, PipelineStage::Open);
    }

    /// `read_into` fills caller memory without allocating (the
    /// borrow twin — the demuxer's buffer-fill path).
    #[test]
    fn blob_source_read_into_fills_caller_memory() {
        let mut source = BlobSource::new("fixture", b"0123456789".as_slice()).must();
        let mut out = [0u8; 4];
        assert_eq!(source.read_into(&mut out).must(), 4);
        assert_eq!(&out, b"0123");
        // Zero-length buffer: zero-length success, no position move.
        assert_eq!(source.read_into(&mut []).must(), 0);
        assert_eq!(source.position(), SourcePosition(4));
        // Partial fill at end of stream.
        let mut rest = [0u8; 8];
        assert_eq!(source.read_into(&mut rest).must(), 6);
        assert_eq!(&rest[..6], b"456789");
    }

    /// The registry resolves names, replaces same-name loads and
    /// refuses unknown names — typed, never a guess.
    #[test]
    fn registry_resolves_replaces_and_refuses() {
        let mut registry = BlobRegistry::new();
        assert_eq!(registry.load("a", vec![1, 2, 3]).must(), 3);
        assert_eq!(registry.size_of("a"), Some(3));
        // Replacement does not grow the map.
        assert_eq!(registry.load("a", vec![1]).must(), 1);
        assert_eq!(registry.size_of("a"), Some(1));
        let err = testkit::assert_err(registry.open("missing"), "unknown name");
        assert!(err.to_string().contains("no blob named `missing`"));
        let source = registry.open("a").must();
        assert_eq!(source.size(), Some(1));
    }

    /// The registry is bounded: the 17th distinct name is the typed
    /// refusal (bounded memory, MP §11).
    #[test]
    fn registry_is_bounded() {
        let mut registry = BlobRegistry::new();
        for index in 0..MAX_BLOBS {
            let name = format!("blob-{index}");
            match registry.load(&name, vec![0]) {
                Ok(_) => {}
                Err(err) => unreachable!("load {name} must fit the bound: {err}"),
            }
        }
        let err = testkit::assert_err(registry.load("one-too-many", vec![0]), "over the bound");
        assert!(err.to_string().contains("registry full"));
        // Replacing an existing name still works at the bound.
        assert_eq!(registry.load("blob-0", vec![9]).must(), 1);
    }

    /// A `blob:` URI round-trips through the L0 grammar (the scheme
    /// is data, not a closed enum — `blob` parses). The L0 grammar
    /// rejects spaces in the remainder, so blob names are
    /// space-free by construction — pinned here, typed.
    #[test]
    fn blob_uri_parses() {
        let source = BlobSource::new("first-vertical.avi", b"x".as_slice()).must();
        let uri = source.uri();
        assert_eq!(uri.scheme().as_str(), "blob");
        // A space-bearing name is the typed L0 refusal, never a
        // silent accept (the grammar's own invariant).
        let err = testkit::assert_err(
            BlobSource::new("first vertical.avi", b"x".as_slice()),
            "space in the name",
        );
        assert!(err.to_string().contains("uri.control_chars"), "{err}");
    }
}
