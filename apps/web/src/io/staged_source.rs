//! The staged window source (card 7.H4; doc 09 §6.1) — the fetch
//! twin of `crates/streaming`'s `RangedResource` (card 6.A2a/6.A2b),
//! serving the same [`SourceStream`] contract demuxers already
//! consume.
//!
//! # Why staging, not streaming
//!
//! The browser `fetch()` is async and lives in the Worker's JS glue
//! (the wasm module has no imports); the engine's demuxer reads are
//! synchronous. The bridge is **staged windows with a pump gate**:
//!
//! - The source mints typed [`WindowRequest`]s (surfaced upstream as
//!   `FetchRequest` messages for the glue to serve).
//! - Responses arrive as `FetchResponse` messages and are absorbed
//!   into a bounded window cache ([`StagedConfig::window_bytes`]).
//! - [`StagedSource::ready`] answers whether the read cursor is
//!   covered; the HOST's pump gates engine polls on it (doc 09
//!   §3.2's cooperative rule adapted to I/O: a poll is only issued
//!   when the bytes it would read are staged), so a synchronous
//!   demuxer read never sees an unstaged byte and never blocks.
//!
//! # Modes (mirrored from `ranged.rs`)
//!
//! - **Ranged** (probe answered `206`): window fetches under
//!   `chunk_bytes`; reads coalesce across windows; read-ahead keeps
//!   pipelining while the cache is under budget.
//! - **Scan** (probe answered `200` — the no-range server, card
//!   6.A2b's shape): one whole-body fetch under the window budget; a
//!   body over budget is the typed over-budget refusal; reads drain
//!   forward-only; a seek behind the cursor re-fetches from byte 0
//!   (counted in [`StagedStats::refetches`] — the honest linear
//!   cost, accounted rather than hidden).
//! - A mid-stream `200` (a ranged fetch suddenly answered full-body)
//!   **resyncs** into scan mode (counted in
//!   [`StagedStats::resyncs`]) — the mirror of `ranged.rs`'s flip.
//!
//! # Bounded memory (MP §11)
//!
//! Every buffer is bounded by [`StagedConfig`]: windows aged past the
//! read cursor drop first (forward reads can never touch them
//! again), then least-recently-staged windows evict — never the
//! cursor's own window (eviction cannot deadlock the gate); one
//! response body is bounded by the request's `response_limit`; one
//! `read(len)` result is bounded by [`MAX_READ_BYTES`]. The scan
//! buffer is bounded by the window budget at fetch time.
//!
//! # Seeks
//!
//! [`SourceStream::supports_seek`] answers the SERVER truth (the
//! [`Seekability`] detection) — `media.get-info` reports it. Serving
//! a seek needs its target window staged: in ranged mode an unstaged
//! target mints the window request (a cheap ranged re-fetch) and
//! answers the typed `web.fetch.window-not-staged` refusal until it
//! lands; in scan mode an unstaged target mints the whole-body
//! re-fetch the same way. The honest v1 boundary: the host's
//! `playback.seek` is refused upstream before this seam (see
//! [`crate::host`]'s served-surface docs), so the refusal names a
//! real boundary rather than hiding one.

use media_types::EngineError;
use media_types::error::{AccessDenied, ErrorContext};
use media_types::ids::SourceUri;
use plugin_api::{SeekOrigin, SourcePosition, SourceStream};

use crate::io::fetch_port::{
    DEFAULT_CHUNK_BYTES, DEFAULT_WINDOW_BYTES, FetchError, FetchRequest, FetchResponse,
    RangeRequest,
};
use crate::io::seekability::{Seekability, detect_seekability};

/// Ceiling on a single `read(len)` result, mirroring the input
/// crate's `MAX_READ_BYTES` payload ceiling (64 MiB) — the
/// caller-owned result of one bounded read is the only memory
/// outside the window budget. Streaming restates the same value for
/// the same reason; this is the web lane's restatement.
pub const MAX_READ_BYTES: usize = 64 * 1024 * 1024;

/// The read-ahead readiness margin: the gate holds the engine poll
/// until the staged span covers `[pos, pos + margin)` (or the known
/// end). Demuxer reads are bounded well under this (the AVI lane's
/// largest single read is the 32 KiB probe window); reads larger
/// than the margin may see mid-stream short reads, which the trait
/// permits as data — documented, not hidden.
pub const READY_MARGIN: u64 = 256 * 1024;

/// The stage tag for the source's typed refusals.
const STAGE: media_types::PipelineStage = media_types::PipelineStage::Open;

fn mint(kind: media_types::ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(STAGE, kind, ErrorContext::new(code))
}

/// The typed not-staged refusal (the seek boundary's honest v1
/// answer — see the module docs).
fn window_not_staged(target: u64, scan: bool) -> EngineError {
    let mode = if scan {
        "a whole-body re-fetch (scan mode)"
    } else {
        "the window fetch (ranged mode)"
    };
    mint(
        media_types::ErrorKind::Unsupported { format: None },
        "web.fetch.window-not-staged",
    )
    .with_source(std::io::Error::other(format!(
        "seek target {target} is not staged — {mode} request was minted; retry the seek once \
         the response lands (the host's playback.seek is refused upstream in v1; see \
         apps/web/src/host.rs)"
    )))
}

/// Tuning of a [`StagedSource`] (the mirror of `ranged.rs`'s
/// `RangedConfig`, web-tuned defaults — see
/// [`crate::io::fetch_port`]'s budget note).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StagedConfig {
    /// Window-cache budget: the maximum bytes held across all staged
    /// windows (aging/eviction above it) — and, in scan mode, the
    /// fetch budget on the whole body (a larger body is the typed
    /// over-budget refusal). Default 8 MiB (the web-tuned mirror of
    /// streaming's 64 MiB).
    pub window_bytes: usize,
    /// Request granularity / forward read-ahead: the maximum `Range:`
    /// extent per fetch (also that fetch's response budget). Default
    /// 256 KiB.
    pub chunk_bytes: usize,
}

impl Default for StagedConfig {
    fn default() -> StagedConfig {
        StagedConfig {
            window_bytes: DEFAULT_WINDOW_BYTES,
            chunk_bytes: DEFAULT_CHUNK_BYTES,
        }
    }
}

impl StagedConfig {
    /// Validate: both knobs positive, chunk at or below the window
    /// (a chunk that cannot fit in its own cache would force
    /// eviction of the window being served — a configuration bug,
    /// refused typed, never clamped; `ranged.rs`'s rule, verbatim).
    ///
    /// # Errors
    /// [`EngineError`] — `web.fetch.config-zero` or
    /// `web.fetch.chunk-over-window`.
    pub fn validate(&self) -> Result<(), EngineError> {
        if self.window_bytes == 0 || self.chunk_bytes == 0 {
            return Err(mint(
                media_types::ErrorKind::Unsupported { format: None },
                "web.fetch.config-zero",
            ));
        }
        if self.chunk_bytes > self.window_bytes {
            return Err(mint(
                media_types::ErrorKind::Unsupported { format: None },
                "web.fetch.chunk-over-window",
            ));
        }
        Ok(())
    }
}

/// One window fetch the source needs (surfaced upstream as a
/// `FetchRequest` for the transport to serve).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowRequest {
    /// The correlation id (the request's own).
    pub id: u64,
    /// The URL (post-redirect final URL is the transport's business).
    pub url: String,
    /// The byte range, `None` for scan-mode whole-body fetches.
    pub range: Option<RangeRequest>,
    /// The response body budget for this fetch: the window budget,
    /// on every request shape (the transport — browser `fetch()`
    /// glue or mock — refuses a body advertising or delivering more,
    /// the lane's bounded-memory contract; see [`StagedSource`]'s
    /// mint note for why the chunk extent is not the bound).
    pub limit: u64,
}

impl WindowRequest {
    /// The [`FetchRequest`] form (what actually rides the port).
    #[must_use]
    pub fn as_fetch_request(&self) -> FetchRequest {
        FetchRequest {
            id: self.id,
            url: self.url.clone(),
            range: self.range,
            response_limit: self.limit,
        }
    }
}

/// Test/telemetry-facing counters (the mirror of `RangedStats`; no
/// production path reads them).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StagedStats {
    /// Fetch requests issued (probe included).
    pub requests: u64,
    /// Body bytes fetched off the transport (probe included).
    pub bytes_fetched: u64,
    /// Reads served wholly or partly from staged windows without a
    /// new fetch.
    pub cache_hits: u64,
    /// Windows dropped from the cache (aged past the read cursor, or
    /// evicted for budget) — bounded memory, counted.
    pub cache_evictions: u64,
    /// Scan-mode whole-body re-fetches (a read before the scan
    /// frontier — the honest linear cost, counted).
    pub refetches: u64,
    /// Bytes consumed and discarded by scan-mode skips.
    pub skipped_bytes: u64,
    /// Mid-stream resyncs into scan mode (a ranged fetch suddenly
    /// answered `200`).
    pub resyncs: u64,
}

/// One cached window: resource bytes `[start, start + data.len())`.
struct Window {
    start: u64,
    data: Vec<u8>,
}

impl Window {
    /// Whether the window covers byte offset `at`.
    fn covers(&self, at: u64) -> bool {
        at >= self.start && (at - self.start) < self.data.len() as u64
    }
}

/// The fetch strategy (the mirror of `ranged.rs`'s `Mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Ranged window fetches (the server honors `Range:`).
    Ranged,
    /// Linear scan (the no-range server): one whole-body fetch,
    /// forward-only reads.
    Scan,
}

/// The scan-mode forward buffer (the no-range server's linear lane):
/// `data` holds the fetched body at resource offset `base`, and
/// `consumed` is the monotonic read frontier — reads and forward
/// seeks advance it, and bytes behind it are gone forever (a no-range
/// server cannot re-serve them; a seek behind it is the whole-body
/// re-fetch, counted in [`StagedStats::refetches`]). The retained
/// prefix stays allocated until the next re-fetch replaces the
/// buffer — bounded by the window budget, checked at fetch time.
#[derive(Debug)]
struct ScanBuffer {
    /// The resource offset of `data[0]` (always 0 in v1: whole-body
    /// fetches — kept as a field so the invariant is visible, not
    /// implied).
    base: u64,
    /// The monotonic consumed frontier (index into `data`).
    consumed: usize,
    /// The fetched body.
    data: Vec<u8>,
}

impl ScanBuffer {
    /// A fresh buffer over `data` at `base` (nothing consumed).
    fn new(base: u64, data: Vec<u8>) -> ScanBuffer {
        ScanBuffer {
            base,
            consumed: 0,
            data,
        }
    }

    /// The end of the buffered span (one past the last byte).
    fn end(&self) -> u64 {
        self.base + self.data.len() as u64
    }

    /// Whether `target` is servable from the buffer: at or ahead of
    /// the consumed frontier, and inside the buffered span (the
    /// span's end position serves — a read there is the honest short
    /// read at the staged end).
    fn serves(&self, target: u64) -> bool {
        target >= self.base + self.consumed as u64 && target <= self.end()
    }
}

/// A seekable, bounded-memory byte stream over a staged http(s)
/// resource — the fetch twin of the file source and blob source,
/// serving the same [`SourceStream`] contract demuxers consume.
///
/// Construct with [`StagedSource::open`] (answers the probe
/// [`WindowRequest`] the caller must serve), then drive the
/// request/absorb cycle: [`StagedSource::pending_request`] mints
/// read-ahead, the transport's answers go to [`StagedSource::absorb`].
pub struct StagedSource {
    uri: SourceUri,
    url: String,
    config: StagedConfig,
    mode: Mode,
    seekability: Option<Seekability>,
    len: Option<u64>,
    pos: u64,
    /// The window cache, sorted by `start` (insertion keeps the
    /// order; scans are linear, never quadratic).
    windows: Vec<Window>,
    cached: usize,
    /// The scan-mode forward buffer (drained by the consumed
    /// frontier — see [`ScanBuffer`]).
    scan: Option<ScanBuffer>,
    /// The outstanding request (issued, not yet absorbed).
    outstanding: Option<WindowRequest>,
    next_id: u64,
    stats: StagedStats,
}

impl StagedSource {
    /// Open `url`: validate the config and URL, mint the probe
    /// request (`Range: bytes=0-(chunk-1)` — the detection fetch of
    /// [`crate::io::seekability`], doubling as the first
    /// read-ahead). The caller serves it and feeds the response to
    /// [`StagedSource::absorb`].
    ///
    /// # Errors
    /// [`EngineError`] (stage `Open`) — config/URL validation.
    pub fn open(
        url: &str,
        config: StagedConfig,
    ) -> Result<(StagedSource, WindowRequest), EngineError> {
        config.validate()?;
        let uri = crate::io::fetch_port::validate_http_url(url)?;
        let mut source = StagedSource {
            uri,
            url: url.to_string(),
            config,
            mode: Mode::Ranged,
            seekability: None,
            len: None,
            pos: 0,
            windows: Vec::new(),
            cached: 0,
            scan: None,
            outstanding: None,
            next_id: 1,
            stats: StagedStats::default(),
        };
        let probe = source.mint_request(Some(RangeRequest {
            start: 0,
            end_exclusive: config.chunk_bytes as u64,
        }));
        Ok((source, probe))
    }

    /// The source's URL (as opened).
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The detected seekability (after the probe response was
    /// absorbed; `None` before).
    #[must_use]
    pub fn seekability(&self) -> Option<Seekability> {
        self.seekability
    }

    /// The counters (test/telemetry evidence).
    #[must_use]
    pub fn stats(&self) -> StagedStats {
        self.stats
    }

    /// The outstanding request (issued, not yet absorbed) — the
    /// host's `waiting-for-io` truth.
    #[must_use]
    pub fn outstanding(&self) -> Option<&WindowRequest> {
        self.outstanding.as_ref()
    }

    /// The next request the caller should serve, when the read-ahead
    /// wants more than the outstanding request covers (pipelining
    /// while the cache is under budget and the end is not staged).
    /// `None` while a request is outstanding (one at a time — the
    /// message lane is ordered), when the scan body already reaches
    /// the end, or when aging cannot free enough budget for a chunk
    /// (the honest over-budget shape: the caller sees
    /// [`StagedSource::ready`] stay false and reports it).
    pub fn pending_request(&mut self) -> Option<WindowRequest> {
        if self.outstanding.is_some() {
            return None;
        }
        match self.mode {
            Mode::Ranged => {}
            Mode::Scan => return None,
        }
        self.age_behind_cursor();
        let frontier = self.frontier();
        if self.len.is_some_and(|len| frontier >= len) {
            return None;
        }
        if self.cached + self.config.chunk_bytes > self.config.window_bytes {
            return None;
        }
        Some(self.mint_request(Some(RangeRequest {
            start: frontier,
            end_exclusive: frontier.saturating_add(self.config.chunk_bytes as u64),
        })))
    }

    /// Whether the read cursor is covered for a demuxer read: the
    /// staged span covers `[pos, pos + READY_MARGIN)` or the known
    /// end falls inside it (the pump's gate — see the module docs).
    #[must_use]
    pub fn ready(&self) -> bool {
        let need_end = self.pos.saturating_add(READY_MARGIN);
        let need = match self.len {
            Some(len) if len <= need_end => {
                // The known end is inside the margin: coverage up to
                // the end (or past it) is readiness.
                return self.pos >= len || self.covers_staged(self.pos, len);
            }
            _ => need_end,
        };
        if self.pos >= need {
            return true;
        }
        self.covers_staged(self.pos, need)
    }

    /// Absorb one served response: correlate by id, validate, stage.
    ///
    /// The PROBE response (the first) runs the seekability detection
    /// and fixes the mode. A mid-stream `200` under ranged mode
    /// resyncs into scan mode (the `ranged.rs` flip, mirrored). A
    /// truncated body that lands exactly on the resource end is the
    /// honest wire-end shape (the length fixes from it); anything
    /// else surfaces typed.
    ///
    /// # Errors
    /// [`EngineError`] — a correlation mismatch, the transport's
    /// typed failures, or the detection's protocol violations (all
    /// stage `Open`).
    pub fn absorb(&mut self, response: FetchResponse) -> Result<(), EngineError> {
        let Some(outstanding) = self.outstanding.take() else {
            return Err(mint(
                media_types::ErrorKind::Unsupported { format: None },
                "web.fetch.unsolicited",
            )
            .with_source(std::io::Error::other(format!(
                "a response (id {}) arrived with no outstanding request",
                response.id
            ))));
        };
        if response.id != outstanding.id {
            self.outstanding = Some(outstanding.clone());
            return Err(mint(
                media_types::ErrorKind::Unsupported { format: None },
                "web.fetch.correlation",
            )
            .with_source(std::io::Error::other(format!(
                "response id {} does not match the outstanding request id {}",
                response.id, outstanding.id
            ))));
        }
        self.stats.bytes_fetched = self
            .stats
            .bytes_fetched
            .saturating_add(response.body.len() as u64);
        let request = outstanding.as_fetch_request();
        // The hard Rust-side bound on whole-body answers (the
        // transport refuses first; a body that slips past it is
        // still never staged beyond the window budget — bounded
        // memory is this side's own duty, not the glue's promise).
        if response.status == 200
            && u64::try_from(response.body.len()).unwrap_or(u64::MAX)
                > u64::try_from(self.config.window_bytes).unwrap_or(u64::MAX)
        {
            return Err(FetchError::ResponseTooLarge {
                limit: u64::try_from(self.config.window_bytes).unwrap_or(u64::MAX),
                actual: response.body.len() as u64,
            }
            .as_engine_error());
        }
        match detect_seekability(&request, &response) {
            Ok(seekability) => {
                if self.seekability.is_none() {
                    // The probe answer: fix the mode and the length.
                    self.seekability = Some(seekability);
                    self.len = seekability.length();
                    match seekability {
                        Seekability::Ranged { .. } => self.insert_window(0, response.body),
                        Seekability::Unseekable { .. } | Seekability::Empty => {
                            self.mode = Mode::Scan;
                            if !response.body.is_empty() {
                                self.scan = Some(ScanBuffer::new(0, response.body));
                            }
                        }
                    }
                } else {
                    match (self.mode, response.status) {
                        // A later ranged window: stage at the range start.
                        (Mode::Ranged, 206) => {
                            let start = outstanding.range.map_or(0, |range| range.start);
                            self.insert_window(start, response.body);
                        }
                        // The mid-stream flip: a ranged fetch suddenly
                        // answered full-body — resync into scan.
                        (Mode::Ranged, 200) => {
                            self.stats.resyncs = self.stats.resyncs.saturating_add(1);
                            self.mode = Mode::Scan;
                            self.windows.clear();
                            self.cached = 0;
                            self.len = response
                                .header("Content-Length")
                                .and_then(|value| value.trim().parse::<u64>().ok())
                                .or(Some(response.body.len() as u64));
                            if !response.body.is_empty() {
                                self.scan = Some(ScanBuffer::new(0, response.body));
                            }
                        }
                        // The scan re-fetch landed (or the resource
                        // answered empty — a mid-stream 416): replace
                        // the buffer when a body came, otherwise keep
                        // the staged state (never a crash, never a
                        // silent corruption).
                        (_, _) => {
                            if !response.body.is_empty() {
                                self.scan = Some(ScanBuffer::new(0, response.body));
                            }
                        }
                    }
                }
                Ok(())
            }
            // A truncated body is the honest wire-end shape when it
            // lands on the resource end (or discovers it — unknown
            // length); mid-resource it is the typed truncation,
            // never a silent EOF.
            Err(FetchError::Truncated { expected, got }) => {
                let start = outstanding.range.map_or(0, |range| range.start);
                let end = start.saturating_add(got);
                let at_end = self.len.is_none() || self.len == Some(end);
                if at_end {
                    if self.len.is_none() {
                        self.len = Some(end);
                        if self.seekability.is_none() {
                            self.seekability = Some(Seekability::Ranged { total: Some(end) });
                        }
                    }
                    self.insert_window(start, response.body);
                    Ok(())
                } else {
                    Err(FetchError::Truncated { expected, got }.as_engine_error())
                }
            }
            Err(other) => Err(other.as_engine_error()),
        }
    }

    /// Serve one seek (the [`SourceStream`] contract). Staged targets
    /// serve directly; unstaged targets mint the fetch request and
    /// answer the typed [`window_not_staged`] refusal until the
    /// response lands (the caller retries).
    fn seek_to(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError> {
        let target: i64 = match origin {
            SeekOrigin::Start(offset) => offset as i64,
            SeekOrigin::Current(delta) => (self.pos as i64).saturating_add(delta),
        };
        if target < 0 {
            return Err(self.out_of_range());
        }
        let target = target as u64;
        if let Some(len) = self.len
            && target > len
        {
            return Err(self.out_of_range());
        }
        // The one-past-the-end position (EOF) always serves without
        // staging: a read there is the honest empty read at the known
        // end, in every mode.
        if let Some(len) = self.len
            && target == len
        {
            let skipped = target.saturating_sub(self.pos);
            self.stats.skipped_bytes = self.stats.skipped_bytes.saturating_add(skipped);
            self.pos = target;
            return Ok(SourcePosition(target));
        }
        match self.mode {
            Mode::Ranged => {
                if self.windows.iter().any(|window| window.covers(target)) {
                    self.pos = target;
                    return Ok(SourcePosition(target));
                }
                if self.outstanding.is_none() {
                    let range = RangeRequest {
                        start: target,
                        end_exclusive: target.saturating_add(self.config.chunk_bytes as u64),
                    };
                    let _ = self.mint_request(Some(range));
                }
                Err(window_not_staged(target, false))
            }
            Mode::Scan => {
                if let Some(scan) = self.scan.as_mut()
                    && scan.serves(target)
                {
                    // The jump's bytes are consumed-and-discarded (the
                    // honest skip accounting); the frontier advances.
                    let skipped = target.saturating_sub(self.pos);
                    self.stats.skipped_bytes = self.stats.skipped_bytes.saturating_add(skipped);
                    scan.consumed = ((target - scan.base) as usize).max(scan.consumed);
                    self.pos = target;
                    return Ok(SourcePosition(target));
                }
                // Behind the consumed frontier (or ahead of the staged
                // body): the whole-body re-fetch, counted (the honest
                // linear cost of a no-range server).
                self.stats.refetches = self.stats.refetches.saturating_add(1);
                if self.outstanding.is_none() {
                    let _ = self.mint_request(None);
                }
                Err(window_not_staged(target, true))
            }
        }
    }

    fn out_of_range(&self) -> EngineError {
        mint(
            media_types::ErrorKind::Inaccessible {
                reason: AccessDenied::NotFound,
            },
            "input.seek.out-of-range",
        )
    }

    /// The staged frontier (ranged mode): one past the last staged
    /// byte.
    fn frontier(&self) -> u64 {
        self.windows
            .last()
            .map_or(0, |window| window.start + window.data.len() as u64)
    }

    /// Whether the staged span covers `[from, to)` in the active
    /// mode (windows chain without gaps; the scan buffer is one
    /// span).
    fn covers_staged(&self, from: u64, to: u64) -> bool {
        if to <= from {
            return true;
        }
        match self.mode {
            Mode::Ranged => {
                let mut cursor = from;
                'outer: while cursor < to {
                    for window in &self.windows {
                        if window.covers(cursor) {
                            cursor = window.start + window.data.len() as u64;
                            continue 'outer;
                        }
                    }
                    return false;
                }
                true
            }
            Mode::Scan => match &self.scan {
                Some(scan) => {
                    from >= scan.base + scan.consumed as u64
                        && (to - scan.base) <= scan.data.len() as u64
                }
                None => false,
            },
        }
    }

    /// Drop windows the forward read cursor can never touch again
    /// (their end is at or before `pos`) — the aging half of the
    /// bounded-memory contract.
    fn age_behind_cursor(&mut self) {
        let cursor = self.pos;
        let before = self.windows.len();
        self.windows.retain(|window| {
            let keep = window.start + window.data.len() as u64 > cursor;
            if !keep {
                self.cached = self.cached.saturating_sub(window.data.len());
                self.stats.cache_evictions = self.stats.cache_evictions.saturating_add(1);
            }
            keep
        });
        let _ = before;
    }

    /// Insert one window at `start`, keeping the cache sorted by
    /// `start` and under budget: age first, then evict
    /// front-most windows (never one covering the cursor).
    fn insert_window(&mut self, start: u64, data: Vec<u8>) {
        self.age_behind_cursor();
        self.cached = self.cached.saturating_add(data.len());
        let index = self.windows.partition_point(|window| window.start < start);
        self.windows.insert(index, Window { start, data });
        while self.cached > self.config.window_bytes && self.windows.len() > 1 {
            // Evict the lowest-start window that does not cover the
            // cursor (the oldest by position; sorted order makes the
            // scan linear).
            let cursor = self.pos;
            let index = self
                .windows
                .iter()
                .position(|window| !window.covers(cursor))
                .unwrap_or(0);
            let Some(removed) = self.windows.get(index) else {
                break;
            };
            let removed_len = removed.data.len();
            self.cached = self.cached.saturating_sub(removed_len);
            self.stats.cache_evictions = self.stats.cache_evictions.saturating_add(1);
            self.windows.remove(index);
        }
    }

    /// Mint one request (takes the id and marks it outstanding).
    /// The response budget is the WINDOW budget on every request —
    /// the lane's bounded-memory contract (`fetch()` answers
    /// whole bodies, and a no-range server's `200` is legitimately
    /// larger than one chunk, so the chunk extent cannot be the
    /// bound; the chunk discipline keeps normal `206` spans far
    /// under it, and the transport refuses anything over it).
    fn mint_request(&mut self, range: Option<RangeRequest>) -> WindowRequest {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.stats.requests = self.stats.requests.saturating_add(1);
        let limit = u64::try_from(self.config.window_bytes).unwrap_or(u64::MAX);
        let request = WindowRequest {
            id,
            url: self.url.clone(),
            range,
            limit,
        };
        self.outstanding = Some(request.clone());
        request
    }

    /// Gather `out.len()` bytes from the staged span, advancing the
    /// cursor; answers the bytes gathered (short only at the
    /// staged/known end — see [`READY_MARGIN`]).
    fn gather(&mut self, out: &mut [u8]) -> usize {
        let mut gathered = 0;
        match self.mode {
            Mode::Ranged => {
                'outer: while gathered < out.len() {
                    let pos = self.pos + gathered as u64;
                    for window in &self.windows {
                        if window.covers(pos) {
                            let offset = (pos - window.start) as usize;
                            let take = (out.len() - gathered).min(window.data.len() - offset);
                            out[gathered..gathered + take]
                                .copy_from_slice(&window.data[offset..offset + take]);
                            gathered += take;
                            continue 'outer;
                        }
                    }
                    break;
                }
                if gathered > 0 {
                    self.stats.cache_hits = self.stats.cache_hits.saturating_add(1);
                }
            }
            Mode::Scan => {
                if let Some(scan) = self.scan.as_mut() {
                    let rel = self.pos.saturating_sub(scan.base) as usize;
                    let at = rel.max(scan.consumed);
                    if at < scan.data.len() {
                        let take = out.len().min(scan.data.len() - at);
                        out[..take].copy_from_slice(&scan.data[at..at + take]);
                        scan.consumed = (at + take).max(scan.consumed);
                        gathered = take;
                    }
                }
            }
        }
        self.pos = self.pos.saturating_add(gathered as u64);
        gathered
    }
}

impl SourceStream for StagedSource {
    fn uri(&self) -> &SourceUri {
        &self.uri
    }

    fn size(&self) -> Option<u64> {
        self.len
    }

    fn supports_seek(&self) -> bool {
        // The server truth (the detection); serving is staged-window
        // bounded in v1 — see the module docs' honest boundary note.
        self.seekability
            .is_some_and(|seekability| seekability.is_seekable())
    }

    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        let len = len.min(MAX_READ_BYTES);
        let mut out = vec![0; len];
        let gathered = self.gather(&mut out);
        out.truncate(gathered);
        Ok(out)
    }

    fn read_into(&mut self, out: &mut [u8]) -> Result<usize, EngineError> {
        Ok(self.gather(out))
    }

    fn seek(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError> {
        self.seek_to(origin)
    }

    fn position(&self) -> SourcePosition {
        SourcePosition(self.pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::fetch_port::FetchPort;
    use crate::io::mock_fetch::{MockFetch, RouteBehavior};
    use plugin_api::{Demuxer, SeekOrigin};
    use testkit::{Must, assert_err};

    /// The transport double (see `mock_fetch`): routes are scripted
    /// per test; `serve` drives the request/absorb cycle.
    fn serve(source: &mut StagedSource, transport: &MockFetch) -> Result<(), EngineError> {
        let Some(outstanding) = source.outstanding().cloned() else {
            return Ok(());
        };
        let response = transport.fetch(&outstanding.as_fetch_request()).must();
        source.absorb(response)
    }

    /// Pipeline until no request is pending (bounded loops).
    fn pipeline(source: &mut StagedSource, transport: &MockFetch) {
        for _ in 0..64 {
            match source.pending_request() {
                Some(_) => serve(source, transport).must(),
                None => break,
            }
        }
    }

    /// A tiny ranged server: 12 bytes, chunk 4.
    fn tiny_ranged() -> (StagedSource, MockFetch) {
        let (mut source, probe) = StagedSource::open(
            // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
            "https://media.example/tiny.bin",
            StagedConfig {
                window_bytes: 16,
                chunk_bytes: 4,
            },
        )
        .must();
        let transport = MockFetch::with_routes(vec![(
            "/tiny.bin",
            RouteBehavior::HonorsRange,
            b"0123456789AB".as_slice(),
        )]);
        assert_eq!(
            probe.range.map(|range| (range.start, range.end_exclusive)),
            Some((0, 4))
        );
        serve(&mut source, &transport).must();
        (source, transport)
    }

    /// Probe + detection: the ranged server fixes `Ranged { total }`
    /// and the length; reads coalesce across windows.
    #[test]
    fn ranged_server_reads_across_windows() {
        let (mut source, transport) = tiny_ranged();
        assert_eq!(
            source.seekability().must(),
            Seekability::Ranged { total: Some(12) }
        );
        assert_eq!(source.size(), Some(12));
        assert!(source.supports_seek());
        // Read 6 bytes: crosses the first window (4) into the second
        // (the read-ahead pipelined it already).
        pipeline(&mut source, &transport);
        assert!(source.ready(), "the staged span covers the margin at 0");
        assert_eq!(source.read(6).must(), b"012345");
        assert_eq!(source.position(), SourcePosition(6));
        // Run to the end through the remaining windows.
        pipeline(&mut source, &transport);
        assert_eq!(source.read(100).must(), b"6789AB", "short read only at EOS");
        assert!(source.read(4).must().is_empty(), "zero only at EOS");
        assert!(source.ready(), "EOS is ready");
    }

    /// The no-range server: scan mode, forward-only, seekable false.
    #[test]
    fn no_range_server_scans_forward_only() {
        let (mut source, probe) = StagedSource::open(
            // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
            "https://media.example/norange.bin",
            StagedConfig {
                window_bytes: 64,
                chunk_bytes: 4,
            },
        )
        .must();
        let transport = MockFetch::with_routes(vec![(
            "/norange.bin",
            RouteBehavior::IgnoresRange,
            b"abcdef".as_slice(),
        )]);
        // The probe is a ranged request; the server answers 200 with
        // the whole body — scan mode.
        let response = transport.fetch(&probe.as_fetch_request()).must();
        assert_eq!(response.status, 200);
        source.absorb(response).must();
        assert_eq!(
            source.seekability().must(),
            Seekability::Unseekable { length: Some(6) }
        );
        assert!(!source.supports_seek());
        assert!(source.ready());
        assert_eq!(source.read(3).must(), b"abc");
        // A forward seek inside the retained body serves; a backward
        // seek re-fetches (counted) and refuses the not-staged
        // boundary until the response lands.
        assert_eq!(source.seek(SeekOrigin::Start(4)).must(), SourcePosition(4));
        assert_eq!(source.read(2).must(), b"ef");
        let err = assert_err(source.seek(SeekOrigin::Start(0)), "backward seek");
        assert_eq!(err.context.code.as_str(), "web.fetch.window-not-staged");
        assert_eq!(source.stats().refetches, 1, "the re-fetch is counted");
        // The re-fetch request was minted: serving it lands the body
        // again, and the seek now succeeds.
        serve(&mut source, &transport).must();
        assert_eq!(source.seek(SeekOrigin::Start(0)).must(), SourcePosition(0));
        assert_eq!(source.read(6).must(), b"abcdef");
        // The skip accounting: one byte jumped inside the buffer
        // (position 3 → 4); the backward seek's cost is the REFETCH
        // (counted above), not a skip — the two counters name the
        // two halves of the no-range server's linear cost.
        assert_eq!(
            source.stats().skipped_bytes,
            1,
            "the in-buffer skip is accounted"
        );
    }

    /// Bounded memory: windows age past the cursor and the budget
    /// holds — forward progress never deadlocks on a full cache
    /// (the starvation `ranged.rs`'s eviction avoids).
    #[test]
    fn window_cache_ages_and_progress_never_deadlocks() {
        let (mut source, transport) = tiny_ranged();
        // Consume the whole 12-byte resource through a 16-byte
        // window budget with 4-byte chunks: reads advance, aging
        // frees budget, pipelining keeps staging.
        let mut consumed = 0;
        for _ in 0..16 {
            pipeline(&mut source, &transport);
            assert!(source.ready(), "ready at {consumed}");
            let chunk = source.read(4).must();
            if chunk.is_empty() {
                break;
            }
            consumed += chunk.len();
            assert!(
                source.cached <= 16 + 4,
                "the budget holds +one chunk in flight"
            );
        }
        assert_eq!(consumed, 12, "the whole resource was read");
        assert!(source.stats().cache_evictions >= 2, "aging dropped windows");
    }

    /// Ready-gate semantics: not ready while the margin is unstaged
    /// (unknown end), ready once the staged span covers it or the
    /// known end falls inside it.
    #[test]
    fn ready_gate_tracks_the_margin_and_the_end() {
        let (mut source, transport) = tiny_ranged();
        // Only the 4-byte probe window is staged; the 256 KiB margin
        // is not covered and the end is known (12) but not staged —
        // not ready.
        assert!(!source.ready(), "the end is known but not staged");
        pipeline(&mut source, &transport);
        // All 12 bytes staged; the margin extends past the end —
        // ready because the known end is covered.
        assert!(source.ready(), "the known end is covered");
        // At the end: ready (EOS).
        assert_eq!(
            source.seek(SeekOrigin::Start(12)).must(),
            SourcePosition(12)
        );
        assert!(source.ready());
        assert!(source.read(8).must().is_empty());
    }

    /// Correlation: an unsolicited response, or one with the wrong
    /// id, refuses typed (never a silent drop), and the outstanding
    /// request survives the mismatch.
    #[test]
    fn correlation_refuses_unsolicited_and_mismatched() {
        let (mut source, transport) = tiny_ranged();
        let request = source.pending_request().must();
        let err = assert_err(
            source.absorb(FetchResponse {
                id: 999,
                status: 206,
                headers: vec![("Content-Range".to_string(), "bytes 4-7/12".to_string())],
                body: b"4567".to_vec(),
                // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
                final_url: "https://media.example/tiny.bin".to_string(),
                redirected: false,
            }),
            "wrong id",
        );
        assert_eq!(err.context.code.as_str(), "web.fetch.correlation");
        // The mismatched response was for nobody; the real request
        // survives — serve it and the window stages.
        let outstanding = source.outstanding().must().clone();
        assert_eq!(outstanding.id, request.id);
        serve(&mut source, &transport).must();
        // An unsolicited response (nothing outstanding) refuses too.
        let err = assert_err(
            source.absorb(FetchResponse {
                id: 1,
                status: 200,
                headers: Vec::new(),
                body: Vec::new(),
                final_url: String::new(),
                redirected: false,
            }),
            "unsolicited",
        );
        assert_eq!(err.context.code.as_str(), "web.fetch.unsolicited");
    }

    /// Mid-stream resync: a ranged fetch suddenly answered `200`
    /// flips the source into scan mode (counted).
    #[test]
    fn mid_stream_200_resyncs_into_scan() {
        let (mut source, _transport) = tiny_ranged();
        // Mint the next window and answer it with a 200 whole body.
        let request = source.pending_request().must();
        let response = FetchResponse {
            id: request.id,
            status: 200,
            headers: vec![("Content-Length".to_string(), "12".to_string())],
            body: b"0123456789AB".to_vec(),
            // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
            final_url: "https://media.example/tiny.bin".to_string(),
            redirected: false,
        };
        source.absorb(response).must();
        assert_eq!(source.stats().resyncs, 1, "the flip is counted");
        // The scan body serves reads from the cursor.
        let _ = source.seek(SeekOrigin::Start(6)).must();
        assert_eq!(source.read(6).must(), b"6789AB");
    }

    /// The truncated mid-resource window is the typed truncation;
    /// the wire-end shape is honest (the length fixes from it).
    #[test]
    fn truncated_windows_are_typed_or_wire_end() {
        // (a) Mid-resource truncation: the typed refusal — never a
        // silent EOF.
        let (mut source, _probe) = StagedSource::open(
            // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
            "https://media.example/trunc.bin",
            StagedConfig {
                window_bytes: 64,
                chunk_bytes: 4,
            },
        )
        .must();
        let transport = MockFetch::with_routes(vec![(
            "/trunc.bin",
            RouteBehavior::HonorsRange,
            b"01234567".as_slice(),
        )]);
        serve(&mut source, &transport).must();
        assert_eq!(source.size(), Some(8));
        let truncating = MockFetch::with_routes(vec![(
            "/trunc.bin",
            RouteBehavior::TruncatesRange,
            b"01234567".as_slice(),
        )]);
        let request = source.pending_request().must();
        let response = truncating.fetch(&request.as_fetch_request()).must();
        let err = assert_err(source.absorb(response), "mid-resource truncation");
        assert!(err.to_string().contains("truncated"), "{err}");
        // (b) The wire-end shape: an 8-byte resource under a 16-byte
        // chunk gets one 8-byte 206 — honest, length fixed.
        let (mut source, probe) = StagedSource::open(
            // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
            "https://media.example/wireend.bin",
            StagedConfig {
                window_bytes: 64,
                chunk_bytes: 16,
            },
        )
        .must();
        let transport = MockFetch::with_routes(vec![(
            "/wireend.bin",
            RouteBehavior::HonorsRange,
            b"01234567".as_slice(),
        )]);
        let response = transport.fetch(&probe.as_fetch_request()).must();
        source.absorb(response).must();
        assert_eq!(source.size(), Some(8));
        assert!(source.ready());
        assert_eq!(source.read(8).must(), b"01234567");
    }

    /// Config validation refuses the impossible shapes typed (never
    /// clamped) — `ranged.rs`'s rule, verbatim.
    #[test]
    fn config_validation_refuses_typed() {
        let zero = StagedConfig {
            window_bytes: 0,
            chunk_bytes: 4,
        };
        let err = assert_err(
            // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
            StagedSource::open("https://media.example/x", zero),
            "zero window",
        );
        assert_eq!(err.context.code.as_str(), "web.fetch.config-zero");
        let over = StagedConfig {
            window_bytes: 4,
            chunk_bytes: 8,
        };
        let err = assert_err(
            // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
            StagedSource::open("https://media.example/x", over),
            "chunk over",
        );
        assert_eq!(err.context.code.as_str(), "web.fetch.chunk-over-window");
        // A non-http(s) URL refuses at open (the early typed
        // validation).
        let err = assert_err(
            StagedSource::open("ftp://media.example/x", StagedConfig::default()),
            "bad scheme",
        );
        assert_eq!(err.context.code.as_str(), "web.fetch.url");
    }

    /// THE FETCH-LANE PLAYBACK PROOF over the REAL first-vertical
    /// fixture: the accept-ranges mock serves the AVI, the staged
    /// source serves the PROBE and the DEMUXER through the same
    /// [`SourceStream`] contract the blob lane uses, and the demuxer
    /// reads the whole media to its end through staged windows. The
    /// mid-stream pump-gate (serving pending requests between engine
    /// polls) is the browser message lane's wiring — [PROPOSAL] with
    /// 7.I1; here the whole resource stages first (bounded by the
    /// window budget), which the read-ahead pipeline does by itself.
    #[test]
    fn fetch_lane_serves_the_real_fixture_to_the_demuxer() {
        let bytes = testkit::fixture_bytes("generated/valid/avi_mjpeg_pcm.avi").must();
        let transport = MockFetch::with_routes(vec![(
            "/fixture.avi",
            RouteBehavior::HonorsRange,
            bytes.as_slice(),
        )]);
        let (mut source, probe) = StagedSource::open(
            // lumen-allow-literal: reserved example host — fetch-lane test vector over the real fixture, never a shipped endpoint
            "https://media.example/fixture.avi",
            StagedConfig::default(),
        )
        .must();
        assert_eq!(
            probe.range.map(|range| (range.start, range.end_exclusive)),
            Some((0, DEFAULT_CHUNK_BYTES as u64)),
            "the probe is the first read-ahead"
        );
        // Serve the probe, then pipeline the read-ahead to the end
        // (the whole fixture fits the 8 MiB window budget).
        serve(&mut source, &transport).must();
        assert_eq!(
            source.seekability().must(),
            Seekability::Ranged {
                total: Some(bytes.len() as u64)
            }
        );
        for _ in 0..64 {
            match source.pending_request() {
                Some(_) => serve(&mut source, &transport).must(),
                None => break,
            }
        }
        assert!(source.ready(), "the staged span covers the margin");
        assert_eq!(source.stats().bytes_fetched, bytes.len() as u64);
        // The probe names the container through the REAL probe path.
        let registry = media_probe::ProbeRegistry::with_builtins();
        let report =
            media_probe::probe_stream(&registry, &mut source, &media_probe::ProbeConfig::default())
                .must();
        assert_eq!(report.format.as_str(), "avi");
        source.seek(plugin_api::SeekOrigin::Start(0)).must();
        // The demuxer reads the whole media through the staged
        // windows: start + every event to the end.
        let mut demuxer = demux::avi::AviDemuxer::new(Box::new(source)).must();
        let tracks = demuxer.start().must();
        assert_eq!(tracks.len(), 2, "one video + one audio track");
        let mut packets = 0_u64;
        for _ in 0..4096 {
            match demuxer.next_event().must() {
                plugin_api::DemuxEvent::Packet(_) => packets += 1,
                plugin_api::DemuxEvent::EndOfStream => break,
                plugin_api::DemuxEvent::Recovered { .. }
                | plugin_api::DemuxEvent::Discontinuity => {}
            }
        }
        assert!(packets > 0, "the media's packets were demuxed: {packets}");
    }
}
