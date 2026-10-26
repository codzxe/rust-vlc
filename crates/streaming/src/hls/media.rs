//! The HLS **VOD media-playlist session** (card 6.B1a, the card's
//! owned file): fetch the playlist, type it, fetch the segments, and
//! serve their concatenation as one continuous
//! [`SourceStream`](plugin_api::SourceStream).
//!
//! # Composition (the whole point of the card)
//!
//! [`open_vod`] is one call over the landed streaming stack: the
//! playlist and every segment travel through the same
//! [`HttpFetch`](crate::fetch::HttpFetch) port the scheme registry
//! (6.A3) carries — the ureq client in production, the mock testkit
//! or any test adapter in tests — under per-response memory budgets
//! (MP §12). The stream it returns is a plain `SourceStream`, so the
//! demuxer that consumes it (MPEG-TS, ADTS, …) has no idea the bytes
//! arrived as N separate HTTP resources: the existing container
//! hosts are reused, not forked.
//!
//! # What B1a plays (VOD only, honestly refused otherwise)
//!
//! A media playlist is playable here when it is a **VOD** playlist:
//! `#EXT-X-ENDLIST` present, every segment whole-body (no
//! `#EXT-X-BYTERANGE`), no `#EXT-X-MAP` init section, no `#EXT-X-KEY`
//! (keys are *recorded* by the parser, never decrypted — that is a
//! later card). Each violation refuses typed with a stable
//! `streaming.hls.*` code naming the follow-up card:
//!
//! | shape | typed code | follow-up |
//! |---|---|---|
//! | live (no `ENDLIST`) | `streaming.hls.live-playlist` | 6.B1b — now [`crate::hls::live`] |
//! | `EXT-X-KEY` in effect | `streaming.hls.encrypted` | later card |
//! | `EXT-X-BYTERANGE` | `streaming.hls.byterange-segments` | later card |
//! | `EXT-X-MAP` init section | `streaming.hls.init-segment` | fMP4 lane |
//! | zero segments | `streaming.hls.empty-playlist` | — |
//! | master playlist | `streaming.hls.master-playlist` | 6.B2a |
//! | no `#EXTM3U` | `streaming.hls.not-hls` | — |
//! | bad numeric tag | `streaming.hls.bad-tag-value` | — |
//!
//! # What B1b added: discontinuities are recorded, not refused
//!
//! A `#EXT-X-DISCONTINUITY` marker no longer refuses the open (that
//! was B1a's honest refusal; this card moves it to support): the
//! concatenation continues byte-exact across the seam, and the
//! session **records** it — [`HlsVodStream::segment_boundaries`]
//! carries the full segment map (offset, absolute media sequence,
//! EXTINF duration, discontinuity flag) and
//! [`HlsVodStats::discontinuities`] counts the seams. The v0 stream
//! surface stays byte-concatenation: the demuxer consuming it gets
//! the honest answer at the container seam (a discontinuity is an
//! encoding-boundary statement — timestamps and codec parameters
//! may reset), never a silent decode across one. Live-window
//! sessions (no `ENDLIST`) are the sibling module [`crate::hls::live`].
//!
//! # Seekability (the honest shape)
//!
//! The concatenation is served **seekable within the retained
//! prefix** (default 64 MiB, the network-cache budget): the TS
//! demuxer's head scan rewinds after PSI collection, and that small
//! backward seek is exactly what the retained prefix serves. A
//! forward seek fetches and discards the segments it crosses
//! (counted in [`HlsVodStats::segments_skipped`]); a backward seek
//! below the retained prefix — only possible after the budget drops
//! the head — refuses typed (`streaming.hls.seek-unretained`) rather
//! than silently refetching. `size()` is `None` until every segment
//! has been fetched, then the exact total (a value and its absence
//! are different types, MP §8).
//!
//! # Bounded memory (MP §12)
//!
//! Nothing is allocated ahead: each segment body arrives whole under
//! [`HlsVodConfig::segment_response_limit`] (the port returns whole
//! bodies — the same documented tradeoff the ranged resource makes),
//! the retained prefix is capped by
//! [`HlsVodConfig::max_retained_bytes`], and the segment *count* is
//! naturally bounded by the playlist's own response budget. A
//! hostile segment over its budget fails
//! [`NetworkError::ResponseTooLarge`] typed
//! (`streaming.hls.segment-over-budget`), never allocates.

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

/// Default retention budget for the seekable prefix: 64 MiB,
/// mirroring the ranged resource's window default
/// (`limits.cache_network_mib = 64`). Never allocated up front — it
/// caps lazily fetched segment bodies.
pub const DEFAULT_RETAINED_BYTES: usize = 64 * 1024 * 1024;

/// Retention-budget ceiling: the schema maximum (1024 MiB), matching
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
pub(super) fn mint(kind: ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(STAGE, kind, ErrorContext::new(code))
}

/// An `Unsupported` refusal with the HLS format attached.
pub(super) fn refuse(code: &'static str) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: hls_format(),
        },
        code,
    )
}

/// Session configuration (all budgets explicit; see the module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HlsVodConfig {
    /// Per-response memory budget for the playlist fetch itself.
    pub playlist_response_limit: u64,
    /// Per-response memory budget for each segment fetch.
    pub segment_response_limit: u64,
    /// Cap on the retained (seekable) prefix of the concatenation.
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
    /// over-ceiling retention — never a clamp, MP §5).
    ///
    /// # Errors
    /// [`EngineError`] `streaming.hls.config` — a zero response
    /// budget or a retention budget over [`MAX_RETAINED_BYTES`].
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
    /// HTTP requests issued (playlist + probes + segments).
    pub requests: u64,
    /// Segment bodies fetched (probes excluded).
    pub segments_fetched: u64,
    /// Segment bodies fetched only to be discarded by a forward seek.
    pub segments_skipped: u64,
    /// Sum of all body bytes received (probe bodies included).
    pub bytes_fetched: u64,
    /// Size probes issued at open (the known-length contract).
    pub size_probes: u64,
    /// `#EXT-X-DISCONTINUITY` seams in the playlist (B1b: recorded,
    /// not refused — the per-segment map is
    /// [`HlsVodStream::segment_boundaries`]).
    pub discontinuities: u64,
}

/// One media segment's boundary in the served concatenation (card
/// 6.B1b's discontinuity contract): where the segment's bytes start,
/// which absolute `MEDIA-SEQUENCE` number it is, its advisory EXTINF
/// duration, and whether `#EXT-X-DISCONTINUITY` preceded it.
///
/// This is the **recorded** surface — the stream itself stays
/// byte-concatenation; a consumer that cannot span a discontinuity
/// seam gets its honest answer at the container boundary, and
/// nothing is silently decoded across a seam without this record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentBoundary {
    /// Concatenation offset where this segment's bytes start.
    pub offset: u64,
    /// Absolute segment number (`MEDIA-SEQUENCE` base + playlist
    /// index) — the identity a live window slides by.
    pub media_sequence: u64,
    /// The segment's `#EXTINF` duration, when the playlist carried a
    /// finite one.
    pub duration: Option<f64>,
    /// `#EXT-X-DISCONTINUITY` appeared before this segment.
    pub discontinuity: bool,
}

/// One open VOD media-playlist session: the segment queue plus the
/// retained-prefix buffer. Constructed by [`open_vod`].
pub struct HlsVodStream {
    fetch: Arc<dyn HttpFetch>,
    uri: SourceUri,
    /// The typed model the session was opened from (advisory
    /// surface for callers: EXTINF total, target duration, sequence).
    playlist_model: MediaPlaylist,
    /// Absolute segment URIs, playlist order.
    segments: Vec<String>,
    /// Index of the next segment to fetch.
    next: usize,
    /// The retained prefix: bytes `[retained_from, fetched_upto)`.
    retained: Vec<u8>,
    retained_from: u64,
    /// Total concatenation bytes fetched so far.
    fetched_upto: u64,
    pos: u64,
    config: HlsVodConfig,
    stats: HlsVodStats,
    /// The known concatenation total, fixed by the open-time size
    /// probes (the container layer refuses unknown-length sources,
    /// `demux.source.unknown-length` — the pinned contract).
    total_len: Option<u64>,
    /// Whole bodies that arrived during size probes (a no-range
    /// server answers a 1-byte probe with `200` + the full body):
    /// segment `i`'s paid-for bytes, consumed by [`Self::fetch_segment`].
    probe_cache: Vec<Option<Vec<u8>>>,
    /// The full segment map, fixed at open by the probe totals (the
    /// discontinuity record of card 6.B1b).
    boundaries: Vec<SegmentBoundary>,
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

    /// The full segment map of the concatenation (offset, absolute
    /// media sequence, EXTINF duration, discontinuity flag) — fixed
    /// at open by the probe totals, so the whole map is available
    /// before the first read. The discontinuity seams live here,
    /// recorded (never silently decoded across).
    #[must_use]
    pub fn segment_boundaries(&self) -> &[SegmentBoundary] {
        &self.boundaries
    }

    /// Fetch segment `i` and absorb its body into the buffer state.
    /// A probe-cached body (the no-range server's `200` answer to
    /// the size probe) is consumed without a second request.
    fn fetch_segment(&mut self, i: usize) -> Result<(), EngineError> {
        if let Some(body) = self.probe_cache[i].take() {
            self.stats.segments_fetched += 1;
            self.absorb(body);
            return Ok(());
        }
        let request = HttpRequest::get(self.segments[i].clone())
            .with_response_limit(self.config.segment_response_limit);
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
        self.absorb(body);
        Ok(())
    }

    /// Absorb a fetched body into the retained-prefix state (the
    /// budget rule of the module docs: retain what fits, drop the
    /// head when it does not, retain nothing for a single body over
    /// the whole budget).
    fn absorb(&mut self, body: Vec<u8>) {
        let start = self.fetched_upto;
        self.fetched_upto += body.len() as u64;
        if self.retained.len() + body.len() <= self.config.max_retained_bytes {
            self.retained.extend_from_slice(&body);
        } else {
            self.retained.clear();
            self.retained_from = start;
            if body.len() <= self.config.max_retained_bytes {
                self.retained = body;
            } else {
                self.retained_from = self.fetched_upto;
            }
        }
    }

    /// Serve up to `len` bytes at `pos`, fetching segments as needed.
    fn serve(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        let mut out = Vec::new();
        while out.len() < len {
            if self.pos < self.fetched_upto {
                let at = (self.pos - self.retained_from) as usize;
                let have = self.retained.len() - at;
                let take = have.min(len - out.len());
                out.extend_from_slice(&self.retained[at..at + take]);
                self.pos += take as u64;
            } else if self.next < self.segments.len() {
                let i = self.next;
                self.next += 1;
                self.fetch_segment(i)?;
            } else {
                break; // EOF: every segment fetched and consumed
            }
        }
        Ok(out)
    }

    /// Seek target resolved against the current position (the input
    /// contract's shared math; `i128` so a negative `Current` cannot
    /// overflow) — shared with the live session (6.B1b).
    fn resolve_origin(&self, origin: SeekOrigin) -> Result<u64, EngineError> {
        resolve_seek_origin(self.pos, origin)
    }
}

/// Resolve a seek origin against `pos` (the input contract's shared
/// math; `i128` so a negative `Current` cannot overflow). The VOD and
/// live sessions both refuse before-start/overflow typed with the
/// same codes.
pub(super) fn resolve_seek_origin(
    pos: u64,
    origin: SeekOrigin,
) -> Result<u64, EngineError> {
    match origin {
        SeekOrigin::Start(to) => Ok(to),
        SeekOrigin::Current(delta) => {
            let target = i128::from(pos) + i128::from(delta);
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

impl SourceStream for HlsVodStream {
    fn uri(&self) -> &SourceUri {
        &self.uri
    }

    fn size(&self) -> Option<u64> {
        // The open-time probes fix the total before the first read
        // (the container layer's unknown-length refusal, the pinned
        // contract). A stream whose probes somehow never ran keeps
        // the honest None (MP §8: a value and its absence are
        // different types).
        self.total_len
            .or((self.next == self.segments.len()).then_some(self.fetched_upto))
    }

    fn supports_seek(&self) -> bool {
        // Within the retained prefix, plus forward skip-fetches —
        // see the module docs for the honest boundary.
        true
    }

    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        self.serve(len.min(MAX_READ_BYTES))
    }

    fn seek(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError> {
        let target = self.resolve_origin(origin)?;
        if target < self.pos {
            // Backward: only within the retained prefix.
            if target < self.retained_from {
                return Err(mint(
                    ErrorKind::Inaccessible {
                        reason: AccessDenied::NotFound,
                    },
                    "streaming.hls.seek-unretained",
                ));
            }
        } else if target > self.fetched_upto {
            // Forward beyond fetched data: fetch and discard the
            // segments crossed (the honest linear cost, accounted).
            while self.fetched_upto < target && self.next < self.segments.len() {
                let i = self.next;
                self.next += 1;
                self.fetch_segment(i)?;
                self.stats.segments_skipped += 1;
            }
            if self.fetched_upto < target {
                return Err(mint(
                    ErrorKind::Inaccessible {
                        reason: AccessDenied::NotFound,
                    },
                    "streaming.hls.seek-out-of-range",
                ));
            }
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
/// and the segment queue — see the module docs for the composition
/// and the refusal table.
///
/// # Errors
/// [`EngineError`] (stage `Open`): transport failures mapped from
/// [`NetworkError`] (`streaming.hls.fetch-failed` /
/// `.response-over-budget`), the playlist refusals of the table in
/// the module docs, and `streaming.hls.playlist-not-utf8` /
/// `.playlist-status` for the two body-level shapes.
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
    // B1b note: a discontinuity no longer refuses — it is recorded
    // in the segment map and surfaced via stats (see the module
    // docs' "recorded, not refused" section).
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
    let discontinuities = model
        .segments
        .iter()
        .filter(|s| s.discontinuity)
        .count() as u64;

    let segments: Vec<String> = model.segments.iter().map(|s| s.uri.clone()).collect();
    // ---- The known-length probes (the module-docs contract) ----
    // The container layer refuses unknown-length sources at open
    // (`demux.source.unknown-length`), so a VOD session fixes the
    // concatenation total BEFORE the first read: one 1-byte Range
    // probe per segment. A `206` names the total in `Content-Range`
    // (one byte paid); a `200` (no-range server, the 6.A2b shape)
    // delivers the whole body — paid for, so it is cached as that
    // segment's content. Anything else is the typed
    // `streaming.hls.unknown-segment-size` refusal — never a guess.
    //
    // The probe totals also fix the full segment map (the B1b
    // discontinuity record) before the first read.
    let mut probe_cache: Vec<Option<Vec<u8>>> = Vec::with_capacity(segments.len());
    let mut boundaries: Vec<SegmentBoundary> = Vec::with_capacity(segments.len());
    let mut total: u64 = 0;
    let mut probes = 0u64;
    let mut probe_bytes = 0u64;
    for (i, url) in segments.iter().enumerate() {
        let request = HttpRequest::get(url.clone())
            .with_range(ByteRange::Bounded {
                start: 0,
                end_exclusive: 1,
            })
            .with_response_limit(config.segment_response_limit);
        let resp = match fetch.fetch(request) {
            Ok(resp) if resp.is_success() => resp,
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
        probes += 1;
        probe_bytes += resp.body.len() as u64;
        let segment = &model.segments[i];
        boundaries.push(SegmentBoundary {
            offset: total,
            media_sequence: model.media_sequence + i as u64,
            duration: segment.duration,
            discontinuity: segment.discontinuity,
        });
        if resp.status == 206 {
            let Some(len) = content_range_total(resp.header("content-range")) else {
                return Err(refuse("streaming.hls.unknown-segment-size"));
            };
            total += len;
            probe_cache.push(None);
        } else {
            // `200`: the whole body — the size AND the bytes.
            total += resp.body.len() as u64;
            probe_cache.push(Some(resp.body));
        }
    }
    Ok(HlsVodStream {
        fetch,
        uri,
        playlist_model: model,
        segments,
        next: 0,
        retained: Vec::new(),
        retained_from: 0,
        fetched_upto: 0,
        pos: 0,
        config,
        total_len: Some(total),
        probe_cache,
        boundaries,
        // The playlist fetch above is request #1; the probes follow.
        stats: HlsVodStats {
            requests: 1 + probes,
            size_probes: probes,
            bytes_fetched: probe_bytes,
            discontinuities,
            ..HlsVodStats::default()
        },
    })
}

/// Parse the total out of a `206`'s `Content-Range` header value
/// (`bytes 0-0/12345` → `12345`; `*` or malformed → `None`).
fn content_range_total(value: Option<&str>) -> Option<u64> {
    let value = value?.trim();
    let slash = value.rfind('/')?;
    let total = &value[slash + 1..];
    if total == "*" {
        return None;
    }
    total.parse::<u64>().ok()
}

/// Map the playlists crate's typed refusals into the engine taxonomy
/// (each cause keeps its own dotted code so tests and UI copy can
/// match on vocabulary, not Display text).
pub(super) fn parse_media_playlist_typed(
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
pub(super) fn playlist_corrupt(code: &'static str) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        code,
    )
}

/// Fold a transport [`NetworkError`] into the engine taxonomy, HLS
/// session flavor (the ranged resource's mapping shape, `hls` codes).
pub(super) fn map_fetch_err(err: NetworkError) -> EngineError {
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

/// Fetch and type one media playlist (shared by the VOD and live
/// sessions, 6.B1b's extraction of the identical open/reload
/// prelude): one budgeted GET, the success check, the UTF-8 check,
/// and the typed model. Returns the model plus the response body's
/// byte count — the caller folds both into its own stats accounting.
pub(super) fn fetch_playlist_model(
    fetch: &Arc<dyn HttpFetch>,
    url: &str,
    response_limit: u64,
) -> Result<(MediaPlaylist, u64), EngineError> {
    let request = HttpRequest::get(url.to_owned()).with_response_limit(response_limit);
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
    let bytes = body.len() as u64;
    let text =
        std::str::from_utf8(&body).map_err(|_| playlist_corrupt("streaming.hls.playlist-not-utf8"))?;
    let (model, _warnings) = parse_media_playlist_typed(url, text)?;
    Ok((model, bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch::HttpResponse;
    use crate::mockhttp::{MockClient, MockConfig, MockServer};

    /// An in-test transport that routes by URL path through the REAL
    /// [`HttpFetch`] port — the same surface the ureq client and the
    /// mock testkit implement (no socket; unit-test speed).
    struct RouterFetch {
        routes: Vec<(String, Vec<u8>)>,
    }

    impl RouterFetch {
        fn get(&self, path: &str) -> HttpResponse {
            match self.routes.iter().find(|(p, _)| p == path) {
                Some((_, body)) => HttpResponse {
                    status: 200,
                    reason: "OK".to_owned(),
                    headers: Vec::new(),
                    body: body.clone(),
                },
                None => HttpResponse {
                    status: 404,
                    reason: "Not Found".to_owned(),
                    headers: Vec::new(),
                    body: Vec::new(),
                },
            }
        }
    }

    impl HttpFetch for RouterFetch {
        fn fetch(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError> {
            let path = crate::fetch::Url::parse(&request.url)?.path_and_query;
            let resp = self.get(&path);
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

    /// The happy path: the concatenation is served byte-exact, in
    /// order, through the real port, and `size()` is known AT OPEN
    /// (the known-length contract: the probes fix the total before
    /// the first read — the container layer's unknown-length
    /// refusal is the pinned consumer).
    #[test]
    fn vod_concatenation_is_byte_exact_and_ordered() {
        let fetch = router(&[
            ("/pl.m3u8", PLAIN_VOD.as_bytes()),
            ("/seg0.ts", b"AAAA"),
            ("/seg1.ts", b"BBBBBB"),
        ]);
        let mut stream = match open_vod(fetch, "http://unit.test/pl.m3u8", HlsVodConfig::default())
        {
            Ok(s) => s,
            Err(e) => unreachable!("VOD must open: {e}"),
        };
        assert_eq!(stream.size(), Some(10), "total fixed by the probes at open");
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
        assert_eq!(stream.size(), Some(10), "exact total after the end");
        assert_eq!(stream.stats().segments_fetched, 2);
        // Playlist + 2 probes; the probe bodies (this router answers
        // `200` whole bodies — no Range support) ARE the segment
        // bodies, consumed from the probe cache without re-fetching.
        assert_eq!(stream.stats().requests, 3, "playlist + 2 probes");
        assert_eq!(stream.stats().size_probes, 2);
        assert_eq!(stream.stats().bytes_fetched, 10);
    }

    /// The refusal table: each still-refused shape refuses with its
    /// own code. (The discontinuity row moved to SUPPORT in 6.B1b —
    /// see `vod_discontinuity_is_recorded_not_refused` below.)
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
            code_for("#EXTM3U\n#EXT-X-BYTERANGE:100@0\n#EXTINF:1.0,\ns.ts\n#EXT-X-ENDLIST\n"),
            "streaming.hls.byterange-segments"
        );
        assert_eq!(
            code_for("#EXTM3U\n#EXT-X-MAP:URI=\"i.mp4\"\n#EXTINF:1.0,\ns.m4s\n#EXT-X-ENDLIST\n"),
            "streaming.hls.init-segment"
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

    /// The B1b discontinuity contract, VOD side: a marked playlist
    /// OPENS, the concatenation stays byte-exact across the seam, and
    /// the seam is recorded — the full segment map is fixed at open
    /// (the probe totals know every offset before the first read).
    #[test]
    fn vod_discontinuity_is_recorded_not_refused() {
        let playlist = "#EXTM3U\n\
            #EXT-X-VERSION:3\n\
            #EXT-X-TARGETDURATION:2\n\
            #EXT-X-MEDIA-SEQUENCE:7\n\
            #EXT-X-PLAYLIST-TYPE:VOD\n\
            #EXTINF:1.0,\n\
            seg0.ts\n\
            #EXT-X-DISCONTINUITY\n\
            #EXTINF:2.5,\n\
            seg1.ts\n\
            #EXT-X-ENDLIST\n";
        let fetch = router(&[
            ("/pl.m3u8", playlist.as_bytes()),
            ("/seg0.ts", b"AAAA"),
            ("/seg1.ts", b"BBBBBB"),
        ]);
        let mut stream = match open_vod(fetch, "http://unit.test/pl.m3u8", HlsVodConfig::default())
        {
            Ok(s) => s,
            Err(e) => unreachable!("a discontinuity VOD must open (B1b): {e}"),
        };
        // The whole map is known at open (probe-fixed offsets), with
        // the seam flagged on the segment that follows it.
        let boundaries = stream.segment_boundaries();
        assert_eq!(boundaries.len(), 2, "one boundary per segment");
        assert_eq!(boundaries[0].offset, 0);
        assert_eq!(boundaries[0].media_sequence, 7, "absolute sequence");
        assert_eq!(boundaries[0].duration, Some(1.0));
        assert!(!boundaries[0].discontinuity);
        assert_eq!(boundaries[1].offset, 4, "seg1 starts after seg0's 4 bytes");
        assert_eq!(boundaries[1].media_sequence, 8);
        assert_eq!(boundaries[1].duration, Some(2.5));
        assert!(boundaries[1].discontinuity, "the seam is recorded");
        assert_eq!(stream.stats().discontinuities, 1);
        // The stream surface stays byte-concatenation across the seam.
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
        assert_eq!(all, b"AAAABBBBBB", "concatenation is seamless bytes");
        assert_eq!(stream.size(), Some(10));
    }

    /// Segment-fetch failures surface typed (404 → engine Network) —
    /// at OPEN, on the size probe (the known-length contract moved
    /// detection ahead of the first read).
    #[test]
    fn missing_segment_fails_typed_at_open() {
        let fetch = router(&[("/pl.m3u8", PLAIN_VOD.as_bytes()), ("/seg0.ts", b"AAAA")]);
        let err = match open_vod(fetch, "http://unit.test/pl.m3u8", HlsVodConfig::default()) {
            Ok(_) => unreachable!("seg1 is a 404 — the size probe must fail the open"),
            Err(e) => e,
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
    /// allocates (MP §12) — at OPEN, on the size probe.
    #[test]
    fn oversize_segment_refuses_over_budget() {
        let fetch = router(&[
            ("/pl.m3u8", PLAIN_VOD.as_bytes()),
            ("/seg0.ts", b"AAAA"),
            ("/seg1.ts", b"BBBBBBBBBB"),
        ]);
        let config = HlsVodConfig {
            playlist_response_limit: 1024,
            segment_response_limit: 8, // seg1 is 10 bytes: over budget
            max_retained_bytes: 1024,
        };
        let err = match open_vod(fetch, "http://unit.test/pl.m3u8", config) {
            Ok(_) => unreachable!("seg1 is over budget — the probe must fail the open"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "streaming.hls.response-over-budget");
        assert!(matches!(
            err.kind,
            media_types::ErrorKind::ResourceLimit { .. }
        ));
    }

    /// Seek: backward within the retained prefix (the TS demuxer's
    /// head rewind shape), forward skip-fetch, and the honest
    /// unretained-backward refusal once the budget drops the head.
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
            max_retained_bytes: 8, // seg0+seg1 (10) overflow the budget
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
        // Backward within the retained prefix: free, no new request.
        let requests_before = stream.stats().requests;
        let pos = match stream.seek(SeekOrigin::Start(0)) {
            Ok(p) => p,
            Err(e) => unreachable!("seek to 0: {e}"),
        };
        assert_eq!(pos, SourcePosition(0));
        assert_eq!(stream.stats().requests, requests_before);
        // The full concatenation re-serves from the retained head;
        // the 10-byte total then exceeds the 8-byte budget, the head
        // is dropped, and a *further* backward seek refuses typed.
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
        let Err(err) = stream.seek(SeekOrigin::Start(0)) else {
            unreachable!("head was dropped — seek to 0 must refuse");
        };
        assert_eq!(err.context.code, "streaming.hls.seek-unretained");
        // Seek past the whole concatenation: typed out-of-range.
        let Err(err) = stream.seek(SeekOrigin::Start(999)) else {
            unreachable!("past the end must refuse");
        };
        assert_eq!(err.context.code, "streaming.hls.seek-out-of-range");
    }

    /// Relative segment URIs resolve against the playlist URL before
    /// any fetch goes out; the request log proves the paths.
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
        let mut stream = match open_vod(fetch, &server.url("hls/pl.m3u8"), HlsVodConfig::default())
        {
            Ok(s) => s,
            Err(e) => unreachable!("open over real socket: {e}"),
        };
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
        // The probe wire shape (the known-length contract): playlist
        // first, then per segment a 1-byte Range probe, then the
        // whole-body GET.
        assert_eq!(
            requests.iter().map(|r| r.path.clone()).collect::<Vec<_>>(),
            vec!["/hls/pl.m3u8", "/hls/seg0.ts", "/hls/seg0.ts"],
            "playlist, probe, then the segment body — relative-resolved"
        );
        assert_eq!(requests[1].header("range"), Some("bytes=0-0"), "the probe");
        assert!(
            requests[0].header("range").is_none() && requests[2].header("range").is_none(),
            "whole-body GETs carry no Range (the probe is the only ranged request)"
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
}
