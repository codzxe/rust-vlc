//! The HLS **VOD media-playlist session** (card 6.B1a, the card's
//! owned file): fetch the playlist, type it, size the segments, and
//! serve their concatenation as one continuous, **seekable**
//! [`SourceStream`](plugin_api::SourceStream).
//!
//! # Composition (the whole point of the card)
//!
//! [`open_vod`] is one call over the landed streaming stack: the
//! playlist, the per-segment size probes, and every segment body
//! travel through the same [`HttpFetch`](crate::fetch::HttpFetch)
//! port the scheme registry (6.A3) carries — the ureq client in
//! production, the mock testkit or any test adapter in tests — under
//! per-response memory budgets (MP §12). The stream it returns is a
//! plain `SourceStream`, so the demuxer that consumes it (MPEG-TS,
//! ADTS, …) has no idea the bytes arrived as N separate HTTP
//! resources: the existing container hosts are reused, not forked.
//!
//! # Known length (why the probes exist)
//!
//! The container layer refuses unknown-length sources at open
//! (`demux.source.unknown-length` — the pinned contract), so a VOD
//! session must know the concatenation's total before the first
//! read. [`open_vod`] therefore issues **one `Range: bytes=0-0`
//! probe per segment** after the playlist parses: a `206`'s
//! `Content-Range` total fixes that segment's length (the same probe
//! shape the ranged resource's open uses); a `200` (no-range server,
//! the 6.A2b shape) delivers the whole body, which fixes the length
//! *and* seeds the cache. A server that answers `bytes 0-0/*` keeps
//! the length unknown — that is the typed
//! `streaming.hls.unknown-segment-size` refusal, never a guess. The
//! probe cost is one extra request per segment at open; VOD
//! scheduling policy (batching, laziness) is 6.B1b/6.D1 scope,
//! recorded here as the tradeoff it is.
//!
//! # What B1a plays (VOD only, honestly refused otherwise)
//!
//! A media playlist is playable here when it is a **VOD** playlist:
//! `#EXT-X-ENDLIST` present, every segment whole-body (no
//! `#EXT-X-BYTERANGE`), no `#EXT-X-MAP` init section, no `#EXT-X-KEY`
//! (keys are *recorded* by the parser, never decrypted — that is a
//! later card), no `#EXT-X-DISCONTINUITY`. Each violation refuses
//! typed with a stable `streaming.hls.*` code naming the follow-up
//! card:
//!
//! | shape | typed code | follow-up |
//! |---|---|---|
//! | live (no `ENDLIST`) | `streaming.hls.live-playlist` | 6.B1b |
//! | discontinuity | `streaming.hls.discontinuity` | 6.B1b |
//! | `EXT-X-KEY` in effect | `streaming.hls.encrypted` | later card |
//! | `EXT-X-BYTERANGE` | `streaming.hls.byterange-segments` | later card |
//! | `EXT-X-MAP` init section | `streaming.hls.init-segment` | fMP4 lane |
//! | zero segments | `streaming.hls.empty-playlist` | — |
//! | master playlist | `streaming.hls.master-playlist` | 6.B2a |
//! | no `#EXTM3U` | `streaming.hls.not-hls` | — |
//! | bad numeric tag | `streaming.hls.bad-tag-value` | — |
//! | unknowable segment size | `streaming.hls.unknown-segment-size` | 6.B1b |
//!
//! # Seekability (the honest shape)
//!
//! VOD segments are immutable, so the concatenation is **fully
//! seekable**: the segment cache (default 64 MiB, the network-cache
//! budget, FIFO-evicted) serves repeats — the TS demuxer's
//! head-scan rewind after PSI collection is the canonical case —
//! and an evicted segment is simply **refetched** on demand, counted
//! in [`HlsVodStats::segment_refetches`] rather than hidden (the
//! honest linear cost of a small cache, same accounting posture as
//! the ranged resource's `refetches`). A seek past the known total
//! refuses typed (`streaming.hls.seek-out-of-range`). One over-budget
//! segment may be held outside the cache while it is being served —
//! the same window-plus-one-chunk tradeoff the ranged resource makes.
//!
//! # Bounded memory (MP §12)
//!
//! Nothing is allocated ahead: each segment body arrives whole under
//! [`HlsVodConfig::segment_response_limit`] (the port returns whole
//! bodies — the same documented tradeoff the ranged resource makes),
//! the cache is capped by [`HlsVodConfig::max_retained_bytes`], and
//! the segment *count* is naturally bounded by the playlist's own
//! response budget. A hostile segment over its budget fails
//! [`NetworkError::ResponseTooLarge`] typed
//! (`streaming.hls.response-over-budget`), never allocates.

use std::collections::VecDeque;
use std::sync::Arc;

use media_types::error::{
    AccessDenied, CorruptDetail, ErrorContext, FormatInfo, LimitKind, NetworkCause,
};
use media_types::ids::SourceUri;
use media_types::{EngineError, ErrorKind, PipelineStage};
use playlists::{HlsPlaylistError, MediaPlaylist};
use plugin_api::{SeekOrigin, SourcePosition, SourceStream};

use crate::error::NetworkError;
use crate::fetch::{ByteRange, DEFAULT_RESPONSE_LIMIT, HttpFetch, HttpRequest};

/// Default cache budget: 64 MiB, mirroring the ranged resource's
/// window default (`limits.cache_network_mib = 64`). Never allocated
/// up front — it caps lazily fetched segment bodies.
pub const DEFAULT_RETAINED_BYTES: usize = 64 * 1024 * 1024;

/// Cache-budget ceiling: the schema maximum (1024 MiB), matching
/// [`crate::ranged::MAX_WINDOW_BYTES`]. A larger budget is a typed
/// refusal, never a clamp (MP §5).
pub const MAX_RETAINED_BYTES: usize = 1024 * 1024 * 1024;

/// Ceiling on a single `read(len)` result, mirroring the input
/// crate's payload ceiling and the ranged resource's restatement.
pub const MAX_READ_BYTES: usize = 64 * 1024 * 1024;

/// The stage tag every session error carries (sources fail while
/// being opened or read — the input-layer convention).
const STAGE: PipelineStage = PipelineStage::Open;

/// The HLS format tag attached to `Unsupported` refusals (the
/// known-format info of doc 01 §3.4 — the session *did* recognize
/// the container family before refusing the shape).
fn hls_format() -> Option<FormatInfo> {
    Some(FormatInfo {
        name: Some("hls-media-playlist".to_owned()),
        sniffed: Vec::new(),
    })
}

/// Mint a stage-`Open` error with a stable dotted code.
fn mint(kind: ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(STAGE, kind, ErrorContext::new(code))
}

/// An `Unsupported` refusal with the HLS format attached.
fn refuse(code: &'static str) -> EngineError {
    mint(ErrorKind::Unsupported { format: hls_format() }, code)
}

/// Session configuration (all budgets explicit; see the module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HlsVodConfig {
    /// Per-response memory budget for the playlist fetch itself.
    pub playlist_response_limit: u64,
    /// Per-response memory budget for each segment fetch (probe and
    /// body alike — a probe's `206` is one byte, a no-range probe's
    /// `200` is the whole body, budgeted here).
    pub segment_response_limit: u64,
    /// Cap on the segment cache (FIFO-evicted; see the module docs).
    pub max_retained_bytes: usize,
}

impl Default for HlsVodConfig {
    fn default() -> Self {
        HlsVodConfig {
            playlist_response_limit: DEFAULT_RESPONSE_LIMIT,
            segment_response_limit: DEFAULT_RESPONSE_LIMIT,
            max_retained_bytes: DEFAULT_RETAINED_BYTES,
        }
    }
}

impl HlsVodConfig {
    /// Validate the budgets (typed refusal on a zero limit or an
    /// over-ceiling cache — never a clamp, MP §5).
    ///
    /// # Errors
    /// [`EngineError`] `streaming.hls.config` — a zero response
    /// budget or a cache budget over [`MAX_RETAINED_BYTES`].
    fn validate(&self) -> Result<(), EngineError> {
        if self.playlist_response_limit == 0 || self.segment_response_limit == 0 {
            return Err(mint(
                ErrorKind::ResourceLimit {
                    limit: LimitKind::Memory,
                    budget: 0,
                    attempted: 0,
                },
                "streaming.hls.config",
            ));
        }
        if self.max_retained_bytes > MAX_RETAINED_BYTES {
            return Err(mint(
                ErrorKind::ResourceLimit {
                    limit: LimitKind::Memory,
                    budget: MAX_RETAINED_BYTES as u64,
                    attempted: self.max_retained_bytes as u64,
                },
                "streaming.hls.config",
            ));
        }
        Ok(())
    }
}

/// What the session did, for tests and honest logging.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HlsVodStats {
    /// HTTP requests issued (playlist + probes + segment bodies).
    pub requests: u64,
    /// Segment size probes issued (the `bytes=0-0` opens).
    pub size_probes: u64,
    /// Segment bodies fetched.
    pub segments_fetched: u64,
    /// Fetches of a segment that had been fetched (and evicted)
    /// before — the honest refetch cost of a small cache.
    pub segment_refetches: u64,
    /// Sum of all body bytes received (probes included).
    pub bytes_fetched: u64,
}

/// One open VOD media-playlist session: the typed playlist, the
/// segment offset table, and the bounded segment cache. Constructed
/// by [`open_vod`].
pub struct HlsVodStream {
    fetch: Arc<dyn HttpFetch>,
    uri: SourceUri,
    /// The typed model the session was opened from (advisory
    /// surface for callers: EXTINF total, target duration, sequence).
    playlist_model: MediaPlaylist,
    /// Absolute segment URIs, playlist order.
    uris: Vec<String>,
    /// Cumulative segment start offsets; `starts[i]` is segment
    /// `i`'s first byte, `starts[n]` is the concatenation total.
    starts: Vec<u64>,
    /// The segment cache: `cache[i]` holds segment `i`'s body while
    /// it is retained (FIFO-evicted under the budget).
    cache: Vec<Option<Vec<u8>>>,
    /// Insertion order of the retained segments (front = oldest).
    cache_order: VecDeque<usize>,
    cached_bytes: usize,
    /// The over-budget segment currently being served (held outside
    /// the cache; replaced when the position leaves it).
    active: Option<(usize, Vec<u8>)>,
    /// Whether segment `i` was ever fetched (refetch accounting).
    ever_fetched: Vec<bool>,
    pos: u64,
    config: HlsVodConfig,
    stats: HlsVodStats,
}

impl HlsVodStream {
    /// The typed model the session was opened from (advisory fields
    /// for the UI: EXTINF total, target duration, media sequence).
    #[must_use]
    pub fn playlist(&self) -> &MediaPlaylist {
        &self.playlist_model
    }

    /// What the session has done so far.
    #[must_use]
    pub fn stats(&self) -> HlsVodStats {
        self.stats
    }

    /// The segment index containing byte `pos`, or `None` at/past
    /// the end. Empty segments are skipped by construction (their
    /// start offsets coincide).
    fn segment_index(&self, pos: u64) -> Option<usize> {
        let n = self.uris.len();
        // Last i with starts[i] <= pos, among starts[0..n].
        let i = self.starts[..n].partition_point(|&s| s <= pos);
        // i is the count of starts <= pos; the containing segment is
        // i-1. pos < starts[n] (checked by callers) guarantees i >= 1.
        i.checked_sub(1).filter(|&i| i < n)
    }

    /// Fetch segment `i` whole and ensure it is available (cache or
    /// active slot).
    fn ensure_segment(&mut self, i: usize) -> Result<(), EngineError> {
        if self.cache[i].is_some() || matches!(&self.active, Some((s, _)) if *s == i) {
            return Ok(());
        }
        let uri = self.uris[i].clone();
        let request =
            HttpRequest::get(uri).with_response_limit(self.config.segment_response_limit);
        self.stats.requests += 1;
        let body = match self.fetch.fetch(request) {
            Ok(resp) if resp.is_success() => resp.body,
            Ok(resp) => {
                // The port maps non-2xx to `HttpStatus` errors, but a
                // transport that surfaces the response instead gets
                // the same typed refusal here (never a silent body).
                let code = resp.status;
                return Err(map_fetch_err(NetworkError::HttpStatus {
                    code,
                    is_client_error: (400..=499).contains(&code),
                    is_server_error: (500..=599).contains(&code),
                }));
            }
            Err(err) => return Err(map_fetch_err(err)),
        };
        self.stats.segments_fetched += 1;
        self.stats.bytes_fetched += body.len() as u64;
        if self.ever_fetched[i] {
            self.stats.segment_refetches += 1;
        }
        self.ever_fetched[i] = true;
        if body.len() <= self.config.max_retained_bytes {
            self.evict_for(body.len());
            self.cache_order.push_back(i);
            self.cached_bytes += body.len();
            self.cache[i] = Some(body);
        } else {
            // One over-budget segment may be held outside the cache
            // while serving from it (the module-docs tradeoff).
            self.active = Some((i, body));
        }
        Ok(())
    }

    /// Evict oldest cached segments until `incoming` more bytes fit
    /// the budget (never evicts nothing when nothing needs to go).
    fn evict_for(&mut self, incoming: usize) {
        while self.cached_bytes + incoming > self.config.max_retained_bytes {
            match self.cache_order.pop_front() {
                Some(i) => {
                    if let Some(body) = self.cache[i].take() {
                        self.cached_bytes -= body.len();
                    }
                }
                None => break,
            }
        }
    }

    /// Serve up to `len` bytes at `pos`, fetching segments as needed.
    fn serve(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        let mut out = Vec::new();
        while out.len() < len {
            let Some(seg) = self.segment_index(self.pos) else {
                break; // EOF: position at/past the concatenation end
            };
            self.ensure_segment(seg)?;
            let seg_start = self.starts[seg];
            let off = (self.pos - seg_start) as usize;
            // Copy out of the cache or the active slot (the borrow
            // ends before the next loop iteration's mutations).
            let served = if let Some(body) = self.cache[seg].as_ref() {
                let take = (body.len() - off).min(len - out.len());
                out.extend_from_slice(&body[off..off + take]);
                take
            } else if let Some((_, body)) = self.active.as_ref() {
                let take = (body.len() - off).min(len - out.len());
                out.extend_from_slice(&body[off..off + take]);
                take
            } else {
                // ensure_segment guaranteed one of the two slots; an
                // empty (0-byte) segment reaches here with take == 0
                // handled above. Unreachable otherwise.
                break;
            };
            if served == 0 {
                // Zero-byte segment: advance past it (its start range
                // is empty; segment_index already skips it, but a
                // same-offset next segment would loop forever).
                self.pos = self.starts[seg + 1];
                continue;
            }
            self.pos += served as u64;
        }
        Ok(out)
    }

    /// Seek target resolved against the current position (the input
    /// contract's shared math; `i128` so a negative `Current` cannot
    /// overflow).
    fn resolve_origin(&self, origin: SeekOrigin) -> Result<u64, EngineError> {
        match origin {
            SeekOrigin::Start(to) => Ok(to),
            SeekOrigin::Current(delta) => {
                let target = i128::from(self.pos) + i128::from(delta);
                if target < 0 {
                    Err(mint(
                        ErrorKind::Inaccessible {
                            reason: AccessDenied::NotFound,
                        },
                        "streaming.hls.seek-before-start",
                    ))
                } else if target > i128::from(u64::MAX) {
                    Err(mint(
                        ErrorKind::Inaccessible {
                            reason: AccessDenied::NotFound,
                        },
                        "streaming.hls.seek-overflow",
                    ))
                } else {
                    Ok(target as u64)
                }
            }
        }
    }
}

impl SourceStream for HlsVodStream {
    fn uri(&self) -> &SourceUri {
        &self.uri
    }

    fn size(&self) -> Option<u64> {
        // Known from the probes at open (the module docs explain why
        // the container layer needs it).
        Some(self.starts[self.uris.len()])
    }

    fn supports_seek(&self) -> bool {
        // Fully: VOD segments are immutable; cache hits are free and
        // misses refetch on demand (accounted in the stats).
        true
    }

    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        self.serve(len.min(MAX_READ_BYTES))
    }

    fn seek(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError> {
        let target = self.resolve_origin(origin)?;
        if target > self.starts[self.uris.len()] {
            return Err(mint(
                ErrorKind::Inaccessible {
                    reason: AccessDenied::NotFound,
                },
                "streaming.hls.seek-out-of-range",
            ));
        }
        self.pos = target;
        Ok(SourcePosition(target))
    }

    fn position(&self) -> SourcePosition {
        SourcePosition(self.pos)
    }
}

/// Open a VOD HLS media playlist at `url` through `fetch`.
///
/// One playlist GET (budgeted by
/// [`HlsVodConfig::playlist_response_limit`]), the typed model from
/// [`playlists::parse_media_playlist`], the B1a playability checks,
/// then one `Range: bytes=0-0` size probe per segment (see the module
/// docs) — leaving a fully seekable, known-length stream over the
/// segment concatenation.
///
/// # Errors
/// [`EngineError`] (stage `Open`): transport failures mapped from
/// [`NetworkError`] (`streaming.hls.fetch-failed` /
/// `.response-over-budget`), the playlist refusals of the table in
/// the module docs, and `streaming.hls.playlist-not-utf8` /
/// `.playlist-status` / `.invalid-content-range` /
/// `.unknown-segment-size` for the body-level shapes.
pub fn open_vod(
    fetch: Arc<dyn HttpFetch>,
    url: &str,
    config: HlsVodConfig,
) -> Result<HlsVodStream, EngineError> {
    config.validate()?;
    // The port's URL grammar, refused before any socket (same early
    // rejection the ranged resource performs).
    crate::fetch::Url::parse(url).map_err(map_fetch_err)?;
    let uri = SourceUri::parse(url)?;

    let request =
        HttpRequest::get(url.to_owned()).with_response_limit(config.playlist_response_limit);
    let body = match fetch.fetch(request) {
        Ok(resp) if resp.is_success() => resp.body,
        Ok(resp) => {
            let code = resp.status;
            return Err(map_fetch_err(NetworkError::HttpStatus {
                code,
                is_client_error: (400..=499).contains(&code),
                is_server_error: (500..=599).contains(&code),
            }));
        }
        Err(err) => return Err(map_fetch_err(err)),
    };
    let text = std::str::from_utf8(&body)
        .map_err(|_| playlist_corrupt("streaming.hls.playlist-not-utf8"))?;
    let (model, _warnings) = parse_media_playlist_typed(url, text)?;

    // ---- B1a playability (the module-docs refusal table) ----
    if !model.end_list {
        return Err(refuse("streaming.hls.live-playlist"));
    }
    if model.segments.is_empty() {
        return Err(refuse("streaming.hls.empty-playlist"));
    }
    if model.segments.iter().any(|s| s.key.is_some()) {
        return Err(refuse("streaming.hls.encrypted"));
    }
    if model.segments.iter().any(|s| s.byte_range.is_some()) {
        return Err(refuse("streaming.hls.byterange-segments"));
    }
    if model.segments.iter().any(|s| s.map.is_some()) {
        return Err(refuse("streaming.hls.init-segment"));
    }
    if model.segments.iter().any(|s| s.discontinuity) {
        return Err(refuse("streaming.hls.discontinuity"));
    }

    // ---- Size probes: one bytes=0-0 per segment (module docs) ----
    let n = model.segments.len();
    let mut starts = Vec::with_capacity(n + 1);
    starts.push(0u64);
    let mut cache = vec![None; n];
    let mut cache_order = VecDeque::new();
    let mut cached_bytes = 0usize;
    let mut stats = HlsVodStats {
        requests: 1, // the playlist fetch above
        ..HlsVodStats::default()
    };
    for (i, segment) in model.segments.iter().enumerate() {
        let request = HttpRequest::get(segment.uri.clone())
            .with_range(ByteRange::Bounded {
                start: 0,
                end_exclusive: 1,
            })
            .with_response_limit(config.segment_response_limit);
        stats.requests += 1;
        stats.size_probes += 1;
        match fetch.fetch(request) {
            Ok(resp) if resp.status == 206 => {
                let header = resp.header("content-range");
                let total = match parse_content_range_total(header) {
                    Some(total) => total,
                    None if header.is_some_and(|h| h.contains('*')) => {
                        // `bytes 0-0/*` — the server will not name the
                        // total; the container layer needs one (module
                        // docs), so this is the typed refusal, not a
                        // guess (MP §8).
                        return Err(refuse("streaming.hls.unknown-segment-size"));
                    }
                    None => return Err(invalid_content_range()),
                };
                stats.bytes_fetched += resp.body.len() as u64;
                starts.push(starts[i] + total);
            }
            Ok(resp) if resp.is_success() => {
                // No-range server (the 6.A2b shape): the whole body
                // fixes the length and seeds the cache.
                let len = resp.body.len() as u64;
                stats.bytes_fetched += len;
                if resp.body.len() <= config.max_retained_bytes {
                    cache[i] = Some(resp.body);
                    cache_order.push_back(i);
                    cached_bytes += len as usize;
                }
                starts.push(starts[i] + len);
            }
            Ok(resp) => {
                let code = resp.status;
                return Err(map_fetch_err(NetworkError::HttpStatus {
                    code,
                    is_client_error: (400..=499).contains(&code),
                    is_server_error: (500..=599).contains(&code),
                }));
            }
            Err(NetworkError::RangeNotSatisfiable) => {
                // `bytes=0-0` unsatisfiable: the empty-segment shape
                // (the ranged resource's open treats its probe the
                // same way).
                starts.push(starts[i]);
            }
            Err(err) => return Err(map_fetch_err(err)),
        }
    }

    let uris = model.segments.iter().map(|s| s.uri.clone()).collect();
    // Segments already held (the no-range probe seeded them) count as
    // fetched — only their later re-fetches are refetches.
    let ever_fetched = cache.iter().map(|c| c.is_some()).collect();
    Ok(HlsVodStream {
        fetch,
        uri,
        playlist_model: model,
        uris,
        starts,
        cache,
        cache_order,
        cached_bytes,
        active: None,
        ever_fetched,
        pos: 0,
        config,
        stats,
    })
}

/// `Content-Range: bytes 0-0/<total>` → `<total>`; `*` (unknown) and
/// every malformed shape return `None` (the caller refuses typed).
fn parse_content_range_total(header: Option<&str>) -> Option<u64> {
    let header = header?.trim();
    let rest = header.strip_prefix("bytes ")?;
    let total = rest.rsplit('/').next()?;
    if total == "*" {
        return None;
    }
    total.parse::<u64>().ok()
}

/// A `206` whose `Content-Range` is missing or malformed.
fn invalid_content_range() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        "streaming.hls.invalid-content-range",
    )
}

/// Map the playlists crate's typed refusals into the engine taxonomy
/// (each cause keeps its own dotted code so tests and UI copy can
/// match on vocabulary, not Display text).
fn parse_media_playlist_typed(
    base: &str,
    text: &str,
) -> Result<(MediaPlaylist, Vec<playlists::ParseWarning>), EngineError> {
    playlists::parse_media_playlist(Some(base), text).map_err(|err| {
        let code = match &err {
            HlsPlaylistError::NotHls => "streaming.hls.not-hls",
            HlsPlaylistError::MasterPlaylist => "streaming.hls.master-playlist",
            HlsPlaylistError::BadTagValue { .. } => "streaming.hls.bad-tag-value",
            HlsPlaylistError::Parse { .. } => "streaming.hls.parse-failed",
        };
        refuse(code)
    })
}

/// A malformed playlist body (not UTF-8): typed `Corrupt`, never a
/// guess at an encoding.
fn playlist_corrupt(code: &'static str) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        code,
    )
}

/// Fold a transport [`NetworkError`] into the engine taxonomy, HLS
/// session flavor (the ranged resource's mapping shape, `hls` codes).
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
                "streaming.hls.response-over-budget",
            );
        }
        NetworkError::UnsupportedScheme { .. }
        | NetworkError::ProtocolViolation { .. }
        | NetworkError::InvalidContentRange
        | NetworkError::RangeNotSatisfiable => {
            return refuse("streaming.hls.fetch-refused");
        }
        NetworkError::Io { .. } => NetworkCause::ConnectionLost,
    };
    mint(ErrorKind::Network { cause }, "streaming.hls.fetch-failed")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch::HttpResponse;
    use crate::mockhttp::{MockClient, MockConfig, MockServer};

    /// An in-test transport that routes by URL path through the REAL
    /// [`HttpFetch`] port — the same surface the ureq client and the
    /// mock testkit implement (no socket; unit-test speed). Honors
    /// `Range:` and enforces the response budget like the real ones.
    struct RouterFetch {
        routes: Vec<(String, Vec<u8>)>,
    }

    impl RouterFetch {
        fn respond(&self, request: &HttpRequest) -> HttpResponse {
            let path = match crate::fetch::Url::parse(&request.url) {
                Ok(url) => url.path_and_query,
                Err(err) => unreachable!("router URLs are valid: {err}"),
            };
            let Some((_, body)) = self.routes.iter().find(|(p, _)| *p == path) else {
                return HttpResponse {
                    status: 404,
                    reason: "Not Found".to_owned(),
                    headers: Vec::new(),
                    body: Vec::new(),
                };
            };
            match request.range {
                Some(ByteRange::Bounded { start, end_exclusive }) => {
                    let len = body.len() as u64;
                    if len == 0 || start >= len {
                        return HttpResponse {
                            status: 416,
                            reason: "Range Not Satisfiable".to_owned(),
                            headers: Vec::new(),
                            body: Vec::new(),
                        };
                    }
                    let last = (end_exclusive - 1).min(len - 1);
                    let slice = &body[start as usize..=last as usize];
                    HttpResponse {
                        status: 206,
                        reason: "Partial Content".to_owned(),
                        headers: vec![(
                            "Content-Range".to_owned(),
                            format!("bytes {start}-{last}/{len}"),
                        )],
                        body: slice.to_vec(),
                    }
                }
                _ => HttpResponse {
                    status: 200,
                    reason: "OK".to_owned(),
                    headers: Vec::new(),
                    body: body.clone(),
                },
            }
        }
    }

    impl HttpFetch for RouterFetch {
        fn fetch(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError> {
            let resp = self.respond(&request);
            // The port contract: a body over the request's budget
            // fails `ResponseTooLarge` instead of allocating (the
            // ureq client and the mock testkit both enforce it; this
            // adapter implements the same contract).
            if resp.body.len() as u64 > request.response_limit {
                return Err(NetworkError::ResponseTooLarge {
                    limit: request.response_limit,
                    actual: resp.body.len() as u64,
                });
            }
            Ok(resp)
        }
    }

    fn router(routes: &[(&str, &[u8])]) -> Arc<dyn HttpFetch> {
        Arc::new(RouterFetch {
            routes: routes
                .iter()
                .map(|(p, b)| ((*p).to_owned(), b.to_vec()))
                .collect(),
        })
    }

    const PLAIN_VOD: &str = "#EXTM3U\n\
        #EXT-X-VERSION:3\n\
        #EXT-X-TARGETDURATION:1\n\
        #EXT-X-MEDIA-SEQUENCE:0\n\
        #EXT-X-PLAYLIST-TYPE:VOD\n\
        #EXTINF:1.0,\n\
        seg0.ts\n\
        #EXTINF:1.0,\n\
        seg1.ts\n\
        #EXT-X-ENDLIST\n";

    /// The happy path: known length from the probes, the
    /// concatenation byte-exact and in order, size stable at open.
    #[test]
    fn vod_concatenation_is_byte_exact_and_ordered() {
        let fetch = router(&[
            ("/pl.m3u8", PLAIN_VOD.as_bytes()),
            ("/seg0.ts", b"AAAA"),
            ("/seg1.ts", b"BBBBBB"),
        ]);
        let mut stream =
            match open_vod(fetch, "http://unit.test/pl.m3u8", HlsVodConfig::default()) {
                Ok(s) => s,
                Err(e) => unreachable!("VOD must open: {e}"),
            };
        assert_eq!(stream.size(), Some(10), "probes fixed the total at open");
        assert!(stream.supports_seek());
        let mut all = Vec::new();
        loop {
            let chunk = match stream.read(3) {
                Ok(c) => c,
                Err(e) => unreachable!("read: {e}"),
            };
            if chunk.is_empty() {
                break;
            }
            all.extend_from_slice(&chunk);
        }
        assert_eq!(all, b"AAAABBBBBB", "concatenation byte-exact");
        let stats = stream.stats();
        assert_eq!(stats.segments_fetched, 2);
        assert_eq!(stats.size_probes, 2);
        assert_eq!(stats.requests, 5, "playlist + 2 probes + 2 bodies");
        assert_eq!(stats.bytes_fetched, 12, "10 body bytes + 2 probe bytes");
        assert_eq!(stats.segment_refetches, 0);
    }

    /// The refusal table: each B1b+ shape refuses with its own code,
    /// before any probe goes out.
    #[test]
    fn b1b_shapes_refuse_typed_with_their_codes() {
        fn code_for(playlist: &str) -> String {
            let fetch = router(&[("/pl.m3u8", playlist.as_bytes()), ("/s.ts", b"x")]);
            match open_vod(fetch, "http://unit.test/pl.m3u8", HlsVodConfig::default()) {
                Ok(_) => unreachable!("shape must refuse: {playlist:?}"),
                Err(e) => e.context.code.clone(),
            }
        }
        assert_eq!(
            code_for("#EXTM3U\n#EXTINF:1.0,\ns.ts\n"),
            "streaming.hls.live-playlist"
        );
        assert_eq!(
            code_for("#EXTM3U\n#EXT-X-ENDLIST\n"),
            "streaming.hls.empty-playlist"
        );
        assert_eq!(
            code_for(
                "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"k\"\n#EXTINF:1.0,\ns.ts\n#EXT-X-ENDLIST\n"
            ),
            "streaming.hls.encrypted"
        );
        assert_eq!(
            code_for(
                "#EXTM3U\n#EXT-X-BYTERANGE:100@0\n#EXTINF:1.0,\ns.ts\n#EXT-X-ENDLIST\n"
            ),
            "streaming.hls.byterange-segments"
        );
        assert_eq!(
            code_for(
                "#EXTM3U\n#EXT-X-MAP:URI=\"i.mp4\"\n#EXTINF:1.0,\ns.m4s\n#EXT-X-ENDLIST\n"
            ),
            "streaming.hls.init-segment"
        );
        assert_eq!(
            code_for(
                "#EXTM3U\n#EXTINF:1.0,\ns.ts\n#EXT-X-DISCONTINUITY\n#EXTINF:1.0,\ns.ts\n#EXT-X-ENDLIST\n"
            ),
            "streaming.hls.discontinuity"
        );
        assert_eq!(
            code_for("#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1\nv.m3u8\n"),
            "streaming.hls.master-playlist"
        );
        assert_eq!(
            code_for("#EXTM3U\n#EXT-X-TARGETDURATION:x\n#EXTINF:1.0,\ns.ts\n#EXT-X-ENDLIST\n"),
            "streaming.hls.bad-tag-value"
        );
        assert_eq!(code_for("s.ts\n"), "streaming.hls.not-hls");
    }

    /// Segment-fetch failures surface typed (404 → engine Network).
    #[test]
    fn missing_segment_fails_typed_at_read_time() {
        let fetch = router(&[
            ("/pl.m3u8", PLAIN_VOD.as_bytes()),
            ("/seg0.ts", b"AAAA"),
        ]);
        let mut stream =
            match open_vod(fetch, "http://unit.test/pl.m3u8", HlsVodConfig::default()) {
                Ok(s) => s,
                Err(e) => unreachable!("open: {e}"),
            };
        let first = match stream.read(4) {
            Ok(c) => c,
            Err(e) => unreachable!("seg0 present: {e}"),
        };
        assert_eq!(first, b"AAAA");
        let Err(err) = stream.read(4) else {
            unreachable!("seg1 is a 404 — read must fail");
        };
        assert!(
            matches!(
                err.kind,
                media_types::ErrorKind::Network {
                    cause: NetworkCause::ConnectionLost
                }
            ),
            "404 segment maps to a typed network error, got {:?}",
            err.kind
        );
        assert_eq!(err.context.code, "streaming.hls.fetch-failed");
    }

    /// A segment over its response budget refuses typed, never
    /// allocates (MP §12). The probe (1 byte) fits; the body does not.
    #[test]
    fn oversize_segment_refuses_over_budget() {
        let fetch = router(&[
            ("/pl.m3u8", PLAIN_VOD.as_bytes()),
            ("/seg0.ts", b"AAAA"),
            ("/seg1.ts", b"BBBBBBBBBB"),
        ]);
        let config = HlsVodConfig {
            playlist_response_limit: 1024,
            segment_response_limit: 8, // seg1's body is 10 bytes
            max_retained_bytes: 1024,
        };
        let mut stream = match open_vod(fetch, "http://unit.test/pl.m3u8", config) {
            Ok(s) => s,
            Err(e) => unreachable!("open (probes are 1-byte 206s): {e}"),
        };
        let Ok(_) = stream.read(4) else {
            unreachable!("seg0 fits");
        };
        let Err(err) = stream.read(4) else {
            unreachable!("seg1 body is over budget");
        };
        assert_eq!(err.context.code, "streaming.hls.response-over-budget");
        assert!(matches!(
            err.kind,
            media_types::ErrorKind::ResourceLimit { .. }
        ));
    }

    /// Seek: backward into the cache is free (the TS demuxer's head
    /// rewind), backward into an evicted segment refetches (counted),
    /// forward is fetch-on-demand, past-the-end refuses typed.
    #[test]
    fn seek_shapes_are_honest() {
        let fetch = router(&[
            ("/pl.m3u8", PLAIN_VOD.as_bytes()),
            ("/seg0.ts", b"AAAA"),
            ("/seg1.ts", b"BBBBBB"),
        ]);
        let config = HlsVodConfig {
            playlist_response_limit: 1024,
            segment_response_limit: 1024,
            max_retained_bytes: 4, // only one 4-byte segment fits
        };
        let mut stream = match open_vod(fetch, "http://unit.test/pl.m3u8", config) {
            Ok(s) => s,
            Err(e) => unreachable!("open: {e}"),
        };
        let head = match stream.read(4) {
            Ok(c) => c,
            Err(e) => unreachable!("read: {e}"),
        };
        assert_eq!(head, b"AAAA");
        // Backward within the cache: free, no new request.
        let requests_before = stream.stats().requests;
        let pos = match stream.seek(SeekOrigin::Start(0)) {
            Ok(p) => p,
            Err(e) => unreachable!("seek to 0: {e}"),
        };
        assert_eq!(pos, SourcePosition(0));
        let again = match stream.read(4) {
            Ok(c) => c,
            Err(e) => unreachable!("cached re-read: {e}"),
        };
        assert_eq!(again, b"AAAA");
        assert_eq!(stream.stats().requests, requests_before);
        // Drain past seg1: the cache evicts seg0 (budget 4).
        let mut sink = Vec::new();
        loop {
            let chunk = match stream.read(64) {
                Ok(c) => c,
                Err(e) => unreachable!("drain: {e}"),
            };
            if chunk.is_empty() {
                break;
            }
            sink.extend_from_slice(&chunk);
        }
        assert_eq!(sink, b"AAAABBBBBB");
        assert_eq!(stream.size(), Some(10));
        // Backward into the evicted segment: refetched, counted.
        let refetches_before = stream.stats().segment_refetches;
        let pos = match stream.seek(SeekOrigin::Start(0)) {
            Ok(p) => p,
            Err(e) => unreachable!("VOD segments are immutable — seek refetches: {e}"),
        };
        assert_eq!(pos, SourcePosition(0));
        let re_head = match stream.read(4) {
            Ok(c) => c,
            Err(e) => unreachable!("refetched read: {e}"),
        };
        assert_eq!(re_head, b"AAAA");
        assert_eq!(stream.stats().segment_refetches, refetches_before + 1);
        // Past the known total: typed out-of-range.
        let Err(err) = stream.seek(SeekOrigin::Start(999)) else {
            unreachable!("past the end must refuse");
        };
        assert_eq!(err.context.code, "streaming.hls.seek-out-of-range");
    }

    /// Relative segment URIs resolve against the playlist URL; the
    /// request log proves the probe/fetch wire shape over a real
    /// socket.
    #[test]
    fn relative_uris_resolve_against_the_playlist_url() {
        let playlist = "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXTINF:1.0,\nseg0.ts\n#EXT-X-ENDLIST\n";
        let server = match MockServer::spawn(MockConfig {
            routes: vec![
                ("/hls/pl.m3u8".to_owned(), playlist.as_bytes().to_vec()),
                ("/hls/seg0.ts".to_owned(), b"TS-BYTES".to_vec()),
            ],
            ..MockConfig::default()
        }) {
            Ok(s) => s,
            Err(e) => unreachable!("spawn: {e}"),
        };
        let fetch: Arc<dyn HttpFetch> = Arc::new(MockClient::new());
        let mut stream =
            match open_vod(fetch, &server.url("hls/pl.m3u8"), HlsVodConfig::default()) {
                Ok(s) => s,
                Err(e) => unreachable!("open over real socket: {e}"),
            };
        assert_eq!(stream.size(), Some(8), "probe fixed the total");
        let mut all = Vec::new();
        loop {
            let chunk = match stream.read(64) {
                Ok(c) => c,
                Err(e) => unreachable!("read: {e}"),
            };
            if chunk.is_empty() {
                break;
            }
            all.extend_from_slice(&chunk);
        }
        assert_eq!(all, b"TS-BYTES");
        let requests = server.requests();
        assert_eq!(
            requests.iter().map(|r| r.path.clone()).collect::<Vec<_>>(),
            vec!["/hls/pl.m3u8", "/hls/seg0.ts", "/hls/seg0.ts"],
            "playlist, probe, then the whole-body fetch"
        );
        assert_eq!(
            requests[1].header("range"),
            Some("bytes=0-0"),
            "the size probe asks for exactly one byte"
        );
        assert!(
            requests[2].header("range").is_none(),
            "the body fetch is a whole-body GET"
        );
    }

    /// Config validation: zero budgets and over-ceiling retention
    /// refuse typed at open, before any socket.
    #[test]
    fn config_bounds_refuse_typed() {
        let fetch = router(&[("/pl.m3u8", PLAIN_VOD.as_bytes())]);
        let zero = HlsVodConfig {
            segment_response_limit: 0,
            ..HlsVodConfig::default()
        };
        let Err(err) = open_vod(fetch.clone(), "http://unit.test/pl.m3u8", zero) else {
            unreachable!("zero budget must refuse");
        };
        assert_eq!(err.context.code, "streaming.hls.config");
        let over = HlsVodConfig {
            max_retained_bytes: MAX_RETAINED_BYTES + 1,
            ..HlsVodConfig::default()
        };
        let Err(err) = open_vod(fetch, "http://unit.test/pl.m3u8", over) else {
            unreachable!("over-ceiling retention must refuse");
        };
        assert_eq!(err.context.code, "streaming.hls.config");
    }

    /// A 206 whose Content-Range total is unknown (`*`) refuses
    /// typed — the length is a value, not a guess (MP §8).
    #[test]
    fn unknown_content_range_total_refuses_typed() {
        struct WildcardProbe;
        impl HttpFetch for WildcardProbe {
            fn fetch(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError> {
                if request.range.is_some() {
                    return Ok(HttpResponse {
                        status: 206,
                        reason: "Partial Content".to_owned(),
                        headers: vec![(
                            "Content-Range".to_owned(),
                            "bytes 0-0/*".to_owned(),
                        )],
                        body: b"x".to_vec(),
                    });
                }
                Ok(HttpResponse {
                    status: 200,
                    reason: "OK".to_owned(),
                    headers: Vec::new(),
                    body: PLAIN_VOD.as_bytes().to_vec(),
                })
            }
        }
        let fetch: Arc<dyn HttpFetch> = Arc::new(WildcardProbe);
        let Err(err) = open_vod(fetch, "http://unit.test/pl.m3u8", HlsVodConfig::default())
        else {
            unreachable!("unknown total must refuse");
        };
        assert_eq!(err.context.code, "streaming.hls.unknown-segment-size");
    }
}
