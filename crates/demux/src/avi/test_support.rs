//! In-memory byte source for driving the demuxer in doctests, the
//! fuzz harness and downstream engine tests (card 2.B2).
//!
//! This module is **public on purpose** (the `input` crate's
//! `SyntheticStream` precedent): rustdoc examples cannot see
//! `#[cfg(test)]` items, the cargo-fuzz target (a separate crate, doc
//! 04 §8.5) needs a `SourceStream` over raw bytes, and the engine's
//! pipeline tests will want the same. It is test-support surface, not
//! player surface — nothing in the demuxer itself uses it.

use std::sync::Arc;

use media_types::ids::SourceUri;
use media_types::{EngineError, ErrorKind, PipelineStage};
use plugin_api::{SeekOrigin, SourcePosition, SourceStream};

/// A read-only, seekable [`SourceStream`] over shared bytes.
///
/// Positional semantics mirror a regular file: reads at the end
/// return empty (clean EOF), seeks land anywhere within the byte
/// range, seeks past the end are typed out-of-range errors and leave
/// the position unchanged.
#[derive(Debug, Clone)]
pub struct BytesStream {
    uri: SourceUri,
    data: Arc<[u8]>,
    pos: u64,
}

impl BytesStream {
    /// Build over `data`.
    ///
    /// # Errors
    /// Never fails for the fixed `memory://` URI; the `Result` shape
    /// keeps the constructor honest about the URI parse it performs.
    pub fn new(data: Arc<[u8]>) -> Result<BytesStream, EngineError> {
        let uri = SourceUri::parse("memory://lumen-demux-test-support")?; // lumen-allow-literal: test-support synthetic URI — test infrastructure, not product identity
        Ok(BytesStream { uri, data, pos: 0 })
    }

    /// The bytes behind the stream (assertions in tests).
    pub fn bytes(&self) -> &[u8] {
        &self.data
    }
}

impl SourceStream for BytesStream {
    fn uri(&self) -> &SourceUri {
        &self.uri
    }

    fn size(&self) -> Option<u64> {
        Some(self.data.len() as u64)
    }

    fn supports_seek(&self) -> bool {
        true
    }

    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        let end = (self.pos + len as u64).min(self.data.len() as u64);
        let start = (self.pos as usize).min(self.data.len());
        let out = self.data[start..end as usize].to_vec();
        self.pos = end;
        Ok(out)
    }

    /// The borrow-twin override (card 3.B3): a direct copy out of the
    /// backing `Arc<[u8]>` into caller memory — zero allocation, so
    /// the fuzz drivers and the zero-alloc proofs measure the
    /// demuxer's own cost, not this seam's.
    fn read_into(&mut self, out: &mut [u8]) -> Result<usize, EngineError> {
        let end = (self.pos + out.len() as u64).min(self.data.len() as u64);
        let start = (self.pos as usize).min(self.data.len());
        out[..(end as usize - start)].copy_from_slice(&self.data[start..end as usize]);
        let got = end as usize - start;
        self.pos = end;
        Ok(got)
    }

    fn seek(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError> {
        let target = match origin {
            SeekOrigin::Start(to) => to,
            SeekOrigin::Current(delta) => {
                let base = i128::from(self.pos);
                let sum = base + i128::from(delta);
                if sum < 0 || sum > i128::from(u64::MAX) {
                    return Err(out_of_range());
                }
                sum as u64
            }
        };
        if target > self.data.len() as u64 {
            return Err(out_of_range());
        }
        self.pos = target;
        Ok(SourcePosition(target))
    }

    fn position(&self) -> SourcePosition {
        SourcePosition(self.pos)
    }
}

/// Typed out-of-range seek for [`BytesStream`].
fn out_of_range() -> EngineError {
    EngineError::new(
        PipelineStage::Open,
        ErrorKind::Inaccessible {
            reason: media_types::error::AccessDenied::NotFound,
        },
        media_types::error::ErrorContext::new("demux.test.bytes-stream.seek-out-of-range"),
    )
}

/// A boxed [`SourceStream`] over `bytes` — the one-liner doctests and
/// fuzz drivers use.
///
/// # Errors
/// Propagates [`BytesStream::new`]'s (never-failing) URI parse.
pub fn mem_stream(bytes: impl Into<Arc<[u8]>>) -> Result<Box<dyn SourceStream>, EngineError> {
    let stream = BytesStream::new(bytes.into())?;
    Ok(Box::new(stream))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn must_read(stream: &mut dyn SourceStream, len: usize) -> Vec<u8> {
        match stream.read(len) {
            Ok(bytes) => bytes,
            Err(err) => unreachable!("mem read: {err}"),
        }
    }

    #[test]
    fn reads_advance_and_clean_eof_is_sticky() {
        let mut stream = mem_stream(b"0123456789".as_slice().to_vec())
            .unwrap_or_else(|err| unreachable!("{err}"));
        assert_eq!(must_read(stream.as_mut(), 4), b"0123");
        assert_eq!(must_read(stream.as_mut(), 100), b"456789");
        // Clean EOF: empty read, repeatedly, position pinned at end.
        assert!(must_read(stream.as_mut(), 8).is_empty());
        assert!(must_read(stream.as_mut(), 8).is_empty());
        assert_eq!(stream.position().0, 10);
    }

    #[test]
    fn seek_bounds() {
        let mut stream = mem_stream(vec![7u8; 5]).unwrap_or_else(|err| unreachable!("{err}"));
        assert!(stream.seek(SeekOrigin::Start(5)).is_ok());
        assert!(stream.seek(SeekOrigin::Start(6)).is_err());
        assert_eq!(stream.position().0, 5, "failed seek leaves position");
        assert!(stream.seek(SeekOrigin::Current(-5)).is_ok());
        assert_eq!(stream.position().0, 0);
        assert!(stream.seek(SeekOrigin::Current(-1)).is_err());
        assert!(stream.seek(SeekOrigin::Start(0)).is_ok());
        assert_eq!(must_read(stream.as_mut(), 5), vec![7u8; 5]);
    }

    #[test]
    fn size_and_uri_are_honest() {
        let stream = mem_stream(vec![0u8; 12]).unwrap_or_else(|err| unreachable!("{err}"));
        assert_eq!(stream.size(), Some(12));
        assert!(stream.supports_seek());
        assert_eq!(stream.uri().scheme().as_str(), "memory");
    }
}
