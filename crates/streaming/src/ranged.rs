//! The ranged seekable resource (card 6.A2a): a bounded-memory,
//! byte-range abstraction over the [`HttpFetch`] port.
//!
//! [`RangedResource`] is what a demuxer reads network media through: it
//! serves the same [`plugin_api::SourceStream`] contract the local-file
//! source serves (crates/input/src/file.rs), so nothing downstream
//! knows the bytes came from HTTP. Position, length, and seeks follow
//! the input crate's seek semantics (crates/input/src/seekable.rs):
//!
//! - `size()` is the `Content-Range` total observed at open (a value
//!   and its absence are different types, MP §8); servers that do not
//!   reveal a total (`bytes 0-0/*`) yield an **unknown-length** stream
//!   — the underlying store is the judge, EOF is a clean short read.
//! - A seek to exactly the length is legal (next read is EOF); a seek
//!   **beyond** a known length fails typed before any request goes out
//!   (`input.seek.out-of-range`, the input contract's stable code —
//!   the conformance kit pins it, so it is minted here verbatim rather
//!   than re-prefixed).
//! - A read that hits the wire-end before the open-time length fails
//!   typed (`Corrupt::Truncated`) — never a silent EOF, matching the
//!   file source's truncation contract. A server that shrinks the
//!   resource mid-stream (416, or a smaller `Content-Range` total)
//!   surfaces the same typed truncation.
//!
//! # Bounded memory (MP §12)
//!
//! Every buffer is bounded by [`RangedConfig`]: each `Range:` request
//! asks for at most `chunk_bytes` (also the transport
//! `response_limit`, so a hostile over-long body fails
//! `ResponseTooLarge` instead of allocating), and the window cache
//! holds at most `window_bytes` with least-recently-used eviction.
//! The default window mirrors `product.toml`'s
//! `limits.cache_network_mib = 64` (schema 4..=1024); the default
//! request chunk is 256 KiB. Live product-config plumbing lands with
//! the http open-path card; the constants here are the documented
//! defaults for that join.
//!
//! # Request granularity (documented tradeoff)
//!
//! The 6.A1a port returns **whole bodies** (no incremental reader
//! over the socket), so a range fetch buffers one `chunk_bytes`-sized
//! body in memory before it can be served. Reading `N` bytes forward
//! therefore costs `ceil(N / chunk_bytes)` requests — the chunk is
//! the read-ahead knob, and a larger chunk trades memory for fewer
//! round-trips. An incremental-body port extension would remove the
//! per-chunk buffering but not the request count.

use std::collections::VecDeque;
use std::sync::Arc;

use media_types::error::{AccessDenied, CorruptDetail, ErrorContext, LimitKind, NetworkCause};
use media_types::{EngineError, ErrorKind, PipelineStage};
use plugin_api::{SeekOrigin, SourcePosition, SourceStream};

use crate::error::NetworkError;
use crate::fetch::{ByteRange, HttpFetch, HttpRequest};

/// Default window-cache budget: 64 MiB, mirroring `product.toml`'s
/// `limits.cache_network_mib = 64` (the "network buffer" limit, schema
/// 4..=1024 MiB). The budget caps lazily fetched chunks — it is never
/// allocated up front.
pub const DEFAULT_WINDOW_BYTES: usize = 64 * 1024 * 1024;

/// Window-budget ceiling: the schema maximum for
/// `limits.cache_network_mib` (1024 MiB). A larger budget is a typed
/// refusal, never a clamp (MP §5).
pub const MAX_WINDOW_BYTES: usize = 1024 * 1024 * 1024;

/// Default read-ahead / request granularity: 256 KiB per `Range:`
/// request (CDN-friendly; the demux arena's small-header reads are
/// served from the window cache, not new requests).
pub const DEFAULT_CHUNK_BYTES: usize = 256 * 1024;

/// Ceiling on a single `read(len)` result, mirroring the input
/// crate's `MAX_READ_BYTES` payload ceiling (64 MiB): the caller-owned
/// result of one bounded read is the only memory outside the window
/// budget. Streaming does not depend on `input`, so the value is
/// restated here with this note.
pub const MAX_READ_BYTES: usize = 64 * 1024 * 1024;

/// The stage tag every ranged-resource error carries (sources fail
/// while being opened or read — the input-layer convention, doc 03
/// §4.1(d)).
const STAGE: PipelineStage = PipelineStage::Open;

/// Mint a stage-`Open` error with a stable dotted code (input's
/// constructor pattern; the taxonomy itself stays media-types').
fn mint(kind: ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(STAGE, kind, ErrorContext::new(code))
}

/// A seek target beyond the known end of the resource — the input
/// contract's stable code, minted verbatim (see module docs).
fn seek_out_of_range() -> EngineError {
    mint(
        ErrorKind::Inaccessible {
            reason: AccessDenied::NotFound,
        },
        "input.seek.out-of-range",
    )
}

/// A relative seek that resolves before the start of the stream.
fn seek_before_start() -> EngineError {
    mint(
        ErrorKind::Inaccessible {
            reason: AccessDenied::NotFound,
        },
        "input.seek.before-start",
    )
}

/// A relative seek that overflows `u64`.
fn seek_overflow() -> EngineError {
    mint(
        ErrorKind::Inaccessible {
            reason: AccessDenied::NotFound,
        },
        "input.seek.overflow",
    )
}

/// The resource shrank below its open-time length mid-read: typed
/// `Corrupt::Truncated`, never a silent EOF (the file source's
/// contract, mirrored for network sources).
fn truncated(expected: u64, found: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated { expected, found },
        },
        "streaming.ranged.truncated",
    )
}

/// The server ignored `Range:` — a ranged resource cannot serve seeks
/// over it. The linear-scan fallback is card 6.A2b's scope; refusing
/// typed here is the honest v0 behavior.
fn no_range_support() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "streaming.ranged.no-range-support",
    )
}

/// A `206` without a parseable `Content-Range`, or one that disagrees
/// with the request's start offset.
fn protocol_violation() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "streaming.ranged.protocol-violation",
    )
}

/// Fold a transport [`NetworkError`] into the engine taxonomy (doc 01
/// §3.4 `Network { cause }`), stage `Open`, code `streaming.ranged.*`.
fn map_fetch_err(err: NetworkError) -> EngineError {
    let cause = match &err {
        NetworkError::DnsFailure => NetworkCause::Dns,
        NetworkError::ConnectTimeout | NetworkError::ReadTimeout | NetworkError::WriteTimeout => {
            NetworkCause::Timeout
        }
        NetworkError::TlsFailure => NetworkCause::Tls,
        NetworkError::ConnectRefused
        | NetworkError::BodyTruncated { .. }
        | NetworkError::TooManyRedirects { .. }
        | NetworkError::RedirectLoop { .. }
        | NetworkError::CrossHostRedirect { .. }
        | NetworkError::HttpStatus { .. } => NetworkCause::ConnectionLost,
        NetworkError::ResponseTooLarge { limit, actual } => {
            return mint(
                ErrorKind::ResourceLimit {
                    limit: LimitKind::Memory,
                    budget: *limit,
                    attempted: *actual,
                },
                "streaming.ranged.response-over-budget",
            );
        }
        NetworkError::UnsupportedScheme { .. }
        | NetworkError::ProtocolViolation { .. }
        | NetworkError::InvalidContentRange
        | NetworkError::RangeNotSatisfiable => {
            return mint(
                ErrorKind::Unsupported { format: None },
                "streaming.ranged.fetch-refused",
            );
        }
        NetworkError::Io { .. } => NetworkCause::ConnectionLost,
    };
    mint(
        ErrorKind::Network { cause },
        "streaming.ranged.fetch-failed",
    )
}

/// Resolve a [`SeekOrigin`] against `from` into an absolute target
/// (the input contract's shared math; the codes are its stable ones).
fn resolve_origin(from: u64, origin: SeekOrigin) -> Result<u64, EngineError> {
    match origin {
        SeekOrigin::Start(to) => Ok(to),
        SeekOrigin::Current(delta) => {
            let target = i128::from(from) + i128::from(delta);
            if target < 0 {
                Err(seek_before_start())
            } else if target > i128::from(u64::MAX) {
                Err(seek_overflow())
            } else {
                Ok(target as u64)
            }
        }
    }
}

/// One fetched, cached byte range: `[start, start + data.len())`.
struct Window {
    start: u64,
    data: Vec<u8>,
}

impl Window {
    /// Bytes held by this window.
    fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether this window serves the byte at `pos`.
    fn covers(&self, pos: u64) -> bool {
        pos >= self.start && (pos - self.start) < self.data.len() as u64
    }
}

/// Tuning of a [`RangedResource`]: the memory contract in two knobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangedConfig {
    /// Window-cache budget: the maximum bytes held across all cached
    /// windows (LRU eviction above it). Mirrors
    /// `limits.cache_network_mib` (default 64 MiB, ceiling
    /// [`MAX_WINDOW_BYTES`]).
    pub window_bytes: usize,
    /// Request granularity / forward read-ahead: the maximum `Range:`
    /// extent per fetch (also the transport `response_limit` for that
    /// fetch — a hostile longer body fails typed instead of
    /// allocating).
    pub chunk_bytes: usize,
}

impl Default for RangedConfig {
    fn default() -> Self {
        RangedConfig {
            window_bytes: DEFAULT_WINDOW_BYTES,
            chunk_bytes: DEFAULT_CHUNK_BYTES,
        }
    }
}

impl RangedConfig {
    /// Validate: both knobs positive, window at or below the schema
    /// ceiling, chunk at or below the window (a chunk that cannot fit
    /// in its own cache would force eviction of the window being
    /// served — a configuration bug, refused typed, never clamped).
    ///
    /// # Errors
    /// [`EngineError`] — `streaming.ranged.config-zero`,
    /// `streaming.ranged.window-over-budget`, or
    /// `streaming.ranged.chunk-over-window`.
    pub fn validate(&self) -> Result<(), EngineError> {
        if self.window_bytes == 0 || self.chunk_bytes == 0 {
            return Err(mint(
                ErrorKind::Unsupported { format: None },
                "streaming.ranged.config-zero",
            ));
        }
        if self.window_bytes > MAX_WINDOW_BYTES {
            return Err(mint(
                ErrorKind::ResourceLimit {
                    limit: LimitKind::Memory,
                    budget: MAX_WINDOW_BYTES as u64,
                    attempted: self.window_bytes as u64,
                },
                "streaming.ranged.window-over-budget",
            ));
        }
        if self.chunk_bytes > self.window_bytes {
            return Err(mint(
                ErrorKind::Unsupported { format: None },
                "streaming.ranged.chunk-over-window",
            ));
        }
        Ok(())
    }
}

/// Test/telemetry-facing counters of a [`RangedResource`] (the
/// bounded-memory proofs assert on these; no production path reads
/// them).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RangedStats {
    /// HTTP requests issued (probe included).
    pub requests: u64,
    /// Body bytes fetched off the wire (probe included).
    pub bytes_fetched: u64,
    /// Reads served wholly or partly from the window cache without a
    /// new fetch.
    pub cache_hits: u64,
    /// Windows evicted to stay inside the budget.
    pub cache_evictions: u64,
}

/// A seekable, bounded-memory byte stream over an HTTP resource
/// (card 6.A2a) — the network twin of the file source, serving the
/// same [`SourceStream`] contract demuxers already consume.
///
/// Construct with [`RangedResource::open`] (one probe request: a
/// `Range: bytes=0-(chunk-1)` fetch doubles as the first read-ahead,
/// its `Content-Range` total — or `*` — fixes the length mode).
pub struct RangedResource {
    fetch: Arc<dyn HttpFetch>,
    url: String,
    uri: media_types::ids::SourceUri,
    config: RangedConfig,
    /// The open-time length when the server revealed it; `None` for
    /// unknown-length streams (`bytes a-b/*`).
    len: Option<u64>,
    pos: u64,
    /// The bounded window cache, insertion-ordered (front = LRU).
    windows: VecDeque<Window>,
    cached: usize,
    stats: RangedStats,
}

impl RangedResource {
    /// Open `url` through `fetch`: validate the config and URL, issue
    /// the probe (`Range: bytes=0-(chunk-1)`, budgeted to the chunk),
    /// and seed the window cache with its body.
    ///
    /// # Errors
    /// [`EngineError`] (stage `Open`) — config/URL validation, the
    /// transport's typed failures, `streaming.ranged.no-range-support`
    /// when the server answers `200` to a ranged request (the linear
    /// fallback is 6.A2b), or `streaming.ranged.protocol-violation`
    /// for a malformed/disagreeing `Content-Range`. A `416` to the
    /// probe is honored as an empty resource (`size() == Some(0)`).
    pub fn open(
        fetch: Arc<dyn HttpFetch>,
        url: &str,
        config: RangedConfig,
    ) -> Result<RangedResource, EngineError> {
        config.validate()?;
        // Early typed rejection of URL shapes the transport would
        // refuse anyway (the port owns the grammar; this keeps the
        // error at open, before any socket).
        crate::fetch::Url::parse(url).map_err(map_fetch_err)?;
        let uri = media_types::ids::SourceUri::parse(url)?;
        let mut resource = RangedResource {
            fetch,
            url: url.to_string(),
            uri,
            config,
            len: None,
            pos: 0,
            windows: VecDeque::new(),
            cached: 0,
            stats: RangedStats::default(),
        };
        // The probe: a bounded first-chunk fetch. `absorb` validates
        // it: a 206 fixes the length from `Content-Range` (its body
        // becomes the first cached window), a 200 is the no-range
        // refusal, anything else is a protocol violation.
        match resource.fetch.fetch(resource.probe_request()) {
            Ok(resp) => {
                let body = resource.absorb(&resp, 0)?;
                resource.insert_window(Window {
                    start: 0,
                    data: body,
                });
            }
            Err(NetworkError::RangeNotSatisfiable) => {
                // `bytes=0-(chunk-1)` unsatisfiable: nothing exists at
                // offset 0 — the empty-resource shape.
                resource.len = Some(0);
            }
            Err(err) => return Err(map_fetch_err(err)),
        }
        Ok(resource)
    }

    /// The probe / first-chunk request.
    fn probe_request(&self) -> HttpRequest {
        HttpRequest::get(self.url.clone())
            .with_range(ByteRange::Bounded {
                start: 0,
                end_exclusive: self.config.chunk_bytes as u64,
            })
            .with_response_limit(self.config.chunk_bytes as u64)
    }

    /// Validate a fetched response for `start` and return its body
    /// (shared by the probe and every window fetch). Only a `206` with
    /// an agreeing `Content-Range` validates; a `200` is the no-range
    /// refusal, anything else is a protocol violation.
    fn absorb(
        &mut self,
        resp: &crate::fetch::HttpResponse,
        start: u64,
    ) -> Result<Vec<u8>, EngineError> {
        self.stats.requests += 1;
        self.stats.bytes_fetched += resp.body.len() as u64;
        if resp.status == 206 {
            let (rstart, _rend, total) = parse_content_range(resp.header("Content-Range"))?;
            if rstart != start {
                return Err(protocol_violation());
            }
            if let Some(total) = total {
                match self.len {
                    Some(known) if total < known => {
                        // The resource shrank below its open-time
                        // length: typed truncation, never silent.
                        return Err(truncated(known, total));
                    }
                    _ => self.len = Some(total),
                }
            }
            Ok(resp.body.clone())
        } else if resp.status == 200 {
            // A no-range answer mid-stream: the body starts at 0, not
            // at `start` — serving it would corrupt position math.
            Err(no_range_support())
        } else {
            Err(protocol_violation())
        }
    }

    /// Fetch `[start, start + chunk)` (clamped to a known length),
    /// validate, and insert into the cache. Returns the fetched
    /// window's byte length.
    fn fetch_window(&mut self, start: u64) -> Result<usize, EngineError> {
        let mut end = start.saturating_add(self.config.chunk_bytes as u64);
        if let Some(len) = self.len {
            end = end.min(len);
            if start >= len {
                return Ok(0);
            }
        }
        let request = HttpRequest::get(self.url.clone())
            .with_range(ByteRange::Bounded {
                start,
                end_exclusive: end,
            })
            .with_response_limit(self.config.chunk_bytes as u64);
        match self.fetch.fetch(request) {
            Ok(resp) => {
                let body = self.absorb(&resp, start)?;
                let got = body.len();
                self.insert_window(Window { start, data: body });
                Ok(got)
            }
            // A 416 mid-read on a known-length resource: it shrank
            // underneath us — typed truncation (unknown-length: the
            // store is the judge, a clean EOF).
            Err(NetworkError::RangeNotSatisfiable) => {
                if let Some(len) = self.len {
                    Err(truncated(len, start))
                } else {
                    Ok(0)
                }
            }
            Err(err) => Err(map_fetch_err(err)),
        }
    }

    /// Insert a window, then evict least-recently-used windows until
    /// the budget holds. The fresh window sits at the back and is never
    /// the only window over budget — a body is bounded by
    /// `chunk_bytes <= window_bytes` for budget-honoring transports
    /// (the port's `response_limit` enforcement) — so eviction only
    /// ever drops *older* windows and a served window is never evicted
    /// out from under the read that fetched it (no refetch loop).
    fn insert_window(&mut self, window: Window) {
        self.cached += window.len();
        self.windows.push_back(window);
        while self.cached > self.config.window_bytes && self.windows.len() > 1 {
            match self.windows.pop_front() {
                Some(evicted) => {
                    self.cached -= evicted.len();
                    self.stats.cache_evictions += 1;
                }
                None => break,
            }
        }
    }

    /// The first cached window serving `pos`, moved to the back
    /// (most-recently-used), with its slice offset.
    fn take_hit(&mut self, pos: u64) -> Option<(usize, usize)> {
        let idx = self.windows.iter().position(|w| w.covers(pos))?;
        let Some(window) = self.windows.remove(idx) else {
            return None;
        };
        let off = (pos - window.start) as usize;
        let len = window.len();
        self.windows.push_back(window);
        Some((off, len))
    }

    /// Copy from the window at `pos` (already moved MRU) into `out`.
    fn copy_hit(&mut self, pos: u64, out: &mut [u8]) -> usize {
        let Some((off, wlen)) = self.take_hit(pos) else {
            return 0;
        };
        let available = wlen - off;
        let n = out.len().min(available);
        let Some(window) = self.windows.back() else {
            return 0;
        };
        out[..n].copy_from_slice(&window.data[off..off + n]);
        n
    }

    /// Bytes currently held in the window cache (bounded by
    /// `config.window_bytes` — the memory-bound proof's live accessor).
    pub fn cached_bytes(&self) -> usize {
        self.cached
    }

    /// Counters for tests and telemetry (see [`RangedStats`]).
    pub fn stats(&self) -> RangedStats {
        self.stats
    }

    /// The open-time length mode: `Some(total)` from
    /// `Content-Range`'s total, `None` for unknown-length streams.
    pub fn length(&self) -> Option<u64> {
        self.len
    }
}

impl SourceStream for RangedResource {
    fn uri(&self) -> &media_types::ids::SourceUri {
        &self.uri
    }

    fn size(&self) -> Option<u64> {
        self.len
    }

    fn supports_seek(&self) -> bool {
        true
    }

    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        if len == 0 {
            return Ok(Vec::new());
        }
        if len > MAX_READ_BYTES {
            return Err(mint(
                ErrorKind::ResourceLimit {
                    limit: LimitKind::Memory,
                    budget: MAX_READ_BYTES as u64,
                    attempted: len as u64,
                },
                "streaming.ranged.read-over-budget",
            ));
        }
        let mut out = vec![0u8; len];
        let n = self.read_into(&mut out)?;
        out.truncate(n);
        Ok(out)
    }

    fn read_into(&mut self, out: &mut [u8]) -> Result<usize, EngineError> {
        if out.is_empty() {
            return Ok(0);
        }
        let mut filled = 0usize;
        while filled < out.len() {
            let pos = self.pos;
            if let Some(len) = self.len
                && pos >= len
            {
                break; // clean, sticky EOF at the known length
            }
            // Cache-first: a hit serves without touching the wire.
            let hit = self.copy_hit(pos, &mut out[filled..]);
            if hit > 0 {
                self.stats.cache_hits += 1;
                self.pos += hit as u64;
                filled += hit;
                continue;
            }
            // Miss: fetch a chunk at the position and serve from it.
            let got = self.fetch_window(pos)?;
            if got == 0 {
                // Unknown-length store judged EOF, or a clean
                // short-of-chunk end (known-length shortfalls surface
                // as typed truncation inside fetch_window).
                break;
            }
            let hit = self.copy_hit(pos, &mut out[filled..]);
            self.pos += hit as u64;
            filled += hit;
        }
        Ok(filled)
    }

    fn seek(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError> {
        let to = resolve_origin(self.pos, origin)?;
        if let Some(len) = self.len
            && to > len
        {
            return Err(seek_out_of_range());
        }
        // Cached windows survive the seek: a seek back into them
        // serves with zero requests (the no-I/O rewind proof).
        self.pos = to;
        Ok(SourcePosition(to))
    }

    fn position(&self) -> SourcePosition {
        SourcePosition(self.pos)
    }
}

/// Parse a `Content-Range: bytes <start>-<end>/<total|*>` header
/// value (with the `bytes ` unit prefix required; `*` totals are the
/// unknown-length shape).
fn parse_content_range(value: Option<&str>) -> Result<(u64, u64, Option<u64>), EngineError> {
    let Some(value) = value else {
        return Err(protocol_violation());
    };
    let value = value.trim();
    let Some(rest) = value.strip_prefix("bytes ") else {
        return Err(protocol_violation());
    };
    let Some((range, total)) = rest.split_once('/') else {
        return Err(protocol_violation());
    };
    let Some((start, end)) = range.split_once('-') else {
        return Err(protocol_violation());
    };
    let (Ok(start), Ok(end)) = (start.trim().parse::<u64>(), end.trim().parse::<u64>()) else {
        return Err(protocol_violation());
    };
    let total = if total.trim() == "*" {
        None
    } else {
        match total.trim().parse::<u64>() {
            Ok(total) => Some(total),
            Err(_) => return Err(protocol_violation()),
        }
    };
    Ok((start, end, total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch::HttpResponse;
    use crate::mockhttp::{MockConfig, MockServer};
    use testkit::Must;

    /// Deterministic non-repeating pattern (period 251, prime to the
    /// chunk sizes used below).
    fn pattern(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i % 251) as u8).collect()
    }

    /// A small-footprint config: 4 KiB window of 1 KiB chunks, so
    /// multi-request behavior shows up at fixture sizes.
    fn tight() -> RangedConfig {
        RangedConfig {
            window_bytes: 4096,
            chunk_bytes: 1024,
        }
    }

    fn served(body: Vec<u8>) -> (MockServer, Arc<dyn HttpFetch>, Vec<u8>) {
        let server = MockServer::spawn(MockConfig::body(body.clone()))
            .unwrap_or_else(|e| unreachable!("spawn: {e}"));
        let fetch: Arc<dyn HttpFetch> = Arc::new(server.client());
        (server, fetch, body)
    }

    fn open(fetch: Arc<dyn HttpFetch>, url: &str, config: RangedConfig) -> RangedResource {
        RangedResource::open(fetch, url, config).unwrap_or_else(|e| unreachable!("open: {e}"))
    }

    /// Sequential reads in varied chunk sizes reconstruct the served
    /// bytes exactly, with positions tracking consumption.
    #[test]
    fn sequential_reads_equal_the_served_bytes() {
        let data = pattern(10_000);
        let (server, fetch, _) = served(data.clone());
        let mut r = open(fetch, &server.url("media.bin"), tight());
        assert_eq!(r.size(), Some(10_000));
        let mut got = Vec::new();
        let mut pos = 0u64;
        for chunk in [1usize, 2, 3, 7, 100, 3000] {
            loop {
                let piece = r.read(chunk).unwrap_or_else(|e| unreachable!("{e}"));
                if piece.is_empty() {
                    break;
                }
                assert!(piece.len() <= chunk);
                got.extend_from_slice(&piece);
                pos += piece.len() as u64;
                assert_eq!(r.position(), SourcePosition(pos));
                if piece.len() < chunk {
                    break;
                }
            }
        }
        assert_eq!(got, data);
        // EOF is clean and sticky; position pins at the length.
        for _ in 0..2 {
            assert!(
                r.read(64)
                    .unwrap_or_else(|e| unreachable!("{e}"))
                    .is_empty()
            );
        }
        assert_eq!(r.position(), SourcePosition(10_000));
    }

    /// A seek back into cached bytes re-reads them with ZERO new HTTP
    /// requests (the mock's request log is the proof) — the rewind
    /// case a demuxer's index walk hammers.
    #[test]
    fn seek_back_rereads_from_cache_with_no_requests() {
        let data = pattern(6000);
        let (server, fetch, _) = served(data.clone());
        let mut r = open(fetch, &server.url("media.bin"), tight());
        let first = r.read(2000).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(first, &data[..2000]);
        let requests_before = server.requests().len();
        assert!(requests_before >= 2, "probe + chunk fetches happened");

        // Rewind to a cached position and re-read a span the earlier
        // fetches already cover ([0, 2048) was fetched for the first
        // read; the re-read stays inside it).
        r.seek(SeekOrigin::Start(512))
            .unwrap_or_else(|e| unreachable!("{e}"));
        let again = r.read(1000).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(again, &data[512..1512]);
        assert_eq!(
            server.requests().len(),
            requests_before,
            "cache hit: no new HTTP request for cached bytes"
        );
        assert!(r.stats().cache_hits >= 1);
    }

    /// A seek forward beyond the read-ahead issues a new range request
    /// at exactly the right offset — pinned from the mock's request
    /// log (`Range: bytes=<off>-<off+chunk-1>`).
    #[test]
    fn seek_forward_beyond_read_ahead_fetches_at_the_right_offset() {
        let data = pattern(9000);
        let (server, fetch, _) = served(data.clone());
        let mut r = open(fetch, &server.url("media.bin"), tight());
        let _ = r.read(100).unwrap_or_else(|e| unreachable!("{e}"));
        let before = server.requests().len();

        r.seek(SeekOrigin::Start(5000))
            .unwrap_or_else(|e| unreachable!("{e}"));
        let piece = r.read(100).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(piece, &data[5000..5100]);

        let new_reqs = &server.requests()[before..];
        assert!(!new_reqs.is_empty(), "a new range request was issued");
        let range = new_reqs[0]
            .header("range")
            .unwrap_or_else(|| unreachable!("Range header"));
        assert_eq!(
            range, "bytes=5000-6023",
            "the fetch lands at the seek offset (chunk 1024)"
        );
    }

    /// Reading A LOT forward holds the memory bound: the cache never
    /// exceeds the window budget while every byte still arrives
    /// byte-exact (the bounded-memory proof, live stats accessor).
    #[test]
    fn forward_scan_holds_the_window_budget() {
        let data = pattern(50_000);
        let (server, fetch, _) = served(data.clone());
        let mut r = open(fetch, &server.url("media.bin"), tight());
        let mut got = Vec::new();
        let mut peak = 0usize;
        loop {
            let piece = r.read(512).unwrap_or_else(|e| unreachable!("{e}"));
            if piece.is_empty() {
                break;
            }
            got.extend_from_slice(&piece);
            peak = peak.max(r.cached_bytes());
        }
        assert_eq!(got, data, "byte-exact through the bounded window");
        assert!(peak <= 4096, "cache peaked at {peak} > 4096 budget");
        let stats = r.stats();
        assert!(stats.cache_evictions > 0, "old windows were evicted");
        // ceil(50_000 / 1024) = 49 chunk bodies, all within a small
        // factor of the payload (no re-fetch loops).
        assert!(
            stats.bytes_fetched < 60_000,
            "fetched {} bytes for a 50_000-byte resource",
            stats.bytes_fetched
        );
        assert_eq!(stats.requests as usize, server.requests().len());
    }

    /// EOF short read: the final read returns exactly the remaining
    /// bytes (short reads are data), then a clean `Ok(0)`.
    #[test]
    fn eof_short_read_is_data_then_clean_zero() {
        let data = pattern(2500);
        let (server, fetch, _) = served(data.clone());
        let mut r = open(fetch, &server.url("media.bin"), tight());
        let head = r.read(2000).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(head.len(), 2000);
        let tail = r.read(999).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(tail, &data[2000..], "short only at end of stream");
        assert!(
            r.read(16)
                .unwrap_or_else(|e| unreachable!("{e}"))
                .is_empty()
        );
        // Seek to exactly the length: legal, next read is a clean EOF.
        r.seek(SeekOrigin::Start(2500))
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert!(r.read(8).unwrap_or_else(|e| unreachable!("{e}")).is_empty());
    }

    /// A seek past the known end fails typed with the input
    /// contract's stable code — before any request goes out — and
    /// never moves the cursor; before-start likewise.
    #[test]
    fn seek_past_end_is_the_input_contract_error() {
        let data = pattern(1000);
        let (server, fetch, _) = served(data.clone());
        let mut r = open(fetch, &server.url("media.bin"), tight());
        let requests_before = server.requests().len();
        r.seek(SeekOrigin::Start(10))
            .unwrap_or_else(|e| unreachable!("{e}"));
        let err = match r.seek(SeekOrigin::Start(1001)) {
            Err(err) => err,
            Ok(_) => unreachable!("a seek past the end must fail typed"),
        };
        assert_eq!(err.context.code, "input.seek.out-of-range");
        assert_eq!(r.position(), SourcePosition(10), "failed seek never moves");
        assert_eq!(
            server.requests().len(),
            requests_before,
            "the refusal is client-side: no request for a past-end seek"
        );
        let err = match r.seek(SeekOrigin::Current(-11)) {
            Err(err) => err,
            Ok(_) => unreachable!("a seek before the start must fail typed"),
        };
        assert_eq!(err.context.code, "input.seek.before-start");
    }

    /// A server that 416s a mid-read range (the resource shrank below
    /// its open-time length): typed `Corrupt::Truncated`, never a
    /// silent EOF — the file source's contract, mirrored.
    #[test]
    fn mid_read_416_is_a_typed_truncation() {
        struct Shrinker {
            body: Vec<u8>,
        }
        impl HttpFetch for Shrinker {
            fn fetch(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError> {
                let Some(crate::fetch::ByteRange::Bounded {
                    start,
                    end_exclusive,
                }) = request.range
                else {
                    return Err(NetworkError::ProtocolViolation {
                        detail: "bounded requests only in this test",
                    });
                };
                // Claims a 5000-byte total on the probe, then refuses
                // every range past the first chunk: a server whose
                // resource shrank underneath the reader.
                if start == 0 {
                    let end = end_exclusive.min(self.body.len() as u64);
                    return Ok(HttpResponse {
                        status: 206,
                        reason: "Partial Content".to_string(),
                        headers: vec![
                            (
                                "Content-Range".to_string(),
                                format!("bytes 0-{}/5000", end.saturating_sub(1)),
                            ),
                            ("Content-Length".to_string(), format!("{end}")),
                        ],
                        body: self.body[..end as usize].to_vec(),
                    });
                }
                Err(NetworkError::RangeNotSatisfiable)
            }
        }
        let fetch: Arc<dyn HttpFetch> = Arc::new(Shrinker {
            body: pattern(2048),
        });
        let mut r = open(fetch, "http://shrink.example.org/live", tight());
        assert_eq!(
            r.size(),
            Some(5000),
            "the open-time length is the probe's claim"
        );
        let err = match r.read(4000) {
            Err(err) => err,
            Ok(_) => unreachable!("a shrunk resource surfaces typed"),
        };
        assert_eq!(err.context.code, "streaming.ranged.truncated");
        let media_types::ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated { expected, found },
        } = err.kind
        else {
            unreachable!("the shrink is a Corrupt::Truncated");
        };
        assert_eq!(
            (expected, found),
            (5000, 1024),
            "open-time length vs the 416 offset"
        );
    }

    /// Small-file edge: the whole resource is smaller than one chunk
    /// and the window — one probe serves everything, size is honest.
    #[test]
    fn small_file_smaller_than_the_window() {
        let data = pattern(300);
        let (server, fetch, _) = served(data.clone());
        let mut r = open(
            fetch,
            &server.url("tiny.bin"),
            RangedConfig {
                window_bytes: 64 * 1024,
                chunk_bytes: 64 * 1024,
            },
        );
        assert_eq!(r.size(), Some(300));
        let all = r.read(1000).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(all, data);
        assert!(r.read(4).unwrap_or_else(|e| unreachable!("{e}")).is_empty());
        assert_eq!(r.stats().requests, 1, "the probe was the only request");
    }

    /// A no-range server (`honor_range: false`, the 200-full shape)
    /// is refused typed at open: the linear-scan fallback is 6.A2b's
    /// card, and guessing position math off a full-body 200 would
    /// corrupt reads.
    #[test]
    fn no_range_server_is_refused_typed_at_open() {
        let server = MockServer::spawn(MockConfig {
            honor_range: false,
            body: pattern(500),
            ..MockConfig::default()
        })
        .unwrap_or_else(|e| unreachable!("spawn: {e}"));
        let fetch: Arc<dyn HttpFetch> = Arc::new(server.client());
        let err = testkit::assert_err(
            RangedResource::open(fetch, &server.url("x"), tight()),
            "no-range open",
        );
        assert_eq!(err.context.code, "streaming.ranged.no-range-support");
    }

    /// Unknown-length servers (`Content-Range: bytes a-b/*`): size()
    /// is honestly `None`, seeks accept any target, EOF is the store's
    /// verdict — a clean short read, never a fabricated length.
    #[test]
    fn unknown_length_server_is_honest() {
        struct UnknownLen {
            body: Vec<u8>,
        }
        impl HttpFetch for UnknownLen {
            fn fetch(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError> {
                let Some(crate::fetch::ByteRange::Bounded {
                    start,
                    end_exclusive,
                }) = request.range
                else {
                    return Err(NetworkError::ProtocolViolation {
                        detail: "bounded requests only in this test",
                    });
                };
                let end = end_exclusive.min(self.body.len() as u64);
                if start >= end {
                    // Past the store's end: unsatisfiable — the store
                    // is the judge on an unknown-length stream.
                    return Err(NetworkError::RangeNotSatisfiable);
                }
                Ok(HttpResponse {
                    status: 206,
                    reason: "Partial Content".to_string(),
                    headers: vec![
                        (
                            "Content-Range".to_string(),
                            format!("bytes {start}-{}/{}", end.saturating_sub(1), end - start),
                        ),
                        ("Content-Length".to_string(), format!("{}", end - start)),
                    ],
                    body: self.body[start as usize..end as usize].to_vec(),
                })
            }
        }
        // The `*`-total shape is what fixes unknown-length mode; the
        // fake above advertises a total, so the header is rewritten
        // by the `*` variant in a second scripted fetch below — here
        // the honest path is pinned through a `*`-serving wrapper.
        struct StarTotals {
            inner: UnknownLen,
        }
        impl HttpFetch for StarTotals {
            fn fetch(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError> {
                let mut resp = self.inner.fetch(request)?;
                for (name, value) in &mut resp.headers {
                    if name.eq_ignore_ascii_case("Content-Range") {
                        // `bytes a-b/1234` -> `bytes a-b/*`
                        if let Some((range, _total)) = value.split_once('/') {
                            *value = format!("{range}/*");
                        }
                    }
                }
                Ok(resp)
            }
        }
        let data = pattern(3000);
        let fetch: Arc<dyn HttpFetch> = Arc::new(StarTotals {
            inner: UnknownLen { body: data.clone() },
        });
        let mut r = open(fetch, "http://unknown.example.org/live", tight());
        assert_eq!(r.size(), None, "no fabricated length");
        // Seeks accept any target (the store is the judge); reading
        // past the store's end is a clean EOF.
        r.seek(SeekOrigin::Start(2000))
            .unwrap_or_else(|e| unreachable!("{e}"));
        let piece = r.read(2000).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(piece, &data[2000..], "bytes at the seek offset");
        assert!(r.read(8).unwrap_or_else(|e| unreachable!("{e}")).is_empty());
        let err = match r.seek(SeekOrigin::Current(-5000)) {
            Err(err) => err,
            Ok(_) => unreachable!("before-start still fails typed"),
        };
        assert_eq!(err.context.code, "input.seek.before-start");
    }

    /// Config validation: zero knobs and over-ceiling budgets are
    /// typed refusals, never clamps (MP §5).
    #[test]
    fn config_bounds_are_typed_refusals() {
        let zero_window = testkit::assert_err(
            RangedConfig {
                window_bytes: 0,
                chunk_bytes: 1024,
            }
            .validate(),
            "zero window",
        );
        assert_eq!(zero_window.context.code, "streaming.ranged.config-zero");
        let over = testkit::assert_err(
            RangedConfig {
                window_bytes: MAX_WINDOW_BYTES + 1,
                chunk_bytes: 1024,
            }
            .validate(),
            "over-ceiling window",
        );
        assert_eq!(over.context.code, "streaming.ranged.window-over-budget");
        let fat_chunk = testkit::assert_err(
            RangedConfig {
                window_bytes: 4096,
                chunk_bytes: 8192,
            }
            .validate(),
            "chunk over window",
        );
        assert_eq!(fat_chunk.context.code, "streaming.ranged.chunk-over-window");
        tight().validate().must();
    }

    /// A read above the payload ceiling is refused typed (the input
    /// crate's `MAX_READ_BYTES` ceiling, mirrored).
    #[test]
    fn oversized_read_is_refused_typed() {
        let (server, fetch, _) = served(pattern(100));
        let mut r = open(fetch, &server.url("media.bin"), tight());
        let err = testkit::assert_err(r.read(MAX_READ_BYTES + 1), "oversized read");
        assert_eq!(err.context.code, "streaming.ranged.read-over-budget");
    }

    /// Content-Range parsing: the accepted shapes and the typed
    /// refusal for every hostile one.
    #[test]
    fn content_range_parsing_is_strict() {
        assert_eq!(
            parse_content_range(Some("bytes 0-1023/4096")).must(),
            (0, 1023, Some(4096))
        );
        assert_eq!(
            parse_content_range(Some("bytes 5-9/*")).must(),
            (5, 9, None)
        );
        for hostile in [
            None,
            Some("0-1023/4096"),
            Some("bytes 0-1023"),
            Some("bytes -/4096"),
            Some("bytes x-y/4096"),
            Some("bytes 0-1023/many"),
        ] {
            let err = testkit::assert_err(parse_content_range(hostile), "hostile content-range");
            assert_eq!(err.context.code, "streaming.ranged.protocol-violation");
        }
    }

    /// The window-cache eviction math keeps the invariant
    /// `cached <= window_bytes` across arbitrary hit/miss/seek
    /// interleavings (property-style sweep over a fixed script).
    #[test]
    fn cache_invariant_holds_across_a_seek_storm() {
        let data = pattern(8000);
        let (server, fetch, _) = served(data.clone());
        let mut r = open(fetch, &server.url("media.bin"), tight());
        let jump = [0usize, 7000, 300, 7999, 4000, 1024, 6500];
        for &target in jump.iter() {
            r.seek(SeekOrigin::Start(target as u64))
                .unwrap_or_else(|e| unreachable!("{e}"));
            let piece = r.read(600).unwrap_or_else(|e| unreachable!("{e}"));
            let end = (target + 600).min(8000);
            assert_eq!(piece, &data[target..end]);
            assert!(
                r.cached_bytes() <= 4096,
                "budget held at target {target}: {}",
                r.cached_bytes()
            );
        }
    }
}
