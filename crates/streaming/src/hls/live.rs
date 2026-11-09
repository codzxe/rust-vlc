//! The HLS **live media-playlist session** (card 6.B1b, the card's
//! owned file): a playlist WITHOUT `#EXT-X-ENDLIST` — the live shape
//! B1a refused typed — opens a sliding-window session over the same
//! [`HttpFetch`](crate::fetch::HttpFetch) port and serves the played
//! window's segment concatenation as one continuous
//! [`SourceStream`](plugin_api::SourceStream).
//!
//! # The reload/window model (v0, honestly bounded)
//!
//! * **Live edge.** [`open_live`] starts [`HlsLiveConfig::live_edge_back`]
//!   segments behind the newest segment in the opening window (default
//!   3, the classic recommendation): the segments before the start are
//!   never fetched — joining a live stream near the edge is the point.
//! * **Absolute sequence identity.** Every segment is tracked by its
//!   absolute sequence number (`#EXT-X-MEDIA-SEQUENCE` base + playlist
//!   index). A reload maps old and new windows by that number, so a
//!   window that slides (`MEDIA-SEQUENCE` advances) is recognized as
//!   the same stream with old segments dropped — already-fetched bytes
//!   are never refetched, and a slide never disturbs the concatenation
//!   the consumer is reading.
//! * **Demand-driven reload, request-budgeted, optionally paced.**
//!   The playlist is re-fetched exactly when the fetchable window
//!   runs dry — a pull-driven consumer has no reason to poll ahead of
//!   its buffer. A reload that yields no new segment and no `ENDLIST`
//!   is a *stalled* reload; [`HlsLiveConfig::max_stalled_reloads`]
//!   consecutive stalled reloads fail typed
//!   (`streaming.hls.live-timeout`, a `ResourceLimit` on recovery
//!   attempts). **Wall-clock pacing is opt-in** (card 6.D1):
//!   [`HlsLiveConfig::pacing_clock`] injects the wall-clock port and
//!   each reload then waits out the RFC 8216 §6.3.4 cadence — a full
//!   target duration after a productive window, half after a stalled
//!   one — through the injected clock (a `FakeClock` in tests makes
//!   the cadence an assertion, never a wait). Without a clock (the
//!   default) v0's unpaced request budget stands unchanged, keeping
//!   the pacing-free tests deterministic.
//! * **`ENDLIST` termination.** A reload that carries
//!   `#EXT-X-ENDLIST` ends the session: the remaining window drains,
//!   reads then return empty (`EndOfStream`), and `size()` becomes
//!   the exact drained total.
//! * **Segments lost to the window.** If the window slides past the
//!   fetch cursor before a segment was fetched, the byte stream would
//!   need a gap — that is never served silently: the next read that
//!   needs the lost bytes fails typed
//!   (`streaming.hls.segment-lost-window`), with the count recorded
//!   in [`HlsLiveStats::segments_lost_window`]. (A window that moves
//!   *backwards* — `MEDIA-SEQUENCE` decreasing — is a broken/hostile
//!   server and refuses typed at the reload:
//!   `streaming.hls.window-regression`.)
//!
//! # Unknown length (the honest contract)
//!
//! A live total is fundamentally unknowable — future segments do not
//! exist yet — so `size()` is `None` until the session has ended AND
//! drained every segment, then the exact total. The container layer
//! refuses unknown-length sources at open
//! (`demux.source.unknown-length`), which means live playback through
//! the existing demux hosts is a **later integration** (a streaming
//! container lane): this session is the byte surface it will consume.
//!
//! # Discontinuities (the B1b contract)
//!
//! `#EXT-X-DISCONTINUITY` is recorded, never silently decoded across:
//! each fetched segment lands in [`HlsLiveStream::segment_boundaries`]
//! with its offset, absolute sequence, EXTINF duration and the
//! discontinuity flag, and [`HlsLiveStats::discontinuities`] counts
//! the seams. The stream surface stays byte-concatenation — a decoder
//! that cannot span a seam gets its honest answer at the container
//! boundary.
//!
//! # Still refused (typed, unchanged from B1a)
//!
//! `#EXT-X-KEY` (`streaming.hls.encrypted`), `#EXT-X-BYTERANGE`
//! (`streaming.hls.byterange-segments`), `#EXT-X-MAP`
//! (`streaming.hls.init-segment`) — at open **and on every reload**
//! (a live source that turns encrypted mid-stream is refused, not
//! half-played). A playlist that already carries `ENDLIST` refuses
//! `streaming.hls.vod-playlist` (use [`crate::hls::media::open_vod`]).
//!
//! # Seekability (the honest live shape)
//!
//! Backward within the retained prefix works (the TS demuxer's PSI
//! head rewind) and forward within fetched bytes works; a forward
//! seek beyond fetched data refuses typed
//! (`streaming.hls.live-seek-unfetched`) — a live-edge jump is a
//! re-open, not a linear skip-fetch across a moving window (that
//! policy belongs to the buffer model's card).

use std::collections::VecDeque;
use std::sync::Arc;

use media_types::error::{AccessDenied, CorruptDetail, LimitKind};
use media_types::ids::SourceUri;
use media_types::{EngineError, ErrorKind};
use playlists::MediaPlaylist;
use plugin_api::{SeekOrigin, SourcePosition, SourceStream};

use super::media::{
    DEFAULT_RETAINED_BYTES, MAX_READ_BYTES, MAX_RETAINED_BYTES, SegmentBoundary,
    fetch_playlist_model, map_fetch_err, mint, refuse, resolve_seek_origin,
};
use crate::clock::WallClock;
use crate::error::NetworkError;
use crate::fetch::{DEFAULT_RESPONSE_LIMIT, HttpFetch, HttpRequest};
use crate::hls::timestamp_map::TimestampMap;

/// Default join point: 3 segments behind the live edge (the classic
/// recommendation — enough runway to absorb one late segment without
/// stalling, without falling behind the window).
pub const DEFAULT_LIVE_EDGE_BACK: u32 = 3;

/// Default stalled-reload budget: 4 consecutive unproductive reloads
/// tolerated before the typed `streaming.hls.live-timeout` (with
/// `pacing_clock` injected the same budget bounds the paced waits —
/// pacing changes the cadence, never the tolerance).
pub const DEFAULT_MAX_STALLED_RELOADS: u32 = 4;

/// Live-session configuration (all budgets explicit; see the module
/// docs for what each knob bounds).
#[derive(Clone)]
pub struct HlsLiveConfig {
    /// Per-response memory budget for each playlist fetch (open +
    /// every reload).
    pub playlist_response_limit: u64,
    /// Per-response memory budget for each segment fetch.
    pub segment_response_limit: u64,
    /// Cap on the retained (seekable) prefix of the served
    /// concatenation — old window bytes drop out of memory even
    /// though the playlist has already forgotten them.
    pub max_retained_bytes: usize,
    /// How many segments behind the newest the session starts
    /// (0 = join at the newest segment).
    pub live_edge_back: u32,
    /// Consecutive unproductive reloads tolerated before
    /// `streaming.hls.live-timeout`.
    pub max_stalled_reloads: u32,
    /// Opt-in wall-clock reload pacing (card 6.D1): when `Some`, each
    /// reload first waits out the RFC 8216 §6.3.4 cadence since the
    /// last playlist fetch — a full target duration after a
    /// productive window, half after a stalled one — through this
    /// clock (production passes
    /// [`SystemClock::shared`](crate::clock::SystemClock::shared);
    /// tests inject [`FakeClock`](crate::clock::FakeClock), whose
    /// sleep advances virtual time instantly, so the cadence is
    /// asserted, never waited). `None` (the default) keeps v0's
    /// unpaced demand-driven reloads: the budget stays requests.
    /// The stall budget applies either way — pacing changes *when*
    /// a reload happens, never how many are tolerated.
    pub pacing_clock: Option<Arc<dyn WallClock>>,
}

impl Default for HlsLiveConfig {
    fn default() -> Self {
        HlsLiveConfig {
            playlist_response_limit: DEFAULT_RESPONSE_LIMIT,
            segment_response_limit: DEFAULT_RESPONSE_LIMIT,
            max_retained_bytes: DEFAULT_RETAINED_BYTES,
            live_edge_back: DEFAULT_LIVE_EDGE_BACK,
            max_stalled_reloads: DEFAULT_MAX_STALLED_RELOADS,
            pacing_clock: None,
        }
    }
}

/// The clock handle is not `Debug` (the port is behavior, not data);
/// the config's debug surface says whether pacing is armed, which is
/// the honest summary.
impl std::fmt::Debug for HlsLiveConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HlsLiveConfig")
            .field("playlist_response_limit", &self.playlist_response_limit)
            .field("segment_response_limit", &self.segment_response_limit)
            .field("max_retained_bytes", &self.max_retained_bytes)
            .field("live_edge_back", &self.live_edge_back)
            .field("max_stalled_reloads", &self.max_stalled_reloads)
            .field("pacing_clock", &self.pacing_clock.is_some())
            .finish()
    }
}

impl HlsLiveConfig {
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

/// What the live session did, for tests and honest logging.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HlsLiveStats {
    /// HTTP requests issued (playlist fetches + segment fetches).
    pub requests: u64,
    /// Playlist fetches after the open (the reloads).
    pub playlist_reloads: u64,
    /// Reloads that produced neither a new segment nor `ENDLIST`.
    pub stalled_reloads: u64,
    /// Reloads that waited out the RFC 8216 cadence first (the
    /// pacing-clock surface: 0 without a clock injected).
    pub paced_reloads: u64,
    /// Segment bodies fetched.
    pub segments_fetched: u64,
    /// Segments that slid out of the server window before this
    /// session fetched them (the typed refusal's count).
    pub segments_lost_window: u64,
    /// `#EXT-X-DISCONTINUITY` seams fetched across (recorded in
    /// [`HlsLiveStream::segment_boundaries`]).
    pub discontinuities: u64,
    /// Sum of all response body bytes received (playlist bodies
    /// included — live reloads are part of the cost of live).
    pub bytes_fetched: u64,
}

/// A not-yet-fetched media segment of the current window, keyed by
/// its absolute sequence number.
#[derive(Debug, Clone)]
struct PendingSegment {
    /// Absolute sequence number (`MEDIA-SEQUENCE` base + index).
    abs_seq: u64,
    /// Resolved segment URI.
    uri: String,
    /// Advisory `#EXTINF` duration.
    duration: Option<f64>,
    /// `#EXT-X-DISCONTINUITY` precedes this segment.
    discontinuity: bool,
}

/// One open LIVE media-playlist session: the sliding window plus the
/// retained-prefix buffer. Constructed by [`open_live`].
pub struct HlsLiveStream {
    fetch: Arc<dyn HttpFetch>,
    uri: SourceUri,
    /// The playlist URL, re-fetched on reload.
    url: String,
    /// The latest window model (advisory surface for callers).
    window: MediaPlaylist,
    /// Base `MEDIA-SEQUENCE` of the latest window.
    media_sequence: u64,
    /// Unfetched segments of the latest window, in absolute-sequence
    /// order.
    pending: VecDeque<PendingSegment>,
    /// The absolute sequence the concatenation expects next (the
    /// fetch cursor).
    next_abs_seq: u64,
    /// The highest absolute sequence any window has shown (progress
    /// detection for the stall budget).
    max_seen_seq: u64,
    /// `#EXT-X-ENDLIST` appeared: the window is final.
    ended: bool,
    /// A reload jumped the window base past the fetch cursor — the
    /// byte stream has a gap; the next dry read refuses typed.
    gap_pending: bool,
    /// Consecutive unproductive reloads so far.
    stalled: u32,
    /// When the last playlist fetch completed, in the pacing clock's
    /// monotonic milliseconds (the cadence's anchor; `None` only
    /// between construction and the first paced decision — open
    /// always records it when a clock is injected).
    last_playlist_ms: Option<u64>,
    /// The `#EXT-X-TIMESTAMP-MAP` of the latest window (6.B3):
    /// `None` when the playlist carries none. Refreshed on every
    /// reload — a later tag supersedes an earlier one, so an
    /// encoder restart that republishes the pair is picked up at
    /// the next window.
    timestamp_map: Option<TimestampMap>,
    config: HlsLiveConfig,
    stats: HlsLiveStats,
    // The byte-serving state (same shape as the VOD session):
    // the retained prefix is bytes `[retained_from, fetched_upto)`.
    retained: Vec<u8>,
    retained_from: u64,
    /// Total concatenation bytes fetched so far.
    fetched_upto: u64,
    pos: u64,
    /// The segment map recorded per fetch (the discontinuity record).
    boundaries: Vec<SegmentBoundary>,
}

impl HlsLiveStream {
    /// The latest window model (advisory fields for the UI: target
    /// duration, media sequence, EXTINF total).
    #[must_use]
    pub fn window(&self) -> &MediaPlaylist {
        &self.window
    }

    /// Base `MEDIA-SEQUENCE` of the latest window (the sliding
    /// identity: it only ever advances).
    #[must_use]
    pub fn media_sequence(&self) -> u64 {
        self.media_sequence
    }

    /// What the session has done so far.
    #[must_use]
    pub fn stats(&self) -> HlsLiveStats {
        self.stats
    }

    /// The `#EXT-X-TIMESTAMP-MAP` carried by the latest window
    /// (card 6.B3): the PTS↔wallclock anchor, refreshed on every
    /// reload. `None` when the playlist carries no parseable tag.
    /// The mapping itself (exact across 33-bit PTS wraparound) is
    /// [`TimestampMap::wallclock_ms_for_pts`] — the surface the
    /// WebVTT-in-TS subtitle lane syncs against.
    #[must_use]
    pub fn timestamp_map(&self) -> Option<TimestampMap> {
        self.timestamp_map
    }

    /// The wall-clock time (Unix-epoch ms) of the **live edge** —
    /// the end of the newest segment in the current window —
    /// derived from the window's `#EXT-X-TIMESTAMP-MAP` (card 6.B3):
    /// `LOCAL` advanced by the window's total `EXTINF` duration.
    ///
    /// The derivation anchors the tag's `MPEGTS`/`LOCAL` pair at
    /// the media position of the first segment that follows it in
    /// the playlist — how encoders publish the tag in practice;
    /// the spec does not guarantee the anchor, so the record marks
    /// this `[VERIFY]` against a real-world corpus. `None` when the
    /// window carries no parseable tag (the honest answer, never a
    /// guess from the pacing clock).
    #[must_use]
    pub fn live_edge_wallclock_ms(&self) -> Option<i64> {
        let map = self.timestamp_map?;
        let window_ms: u64 = self
            .window
            .segments
            .iter()
            .filter_map(|segment| segment.duration)
            .map(|duration| (duration * 1000.0).round() as u64)
            .sum();
        Some(map.local_ms + window_ms as i64)
    }

    /// The segment map of everything fetched so far (offset, absolute
    /// media sequence, EXTINF duration, discontinuity flag). Unlike
    /// the VOD session this grows per fetch — a live map is never
    /// knowable ahead of the segments existing.
    #[must_use]
    pub fn segment_boundaries(&self) -> &[SegmentBoundary] {
        &self.boundaries
    }

    /// Fetch one pending segment and absorb its body into the
    /// retained-prefix state, recording its boundary.
    fn fetch_pending(&mut self, seg: PendingSegment) -> Result<(), EngineError> {
        let request = HttpRequest::get(seg.uri.clone())
            .with_response_limit(self.config.segment_response_limit);
        self.stats.requests += 1;
        let body = match self.fetch.fetch(request) {
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
        self.stats.segments_fetched += 1;
        self.stats.bytes_fetched += body.len() as u64;
        if seg.discontinuity {
            self.stats.discontinuities += 1;
        }
        self.boundaries.push(SegmentBoundary {
            offset: self.fetched_upto,
            media_sequence: seg.abs_seq,
            duration: seg.duration,
            discontinuity: seg.discontinuity,
        });
        self.next_abs_seq = seg.abs_seq + 1;
        self.absorb(body);
        Ok(())
    }

    /// Absorb a fetched body into the retained-prefix state (the VOD
    /// session's budget rule: retain what fits, drop the head when it
    /// does not, retain nothing for a single body over the whole
    /// budget).
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

    /// Serve up to `len` bytes at `pos`, fetching segments and
    /// reloading the window as demanded. A dry live window answers a
    /// read **partially** when it has bytes (the pull contract: reads
    /// return up to `len`); the *next* read — needing data the window
    /// does not have — spends the reload budget.
    fn serve(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        let mut out = Vec::new();
        while out.len() < len {
            if self.pos < self.fetched_upto {
                let at = (self.pos - self.retained_from) as usize;
                let have = self.retained.len() - at;
                let take = have.min(len - out.len());
                out.extend_from_slice(&self.retained[at..at + take]);
                self.pos += take as u64;
            } else if self.gap_pending {
                // The window slid past unfetched segments: the byte
                // stream would need a gap — never served silently.
                // Bytes already in hand answer this read; more
                // demanded past the gap refuses typed.
                if out.is_empty() {
                    return Err(mint(
                        ErrorKind::Inaccessible {
                            reason: AccessDenied::NotFound,
                        },
                        "streaming.hls.segment-lost-window",
                    ));
                }
                break;
            } else if let Some(seg) = self.pending.front().cloned() {
                self.pending.pop_front();
                self.fetch_pending(seg)?;
            } else if self.ended {
                break; // EOF: the window was final and is drained
            } else if !out.is_empty() {
                // Partial answer: this read got bytes, and a live
                // window with more wanted is the next read's
                // business (where the reload budget lives).
                break;
            } else {
                self.reload()?;
                // Loop: the reload appended pending segments, set
                // `ended`, or spent stall budget (which errors inside
                // `reload` once past the configured bound).
            }
        }
        Ok(out)
    }

    /// The RFC 8216 §6.3.4 reload cadence in milliseconds: one target
    /// duration after the last productive window, half that after a
    /// stalled one. `TARGETDURATION` is required in a media playlist;
    /// a degenerate window without one falls to its largest `EXTINF`
    /// (rounded up), then to 1 s — never a zero wait, which would
    /// degenerate the pacing into a reload busy-loop.
    fn reload_wait_ms(&self) -> u64 {
        let seconds = self
            .window
            .target_duration
            .unwrap_or_else(|| {
                self.pending
                    .iter()
                    .filter_map(|segment| segment.duration)
                    .fold(0u64, |acc, duration| acc.max(duration.ceil() as u64))
                    .max(1)
            })
            .max(1);
        let full_ms = seconds.saturating_mul(1000);
        if self.stalled > 0 {
            full_ms / 2
        } else {
            full_ms
        }
    }

    /// Re-fetch the playlist and slide the window (the module docs'
    /// reload model): map by absolute sequence, count what slid out,
    /// append what is new, and account stall budget.
    fn reload(&mut self) -> Result<(), EngineError> {
        // Pacing first (card 6.D1, opt-in): with a clock injected, a
        // reload happens no sooner than the RFC 8216 cadence since
        // the last playlist fetch. The clock handle is cloned out of
        // the config so the wait and the bookkeeping below never
        // fight over `self`.
        let pacing = self.config.pacing_clock.clone();
        if let Some(clock) = pacing {
            let wait_ms = self.reload_wait_ms();
            let last = self.last_playlist_ms.unwrap_or_else(|| clock.now_ms());
            let elapsed = clock.now_ms().saturating_sub(last);
            if elapsed < wait_ms {
                clock.sleep_ms(wait_ms - elapsed);
                self.stats.paced_reloads += 1;
            }
        }
        let (model, timestamp_map, playlist_bytes) =
            fetch_playlist_model(&self.fetch, &self.url, self.config.playlist_response_limit)?;
        self.timestamp_map = timestamp_map;
        self.stats.requests += 1;
        self.stats.playlist_reloads += 1;
        self.stats.bytes_fetched += playlist_bytes;
        // The cadence re-anchors on THIS fetch's completion (the
        // clock's reading after the wait, so a paced session's next
        // wait measures from the new window's arrival).
        if let Some(clock) = self.config.pacing_clock.as_ref() {
            self.last_playlist_ms = Some(clock.now_ms());
        }

        // The still-refused shapes can arrive mid-live (an encryption
        // key turned on, a byterange segment, an init section): a
        // live source that changes into them is refused, never
        // half-played.
        if model.segments.iter().any(|s| s.key.is_some()) {
            return Err(refuse("streaming.hls.encrypted"));
        }
        if model.segments.iter().any(|s| s.byte_range.is_some()) {
            return Err(refuse("streaming.hls.byterange-segments"));
        }
        if model.segments.iter().any(|s| s.map.is_some()) {
            return Err(refuse("streaming.hls.init-segment"));
        }
        // A window whose base moves backwards is a broken or hostile
        // server (time travel), not a slide.
        if model.media_sequence < self.media_sequence {
            return Err(mint(
                ErrorKind::Corrupt {
                    detail: CorruptDetail::InvalidStructure { offset: None },
                },
                "streaming.hls.window-regression",
            ));
        }
        // The slide: a window base ahead of the fetch cursor means
        // segments vanished before this session fetched them. The
        // next dry read refuses typed rather than serving a gap.
        if model.media_sequence > self.next_abs_seq {
            let lost = model.media_sequence - self.next_abs_seq;
            self.stats.segments_lost_window += lost;
            self.next_abs_seq = model.media_sequence;
            self.gap_pending = true;
        }
        // Rebuild the pending queue from the new window: everything
        // at/after the fetch cursor (already-fetched entries are
        // dropped — their bytes are in the buffer).
        self.pending = model
            .segments
            .iter()
            .enumerate()
            .filter(|(i, _)| model.media_sequence + *i as u64 >= self.next_abs_seq)
            .map(|(i, seg)| PendingSegment {
                abs_seq: model.media_sequence + i as u64,
                uri: seg.uri.clone(),
                duration: seg.duration,
                discontinuity: seg.discontinuity,
            })
            .collect();
        let newly_ended = model.end_list && !self.ended;
        let productive = model
            .segments
            .iter()
            .enumerate()
            .any(|(i, _)| model.media_sequence + i as u64 > self.max_seen_seq)
            || newly_ended;
        // `saturating_sub`: an empty transient window must not
        // underflow (and cannot be progress either).
        let window_max = model.media_sequence + model.segments.len().saturating_sub(1) as u64;
        self.max_seen_seq = self.max_seen_seq.max(window_max);
        self.ended |= model.end_list;
        self.media_sequence = model.media_sequence;
        self.window = model;
        if productive {
            self.stalled = 0;
        } else {
            self.stalled += 1;
            self.stats.stalled_reloads += 1;
            if self.stalled > self.config.max_stalled_reloads {
                return Err(mint(
                    ErrorKind::ResourceLimit {
                        limit: LimitKind::RecoveryAttempts,
                        budget: self.config.max_stalled_reloads as u64,
                        attempted: self.stalled as u64,
                    },
                    "streaming.hls.live-timeout",
                ));
            }
        }
        Ok(())
    }
}

impl SourceStream for HlsLiveStream {
    fn uri(&self) -> &SourceUri {
        &self.uri
    }

    fn size(&self) -> Option<u64> {
        // A live total is unknowable until the source ends AND the
        // final window is drained; only then is the exact total honest
        // (MP §8 — a value and its absence are different types).
        (self.ended && self.pending.is_empty() && !self.gap_pending).then_some(self.fetched_upto)
    }

    fn supports_seek(&self) -> bool {
        // Backward within the retained prefix plus forward within
        // fetched bytes — see the module docs for the honest live
        // boundary (no skip-fetch across a moving window).
        true
    }

    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        self.serve(len.min(MAX_READ_BYTES))
    }

    fn seek(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError> {
        let target = resolve_seek_origin(self.pos, origin)?;
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
            // Forward beyond fetched data: no linear skip-fetch
            // across a live window — refuse typed (the honest live
            // recovery is re-opening at the edge).
            return Err(mint(
                ErrorKind::Inaccessible {
                    reason: AccessDenied::NotFound,
                },
                "streaming.hls.live-seek-unfetched",
            ));
        }
        self.pos = target;
        Ok(SourcePosition(target))
    }

    fn position(&self) -> SourcePosition {
        SourcePosition(self.pos)
    }
}

/// Open a LIVE HLS media playlist at `url` through `fetch`.
///
/// One playlist GET (budgeted by
/// [`HlsLiveConfig::playlist_response_limit`]), the typed model, the
/// still-refused checks of the module docs, and the live-edge join —
/// see the module docs for the reload/window model and the honest v0
/// boundaries (unknown length; no wall-clock pacing).
///
/// # Errors
/// [`EngineError`] (stage `Open`): transport failures mapped from
/// [`NetworkError`] (`streaming.hls.fetch-failed` /
/// `.response-over-budget`), `streaming.hls.vod-playlist` for a
/// playlist that already carries `#EXT-X-ENDLIST` (use
/// [`crate::hls::media::open_vod`]), `streaming.hls.empty-playlist`,
/// the still-refused shape codes (`encrypted`,
/// `byterange-segments`, `init-segment`), and the playlist-body
/// refusals (`playlist-not-utf8`, `not-hls`, `master-playlist`,
/// `bad-tag-value`, `parse-failed`).
pub fn open_live(
    fetch: Arc<dyn HttpFetch>,
    url: &str,
    config: HlsLiveConfig,
) -> Result<HlsLiveStream, EngineError> {
    config.validate()?;
    // The port's URL grammar, refused before any socket.
    crate::fetch::Url::parse(url).map_err(map_fetch_err)?;
    let uri = SourceUri::parse(url)?;

    let (model, timestamp_map, playlist_bytes) =
        fetch_playlist_model(&fetch, url, config.playlist_response_limit)?;
    let stats = HlsLiveStats {
        requests: 1,
        bytes_fetched: playlist_bytes,
        ..HlsLiveStats::default()
    };

    // ---- Live-shape checks (the module docs' refusal table) ----
    if model.end_list {
        return Err(refuse("streaming.hls.vod-playlist"));
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
    // A discontinuity does NOT refuse — it is recorded per segment
    // (the B1b contract, same as the VOD session).

    // ---- The live-edge join ----
    let back = config.live_edge_back as usize;
    let skip = model.segments.len().saturating_sub(back);
    let media_sequence = model.media_sequence;
    let next_abs_seq = media_sequence + skip as u64;
    // The highest absolute sequence the opening window shows (the
    // `saturating_sub` is for the empty-window shape, which the
    // checks above already refused — kept honest anyway).
    let max_seen_seq = media_sequence + model.segments.len().saturating_sub(1) as u64;
    let pending: VecDeque<PendingSegment> = model.segments[skip..]
        .iter()
        .enumerate()
        .map(|(i, seg)| PendingSegment {
            abs_seq: next_abs_seq + i as u64,
            uri: seg.uri.clone(),
            duration: seg.duration,
            discontinuity: seg.discontinuity,
        })
        .collect();
    // Discontinuity seams are counted when CROSSED (fetched) — a
    // live session cannot honestly count seams it has not reached.
    // The pacing anchor: the opening fetch is the cadence's epoch
    // when a clock is injected.
    let last_playlist_ms = config.pacing_clock.as_ref().map(|clock| clock.now_ms());
    Ok(HlsLiveStream {
        fetch,
        uri,
        url: url.to_owned(),
        window: model,
        media_sequence,
        pending,
        next_abs_seq,
        max_seen_seq,
        ended: false,
        gap_pending: false,
        stalled: 0,
        last_playlist_ms,
        timestamp_map,
        config,
        stats,
        retained: Vec::new(),
        retained_from: 0,
        fetched_upto: 0,
        pos: 0,
        boundaries: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FakeClock;
    use crate::fetch::HttpResponse;
    use crate::fetch::Url;
    use std::sync::Mutex;
    use testkit::Must;

    /// An in-test transport with a RELOAD-MUTABLE playlist route (the
    /// RouterFetch pattern from the VOD tests, plus the scripted
    /// per-fetch body the live window needs — the same shape the mock
    /// testkit's `ScriptedRoute` serves over a real socket in the
    /// e2e lane). The playlist advances through `bodies` per fetch
    /// (last repeats); segment routes are static; anything else is a
    /// 404.
    struct LiveRouter {
        playlist_path: String,
        bodies: Vec<Vec<u8>>,
        next: Mutex<usize>,
        statics: Vec<(String, Vec<u8>)>,
    }

    impl LiveRouter {
        fn new(playlist_bodies: &[&str], statics: &[(&str, &[u8])]) -> Arc<LiveRouter> {
            Arc::new(LiveRouter {
                playlist_path: "/pl.m3u8".to_owned(),
                bodies: playlist_bodies
                    .iter()
                    .map(|b| b.as_bytes().to_vec())
                    .collect(),
                next: Mutex::new(0),
                statics: statics
                    .iter()
                    .map(|(p, b)| ((*p).to_owned(), b.to_vec()))
                    .collect(),
            })
        }

        fn playlist_fetches(&self) -> usize {
            match self.next.lock() {
                Ok(guard) => *guard,
                Err(poisoned) => *poisoned.into_inner(),
            }
        }
    }

    impl HttpFetch for LiveRouter {
        fn fetch(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError> {
            let path = Url::parse(&request.url)?.path_and_query;
            let body: Vec<u8> = if path == self.playlist_path {
                let mut next = match self.next.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                let idx = (*next).min(self.bodies.len() - 1);
                *next += 1;
                self.bodies[idx].clone()
            } else if let Some((_, body)) = self.statics.iter().find(|(p, _)| *p == path) {
                body.clone()
            } else {
                return Ok(HttpResponse {
                    status: 404,
                    reason: "Not Found".to_owned(),
                    headers: Vec::new(),
                    body: Vec::new(),
                });
            };
            // The port contract: a body over budget fails
            // `ResponseTooLarge` instead of allocating.
            if body.len() as u64 > request.response_limit {
                return Err(NetworkError::ResponseTooLarge {
                    limit: request.response_limit,
                    actual: body.len() as u64,
                });
            }
            Ok(HttpResponse {
                status: 200,
                reason: "OK".to_owned(),
                headers: Vec::new(),
                body,
            })
        }
    }

    /// Drain a session fully, collecting the bytes (test helper —
    /// only meaningful for ENDED sessions; a live session never
    /// EOFs, it live-times-out when the budget is spent).
    fn drain(stream: &mut HlsLiveStream) -> Result<Vec<u8>, EngineError> {
        let mut all = Vec::new();
        loop {
            let chunk = stream.read(4096)?;
            if chunk.is_empty() {
                break;
            }
            all.extend_from_slice(&chunk);
        }
        Ok(all)
    }

    /// Read exactly `n` bytes (the live consumer's loop: each read
    /// returns what the window has; the loop stops at the wanted
    /// count or propagates the typed edge errors).
    fn read_exactly(stream: &mut HlsLiveStream, n: usize) -> Result<Vec<u8>, EngineError> {
        let mut all = Vec::new();
        while all.len() < n {
            let chunk = stream.read(n - all.len())?;
            if chunk.is_empty() {
                break;
            }
            all.extend_from_slice(&chunk);
        }
        Ok(all)
    }

    fn fast_config() -> HlsLiveConfig {
        HlsLiveConfig {
            live_edge_back: 2,
            max_stalled_reloads: 2,
            ..HlsLiveConfig::default()
        }
    }

    /// Opening joins `live_edge_back` segments behind the edge and
    /// never fetches what fell before the join point.
    #[test]
    fn live_open_joins_back_from_the_edge() {
        let w1 = "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:0\n\
              #EXTINF:1.0,\nseg0\n#EXTINF:1.0,\nseg1\n#EXTINF:1.0,\nseg2\n";
        let fetch = LiveRouter::new(
            &[w1],
            &[("/seg0", b"AAAA"), ("/seg1", b"BBBB"), ("/seg2", b"CC")],
        );
        let mut stream = match open_live(fetch.clone(), "http://unit.test/pl.m3u8", fast_config()) {
            Ok(s) => s,
            Err(e) => unreachable!("live must open: {e}"),
        };
        assert_eq!(stream.size(), None, "a live session has no knowable total");
        assert!(stream.supports_seek());
        let all = match read_exactly(&mut stream, 6) {
            Ok(b) => b,
            Err(e) => unreachable!("read the join window: {e}"),
        };
        // back=2 of a 3-segment window: seg1+seg2, never seg0.
        assert_eq!(all, b"BBBBCC");
        assert_eq!(stream.stats().segments_fetched, 2);
        assert_eq!(stream.stats().playlist_reloads, 0, "no reload before dry");
        assert_eq!(fetch.playlist_fetches(), 1, "one playlist fetch at open");
        // Window identity surfaced.
        assert_eq!(stream.media_sequence(), 0);
        assert_eq!(stream.window().segments.len(), 3);
    }

    /// The slide: a reload that advances `MEDIA-SEQUENCE` maps by
    /// absolute sequence — no refetch of already-fetched segments,
    /// new segments appended, concatenation seamless.
    #[test]
    fn window_slide_maps_by_absolute_sequence() {
        let w1 = "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:0\n\
              #EXTINF:1.0,\nseg0\n#EXTINF:1.0,\nseg1\n#EXTINF:1.0,\nseg2\n";
        let w2 = "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:1\n\
              #EXTINF:1.0,\nseg1\n#EXTINF:1.0,\nseg2\n#EXTINF:1.0,\nseg3\n";
        let w3 = "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:2\n\
              #EXTINF:1.0,\nseg2\n#EXTINF:1.0,\nseg3\n#EXTINF:1.0,\nseg4\n";
        let fetch = LiveRouter::new(
            &[w1, w2, w3],
            &[
                ("/seg0", b"AAAA"),
                ("/seg1", b"BBBB"),
                ("/seg2", b"CC"),
                ("/seg3", b"DDDD"),
                ("/seg4", b"EE"),
            ],
        );
        let mut stream = match open_live(fetch.clone(), "http://unit.test/pl.m3u8", fast_config()) {
            Ok(s) => s,
            Err(e) => unreachable!("open: {e}"),
        };
        let all = match read_exactly(&mut stream, 12) {
            Ok(b) => b,
            Err(e) => unreachable!("read across two slides: {e}"),
        };
        // back=2: seg1, seg2 | slide appends seg3 | slide appends seg4.
        assert_eq!(all, b"BBBBCCDDDDEE", "seamless across two slides");
        assert_eq!(stream.stats().segments_fetched, 4, "seg1..seg4");
        assert_eq!(
            stream.stats().playlist_reloads,
            2,
            "one reload per dry window"
        );
        assert_eq!(stream.stats().segments_lost_window, 0);
        assert_eq!(stream.media_sequence(), 2, "the window slid to base 2");
        // The stall budget resets on progress (2 reloads, both
        // productive, budget 2 — no live-timeout).
        assert_eq!(stream.stats().stalled_reloads, 0);
        // After the last slide the session is still live and dry.
        assert_eq!(stream.size(), None);
    }

    /// `ENDLIST` on a reload ends the session: the window drains,
    /// reads return empty, and the size becomes the exact total.
    #[test]
    fn endlist_terminates_and_fixes_the_total() {
        let w1 = "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:0\n\
              #EXTINF:1.0,\nseg0\n#EXTINF:1.0,\nseg1\n#EXTINF:1.0,\nseg2\n";
        let w2 = "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:1\n\
              #EXTINF:1.0,\nseg1\n#EXTINF:1.0,\nseg2\n#EXTINF:1.0,\nseg3\n#EXT-X-ENDLIST\n";
        let fetch = LiveRouter::new(
            &[w1, w2],
            &[("/seg1", b"BBBB"), ("/seg2", b"CC"), ("/seg3", b"DDDD")],
        );
        let mut stream = match open_live(fetch.clone(), "http://unit.test/pl.m3u8", fast_config()) {
            Ok(s) => s,
            Err(e) => unreachable!("open: {e}"),
        };
        let all = match drain(&mut stream) {
            Ok(b) => b,
            Err(e) => unreachable!("drain: {e}"),
        };
        assert_eq!(all, b"BBBBCCDDDD");
        let again = match stream.read(64) {
            Ok(c) => c,
            Err(e) => unreachable!("post-end read: {e}"),
        };
        assert!(again.is_empty(), "EndOfStream shape after ENDLIST");
        assert_eq!(stream.size(), Some(10), "the exact total once drained");
        assert_eq!(stream.stats().playlist_reloads, 1);
    }

    /// A source that never publishes more: the stalled-reload budget
    /// fails typed, with the request count measured.
    #[test]
    fn stalled_reloads_fail_typed_live_timeout() {
        let w1 = "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:0\n\
              #EXTINF:1.0,\nseg0\n#EXTINF:1.0,\nseg1\n#EXTINF:1.0,\nseg2\n";
        let fetch = LiveRouter::new(&[w1], &[("/seg1", b"BBBB"), ("/seg2", b"CC")]);
        let mut stream = match open_live(fetch.clone(), "http://unit.test/pl.m3u8", fast_config()) {
            Ok(s) => s,
            Err(e) => unreachable!("open: {e}"),
        };
        let all = match read_exactly(&mut stream, 6) {
            Ok(b) => b,
            Err(e) => unreachable!("read the window to the edge: {e}"),
        };
        assert_eq!(all, b"BBBBCC", "the window itself plays");
        // At the edge: reloads stall (budget 2 → the 3rd fails).
        let err = match stream.read(1) {
            Ok(_) => unreachable!("a stalled live source must refuse, not EOF"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "streaming.hls.live-timeout");
        assert!(matches!(
            err.kind,
            ErrorKind::ResourceLimit {
                limit: LimitKind::RecoveryAttempts,
                budget: 2,
                attempted: 3
            }
        ));
        assert_eq!(stream.stats().stalled_reloads, 3, "measured: 3 stalls");
        assert_eq!(stream.stats().playlist_reloads, 3);
        assert_eq!(fetch.playlist_fetches(), 4, "open + 3 reloads");
        assert_eq!(stream.stats().paced_reloads, 0, "no clock: no pacing");
    }

    /// The 6.D1 pacing proof (deterministic): with a `FakeClock`
    /// injected, a stalled dry window waits the RFC 8216 §6.3.4
    /// cadence before each reload — a full target duration after the
    /// productive open, half after each stalled reload — and the
    /// stall budget still bounds the sequence (the waits advance
    /// virtual time instantly; the schedule is *asserted*).
    #[test]
    fn paced_reloads_wait_the_rfc8216_cadence() {
        // TARGETDURATION:2 → full cadence 2000 ms, stalled half 1000.
        let w1 = "#EXTM3U\n#EXT-X-TARGETDURATION:2\n#EXT-X-MEDIA-SEQUENCE:0\n\
              #EXTINF:2.0,\nseg0\n#EXTINF:2.0,\nseg1\n#EXTINF:2.0,\nseg2\n";
        let fetch = LiveRouter::new(&[w1], &[("/seg1", b"BBBB"), ("/seg2", b"CC")]);
        let clock = FakeClock::new();
        let config = HlsLiveConfig {
            live_edge_back: 2,
            max_stalled_reloads: 2,
            pacing_clock: Some(Arc::new(clock.clone())),
            ..HlsLiveConfig::default()
        };
        let mut stream = match open_live(fetch.clone(), "http://unit.test/pl.m3u8", config) {
            Ok(s) => s,
            Err(e) => unreachable!("open: {e}"),
        };
        let all = match read_exactly(&mut stream, 6) {
            Ok(b) => b,
            Err(e) => unreachable!("read the window to the edge: {e}"),
        };
        assert_eq!(all, b"BBBBCC", "the window itself plays");
        // At the edge the reloads stall (budget 2 → the 3rd fails).
        let err = match stream.read(1) {
            Ok(_) => unreachable!("a stalled live source must refuse, not EOF"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "streaming.hls.live-timeout");
        // The cadence: open anchored at t=0; reload 1 (stalled == 0
        // before it) waits the full 2000; reloads 2 and 3 (stalled
        // after each) wait half (1000) — each wait re-anchored on the
        // fetch that followed it, so every wait is the whole cadence
        // step, never a partial remainder.
        assert_eq!(clock.sleeps(), vec![2000, 1000, 1000]);
        assert_eq!(clock.now_ms(), 4000, "virtual time advanced by the waits");
        assert_eq!(stream.stats().paced_reloads, 3);
        assert_eq!(stream.stats().stalled_reloads, 3, "the budget is unchanged");
        assert_eq!(fetch.playlist_fetches(), 4, "open + 3 paced reloads");
    }

    /// A productive reload under pacing waits the full target
    /// duration since the last fetch — the demand-driven model only
    /// reloads when dry, and the cadence measures from the fetch
    /// that satisfied the previous demand.
    #[test]
    fn paced_productive_reload_waits_the_full_cadence() {
        // TARGETDURATION:1 → full cadence 1000 ms.
        let w1 = "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:0\n\
              #EXTINF:1.0,\nseg0\n#EXTINF:1.0,\nseg1\n";
        let w2 = "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:1\n\
              #EXTINF:1.0,\nseg1\n#EXTINF:1.0,\nseg2\n#EXT-X-ENDLIST\n";
        let fetch = LiveRouter::new(&[w1, w2], &[("/seg1", b"BBBB"), ("/seg2", b"CC")]);
        let clock = FakeClock::new();
        let config = HlsLiveConfig {
            live_edge_back: 1,
            pacing_clock: Some(Arc::new(clock.clone())),
            ..HlsLiveConfig::default()
        };
        let mut stream = match open_live(fetch.clone(), "http://unit.test/pl.m3u8", config) {
            Ok(s) => s,
            Err(e) => unreachable!("open: {e}"),
        };
        let all = match drain(&mut stream) {
            Ok(b) => b,
            Err(e) => unreachable!("drain to the ENDLIST: {e}"),
        };
        assert_eq!(all, b"BBBBCC");
        assert_eq!(stream.size(), Some(6), "ended and drained");
        // One reload (dry at the edge): stalled == 0 before it, so
        // the full 1000 ms — the productive cadence.
        assert_eq!(clock.sleeps(), vec![1000]);
        assert_eq!(stream.stats().paced_reloads, 1);
        assert_eq!(stream.stats().stalled_reloads, 0);
        assert_eq!(stream.stats().playlist_reloads, 1);
    }

    /// A window that slides PAST the fetch cursor (segments vanished
    /// unfetched) refuses typed at the gap — never a silent hole.
    #[test]
    fn slid_out_segments_refuse_typed_at_the_gap() {
        let w1 = "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:0\n\
              #EXTINF:1.0,\nseg0\n#EXTINF:1.0,\nseg1\n#EXTINF:1.0,\nseg2\n";
        // The window jumps to base 3: seg1 (fetched), seg2 (fetched),
        // and seq 2.5 never existed — but base 3 > next cursor 2? No:
        // cursor is 3 after seg2 — jump to base 4 instead.
        let w2 = "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:4\n\
              #EXTINF:1.0,\nseg4\n#EXTINF:1.0,\nseg5\n";
        let fetch = LiveRouter::new(
            &[w1, w2],
            &[
                ("/seg1", b"BBBB"),
                ("/seg2", b"CC"),
                ("/seg4", b"DDDD"),
                ("/seg5", b"EE"),
            ],
        );
        let mut stream = match open_live(fetch.clone(), "http://unit.test/pl.m3u8", fast_config()) {
            Ok(s) => s,
            Err(e) => unreachable!("open: {e}"),
        };
        let all = match read_exactly(&mut stream, 6) {
            Ok(b) => b,
            Err(e) => unreachable!("read the fetched prefix: {e}"),
        };
        assert_eq!(all, b"BBBBCC", "the fetched prefix plays in full");
        let err = match stream.read(1) {
            Ok(_) => unreachable!("a gap must refuse, not skip"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "streaming.hls.segment-lost-window");
        assert!(matches!(
            err.kind,
            ErrorKind::Inaccessible {
                reason: AccessDenied::NotFound
            }
        ));
        // seq 3 slid out unfetched (cursor was at 3 after seg2; the
        // new base is 4).
        assert_eq!(stream.stats().segments_lost_window, 1);
        assert_eq!(stream.size(), None, "a gapped stream has no honest total");
    }

    /// A window whose base moves BACKWARDS is a broken server — typed
    /// refusal at the reload, not a refetch storm.
    #[test]
    fn window_regression_refuses_typed() {
        let w1 = "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:5\n\
              #EXTINF:1.0,\nseg5\n#EXTINF:1.0,\nseg6\n#EXTINF:1.0,\nseg7\n";
        let w2 = "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:4\n\
              #EXTINF:1.0,\nseg4\n#EXTINF:1.0,\nseg5\n";
        let fetch = LiveRouter::new(
            &[w1, w2],
            &[("/seg6", b"BB"), ("/seg7", b"CC"), ("/seg5", b"AA")],
        );
        let mut stream = match open_live(fetch.clone(), "http://unit.test/pl.m3u8", fast_config()) {
            Ok(s) => s,
            Err(e) => unreachable!("open: {e}"),
        };
        let all = match read_exactly(&mut stream, 4) {
            Ok(b) => b,
            Err(e) => unreachable!("read the window: {e}"),
        };
        assert_eq!(all, b"BBCC");
        let err = match stream.read(1) {
            Ok(_) => unreachable!("a regressing window must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "streaming.hls.window-regression");
        assert!(matches!(
            err.kind,
            ErrorKind::Corrupt {
                detail: CorruptDetail::InvalidStructure { offset: None }
            }
        ));
    }

    /// A discontinuity marker in the window is recorded — boundary
    /// flag plus stats — while the concatenation stays byte-exact.
    #[test]
    fn discontinuity_is_recorded_across_the_seam() {
        let w1 = "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:0\n\
              #EXTINF:1.0,\nseg0\n#EXTINF:1.0,\nseg1\n#EXTINF:1.0,\nseg2\n";
        let w2 = "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:1\n\
              #EXTINF:1.0,\nseg1\n#EXTINF:1.0,\nseg2\n\
              #EXT-X-DISCONTINUITY\n#EXTINF:2.0,\nseg3\n#EXT-X-ENDLIST\n";
        let fetch = LiveRouter::new(
            &[w1, w2],
            &[("/seg1", b"BBBB"), ("/seg2", b"CC"), ("/seg3", b"DDDD")],
        );
        let mut stream = match open_live(fetch.clone(), "http://unit.test/pl.m3u8", fast_config()) {
            Ok(s) => s,
            Err(e) => unreachable!("open: {e}"),
        };
        let all = match drain(&mut stream) {
            Ok(b) => b,
            Err(e) => unreachable!("drain: {e}"),
        };
        assert_eq!(all, b"BBBBCCDDDD", "byte-concatenation across the seam");
        let boundaries = stream.segment_boundaries();
        assert_eq!(boundaries.len(), 3, "one boundary per fetched segment");
        assert!(!boundaries[0].discontinuity && !boundaries[1].discontinuity);
        assert_eq!(boundaries[2].media_sequence, 3);
        assert_eq!(boundaries[2].offset, 6, "seg3 starts after 4+2 bytes");
        assert_eq!(boundaries[2].duration, Some(2.0));
        assert!(boundaries[2].discontinuity, "the seam is recorded");
        assert_eq!(stream.stats().discontinuities, 1);
    }

    /// The open-time refusal table for live sessions: ended playlists
    /// point at the VOD session; the still-refused shapes keep their
    /// B1a codes.
    #[test]
    fn live_refusal_table_is_typed() {
        fn code_for(playlist: &str) -> String {
            let fetch = LiveRouter::new(&[playlist], &[("/s.ts", b"x")]);
            match open_live(fetch, "http://unit.test/pl.m3u8", fast_config()) {
                Ok(_) => unreachable!("shape must refuse: {playlist:?}"),
                Err(e) => e.context.code.clone(),
            }
        }
        assert_eq!(
            code_for("#EXTM3U\n#EXTINF:1.0,\ns.ts\n#EXT-X-ENDLIST\n"),
            "streaming.hls.vod-playlist"
        );
        assert_eq!(code_for("#EXTM3U\n"), "streaming.hls.empty-playlist");
        assert_eq!(
            code_for("#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"k\"\n#EXTINF:1.0,\ns.ts\n"),
            "streaming.hls.encrypted"
        );
        assert_eq!(
            code_for("#EXTM3U\n#EXT-X-BYTERANGE:100@0\n#EXTINF:1.0,\ns.ts\n"),
            "streaming.hls.byterange-segments"
        );
        assert_eq!(
            code_for("#EXTM3U\n#EXT-X-MAP:URI=\"i.mp4\"\n#EXTINF:1.0,\ns.m4s\n"),
            "streaming.hls.init-segment"
        );
        assert_eq!(
            code_for("#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1\nv.m3u8\n"),
            "streaming.hls.master-playlist"
        );
        // A discontinuity does NOT refuse.
        let disc = "#EXTM3U\n#EXTINF:1.0,\ns.ts\n#EXT-X-DISCONTINUITY\n#EXTINF:1.0,\ns.ts\n";
        let fetch = LiveRouter::new(&[disc], &[("/s.ts", b"x")]);
        assert!(open_live(fetch, "http://unit.test/pl.m3u8", fast_config()).is_ok());
    }

    /// An encrypted shape arriving on a RELOAD refuses too (a live
    /// source turning encrypted mid-stream is refused, not
    /// half-played).
    #[test]
    fn encryption_arriving_on_reload_refuses() {
        let w1 = "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:0\n\
              #EXTINF:1.0,\nseg0\n#EXTINF:1.0,\nseg1\n#EXTINF:1.0,\nseg2\n";
        let w2 = "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:1\n\
              #EXTINF:1.0,\nseg1\n#EXTINF:1.0,\nseg2\n\
              #EXT-X-KEY:METHOD=AES-128,URI=\"k\"\n#EXTINF:1.0,\nseg3\n";
        let fetch = LiveRouter::new(
            &[w1, w2],
            &[("/seg1", b"BBBB"), ("/seg2", b"CC"), ("/seg3", b"DDDD")],
        );
        let mut stream = match open_live(fetch.clone(), "http://unit.test/pl.m3u8", fast_config()) {
            Ok(s) => s,
            Err(e) => unreachable!("open: {e}"),
        };
        let all = match read_exactly(&mut stream, 6) {
            Ok(b) => b,
            Err(e) => unreachable!("read the pre-encryption prefix: {e}"),
        };
        assert_eq!(all, b"BBBBCC", "the pre-encryption prefix plays");
        let err = match stream.read(1) {
            Ok(_) => unreachable!("the encrypted reload must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "streaming.hls.encrypted");
    }

    /// Live seek policy: backward within the retained prefix works
    /// (the demuxer head-rewind shape), backward below it and forward
    /// beyond fetched data refuse typed.
    #[test]
    fn live_seek_is_honest() {
        let w1 = "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:0\n\
              #EXTINF:1.0,\nseg0\n#EXTINF:1.0,\nseg1\n#EXTINF:1.0,\nseg2\n";
        let fetch = LiveRouter::new(&[w1], &[("/seg1", b"BBBB"), ("/seg2", b"CC")]);
        let mut stream = match open_live(fetch.clone(), "http://unit.test/pl.m3u8", fast_config()) {
            Ok(s) => s,
            Err(e) => unreachable!("open: {e}"),
        };
        let head = match stream.read(4) {
            Ok(c) => c,
            Err(e) => unreachable!("read: {e}"),
        };
        assert_eq!(head, b"BBBB");
        let requests_before = stream.stats().requests;
        let pos = match stream.seek(SeekOrigin::Start(0)) {
            Ok(p) => p,
            Err(e) => unreachable!("retained backward seek: {e}"),
        };
        assert_eq!(pos, SourcePosition(0));
        assert_eq!(stream.stats().requests, requests_before, "free rewind");
        let again = match stream.read(2) {
            Ok(c) => c,
            Err(e) => unreachable!("re-read: {e}"),
        };
        assert_eq!(again, b"BB");
        // Forward beyond fetched data: typed refusal (no live
        // skip-fetch).
        let err = match stream.seek(SeekOrigin::Start(999)) {
            Ok(_) => unreachable!("unfetched forward seek must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "streaming.hls.live-seek-unfetched");
        // Backward below the retained prefix (nothing dropped yet —
        // force it with a tiny budget).
        let fetch = LiveRouter::new(&[w1], &[("/seg1", b"BBBB"), ("/seg2", b"CC")]);
        let tiny = HlsLiveConfig {
            live_edge_back: 2,
            max_stalled_reloads: 2,
            max_retained_bytes: 4,
            ..HlsLiveConfig::default()
        };
        let mut stream = match open_live(fetch, "http://unit.test/pl.m3u8", tiny) {
            Ok(s) => s,
            Err(e) => unreachable!("open tiny: {e}"),
        };
        let all = match read_exactly(&mut stream, 6) {
            Ok(b) => b,
            Err(e) => unreachable!("read tiny: {e}"),
        };
        assert_eq!(all, b"BBBBCC");
        let err = match stream.seek(SeekOrigin::Start(0)) {
            Ok(_) => unreachable!("dropped head must refuse"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "streaming.hls.seek-unretained");
    }

    /// Config validation: zero budgets refuse typed before any socket.
    #[test]
    fn live_config_bounds_refuse_typed() {
        let w1 = "#EXTM3U\n#EXTINF:1.0,\ns.ts\n";
        let fetch = LiveRouter::new(&[w1], &[("/s.ts", b"x")]);
        let zero = HlsLiveConfig {
            segment_response_limit: 0,
            ..HlsLiveConfig::default()
        };
        let Err(err) = open_live(fetch.clone(), "http://unit.test/pl.m3u8", zero) else {
            unreachable!("zero budget must refuse");
        };
        assert_eq!(err.context.code, "streaming.hls.config");
        let over = HlsLiveConfig {
            max_retained_bytes: MAX_RETAINED_BYTES + 1,
            ..HlsLiveConfig::default()
        };
        let Err(err) = open_live(fetch, "http://unit.test/pl.m3u8", over) else {
            unreachable!("over-ceiling retention must refuse");
        };
        assert_eq!(err.context.code, "streaming.hls.config");
    }

    /// 6.B3 — the `#EXT-X-TIMESTAMP-MAP` surfaces: parsed at open,
    /// applied to the live-edge wallclock derivation, and superseded
    /// by the tag a reload carries (the encoder-restart shape).
    #[test]
    fn timestamp_map_parses_applies_and_supersedes_per_reload() {
        // 2020-01-01T00:00:00Z = 1_577_836_800_000 ms.
        let w1 = "#EXTM3U\n\
              #EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:0\n\
              #EXT-X-TIMESTAMP-MAP:MPEGTS=900000,LOCAL=2020-01-01T00:00:00Z\n\
              #EXTINF:1.0,\nseg0\n#EXTINF:1.0,\nseg1\n#EXTINF:1.0,\nseg2\n";
        // The reload: the window slid by one (seq 0→1), AND the
        // encoder restarted — a fresh anchor pair (PTS reset to 0,
        // LOCAL jumped an hour ahead).
        let w2 = "#EXTM3U\n\
              #EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:1\n\
              #EXT-X-TIMESTAMP-MAP:MPEGTS=0,LOCAL=2020-01-01T01:00:00Z\n\
              #EXTINF:1.0,\nseg1\n#EXTINF:1.0,\nseg2\n#EXTINF:1.0,\nseg3\n";
        let fetch = LiveRouter::new(
            &[w1, w2],
            &[
                ("/seg0", b"AAAA"),
                ("/seg1", b"BBBB"),
                ("/seg2", b"CC"),
                ("/seg3", b"DDD"),
            ],
        );
        let mut stream = match open_live(fetch.clone(), "http://unit.test/pl.m3u8", fast_config()) {
            Ok(s) => s,
            Err(e) => unreachable!("live must open: {e}"),
        };
        // (1) The opening window's map: parsed, and driving the
        // live-edge wallclock = LOCAL + the window's EXTINF total
        // (3 × 1.0 s here — the join is 2 back; the EDGE is the
        // newest segment's end, the whole window).
        let map = stream.timestamp_map().must();
        assert_eq!(map.mpegts, 900_000);
        assert_eq!(map.local_ms, 1_577_836_800_000);
        assert_eq!(stream.live_edge_wallclock_ms(), Some(1_577_836_803_000));
        // (2) Drain the join (seg1, seg2), then demand one more
        // byte — the dry read that spends the reload.
        let bytes = match read_exactly(&mut stream, 6) {
            Ok(b) => b,
            Err(e) => unreachable!("drain the join: {e}"),
        };
        assert_eq!(bytes, b"BBBBCC");
        let more = match read_exactly(&mut stream, 3) {
            Ok(b) => b,
            Err(e) => unreachable!("read across the reload: {e}"),
        };
        assert_eq!(more, b"DDD", "seg3 arrives through the W2 window");
        assert!(fetch.playlist_fetches() >= 2, "the reload happened");
        // (3) The reloaded window's map superseded the first: PTS
        // reset + LOCAL jump are both visible, and the live edge
        // now derives from the NEW anchor (3 × 1.0 s window).
        let map2 = stream.timestamp_map().must();
        assert_eq!(map2.mpegts, 0);
        assert_eq!(map2.local_ms, 1_577_840_400_000); // 2020-01-01T01:00:00Z
        assert_eq!(stream.live_edge_wallclock_ms(), Some(1_577_840_403_000));
        // (4) The stream itself is unaffected: the mapping is a
        // timing surface, never a byte-stream input.
        assert_eq!(stream.stats().discontinuities, 0);
    }

    /// 6.B3 — a window without a tag answers `None` honestly on
    /// both surfaces (never a guess from the pacing clock).
    #[test]
    fn timestamp_map_absent_is_none_not_a_guess() {
        let w1 = "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:0\n\
              #EXTINF:1.0,\nseg0\n#EXTINF:1.0,\nseg1\n";
        let fetch = LiveRouter::new(&[w1], &[("/seg0", b"AAAA"), ("/seg1", b"BB")]);
        let stream = match open_live(fetch, "http://unit.test/pl.m3u8", fast_config()) {
            Ok(s) => s,
            Err(e) => unreachable!("live must open: {e}"),
        };
        assert_eq!(stream.timestamp_map(), None);
        assert_eq!(stream.live_edge_wallclock_ms(), None);
    }
}
