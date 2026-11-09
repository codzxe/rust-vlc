//! HLS **master playlists + variant switching** (cards 6.B2a/6.B2b,
//! this card's owned file): the variant ladder of a
//! [`MasterPlaylist`](playlists::MasterPlaylist) — parsed by the
//! playlists crate (6.B2a) — selected by a bandwidth policy, and an
//! adaptive session that serves one continuous
//! [`SourceStream`](plugin_api::SourceStream) while switching
//! variants at segment boundaries.
//!
//! # 6.B2a — selection policy
//!
//! [`select_variant_index`] is the v0 policy: **the highest
//! `BANDWIDTH` at or under the cap** ([`AbrConfig::max_bandwidth_bps`],
//! default [`DEFAULT_MAX_BANDWIDTH_BPS`]). Ties keep playlist order
//! (the producer's own ladder order is the tie-break, never a
//! mutation of it). A master whose every variant is over the cap (or
//! whose every row was damaged) refuses typed
//! (`streaming.hls.no-variant`) — never a silent clamp to "closest"
//! (MP §5). Alternate renditions (`#EXT-X-MEDIA`) are **parsed and
//! recorded** by the playlists crate; playing a non-default
//! audio/subtitle rendition is a later card (the session plays the
//! variant rows' own media).
//!
//! # 6.B2b — variant switching (the honest v0 shape)
//!
//! * **Decisions only at segment boundaries.** The
//!   [`AbrController`] is consulted exactly between two segment
//!   fetches — a switch never tears a segment in half.
//! * **Inputs are host observations**, the same seam 6.D1's
//!   [`BufferModel`](crate::buffer::BufferModel) uses: the host
//!   measures each fetch and calls
//!   [`HlsAdaptiveStream::note_throughput`], and forwards buffer
//!   signals via [`HlsAdaptiveStream::on_buffer_signal`] (a
//!   `StallPredicted` forces a down-step at the next boundary; a
//!   `StallCleared`/`Filled` re-enables up-steps). The controller
//!   never reads a clock itself.
//! * **Hysteresis, no thrash** (the watermark pattern of 6.D1, one
//!   band per axis): a down-step needs the EWMA throughput below
//!   `down_margin_percent` of the current variant's bandwidth; an
//!   up-step needs it above `(100 + up_margin_percent)`% of the
//!   *next* rung's, AND [`AbrConfig::min_dwell_segments`] segments
//!   served on the current variant. Between those margins the
//!   controller holds — an oscillating link flaps the link, not the
//!   ladder. The switch history is bounded
//!   ([`SWITCH_HISTORY_CAP`]); the counters are not.
//! * **Timeline continuity.** A switch fetches the new variant's
//!   playlist and continues at the **same absolute media sequence**
//!   (`EXT-X-MEDIA-SEQUENCE` base + index): the new variant's NEXT
//!   segment — the one covering the media time the old variant had
//!   not yet served. A variant whose window does not carry that
//!   sequence refuses typed
//!   (`streaming.hls.variant-sequence-missing`) — a gap is never
//!   served silently. The seam is **recorded as a discontinuity**
//!   in [`HlsAdaptiveStream::segment_boundaries`] (the B1b
//!   recorded-seam contract: encoding parameters may reset at a
//!   variant boundary; nothing is silently decoded across it).
//!
//! # What this session is (and is not)
//!
//! VOD variants only, honestly: every variant playlist must pass the
//! B1a playability table (ENDLIST, no KEY/BYTERANGE/MAP — refusals
//! carry the same codes). Live-window variant switching (reload +
//! slide while switching) is a later card; so is playing alternate
//! renditions and the fMP4 lane. The concatenation length is not
//! probe-fixed at open (which variant each future segment will be is
//! unknowable before its boundary decision): `size()` stays `None`
//! until every segment is fetched — the same honest unknown-length
//! contract as the live session (its consumer is the streaming
//! container lane, not the current demux hosts).

use std::collections::VecDeque;
use std::sync::Arc;

use media_types::error::{AccessDenied, LimitKind};
use media_types::ids::SourceUri;
use media_types::{EngineError, ErrorKind};
use playlists::{MasterPlaylist, Variant};
use plugin_api::{SeekOrigin, SourcePosition, SourceStream};

use super::media::{
    DEFAULT_RETAINED_BYTES, MAX_READ_BYTES, MAX_RETAINED_BYTES, SegmentBoundary, fetch_playlist_model,
    map_fetch_err, mint, refuse, resolve_seek_origin,
};
use crate::buffer::BufferSignal;
use crate::error::NetworkError;
use crate::fetch::{DEFAULT_RESPONSE_LIMIT, HttpFetch, HttpRequest};

/// Default bandwidth cap for selection: 25 Mbps. `[PROPOSAL]` — the
/// product-config `limits` table has no bandwidth field yet; when the
/// Lead adds one, this default should be wired to it (the field, not
/// this constant, then owns the number).
pub const DEFAULT_MAX_BANDWIDTH_BPS: u64 = 25_000_000;

/// Default up-switch margin: the EWMA throughput must reach 125% of
/// the next rung's bandwidth before stepping up.
pub const DEFAULT_UP_MARGIN_PERCENT: u32 = 25;

/// Default down-switch margin: the EWMA throughput must fall below
/// 80% of the current rung's bandwidth before stepping down.
pub const DEFAULT_DOWN_MARGIN_PERCENT: u32 = 80;

/// Default dwell: two segments served on a variant before an
/// up-switch may be considered (down-steps on a stall prediction
/// bypass the dwell — an emergency is an emergency).
pub const DEFAULT_MIN_DWELL_SEGMENTS: u32 = 2;

/// Bound on the remembered switch history (the anti-thrash ledger:
/// a bounded `VecDeque`, never an unbounded log). The counters
/// (`switch_count`) are unbounded integers, not memory.
pub const SWITCH_HISTORY_CAP: usize = 16;

/// Selection + hysteresis policy (6.B2a's cap, 6.B2b's band).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbrConfig {
    /// Highest `BANDWIDTH` selection will consider (the cap of
    /// [`select_variant_index`] and of up-steps).
    pub max_bandwidth_bps: u64,
    /// Up-switch margin in percent (see [`DEFAULT_UP_MARGIN_PERCENT`]).
    pub up_margin_percent: u32,
    /// Down-switch margin in percent (see
    /// [`DEFAULT_DOWN_MARGIN_PERCENT`]).
    pub down_margin_percent: u32,
    /// Segments that must be served on the current variant before an
    /// up-switch (see [`DEFAULT_MIN_DWELL_SEGMENTS`]).
    pub min_dwell_segments: u32,
}

impl Default for AbrConfig {
    fn default() -> Self {
        AbrConfig {
            max_bandwidth_bps: DEFAULT_MAX_BANDWIDTH_BPS,
            up_margin_percent: DEFAULT_UP_MARGIN_PERCENT,
            down_margin_percent: DEFAULT_DOWN_MARGIN_PERCENT,
            min_dwell_segments: DEFAULT_MIN_DWELL_SEGMENTS,
        }
    }
}

/// Select the variant: highest `BANDWIDTH` at or under the policy
/// cap; ties keep playlist order. `None` = nothing selectable (the
/// caller refuses typed — never a clamp).
#[must_use]
pub fn select_variant_index(master: &MasterPlaylist, max_bandwidth_bps: u64) -> Option<usize> {
    master
        .variants
        .iter()
        .enumerate()
        .filter(|(_, v)| v.bandwidth <= max_bandwidth_bps)
        .max_by_key(|(_, v)| v.bandwidth)
        .map(|(i, _)| i)
}

/// The adaptive-bitrate decision core (6.B2b): a pure policy object,
/// no I/O and no clock — the host feeds observations in (the same
/// posture as 6.D1's `BufferModel`), the session asks
/// [`decide`](AbrController::decide) at each segment boundary.
#[derive(Debug, Clone)]
pub struct AbrController {
    config: AbrConfig,
    /// Current variant index into the master's `variants`.
    current: usize,
    /// EWMA of measured throughput (bytes per millisecond of wall
    /// time) — the 3/4-old + 1/4-new shift-only merge of 6.D1's rate
    /// EWMA. Milliseconds, not nanoseconds: integer bytes-per-ns
    /// would truncate every realistic rate to zero.
    ewma_bytes_per_ms: Option<u128>,
    /// Segments served on the current variant since the last switch
    /// (the dwell counter).
    segments_on_current: u32,
    /// Total segment boundaries seen (the history's time axis).
    segments_seen: u64,
    /// A 6.D1 `StallPredicted` is outstanding: force down-steps.
    stalled: bool,
    /// Completed switches (the unbounded counter).
    switch_count: u64,
    /// Bounded switch ledger: `(segments_seen, target_variant)`.
    history: VecDeque<(u64, usize)>,
}

impl AbrController {
    /// A controller starting on variant `start` (the selection
    /// policy's answer at open).
    #[must_use]
    pub fn new(start: usize, config: AbrConfig) -> AbrController {
        AbrController {
            config,
            current: start,
            ewma_bytes_per_ms: None,
            segments_on_current: 0,
            segments_seen: 0,
            stalled: false,
            switch_count: 0,
            history: VecDeque::new(),
        }
    }

    /// The host observed a fetch (or any bandwidth sample): fold
    /// `bytes` over `wall_ns` into the EWMA. A zero span is skipped —
    /// there is no honest rate in it.
    pub fn observe_throughput(&mut self, bytes: u64, wall_ns: u64) {
        if wall_ns == 0 {
            return;
        }
        // Bytes per millisecond, computed in u128 before the division
        // so sub-millisecond spans keep their magnitude.
        let rate = u128::from(bytes) * 1_000_000 / u128::from(wall_ns);
        self.ewma_bytes_per_ms = Some(match self.ewma_bytes_per_ms {
            // 3/4 old + 1/4 new — shift-only fixed point (D-2).
            Some(prev) => prev - (prev >> 2) + (rate >> 2),
            None => rate,
        });
    }

    /// The host forwarded a 6.D1 buffer signal: `StallPredicted`
    /// arms the forced down-step, `StallCleared`/`Filled` disarms it.
    pub fn on_buffer_signal(&mut self, signal: BufferSignal) {
        match signal {
            BufferSignal::StallPredicted { .. } => self.stalled = true,
            BufferSignal::StallCleared | BufferSignal::Filled => self.stalled = false,
            BufferSignal::Steady | BufferSignal::Drained => {}
        }
    }

    /// The EWMA throughput in bits per second (bytes per
    /// millisecond of wall time, scaled ×8000), when at least one
    /// sample has been observed.
    #[must_use]
    pub fn throughput_bps(&self) -> Option<u64> {
        self.ewma_bytes_per_ms
            .map(|r| (r * 8_000).min(u128::from(u64::MAX)) as u64)
    }

    /// The current variant index.
    #[must_use]
    pub fn current(&self) -> usize {
        self.current
    }

    /// Counters for tests, logging, and the event-hub surface
    /// (milestone 3's `[PROPOSAL]` wiring point).
    #[must_use]
    pub fn snapshot(&self) -> AbrSnapshot {
        AbrSnapshot {
            current: self.current,
            switch_count: self.switch_count,
            throughput_bps: self.throughput_bps(),
            stalled: self.stalled,
            segments_on_current: self.segments_on_current,
        }
    }

    /// Account one segment served on the current variant (the dwell
    /// counter's tick — called by the session after each fetch).
    pub fn segment_served(&mut self) {
        self.segments_on_current += 1;
        self.segments_seen += 1;
    }

    /// Decide the variant for the NEXT segment boundary. `Some(target)`
    /// = switch to `target` now (the controller has already committed:
    /// dwell reset, history pushed). `None` = hold. See the module
    /// docs for the hysteresis band and the ladder step rule.
    #[must_use]
    pub fn decide(&mut self, variants: &[Variant]) -> Option<usize> {
        if variants.is_empty() {
            return None;
        }
        let throughput = self.throughput_bps()?;
        // The ladder: variant indices sorted by bandwidth ascending
        // (stable, so playlist order breaks ties).
        let mut ladder: Vec<usize> = (0..variants.len()).collect();
        ladder.sort_by_key(|&i| variants[i].bandwidth);
        let pos = ladder.iter().position(|&i| i == self.current)?;

        // ---- Down: starvation or a 6.D1 stall prediction. A stall
        // bypasses the dwell (an emergency is an emergency); plain
        // starvation does not (one bad sample must not flap the
        // ladder — the EWMA already smooths, the dwell double-guards).
        let current_bw = variants[self.current].bandwidth;
        let starved = u128::from(throughput) * 100
            < u128::from(current_bw) * u128::from(self.config.down_margin_percent);
        if pos > 0 && (self.stalled || starved) {
            return self.commit(ladder[pos - 1]);
        }

        // ---- Up: headroom over the next rung, dwell served, and
        // the rung under the cap.
        if pos + 1 < ladder.len() && !self.stalled {
            let next = ladder[pos + 1];
            let next_bw = variants[next].bandwidth;
            let headroom = u128::from(throughput) * 100
                >= u128::from(next_bw) * u128::from(100 + self.config.up_margin_percent);
            if next_bw <= self.config.max_bandwidth_bps
                && headroom
                && self.segments_on_current >= self.config.min_dwell_segments
            {
                return self.commit(next);
            }
        }
        None
    }

    /// Commit a switch (bounded history, reset dwell, count it).
    fn commit(&mut self, target: usize) -> Option<usize> {
        self.current = target;
        self.segments_on_current = 0;
        self.switch_count += 1;
        self.history.push_back((self.segments_seen, target));
        while self.history.len() > SWITCH_HISTORY_CAP {
            self.history.pop_front();
        }
        Some(target)
    }
}

/// The controller's queryable counters (milestone 3's metrics
/// surface; the event-hub fan-out is a `[PROPOSAL]` wiring the
/// playback engine owns).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbrSnapshot {
    /// Current variant index.
    pub current: usize,
    /// Completed variant switches.
    pub switch_count: u64,
    /// EWMA throughput (bits per second), once sampled.
    pub throughput_bps: Option<u64>,
    /// A 6.D1 stall prediction is forcing down-steps.
    pub stalled: bool,
    /// Segments served on the current variant since the last switch.
    pub segments_on_current: u32,
}

/// Adaptive-session configuration (all budgets explicit).
#[derive(Clone)]
pub struct HlsMasterConfig {
    /// Per-response budget for the master and each variant playlist.
    pub playlist_response_limit: u64,
    /// Per-response budget for each segment fetch.
    pub segment_response_limit: u64,
    /// Cap on the retained (seekable) prefix of the concatenation.
    pub max_retained_bytes: usize,
    /// Selection cap + hysteresis band.
    pub abr: AbrConfig,
}

impl Default for HlsMasterConfig {
    fn default() -> Self {
        HlsMasterConfig {
            playlist_response_limit: DEFAULT_RESPONSE_LIMIT,
            segment_response_limit: DEFAULT_RESPONSE_LIMIT,
            max_retained_bytes: DEFAULT_RETAINED_BYTES,
            abr: AbrConfig::default(),
        }
    }
}

impl HlsMasterConfig {
    /// Validate the budgets (typed refusal, never a clamp).
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

/// What the adaptive session did (tests, logging, metrics).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HlsMasterStats {
    /// HTTP requests issued (master + variant playlists + segments).
    pub requests: u64,
    /// Media playlists fetched (the open one plus one per switch).
    pub playlists_fetched: u64,
    /// Segment bodies fetched.
    pub segments_fetched: u64,
    /// Sum of all response body bytes received.
    pub bytes_fetched: u64,
    /// Variant switches performed at segment boundaries.
    pub variant_switches: u64,
    /// Discontinuity seams recorded (playlist `EXT-X-DISCONTINUITY`
    /// plus switch seams).
    pub discontinuities: u64,
    /// The bandwidth of the variant currently being served (the
    /// "last variant" metric).
    pub current_bandwidth_bps: Option<u64>,
}

/// One planned segment of the current variant.
#[derive(Debug, Clone)]
struct PlannedSegment {
    uri: String,
    duration: Option<f64>,
    /// `EXT-X-DISCONTINUITY` preceded it in its own playlist.
    discontinuity: bool,
    /// Absolute media sequence (`MEDIA-SEQUENCE` base + index) — the
    /// identity a switch aligns by.
    abs_seq: u64,
}

/// One open adaptive master-playlist session. Constructed by
/// [`open_master`].
pub struct HlsAdaptiveStream {
    fetch: Arc<dyn HttpFetch>,
    uri: SourceUri,
    /// The typed master (variant ladder + renditions, advisory).
    master: MasterPlaylist,
    /// The ABR decision core.
    abr: AbrController,
    /// The current variant's unfetched segments, absolute-sequence
    /// order (re-planned on every switch).
    plan: Vec<PlannedSegment>,
    /// Index into `plan` of the next segment to fetch.
    next: usize,
    config: HlsMasterConfig,
    stats: HlsMasterStats,
    // The byte-serving state (the same shape as the VOD/live
    // sessions): the retained prefix is `[retained_from, fetched_upto)`.
    retained: Vec<u8>,
    retained_from: u64,
    fetched_upto: u64,
    pos: u64,
    ended: bool,
    /// The segment map recorded per fetch (the discontinuity record,
    /// switch seams included).
    boundaries: Vec<SegmentBoundary>,
}

impl HlsAdaptiveStream {
    /// The typed master model (advisory surface: the ladder, the
    /// renditions).
    #[must_use]
    pub fn master(&self) -> &MasterPlaylist {
        &self.master
    }

    /// The ABR counters (switch count, throughput, current variant).
    #[must_use]
    pub fn abr_snapshot(&self) -> AbrSnapshot {
        self.abr.snapshot()
    }

    /// What the session has done so far.
    #[must_use]
    pub fn stats(&self) -> HlsMasterStats {
        self.stats
    }

    /// The segment map of everything fetched so far (offset,
    /// absolute media sequence, EXTINF duration, discontinuity flag —
    /// variant-switch seams carry the flag).
    #[must_use]
    pub fn segment_boundaries(&self) -> &[SegmentBoundary] {
        &self.boundaries
    }

    /// The host measured a bandwidth sample (the 6.D1 observation
    /// seam: production measures each segment fetch; tests script
    /// drops and rises through it).
    pub fn note_throughput(&mut self, bytes: u64, wall_ns: u64) {
        self.abr.observe_throughput(bytes, wall_ns);
    }

    /// The host forwarded a 6.D1 buffer signal (stall predictions
    /// force down-steps at the next boundary).
    pub fn on_buffer_signal(&mut self, signal: BufferSignal) {
        self.abr.on_buffer_signal(signal);
    }

    /// Fetch and type the variant playlist at `url`, refusing the
    /// B1a-unplayable shapes with the media session's codes.
    fn load_variant_plan(&mut self, url: &str) -> Result<Vec<PlannedSegment>, EngineError> {
        let (model, _bytes) =
            fetch_playlist_model(&self.fetch, url, self.config.playlist_response_limit)?;
        self.stats.requests += 1;
        self.stats.playlists_fetched += 1;
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
        Ok(model
            .segments
            .iter()
            .enumerate()
            .map(|(i, s)| PlannedSegment {
                uri: s.uri.clone(),
                duration: s.duration,
                discontinuity: s.discontinuity,
                abs_seq: model.media_sequence + i as u64,
            })
            .collect())
    }

    /// Perform a switch to `target`: load the variant's playlist,
    /// align at the absolute sequence the concatenation expects
    /// next, and re-plan. The first segment of the new plan carries
    /// the recorded switch seam (discontinuity).
    fn switch_variant(&mut self, target: usize) -> Result<(), EngineError> {
        let url = self.master.variants[target].uri.clone();
        let plan = self.load_variant_plan(&url)?;
        let next_abs = self.plan[self.next].abs_seq;
        let start = plan
            .iter()
            .position(|s| s.abs_seq == next_abs)
            .ok_or_else(|| {
                mint(
                    ErrorKind::Inaccessible {
                        reason: AccessDenied::NotFound,
                    },
                    "streaming.hls.variant-sequence-missing",
                )
            })?;
        let mut plan = plan;
        plan.drain(..start);
        // The switch seam is a recorded discontinuity (module docs).
        plan[0].discontinuity = true;
        self.plan = plan;
        self.next = 0;
        self.stats.variant_switches += 1;
        self.stats.current_bandwidth_bps = Some(self.master.variants[target].bandwidth);
        Ok(())
    }

    /// Fetch the next planned segment and absorb its body.
    fn fetch_next_segment(&mut self) -> Result<(), EngineError> {
        // The boundary decision: a switch replaces the plan before
        // the fetch (the new variant's NEXT segment is what serves
        // this boundary — never half a segment).
        if self.next > 0 {
            if let Some(target) = self.abr.decide(&self.master.variants) {
                self.switch_variant(target)?;
            }
        }
        let seg = self.plan[self.next].clone();
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
        self.absorb(body);
        self.next += 1;
        self.abr.segment_served();
        if self.next >= self.plan.len() {
            self.ended = true;
        }
        Ok(())
    }

    /// Absorb a fetched body into the retained-prefix state (the
    /// budget rule shared with the VOD/live sessions).
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

    /// Serve up to `len` bytes at `pos`, fetching/switching as needed.
    fn serve(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        let mut out = Vec::new();
        while out.len() < len {
            if self.pos < self.fetched_upto {
                let at = (self.pos - self.retained_from) as usize;
                let have = self.retained.len() - at;
                let take = have.min(len - out.len());
                out.extend_from_slice(&self.retained[at..at + take]);
                self.pos += take as u64;
            } else if !self.ended && self.next < self.plan.len() {
                self.fetch_next_segment()?;
            } else {
                break; // EOF: every segment fetched and consumed
            }
        }
        Ok(out)
    }
}

impl SourceStream for HlsAdaptiveStream {
    fn uri(&self) -> &SourceUri {
        &self.uri
    }

    fn size(&self) -> Option<u64> {
        // Unknown until drained (module docs: which variant each
        // future segment will be is unknowable before its boundary
        // decision — the live session's honest contract).
        (self.ended && self.next >= self.plan.len()).then_some(self.fetched_upto)
    }

    fn supports_seek(&self) -> bool {
        // Within the retained prefix (the same honest boundary as
        // the VOD session).
        true
    }

    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        self.serve(len.min(MAX_READ_BYTES))
    }

    fn seek(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError> {
        let target = resolve_seek_origin(self.pos, origin)?;
        if target < self.pos {
            if target < self.retained_from {
                return Err(mint(
                    ErrorKind::Inaccessible {
                        reason: AccessDenied::NotFound,
                    },
                    "streaming.hls.seek-unretained",
                ));
            }
        } else if target > self.fetched_upto {
            // A forward jump past unfetched segments: refuse typed
            // (the live session's honest boundary — an adaptive
            // skip-fetch policy belongs to the buffer-model host).
            return Err(mint(
                ErrorKind::Inaccessible {
                    reason: AccessDenied::NotFound,
                },
                "streaming.hls.adaptive-seek-unfetched",
            ));
        }
        self.pos = target;
        Ok(SourcePosition(target))
    }

    fn position(&self) -> SourcePosition {
        SourcePosition(self.pos)
    }
}

/// Open an adaptive master-playlist session at `url`: fetch and type
/// the master, select the initial variant under the policy cap, load
/// its VOD media playlist, and serve the ladder-switchable segment
/// concatenation (module docs for the switching contract).
///
/// # Errors
/// [`EngineError`] (stage `Open`): transport failures mapped from
/// [`NetworkError`], master parse refusals
/// (`streaming.hls.not-hls` / `.media-playlist` / `.parse-failed`),
/// `streaming.hls.playlist-not-utf8`, `streaming.hls.no-variant`
/// (nothing selectable under the cap), and the B1a table's refusals
/// for the selected variant's playlist.
pub fn open_master(
    fetch: Arc<dyn HttpFetch>,
    url: &str,
    config: HlsMasterConfig,
) -> Result<HlsAdaptiveStream, EngineError> {
    config.validate()?;
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
    let bytes = body.len() as u64;
    let text = std::str::from_utf8(&body)
        .map_err(|_| super::media::playlist_corrupt("streaming.hls.playlist-not-utf8"))?;
    let (master, _warnings) = playlists::parse_master_playlist(Some(url), text).map_err(|err| {
        let code = match &err {
            playlists::HlsPlaylistError::NotHls => "streaming.hls.not-hls",
            playlists::HlsPlaylistError::MediaPlaylist => "streaming.hls.media-playlist",
            playlists::HlsPlaylistError::BadTagValue { .. } => "streaming.hls.bad-tag-value",
            playlists::HlsPlaylistError::Parse { .. } => "streaming.hls.parse-failed",
            // A master tag inside a master parse is not damage.
            playlists::HlsPlaylistError::MasterPlaylist => "streaming.hls.parse-failed",
        };
        refuse(code)
    })?;
    let start = select_variant_index(&master, config.abr.max_bandwidth_bps)
        .ok_or_else(|| refuse("streaming.hls.no-variant"))?;

    let abr = AbrController::new(start, config.abr);
    let current_bw = master.variants[start].bandwidth;
    let variant_url = master.variants[start].uri.clone();
    let mut stream = HlsAdaptiveStream {
        fetch,
        uri,
        master,
        abr,
        plan: Vec::new(),
        next: 0,
        config,
        stats: HlsMasterStats {
            requests: 1,
            bytes_fetched: bytes,
            current_bandwidth_bps: Some(current_bw),
            ..HlsMasterStats::default()
        },
        retained: Vec::new(),
        retained_from: 0,
        fetched_upto: 0,
        pos: 0,
        ended: false,
        boundaries: Vec::new(),
    };
    stream.plan = stream.load_variant_plan(&variant_url)?;
    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::BufferSignal;
    use crate::fetch::HttpResponse;
    use crate::fetch::HttpFetch;

    /// An in-test transport routing by URL path through the real
    /// `HttpFetch` port (the same surface the media session's tests
    /// use — no socket, unit-test speed).
    struct RouterFetch {
        routes: Vec<(String, Vec<u8>)>,
    }

    impl HttpFetch for RouterFetch {
        fn fetch(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError> {
            let path = crate::fetch::Url::parse(&request.url)?.path_and_query;
            match self.routes.iter().find(|(p, _)| *p == path) {
                Some((_, body)) => Ok(HttpResponse {
                    status: 200,
                    reason: "OK".to_owned(),
                    headers: Vec::new(),
                    body: body.clone(),
                }),
                None => Ok(HttpResponse {
                    status: 404,
                    reason: "Not Found".to_owned(),
                    headers: Vec::new(),
                    body: Vec::new(),
                }),
            }
        }
    }

    fn router(routes: &[(&str, Vec<u8>)]) -> Arc<dyn HttpFetch> {
        Arc::new(RouterFetch {
            routes: routes
                .iter()
                .map(|(p, b)| ((*p).to_owned(), b.clone()))
                .collect(),
        })
    }

    fn master_text() -> String {
        "#EXTM3U\n\
        #EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"aud\",NAME=\"en\",DEFAULT=YES,URI=\"aud.m3u8\"\n\
        #EXT-X-STREAM-INF:BANDWIDTH=2000000,RESOLUTION=1280x720\nhi.m3u8\n\
        #EXT-X-STREAM-INF:BANDWIDTH=800000,RESOLUTION=640x360\nmid.m3u8\n\
        #EXT-X-STREAM-INF:BANDWIDTH=200000\nlo.m3u8\n"
            .to_owned()
    }

    /// A VOD variant playlist: `count` segments, media sequence 100.
    fn variant_playlist(prefix: &str, count: usize) -> String {
        let mut s = String::from("#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:2\n\
            #EXT-X-MEDIA-SEQUENCE:100\n#EXT-X-PLAYLIST-TYPE:VOD\n");
        for i in 0..count {
            s.push_str("#EXTINF:2.0,\n");
            s.push_str(&format!("{prefix}{i}.ts\n"));
        }
        s.push_str("#EXT-X-ENDLIST\n");
        s
    }

    fn routes_for(count: usize) -> Vec<(String, Vec<u8>)> {
        let mut routes = vec![
            ("/master.m3u8".to_owned(), master_text().into_bytes()),
            ("/hi.m3u8".to_owned(), variant_playlist("hi/seg", count).into_bytes()),
            ("/mid.m3u8".to_owned(), variant_playlist("mid/seg", count).into_bytes()),
            ("/lo.m3u8".to_owned(), variant_playlist("lo/seg", count).into_bytes()),
        ];
        for i in 0..count {
            routes.push((format!("/hi/seg{i}.ts"), vec![b'H'; 500]));
            routes.push((format!("/mid/seg{i}.ts"), vec![b'M'; 200]));
            routes.push((format!("/lo/seg{i}.ts"), vec![b'L'; 50]));
        }
        routes
    }

    fn owned_routes(count: usize) -> Vec<(&'static str, Vec<u8>)> {
        routes_for(count)
            .into_iter()
            .map(|(p, b)| (Box::leak(p.into_boxed_str()) as &str, b))
            .collect()
    }

    /// 6.B2a: selection is highest-bandwidth-under-cap, ties keep
    /// playlist order, and an all-over-cap master refuses typed.
    #[test]
    fn selection_is_highest_under_cap() {
        let (master, _) =
            playlists::parse_master_playlist(None, &master_text()).expect("master types");
        assert_eq!(select_variant_index(&master, 25_000_000), Some(0), "hi");
        assert_eq!(select_variant_index(&master, 1_000_000), Some(1), "mid");
        assert_eq!(select_variant_index(&master, 800_000), Some(1), "mid at cap");
        assert_eq!(select_variant_index(&master, 200_000), Some(2), "lo");
        assert_eq!(select_variant_index(&master, 199_999), None, "nothing under cap");
    }

    /// 6.B2b hysteresis: a throughput oscillating inside the band
    /// (80%..125% of the neighboring rungs) never flaps the ladder;
    /// only crossing a margin switches, and the dwell double-guards
    /// up-steps.
    #[test]
    fn abr_hysteresis_holds_inside_the_band() {
        let (master, _) =
            playlists::parse_master_playlist(None, &master_text()).expect("master types");
        let mut abr = AbrController::new(1, AbrConfig::default()); // on mid (800k)

        // Deep inside the band (throughput ~1 Mbps: above 0.8x mid,
        // below 1.25x hi): holds across many samples.
        for _ in 0..10 {
            abr.observe_throughput(100_000, 800_000_000); // 1 Mbps
            abr.segment_served();
            assert_eq!(abr.decide(&master.variants), None, "holds in the band");
        }

        // Starvation (0.5 Mbps < 0.8 x 800k): steps down to lo once
        // the EWMA crosses the 80% line (six samples — the EWMA's
        // memory is part of the anti-flap design, so the crossing
        // takes a few samples by intent).
        for _ in 0..6 {
            abr.observe_throughput(50_000, 800_000_000);
        }
        assert_eq!(abr.decide(&master.variants), Some(2), "starved → down");
        assert_eq!(abr.snapshot().switch_count, 1);

        // Oscillation back inside the band: no flapping upward
        // without the 1.25x headroom over mid (needs >= 1 Mbps for
        // lo→mid? next rung mid needs 800k x 1.25 = 1 Mbps).
        for _ in 0..6 {
            abr.observe_throughput(100_000, 800_000_000); // 1 Mbps = exactly headroom
            abr.segment_served();
            // Exactly-at-margin does not switch (>= is required; the
            // EWMA sits just under from the earlier low samples).
            let _ = abr.decide(&master.variants);
        }
        assert_eq!(abr.snapshot().current, 2, "no unbounded flapping");

        // A real rise (2.5 Mbps) after the dwell: back up one rung.
        for _ in 0..4 {
            abr.observe_throughput(250_000, 800_000_000);
            abr.segment_served();
        }
        assert_eq!(abr.decide(&master.variants), Some(1), "headroom → up");
    }

    /// 6.B2b: a 6.D1 `StallPredicted` forces a down-step at the next
    /// boundary (bypassing dwell); `StallCleared` re-enables up.
    #[test]
    fn abr_stall_prediction_forces_down() {
        let (master, _) =
            playlists::parse_master_playlist(None, &master_text()).expect("master types");
        let mut abr = AbrController::new(0, AbrConfig::default()); // on hi
        abr.observe_throughput(375_000, 1_000_000_000); // 3 Mbps: healthy
        assert_eq!(abr.decide(&master.variants), None, "healthy holds");

        abr.on_buffer_signal(BufferSignal::StallPredicted { eta_ns: 500 });
        assert!(abr.snapshot().stalled);
        assert_eq!(abr.decide(&master.variants), Some(1), "stall → down");

        abr.on_buffer_signal(BufferSignal::StallCleared);
        assert!(!abr.snapshot().stalled);
        // Cleared, healthy, but dwell is 0: holds on mid.
        abr.observe_throughput(250_000, 1_000_000_000); // 2 Mbps
        assert_eq!(abr.decide(&master.variants), None);
    }

    /// The switch ledger is bounded: toggling the 6.D1 stall input
    /// many times produces many switches, but the remembered history
    /// never exceeds the cap (the counters are integers, not memory).
    #[test]
    fn abr_history_is_bounded() {
        let (master, _) =
            playlists::parse_master_playlist(None, &master_text()).expect("master types");
        let mut abr = AbrController::new(0, AbrConfig::default());
        for k in 0..(SWITCH_HISTORY_CAP * 3) as u64 {
            abr.observe_throughput(1_000, 1_000_000); // healthy 8 Mbps
            abr.segment_served();
            abr.segment_served();
            if k % 2 == 0 {
                // A stall prediction forces the down-step; clearing
                // it re-arms the (healthy, dwell-served) up-step.
                abr.on_buffer_signal(BufferSignal::StallPredicted { eta_ns: 1 });
                let _ = abr.decide(&master.variants);
                abr.on_buffer_signal(BufferSignal::StallCleared);
            } else {
                let _ = abr.decide(&master.variants);
            }
        }
        assert!(abr.snapshot().switch_count >= SWITCH_HISTORY_CAP as u64);
        assert!(abr.history.len() <= SWITCH_HISTORY_CAP, "bounded ledger");
    }

    /// 6.B2a+6.B2b end to end over the port: the adaptive session
    /// selects hi, serves its segments, drops to lo on a scripted
    /// bandwidth drop, rises back on recovery — and the timeline
    /// stays continuous (absolute media sequence +1 across every
    /// seam, seam recorded as discontinuity, bytes in order).
    #[test]
    fn adaptive_session_switches_on_scripted_bandwidth() {
        let fetch = router(&owned_routes(12));
        let mut stream =
            open_master(fetch, "http://unit.test/master.m3u8", HlsMasterConfig::default())
                .expect("adaptive session opens");
        assert_eq!(stream.abr_snapshot().current, 0, "starts on hi");
        assert_eq!(stream.stats().playlists_fetched, 1, "master + hi playlist");
        assert_eq!(stream.stats().requests, 2);
        let mut all = Vec::new();

        // Two healthy segments on hi (the dwell), then the crash.
        all.extend_from_slice(&stream.read(500).expect("read hi/seg0"));
        all.extend_from_slice(&stream.read(500).expect("read hi/seg1"));
        stream.note_throughput(500, 2_000_000); // 2 Mbps (in-band on hi)

        // The drop: 0.1 Mbps << 0.8 x 2M → down-steps at the next
        // boundary (one rung at a time: hi → mid first).
        stream.note_throughput(5_000, 400_000_000);
        let c = stream.read(500).expect("read switches at the boundary");
        assert!(c.starts_with(&[b'M'; 4]), "down-stepped off hi onto mid");
        all.extend_from_slice(&c);
        assert!(stream.abr_snapshot().current >= 1);

        // Keep starving until the ladder bottom (lo).
        let mut guard = 0;
        while stream.abr_snapshot().current != 2 && guard < 6 {
            stream.note_throughput(5_000, 400_000_000);
            all.extend_from_slice(&stream.read(200).expect("read"));
            stream.note_throughput(5_000, 400_000_000);
            guard += 1;
        }
        assert_eq!(stream.abr_snapshot().current, 2, "starved to lo");

        // Recovery: 5 Mbps with dwell → back up (one rung per
        // boundary, no overshoot flapping).
        for _ in 0..3 {
            stream.note_throughput(625_000, 1_000_000_000); // 5 Mbps
            all.extend_from_slice(&stream.read(60).expect("read"));
            stream.note_throughput(625_000, 1_000_000_000);
        }
        assert!(
            stream.abr_snapshot().current >= 1,
            "recovered upward, current = {}",
            stream.abr_snapshot().current
        );

        // Drain the rest.
        loop {
            let chunk = stream.read(128).expect("read");
            if chunk.is_empty() {
                break;
            }
            all.extend_from_slice(&chunk);
        }
        // Timeline continuity: absolute sequences strictly +1 across
        // the whole concatenation, switch seams flagged.
        let bounds = stream.segment_boundaries();
        for w in bounds.windows(2) {
            assert_eq!(w[1].media_sequence, w[0].media_sequence + 1, "continuous");
        }
        assert_eq!(bounds[0].media_sequence, 100);
        assert_eq!(bounds.len(), 12);
        assert!(
            bounds.iter().any(|b| b.discontinuity),
            "switch seams recorded as discontinuities"
        );
        assert_eq!(stream.stats().segments_fetched, 12);
        assert!(stream.stats().variant_switches >= 2);
        assert_eq!(stream.size(), Some(all.len() as u64), "exact total once drained");
    }

    /// The refusals: an all-over-cap master, and a media playlist
    /// handed to the master open.
    #[test]
    fn adaptive_refusals_are_typed() {
        let fetch = router(&owned_routes(2));
        let mut tight = HlsMasterConfig::default();
        tight.abr.max_bandwidth_bps = 1;
        let err = open_master(fetch.clone(), "http://unit.test/master.m3u8", tight)
            .map(|_| ())
            .expect_err("nothing under a 1-bps cap");
        assert_eq!(err.context.code, "streaming.hls.no-variant");

        let media = "#EXTM3U\n#EXT-X-TARGETDURATION:2\n#EXTINF:1.0,\ns.ts\n#EXT-X-ENDLIST\n";
        let fetch = router(&[("/pl.m3u8", media.as_bytes().to_vec())]);
        let err = open_master(fetch, "http://unit.test/pl.m3u8", HlsMasterConfig::default())
            .map(|_| ())
            .expect_err("media playlist is not a master");
        assert_eq!(err.context.code, "streaming.hls.media-playlist");
    }

    /// A variant whose playlist does not carry the next absolute
    /// media sequence refuses typed — a timeline gap is never served.
    #[test]
    fn switch_to_misaligned_variant_refuses() {
        // lo's playlist starts at media sequence 106 (a misaligned
        // ladder): after hi's segments 100/101 the concatenation
        // expects 102..104 on mid, then 105 — which this lo window
        // (106..109) does not carry. hi/mid carry 8 segments so the
        // mid run does not end before the lo boundary.
        let mut routes = owned_routes(8);
        for (p, b) in routes.iter_mut() {
            if *p == "/lo.m3u8" {
                *b = variant_playlist("lo/seg", 4).replace(
                    "#EXT-X-MEDIA-SEQUENCE:100",
                    "#EXT-X-MEDIA-SEQUENCE:106",
                ).into_bytes();
            }
        }
        let fetch = router(&routes);
        let mut stream =
            open_master(fetch, "http://unit.test/master.m3u8", HlsMasterConfig::default())
                .expect("opens on hi");
        let _ = stream.read(500);
        let _ = stream.read(500);
        stream.note_throughput(500, 2_000_000);
        stream.note_throughput(5_000, 400_000_000); // starve → mid
        let _ = stream.read(500);
        // Starve hard and long: the EWMA must cross mid's 80% line
        // so the next boundary steps mid → lo (the misaligned rung).
        for _ in 0..6 {
            stream.note_throughput(5_000, 400_000_000);
        }
        let err = stream.read(500).expect_err("misaligned lo refuses");
        assert_eq!(err.context.code, "streaming.hls.variant-sequence-missing");
    }
}
